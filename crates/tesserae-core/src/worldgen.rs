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
use crate::noise::{fbm2, fbm3, fbm3_range};
use crate::voxel::Voxel;

/// The ids generation assigns. They live in [`crate::material`] with the rest of
/// the table and are re-exported so a caller that places terrain finds them here.
pub use crate::material::{DIRT, GRASS, LANTERN, STONE};

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
    /// `None` is a World without caves.
    pub caves: Option<CaveParams>,
}

/// Tunnels carved out of solid ground. Plain `Copy` data like the rest of
/// [`WorldgenParams`]: a Mod chooses numbers, generation stays native.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CaveParams {
    /// The period of the lowest octave of the field the tunnels follow, in
    /// metres: roughly how far apart two tunnels run.
    pub feature_size_m: f32,
    /// Roughly how wide a tunnel is: the radius, in metres, of the largest
    /// sphere that fits in it. Approximate by construction (see [`GRADIENT`]);
    /// the physical size does not change with Voxel size.
    pub radius_m: f32,
    /// No cave below this height. Everything under it is the uniform rock the
    /// deep early-out answers for without sampling anything.
    pub floor_m: f32,
    pub octaves: u32,
}

impl Default for CaveParams {
    fn default() -> Self {
        CaveParams {
            feature_size_m: 64.0,
            radius_m: 4.8,
            floor_m: -24.0,
            octaves: 2,
        }
    }
}

impl Default for WorldgenParams {
    fn default() -> Self {
        WorldgenParams {
            sea_level_m: 24.0,
            relief_m: 48.0,
            feature_size_m: 96.0,
            soil_depth_m: 3.0,
            octaves: 5,
            caves: Some(CaveParams::default()),
        }
    }
}

/// Converts a tunnel radius into a window on the field. Two-octave value noise
/// changes by about this much per lattice cell (measured near the midline: 0.51),
/// so `|field - 0.5| < GRADIENT * r / L` is a slab about `r` metres either side
/// at a feature size `L`. A calibration and not a derivation, and the one place
/// a tunnel's width is approximate; `a_tunnel_is_as_wide_as_its_radius_says`
/// fails if the noise changes enough to need a new number.
const GRADIENT: f32 = 0.5;

/// A tunnel is where two independent fields are both near their midline: each
/// alone is a slab, and two slabs cross in a winding tube. A tube is thin, so
/// [`CaveField::may_carve`] can often prove a box holds none of it.
///
/// Every position here is in metres and is the Voxel's lower corner, as the
/// surface is sampled, so the same point gets the same answer at any Voxel size.
#[derive(Clone, Copy, Debug)]
struct CaveField {
    seeds: [u32; 2],
    inv_feature_size: f32,
    /// Half the width of the window on the field, in field units.
    half_width: f32,
    octaves: u32,
    floor_m: f32,
}

impl CaveField {
    fn new(seed: u32, p: CaveParams) -> Self {
        debug_assert!(p.feature_size_m > 0.0, "caves have a size");
        CaveField {
            // Far from the terrain octaves' `seed + 0..octaves`, so the fields
            // are not correlated with the surface.
            seeds: [
                seed.wrapping_add(0x0ca5_e001),
                seed.wrapping_add(0x0ca5_e101),
            ],
            inv_feature_size: 1.0 / p.feature_size_m,
            half_width: GRADIENT * p.radius_m / p.feature_size_m,
            octaves: p.octaves,
            floor_m: p.floor_m,
        }
    }

    fn carves(&self, x_m: f32, y_m: f32, z_m: f32) -> bool {
        if y_m < self.floor_m {
            return false;
        }
        let k = self.inv_feature_size;
        self.seeds.iter().all(|&s| {
            (fbm3(x_m * k, y_m * k, z_m * k, s, self.octaves) - 0.5).abs() < self.half_width
        })
    }

    /// Whether anything in the box `lo..=hi` (metres) could be carved. `false`
    /// is a proof, and the generator relies on it: it skips the box and gets
    /// what sampling every point would have. `true` only means it was not ruled
    /// out.
    fn may_carve(&self, lo: [f32; 3], hi: [f32; 3]) -> bool {
        if hi[1] < self.floor_m {
            return false;
        }
        let k = self.inv_feature_size;
        self.seeds.iter().all(|&s| {
            let (min, max) = fbm3_range(lo.map(|v| v * k), hi.map(|v| v * k), s, self.octaves);
            // The window is |field - 0.5| < half_width; the nearest the field
            // gets to the midline anywhere in the box.
            let nearest = if min <= 0.5 && 0.5 <= max {
                0.0
            } else {
                (min - 0.5).abs().min((max - 0.5).abs())
            };
            nearest < self.half_width
        })
    }
}

