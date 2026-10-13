//! The rings of one LOD level: the same four extents as [`Rings`], but the mesh
//! and keep extents are shells with a hole where the next finer level draws.
//!
//! Everything is derived from metres. A level's Cell is `2^level` times the
//! level-0 Cell, and every distance in [`StreamingSettings`] scales the same
//! way, so a Cell count never depends on the level except where `lod_distance_m`
//! clamps the outermost one. See `docs/adr/0003-voxel-size-is-per-world.md`.
//!
//! [`Rings`]: super::streaming::Rings

use bevy::prelude::*;

use super::streaming::{Extent, StreamingSettings};

/// Past this the doubling would overflow a Cell count long before it was useful.
const MAX_LEVELS: u32 = 32;

/// How many LOD levels the settings ask for: the smallest `n` with
/// `view_distance_m * 2^(n-1) >= lod_distance_m`, and 1 when the reach is no
/// further than level 0's.
pub fn level_count(settings: &StreamingSettings) -> u32 {
    let mut levels = 1;
    while levels < MAX_LEVELS && level_distance_m(settings, levels - 1) < settings.lod_distance_m {
        levels += 1;
    }
    levels
}

/// The distance, in metres, that `level` is meant to reach.
fn level_distance_m(settings: &StreamingSettings, level: u32) -> f32 {
    settings.view_distance_m * scale(level)
}

fn scale(level: u32) -> f32 {
    2f32.powi(level as i32)
}

/// An [`Extent`] with an optional hole cut out of its middle.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Shell {
    pub outer: Extent,
    pub hole: Option<Extent>,
}

impl Shell {
    pub fn contains(self, delta: IVec3) -> bool {
        self.outer.contains(delta) && !self.hole.is_some_and(|hole| hole.contains(delta))
    }

    /// Every offset inside the shell, nearest first.
    pub fn offsets(self) -> Vec<IVec3> {
        let mut offsets = self.outer.offsets();
        offsets.retain(|&d| self.contains(d));
        offsets
    }
}

/// The four extents of one LOD level, in that level's own Cells.
///
/// Level 0 has no hole and is the same numbers as `Rings`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LevelRings {
    pub level: u32,
    pub mesh: Shell,
    pub keep: Shell,
    pub load: Extent,
    pub evict: Extent,
}

impl LevelRings {
    /// `cell_size_m` is the level-0 Cell, so a caller holding a `WorldSettings`
    /// passes `cell_size()` whatever the level.
    ///
    /// `level` is expected to be below [`level_count`]; a higher one still
    /// yields rings, clamped to the reach, that nothing should stream.
    pub fn new(settings: StreamingSettings, cell_size_m: f32, level: u32) -> Self {
        let cell_m = cell_size_m * scale(level);
        // Rounded up, so a configured distance is reached rather than
        // approached, and at least one Cell so a view distance below one Cell
        // still covers the Chunk the anchor is standing in.
        let cells = |metres: f32| ((metres / cell_m).ceil() as i32).max(1);
        let hysteresis = cells(settings.hysteresis_m * scale(level));

        let outer = outer_extent(settings, cell_size_m, level);
        let hole = (level > 0).then(|| hole(outer_extent(settings, cell_size_m, level - 1)));
        let keep_hole = hole
            .flatten()
            .and_then(|hole| shrunk_horizontally(hole, hysteresis));
        let keep = outer.grown(hysteresis);
        let load = keep.grown(1);

        LevelRings {
            level,
            mesh: Shell {
                outer,
                hole: hole.flatten(),
            },
            keep: Shell {
                outer: keep,
                hole: keep_hole,
            },
            load,
            evict: load.grown(hysteresis),
        }
    }
}

fn outer_extent(settings: StreamingSettings, cell_size_m: f32, level: u32) -> Extent {
    let cell_m = cell_size_m * scale(level);
    let cells = |metres: f32| ((metres / cell_m).ceil() as i32).max(1);
    // Level 0 draws the view distance whatever the reach, so `lod_distance_m`
    // below it means no coarse levels and not a smaller level 0.
    let reach_m = if level == 0 {
        level_distance_m(&settings, 0)
    } else {
        level_distance_m(&settings, level).min(settings.lod_distance_m)
    };
    Extent {
        horizontal: cells(reach_m),
        vertical: cells(settings.vertical_distance_m * scale(level)),
    }
}

/// The finer level's outer extent in this level's Cells, less one Cell on each
/// axis, so the two levels overlap by a Cell for the seam to hide behind. `None`
/// when that leaves nothing, as it does for a one-Cell view distance.
fn hole(finer_outer: Extent) -> Option<Extent> {
    // Half the finer extent, floored, since a Cell doubles.
    let halved = |cells: i32| (cells - 2).div_euclid(2);
    let hole = Extent {
        horizontal: halved(finer_outer.horizontal),
        vertical: halved(finer_outer.vertical),
    };
    (hole.horizontal >= 0 && hole.vertical >= 0).then_some(hole)
}

