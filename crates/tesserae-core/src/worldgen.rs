//! World generation: a Chunk's contents from a seed and a position.
//!
//! Every spatial parameter here is in **metres**, and `voxel_size` is the only
//! place a metre becomes a Voxel index. Tune one of these numbers in Voxel
//! units and Voxel size is permanently fixed, because every number in the game
//! becomes secretly calibrated to it — the one decision in
//! `docs/adr/0003-voxel-size-is-per-world.md` that cannot be repaired later.
//!
//! Contrast `crate::testgen`, which is tuned in Voxel indices on purpose: it
//! feeds the benches a fixed surface and makes no World.

use crate::chunk::{CHUNK_SIZE, Chunk};
use crate::noise::fbm2;
use crate::voxel::{MaterialId, Voxel};

/// Placeholder Materials, until a Mod declares the table in build-order step 7.
/// They live here rather than in `testgen` because generation is what assigns
/// them; `testgen` re-exports these so a fixture and a World agree on what
/// stone is.
pub const STONE: MaterialId = 1;
pub const DIRT: MaterialId = 2;
pub const GRASS: MaterialId = 3;

/// What a Mod parameterises. Plain `Copy` data with no callbacks, because
/// crossing the WASM boundary per Voxel is the mistake
/// `docs/adr/0001-wasm-mod-boundary.md` exists to prevent: a Mod fills this in
/// once and generation stays native.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct WorldgenParams {
    /// The height the surface falls to where the noise bottoms out.
    pub sea_level_m: f32,
    /// How far above `sea_level_m` the surface can reach.
    pub relief_m: f32,
    /// The horizontal period of the lowest octave: how far apart two hilltops
    /// are, in metres. A frequency expressed as a distance, so that halving
    /// Voxel size refines the same terrain instead of shrinking it.
    pub feature_size_m: f32,
    /// How deep the soil runs below the surface. In metres, so it is the same
    /// physical layer whatever a Voxel is worth — four Voxels of dirt at 1m is
    /// sixteen at 0.25m, and both are four metres of dirt.
    pub soil_depth_m: f32,
    pub octaves: u32,
}

impl Default for WorldgenParams {
    fn default() -> Self {
        WorldgenParams {
            sea_level_m: 24.0,
            relief_m: 28.0,
            feature_size_m: 112.0,
            soil_depth_m: 3.0,
            octaves: 4,
        }
    }
}

/// One World's generator: a seed, a Voxel size and the parameters a Mod chose.
#[derive(Clone, Debug)]
pub struct Worldgen {
    voxel_size: f32,
    params: WorldgenParams,
    /// The noise lattice is seeded with a `u32`; a World's seed is a `u64`, so
    /// fold rather than truncate and two seeds differing only above bit 32
    /// still make different worlds.
    seed: u32,
    /// Hoisted out of the inner loop: `chunk` samples this per column.
    inv_feature_size: f32,
}

impl Worldgen {
    pub fn new(seed: u64, voxel_size: f32, params: WorldgenParams) -> Self {
        debug_assert!(voxel_size > 0.0, "a Voxel has an edge length");
        debug_assert!(params.feature_size_m > 0.0, "features have a size");
        Worldgen {
            voxel_size,
            params,
            seed: (seed ^ (seed >> 32)) as u32,
            inv_feature_size: 1.0 / params.feature_size_m,
        }
    }

    pub fn params(&self) -> WorldgenParams {
        self.params
    }

    /// The metre height of the surface over a point on the ground plane.
    pub fn surface_height_m(&self, x_m: f32, z_m: f32) -> f32 {
        let n = fbm2(
            x_m * self.inv_feature_size,
            z_m * self.inv_feature_size,
            self.seed,
            self.params.octaves,
        );
        self.params.sea_level_m + n * self.params.relief_m
    }

    /// The band `surface_height_m` is confined to, as `(lowest, highest)` in
    /// metres. Sound because [`fbm2`] is normalised to `[0, 1)`; the two
    /// early-outs in [`Worldgen::chunk`] and the Server's vertical streaming
    /// extent are both derived from it.
    pub fn surface_bounds_m(&self) -> (f32, f32) {
        (
            self.params.sea_level_m,
            self.params.sea_level_m + self.params.relief_m,
        )
    }