/// Voxels per side of the sub-blocks a Chunk is culled in. An implementation
/// detail with no physical meaning: it only decides how finely a box is ruled
/// out, never what comes out.
const BLOCK: usize = 8;
const BLOCKS: usize = CHUNK_SIZE / BLOCK;

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
    caves: Option<CaveField>,
}

impl Worldgen {
    pub fn new(seed: u64, voxel_size: f32, params: WorldgenParams) -> Self {
        debug_assert!(voxel_size > 0.0, "a Voxel has an edge length");
        debug_assert!(params.feature_size_m > 0.0, "features have a size");
        let folded = (seed ^ (seed >> 32)) as u32;
        Worldgen {
            voxel_size,
            params,
            seed: folded,
            inv_feature_size: 1.0 / params.feature_size_m,
            caves: params.caves.map(|p| CaveField::new(folded, p)),
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
    ///
    /// Caves do not move it: they only turn solid Voxels into air, so nothing
    /// above the band is ever solid, which `streaming::relight`'s open-sky
    /// boundary relies on.
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
        let reach_m = (CHUNK_SIZE - 1) as f32 * self.voxel_size;
        let origin_m = [
            (cx * size) as f32 * self.voxel_size,
            floor_m,
            (cz * size) as f32 * self.voxel_size,
        ];

        // Most Chunks in a streamed ring answer here without sampling noise at
        // all, which is what makes a vertical extent affordable rather than a
        // micro-optimisation: a 3D ring is mostly sky and mostly rock.
        if floor_m > highest {
            return Chunk::air();
        }
        if ceiling_m < lowest - self.params.soil_depth_m {
            let mut chunk = Chunk::filled(Voxel::cube(STONE));
            // A Chunk that only *might* hold a tunnel stays `filled` unless a
            // Voxel is actually carved, so the light and mesh fast paths that
            // key off `uniform()` still fire for it.
            if let Some(caves) = &self.caves
                && caves.may_carve(
                    origin_m,
                    [origin_m[0] + reach_m, ceiling_m, origin_m[2] + reach_m],
                )
            {
                self.carve(&mut chunk, caves, origin_m, ceiling_m);
            }
            return chunk;
        }

        let mut surface_m = [0.0f32; CHUNK_SIZE * CHUNK_SIZE];
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                surface_m[z * CHUNK_SIZE + x] = self.surface_height_m(
                    origin_m[0] + x as f32 * self.voxel_size,
                    origin_m[2] + z as f32 * self.voxel_size,
                );
            }
        }
        // Nothing above the highest column of this Chunk is solid, so nothing
        // there is worth carving.
        let top_m = surface_m.iter().copied().fold(f32::MIN, f32::max);
        let reachable = self
            .caves
            .as_ref()
            .filter(|caves| {
                let ceiling_m = ceiling_m.min(top_m);
                floor_m <= ceiling_m
                    && caves.may_carve(
                        origin_m,
                        [origin_m[0] + reach_m, ceiling_m, origin_m[2] + reach_m],
                    )
            })
            .map(|caves| (caves, self.reachable_blocks(caves, origin_m, top_m)));

        let mut chunk = Chunk::air();
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let x_m = origin_m[0] + x as f32 * self.voxel_size;
                let z_m = origin_m[2] + z as f32 * self.voxel_size;
                let surface_m = surface_m[z * CHUNK_SIZE + x];

