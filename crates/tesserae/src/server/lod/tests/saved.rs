//! A saved World's coarse levels: a change climbs one level at a time, and only
//! as far as a rebuilt Chunk comes out different.

use super::*;

/// Settled on a saved World, with every coarse cover of the quiet Chunk held.
fn saved_world() -> (Harness, ChunkCoord) {
    let mut h = harness();
    h.app.world_mut().resource_mut::<ChunkStore>().persistent = true;
    h.settle();
    h.lod_settle();
    let chunk = h.quiet_chunk();
    for (level, covering) in covers(chunk) {
        assert!(
            h.lod_store().contains(level, covering),
            "level {level} cover not held"
        );
    }
    (h, chunk)
}

fn covers(chunk: ChunkCoord) -> Vec<LodKey> {
    let mut covering = chunk;
    (1..LEVELS)
        .map(|level| {
            covering = cover(covering);
            (level, covering)
        })
        .collect()
}

/// Settles, recording each cover of `chunk` that was ever regenerated.
fn regenerated(h: &mut Harness, chunk: ChunkCoord) -> Vec<u32> {
    let watched = covers(chunk);
    let mut seen = HashSet::new();
    for _ in 0..SETTLE_TICKS {
        h.lod_tick();
        seen.extend(
            watched
                .iter()
                .filter(|&&key| h.lod_jobs().generate.contains(key))
                .map(|&(level, _)| level),
        );
        if h.lod_quiet() {
            let mut seen: Vec<u32> = seen.into_iter().collect();
            seen.sort_unstable();
            return seen;
        }
        if h.lod_in_flight() > 0 {
            std::thread::sleep(std::time::Duration::from_micros(100));
        }
    }
    panic!("coarse streaming was still busy after {SETTLE_TICKS} ticks");
}

#[test]
fn a_change_under_a_saved_world_marks_only_its_level_one_cover() {
    let (mut h, chunk) = saved_world();
    h.edit_block(chunk, [10, 10, 10], stone());
    h.lod_tick();
    let covers = covers(chunk);
    assert!(h.lod_jobs().generate.contains(covers[0]));
    for &(level, covering) in &covers[1..] {
        assert!(
            !h.lod_jobs().generate.contains((level, covering)),
            "level {level}"
        );
        assert!(!h.lod_state().is_stale(level, covering), "level {level}");
    }
}

#[test]
fn a_level_one_rebuild_that_changed_nothing_marks_nothing_above_it() {
    let (mut h, chunk) = saved_world();
    // One Voxel of eight is outvoted: the cover comes out as it was.
    h.edit(
        [chunk.x * 32 + 10, chunk.y * 32 + 10, chunk.z * 32 + 10],
        stone(),
    );
    assert_eq!(regenerated(&mut h, chunk), vec![1]);
}

#[test]
fn a_level_one_rebuild_that_changed_marks_the_level_above_and_stops_where_nothing_changed() {
    let (mut h, chunk) = saved_world();
    // A whole level-1 Voxel turns to stone; one level-1 Voxel of eight is outvoted at level 2.
    h.edit_block(chunk, [10, 10, 10], stone());
    assert_eq!(regenerated(&mut h, chunk), vec![1, 2]);
}
