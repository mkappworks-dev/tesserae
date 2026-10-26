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
use crate::mesh::POS_Y;
use crate::noise::{fbm2, fbm3, fbm3_range};
use crate::shape::{MAX_SHAPED_PER_CHUNK, Orientation, SHAPE_SLAB, SHAPE_STAIR};
use crate::tables::Tables;
use crate::voxel::{MaterialId, Voxel};

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
    /// `None` is a World of cubes.
    pub smoothing: Option<SmoothingParams>,
    /// `None` is a World without lakes.
    pub water: Option<WaterParams>,
}

/// A fluid filling the open air above the ground up to one height: the World's
/// lakes. Plain `Copy` data like the rest of [`WorldgenParams`]. A flat sheet
/// of packed cubes, which Occupants reads as sources: it fills any hollow open
/// to the sky below its level, but never a cave, which is carved out of the
/// ground and not above it.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct WaterParams {
    /// The height of the lake's surface. Clamped to the top of the surface
    /// band, because the Server treats everything above that as open sky.
    pub level_m: f32,
    /// Any registered Material with a fluid row; [`WorldgenParams::validate_lake`]
    /// checks it against the tables.
    pub material: MaterialId,
}

/// The three solid Materials the ground is made of, from the bottom up.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GroundParams {
    /// Everything below the soil.
    pub rock: MaterialId,
    /// The layer `soil_depth_m` deep under the surface.
    pub soil: MaterialId,
    /// The top Voxel of each column, and the smoothing's stairs and slabs.
    pub surface: MaterialId,
}

/// Stairs and slabs placed where the ground steps one Voxel. Plain `Copy` data
/// like the rest of [`WorldgenParams`].
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SmoothingParams {
    /// The smallest step in the ground, in metres, worth easing. A stair or slab
    /// is half a Voxel tall, so what one can ease is a rise of exactly one Voxel,
    /// which is `voxel_size` metres; a World whose Voxel is smaller than this
    /// is not smoothed, because a body steps over such a rise unaided.
    ///
    /// A placeholder for the step height a player controller will own.
    pub min_rise_m: f32,
}

impl Default for SmoothingParams {
    fn default() -> Self {
        SmoothingParams { min_rise_m: 0.75 }
    }
}

/// Tunnels carved out of solid ground. Plain `Copy` data like the rest of
/// [`WorldgenParams`]: a Mod chooses numbers, generation stays native.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CaveParams {
    /// The period of the lowest octave of the field the tunnels follow, in
    /// metres: roughly how far apart two tunnels run.
    pub feature_size_m: f32,
    /// Roughly how wide a tunnel is: the radius, in metres, of the largest
    /// sphere that fits in it. Approximate by construction (see `GRADIENT`);
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

/// Where [`WorldgenParams::with_default_lake`] puts the surface, as a fraction
/// of the relief above sea level: about the 25th percentile of the default
/// ground, so valleys hold lakes and most land is dry.
const DEFAULT_WATER_FRACTION: f32 = 5.0 / 12.0;

impl Default for WorldgenParams {
    fn default() -> Self {
        let (sea_level_m, relief_m) = (24.0, 48.0);
        WorldgenParams {
            sea_level_m,
            relief_m,
            feature_size_m: 96.0,
            soil_depth_m: 3.0,
            octaves: 5,
            caves: Some(CaveParams::default()),
            smoothing: Some(SmoothingParams::default()),
            // The engine declares no fluid, so standing fluid is a Game's to add.
            water: None,
        }
    }
}

impl WorldgenParams {
    /// These parameters with lakes of `material` in the valleys of their ground.
    pub fn with_default_lake(self, material: MaterialId) -> Self {
        WorldgenParams {
            water: Some(WaterParams {
                level_m: self.sea_level_m + self.relief_m * DEFAULT_WATER_FRACTION,
                material,
            }),
            ..self
        }
    }
}

/// The noise scales a coordinate by `2^(octaves - 1)` in `f32` before taking
/// its lattice cell. At 8 that stays exact out to 2^17 feature sizes from the
/// origin; at 24 it is exact only within 2 of them, and past 2^31 the cell
/// index overflows.
pub const MAX_OCTAVES: u32 = 8;

/// How far from zero the surface band may reach, in metres. Coarse light walks
/// the band a Chunk row at a time generating eight Chunks per row, and a metre
/// position stops being exact at small Voxel sizes much past this.
pub const MAX_HEIGHT_M: f32 = 65_536.0;

/// A parameter [`WorldgenParams::validate`] rejected, and the rule it broke.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ParamError {
    pub field: &'static str,
    pub rule: &'static str,
}

impl std::fmt::Display for ParamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.field, self.rule)
    }
}

impl std::error::Error for ParamError {}

fn require(ok: bool, field: &'static str, rule: &'static str) -> Result<(), ParamError> {
    ok.then_some(()).ok_or(ParamError { field, rule })
}

/// A frequency is `1 / size`, so a size whose inverse overflows is as bad as zero.
fn positive_size(field: &'static str, metres: f32) -> Result<(), ParamError> {
    require(
        metres.is_finite() && metres > 0.0 && (1.0 / metres).is_finite(),
        field,
        "must be finite and greater than zero",
    )
}

fn octaves(field: &'static str, count: u32) -> Result<(), ParamError> {
    require(
        (1..=MAX_OCTAVES).contains(&count),
        field,
        "must be from 1 to 8",
    )
}

