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
pub(super) const MAX_LEVELS: u32 = 32;

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

    /// How many Cells are inside, without building them. The hole lies inside the
    /// outer extent.
    #[cfg(any(test, feature = "dev"))]
    pub fn count(self) -> usize {
        self.outer.count() - self.hole.map_or(0, Extent::count)
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

/// The cylinder this level does not mesh, in its own Cells: what the finer level
/// draws for any Eye position, less one Cell of overlap all round.
///
/// Both levels floor the same Eye to their own lattice, so the finer anchor is
/// the coarse anchor's first or second child on each axis and the two discs
/// differ by up to a finer Cell. `None` when no cylinder fits, as for a
/// one-Cell view distance.
fn hole(finer_outer: Extent) -> Option<Extent> {
    Some(Extent {
        horizontal: hole_radius(finer_outer.horizontal)?,
        vertical: hole_half_height(finer_outer.vertical)?,
    })
}

/// The largest radius whose Cells, grown by one of this level's Cells, all lie in
/// a finer disc of `finer` Cells for either anchor parity.
///
/// A Cell at offset `i` reaches `2i + 1` finer Cells out on its far side, the
/// overlap adds two, and a finer anchor that is the near child on one axis and
/// the far child on the other makes `(2i + 3, 2j + 3)` the worst corner of the
/// quadrant. Only the outline of the disc is checked: every Cell inside it has a
/// smaller corner.
fn hole_radius(finer: i32) -> Option<i32> {
    let corner = |cells: i32| 2 * cells + 3;
    (0..=finer / 2).rev().find(|&radius| {
        (0..=radius).all(|i| {
            let j = (radius * radius - i * i).isqrt();
            corner(i).pow(2) + corner(j).pow(2) <= finer * finer
        })
    })
}

/// The largest half-height whose slab lies in the finer level's rows for either
/// anchor parity. No vertical overlap, so a column is drawn by exactly one level
/// where the hole is, and the coarse level takes it over the moment the finer
/// level's rows end.
fn hole_half_height(finer: i32) -> Option<i32> {
    let half = (finer - 1).div_euclid(2);
    (half >= 0).then_some(half)
}

/// The cylinder around the Eye, in metres, inside which a level's terrain is not
/// drawn because the next finer level covers it.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Cut {
    pub radius_m: f32,
    pub half_height_m: f32,
}

