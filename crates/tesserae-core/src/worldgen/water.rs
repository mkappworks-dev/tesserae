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
}

impl Worldgen {
    /// The Flow over a column, in m/s along x and z: down a river or brook, or a lake's slow drift
    /// toward its outlet. Zero on dry ground and on the sea, whose motion is waves from Wind. Pure
    /// in the seed and metres, so the same at every Voxel size.
    pub fn flow_at(&self, x_m: f32, z_m: f32) -> [f32; 2] {
        let sea_m = self.water_level_of(None);
        match self.terrain.inland_water_m(x_m, z_m) {
            Some(level_m) if sea_m.is_some_and(|sea_m| level_m > sea_m) => {
                self.terrain.flow(x_m, z_m)
            }
            _ => [0.0; 2],
        }
    }

    /// The Fall over the column at global Voxel `[gx, gz]`, if it is a lip with a lower pool past
    /// it; of several, the longest drop, the first along [`CARDINALS`] on a tie. A lip is asked on
    /// each Voxel size's own lattice, so a Fall is pure in the seed, metres and Voxel size.
    pub fn fall_at(&self, gx: i32, gz: i32) -> Option<Fall> {
        let (x_m, z_m) = (self.global_m(gx), self.global_m(gz));
        if self.water.is_none() || self.surface_height_m(x_m, z_m) <= self.raw_column(x_m, z_m).0 {
            return None;
        }
        let surface =
            |dx: i32, dz: i32| self.water_surface_m(self.global_m(gx + dx), self.global_m(gz + dz));
        CARDINALS
            .iter()
            .filter_map(|&(dx, dz, _)| {
                let (from_m, to_m) = (surface(dx, dz)?, surface(-dx, -dz)?);
                (self.level_top(to_m) < self.level_top(from_m)).then_some(Fall {
                    toward: [-dx, -dz],
                    from_m,
                    to_m,
                })
            })
            .min_by(|a, b| (b.from_m - b.to_m).total_cmp(&(a.from_m - a.to_m)))
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
    const BOXES_M: [[f32; 2]; 3] = [[-5536.0, 512.0], [7680.0, -1504.0], [-5760.0, -1952.0]];

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
                        let (up, down) = (surface(gx + dx, gz + dz)?, surface(gx - dx, gz - dz)?);
                        below(up, down).then_some(Fall {
                            toward: [-dx, -dz],
                            from_m: up,
                            to_m: down,
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
            assert!(
                falls > 50 && dry_past > 20,
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