impl WorldgenParams {
    /// Whether the generator can run on these: every distance finite and in
    /// metres, and in the ranges its arithmetic assumes. `0` octaves is
    /// tolerated by the noise (a flat World) but rejected here, since relief of
    /// zero says the same thing.
    pub fn validate(&self) -> Result<(), ParamError> {
        let within = |m: f32| (-MAX_HEIGHT_M..=MAX_HEIGHT_M).contains(&m);
        require(
            within(self.sea_level_m),
            "sea_level_m",
            "must be finite and within 65536 m of zero",
        )?;
        require(
            (0.0..=MAX_HEIGHT_M).contains(&self.relief_m)
                && within(self.sea_level_m + self.relief_m),
            "relief_m",
            "must be from 0 and keep sea_level_m + relief_m within 65536 m of zero",
        )?;
        positive_size("feature_size_m", self.feature_size_m)?;
        require(
            (0.0..=MAX_HEIGHT_M).contains(&self.soil_depth_m),
            "soil_depth_m",
            "must be from 0 to 65536 m",
        )?;
        octaves("octaves", self.octaves)?;
        if let Some(caves) = self.caves {
            positive_size("caves.feature_size_m", caves.feature_size_m)?;
            require(
                caves.radius_m > 0.0 && caves.radius_m.is_finite(),
                "caves.radius_m",
                "must be finite and greater than zero",
            )?;
            require(caves.floor_m.is_finite(), "caves.floor_m", "must be finite")?;
            octaves("caves.octaves", caves.octaves)?;
        }
        if let Some(smoothing) = self.smoothing {
            require(
                smoothing.min_rise_m.is_finite() && smoothing.min_rise_m >= 0.0,
                "smoothing.min_rise_m",
                "must be finite and not negative",
            )?;
        }
        if let Some(water) = self.water {
            require(water.level_m.is_finite(), "water.level_m", "must be finite")?;
        }
        Ok(())
    }

    /// Whether the lake's Material can fill a lake: registered, and a fluid.
    /// Apart from [`Self::validate`] because it needs every fluid row, which a
    /// load has only once each Declaration is in.
    pub fn validate_lake(&self, tables: &Tables) -> Result<(), ParamError> {
        let Some(water) = self.water else {
            return Ok(());
        };
        let m = water.material;
        require(
            tables.materials.contains(m),
            "water.material",
            "names no registered Material",
        )?;
        require(
            !tables.materials.solid(Voxel::cube(m)),
            "water.material",
            "is solid, not a fluid",
        )?;
        require(
            tables.fluids.contains(m),
            "water.material",
            "has no fluid row",
        )
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

/// Columns per side of the surface array: the Chunk's own plus one either side,
/// so a column on an edge can ask its neighbour in the next Chunk without
/// loading it.
const SURFACE_W: usize = CHUNK_SIZE + 2;

/// How many shaped Voxels smoothing may place in one Chunk: half of
/// [`MAX_SHAPED_PER_CHUNK`], the other half being a player's. On ground that
/// steps at every column (a 45-degree ramp) a stair per column would fill the
/// cap exactly and leave nobody a Place.
pub const MAX_SMOOTHED_PER_CHUNK: usize = MAX_SHAPED_PER_CHUNK / 2;

/// The rise a half-height Shape can ease, in Voxels.
const EASED_RISE_VOXELS: i32 = 1;

/// A column's four cardinal neighbours as `(dx, dz, turn)`, where `turn` is the
/// quarter-turns about +Y that put a stair's tall side, canonically +Z, towards
/// it. `smoothing_stairs_face_the_step_they_climb` checks it against the table.
const CARDINALS: [(i32, i32, u8); 4] = [(0, 1, 0), (1, 0, 1), (0, -1, 2), (-1, 0, 3)];

fn surface_index(x: i32, z: i32) -> usize {
    (z + 1) as usize * SURFACE_W + (x + 1) as usize
}

/// The highest Voxel `y` (counted from the Chunk's floor, and out of range when
/// the surface is not in this Chunk's band) that the cube pass fills in a column
/// whose surface is `surface_m`. Uses the cube pass's own comparison, not
/// `floor(surface / voxel_size)`, so the two cannot disagree by an ulp.
fn top_index(surface_m: f32, floor_m: f32, voxel_size: f32) -> i32 {
    let mut y = ((surface_m - floor_m) / voxel_size).floor() as i32;
    while floor_m + (y + 1) as f32 * voxel_size <= surface_m {
        y += 1;
    }
    while floor_m + y as f32 * voxel_size > surface_m {
        y -= 1;
    }
    y
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
    caves: Option<CaveField>,
    /// Whether the smoothing pass runs: configured, and a one-Voxel step is at
    /// least `min_rise_m` at this Voxel size.
    smoothing: bool,
    /// The lake, its level already clamped to the surface band.
    water: Option<WaterParams>,
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
            smoothing: params
                .smoothing
                .is_some_and(|p| EASED_RISE_VOXELS as f32 * voxel_size >= p.min_rise_m),
            water: params.water.map(|w| WaterParams {
                level_m: w.level_m.min(params.sea_level_m + params.relief_m),
                ..w
            }),
        }
    }

    pub fn params(&self) -> WorldgenParams {
        self.params
    }

    /// The edge of the Voxels this generator makes, in metres.
    pub fn voxel_size(&self) -> f32 {
        self.voxel_size
    }