/// Per level, what to cut. Level 0 draws from the Eye outward, so its entry is
/// empty.
///
/// Level `k`'s Chunks lie under the finer level for the whole width of the hole's
/// parity slack and more, and a coarse surface up to one coarse Voxel above the
/// real one would stand above the finer silhouette there. The Client cuts them
/// in metres instead. The finer level reaches `R - sqrt(2)` of its Cells
/// horizontally and `V` vertically from any Eye position, so nothing inside that
/// is left uncovered, and one coarse Voxel less on each keeps the coarse surface
/// behind the finer level's boundary faces. The height matters: above the finer
/// level's rows nothing covers the ground, and cutting there shows the sky.
pub fn cuts(settings: &StreamingSettings, cell_size_m: f32) -> Vec<Cut> {
    (0..level_count(settings))
        .map(|level| {
            if level == 0 {
                return Cut::default();
            }
            let finer = outer_extent(*settings, cell_size_m, level - 1);
            let finer_cell_m = cell_size_m * scale(level - 1);
            let voxel_m = cell_size_m * scale(level) / tesserae_core::CHUNK_SIZE as f32;
            Cut {
                radius_m: ((finer.horizontal as f32 - std::f32::consts::SQRT_2) * finer_cell_m
                    - voxel_m)
                    .max(0.0),
                half_height_m: (finer.vertical as f32 * finer_cell_m - voxel_m).max(0.0),
            }
        })
        .collect()
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
                assert_eq!(r.mesh.hole, Some(ext(1, 1)), "level {level}");
                assert_eq!(r.keep.hole, Some(ext(0, 1)), "level {level}");
            }
        }
    }

    /// Whether a hole of `radius` and `half_height` Cells fits inside a finer
    /// disc of `finer` Cells and `finer_rows` rows for every Eye position, by
    /// walking every Cell of both. Written without the corner shortcut
    /// `hole_radius` takes, so it checks that one.
    fn fits(finer: i32, finer_rows: i32, radius: i32, half_height: i32) -> bool {
        // Both anchors floor the same Eye, so the finer one is the coarse
        // anchor's first or second child on each axis.
        let vertical = [0, 1].into_iter().all(|py| {
            // Rows `-2h ..= 2h + 1` against `py - rows ..= py + rows`.
            -2 * half_height >= py - finer_rows && 2 * half_height < py + finer_rows
        });
        let parities = [(0, 0), (0, 1), (1, 0), (1, 1)];
        vertical
            && parities.into_iter().all(|(px, pz)| {
                let in_disc = |x: i32, z: i32| (x - px).pow(2) + (z - pz).pow(2) <= finer * finer;
                (-radius..=radius).all(|i| {
                    (-radius..=radius)
                        .filter(|&j| i * i + j * j <= radius * radius)
                        .all(|j| {
                            // The Cell's four finer Cells, grown by two on each
                            // side: one of this level's Cells all round.
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
                    // Nothing fits: not even a single Cell.
                    assert!(!fits(horizontal, vertical, 0, 0), "{finer:?}");
                    continue;
                };
                assert!(
                    fits(horizontal, vertical, hole.horizontal, hole.vertical),
                    "{finer:?} -> {hole:?} leaves less than a Cell"
                );
                // Maximal on each axis, so no more of the finer level is
                // drawn twice than the guarantee costs.
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
        // The spec's first arithmetic, `(finer - 2) / 2` on both axes: at the
        // defaults a radius of 2 against 6 Cells.
        assert!(!fits(6, 3, 2, 0));
        assert!(fits(6, 3, 1, 1));
    }

    /// Every point inside a level's cut is in a Cell the finer level meshes, for
    /// every position of the Eye in its anchor Cell: cutting a coarse level there
    /// can never show the sky.
    #[test]
    fn what_a_coarse_level_leaves_out_is_always_covered_by_the_finer_one() {
        for voxel_size in [1.0, 0.25] {
            let cell_m = world(voxel_size).cell_size();
            let settings = StreamingSettings::default();
            let cuts = cuts(&settings, cell_m);
            assert_eq!(cuts.len() as u32, level_count(&settings));
            assert_eq!(cuts[0], Cut::default());
            for (level, &cut) in cuts.iter().enumerate().skip(1) {
                let finer = rings(voxel_size, level as u32 - 1).mesh.outer;
                let c = cell_m * scale(level as u32 - 1);
                assert!(
                    cut.radius_m > 0.0 && cut.half_height_m > 0.0,
                    "level {level}"
                );
                let fractions = [0.0, 0.3, 0.5, 0.999];
                for (fx, fy, fz) in fractions
                    .into_iter()
                    .flat_map(|x| fractions.into_iter().map(move |y| (x, y)))
                    .flat_map(|(x, y)| fractions.into_iter().map(move |z| (x, y, z)))
                {
                    let eye = Vec3::new(fx, fy, fz) * c;
                    let steps = (cut.radius_m / (c / 8.0)).ceil() as i32;
                    for step in 0..=steps {
                        let r = (step as f32 / steps as f32) * cut.radius_m;
                        for degrees in (0..360).step_by(6) {
                            let (sin, cos) = (degrees as f32).to_radians().sin_cos();
                            for dy in [-cut.half_height_m, 0.0, cut.half_height_m] {
                                let at = eye + Vec3::new(cos * r, dy, sin * r);
                                let offset = (at / c).floor().as_ivec3();
                                assert!(
                                    finer.contains(offset),
                                    "{voxel_size} level {level}: {at} is not in level {}",
                                    level - 1
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn the_cut_is_a_coarse_voxel_inside_what_the_finer_level_guarantees() {
        // R = 6 finer Cells of 32 m across and 3 high: 6 - sqrt(2) of them, and 3,
        // each less a Voxel of 2 m.
        let cuts = cuts(&StreamingSettings::default(), 32.0);
        assert!((cuts[1].radius_m - ((6.0 - 2f32.sqrt()) * 32.0 - 2.0)).abs() < 1e-3);
        assert!((cuts[1].half_height_m - (3.0 * 32.0 - 2.0)).abs() < 1e-3);
        assert_eq!(cuts.len(), 6);
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
        // The hole is a cylinder of radius 1 and half-height 1: the anchor and
        // its four neighbours in the row above and below are inside it.
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
        // One Cell out is in the mesh hole and kept; the anchor's own Cell is
        // in the keep hole too.
        assert!(r.mesh.contains(IVec3::new(2, 0, 0)));
        assert!(!r.mesh.contains(IVec3::new(1, 0, 0)));
        assert!(r.keep.contains(IVec3::new(1, 0, 0)));
        assert!(!r.keep.contains(IVec3::ZERO));
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
        // Level 0 is one Cell out and no hole fits in it; a negative radius
        // would pass `Extent::contains`, which squares it.
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