/// A negative radius would still pass `Extent::contains`, which squares it.
fn shrunk_horizontally(hole: Extent, cells: i32) -> Option<Extent> {
    let horizontal = hole.horizontal - cells;
    (horizontal >= 0).then_some(Extent { horizontal, ..hole })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::WorldSettings;

    fn world(voxel_size: f32) -> WorldSettings {
        WorldSettings {
            voxel_size,
            ..default()
        }
    }

    fn rings(voxel_size: f32, level: u32) -> LevelRings {
        LevelRings::new(
            StreamingSettings::default(),
            world(voxel_size).cell_size(),
            level,
        )
    }

    fn ext(horizontal: i32, vertical: i32) -> Extent {
        Extent {
            horizontal,
            vertical,
        }
    }

    /// The body `Rings::new` had before levels, kept as the reference.
    fn level_zero_reference(settings: StreamingSettings, cell_size_m: f32) -> [Extent; 4] {
        let cells = |metres: f32| ((metres / cell_size_m).ceil() as i32).max(1);
        let hysteresis = cells(settings.hysteresis_m);
        let mesh = ext(
            cells(settings.view_distance_m),
            cells(settings.vertical_distance_m),
        );
        let keep = mesh.grown(hysteresis);
        let load = keep.grown(1);
        [mesh, keep, load, load.grown(hysteresis)]
    }

    #[test]
    fn level_zero_is_the_rings_streaming_always_had() {
        let settings = [
            StreamingSettings::default(),
            StreamingSettings {
                view_distance_m: 1.0,
                vertical_distance_m: 1.0,
                hysteresis_m: 1.0,
                ..default()
            },
            StreamingSettings {
                view_distance_m: 33.0,
                vertical_distance_m: 100.0,
                hysteresis_m: 0.0,
                ..default()
            },
            StreamingSettings {
                view_distance_m: 1000.0,
                vertical_distance_m: 31.0,
                hysteresis_m: 65.0,
                lod_distance_m: 10.0,
            },
        ];
        for settings in settings {
            for cell_size_m in [32.0, 8.0, 16.0] {
                let r = LevelRings::new(settings, cell_size_m, 0);
                let [mesh, keep, load, evict] = level_zero_reference(settings, cell_size_m);
                let expected = LevelRings {
                    level: 0,
                    mesh: Shell {
                        outer: mesh,
                        hole: None,
                    },
                    keep: Shell {
                        outer: keep,
                        hole: None,
                    },
                    load,
                    evict,
                };
                assert_eq!(r, expected, "{settings:?} at {cell_size_m}");
            }
        }
    }

    #[test]
    fn the_defaults_give_six_levels_with_the_numbers_the_spec_derives() {
        assert_eq!(level_count(&StreamingSettings::default()), 6);

        let outer_horizontal = [6, 6, 6, 6, 6, 4];
        for (level, &horizontal) in outer_horizontal.iter().enumerate() {
            let r = rings(1.0, level as u32);
            assert_eq!(r.mesh.outer, ext(horizontal, 3), "level {level}");
            assert_eq!(r.keep.outer, ext(horizontal + 1, 4), "level {level}");
            assert_eq!(r.load, ext(horizontal + 2, 5), "level {level}");
            assert_eq!(r.evict, ext(horizontal + 3, 6), "level {level}");
            if level == 0 {
                assert_eq!(r.mesh.hole, None);
                assert_eq!(r.keep.hole, None);
            } else {
                assert_eq!(r.mesh.hole, Some(ext(2, 0)), "level {level}");
                assert_eq!(r.keep.hole, Some(ext(1, 0)), "level {level}");
            }
        }
    }

    #[test]
    fn the_hole_is_at_most_the_finer_level_in_metres_and_leaves_a_cell_of_overlap() {
        let metres = |level: u32, extent: Extent, voxel_size: f32| {
            let cell_m = world(voxel_size).cell_size() * scale(level);
            (
                extent.horizontal as f32 * cell_m,
                extent.vertical as f32 * cell_m,
            )
        };
        for voxel_size in [1.0, 0.25] {
            let cell_m = world(voxel_size).cell_size();
            for level in 1..level_count(&StreamingSettings::default()) {
                let finer = rings(voxel_size, level - 1).mesh.outer;
                let hole = rings(voxel_size, level).mesh.hole.expect("a hole");
                let (finer_h, finer_v) = metres(level - 1, finer, voxel_size);
                let (hole_h, hole_v) = metres(level, hole, voxel_size);
                let own_cell_m = cell_m * scale(level);
                assert!(hole_h + own_cell_m <= finer_h, "level {level} horizontal");
                assert!(hole_v + own_cell_m <= finer_v, "level {level} vertical");
            }
        }
    }

    #[test]
    fn voxel_size_changes_the_cell_count_and_not_the_distance() {
        let table: [(f32, u32, i32, i32); 6] = [
            // (voxel size, level, outer horizontal, vertical) in that level's Cells.
            (1.0, 0, 6, 3),
            (1.0, 3, 6, 3),
            (1.0, 5, 4, 3),
            (0.25, 0, 24, 12),
            (0.25, 3, 24, 12),
            (0.25, 5, 16, 12),
        ];
        for (voxel_size, level, horizontal, vertical) in table {
            let r = rings(voxel_size, level);
            assert_eq!(
                r.mesh.outer,
                ext(horizontal, vertical),
                "{voxel_size} {level}"
            );
            let cell_m = world(voxel_size).cell_size() * scale(level);
            let reach_m = horizontal as f32 * cell_m;
            let want_m = (192.0 * scale(level)).min(4096.0);
            assert_eq!(reach_m, want_m, "{voxel_size} {level}");
        }
        assert_eq!(rings(0.25, 3).mesh.hole, Some(ext(11, 5)));
    }

    #[test]
    fn a_reach_no_further_than_the_view_distance_is_one_level() {
        for lod_distance_m in [0.0, 100.0, 192.0] {
            let settings = StreamingSettings {
                lod_distance_m,
                ..default()
            };
            assert_eq!(level_count(&settings), 1, "{lod_distance_m}");
        }
        let just_past = StreamingSettings {
            lod_distance_m: 192.5,
            ..default()
        };
        assert_eq!(level_count(&just_past), 2);
    }

    #[test]
    fn a_shell_is_its_outer_extent_less_its_hole() {
        let r = rings(1.0, 1);
        let offsets = r.mesh.offsets();
        assert!(offsets.iter().all(|&d| r.mesh.contains(d)));
        assert!(!offsets.contains(&IVec3::ZERO));
        assert!(offsets.contains(&IVec3::new(3, 0, 0)));
        // The hole is a cylinder of radius 2 and half-height 0, so a Cell above
        // the anchor's row is outside it.
        assert!(offsets.contains(&IVec3::new(0, 1, 0)));
        assert!(!offsets.contains(&IVec3::new(2, 0, 0)));
        assert_eq!(
            offsets.len(),
            r.mesh.outer.offsets().len() - ext(2, 0).offsets().len()
        );
    }

    #[test]
    fn keep_releases_the_coarse_layer_as_the_eye_walks_onto_it() {
        let r = rings(1.0, 1);
        // Meshed at 2 Cells out, and past the shrunk keep hole at 2, but not at 1.
        assert!(r.mesh.contains(IVec3::new(3, 0, 0)));
        assert!(!r.mesh.contains(IVec3::new(2, 0, 0)));
        assert!(r.keep.contains(IVec3::new(2, 0, 0)));
        assert!(!r.keep.contains(IVec3::new(1, 0, 0)));
        // And vertically the keep hole is not shrunk.
        assert_eq!(
            r.keep.hole.map(|h| h.vertical),
            r.mesh.hole.map(|h| h.vertical)
        );
    }

    #[test]
    fn every_mesh_cell_is_kept_and_nothing_generated_has_a_hole() {
        for voxel_size in [1.0, 0.25] {
            for level in 0..level_count(&StreamingSettings::default()) {
                let r = rings(voxel_size, level);
                for d in r.mesh.offsets() {
                    assert!(r.keep.contains(d), "{voxel_size} {level} {d}");
                }
                for d in r.keep.offsets() {
                    assert!(r.load.contains(d), "{voxel_size} {level} {d}");
                }
                assert!(r.load.contains(IVec3::ZERO));
            }
        }
    }

    #[test]
    fn a_hole_that_would_be_empty_is_none() {
        let settings = StreamingSettings {
            view_distance_m: 32.0,
            vertical_distance_m: 32.0,
            hysteresis_m: 32.0,
            lod_distance_m: 128.0,
        };
        // Level 0 is one Cell out, so level 1's hole would have radius -1, and
        // `Extent::contains` squares its radius.
        let r = LevelRings::new(settings, 32.0, 1);
        assert_eq!(r.mesh.hole, None);
        assert!(r.mesh.contains(IVec3::ZERO));
    }

    #[test]
    fn the_anchor_floors_rather_than_truncating_below_the_origin() {
        let world = WorldSettings::default();
        let eye = Vec3::new(-1.0, 1.0, -65.0);
        assert_eq!(world.chunk_at_level(eye, 0), world.chunk_at(eye));
        // Level 1's Cell is 64 m: truncation would put the first in Chunk 0.
        assert_eq!(
            world.chunk_at_level(eye, 1),
            crate::world::ChunkCoord::new(-1, 0, -2)
        );
        assert_eq!(
            world.chunk_at_level(Vec3::new(63.9, 0.0, 64.0), 1),
            crate::world::ChunkCoord::new(0, 0, 1)
        );
        let fine = WorldSettings {
            voxel_size: 0.25,
            ..default()
        };
        // 0.25 m Voxels: level 2's Cell is 8 m * 4 = 32 m.
        assert_eq!(
            fine.chunk_at_level(Vec3::new(-0.1, 31.9, 32.0), 2),
            crate::world::ChunkCoord::new(-1, 0, 1)
        );
    }
}