    /// The lake, its level clamped as generation clamps it.
    pub fn water(&self) -> Option<WaterParams> {
        self.water
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
    /// boundary relies on. Neither does smoothing: a stair needs a cube beside it
    /// in the same cell, so it sits no higher than the ground it eases.
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

        // The border is only read by smoothing, so a World of cubes does not
        // pay for it.
        let reach = if self.smoothing { 1 } else { 0 };
        let mut surface_m = [0.0f32; SURFACE_W * SURFACE_W];
        for z in -reach..CHUNK_SIZE as i32 + reach {
            for x in -reach..CHUNK_SIZE as i32 + reach {
                surface_m[surface_index(x, z)] =
                    self.surface_height_m(self.axis_m(cx, x), self.axis_m(cz, z));
            }
        }
        // Nothing above the highest column of this Chunk is solid, so nothing
        // there is worth carving. The border is another Chunk's ground.
        let top_m = (0..CHUNK_SIZE as i32)
            .flat_map(|z| (0..CHUNK_SIZE as i32).map(move |x| surface_index(x, z)))
            .map(|i| surface_m[i])
            .fold(f32::MIN, f32::max);
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
                let surface_m = surface_m[surface_index(x as i32, z as i32)];

                for y in 0..CHUNK_SIZE {
                    let y_m = floor_m + y as f32 * self.voxel_size;
                    let below_m = surface_m - y_m;
                    if below_m < 0.0 {
                        if let Some(water) = self.water
                            && y_m < water.level_m
                        {
                            chunk.set(x, y, z, Voxel::cube(water.material));
                            continue;
                        }
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
        if self.smoothing {
            self.smooth(&mut chunk, &surface_m, [cx, cy, cz], floor_m);
        }
        chunk
    }

    /// Eases every one-Voxel step in the ground with a stair or slab on top of
    /// the lower column, in the cell beside the higher column's top cube. A
    /// Chunk is asked only about the cells it holds: a column whose top is the
    /// last Voxel of the Chunk below is smoothed by the Chunk above, from the
    /// same surface heights.
    fn smooth(
        &self,
        chunk: &mut Chunk,
        surface_m: &[f32; SURFACE_W * SURFACE_W],
        coord: [i32; 3],
        floor_m: f32,
    ) {
        let size = CHUNK_SIZE as i32;
        let tops = surface_m.map(|s| top_index(s, floor_m, self.voxel_size));
        let mut placed = 0;
        for z in 0..size {
            for x in 0..size {
                let top = tops[surface_index(x, z)];
                let y = top + 1;
                if !(0..size).contains(&y) || !self.still_solid(chunk, coord, [x, top, z]) {
                    continue;
                }
                // Under water the step stays a step: a stair holds one Voxel,
                // so it would leave an air pocket where its missing half is.
                if !chunk.get(x as usize, y as usize, z as usize).is_air() {
                    continue;
                }
                let (mut rising, mut turn) = (0, 0);
                for (dx, dz, quarter) in CARDINALS {
                    let (nx, nz) = (x + dx, z + dz);
                    if tops[surface_index(nx, nz)] == top + EASED_RISE_VOXELS
                        && self.still_solid(chunk, coord, [nx, y, nz])
                    {
                        rising += 1;
                        turn = quarter;
                    }
                }
                let voxel = match rising {
                    0 => continue,
                    1 => Voxel::shaped(GRASS, SHAPE_STAIR, Orientation::new(POS_Y, turn)),
                    _ => Voxel::shaped(GRASS, SHAPE_SLAB, Orientation::IDENTITY),
                };
                chunk.set(x as usize, y as usize, z as usize, voxel);
                placed += 1;
                if placed == MAX_SMOOTHED_PER_CHUNK {
                    return;
                }
            }
        }
    }

    /// The metre position of Voxel `local` (possibly one outside) along an axis
    /// of the Chunk at `chunk`, computed from the Chunk that owns it exactly as
    /// that Chunk computes it. Two Chunks must ask the surface and the caves the
    /// same question about a shared edge, and `(c * size + l) * voxel_size`
    /// differs from `c * size * voxel_size + l * voxel_size` in the last bit
    /// at Voxel sizes that are not a power of two.
    fn axis_m(&self, chunk: i32, local: i32) -> f32 {
        let size = CHUNK_SIZE as i32;
        let owner = chunk + local.div_euclid(size);
        (owner * size) as f32 * self.voxel_size + local.rem_euclid(size) as f32 * self.voxel_size
    }

    /// Whether the cell `at` (Voxel offsets from this Chunk's corner, possibly
    /// one outside it) is solid ground, given that the cube pass made it so
    /// before caves. Inside the Chunk that is a read; outside, the cell belongs
    /// to a Chunk not held here, so it is asked of the cave field by metre
    /// position, which is what that Chunk asks.
    fn still_solid(&self, chunk: &Chunk, coord: [i32; 3], at: [i32; 3]) -> bool {
        if at.iter().all(|c| (0..CHUNK_SIZE as i32).contains(c)) {
            return !chunk
                .get(at[0] as usize, at[1] as usize, at[2] as usize)
                .is_air();
        }
        self.caves.as_ref().is_none_or(|caves| {
            !caves.carves(
                self.axis_m(coord[0], at[0]),
                self.axis_m(coord[1], at[1]),
                self.axis_m(coord[2], at[2]),
            )
        })
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

    /// The terrain tests are about the surface, so they run without caves or
    /// smoothing, which would put a stair on the Grass cap; the ones about
    /// caves say `caved` and the ones about smoothing say `smoothed`.
    fn terrain(voxel_size: f32) -> Worldgen {
        Worldgen::new(
            SEED,
            voxel_size,
            WorldgenParams {
                caves: None,
                smoothing: None,
                water: None,
                ..WorldgenParams::default()
            },
        )
    }

    fn caved(voxel_size: f32) -> Worldgen {
        Worldgen::new(
            SEED,
            voxel_size,
            WorldgenParams {
                smoothing: None,
                water: None,
                ..WorldgenParams::default()
            },
        )
    }

    /// Smoothing with no minimum rise, so it fires at any Voxel size: the tests
    /// about where it places things, rather than whether it runs.
    fn smoothed(voxel_size: f32, caves: bool) -> Worldgen {
        let defaults = WorldgenParams::default();
        Worldgen::new(
            SEED,
            voxel_size,
            WorldgenParams {
                caves: caves.then_some(CaveParams::default()),
                smoothing: Some(SmoothingParams { min_rise_m: 0.0 }),
                water: None,
                ..defaults
            },
        )
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

    /// `smoothing: None` is a World of cubes. Nothing downstream polices
    /// `MAX_SHAPED_PER_CHUNK`, so a generator that places shapes must, as
    /// smoothing does (`smoothed_generation_stays_within_the_budget`).
    #[test]
    fn without_smoothing_generation_places_no_shaped_voxels() {
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

    /// The fixture water and a lava that is hot, opaque and keeps out of
    /// Shapes: a lake of either is made the same way.
    fn lake_tables() -> (Tables, [MaterialId; 2]) {
        let mut tables = Tables::builtin();
        let water = crate::testgen::register_water(&mut tables);
        let lava = tables
            .materials
            .register(crate::MaterialDef {
                emission: 12,
                dimming: crate::light::MAX_LIGHT,
                occlusion: crate::Occlusion::Opaque,
                render: crate::RenderClass::Opaque,
                solid: false,
            })
            .unwrap();
        let hot = crate::FluidDef {
            waterlogs: false,
            extinguishes: false,
            heat: Some(crate::Heat {
                strength: 10,
                reach_m: 2.0,
            }),
            ..crate::testgen::WATER_FLUID
        };
        tables.register_fluid(lava, hot).unwrap();
        (tables, [water, lava])
    }

    fn flooded(voxel_size: f32, level_m: f32, material: MaterialId) -> Worldgen {
        Worldgen::new(
            SEED,
            voxel_size,
            WorldgenParams {
                water: Some(WaterParams { level_m, material }),
                // Smoothing yields to water; `smoothing_never_replaces_water`
                // is about that, and this about the water alone.
                smoothing: None,
                ..WorldgenParams::default()
            },
        )
    }

    /// A lake only ever replaces air above the ground and below its level, and
    /// fills all of that with its own fluid: a cave stays dry and the ground is
    /// unchanged.
    #[test]
    fn water_fills_the_open_air_between_the_ground_and_its_level_and_nothing_else() {
        let (_, fluids) = lake_tables();
        for (voxel_size, material) in [1.0f32, 0.5]
            .into_iter()
            .flat_map(|v| fluids.map(|m| (v, m)))
        {
            let dry = Worldgen::new(
                SEED,
                voxel_size,
                WorldgenParams {
                    water: None,
                    smoothing: None,
                    ..WorldgenParams::default()
                },
            );
            // Halfway up the ground the sweep covers, so it has both shores and
            // dry land.
            let heights: Vec<f32> = (-48..48)
                .flat_map(|x| (-48..48).map(move |z| (x as f32, z as f32)))
                .map(|(x, z)| dry.surface_height_m(x, z))
                .collect();
            let lowest = heights.iter().copied().fold(f32::MAX, f32::min);
            let highest = heights.iter().copied().fold(f32::MIN, f32::max);
            let level_m = (lowest + highest) / 2.0;
            let wet = flooded(voxel_size, level_m, material);
            let mut filled = 0;
            for (cx, cy, cz) in sweep(voxel_size) {
                let (a, b) = (dry.chunk(cx, cy, cz), wet.chunk(cx, cy, cz));
                for z in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        let x_m = dry.axis_m(cx, x as i32);
                        let z_m = dry.axis_m(cz, z as i32);
                        let surface_m = dry.surface_height_m(x_m, z_m);
                        for y in 0..CHUNK_SIZE {
                            let y_m = dry.axis_m(cy, y as i32);
                            let (before, after) = (a.get(x, y, z), b.get(x, y, z));
                            let open = before.is_air() && y_m > surface_m && y_m < level_m;
                            if open {
                                assert_eq!(after, Voxel::cube(material), "{x_m} {y_m} {z_m}");
                                filled += 1;
                            } else {
                                assert_eq!(after, before, "{x_m} {y_m} {z_m}");
                            }
                        }
                    }
                }
            }
            assert!(filled > 0, "the sweep floods something at {voxel_size} m");
        }
    }

    #[test]
    fn the_default_parameters_have_no_lakes_and_generate_no_fluid() {
        assert_eq!(WorldgenParams::default().water, None);
        let g = Worldgen::new(SEED, 1.0, WorldgenParams::default());
        for (cx, cy, cz) in sweep(1.0) {
            for v in g.chunk(cx, cy, cz).kinds() {
                assert!(
                    [crate::voxel::AIR, STONE, DIRT, GRASS].contains(&v.material()),
                    "{v:?}"
                );
            }
        }
    }

    /// A packed cube of any fluid is a source, so a lake of lava stands as a
    /// lake of water does.
    #[test]
    fn a_lake_of_any_fluid_is_read_as_its_sources() {
        let (tables, fluids) = lake_tables();
        for material in fluids {
            let g = flooded(1.0, mid_level_m(), material);
            let lake = sweep(1.0)
                .into_iter()
                .map(|(cx, cy, cz)| g.chunk(cx, cy, cz))
                .find_map(|c| c.kinds().iter().copied().find(|v| v.material() == material))
                .expect("the sweep holds some of the lake");
            assert_eq!(
                crate::Occupants::of(lake, crate::Overlay::NONE, &tables.fluids).fluid,
                Some(crate::Fluid::source(material))
            );
        }
    }

    /// Between the lowest and highest ground of the default sweep.
    fn mid_level_m() -> f32 {
        let p = WorldgenParams::default();
        p.sea_level_m + p.relief_m / 2.0
    }

    /// Under water a step stays a step: a stair holds one Voxel, and in a water
    /// cell its missing half would be an air pocket.
    #[test]
    fn smoothing_never_replaces_water() {
        let (_, fluids) = lake_tables();
        for material in fluids {
            smoothing_never_replaces_a_lake_of(material);
        }
    }

    fn smoothing_never_replaces_a_lake_of(material: MaterialId) {
        let wet = |smoothing| {
            Worldgen::new(
                SEED,
                1.0,
                WorldgenParams {
                    smoothing,
                    ..WorldgenParams::default()
                }
                .with_default_lake(material),
            )
        };
        let (plain, smoothed) = (wet(None), wet(Some(SmoothingParams::default())));
        let (mut placed, mut under_water) = (0, 0);
        for (cx, cy, cz) in sweep(1.0) {
            let (a, b) = (plain.chunk(cx, cy, cz), smoothed.chunk(cx, cy, cz));
            for i in 0..crate::chunk::CHUNK_VOLUME {
                let (x, y, z) = (
                    i % CHUNK_SIZE,
                    i / CHUNK_SIZE % CHUNK_SIZE,
                    i / (CHUNK_SIZE * CHUNK_SIZE),
                );
                let (before, after) = (a.get(x, y, z), b.get(x, y, z));
                if before != after {
                    assert!(before.is_air(), "{before:?} became {after:?}");
                    placed += 1;
                }
                under_water += usize::from(before == Voxel::cube(material));
            }
        }
        assert!(
            placed > 0 && under_water > 0,
            "the sweep has both stairs and water"
        );
    }

    /// A level above the surface band would put water where the Server assumes
    /// open sky, so it stops at the band's top.
    #[test]
    fn water_never_rises_above_the_highest_ground() {
        let (_, [water, _]) = lake_tables();
        let g = flooded(1.0, 10_000.0, water);
        let (_, highest_m) = g.surface_bounds_m();
        let cy = (highest_m / CHUNK_SIZE as f32).floor() as i32;
        let chunk = g.chunk(0, cy, 0);
        let mut water = 0;
        for y in 0..CHUNK_SIZE {
            let y_m = g.axis_m(cy, y as i32);
            for (x, z) in (0..CHUNK_SIZE).flat_map(|x| (0..CHUNK_SIZE).map(move |z| (x, z))) {
                let v = chunk.get(x, y, z);
                if y_m > highest_m {
                    assert!(v.is_air(), "{v:?} at {y_m} m");
                } else if v == Voxel::cube(water) {
                    water += 1;
                }
            }
        }
        assert!(water > 0, "the band's top Chunk is flooded below the top");
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

    /// A World's Voxels by global index, generating each Chunk once, for tests
    /// that look across Chunk edges.
    struct Probe<'a> {
        g: &'a Worldgen,
        chunks: std::collections::HashMap<[i32; 3], Chunk>,
    }

    impl<'a> Probe<'a> {
        fn new(g: &'a Worldgen) -> Self {
            Probe {
                g,
                chunks: Default::default(),
            }
        }

        fn at(&mut self, at: [i32; 3]) -> Voxel {
            let size = CHUNK_SIZE as i32;
            let coord = at.map(|c| c.div_euclid(size));
            let g = self.g;
            let chunk = self
                .chunks
                .entry(coord)
                .or_insert_with(|| g.chunk(coord[0], coord[1], coord[2]));
            let l = at.map(|c| c.rem_euclid(size) as usize);
            chunk.get(l[0], l[1], l[2])
        }
    }

    /// Every shaped Voxel a sweep of Chunks holds, as global Voxel indices.
    fn shaped_voxels(g: &Worldgen, probe: &mut Probe) -> Vec<([i32; 3], Voxel)> {
        let size = CHUNK_SIZE as i32;
        let mut out = vec![];
        for (cx, cy, cz) in sweep(g.voxel_size) {
            let chunk = g.chunk(cx, cy, cz);
            if !chunk.may_contain_shaped() {
                continue;
            }
            for z in 0..CHUNK_SIZE {
                for y in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        let v = chunk.get(x, y, z);
                        if v.is_shaped() {
                            let at = [
                                cx * size + x as i32,
                                cy * size + y as i32,
                                cz * size + z as i32,
                            ];
                            assert_eq!(probe.at(at), v);
                            out.push((at, v));
                        }
                    }
                }
            }
        }
        out
    }

    #[test]
    fn smoothed_generation_stays_within_the_budget() {
        let mut densest = (0, 0.0f32, (0, 0, 0));
        for voxel_size in [1.0, 0.5, 0.25] {
            for caves in [false, true] {
                let g = smoothed(voxel_size, caves);
                let mut anywhere = 0;
                for (cx, cy, cz) in sweep(voxel_size) {
                    let n = g.chunk(cx, cy, cz).shaped_count();
                    assert!(
                        n <= MAX_SMOOTHED_PER_CHUNK,
                        "{n} shaped Voxels at {voxel_size}m in {cx},{cy},{cz}"
                    );
                    // At the Voxel size the default smooths, real terrain never
                    // reaches the budget, so the budget is a valve and not a
                    // thing that shapes the ground.
                    if voxel_size == 1.0 {
                        assert!(n < MAX_SMOOTHED_PER_CHUNK, "{n} at {cx},{cy},{cz}");
                    }
                    anywhere += n;
                    if n > densest.0 {
                        densest = (n, voxel_size, (cx, cy, cz));
                    }
                }
                assert!(anywhere > 0, "no stair at {voxel_size}m, caves {caves}");
            }
        }
        const { assert!(MAX_SMOOTHED_PER_CHUNK < MAX_SHAPED_PER_CHUNK) };
        eprintln!("densest Chunk: {densest:?} of {MAX_SMOOTHED_PER_CHUNK}");
    }

    /// A 45-degree ramp: every column's neighbour on one side is one Voxel
    /// higher, so every column is a candidate, which is the most a Chunk can ask
    /// for. The budget decides exactly where it stops.
    #[test]
    fn a_ramp_that_steps_at_every_column_stops_at_the_budget() {
        let g = smoothed(1.0, false);
        let mut surface = [0.0f32; SURFACE_W * SURFACE_W];
        for z in -1..=CHUNK_SIZE as i32 {
            for x in -1..=CHUNK_SIZE as i32 {
                // Rising along x only, from y = 4.
                surface[surface_index(x, z)] = 4.0 + (x + 1) as f32;
            }
        }
        let mut chunk = Chunk::air();
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                for y in 0..=4 + x + 1 {
                    if y < CHUNK_SIZE {
                        chunk.set(x, y, z, Voxel::cube(GRASS));
                    }
                }
            }
        }
        g.smooth(&mut chunk, &surface, [0, 0, 0], 0.0);
        // A stair's cell has to be in the Chunk, so columns 26 and up have none
        // to place: 26 candidates a row.
        let per_row = CHUNK_SIZE - 6;
        assert!(per_row * CHUNK_SIZE > MAX_SMOOTHED_PER_CHUNK);
        assert_eq!(chunk.shaped_count(), MAX_SMOOTHED_PER_CHUNK);