                for y in 0..CHUNK_SIZE {
                    let y_m = floor_m + y as f32 * self.voxel_size;
                    let below_m = surface_m - y_m;
                    if below_m < 0.0 {
                        // y_m only rises, so the rest of the column is sky.
                        break;
                    }
                    // A cave carves through the Grass cap like anything else,
                    // which is what a mouth is.
                    if let Some((caves, blocks)) = &reachable
                        && blocks[block_of(x, y, z)]
                        && caves.carves(x_m, y_m, z_m)
                    {
                        continue;
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

    /// For each sub-block of the Chunk whose lowest Voxel is at `origin_m`,
    /// whether a tunnel could be inside its solid part, which ends at `top_m`.
    ///
    /// One level. A second, over octants, was tried and moved the cost of a
    /// Chunk that passes the whole-Chunk test and carves nothing by under 10%:
    /// that cost is the Voxels sampled in sub-blocks the bound is too loose to
    /// rule out, not the bound.
    fn reachable_blocks(
        &self,
        caves: &CaveField,
        origin_m: [f32; 3],
        top_m: f32,
    ) -> [bool; BLOCKS * BLOCKS * BLOCKS] {
        let step_m = (BLOCK as f32) * self.voxel_size;
        let reach_m = (BLOCK - 1) as f32 * self.voxel_size;
        let mut reachable = [false; BLOCKS * BLOCKS * BLOCKS];
        for bz in 0..BLOCKS {
            for by in 0..BLOCKS {
                for bx in 0..BLOCKS {
                    let lo = [
                        origin_m[0] + bx as f32 * step_m,
                        origin_m[1] + by as f32 * step_m,
                        origin_m[2] + bz as f32 * step_m,
                    ];
                    let hi = [
                        lo[0] + reach_m,
                        (lo[1] + reach_m).min(top_m),
                        lo[2] + reach_m,
                    ];
                    reachable[(bz * BLOCKS + by) * BLOCKS + bx] =
                        lo[1] <= hi[1] && caves.may_carve(lo, hi);
                }
            }
        }
        reachable
    }

    /// Carves a Chunk that is solid rock throughout, visiting only the
    /// sub-blocks a tunnel can reach.
    fn carve(&self, chunk: &mut Chunk, caves: &CaveField, origin_m: [f32; 3], top_m: f32) {
        let reachable = self.reachable_blocks(caves, origin_m, top_m);
        for z in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    if reachable[block_of(x, y, z)]
                        && caves.carves(
                            origin_m[0] + x as f32 * self.voxel_size,
                            origin_m[1] + y as f32 * self.voxel_size,
                            origin_m[2] + z as f32 * self.voxel_size,
                        )
                    {
                        chunk.set(x, y, z, Voxel::AIR);
                    }
                }
            }
        }
    }
}

