//! The rings of one LOD level: the same four extents as [`Rings`], but the mesh
//! and keep extents are shells with a hole where the next finer level draws.
//!
//! Everything is derived from metres, so a Cell count never depends on the level except
//! where `lod_distance_m` clamps the outermost one. See
//! `docs/adr/0003-voxel-size-is-per-world.md`.
//!
//! [`Rings`]: super::streaming::Rings

use bevy::prelude::*;

use super::streaming::{Extent, StreamingSettings};

/// Past this the doubling would overflow a Cell count long before it was useful.
pub(super) const MAX_LEVELS: u32 = 32;

/// The smallest `n` with `view_distance_m * 2^(n-1) >= lod_distance_m`.
pub fn level_count(settings: &StreamingSettings) -> u32 {
    let mut levels = 1;
    while levels < MAX_LEVELS && level_distance_m(settings, levels - 1) < settings.lod_distance_m {
        levels += 1;
    }
    levels
}

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

    /// Relies on the hole lying inside the outer extent.
    #[cfg(any(test, feature = "dev", feature = "profiling"))]
    pub fn count(self) -> usize {
        self.outer.count() - self.hole.map_or(0, Extent::count)
    }

    pub fn offsets(self) -> Vec<IVec3> {
        let mut offsets = self.outer.offsets();
        offsets.retain(|&d| self.contains(d));
        offsets
    }
}

/// The four extents of one LOD level, in that level's own Cells.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LevelRings {
    pub level: u32,
    pub mesh: Shell,
    pub keep: Shell,
    pub load: Extent,
    pub evict: Extent,
}