        // Rows are visited in order, so the budget runs out part-way along the
        // row it reaches and every later row stays cubes.
        let (full_rows, rest) = (
            MAX_SMOOTHED_PER_CHUNK / per_row,
            MAX_SMOOTHED_PER_CHUNK % per_row,
        );
        for z in 0..CHUNK_SIZE {
            let stairs = (0..CHUNK_SIZE)
                .filter(|&x| (0..CHUNK_SIZE).any(|y| chunk.get(x, y, z).is_shaped()))
                .count();
            let want = match z.cmp(&full_rows) {
                std::cmp::Ordering::Less => per_row,
                std::cmp::Ordering::Equal => rest,
                std::cmp::Ordering::Greater => 0,
            };
            assert_eq!(stairs, want, "row {z}");
        }
    }

    #[test]
    fn smoothing_stairs_face_the_step_they_climb() {
        use crate::mesh::{NEG_X, NEG_Z, POS_X, POS_Z};
        use crate::shape::ShapeTable;
        let table = ShapeTable::builtin();
        let g = smoothed(1.0, false);
        let mut probe = Probe::new(&g);
        let sides = [
            (POS_X, [1, 0, 0]),
            (NEG_X, [-1, 0, 0]),
            (POS_Z, [0, 0, 1]),
            (NEG_Z, [0, 0, -1]),
        ];
        let (mut stairs, mut slabs, mut facings) = (0, 0, std::collections::HashSet::new());
        for (at, v) in shaped_voxels(&g, &mut probe) {
            // Which of the four neighbours stand at this Voxel's own level.
            // A neighbour that rises two Voxels also has a cube at this level,
            // but one above it; only the one-Voxel rises are what was eased.
            let level: Vec<usize> = sides
                .iter()
                .filter(|(_, d)| {
                    let beside = [0, 1, 2].map(|a| at[a] + d[a]);
                    probe.at(beside).is_full_cube()
                        && !probe
                            .at([beside[0], beside[1] + 1, beside[2]])
                            .is_full_cube()
                })
                .map(|&(f, _)| f)
                .collect();
            assert_eq!(v.material(), GRASS);
            assert!(probe.at([at[0], at[1] - 1, at[2]]).is_full_cube(), "{at:?}");
            match v.shape() {
                SHAPE_STAIR => {
                    stairs += 1;
                    assert_eq!(level.len(), 1, "a stair climbs exactly one step: {at:?}");
                    for (face, _) in sides {
                        assert_eq!(table.seals(v, face), face == level[0], "{at:?} face {face}");
                    }
                    facings.insert(level[0]);
                }
                SHAPE_SLAB => {
                    slabs += 1;
                    assert!(level.len() >= 2, "a slab eases two or more: {at:?}");
                    assert_eq!(v.orientation(), 0);
                }
                other => panic!("shape {other} at {at:?}"),
            }
        }
        assert!(stairs > 0 && slabs > 0, "{stairs} stairs, {slabs} slabs");
        assert_eq!(facings.len(), 4, "stairs face every way: {facings:?}");
    }

    /// Every shaped Voxel of a sweep rests on a cube, and every stair has a cube
    /// beside it at its own level with open air above that cube: the one-Voxel
    /// step it climbs. Returns how many of them sit on a Chunk's x or z edge,
    /// which is where the Chunk that placed one asked a Chunk it does not hold.
    fn assert_ramps_are_whole(g: &Worldgen) -> (usize, usize) {
        let mut probe = Probe::new(g);
        let size = CHUNK_SIZE as i32;
        let (mut at_edge, mut total) = (0, 0);
        for (at, v) in shaped_voxels(g, &mut probe) {
            total += 1;
            at_edge += usize::from(
                [at[0], at[2]]
                    .iter()
                    .any(|c| matches!(c.rem_euclid(size), 0 | 31)),
            );
            assert!(
                probe.at([at[0], at[1] - 1, at[2]]).is_full_cube(),
                "{at:?} floats"
            );
            if v.shape() == SHAPE_STAIR {
                let climbs = [[1, 0], [-1, 0], [0, 1], [0, -1]]
                    .iter()
                    .filter(|d| {
                        probe.at([at[0] + d[0], at[1], at[2] + d[1]]).is_full_cube()
                            && !probe
                                .at([at[0] + d[0], at[1] + 1, at[2] + d[1]])
                                .is_full_cube()
                    })
                    .count();
                assert_eq!(climbs, 1, "{at:?} climbs {climbs} steps");
            }
        }
        (at_edge, total)
    }

    #[test]
    fn a_ramp_is_never_cut_by_a_chunk_edge_or_floating_over_a_cave() {
        let g = smoothed(1.0, true);
        let (at_edge, total) = assert_ramps_are_whole(&g);
        assert!(at_edge > 0 && total > at_edge, "{at_edge} of {total}");

        // A cave is what makes one fail to stand: where the same World without
        // tunnels has a stair or slab, this one sometimes has none.
        let bare = smoothed(1.0, false);
        let suppressed: usize = sweep(1.0)
            .into_iter()
            .map(|(x, y, z)| {
                let (with, without) = (g.chunk(x, y, z), bare.chunk(x, y, z));
                (0..CHUNK_SIZE)
                    .flat_map(|k| {
                        (0..CHUNK_SIZE).flat_map(move |j| (0..CHUNK_SIZE).map(move |i| (i, j, k)))
                    })
                    .filter(|&(i, j, k)| {
                        without.get(i, j, k).is_shaped() && with.get(i, j, k).is_air()
                    })
                    .count()
            })
            .sum();
        assert!(suppressed > 0, "no cave touches a ramp in the sweep");
    }

    #[test]
    fn two_chunks_agree_on_a_ramp_at_a_voxel_size_that_is_not_a_power_of_two() {
        // `c * size * voxel_size + l * voxel_size` and `(c * size + l) *
        // voxel_size` differ in the last bit here, and a stair on a Chunk's edge
        // is decided by that Chunk about a column in the next.
        let (at_edge, total) = assert_ramps_are_whole(&smoothed(0.3, true));
        assert!(at_edge > 0 && total > at_edge, "{at_edge} of {total}");
    }

    #[test]
    fn a_stair_whose_cell_is_in_the_chunk_above_is_made_there() {
        // The ground reaches 31 m to 63 m here, so a Chunk floor at 32 m or
        // 64 m is crossed by ramps. A stair at y = 0 rests on the last Voxel of
        // the Chunk below, which that Chunk neither holds a stair for nor
        // knows about.
        let g = smoothed(1.0, true);
        let mut probe = Probe::new(&g);
        let mut at_floor = 0;
        for cx in -4..4 {
            for cz in -4..4 {
                for cy in 1..3 {
                    let chunk = g.chunk(cx, cy, cz);
                    for z in 0..CHUNK_SIZE {
                        for x in 0..CHUNK_SIZE {
                            if !chunk.get(x, 0, z).is_shaped() {
                                continue;
                            }
                            at_floor += 1;
                            let at = [cx * 32 + x as i32, cy * 32, cz * 32 + z as i32];
                            assert!(probe.at([at[0], at[1] - 1, at[2]]).is_full_cube(), "{at:?}");
                            let below = g.chunk(cx, cy - 1, cz);
                            assert!(!below.get(x, CHUNK_SIZE - 1, z).is_shaped());
                        }
                    }
                }
            }
        }
        assert!(at_floor > 0, "no ramp crosses a Chunk floor");
    }

    #[test]
    fn smoothed_generation_is_deterministic() {
        let g = smoothed(1.0, true);
        for (cx, cy, cz) in [(0, 0, 0), (3, 1, -2), (-5, 0, 7)] {
            let (a, b) = (g.chunk(cx, cy, cz), g.chunk(cx, cy, cz));
            for z in 0..CHUNK_SIZE {
                for y in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        assert_eq!(a.get(x, y, z), b.get(x, y, z));
                    }
                }
            }
        }
    }

    #[test]
    fn smoothing_only_adds_shaped_voxels_above_the_ground() {
        // Against the same World without it, every Voxel is unchanged except
        // that air gained a shaped Voxel.
        let (with, without) = (smoothed(1.0, true), {
            let defaults = WorldgenParams::default();
            Worldgen::new(
                SEED,
                1.0,
                WorldgenParams {
                    smoothing: None,
                    water: None,
                    ..defaults
                },
            )
        });
        let mut added = 0;
        for (cx, cy, cz) in sweep(1.0) {
            let (a, b) = (with.chunk(cx, cy, cz), without.chunk(cx, cy, cz));
            for z in 0..CHUNK_SIZE {
                for y in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        let (got, was) = (a.get(x, y, z), b.get(x, y, z));
                        if got != was {
                            assert!(was.is_air() && got.is_shaped(), "{was:?} -> {got:?}");
                            added += 1;
                        }
                    }
                }
            }
        }
        assert!(added > 0);
    }

    #[test]
    fn the_minimum_rise_is_a_height_in_metres() {
        // A one-Voxel step is `voxel_size` metres, so the pass fires exactly
        // when that clears the threshold, whatever the Voxel size.
        let fires = |voxel_size: f32, min_rise_m: f32| {
            let defaults = WorldgenParams::default();
            let g = Worldgen::new(
                SEED,
                voxel_size,
                WorldgenParams {
                    caves: None,
                    smoothing: Some(SmoothingParams { min_rise_m }),
                    ..defaults
                },
            );
            sweep(voxel_size)
                .into_iter()
                .any(|(x, y, z)| g.chunk(x, y, z).may_contain_shaped())
        };
        for voxel_size in [1.0, 0.5, 0.25] {
            assert!(
                fires(voxel_size, voxel_size),
                "{voxel_size}m at its own height"
            );
            assert!(
                fires(voxel_size, voxel_size * 0.5),
                "{voxel_size}m under it"
            );
            assert!(
                !fires(voxel_size, voxel_size * 1.5),
                "{voxel_size}m over it"
            );
        }
    }

    #[test]
    fn the_default_smooths_a_1m_world_and_not_a_half_metre_one() {
        let defaults = WorldgenParams::default();
        let any = |voxel_size: f32| {
            let g = Worldgen::new(SEED, voxel_size, defaults);
            sweep(voxel_size)
                .into_iter()
                .any(|(x, y, z)| g.chunk(x, y, z).may_contain_shaped())
        };
        assert!(any(1.0));
        assert!(!any(0.5));
    }

    #[test]
    fn no_shaped_voxel_is_in_a_chunk_the_early_outs_skip() {
        // The two early-outs answer without running the pass, so they are sound
        // only if the pass could not have placed anything there.
        let g = smoothed(1.0, true);
        let (lowest, highest) = g.surface_bounds_m();
        let soil = WorldgenParams::default().soil_depth_m;
        let cell = CHUNK_SIZE as f32;
        let mut seen = 0;
        for cx in -4..4 {
            for cz in -4..4 {
                for cy in -3..5 {
                    if g.chunk(cx, cy, cz).shaped_count() == 0 {
                        continue;
                    }
                    seen += 1;
                    let floor_m = cy as f32 * cell;
                    let ceiling_m = floor_m + (cell - 1.0);
                    assert!(floor_m <= highest, "{cx},{cy},{cz} is above the band");
                    assert!(ceiling_m >= lowest - soil, "{cx},{cy},{cz} is below it");
                }
            }
        }
        assert!(seen > 0);
    }

    mod validate {
        use super::*;

        #[test]
        fn a_lake_must_be_made_of_a_registered_fluid() {
            let (mut tables, fluids) = lake_tables();
            let dry = tables
                .materials
                .register(crate::testgen::WATER_MATERIAL)
                .unwrap();
            let lake = |material| WorldgenParams::default().with_default_lake(material);
            for material in fluids {
                assert_eq!(lake(material).validate_lake(&tables), Ok(()));
            }
            assert_eq!(WorldgenParams::default().validate_lake(&tables), Ok(()));
            for (material, rule) in [
                (999, "names no registered Material"),
                (STONE, "is solid, not a fluid"),
                (dry, "has no fluid row"),
            ] {
                assert_eq!(
                    lake(material).validate_lake(&tables),
                    Err(ParamError {
                        field: "water.material",
                        rule
                    }),
                    "{material}"
                );
            }
        }

        fn field(params: WorldgenParams) -> &'static str {
            params.validate().expect_err("rejected").field
        }

        #[test]
        fn the_default_is_valid_with_and_without_its_options() {
            let d = WorldgenParams::default();
            assert_eq!(d.validate(), Ok(()));
            let bare = WorldgenParams {
                caves: None,
                smoothing: None,
                water: None,
                ..d
            };
            assert_eq!(bare.validate(), Ok(()));
        }

        #[test]
        fn a_value_that_is_not_a_finite_number_names_its_field() {
            for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                let d = WorldgenParams::default();
                let cave = CaveParams::default();
                assert_eq!(
                    field(WorldgenParams {
                        sea_level_m: bad,
                        ..d
                    }),
                    "sea_level_m"
                );
                assert_eq!(field(WorldgenParams { relief_m: bad, ..d }), "relief_m");
                assert_eq!(
                    field(WorldgenParams {
                        feature_size_m: bad,
                        ..d
                    }),
                    "feature_size_m"
                );
                assert_eq!(
                    field(WorldgenParams {
                        soil_depth_m: bad,
                        ..d
                    }),
                    "soil_depth_m"
                );
                let with = |caves| WorldgenParams {
                    caves: Some(caves),
                    ..d
                };
                assert_eq!(
                    field(with(CaveParams {
                        feature_size_m: bad,
                        ..cave
                    })),
                    "caves.feature_size_m"
                );
                assert_eq!(
                    field(with(CaveParams {
                        radius_m: bad,
                        ..cave
                    })),
                    "caves.radius_m"
                );
                assert_eq!(
                    field(with(CaveParams {
                        floor_m: bad,
                        ..cave
                    })),
                    "caves.floor_m"
                );
                let smoothing = Some(SmoothingParams { min_rise_m: bad });
                assert_eq!(
                    field(WorldgenParams { smoothing, ..d }),
                    "smoothing.min_rise_m"
                );
                let water = Some(WaterParams {
                    level_m: bad,
                    material: crate::voxel::AIR,
                });
                assert_eq!(field(WorldgenParams { water, ..d }), "water.level_m");
            }
        }

        #[test]
        fn sizes_depths_and_relief_have_their_floors() {
            let d = WorldgenParams::default();
            assert_eq!(
                field(WorldgenParams {
                    relief_m: -1.0,
                    ..d
                }),
                "relief_m"
            );
            assert_eq!(
                field(WorldgenParams {
                    soil_depth_m: -0.5,
                    ..d
                }),
                "soil_depth_m"
            );
            assert_eq!(
                field(WorldgenParams {
                    feature_size_m: 0.0,
                    ..d
                }),
                "feature_size_m"
            );
            assert_eq!(
                field(WorldgenParams {
                    feature_size_m: -96.0,
                    ..d
                }),
                "feature_size_m"
            );
            assert_eq!(
                field(WorldgenParams {
                    feature_size_m: 1e-40,
                    ..d
                }),
                "feature_size_m"
            );
            let cave = CaveParams {
                radius_m: 0.0,
                ..CaveParams::default()
            };
            assert_eq!(
                field(WorldgenParams {
                    caves: Some(cave),
                    ..d
                }),
                "caves.radius_m"
            );
            let smoothing = Some(SmoothingParams { min_rise_m: -0.1 });
            assert_eq!(
                field(WorldgenParams { smoothing, ..d }),
                "smoothing.min_rise_m"
            );
            assert_eq!(
                WorldgenParams {
                    relief_m: 0.0,
                    soil_depth_m: 0.0,
                    ..d
                }
                .validate(),
                Ok(())
            );
        }

        #[test]
        fn octaves_run_from_one_to_the_lattices_reach() {
            let d = WorldgenParams::default();
            assert_eq!(field(WorldgenParams { octaves: 0, ..d }), "octaves");
            assert_eq!(
                field(WorldgenParams {
                    octaves: MAX_OCTAVES + 1,
                    ..d
                }),
                "octaves"
            );
            for octaves in [1, MAX_OCTAVES] {
                assert_eq!(WorldgenParams { octaves, ..d }.validate(), Ok(()));
            }
            let cave = |octaves| CaveParams {
                octaves,
                ..CaveParams::default()
            };
            assert_eq!(
                field(WorldgenParams {
                    caves: Some(cave(0)),
                    ..d
                }),
                "caves.octaves"
            );
            assert_eq!(
                field(WorldgenParams {
                    caves: Some(cave(9)),
                    ..d
                }),
                "caves.octaves"
            );
        }

        #[test]
        fn the_surface_band_stays_within_reach_of_the_origin() {
            let d = WorldgenParams::default();
            assert_eq!(
                field(WorldgenParams {
                    sea_level_m: 1e9,
                    ..d
                }),
                "sea_level_m"
            );
            assert_eq!(field(WorldgenParams { relief_m: 1e9, ..d }), "relief_m");
            let high = WorldgenParams {
                sea_level_m: MAX_HEIGHT_M,
                relief_m: 1.0,
                ..d
            };
            assert_eq!(field(high), "relief_m");
        }
    }
}
