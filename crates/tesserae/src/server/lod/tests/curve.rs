//! A Quality curve changed at run time, from the Client's request to the rings streamed.

use super::*;
use crate::server::quality::{self, DeclaredStreaming, QualityCurve};

/// The doubling from 32 m to 1024 m: six levels, each one Cell thick.
fn game() -> StreamingSettings {
    StreamingSettings {
        lod_distance_m: 1024.0,
        ..small()
    }
}

const GAME_LEVELS: u32 = 6;

/// What the Client draws of every level by ADR-0019: an unloaded Chunk stays drawn until a
/// coarser level delivers its ancestor or finer levels deliver all its region.
#[derive(Default)]
struct Drawn {
    delivered: HashSet<LodKey>,
    retained: HashSet<LodKey>,
}

impl Drawn {
    fn apply(
        &mut self,
        level_zero: (Vec<ChunkMeshReady>, Vec<ChunkCoord>),
        coarse: (Vec<LodMeshReady>, Vec<LodUnloaded>),
    ) {
        let ready = level_zero.0.iter().map(|m| (0, m.coord));
        for key in ready.chain(coarse.0.iter().map(|m| (m.level, m.coord))) {
            self.retained.remove(&key);
            self.delivered.insert(key);
        }
        let unloaded = level_zero.1.iter().map(|&c| (0, c));
        for key in unloaded.chain(coarse.1.iter().map(|u| (u.level, u.coord))) {
            if self.delivered.remove(&key) {
                self.retained.insert(key);
            }
        }
        let released: Vec<LodKey> = self
            .retained
            .iter()
            .copied()
            .filter(|&key| self.under_delivered(key) || self.over_delivered(key))
            .collect();
        for key in released {
            self.retained.remove(&key);
        }
    }

    fn under_delivered(&self, (level, coord): LodKey) -> bool {
        (level + 1..GAME_LEVELS).any(|coarser| {
            self.delivered
                .contains(&(coarser, coord.ancestor(coarser - level)))
        })
    }

    fn over_delivered(&self, (level, coord): LodKey) -> bool {
        level > 0
            && coord.children().all(|child| {
                let key = (level - 1, child);
                self.delivered.contains(&key) || self.over_delivered(key)
            })
    }

    fn draws(&self, world: &WorldSettings, at: Vec3, delivered_only: bool) -> bool {
        (0..GAME_LEVELS).any(|level| {
            let key = (level, world.chunk_at_level(at, level));
            self.delivered.contains(&key) || (!delivered_only && self.retained.contains(&key))
        })
    }
}

/// Every level-0 Cell centre of the field, in two rows every level's vertical extent holds.
fn samples(world: &WorldSettings) -> Vec<Vec3> {
    let cell = world.cell_size();
    let reach = (game().lod_distance_m / cell) as i32 * 2;
    let mut points = Vec::new();
    for x in -reach..reach {
        for z in -reach..reach {
            for y in [-1, 0] {
                points.push((IVec3::new(x, y, z).as_vec3() + 0.5) * cell);
            }
        }
    }
    points
}

fn request(h: &mut Harness, curve: QualityCurve) {
    let world = h.app.world_mut();
    world.write_message(curve);
    world
        .run_system_once(quality::apply)
        .expect("the request was read");
}

/// Streams to quiescence, asserting that every point drawn now and drawn at the end is drawn
/// at every tick between.
fn settle_without_a_gap(h: &mut Harness, drawn: &mut Drawn, label: &str) {
    let world = *h.app.world().resource::<WorldSettings>();
    let before: Vec<Vec3> = samples(&world)
        .into_iter()
        .filter(|&p| drawn.draws(&world, p, true))
        .collect();
    let mut ticks: Vec<Vec<Vec3>> = Vec::new();
    let started = std::time::Instant::now();
    loop {
        assert!(started.elapsed() < SETTLE_CEILING, "{label}: never settled");
        let level_zero = h.tick();
        let coarse = h.lod_tick();
        let quiet = [level_zero.0.is_empty(), level_zero.1.is_empty()]
            .into_iter()
            .chain([coarse.0.is_empty(), coarse.1.is_empty()])
            .all(|empty| empty)
            && h.quiet()
            && h.lod_quiet();
        drawn.apply(level_zero, coarse);
        ticks.push(
            before
                .iter()
                .copied()
                .filter(|&p| !drawn.draws(&world, p, false))
                .collect(),
        );
        if quiet {
            break;
        }
        std::thread::sleep(std::time::Duration::from_micros(100));
    }
    let kept: HashSet<IVec3> = before
        .iter()
        .filter(|&&p| drawn.draws(&world, p, true))
        .map(|p| p.as_ivec3())
        .collect();
    assert!(
        before.is_empty() || !kept.is_empty(),
        "{label}: nothing is drawn after"
    );
    for (tick, undrawn) in ticks.iter().enumerate() {
        let gap: Vec<&Vec3> = undrawn
            .iter()
            .filter(|p| kept.contains(&p.as_ivec3()))
            .collect();
        assert!(
            gap.is_empty(),
            "{label}: tick {tick} drew nothing at {gap:?}"
        );
    }
}

fn assert_streams_the_rings_in_effect(h: &Harness, levels: u32, label: &str) {
    let settings = *h.app.world().resource::<StreamingSettings>();
    assert_eq!(level_count(&settings), levels, "{label}");
    for level in 1..levels {
        let shell = h.mesh_shell(level);
        assert!(
            shell.is_subset(&h.delivered_at(level)),
            "{label}: level {level} is missing part of its shell"
        );
        let keep = h.rings_of(level).keep;
        let anchor = h.anchor_of(level);
        for coord in h.delivered_at(level) {
            assert!(
                keep.contains(delta(anchor, coord)),
                "{label}: {level} {coord}"
            );
        }
    }
    for level in levels..GAME_LEVELS {
        assert!(h.delivered_at(level).is_empty(), "{label}: level {level}");
        assert_eq!(h.held(level).len(), 0, "{label}: level {level}");
    }
}

#[test]
fn a_new_curve_streams_its_rings_with_no_gap_and_back_again() {
    let mut h = harness_with(game(), unlimited());
    h.app
        .add_message::<QualityCurve>()
        .insert_resource(DeclaredStreaming(game()));
    let mut drawn = Drawn::default();
    settle_without_a_gap(&mut h, &mut drawn, "the first fill");
    assert_streams_the_rings_in_effect(&h, GAME_LEVELS, "the Game's doubling");

    // 256, 512 and 1024 m: four Cells thick, and four levels where the doubling takes six.
    request(
        &mut h,
        QualityCurve {
            unit_m: 128.0,
            reach_m: 8192.0,
        },
    );
    assert_eq!(
        h.app.world().resource::<StreamingSettings>().lod_distance_m,
        1024.0,
        "the reach is clamped to the Game's"
    );
    settle_without_a_gap(&mut h, &mut drawn, "to the wider unit");
    assert_streams_the_rings_in_effect(&h, 4, "the wider unit");

    request(
        &mut h,
        QualityCurve {
            unit_m: 0.0,
            reach_m: 1024.0,
        },
    );
    settle_without_a_gap(&mut h, &mut drawn, "back to the doubling");
    assert_streams_the_rings_in_effect(&h, GAME_LEVELS, "back to the doubling");
}