    /// A Chunk of terrain at Chunk coordinates `(cx, cy, cz)`.
    pub fn chunk(&self, cx: i32, cy: i32, cz: i32) -> Chunk {
        let size = CHUNK_SIZE as i32;
        let (lowest, highest) = self.surface_bounds_m();

        // The metre heights of this Chunk's bottom and top Voxels.
        let floor_m = (cy * size) as f32 * self.voxel_size;
        let ceiling_m = floor_m + (CHUNK_SIZE - 1) as f32 * self.voxel_size;

        // Most Chunks in a streamed ring answer here without sampling noise at
        // all, which is what makes a vertical extent affordable rather than a
        // micro-optimisation: a 3D ring is mostly sky and mostly rock.
        if floor_m > highest {
            return Chunk::air();
        }
        if ceiling_m < lowest - self.params.soil_depth_m {
            return Chunk::filled(Voxel::cube(STONE));
        }

        let mut chunk = Chunk::air();
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let x_m = (cx * size + x as i32) as f32 * self.voxel_size;
                let z_m = (cz * size + z as i32) as f32 * self.voxel_size;
                let surface_m = self.surface_height_m(x_m, z_m);

                for y in 0..CHUNK_SIZE {
                    let y_m = floor_m + y as f32 * self.voxel_size;
                    let below_m = surface_m - y_m;
                    if below_m < 0.0 {
                        // y_m only rises, so the rest of the column is sky.
                        break;
                    }
                    let material = if below_m < self.voxel_size {
                        GRASS
                    } else if below_m < self.params.soil_depth_m {
                        DIRT
                    } else {
                        STONE
                    };
                    chunk.set(x, y, z, Voxel::cube(material));
                }
            }
        }
        chunk
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEED: u64 = 0x7e55_e2ae;

    fn generator(voxel_size: f32) -> Worldgen {
        Worldgen::new(SEED, voxel_size, WorldgenParams::default())
    }

    /// One column of Voxels from y=0 metres up past the top of the surface
    /// band, bottom-up. The band spans more than one Chunk at every Voxel size,
    /// so a test that looks at a single Chunk is testing whichever slice of rock
    /// or sky it happened to land in.
    fn column(g: &Worldgen, x: usize, z: usize) -> Vec<Voxel> {
        let (_, highest) = g.surface_bounds_m();
        let cell_m = CHUNK_SIZE as f32 * g.voxel_size;
        let top_cy = (highest / cell_m).ceil() as i32;
        (0..=top_cy)
            .flat_map(|cy| {
                let chunk = g.chunk(0, cy, 0);
                (0..CHUNK_SIZE)
                    .map(move |y| chunk.get(x, y, z))
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    fn highest_solid(column: &[Voxel]) -> Option<usize> {
        column.iter().rposition(|v| !v.is_air())
    }

    #[test]
    fn surface_is_the_same_metre_height_at_any_voxel_size() {
        // The test ADR-0003 exists for. If a frequency, a sea level or a relief
        // were ever retuned in Voxel units, this is what would catch it.
        let coarse = generator(1.0);
        let fine = generator(0.25);
        for i in 0..200 {
            let (x_m, z_m) = (i as f32 * 3.7 - 300.0, i as f32 * -2.3 + 120.0);
            assert_eq!(
                coarse.surface_height_m(x_m, z_m),
                fine.surface_height_m(x_m, z_m),
                "at {x_m}m, {z_m}m"
            );
        }
    }

    #[test]
    fn the_generated_surface_lands_at_the_same_metre_height_at_any_voxel_size() {
        let coarse = generator(1.0);
        let fine = generator(0.5);
        // Kept below half of CHUNK_SIZE so that doubling the Voxel index stays
        // inside Chunk (0, cy, 0) instead of crossing into its neighbour.
        for (x, z) in [(0, 0), (7, 3), (15, 15)] {
            let coarse_top = highest_solid(&column(&coarse, x, z)).expect("a surface");
            // At half the Voxel size two Voxels span one of the coarse ones, so
            // the same metre column is Voxel 2x, 2z.
            let fine_top = highest_solid(&column(&fine, 2 * x, 2 * z)).expect("a surface");
            let coarse_m = coarse_top as f32 * 1.0;
            let fine_m = fine_top as f32 * 0.5;
            assert!(
                (coarse_m - fine_m).abs() <= 1.0,
                "column {x},{z}: {coarse_m}m coarse vs {fine_m}m fine"
            );
        }
    }

    #[test]
    fn soil_is_a_physical_depth_not_a_voxel_count() {
        let soil_m = WorldgenParams::default().soil_depth_m;
        for voxel_size in [1.0, 0.5, 0.25] {
            let g = generator(voxel_size);
            let dirt = column(&g, 0, 0)
                .iter()
                .filter(|v| v.material() == DIRT)
                .count();
            let depth_m = dirt as f32 * voxel_size;
            // A Voxel of slack at each end: grass caps the soil, and the lower
            // boundary falls wherever the surface height puts it.
            assert!(
                (depth_m - soil_m).abs() <= 2.0 * voxel_size,
                "{depth_m}m of dirt at {voxel_size}m Voxels, wanted {soil_m}m"
            );
        }
    }

    #[test]
    fn exactly_one_grass_voxel_caps_every_column() {
        let g = generator(1.0);
        for (x, z) in [(0, 0), (5, 17), (31, 2), (12, 31)] {
            let c = column(&g, x, z);
            let grass: Vec<usize> = (0..c.len()).filter(|&y| c[y].material() == GRASS).collect();
            assert_eq!(grass.len(), 1, "column {x},{z} has {grass:?}");
            assert_eq!(Some(grass[0]), highest_solid(&c), "grass is not on top");
        }
    }

    #[test]
    fn generation_is_deterministic() {
        let g = generator(1.0);
        let (a, b) = (g.chunk(3, 0, -2), g.chunk(3, 1, -2));
        let (c, d) = (g.chunk(3, 0, -2), g.chunk(3, 1, -2));
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    assert_eq!(a.get(x, y, z), c.get(x, y, z));
                    assert_eq!(b.get(x, y, z), d.get(x, y, z));
                }
            }
        }
    }

    /// Compared through `surface_height_m` rather than through a Chunk: two
    /// seeds give different terrain, but any single Chunk may be solid rock or
    /// empty sky in both.
    #[test]
    fn a_different_seed_is_a_different_world() {
        let a = Worldgen::new(1, 1.0, WorldgenParams::default());
        let b = Worldgen::new(2, 1.0, WorldgenParams::default());
        assert!((0..200).any(|i| {
            let m = i as f32 * 11.3;
            a.surface_height_m(m, -m) != b.surface_height_m(m, -m)
        }));
    }

    #[test]
    fn a_seed_differing_only_in_its_high_bits_is_a_different_world() {
        // Truncating the u64 instead of folding it would make these one world.
        let a = Worldgen::new(1, 1.0, WorldgenParams::default());
        let b = Worldgen::new(1 | (1 << 40), 1.0, WorldgenParams::default());
        assert!((0..200).any(|i| {
            let m = i as f32 * 11.3;
            a.surface_height_m(m, -m) != b.surface_height_m(m, -m)
        }));
    }

    #[test]
    fn both_early_outs_fire_and_a_straddling_chunk_takes_neither() {
        let g = generator(1.0);
        let (lowest, highest) = g.surface_bounds_m();
        let cell = CHUNK_SIZE as f32;

        let sky = g.chunk(0, (highest / cell) as i32 + 1, 0);
        assert_eq!(sky.uniform(), Some(Voxel::AIR), "a Chunk above the band");

        let deep = g.chunk(0, (lowest / cell) as i32 - 2, 0);
        assert_eq!(
            deep.uniform(),
            Some(Voxel::cube(STONE)),
            "a Chunk below the band"
        );

        let straddling = g.chunk(0, (highest / cell) as i32, 0);
        assert_eq!(straddling.uniform(), None, "a Chunk holding the surface");
    }

    #[test]
    fn the_surface_never_leaves_the_bounds_the_early_outs_trust() {
        let g = generator(1.0);
        let (lowest, highest) = g.surface_bounds_m();
        for i in 0..5000 {
            let (x_m, z_m) = (i as f32 * 13.7 - 2000.0, i as f32 * -7.1 + 900.0);
            let h = g.surface_height_m(x_m, z_m);
            assert!((lowest..highest).contains(&h), "{h}m at {x_m},{z_m}");
        }
    }
}
