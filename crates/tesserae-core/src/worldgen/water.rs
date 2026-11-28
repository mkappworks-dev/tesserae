//! What the World answers of how its generated water moves: each column's Flow and where each Fall
//! is. Both are seen and move nothing (ADR-0038), and neither changes the terrain.

use super::{CARDINALS, Worldgen};

/// Water running from a pool over a lip, the dry column whose ground rose to hold that pool, down
/// to the lower pool past it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fall {
    /// The step, one column along x or z, from the upper pool across the lip toward the lower.
    pub toward: [i32; 2],
    /// The upper pool's Water surface, in metres.
    pub from_m: f32,
    /// The lower pool's Water surface, in metres.
    pub to_m: f32,
    /// How many columns past the lip the lower pool starts: a tall Fall's face is a short steep
    /// slope, and the dry columns between are ledges its water runs down.
    pub reach: u8,
}

/// How far past its lip a Fall's lower pool may lie: a face is 3.5 in 1, so this holds a drop
/// of some 40 m.
pub const FALL_REACH_M: f32 = 12.0;

/// [`FALL_REACH_M`] in columns of `voxel_size`.
pub fn fall_reach(voxel_size: f32) -> i32 {
    (FALL_REACH_M / voxel_size).ceil() as i32
}

impl Worldgen {
    /// The Flow over a column, in m/s along x and z: down a river or brook, or a lake's slow drift
    /// toward its outlet. Zero on dry ground and on the sea, whose motion is waves from Wind. Pure
    /// in the seed and metres, so the same at every Voxel size.
    pub fn flow_at(&self, x_m: f32, z_m: f32) -> [f32; 2] {
        self.flow_over(self.water_level_m(x_m, z_m), [x_m, z_m])
    }

    /// [`Self::flow_at`] of a column whose water level is `level_m`.
    fn flow_over(&self, level_m: Option<f32>, [x_m, z_m]: [f32; 2]) -> [f32; 2] {
        match (level_m, self.water_level_of(None)) {
            (Some(level_m), Some(sea_m)) if level_m > sea_m => self.terrain.flow(x_m, z_m),
            _ => [0.0; 2],
        }
    }

    /// [`Self::water_surface_m`] over global column `[gx, gz]`, read from its Cell's cache when a
    /// Chunk has built it.
    fn surface_over(&self, gx: i32, gz: i32) -> Option<f32> {
        self.wet_m(None, [self.global_m(gx), self.global_m(gz)])
    }

    /// The Fall over the column at global Voxel `[gx, gz]`, if it is a lip with a lower pool past
    /// it; of several, the longest drop, the first along `CARDINALS` on a tie. A lip is asked on
    /// each Voxel size's own lattice, so a Fall is pure in the seed, metres and Voxel size.
    pub fn fall_at(&self, gx: i32, gz: i32) -> Option<Fall> {
        self.water?;
        let surface = |dx: i32, dz: i32| self.surface_over(gx + dx, gz + dz);
        let fall = CARDINALS
            .iter()
            .filter_map(|&(dx, dz, _)| {
                let from_m = surface(dx, dz)?;
                let (reach, to_m) = self.lower_pool([gx, gz], [dx, dz], from_m)?;
                Some(Fall {
                    toward: [-dx, -dz],
                    from_m,
                    to_m,
                    reach: reach as u8,
                })
            })
            .min_by(|a, b| (b.from_m - b.to_m).total_cmp(&(a.from_m - a.to_m)))?;
        // Asked last: the raw column is a drainage walk, and most columns have no pools across.
        let (x_m, z_m) = (self.global_m(gx), self.global_m(gz));
        (self.surface_height_m(x_m, z_m) > self.raw_column(x_m, z_m).0).then_some(fall)
    }
}

impl Worldgen {
    /// The pool a Fall over lip `[gx, gz]` lands in, its upper pool `step` away at `from_m`:
    /// how many columns out and its surface. A tall Fall's face is a short steep slope, dry below
    /// the lip, so the pool lies a few columns out, and a stream that crosses the edge at a slant
    /// lands to one side of straight on: the first water along the step's line and the two
    /// diagonals beside it, nearest first, whose surface is a Voxel or more below.
    pub(super) fn lower_pool(
        &self,
        [gx, gz]: [i32; 2],
        [dx, dz]: [i32; 2],
        from_m: f32,
    ) -> Option<(i32, f32)> {
        let (px, pz) = (dz, dx);
        let reach = fall_reach(self.voxel_size);
        [0, 1, -1]
            .into_iter()
            .filter_map(|side| {
                (1..=reach).find_map(|k| {
                    let (x, z) = (gx - k * dx + side * k * px, gz - k * dz + side * k * pz);
                    Some((k, self.surface_over(x, z)?))
                })
            })
            .filter(|&(_, to_m)| self.level_top(to_m) < self.level_top(from_m))
            .min_by_key(|&(k, _)| k)
    }
}

impl crate::water_columns::WaterAnswers for Worldgen {
    fn voxel_size(&self) -> f32 {
        self.voxel_size
    }