fn block_of(x: usize, y: usize, z: usize) -> usize {
    ((z / BLOCK) * BLOCKS + y / BLOCK) * BLOCKS + x / BLOCK
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEED: u64 = 0x7e55_e2ae;

    /// The terrain tests are about the surface, so they run without caves; the
    /// ones about caves say `caved`.
    fn terrain(voxel_size: f32) -> Worldgen {
        Worldgen::new(
            SEED,
            voxel_size,
            WorldgenParams {
                caves: None,
                ..WorldgenParams::default()
            },
        )
    }

    fn caved(voxel_size: f32) -> Worldgen {
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
        let coarse = terrain(1.0);
        let fine = terrain(0.25);
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
        let coarse = terrain(1.0);
        let fine = terrain(0.5);
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
            let g = terrain(voxel_size);
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
        let g = terrain(1.0);
        for (x, z) in [(0, 0), (5, 17), (31, 2), (12, 31)] {
            let c = column(&g, x, z);
            let grass: Vec<usize> = (0..c.len()).filter(|&y| c[y].material() == GRASS).collect();
            assert_eq!(grass.len(), 1, "column {x},{z} has {grass:?}");
            assert_eq!(Some(grass[0]), highest_solid(&c), "grass is not on top");
        }
    }

    /// The contract the shaped-Voxel cap rests on: terrain is cubes. A generator
    /// that places stairs or slabs - slope smoothing is the obvious use - changes
    /// this test deliberately and stays under `MAX_SHAPED_PER_CHUNK`, because
    /// nothing downstream polices it. See `crate::shape::MAX_SHAPED_PER_CHUNK`.
    #[test]
    fn generation_places_no_shaped_voxels() {
        for g in [terrain(1.0), caved(1.0)] {
            for cy in -1..5 {
                for (cx, cz) in [(0, 0), (3, -2), (-5, 7), (11, 11)] {
                    assert_eq!(g.chunk(cx, cy, cz).shaped_count(), 0, "{cx} {cy} {cz}");
                }
            }
        }
    }

    #[test]
    fn generation_is_deterministic() {
        let g = caved(1.0);
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
        let g = terrain(1.0);
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

        // From the surface over this column, not from the top of the band: the
        // band is where the surface *can* reach, and the Chunk at the top of it
        // is usually empty sky.
        let here = (g.surface_height_m(0.0, 0.0) / cell).floor() as i32;
        let straddling = g.chunk(0, here, 0);
        assert_eq!(straddling.uniform(), None, "a Chunk holding the surface");
    }

    #[test]
    fn the_surface_never_leaves_the_bounds_the_early_outs_trust() {
        let g = terrain(1.0);
        let (lowest, highest) = g.surface_bounds_m();
        for i in 0..5000 {
            let (x_m, z_m) = (i as f32 * 13.7 - 2000.0, i as f32 * -7.1 + 900.0);
            let h = g.surface_height_m(x_m, z_m);
            assert!((lowest..highest).contains(&h), "{h}m at {x_m},{z_m}");
        }
    }

    /// Every Chunk of a sweep, as metres-anchored `(coord, with, without)`.
    fn sweep(voxel_size: f32) -> Vec<(i32, i32, i32)> {
        // Eight Chunks high from below the cave floor to past the surface band,
        // over a square of columns, at any Voxel size.
        let cell_m = CHUNK_SIZE as f32 * voxel_size;
        let lo = (-40.0 / cell_m).floor() as i32;
        let hi = (80.0 / cell_m).ceil() as i32;
        let span = (96.0 / cell_m).ceil() as i32;
        let mut coords = vec![];
        for cx in -span / 2..span / 2 {
            for cz in -span / 2..span / 2 {
                for cy in lo..hi {
                    coords.push((cx, cy, cz));
                }
            }
        }
        coords
    }

    #[test]
    fn caves_only_ever_remove_voxels() {
        // The structural reason nothing is solid above `surface_bounds_m().1`
        // and the Server's open-sky boundary stays exact.
        let (with, without) = (caved(1.0), terrain(1.0));
        let mut carved = 0;
        for (cx, cy, cz) in sweep(1.0) {
            let (a, b) = (with.chunk(cx, cy, cz), without.chunk(cx, cy, cz));
            for z in 0..CHUNK_SIZE {
                for y in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        let (got, was) = (a.get(x, y, z), b.get(x, y, z));
                        if got != was {
                            assert!(
                                got.is_air(),
                                "{cx},{cy},{cz} {x},{y},{z}: {was:?} -> {got:?}"
                            );
                            carved += 1;
                        }
                    }
                }
            }
        }
        assert!(carved > 0, "no cave anywhere in the sweep");
    }

    /// The early-out rests on a bound being sound. This builds the Chunk the
    /// slow way - every Voxel sampled, nothing ruled out - and compares, so no
    /// layer of the culling can change an output.
    fn slow(g: &Worldgen, cx: i32, cy: i32, cz: i32) -> Chunk {
        let size = CHUNK_SIZE as i32;
        let mut chunk = Chunk::air();
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let x_m = (cx * size + x as i32) as f32 * g.voxel_size;
                let z_m = (cz * size + z as i32) as f32 * g.voxel_size;
                let surface_m = g.surface_height_m(x_m, z_m);
                for y in 0..CHUNK_SIZE {
                    let y_m = (cy * size + y as i32) as f32 * g.voxel_size;
                    let below_m = surface_m - y_m;
                    if below_m < 0.0 || g.caves.as_ref().is_some_and(|c| c.carves(x_m, y_m, z_m)) {
                        continue;
                    }
                    let material = if below_m < g.voxel_size {
                        GRASS
                    } else if below_m < g.params.soil_depth_m {
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

    #[test]
    fn ruling_a_box_out_never_changes_the_chunk() {
        for voxel_size in [1.0, 0.5] {
            let g = caved(voxel_size);
            for (cx, cy, cz) in sweep(voxel_size) {
                let (fast, reference) = (g.chunk(cx, cy, cz), slow(&g, cx, cy, cz));
                for z in 0..CHUNK_SIZE {
                    for y in 0..CHUNK_SIZE {
                        for x in 0..CHUNK_SIZE {
                            assert_eq!(
                                fast.get(x, y, z),
                                reference.get(x, y, z),
                                "{voxel_size}m Voxels, {cx},{cy},{cz} at {x},{y},{z}"
                            );
                        }
                    }
                }
            }
        }
    }

    /// Which of the metre points `0..` (whole metres) are solid: the same
    /// physical ground at any Voxel size, or the caves are tuned in Voxels.
    fn solid_at(g: &Worldgen, x_m: i32, y_m: i32, z_m: i32) -> bool {
        let size = CHUNK_SIZE as i32;
        let index = |m: i32| (m as f32 / g.voxel_size) as i32;
        let (x, y, z) = (index(x_m), index(y_m), index(z_m));
        let chunk = g.chunk(x.div_euclid(size), y.div_euclid(size), z.div_euclid(size));
        !chunk
            .get(
                x.rem_euclid(size) as usize,
                y.rem_euclid(size) as usize,
                z.rem_euclid(size) as usize,
            )
            .is_air()
    }

    #[test]
    fn the_same_metre_point_is_carved_at_any_voxel_size() {
        let (coarse, fine) = (caved(1.0), caved(0.5));
        let (mut carved, mut checked) = (0, 0);
        for i in 0..1500 {
            let (x, z) = (i * 7 % 61 - 30, i * 13 % 67 - 33);
            let y = -20 + (i * 5) % 70;
            // Clear of the surface, where the Voxel size changes which Material
            // the topmost Voxels are and so where the ground ends.
            if (y as f32) > coarse.surface_height_m(x as f32, z as f32) - 2.0 {
                continue;
            }
            checked += 1;
            let (a, b) = (solid_at(&coarse, x, y, z), solid_at(&fine, x, y, z));
            assert_eq!(a, b, "{x},{y},{z}");
            carved += usize::from(!a);
        }
        assert!(carved > 0 && carved < checked, "{carved} of {checked}");
    }

    #[test]
    fn a_tunnel_is_the_same_width_at_any_voxel_size() {
        // The share of ground a cave takes is a physical quantity. Sampled on a
        // 1 m grid it is the same set of points, so it is the same share; at
        // 0.25 m Voxels a width tuned in Voxels would quarter it.
        let share = |g: &Worldgen| {
            let (mut under, mut air) = (0, 0);
            for i in 0..3000 {
                let (x, z) = (i * 7 % 83 - 41, i * 11 % 89 - 44);
                let y = -20 + (i * 3) % 40;
                under += 1;
                air += usize::from(
                    g.caves
                        .as_ref()
                        .unwrap()
                        .carves(x as f32, y as f32, z as f32),
                );
            }
            air as f32 / under as f32
        };
        let (a, b) = (share(&caved(1.0)), share(&caved(0.25)));
        assert_eq!(a, b);
        assert!((0.005..0.2).contains(&a), "{a} of the ground is cave");
    }

    /// The slab either side of a field's midline, measured: the distance along
    /// the gradient from the midline to the edge of the carve window. Its median
    /// over many points should be the radius the parameter asks for, within the
    /// slack a calibration allows.
    #[test]
    fn a_tunnel_is_as_wide_as_its_radius_says() {
        use crate::noise::fbm3;
        let params = CaveParams::default();
        let field = CaveField::new(7, params);
        let k = field.inv_feature_size;
        let h = 0.05;
        let mut half_widths = vec![];
        for i in 0..20_000u32 {
            let unit = |salt: u32| {
                let x = i
                    .wrapping_mul(2_654_435_761)
                    .wrapping_add(salt.wrapping_mul(40_503));
                ((x ^ (x >> 15)).wrapping_mul(2_246_822_519) >> 8) as f32 / (1u32 << 24) as f32
            };
            let p = [unit(1) * 300.0, unit(2) * 300.0, unit(3) * 300.0];
            let f = |p: [f32; 3]| fbm3(p[0] * k, p[1] * k, p[2] * k, field.seeds[0], field.octaves);
            if (f(p) - 0.5).abs() > 0.02 {
                continue;
            }
            let g: [f32; 3] = std::array::from_fn(|a| {
                let (mut lo, mut hi) = (p, p);
                lo[a] -= h;
                hi[a] += h;
                (f(hi) - f(lo)) / (2.0 * h)
            });
            let slope = (g[0] * g[0] + g[1] * g[1] + g[2] * g[2]).sqrt();
            half_widths.push(field.half_width / slope);
        }
        half_widths.sort_by(f32::total_cmp);
        let median = half_widths[half_widths.len() / 2];
        assert!(
            (0.8..1.25).contains(&(median / params.radius_m)),
            "median half-width {median} m for a radius of {} m",
            params.radius_m
        );
    }

    #[test]
    fn nothing_below_the_cave_floor_is_carved() {
        let g = caved(1.0);
        let floor = WorldgenParams::default().caves.unwrap().floor_m;
        let cell = CHUNK_SIZE as f32;
        let below = (floor / cell).floor() as i32 - 1;
        for (cx, cz) in [(0, 0), (4, -3), (-7, 2)] {
            assert_eq!(
                g.chunk(cx, below, cz).uniform(),
                Some(Voxel::cube(STONE)),
                "{cx},{cz}"
            );
        }
    }

    #[test]
    fn the_range_test_keeps_deep_chunks_uniform_that_no_tunnel_crosses() {
        // The floor does most of the early-out's work. Inside the band the
        // whole-Chunk range test is what is left, and it must rule out Chunks
        // the floor cannot: a bound that ruled nothing out would show here long
        // before it showed up in a benchmark. Feature size is generous so the
        // band is not simply full of tunnels.
        let params = WorldgenParams {
            caves: Some(CaveParams {
                feature_size_m: 128.0,
                floor_m: -120.0,
                ..CaveParams::default()
            }),
            ..WorldgenParams::default()
        };
        let g = Worldgen::new(SEED, 1.0, params);
        let (mut uniform, mut carved) = (0, 0);
        for cx in -5..5 {
            for cz in -5..5 {
                for cy in -4..0 {
                    match g.chunk(cx, cy, cz).uniform() {
                        Some(v) => {
                            assert_eq!(v, Voxel::cube(STONE));
                            uniform += 1;
                        }
                        None => carved += 1,
                    }
                }
            }
        }
        assert!(carved > 0, "no deep Chunk has a cave");
        assert!(uniform > carved, "{uniform} uniform, {carved} carved");
    }

    #[test]
    fn a_column_has_at_most_one_grass_voxel_and_it_is_on_top() {
        // A cave mouth removes the cap, so "exactly one" holds only without
        // caves (`exactly_one_grass_voxel_caps_every_column`).
        let g = caved(1.0);
        for i in 0..40 {
            let (x, z) = ((i * 7) % CHUNK_SIZE, (i * 13) % CHUNK_SIZE);
            let c = column(&g, x, z);
            let grass: Vec<usize> = (0..c.len()).filter(|&y| c[y].material() == GRASS).collect();
            assert!(grass.len() <= 1, "column {x},{z} has {grass:?}");
            if let Some(&y) = grass.first() {
                assert_eq!(Some(y), highest_solid(&c), "grass is not on top at {x},{z}");
            }
        }
    }

    #[test]
    fn a_mouth_exists_and_its_walls_show_dirt_and_stone() {
        let (with, without) = (caved(1.0), terrain(1.0));
        let (mut mouths, mut walls) = (0, std::collections::HashSet::new());
        for (cx, cy, cz) in sweep(1.0) {
            let (a, b) = (with.chunk(cx, cy, cz), without.chunk(cx, cy, cz));
            for z in 0..CHUNK_SIZE {
                for y in 1..CHUNK_SIZE - 1 {
                    for x in 1..CHUNK_SIZE - 1 {
                        if a.get(x, y, z).is_air() && !b.get(x, y, z).is_air() {
                            mouths += usize::from(b.get(x, y, z).material() == GRASS);
                            walls.insert(a.get(x - 1, y, z).material());
                            walls.insert(a.get(x + 1, y, z).material());
                            walls.insert(a.get(x, y - 1, z).material());
                            walls.insert(a.get(x, y + 1, z).material());
                        }
                    }
                }
            }
        }
        assert!(mouths > 0, "no cave breaches the surface");
        assert!(walls.contains(&DIRT), "{walls:?}");
        assert!(walls.contains(&STONE), "{walls:?}");
    }

    #[test]
    fn a_world_without_caves_is_what_it_was() {
        // `caves: None` costs one `Option` check and changes nothing else.
        let g = terrain(1.0);
        assert!(g.caves.is_none());
        let cy = (g.surface_height_m(0.0, 0.0) / CHUNK_SIZE as f32).floor() as i32;
        assert_eq!(g.chunk(0, cy, 0).uniform(), None);
        assert_eq!(
            g.chunk(0, (-24.0 / CHUNK_SIZE as f32).floor() as i32, 0)
                .uniform(),
            Some(Voxel::cube(STONE))
        );
    }
}