impl LevelRings {
    /// `cell_size_m` is the level-0 Cell, whatever the level. A `level` at or above
    /// [`level_count`] yields rings clamped to the reach that nothing should stream.
    pub fn new(settings: StreamingSettings, cell_size_m: f32, level: u32) -> Self {
        let cell_m = cell_size_m * scale(level);
        // Rounded up so a configured distance is reached; at least one Cell so a tiny view
        // distance still covers the Chunk the anchor stands in.
        let cells = |metres: f32| ((metres / cell_m).ceil() as i32).max(1);
        let hysteresis = cells(settings.hysteresis_m * scale(level));

        let outer = outer_extent(settings, cell_size_m, level);
        let hole = (level > 0).then(|| hole(outer_extent(settings, cell_size_m, level - 1)));
        let keep_hole = hole.flatten().and_then(|hole| shrunk(hole, hysteresis));
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
    // `lod_distance_m` below the view distance means no coarse levels, not a smaller level 0.
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

/// What the finer level draws for any Eye position, less one Cell of overlap all round.
///
/// Both levels floor the same Eye to their own lattice, so the finer anchor is the coarse
/// anchor's first or second child on each axis. `None` when no cylinder fits.
fn hole(finer_outer: Extent) -> Option<Extent> {
    Some(Extent {
        horizontal: hole_radius(finer_outer.horizontal)?,
        vertical: hole_half_height(finer_outer.vertical)?,
    })
}

/// A Cell at offset `i` reaches `2i + 1` finer Cells out on its far side and the overlap
/// adds two; a finer anchor that is the near child on one axis and the far child on the
/// other makes `(2i + 3, 2j + 3)` the worst corner. Only the disc's outline is checked:
/// every Cell inside it has a smaller corner.
fn hole_radius(finer: i32) -> Option<i32> {
    let corner = |cells: i32| 2 * cells + 3;
    (0..=finer / 2).rev().find(|&radius| {
        (0..=radius).all(|i| {
            let j = (radius * radius - i * i).isqrt();
            corner(i).pow(2) + corner(j).pow(2) <= finer * finer
        })
    })
}

/// No vertical overlap, so a column is drawn by exactly one level and the coarse level
/// takes it over the moment the finer level's rows end.
fn hole_half_height(finer: i32) -> Option<i32> {
    let half = (finer - 1).div_euclid(2);
    (half >= 0).then_some(half)
}

/// A negative radius would still pass `Extent::contains`, which squares it, so it is `None`.
///
/// Shrunk on both axes so a coarse Chunk that has just fallen into the hole is held while
/// the finer level meshes it. The height stops at one row so the column the Eye walks
/// onto is still released.
fn shrunk(hole: Extent, cells: i32) -> Option<Extent> {
    let horizontal = hole.horizontal - cells;
    (horizontal >= 0).then_some(Extent {
        horizontal,
        vertical: (hole.vertical - cells).max(0),
    })
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

    /// Level 0 must stay the derivation `Rings::new` always used.
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
                assert_eq!(r.mesh.hole, Some(ext(1, 1)), "level {level}");
                assert_eq!(r.keep.hole, Some(ext(0, 0)), "level {level}");
            }
        }
    }

    /// Walks every Cell instead of using the corner shortcut `hole_radius` takes, so it
    /// checks that shortcut.
    fn fits(finer: i32, finer_rows: i32, radius: i32, half_height: i32) -> bool {
        let vertical = [0, 1]
            .into_iter()
            .all(|py| -2 * half_height >= py - finer_rows && 2 * half_height < py + finer_rows);
        let parities = [(0, 0), (0, 1), (1, 0), (1, 1)];
        vertical
            && parities.into_iter().all(|(px, pz)| {
                let in_disc = |x: i32, z: i32| (x - px).pow(2) + (z - pz).pow(2) <= finer * finer;
                (-radius..=radius).all(|i| {
                    (-radius..=radius)
                        .filter(|&j| i * i + j * j <= radius * radius)
                        .all(|j| {
                            (2 * i - 2..2 * i + 4)
                                .all(|x| (2 * j - 2..2 * j + 4).all(|z| in_disc(x, z)))
                        })
                })
            })
    }

    #[test]
    fn the_hole_leaves_a_cell_of_overlap_for_any_eye_position_and_no_more() {
        for horizontal in 1..=40 {
            for vertical in 1..=8 {
                let finer = ext(horizontal, vertical);
                let Some(hole) = hole(finer) else {
                    assert!(!fits(horizontal, vertical, 0, 0), "{finer:?}");
                    continue;
                };
                assert!(
                    fits(horizontal, vertical, hole.horizontal, hole.vertical),
                    "{finer:?} -> {hole:?} leaves less than a Cell"
                );
                // Maximal, so no more is drawn twice than the guarantee costs.
                assert!(
                    !fits(horizontal, vertical, hole.horizontal + 1, hole.vertical),
                    "{finer:?} -> {hole:?} could be wider"
                );
                assert!(
                    !fits(horizontal, vertical, hole.horizontal, hole.vertical + 1),
                    "{finer:?} -> {hole:?} could be taller"
                );
            }
        }
    }

    #[test]
    fn the_old_hole_left_no_overlap_when_the_anchors_differ() {
        // The spec's first arithmetic, `(finer - 2) / 2` on both axes.
        assert!(!fits(6, 3, 2, 0));
        assert!(fits(6, 3, 1, 1));
    }

    #[test]
    fn voxel_size_changes_the_cell_count_and_not_the_distance() {
        let table: [(f32, u32, i32, i32); 6] = [
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
        assert_eq!(rings(0.25, 3).mesh.hole, Some(ext(9, 5)));
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
        assert!(!offsets.contains(&IVec3::new(0, 1, 0)));
        assert!(!offsets.contains(&IVec3::new(1, -1, 0)));
        assert!(offsets.contains(&IVec3::new(0, 2, 0)));
        assert!(offsets.contains(&IVec3::new(1, 0, 1)));
        assert!(offsets.contains(&IVec3::new(2, 0, 0)));
        assert_eq!(
            offsets.len(),
            r.mesh.outer.offsets().len() - ext(1, 1).offsets().len()
        );
    }

    #[test]
    fn counting_a_shell_agrees_with_building_it() {
        for level in 0..level_count(&StreamingSettings::default()) {
            let r = rings(1.0, level);
            for shell in [
                r.mesh,
                r.keep,
                Shell {
                    outer: r.load,
                    hole: None,
                },
            ] {
                assert_eq!(shell.count(), shell.offsets().len(), "level {level}");
            }
            assert_eq!(r.evict.count(), r.evict.offsets().len(), "level {level}");
        }
    }

    #[test]
    fn keep_releases_the_coarse_layer_as_the_eye_walks_onto_it() {
        let r = rings(1.0, 1);
        assert!(r.mesh.contains(IVec3::new(2, 0, 0)));
        assert!(!r.mesh.contains(IVec3::new(1, 0, 0)));
        assert!(r.keep.contains(IVec3::new(1, 0, 0)));
        assert!(!r.keep.contains(IVec3::ZERO));
        assert_eq!(
            r.keep.hole.map(|h| h.vertical),
            r.mesh.hole.map(|h| h.vertical - 1)
        );
    }

    /// Releasing a Chunk the finer level has just taken over would leave a block drawn by
    /// neither, as the finer level has had no tick to mesh it.
    #[test]
    fn a_chunk_the_finer_level_takes_over_is_kept_for_a_step_after() {
        for voxel_size in [1.0, 0.25] {
            for level in 1..level_count(&StreamingSettings::default()) {
                let r = rings(voxel_size, level);
                // Axis steps only: landing on the anchor Cell diagonally is released on purpose.
                for step in [
                    IVec3::X,
                    IVec3::NEG_X,
                    IVec3::Y,
                    IVec3::NEG_Y,
                    IVec3::Z,
                    IVec3::NEG_Z,
                ] {
                    for offset in r.mesh.offsets() {
                        let now = offset - step;
                        let in_hole = r.mesh.hole.is_some_and(|hole| hole.contains(now));
                        assert!(
                            !in_hole || r.keep.contains(now),
                            "{voxel_size} level {level}: {offset} released after a step of {step}"
                        );
                    }
                }
            }
        }
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
        let r = LevelRings::new(settings, 32.0, 1);
        assert_eq!(r.mesh.hole, None);
        assert!(r.mesh.contains(IVec3::ZERO));
    }

    #[test]
    fn the_anchor_floors_rather_than_truncating_below_the_origin() {
        let world = WorldSettings::default();
        let eye = Vec3::new(-1.0, 1.0, -65.0);
        assert_eq!(world.chunk_at_level(eye, 0), world.chunk_at(eye));
        // Truncation would put the first in Chunk 0.
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
        assert_eq!(
            fine.chunk_at_level(Vec3::new(-0.1, 31.9, 32.0), 2),
            crate::world::ChunkCoord::new(-1, 0, 1)
        );
    }
}