    fn water_material(&self) -> Option<crate::MaterialId> {
        self.water.map(|w| w.material)
    }

    fn water_top(&self, [gx, gz]: [i32; 2]) -> Option<(f32, i32)> {
        let level_m = self.surface_over(gx, gz)?;
        Some((level_m, self.level_top(level_m)))
    }

    fn ground_m(&self, [gx, gz]: [i32; 2]) -> f32 {
        self.surface_height_m(self.global_m(gx), self.global_m(gz))
    }

    fn flow(&self, [gx, gz]: [i32; 2]) -> [f32; 2] {
        let at = [self.global_m(gx), self.global_m(gz)];
        self.flow_over(self.level_m(None, at), at)
    }

    fn fall(&self, [gx, gz]: [i32; 2]) -> Option<Fall> {
        self.fall_at(gx, gz)
    }

    fn fall_toward(&self, [gx, gz]: [i32; 2], [tx, tz]: [i32; 2]) -> Option<Fall> {
        let from_m = self.surface_over(gx - tx, gz - tz)?;
        let (reach, to_m) = self.lower_pool([gx, gz], [-tx, -tz], from_m)?;
        Some(Fall {
            toward: [tx, tz],
            from_m,
            to_m,
            reach: reach as u8,
        })
    }

    fn sea_level_m(&self) -> Option<f32> {
        self.water_level_of(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::CHUNK_SIZE;
    use crate::worldgen::{WaterParams, WorldgenParams};

    const SEED: u64 = 0x7e55_e2ae_0000_0001;

    fn wet(voxel_size: f32) -> Worldgen {
        let params = crate::testgen::world_params();
        Worldgen::new(
            SEED,
            voxel_size,
            WorldgenParams {
                water: Some(WaterParams {
                    level_m: params.sea_level_m,
                    material: crate::testgen::fixtures().water,
                }),
                sea_level_m: 24.0,
                relief_m: 24.0,
                landforms: crate::terrain::LandformParams {
                    depth_m: 100.0,
                    height_m: 200.0,
                    ..crate::terrain::LandformParams::default()
                },
                ..params
            },
        )
    }

    /// The 1 m Chunks, as metre boxes, over steep brooks: one stepping down in Falls, one whose
    /// lips stand beside dry ground, and one of both.
    const BOXES_M: [[f32; 2]; 3] = [[-5568.0, 576.0], [4608.0, 5088.0], [-5760.0, -1952.0]];

    /// Every global column of a box at `voxel_size`.
    fn columns(voxel_size: f32) -> impl Iterator<Item = [i32; 2]> {
        let side = (CHUNK_SIZE as f32 / voxel_size) as i32;
        BOXES_M.into_iter().flat_map(move |[x_m, z_m]| {
            let (gx, gz) = ((x_m / voxel_size) as i32, (z_m / voxel_size) as i32);
            (0..side).flat_map(move |z| (0..side).map(move |x| [gx + x, gz + z]))
        })
    }

    fn falls(g: &Worldgen, size: f32) -> Vec<Option<Fall>> {
        columns(size).map(|[gx, gz]| g.fall_at(gx, gz)).collect()
    }

    /// A level-1 water column, over the centre of a 2 m cell, against the level-0 columns it
    /// covers: the generated rule's depth, from the drawn top to the ground.
    #[test]
    fn a_coarse_columns_depth_agrees_with_level_zeros_within_a_coarse_voxel() {
        use crate::water_columns::{WaterAnswers, coarse_water_columns, water_index};
        let fine = wet(1.0);
        let coarse_m = 2.0;
        let origin = [-8256.0f32, 2464.0];
        let coarse = coarse_water_columns(
            origin,
            coarse_m,
            |x, z| fine.water_level_m(x, z),
            |x, z| fine.surface_height_m(x, z),
            WaterAnswers::sea_level_m(&fine),
        );
        assert!(!coarse.is_empty(), "the coast fixture's sea");
        let mut compared = 0;
        for z in 0..CHUNK_SIZE as i32 {
            for x in 0..CHUNK_SIZE as i32 {
                let column = coarse[water_index(x, z)];
                if !column.is_wet() || column.depth == u8::MAX {
                    continue;
                }
                for (dx, dz) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let g = [origin[0] as i32 + 2 * x + dx, origin[1] as i32 + 2 * z + dz];
                    let Some((_, top)) = WaterAnswers::water_top(&fine, g) else {
                        continue;
                    };
                    let fine_m = (top + 1) as f32 - WaterAnswers::ground_m(&fine, g);
                    assert!(
                        (column.depth_m() - fine_m).abs() <= coarse_m,
                        "{g:?}: {} against {fine_m}",
                        column.depth_m()
                    );
                    compared += 1;
                }
            }
        }
        assert!(compared > 100, "{compared} columns compared");
    }

    /// How many columns the containment raises over a box's ground, how many by more than
    /// 3 m, and the most it raises one.
    fn walls(g: &Worldgen, size: f32) -> (u32, u32, f32) {
        let (mut raised, mut tall, mut most) = (0, 0, 0.0f32);
        for [gx, gz] in big_columns(size) {
            let (x_m, z_m) = (g.global_m(gx), g.global_m(gz));
            let up = g.surface_height_m(x_m, z_m) - g.raw_column(x_m, z_m).0;
            if up > 0.0 {
                raised += 1;
                tall += (up > 3.0) as u32;
                most = most.max(up);
            }
        }
        (raised, tall, most)
    }

    /// Every column of a wider square round each box.
    fn big_columns(voxel_size: f32) -> impl Iterator<Item = [i32; 2]> {
        let side = (256.0 / voxel_size) as i32;
        BOXES_M.into_iter().flat_map(move |[x_m, z_m]| {
            let (gx, gz) = (
                ((x_m - 112.0) / voxel_size) as i32,
                ((z_m - 112.0) / voxel_size) as i32,
            );
            (0..side).flat_map(move |z| (0..side).map(move |x| [gx + x, gz + z]))
        })
    }

    /// Generated water stands only as deep over the ground as its own channel, so the rock the
    /// containment raises to hold it is low: of the columns it raised round these boxes, 351 of
    /// 645 rose more than 3 m before, 69 of 391 after; what is left is lakes whose rim the 32 m
    /// lattice saw whole.
    #[test]
    fn the_containment_raises_little_rock_round_generated_water() {
        let (raised, tall, _) = walls(&wet(1.0), 1.0);
        assert!(raised > 100, "{raised} columns raised");
        assert!(
            tall * 4 <= raised,
            "{tall} of {raised} raised more than 3 m"
        );
    }

    /// A stream that crosses the edge at a slant pours along all of its lip: 192 Fall columns
    /// round these boxes when a lip looked straight on alone, 249 with the diagonals.
    #[test]
    fn a_slanting_stream_pours_along_its_lip() {
        let g = wet(1.0);
        let falls = big_columns(1.0)
            .filter(|&[x, z]| g.fall_at(x, z).is_some())
            .count();
        assert!(falls >= 240, "{falls} Fall columns");
    }

    #[test]
    fn a_fall_runs_over_every_lip_with_a_lower_pool_past_it_and_nowhere_else() {
        for size in [1.0, 0.5] {
            let g = wet(size);
            let surface = |gx: i32, gz: i32| g.water_surface_m(g.global_m(gx), g.global_m(gz));
            let (mut falls, mut dry_past) = (0, 0);
            for [gx, gz] in columns(size) {
                let (x_m, z_m) = (g.global_m(gx), g.global_m(gz));
                let lip = g.surface_height_m(x_m, z_m) > g.raw_column(x_m, z_m).0;
                let below = |up: f32, down: f32| g.level_top(down) < g.level_top(up);
                let over: Vec<Fall> = CARDINALS
                    .iter()
                    .filter_map(|&(dx, dz, _)| {
                        let up = surface(gx + dx, gz + dz)?;
                        let (reach, down) = g.lower_pool([gx, gz], [dx, dz], up)?;
                        below(up, down).then_some(Fall {
                            toward: [-dx, -dz],
                            from_m: up,
                            to_m: down,
                            reach: reach as u8,
                        })
                    })
                    .collect();
                let fall = g.fall_at(gx, gz);
                if !lip || over.is_empty() {
                    assert_eq!(fall, None, "a Fall at {gx},{gz}, {size} m");
                    dry_past += usize::from(lip);
                    continue;
                }
                let fall = fall.unwrap_or_else(|| panic!("no Fall over the lip at {gx},{gz}"));
                assert!(over.contains(&fall), "{fall:?} at {gx},{gz}");
                assert_eq!(surface(gx, gz), None, "the lip at {gx},{gz} is wet");
                falls += 1;
            }
            // A steep reach drops at one tall Fall rather than a Voxel at a time, so the boxes
            // hold few.
            assert!(
                falls > 10 && dry_past > 10,
                "{falls} Falls and {dry_past} lips beside dry ground at {size} m"
            );
        }
    }

    #[test]
    fn falls_are_the_same_in_any_chunk_order_and_with_a_cold_cache() {
        for size in [1.0, 0.5] {
            let asked = falls(&wet(size), size);
            let g = wet(size);
            let side = (CHUNK_SIZE as f32 * size) as i32;
            for [x_m, z_m] in BOXES_M.into_iter().rev() {
                let (cx, cz) = ((x_m / side as f32) as i32, (z_m / side as f32) as i32);
                for (dx, dz) in [(1, 1), (0, 1), (1, 0), (0, 0), (-1, 0), (0, -1)] {
                    let y_m = g.surface_height_m(x_m, z_m);
                    let cy = (y_m / side as f32).floor() as i32;
                    g.chunk(cx + dx, cy, cz + dz);
                }
            }
            assert_eq!(falls(&g, size), asked, "after Chunks, {size} m");
            g.forget_caches();
            assert_eq!(falls(&g, size), asked, "cold, {size} m");
        }
    }
}
