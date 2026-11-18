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

mod crossings;
mod fields;
mod ground;
mod layouts;
mod loot;
mod settlements;
mod structures;

pub use loot::{Loot, Stood};

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::chunk::{CHUNK_SIZE, Chunk, ChunkBuilder};
use crate::climate::{
    Biome, BiomeId, BiomeTable, Climate, ClimateField, ClimateParams, MAX_BIOMES, SkyFogBlend,
    Strata, SurfaceFeature, SurfaceFeatures, SurfacePlant, SurfacePlantKind, SurfaceTree,
    SurfaceVine,
};
use crate::loot::LootTable;
use crate::mesh::POS_Y;
use crate::noise::{Fbm3Cache, fbm3_cached, fbm3_range};
use crate::ore::{OreField, OreMask, OreTable};
use crate::plant::tree::{Part, fruit_stage, tree_voxels, vine_voxels};
use crate::plant::{CountRange, Fruit, Stage, Tree, first_fruit_offsets};
use crate::settlement::Settlement;
use crate::shape::{MAX_SHAPED_PER_CHUNK, Orientation, SHAPE_SLAB, SHAPE_STAIR};
use crate::tables::Tables;
use crate::template::Template;
use crate::terrain::{CAVERN_MIN_DEPTH_M, Caverns, LandformParams, Terrain};
use crate::underground::{
    AQUIFER_WALL_M, CAVE_BIOME_MIN_DEPTH_M, CaveBiomeTable, LAVA_DEPTH_M, LAVA_SEAL_M,
    SPRING_SPACING_M, aquifer_cell, aquifer_rise_m, cave_biome_at,
};
use crate::voxel::SHAPE_FULL_CUBE;
use crate::voxel::{MaterialId, Voxel};

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
    pub ground: Option<GroundParams>,
    pub climate: ClimateParams,
    /// Empty is a World with no Biomes.
    pub biomes: BiomeTable,
    /// Empty is a World without ore.
    pub ores: OreTable,
    /// A World pins the version it was created with, so a revision never reshapes existing terrain.
    pub generator: u32,
    pub landforms: LandformParams,
    /// The fluid version 3's deepest caverns hold; `None` leaves them open.
    pub lava: Option<MaterialId>,
    /// Empty leaves the underground as the surface Biomes lay it.
    pub cave_biomes: CaveBiomeTable,
}

/// A fluid filling the open air above the ground up to one height: the World's
/// lakes. Plain `Copy` data like the rest of [`WorldgenParams`]. A flat sheet
/// of packed cubes, which Occupants reads as sources: it fills any hollow open
/// to the sky below its level, but never a cave, which is carved out of the
/// ground and not above it.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct WaterParams {
    /// Not read: the generator's water stands at Sea level. Kept because the contract's
    /// `water-params` and the save still carry it.
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
#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
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
#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
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
            ground: None,
            climate: ClimateParams::default(),
            biomes: BiomeTable::default(),
            ores: OreTable::default(),
            generator: crate::save::GENERATOR_VERSION,
            landforms: LandformParams::default(),
            lava: None,
            cave_biomes: CaveBiomeTable::default(),
        }
    }
}

impl WorldgenParams {
    /// These parameters with a sea of `material` up to Sea level, and lakes and rivers inland.
    pub fn with_default_lake(self, material: MaterialId) -> Self {
        WorldgenParams {
            water: Some(WaterParams {
                level_m: self.sea_level_m,
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
        positive_size(
            "climate.temperature_size_m",
            self.climate.temperature_size_m,
        )?;
        positive_size("climate.humidity_size_m", self.climate.humidity_size_m)?;
        require(
            self.ores
                .as_slice()
                .iter()
                .all(|o| o.biomes.ids().all(|id| id.index() < self.biomes.len())),
            "ores.biomes",
            "must name Biomes the World declares",
        )?;
        let land = self.landforms;
        require(
            [land.depth_m, land.height_m, land.overhang_m]
                .iter()
                .all(|m| (0.0..=MAX_HEIGHT_M).contains(m)),
            "landforms",
            "must be finite and within 65536 m",
        )?;
        require(
            land.height_m >= self.relief_m && land.depth_m >= 0.0,
            "landforms.height_m",
            "must hold the relief above Sea level",
        )?;
        require(
            self.generator == crate::save::GENERATOR_VERSION,
            "generator",
            "must be a supported Generator version",
        )?;
        Ok(())
    }

    /// The same World with every Biome's ground bare: what the coarse levels draw (ADR-0011).
    pub fn without_surface_features(self) -> Self {
        let bare: Vec<Biome> = self
            .biomes
            .as_slice()
            .iter()
            .map(|b| Biome {
                surface_features: SurfaceFeatures::NONE,
                ..*b
            })
            .collect();
        WorldgenParams {
            biomes: BiomeTable::new(&bare)
                .and_then(|t| t.with_selectors(self.biomes.selectors()))
                .and_then(|t| t.with_strata(self.biomes.strata()))
                .expect("only the features changed"),
            ..self
        }
    }

    /// Checks the World's ground and every Biome's alike, and what the Biomes stand on it.
    pub fn validate_ground(&self, tables: &Tables) -> Result<(), ParamError> {
        let grounds = self
            .ground
            .into_iter()
            .chain(self.biomes.as_slice().iter().filter_map(|b| b.ground));
        for ground in grounds {
            for (field, material) in [
                ("ground.rock", ground.rock),
                ("ground.soil", ground.soil),
                ("ground.surface", ground.surface),
            ] {
                require(
                    tables.materials.contains(material),
                    field,
                    "names no registered Material",
                )?;
                require(
                    tables.materials.solid(Voxel::cube(material)),
                    field,
                    "is not solid",
                )?;
                require(!tables.fluids.contains(material), field, "is a fluid")?;
            }
        }
        for stratum in self.biomes.strata().iter().flat_map(|s| s.as_slice()) {
            let (field, material) = ("biomes.strata.material", stratum.material);
            require(
                tables.materials.contains(material),
                field,
                "names no registered Material",
            )?;
            require(
                tables.materials.solid(Voxel::cube(material)),
                field,
                "is not solid",
            )?;
            require(!tables.fluids.contains(material), field, "is a fluid")?;
        }
        let field = "biomes.surface_features.material";
        for feature in self
            .biomes
            .as_slice()
            .iter()
            .flat_map(|b| b.surface_features.as_slice())
        {
            if let Some(plant) = feature.plant {
                for m in plant.materials() {
                    require(
                        tables.materials.contains(m),
                        "biomes.surface_features.plant",
                        "names no registered Material",
                    )?;
                }
                continue;
            }
            let m = feature.material;
            require(
                tables.materials.contains(m),
                field,
                "names no registered Material",
            )?;
            require(
                tables.materials.solid(Voxel::cube(m)),
                field,
                "is not solid",
            )?;
            require(!tables.fluids.contains(m), field, "is a fluid")?;
        }
        for field in self
            .biomes
            .as_slice()
            .iter()
            .flat_map(|b| b.surface_features.crop_fields())
        {
            for m in field.materials() {
                require(
                    tables.materials.contains(m),
                    "biomes.crop_fields",
                    "names no registered Material",
                )?;
            }
            if let Some(path) = field.path {
                require(
                    tables.materials.solid(Voxel::cube(path)) && !tables.fluids.contains(path),
                    "biomes.crop_fields.path",
                    "is not a solid Material",
                )?;
            }
        }
        Ok(())
    }

    /// Whether the lake's Material can fill a lake: registered, and a fluid.
    /// Apart from [`Self::validate`] because it needs every fluid row, which a
    /// load has only once each Declaration is in.
    pub fn validate_lake(&self, tables: &Tables) -> Result<(), ParamError> {
        self.validate_ground(tables)?;
        for ditch in self
            .biomes
            .as_slice()
            .iter()
            .flat_map(|b| b.surface_features.crop_fields())
            .filter_map(|f| f.ditch)
        {
            require(
                tables.fluids.contains(ditch),
                "biomes.crop_fields.ditch",
                "has no fluid row",
            )?;
        }
        let Some(water) = self.water else {
            return Ok(());
        };
        require(self.ground.is_some(), "water", "requires ground")?;
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

    /// `scale` multiplies the tunnel radius; 1 is the World's own caves.
    fn carves(&self, x_m: f32, y_m: f32, z_m: f32, scale: f32) -> bool {
        self.carves_near(&mut Default::default(), x_m, y_m, z_m, scale)
    }

    /// [`CaveField::carves`] for a run of nearby samples sharing `cache`.
    fn carves_near(
        &self,
        cache: &mut [Fbm3Cache; 2],
        x_m: f32,
        y_m: f32,
        z_m: f32,
        scale: f32,
    ) -> bool {
        if y_m < self.floor_m {
            return false;
        }
        let k = self.inv_feature_size;
        let half_width = self.half_width * scale;
        self.seeds.iter().zip(cache).all(|(&s, cache)| {
            (fbm3_cached(x_m * k, y_m * k, z_m * k, s, self.octaves, cache) - 0.5).abs()
                < half_width
        })
    }

    /// Whether anything in the box `lo..=hi` (metres) could be carved. `false`
    /// is a proof, and the generator relies on it: it skips the box and gets
    /// what sampling every point would have. `true` only means it was not ruled
    /// out.
    ///
    /// `scale` must be at least the scale of every column in the box.
    fn may_carve(&self, lo: [f32; 3], hi: [f32; 3], scale: f32) -> bool {
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
            nearest < self.half_width * scale
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

/// How far round a Structure's or a Settlement's footprint the overhang density keeps off.
const OVERHANG_CLEAR_M: f32 = 6.0;

/// Columns a Structure or a Settlement claims ahead of fields, trees and scatter: `[min, max)` per
/// axis, x then z, or only those whose centre is in a disc inside that box.
#[derive(Clone, Copy, Debug)]
struct Footprint {
    min: [i32; 2],
    max: [i32; 2],
    /// Centre and radius, in Voxels.
    disc: Option<([f32; 2], f32)>,
}

impl Footprint {
    fn rect([min, max]: [[i32; 2]; 2]) -> Self {
        Footprint {
            min,
            max,
            disc: None,
        }
    }

    /// This footprint and `n` columns round it.
    fn widened(self, n: i32) -> Self {
        Footprint {
            min: self.min.map(|m| m - n),
            max: self.max.map(|m| m + n),
            disc: self.disc.map(|(centre, r)| (centre, r + n as f32)),
        }
    }

    fn covers(&self, gx: i32, gz: i32) -> bool {
        let boxed =
            (self.min[0]..self.max[0]).contains(&gx) && (self.min[1]..self.max[1]).contains(&gz);
        boxed
            && self.disc.is_none_or(|([cx, cz], r)| {
                let (dx, dz) = (gx as f32 + 0.5 - cx, gz as f32 + 0.5 - cz);
                dx * dx + dz * dz <= r * r
            })
    }
}

/// Whether column `(gx, gz)` is under any of `footprints`: what trees, scatter and fields keep off.
fn covers(footprints: &[Footprint], gx: i32, gz: i32) -> bool {
    footprints.iter().any(|f| f.covers(gx, gz))
}

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
    terrain: Terrain,
    /// The large chambers.
    caverns: Option<Caverns>,
    /// The highest ground over a column footprint, by its corners and size in metres, so the sky
    /// light does not wait on the band's global top.
    tops: Arc<Mutex<HashMap<[u32; 5], f32>>>,
    /// What every Chunk stacked over a column of Chunks reads alike, by `[cx, cz]`.
    stacks: Arc<Mutex<HashMap<[i32; 2], Arc<Stack>>>>,
    caves: Option<CaveField>,
    /// Whether the smoothing pass runs: configured, and a one-Voxel step is at
    /// least `min_rise_m` at this Voxel size.
    smoothing: bool,
    /// The World's water, its level Sea level.
    water: Option<WaterParams>,
    climate: ClimateField,
    biome_grounds: Option<BiomeGrounds>,
    /// Whether any Biome lays strata.
    strata: bool,
    /// Whether there are cave Biomes to choose.
    underground_biomes: bool,
    /// `None` without ores.
    ores: Option<OreField>,
    cave_tuning: Option<CaveTuning>,
    scatter: Option<SurfaceScatter>,
    /// How far Plants raise [`Self::surface_bounds_m`] above the terrain band; 0 without any.
    plant_rise_m: f32,
    /// Every Template the World has, which Structure rows and Settlements index.
    templates: Arc<[Template]>,
    structures: Option<structures::StructurePass>,
    settlements: Option<settlements::SettlementPass>,
    layouts: Arc<layouts::Layouts>,
    /// Each Biome's loot, by id: what its wilderness Structures' chests hold.
    biome_loot: Arc<[Option<LootTable>]>,
}

/// The water table, as how far under Sea level it lies: a cavern below it is flooded.
const WATER_TABLE_BELOW_SEA_M: f32 = 2.0;
/// How far a stratum's edges rise and fall, and over what span, so bedding never reads as a ruled
/// line.
const STRATA_WAVE_M: f32 = 3.0;
const STRATA_WAVE_SIZE_M: f32 = 48.0;
const STRATA_SALT: u32 = 0x5e7a_7a00;
const SALT_SPRING: u32 = 0x5971_0000;
/// How far round a Structure, or a Settlement's disc, paths and outskirts, version 3 opens no cave.
const CAVE_CLEAR_M: f32 = 16.0;
const SALT_CAVE_FEATURE: u32 = 0xcafe_0000;

/// How far down a column's strata are shifted at `(x_m, z_m)`, from 0 to [`STRATA_WAVE_M`]: one
/// field in metres, so every Voxel size and Chunk lays the same bedding.
fn strata_wave_m(seed: u32, x_m: f32, z_m: f32) -> f32 {
    let n = crate::noise::fbm2(
        x_m / STRATA_WAVE_SIZE_M,
        z_m / STRATA_WAVE_SIZE_M,
        seed ^ STRATA_SALT,
        2,
    );
    n * STRATA_WAVE_M
}

/// How many of its crown radii a tree keeps clear of other trunks: crowns that meet without
/// closing into one roof.
const CROWN_SPACING: f32 = 1.5;

/// Voxels of slack over a Plant's declared rise, for the base's own cell and rounding up.
const PLANT_RISE_SLACK_VOXELS: f32 = 3.0;

/// Salts the draw that varies a Plant's Stage and run, apart from the one that picks it.
const PLANT_VARY_SALT: u64 = 0x91a7_0000_5eed_0001;

/// A Plant a Biome stands on one column: where, and the draw that varies its Stage and height.
struct Standing {
    plant: SurfacePlant,
    base: [i32; 3],
    vary: f64,
}

/// Per feature index, so two clumped features of a Biome gather in different places.
const CLUMP_SALT: u32 = 0x9e37_79b9;

/// Keeps the feature draw apart from every noise field, which hash the folded seed directly.
const SCATTER_SEED: u64 = 0x5ca7_7e12_f00d_0001;

/// Which feature, if any, stands on a column. Built only when some Biome declares one, so a World
/// without features is generated exactly as before they existed.
#[derive(Clone, Copy, Debug)]
struct SurfaceScatter {
    /// A column's chance of one feature spaced 1 m apart: `voxel_size²`, since a column is that
    /// much ground.
    unit_chance: f64,
    /// No draw at or above this picks a feature in any Biome, so most columns never ask their Biome.
    most: f64,
    plants: bool,
    /// The tallest a Plant feature stands above its base, in metres; 0 with none.
    rise_m: f32,
    /// The widest a tree or vine feature reaches from its column, in metres; 0 with none.
    reach_m: f32,
    /// Whether some Biome declares planted fields.
    fields: bool,
}

impl SurfaceScatter {
    #[expect(
        clippy::disallowed_methods,
        reason = "generation: generator-determinism ticket 12 makes this a multiplication"
    )]
    fn new(params: &WorldgenParams, voxel_size: f32) -> Option<Self> {
        params.ground?;
        let unit_chance = f64::from(voxel_size).powi(2);
        let most = params
            .biomes
            .as_slice()
            .iter()
            .map(|b| Self::total(unit_chance, &b.surface_features))
            .fold(0.0, f64::max);
        let plants = || {
            params
                .biomes
                .as_slice()
                .iter()
                .flat_map(|b| b.surface_features.as_slice())
                .filter_map(|f| f.plant)
        };
        let rise_m = plants().map(|p| p.rise_m()).fold(0.0, f32::max);
        let reach_m = plants()
            .filter_map(|p| match p.kind {
                SurfacePlantKind::Tree(t) => Some(t.reach_m() + t.rounding_slack_m(voxel_size)),
                SurfacePlantKind::Vine(v) => Some(v.reach_m),
                _ => None,
            })
            .fold(0.0, f32::max);
        let fields = params
            .biomes
            .as_slice()
            .iter()
            .any(|b| !b.surface_features.crop_fields().is_empty());
        (most > 0.0 || fields).then_some(SurfaceScatter {
            unit_chance,
            most,
            plants: plants().next().is_some(),
            rise_m,
            reach_m,
            fields,
        })
    }

    #[expect(
        clippy::disallowed_methods,
        reason = "generation: generator-determinism ticket 12 makes this a multiplication"
    )]
    fn chance(unit_chance: f64, spacing_m: f32) -> f64 {
        (unit_chance / f64::from(spacing_m).powi(2)).min(1.0)
    }

    /// The most a Biome's chances sum to, each feature at its clump's densest, so `most` bounds
    /// every column's draw.
    fn total(unit_chance: f64, features: &SurfaceFeatures) -> f64 {
        features
            .as_slice()
            .iter()
            .map(|f| {
                let densest = f.clump.map_or(1.0, |c| c.max_factor());
                (Self::chance(unit_chance, f.spacing_m) * densest).min(1.0)
            })
            .sum()
    }

    /// One draw against the features' chances laid end to end, so each keeps its own density.
    ///
    /// A Biome whose chances sum past 1 (16 features at 4 m spacing and 1 m Voxels reach exactly 1)
    /// leaves the last ones no draw to win: the earlier declaration takes the column, the same on
    /// every Chunk and Voxel size, and a World recorded that way keeps its digests.
    ///
    /// A clump multiplies its feature's chance by its field at the column, which is evaluated here
    /// and only for columns whose draw passed `most`, once per clumped feature.
    fn pick<'a>(
        &self,
        features: &'a SurfaceFeatures,
        draw: f64,
        seed: u32,
        x_m: f32,
        z_m: f32,
    ) -> Option<&'a SurfaceFeature> {
        let mut reach = 0.0;
        let (index, feature) = features.as_slice().iter().enumerate().find(|&(i, f)| {
            let mut chance = Self::chance(self.unit_chance, f.spacing_m);
            if let Some(clump) = f.clump {
                let salt = CLUMP_SALT.wrapping_mul(i as u32 + 1);
                chance = (chance * clump.factor(seed.wrapping_add(salt), x_m, z_m)).min(1.0);
            }
            reach += chance;
            draw < reach
        })?;
        // A drawn member the region does not hold leaves the column bare, not to the next feature.
        fields::admits(features, index, seed, x_m, z_m).then_some(feature)
    }

    /// Uniform in `[0, 1)`, from the seed and the column's metre position alone.
    fn draw(seed: u32, x_m: f32, z_m: f32) -> f64 {
        Self::draw_salted(seed, x_m, z_m, 0)
    }

    /// A second draw for the same column, independent of the first, for what a Plant varies.
    fn draw_salted(seed: u32, x_m: f32, z_m: f32, salt: u64) -> f64 {
        let mix = |mut z: u64| {
            z = z.wrapping_add(0x9e37_79b9_7f4a_7c15);
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^ (z >> 31)
        };
        let column = u64::from(x_m.to_bits()) << 32 | u64::from(z_m.to_bits());
        let h = mix(mix(SCATTER_SEED ^ u64::from(seed) ^ salt) ^ column);
        (h >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// Offsets from the World's scale of 1, so untuned Biomes blend to exactly 1. Built only when some
/// Biome tunes the caves: otherwise no column needs its Climate.
#[derive(Clone, Copy, Debug)]
struct CaveTuning {
    offsets: [f32; MAX_BIOMES],
    /// Every early-out must hold to this: a blend never leaves the range of the scales it mixes.
    max_scale: f32,
}

impl CaveTuning {
    fn new(params: &WorldgenParams) -> Option<Self> {
        let tuned = |b: &crate::Biome| b.cave_scale.filter(|&s| s != 1.0);
        if params.caves.is_none() || !params.biomes.as_slice().iter().any(|b| tuned(b).is_some()) {
            return None;
        }
        let mut offsets = [0.0; MAX_BIOMES];
        for (slot, biome) in offsets.iter_mut().zip(params.biomes.as_slice()) {
            *slot = biome.cave_scale.unwrap_or(1.0) - 1.0;
        }
        let max_scale = 1.0 + offsets.iter().copied().fold(0.0, f32::max);
        Some(CaveTuning { offsets, max_scale })
    }
}

struct ColumnCaves {
    scales: [f32; CHUNK_SIZE * CHUNK_SIZE],
    max: f32,
}

/// Omissions filled from the World's, so a column's Materials are one index away. Built only when
/// some Biome's ground differs: otherwise no column needs its Biome looked up.
#[derive(Clone, Copy, Debug)]
struct BiomeGrounds {
    world: GroundParams,
    by_biome: [GroundParams; MAX_BIOMES],
    /// Lets the deep early-out answer without asking which Biome each column is in.
    same_rock: bool,
}

impl BiomeGrounds {
    fn new(params: &WorldgenParams) -> Option<Self> {
        let world = params.ground?;
        if !params
            .biomes
            .as_slice()
            .iter()
            .any(|b| b.ground.is_some_and(|g| g != world))
        {
            return None;
        }
        let mut by_biome = [world; MAX_BIOMES];
        for (slot, biome) in by_biome.iter_mut().zip(params.biomes.as_slice()) {
            *slot = biome.ground.unwrap_or(world);
        }
        Some(BiomeGrounds {
            world,
            same_rock: by_biome.iter().all(|g| g.rock == world.rock),
            by_biome,
        })
    }
}

type ColumnGrounds = [GroundParams; CHUNK_SIZE * CHUNK_SIZE];

/// A column of Chunks' surface and water levels (with a one-column border) and grounds. Version
/// 3's band stands some ten Chunks tall, and each would otherwise sample all of it again.
#[derive(Debug)]
struct Stack {
    surface_m: [f32; SURFACE_W * SURFACE_W],
    water_level_m: [Option<f32>; SURFACE_W * SURFACE_W],
    /// How far the overhang density may push each column, before water and Structures limit it.
    overhang_m: [f32; CHUNK_SIZE * CHUNK_SIZE],
    /// Columns where a sea-floor formation lays bare rock instead of the ground's surface.
    rock: [bool; CHUNK_SIZE * CHUNK_SIZE],
    grounds: Option<ColumnGrounds>,
}

/// About 20 MB of stacks, far more than a streamed ring holds at once.
const MAX_STACKS: usize = 1024;

impl Worldgen {
    pub fn new(seed: u64, voxel_size: f32, params: WorldgenParams) -> Self {
        debug_assert!(voxel_size > 0.0, "a Voxel has an edge length");
        debug_assert!(params.feature_size_m > 0.0, "features have a size");
        let folded = (seed ^ (seed >> 32)) as u32;
        let scatter = SurfaceScatter::new(&params, voxel_size);
        Worldgen {
            voxel_size,
            params,
            seed: folded,
            terrain: Terrain::new(folded, &params),
            tops: Arc::default(),
            stacks: Arc::default(),
            // A Game with cave Biomes and no tunnels still has caverns for them.
            caverns: (params.caves.is_some() || !params.cave_biomes.is_empty())
                .then(|| Caverns::new(folded, &params)),
            caves: params.caves.map(|p| CaveField::new(folded, p)),
            smoothing: params
                .smoothing
                .is_some_and(|p| EASED_RISE_VOXELS as f32 * voxel_size >= p.min_rise_m),
            water: params.water.map(|w| WaterParams {
                level_m: params.sea_level_m,
                ..w
            }),
            climate: ClimateField::new(folded, params.climate),
            biome_grounds: BiomeGrounds::new(&params),
            strata: params
                .biomes
                .strata()
                .iter()
                .any(|s| !s.as_slice().is_empty()),
            underground_biomes: !params.cave_biomes.is_empty(),
            ores: OreField::new(folded, &params.ores),
            cave_tuning: CaveTuning::new(&params),
            plant_rise_m: scatter
                .filter(|s| s.rise_m > 0.0)
                .map_or(0.0, |s| s.rise_m + PLANT_RISE_SLACK_VOXELS * voxel_size),
            scatter,
            templates: Arc::new([]),
            structures: None,
            settlements: None,
            layouts: Arc::default(),
            biome_loot: Arc::new([]),
        }
    }

    /// This generator standing the Structures its Biomes list, each row naming one of
    /// `templates` by index. Rows naming no Template stand nothing.
    pub fn with_templates(mut self, templates: &[Template]) -> Self {
        self.structures = structures::StructurePass::new(&self.params, self.voxel_size, templates);
        self.templates = templates.into();
        self.layouts = Arc::default();
        self
    }

    /// This generator laying out the Settlements its Biomes list, each row naming one of
    /// `settlements` by index, whose Buildings name Templates given to [`Self::with_templates`]
    /// first. Rows naming no Settlement lay out nothing.
    pub fn with_settlements(mut self, settlements: &[Settlement]) -> Self {
        self.settlements = settlements::SettlementPass::new(
            &self.params,
            self.voxel_size,
            &self.templates,
            settlements,
        );
        self.layouts = Arc::default();
        self
    }

    pub fn params(&self) -> WorldgenParams {
        self.params
    }

    /// The Biomes by reference: [`Self::params`] copies every Biome and their features.
    pub fn biomes(&self) -> &BiomeTable {
        &self.params.biomes
    }

    /// The edge of the Voxels this generator makes, in metres.
    pub fn voxel_size(&self) -> f32 {
        self.voxel_size
    }

    /// The sea's level is Sea level; ask [`Self::water_level_m`] for a column's own.
    pub fn water(&self) -> Option<WaterParams> {
        self.water
    }

    /// The height up to which open air over a column fills with the Game's fluid, if the Game
    /// declared one. The ground there may stand above it, which is a dry column.
    pub fn water_level_m(&self, x_m: f32, z_m: f32) -> Option<f32> {
        let sea = self.water?.level_m;
        Some(
            self.terrain
                .inland_water_m(x_m, z_m)
                .map_or(sea, |l| l.max(sea)),
        )
    }

    /// The dry column nearest `from`, searched outwards to `max_m`, for a Player to spawn on: not
    /// under water, a metre or more above it, no cliff within a few metres, and dry ground all
    /// round to ~100 m so the Player is on land and not a spit or an islet.
    #[expect(
        clippy::disallowed_methods,
        reason = "generation: generator-determinism ticket 12 moves this to libm"
    )]
    pub fn dry_land_near_m(&self, from: [f32; 2], max_m: f32) -> Option<[f32; 2]> {
        const STEP_M: f32 = 16.0;
        const CLIFF_M: f32 = 6.0;
        const CLIFF_DROP_M: f32 = 4.0;
        const SHORE_RINGS_M: [f32; 2] = [48.0, 100.0];
        let dry = |x: f32, z: f32| {
            self.water_level_m(x, z)
                .is_none_or(|level_m| self.surface_height_m(x, z) >= level_m + 0.5)
        };
        let suits = |x: f32, z: f32| {
            let h = self.surface_height_m(x, z);
            let clear_of_water = self
                .water_level_m(x, z)
                .is_none_or(|level_m| h >= level_m + 1.0);
            clear_of_water
                && SHORE_RINGS_M.iter().all(|&r| {
                    (0..8).all(|k| {
                        let a = k as f32 * std::f32::consts::FRAC_PI_4;
                        dry(x + r * a.cos(), z + r * a.sin())
                    })
                })
                && (0..8).all(|k| {
                    let a = k as f32 * std::f32::consts::FRAC_PI_4;
                    let (sx, sz) = (x + CLIFF_M * a.cos(), z + CLIFF_M * a.sin());
                    (self.surface_height_m(sx, sz) - h).abs() <= CLIFF_DROP_M
                })
        };
        if suits(from[0], from[1]) {
            return Some(from);
        }
        let rings = (max_m / STEP_M) as u32;
        for ring in 1..=rings {
            let r = ring as f32 * STEP_M;
            let n = ((std::f32::consts::TAU * r / STEP_M).ceil() as u32).max(6);
            for k in 0..n {
                let a = k as f32 * std::f32::consts::TAU / n as f32;
                let (x, z) = (from[0] + r * a.cos(), from[1] + r * a.sin());
                if suits(x, z) {
                    return Some([x, z]);
                }
            }
        }
        None
    }

    /// Where a new Player spawns: inside the Settlement nearest the origin, searched in widening
    /// rings, else the dry land nearest the origin, else the origin.
    pub fn spawn_m(&self) -> [f32; 2] {
        // Settlements stand only on level ground, so the nearest may be kilometres out.
        const SETTLEMENT_RINGS_M: [f32; 4] = [512.0, 1024.0, 2048.0, 4096.0];
        const DRY_LAND_SEARCH_M: f32 = 6000.0;
        // A Settlement's disc is level dry ground already; only the origin needs looking round.
        SETTLEMENT_RINGS_M
            .iter()
            .find_map(|&r| self.settlement_spawn_m(r))
            .or_else(|| self.dry_land_near_m([0.0; 2], DRY_LAND_SEARCH_M))
            .unwrap_or([0.0; 2])
    }

    /// The Water surface over a column: the height of the water standing or running on it, `None`
    /// where the ground is above the water or the Game declared no fluid.
    pub fn water_surface_m(&self, x_m: f32, z_m: f32) -> Option<f32> {
        self.water_level_m(x_m, z_m)
            .filter(|&level_m| self.surface_height_m(x_m, z_m) < level_m)
    }

    pub fn climate_at(&self, x_m: f32, z_m: f32) -> Climate {
        self.climate.at(x_m, z_m)
    }

    /// The Climate at a height: the air is colder higher up and wetter near the sea, as Weather
    /// will read it.
    pub fn climate_at_height(&self, x_m: f32, y_m: f32, z_m: f32) -> Climate {
        let c = self.climate.at(x_m, z_m);
        let (_, (_, humidity)) = self.terrain.site(x_m, z_m);
        let (temperature, _) =
            crate::terrain::climate_shift(y_m - self.params.sea_level_m, f32::MAX);
        Climate {
            temperature: (c.temperature + temperature).clamp(0.0, 1.0),
            humidity: (c.humidity + humidity).clamp(0.0, 1.0),
        }
    }

    /// `None` where the World has no Biomes. Pure in the seed and the position, so the same at
    /// every Voxel size and LOD level.
    pub fn biome_at(&self, x_m: f32, z_m: f32) -> Option<BiomeId> {
        let site = self.terrain.site(x_m, z_m);
        self.climate.biome_at(&self.params.biomes, x_m, z_m, site)
    }

    /// The ground the column at global Voxel `(gx, gz)` is made of, as the Chunk holding it lays
    /// it: its Biome's, or the World's. `None` in a World without ground.
    pub fn column_ground(&self, gx: i32, gz: i32) -> Option<GroundParams> {
        let Some(grounds) = &self.biome_grounds else {
            return self.params.ground;
        };
        Some(
            self.biome_at(self.global_m(gx), self.global_m(gz))
                .map_or(grounds.world, |id| grounds.by_biome[id.index()]),
        )
    }

    /// `None` where no Biome has colours. Pure like [`Self::biome_at`].
    pub fn sky_fog_at(&self, x_m: f32, z_m: f32) -> Option<SkyFogBlend> {
        let site = self.terrain.site(x_m, z_m);
        self.climate.sky_fog_at(&self.params.biomes, x_m, z_m, site)
    }

    /// `None` when every column has the World's. Asked at the metre position of the column's
    /// surface, so every LOD level reads the same Biome there.
    fn column_grounds(&self, cx: i32, cz: i32) -> Option<ColumnGrounds> {
        let grounds = self.biome_grounds.as_ref()?;
        Some(std::array::from_fn(|i| {
            let (x, z) = ((i % CHUNK_SIZE) as i32, (i / CHUNK_SIZE) as i32);
            self.biome_at(self.axis_m(cx, x), self.axis_m(cz, z))
                .map_or(grounds.world, |id| grounds.by_biome[id.index()])
        }))
    }

    /// The stack over Chunk column `[cx, cz]`, built on first use. Two workers may both build one;
    /// they build the same.
    fn stack(&self, cx: i32, cz: i32) -> Arc<Stack> {
        if let Some(s) = self.stacks.lock().expect("stacks").get(&[cx, cz]) {
            return Arc::clone(s);
        }
        // The surface is kept to one column past the Chunk; its containment reads one further.
        const RAW_W: usize = SURFACE_W + 2;
        let raw_index = |x: i32, z: i32| (z + 2) as usize * RAW_W + (x + 2) as usize;
        let mut raw = [(0.0f32, None); RAW_W * RAW_W];
        for z in -2..CHUNK_SIZE as i32 + 2 {
            for x in -2..CHUNK_SIZE as i32 + 2 {
                raw[raw_index(x, z)] = self.raw_column(self.axis_m(cx, x), self.axis_m(cz, z));
            }
        }
        let mut surface_m = [0.0f32; SURFACE_W * SURFACE_W];
        let mut water_level_m = [None; SURFACE_W * SURFACE_W];
        for z in -1..CHUNK_SIZE as i32 + 1 {
            for x in -1..CHUNK_SIZE as i32 + 1 {
                let around = CARDINALS.map(|(dx, dz, _)| raw[raw_index(x + dx, z + dz)]);
                surface_m[surface_index(x, z)] = self.contained_m(raw[raw_index(x, z)], around);
                water_level_m[surface_index(x, z)] = raw[raw_index(x, z)].1;
            }
        }
        let terrain = &self.terrain;
        let overhang_m = std::array::from_fn(|i| {
            let (x, z) = ((i % CHUNK_SIZE) as i32, (i / CHUNK_SIZE) as i32);
            terrain.overhang_amp_m(self.axis_m(cx, x), self.axis_m(cz, z))
        });
        let sea_m = self.params.sea_level_m;
        let rock = std::array::from_fn(|i| {
            let (x, z) = ((i % CHUNK_SIZE) as i32, (i / CHUNK_SIZE) as i32);
            surface_m[surface_index(x, z)] < sea_m - 1.0
                && terrain.seabed_rock(self.axis_m(cx, x), self.axis_m(cz, z))
        });
        let stack = Arc::new(Stack {
            surface_m,
            water_level_m,
            overhang_m,
            rock,
            grounds: self.column_grounds(cx, cz),
        });
        let mut stacks = self.stacks.lock().expect("stacks");
        if stacks.len() >= MAX_STACKS {
            stacks.clear();
        }
        stacks.insert([cx, cz], Arc::clone(&stack));
        stack
    }

    /// What the overhang density keeps off around the Chunk column `[cx, cz]`: Structures stand
    /// on the heightfield, and the density would bury a wall or lean over a roof.
    fn overhang_kept_off(&self, [cx, cz]: [i32; 2]) -> Vec<Footprint> {
        let clear = (OVERHANG_CLEAR_M / self.voxel_size).ceil() as i32;
        let claims = self.claims_near(self.chunk_columns([cx, cz], clear));
        self.footprints(&claims)
            .into_iter()
            .map(|f| f.widened(clear))
            .collect()
    }

    /// How far the density pushes column `[x, z]` of the Chunk column `[cx, cz]`: nowhere on
    /// `kept_off`, and never down to water here or beside, where it would open a hollow that
    /// water would stand or pour into.
    fn overhang_amp_at(
        &self,
        s: &Stack,
        kept_off: &[Footprint],
        [cx, cz]: [i32; 2],
        [x, z]: [i32; 2],
    ) -> f32 {
        let size = CHUNK_SIZE as i32;
        if covers(kept_off, cx * size + x, cz * size + z) {
            return 0.0;
        }
        let wettest_m = CARDINALS
            .iter()
            .map(|&(dx, dz, _)| surface_index(x + dx, z + dz))
            .filter(|&i| s.water_level_m[i].is_some_and(|l| l > s.surface_m[i]))
            .chain([surface_index(x, z)])
            .filter_map(|i| s.water_level_m[i])
            .fold(f32::MIN, f32::max);
        s.overhang_m[z as usize * CHUNK_SIZE + x as usize]
            .min((s.surface_m[surface_index(x, z)] - wettest_m).max(0.0))
    }

    /// The highest Voxel the cube pass fills in global column `(gx, gz)`, the overhang density's
    /// push above the surface included. Caves only take away, so they are not asked.
    fn ground_top(&self, gx: i32, gz: i32) -> i32 {
        let (x_m, z_m) = (self.global_m(gx), self.global_m(gz));
        let terrain = &self.terrain;
        if terrain.overhang_max_m() <= 0.0 {
            return top_index(self.surface_height_m(x_m, z_m), 0.0, self.voxel_size);
        }
        let size = CHUNK_SIZE as i32;
        let (cx, cz) = (gx.div_euclid(size), gz.div_euclid(size));
        let (x, z) = (gx.rem_euclid(size), gz.rem_euclid(size));
        let stack = self.stack(cx, cz);
        let surface_m = stack.surface_m[surface_index(x, z)];
        let top = top_index(surface_m, 0.0, self.voxel_size);
        let amp_m =
            self.overhang_amp_at(&stack, &self.overhang_kept_off([cx, cz]), [cx, cz], [x, z]);
        if amp_m <= 0.0 {
            return top;
        }
        // The Chunk's own test, Voxel by Voxel down from the highest the push can reach.
        (top + 1..=top_index(surface_m + amp_m, 0.0, self.voxel_size))
            .rev()
            .find(|&y| {
                let y_m = self.global_m(y);
                let base_m = surface_m - y_m;
                base_m.abs() <= amp_m
                    && base_m + terrain.overhang_push_m(x_m, y_m, z_m, amp_m) >= 0.0
            })
            .unwrap_or(top)
    }

    /// The ground and water level over a column before either is reconciled with its
    /// neighbours'.
    fn raw_column(&self, x_m: f32, z_m: f32) -> (f32, Option<f32>) {
        (
            self.terrain.height_m(x_m, z_m),
            self.water_level_m(x_m, z_m),
        )
    }

    /// The highest Voxel the water over a column fills, if it stands above the ground: the cube
    /// pass's own comparisons, `y_m < level` for water and `top_index` for ground.
    fn water_top(&self, (ground_m, level_m): (f32, Option<f32>)) -> Option<i32> {
        let k = self.level_top(level_m?);
        (k > top_index(ground_m, 0.0, self.voxel_size)).then_some(k)
    }

    /// The highest Voxel water standing at `level_m` fills, whatever the ground under it.
    fn level_top(&self, level_m: f32) -> i32 {
        let mut k = (level_m / self.voxel_size).ceil() as i32;
        while k as f32 * self.voxel_size >= level_m {
            k -= 1;
        }
        k
    }

    /// A column's surface once its water is contained: where a neighbour's water stands higher
    /// than this column's own, the ground rises to hold it as a rock lip, so no generated water
    /// has open air beside it for the fluid simulation to pour into. Equal tops meet as one
    /// surface. Asked on the Voxel lattice, so each Voxel size draws its own lips.
    fn contained_m(&self, own: (f32, Option<f32>), around: [(f32, Option<f32>); 4]) -> f32 {
        let wet = self.water_top(own);
        let held = around
            .into_iter()
            .filter_map(|q| self.water_top(q))
            .max()
            .filter(|&h| wet.is_none_or(|w| h > w));
        match held {
            Some(h) if h > top_index(own.0, 0.0, self.voxel_size) => {
                (h as f32 + 0.5) * self.voxel_size
            }
            _ => own.0,
        }
    }

    /// [`Self::contained_m`] at a point, its neighbours a Voxel away along each axis, reading their
    /// ground only where their water could stand above this column's top.
    fn contained_at_m(&self, [x_m, z_m]: [f32; 2], around: [[f32; 2]; 4]) -> f32 {
        let own = self.raw_column(x_m, z_m);
        let reach =
            self.water_top(own)
                .unwrap_or(i32::MIN)
                .max(top_index(own.0, 0.0, self.voxel_size));
        let around = around.map(|[x_m, z_m]| {
            let level_m = self.water_level_m(x_m, z_m);
            if level_m.is_none_or(|l| self.level_top(l) <= reach) {
                (own.0, None)
            } else {
                self.raw_column(x_m, z_m)
            }
        });
        self.contained_m(own, around)
    }

    /// 1 where no Biome tunes the caves, otherwise a mean that moves continuously across a border,
    /// unlike the ground it sits beside.
    fn cave_scale_at(&self, x_m: f32, z_m: f32) -> f32 {
        self.cave_tuning.as_ref().map_or(1.0, |t| {
            1.0 + self
                .climate
                .blend(&self.params.biomes, x_m, z_m, &t.offsets)
        })
    }

    /// Lets a box be ruled out before its columns are asked.
    fn cave_scale_bound(&self) -> f32 {
        self.cave_tuning.as_ref().map_or(1.0, |t| t.max_scale)
    }

    /// `None` when no Biome tunes the caves. Asked at the same metre position as the Voxels.
    /// The columns of the Chunk at `[cx, cz]` that no cave may open in: within
    /// [`CAVE_CLEAR_M`] of a Structure's footprint, or of a Settlement's disc, approach paths or
    /// outskirts. `None` where nothing stands near.
    fn cave_free(&self, cx: i32, cz: i32) -> Option<[bool; CHUNK_SIZE * CHUNK_SIZE]> {
        if self.caves.is_none() && self.caverns.is_none() {
            return None;
        }
        let clear = (CAVE_CLEAR_M / self.voxel_size).ceil() as i32;
        let near: Vec<Footprint> = self
            .footprints_near(self.chunk_columns([cx, cz], clear))
            .into_iter()
            .map(|f| f.widened(clear))
            .collect();
        if near.is_empty() {
            return None;
        }
        let size = CHUNK_SIZE as i32;
        let free: [bool; CHUNK_SIZE * CHUNK_SIZE] = std::array::from_fn(|i| {
            let (x, z) = ((i % CHUNK_SIZE) as i32, (i / CHUNK_SIZE) as i32);
            covers(&near, cx * size + x, cz * size + z)
        });
        free.contains(&true).then_some(free)
    }

    /// [`Self::column_caves`] with no tunnel in the columns `free` keeps clear.
    fn column_caves_clear_of(
        &self,
        cx: i32,
        cz: i32,
        free: Option<&[bool; CHUNK_SIZE * CHUNK_SIZE]>,
    ) -> Option<ColumnCaves> {
        let Some(free) = free else {
            return self.column_caves(cx, cz);
        };
        let mut columns = self.column_caves(cx, cz).unwrap_or(ColumnCaves {
            scales: [1.0; CHUNK_SIZE * CHUNK_SIZE],
            max: 1.0,
        });
        for (scale, &kept) in columns.scales.iter_mut().zip(free) {
            if kept {
                *scale = 0.0;
            }
        }
        columns.max = columns.scales.iter().copied().fold(0.0, f32::max);
        Some(columns)
    }

    fn column_caves(&self, cx: i32, cz: i32) -> Option<ColumnCaves> {
        self.cave_tuning.as_ref()?;
        let mut columns = ColumnCaves {
            scales: [1.0; CHUNK_SIZE * CHUNK_SIZE],
            max: 0.0,
        };
        for (i, scale) in columns.scales.iter_mut().enumerate() {
            let (x, z) = ((i % CHUNK_SIZE) as i32, (i / CHUNK_SIZE) as i32);
            *scale = self.cave_scale_at(self.axis_m(cx, x), self.axis_m(cz, z));
            columns.max = columns.max.max(*scale);
        }
        Some(columns)
    }

    /// A height in metres above which nothing generated stands anywhere over the footprint
    /// `min_m..max_m` on the ground plane: the footprint's own highest ground with the overhang and what Plants and Structures stand
    /// above it, so a Chunk over low ground sees sky without the Chunks above it, which the
    /// Server does not hold, reaching the band's top.
    pub fn surface_top_m(&self, min_m: [f32; 2], max_m: [f32; 2]) -> f32 {
        let (_, global) = self.surface_bounds_m();
        let terrain = &self.terrain;
        let key = [
            min_m[0].to_bits(),
            min_m[1].to_bits(),
            max_m[0].to_bits(),
            max_m[1].to_bits(),
            0,
        ];
        if let Some(&top) = self.tops.lock().unwrap().get(&key) {
            return top;
        }
        // Ground that rises between two samples is smooth to within the detail octaves, so a
        // fixed slack covers the gap; the lattice is no coarser than a few Voxels of detail.
        const STEP_M: f32 = 4.0;
        const SLACK_M: f32 = 8.0;
        let steps = |a: usize| ((max_m[a] - min_m[a]) / STEP_M).ceil() as u32 + 1;
        let mut highest = f32::MIN;
        for j in 0..steps(1) {
            for i in 0..steps(0) {
                let x = (min_m[0] + i as f32 * STEP_M).min(max_m[0]);
                let z = (min_m[1] + j as f32 * STEP_M).min(max_m[1]);
                // Water stands above ground that is under it, so its level counts.
                let ground = terrain.height_m(x, z);
                let water = self
                    .water_level_m(x, z)
                    .map_or(ground, |level_m| level_m.max(ground));
                highest = highest.max(water);
            }
        }
        let above_ground = global - self.terrain_band_m().1;
        let top = (highest + terrain.overhang_max_m() + above_ground + SLACK_M).min(global);
        let mut tops = self.tops.lock().unwrap();
        if tops.len() >= 8192 {
            tops.clear();
        }
        tops.insert(key, top);
        top
    }

    /// Drops the drainage regions and column stacks this generator has cached. Nothing it answers
    /// changes: each is built again, to the same result, when it is next asked.
    pub fn forget_regions(&self) {
        self.stacks.lock().expect("stacks").clear();
        self.terrain.forget_regions();
    }

    /// The ground in metres before water is contained, which is the same at every Voxel size:
    /// [`Self::surface_height_m`] rises from it only by a lip a Voxel wide beside water.
    pub fn terrain_height_m(&self, x_m: f32, z_m: f32) -> f32 {
        self.terrain.height_m(x_m, z_m)
    }

    /// The landform fields at a column.
    pub fn landform_fields(&self, x_m: f32, z_m: f32) -> crate::terrain::Fields {
        self.terrain.fields(x_m, z_m)
    }

    /// The metre height of the surface over a point on the ground plane: that of the Voxel column
    /// nearest it, with its water contained: what every Chunk stands on.
    pub fn surface_height_m(&self, x_m: f32, z_m: f32) -> f32 {
        let column = |m: f32| (m / self.voxel_size).round() as i32;
        let (gx, gz) = (column(x_m), column(z_m));
        // A Chunk asks at its columns' corners, the Stack's lattice; anything else asks its
        // own point and the points a Voxel from it.
        if self.global_m(gx) != x_m || self.global_m(gz) != z_m {
            let size = self.voxel_size;
            return self.contained_at_m(
                [x_m, z_m],
                CARDINALS.map(|(dx, dz, _)| [x_m + dx as f32 * size, z_m + dz as f32 * size]),
            );
        }
        let (cx, cz) = (
            gx.div_euclid(CHUNK_SIZE as i32),
            gz.div_euclid(CHUNK_SIZE as i32),
        );
        if let Some(s) = self.stacks.lock().expect("stacks").get(&[cx, cz]) {
            let (x, z) = (
                gx.rem_euclid(CHUNK_SIZE as i32),
                gz.rem_euclid(CHUNK_SIZE as i32),
            );
            return s.surface_m[surface_index(x, z)];
        }
        self.contained_at_m(
            [x_m, z_m],
            CARDINALS.map(|(dx, dz, _)| [self.global_m(gx + dx), self.global_m(gz + dz)]),
        )
    }

    /// The metre height above which nothing generated stands in any column
    /// under a footprint on the ground plane: each column's top Voxel, and a
    /// surface feature on it. Caves only take away and a lake is apart.
    pub fn clear_height_m(&self, min_m: [f32; 2], max_m: [f32; 2]) -> f32 {
        let size = self.voxel_size;
        let columns =
            |a: usize| (min_m[a] / size).floor() as i32..=(max_m[a] / size).floor() as i32;
        let mut clear_m = f32::MIN;
        let (xs, zs) = (columns(0), columns(1));
        let kept =
            self.footprints_near([[*xs.start(), *zs.start()], [*xs.end() + 1, *zs.end() + 1]]);
        for z in zs {
            for x in xs.clone() {
                let (x_m, z_m) = (self.axis_m(0, x), self.axis_m(0, z));
                let top = self.ground_top(x, z);
                let featured = self.scatter.is_some_and(|scatter| {
                    let draw = SurfaceScatter::draw(self.seed, x_m, z_m);
                    draw < scatter.most
                        && !covers(&kept, x, z)
                        && self
                            .biome_at(x_m, z_m)
                            .and_then(|id| self.params.biomes.get(id))
                            .and_then(|biome| {
                                scatter.pick(&biome.surface_features, draw, self.seed, x_m, z_m)
                            })
                            .is_some_and(|f| f.plant.is_none())
                });
                let voxels = top + 1 + i32::from(featured);
                clear_m = clear_m.max(voxels as f32 * size);
            }
        }
        if let Some(scatter) = &self.scatter {
            clear_m = clear_m.max(self.plants_clear_m(scatter, min_m, max_m));
        }
        let region = [
            [0, 1].map(|a| *columns(a).start()),
            [0, 1].map(|a| columns(a).end() + 1),
        ];
        if let Some(roof_m) = self.structures_clear_m(region) {
            clear_m = clear_m.max(roof_m);
        }
        clear_m
    }

    /// The band `surface_height_m` is confined to, as `(lowest, highest)` in
    /// metres: the landforms' declared band, which clamps every peak. The two
    /// early-outs in [`Worldgen::chunk`] and the Server's vertical streaming
    /// extent are both derived from it.
    ///
    /// Caves do not move it: they only turn solid Voxels into air, so nothing
    /// above the band is ever solid, which `streaming::relight`'s open-sky
    /// boundary relies on. Neither does smoothing: a stair needs a cube beside it
    /// in the same cell, so it sits no higher than the ground it eases. Nor does a
    /// surface feature: one stands only where its Voxel's floor is in the band. A Plant is the
    /// exception: a tree or a column stands above it, so the upper bound rises by the tallest
    /// one a Biome declares. So is a Structure, standing above or sunk below it by the tallest and
    /// deepest Template a Biome lists. A World with neither is bounded as before.
    pub fn surface_bounds_m(&self) -> (f32, f32) {
        let (lowest, highest) = self.ground_bounds_m();
        self.structure_band_m()
            .map_or((lowest, highest), |(low, high)| {
                (lowest.min(low), highest.max(high))
            })
    }

    /// [`Self::surface_bounds_m`] without Structures, which stand themselves in any Chunk they
    /// may reach: so a sky or deep-rock Chunk beside a tower or a cellar keeps its early-out.
    fn ground_bounds_m(&self) -> (f32, f32) {
        let (lowest, highest) = self.terrain_band_m();
        (lowest, highest + self.plant_rise_m)
    }

    /// The band the ground itself is in, below anything a Plant stands up to.
    fn terrain_band_m(&self) -> (f32, f32) {
        let land = self.params.landforms;
        let sea_m = self.params.sea_level_m;
        (
            sea_m - land.depth_m - land.overhang_m,
            sea_m + land.height_m + land.overhang_m,
        )
    }

    /// A Chunk of terrain at Chunk coordinates `(cx, cy, cz)`.
    pub fn chunk(&self, cx: i32, cy: i32, cz: i32) -> Chunk {
        let Some(ground) = self.params.ground else {
            return Chunk::air();
        };
        let size = CHUNK_SIZE as i32;
        let (lowest, highest) = self.ground_bounds_m();

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
            let mut chunk = Chunk::air();
            if self.structures_may_reach(floor_m, ceiling_m) {
                let claims = self.claims_near(self.chunk_columns([cx, cz], 0));
                self.stand_claims(&mut chunk, [cx, cy, cz], &claims);
            }
            return chunk;
        }
        if ceiling_m < lowest - self.solid_below_m() {
            let mut chunk = self.deep_rock(cx, cz, ground);
            if let Some((field, ores)) = self.ores_reaching(floor_m, ceiling_m) {
                self.sow_deep_rock(&mut chunk, [cx, cz], origin_m, field, ores);
            }
            // A Chunk that only *might* hold a tunnel stays `filled` unless a
            // Voxel is actually carved, so the light and mesh fast paths that
            // key off `uniform()` still fire for it.
            if let Some(caves) = &self.caves
                && caves.may_carve(
                    origin_m,
                    [origin_m[0] + reach_m, ceiling_m, origin_m[2] + reach_m],
                    self.cave_scale_bound(),
                )
            {
                let free = self.cave_free(cx, cz);
                let columns = self.column_caves_clear_of(cx, cz, free.as_ref());
                self.carve(&mut chunk, caves, origin_m, ceiling_m, columns.as_ref());
            }
            if self.structures_may_reach(floor_m, ceiling_m) {
                let claims = self.claims_near(self.chunk_columns([cx, cz], 0));
                self.stand_claims(&mut chunk, [cx, cy, cz], &claims);
            }
            return chunk;
        }

        let stack = self.stack(cx, cz);
        let surface_m = stack.surface_m;
        // Nothing above the highest column of this Chunk is solid, so nothing
        // there is worth carving. The border is another Chunk's ground.
        let top_m = (0..CHUNK_SIZE as i32)
            .flat_map(|z| (0..CHUNK_SIZE as i32).map(move |x| surface_index(x, z)))
            .map(|i| surface_m[i])
            .fold(f32::MIN, f32::max)
            + self.terrain.overhang_max_m();
        // A Chunk above every column holds only air and water, which take nothing from a Biome.
        let columns = if floor_m > top_m { None } else { stack.grounds };
        let reach_box = |caves: &CaveField, scale| {
            let ceiling_m = ceiling_m.min(top_m);
            floor_m <= ceiling_m
                && caves.may_carve(
                    origin_m,
                    [origin_m[0] + reach_m, ceiling_m, origin_m[2] + reach_m],
                    scale,
                )
        };
        let cave_free = self.cave_free(cx, cz);
        let cave_columns = self
            .caves
            .as_ref()
            .filter(|caves| reach_box(caves, self.cave_scale_bound()))
            .and_then(|_| self.column_caves_clear_of(cx, cz, cave_free.as_ref()));
        let free_at = |x: usize, z: usize| cave_free.is_some_and(|f| f[z * CHUNK_SIZE + x]);
        let reachable = self
            .caves
            .as_ref()
            .filter(|caves| {
                reach_box(
                    caves,
                    cave_columns
                        .as_ref()
                        .map_or(self.cave_scale_bound(), |c| c.max),
                )
            })
            .map(|caves| {
                let max = cave_columns.as_ref().map(|c| c.max);
                (
                    caves,
                    self.reachable_blocks(caves, origin_m, top_m, max.unwrap_or(1.0)),
                )
            });

        // Structures stand on the heightfield, so the density keeps off them and a margin round
        // them, where it would bury a wall or lean over a roof.
        let built = if self.terrain.overhang_max_m() > 0.0 && floor_m <= top_m {
            self.overhang_kept_off([cx, cz])
        } else {
            Default::default()
        };
        let cavern_blocks = self.caverns.as_ref().map(|c| {
            let mut blocks = [false; BLOCKS * BLOCKS * BLOCKS];
            let step_m = (BLOCK as f32) * self.voxel_size;
            let reach_m = (BLOCK - 1) as f32 * self.voxel_size;
            let roof_m = top_m - CAVERN_MIN_DEPTH_M;
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
                            (lo[1] + reach_m).min(roof_m),
                            lo[2] + reach_m,
                        ];
                        blocks[(bz * BLOCKS + by) * BLOCKS + bx] =
                            lo[1] <= hi[1] && c.may_carve(lo, hi);
                    }
                }
            }
            blocks
        });

        let shafts = self
            .caverns
            .as_ref()
            .filter(|_| floor_m <= top_m)
            .map(|c| {
                c.shafts(
                    [origin_m[0], origin_m[2]],
                    [origin_m[0] + reach_m, origin_m[2] + reach_m],
                    |x, z| self.surface_height_m(x, z),
                    |x, z| self.water_surface_m(x, z).is_none(),
                )
            })
            .unwrap_or_default();

        let ores = self.ores_reaching(floor_m, ceiling_m);
        let mut cave_biomes: HashMap<[i32; 3], Option<u8>> = HashMap::new();
        // Per Chunk, so the column loop asks nothing of version 3's underground where none is.
        let springs = self.caverns.is_some() && self.water.is_some();
        let mut chunk = ChunkBuilder::new();
        let mut cave_cache = Default::default();
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let x_m = origin_m[0] + x as f32 * self.voxel_size;
                let z_m = origin_m[2] + z as f32 * self.voxel_size;
                let surface_m = surface_m[surface_index(x as i32, z as i32)];
                let ground = columns
                    .as_ref()
                    .map_or(ground, |columns| columns[z * CHUNK_SIZE + x]);
                let ground = if stack.rock[z * CHUNK_SIZE + x] {
                    GroundParams {
                        surface: ground.rock,
                        soil: ground.rock,
                        ..ground
                    }
                } else {
                    ground
                };
                let biome = self.column_biome([cx, cz], [x, z]);
                let water_level_m = stack.water_level_m[surface_index(x as i32, z as i32)];
                let mut sealed_m = None;
                let mut wave_m = None;
                let table_m = self.water_table_m(x_m, z_m);
                // The density never reaches down to water here or beside, so it opens no hollow
                // that water would stand or pour into.
                let amp_m = self.overhang_amp_at(&stack, &built, [cx, cz], [x as i32, z as i32]);
                let push_m = |y_m: f32| self.terrain.overhang_push_m(x_m, y_m, z_m, amp_m);

                for y in 0..CHUNK_SIZE {
                    let y_m = floor_m + y as f32 * self.voxel_size;
                    let base_m = surface_m - y_m;
                    // Only within `amp_m` of the base surface does the density move it.
                    let banded = amp_m > 0.0 && base_m.abs() <= amp_m;
                    let below_m = if banded { base_m + push_m(y_m) } else { base_m };
                    if below_m < 0.0 {
                        if let (Some(water), Some(level_m)) = (self.water, water_level_m)
                            && y_m < level_m
                        {
                            chunk.set(x, y, z, Voxel::cube(water.material));
                            continue;
                        }
                        if base_m < -amp_m {
                            // y_m only rises, so the rest of the column is sky.
                            break;
                        }
                        continue;
                    }
                    let walled = || self.walled_in([x_m, y_m, z_m], table_m);
                    // A cave carves through the Grass cap like anything else,
                    // which is what a mouth is.
                    if let Some((caves, blocks)) = &reachable
                        && blocks[block_of(x, y, z)]
                        && caves.carves_near(
                            &mut cave_cache,
                            x_m,
                            y_m,
                            z_m,
                            cave_columns
                                .as_ref()
                                .map_or(1.0, |c| c.scales[z * CHUNK_SIZE + x]),
                        )
                        && y_m
                            >= *sealed_m
                                .get_or_insert_with(|| self.sealed_below_m(x_m, z_m, surface_m))
                        && !walled()
                    {
                        // An overhang's rock above the ground, cut by a cave, is open water, not a hole.
                        if base_m < 0.0
                            && let (Some(water), Some(level_m)) = (self.water, water_level_m)
                            && y_m < level_m
                        {
                            chunk.set(x, y, z, Voxel::cube(water.material));
                        }
                        continue;
                    }
                    if !free_at(x, z)
                        && shafts.iter().any(|s| s.cuts(x_m, y_m, z_m))
                        && y_m
                            >= *sealed_m
                                .get_or_insert_with(|| self.sealed_below_m(x_m, z_m, surface_m))
                        && !walled()
                    {
                        continue;
                    }
                    if let (Some(caverns), Some(blocks)) = (&self.caverns, &cavern_blocks)
                        && blocks[block_of(x, y, z)]
                        && !free_at(x, z)
                        && base_m >= CAVERN_MIN_DEPTH_M
                        && self.cavern_open(caverns, [x_m, y_m, z_m])
                        && y_m
                            >= *sealed_m
                                .get_or_insert_with(|| self.sealed_below_m(x_m, z_m, surface_m))
                    {
                        if let Some(fill) =
                            self.cavern_fill(caverns, [x_m, y_m, z_m], table_m, || {
                                self.cave_biome_cached(&mut cave_biomes, [x_m, y_m, z_m], base_m)
                            })
                        {
                            chunk.set(x, y, z, fill);
                        }
                        continue;
                    }
                    let exposed = if banded {
                        base_m - self.voxel_size + push_m(y_m + self.voxel_size) < 0.0
                    } else {
                        below_m < self.voxel_size
                    };
                    let material = if exposed {
                        ground.surface
                    } else if below_m < self.params.soil_depth_m {
                        ground.soil
                    } else if let Some(water) = self.water
                        && springs
                        && self.spring_at([x_m, y_m, z_m], base_m, table_m)
                    {
                        water.material
                    } else {
                        ores.and_then(|(field, mask)| {
                            field.at(mask, [x_m, y_m, z_m], below_m, &biome)
                        })
                        .or_else(|| {
                            if !self.underground_biomes {
                                return None;
                            }
                            let at = [x_m, y_m, z_m];
                            let cave = self.cave_biome_cached(&mut cave_biomes, at, base_m)?;
                            let cave = &self.params.cave_biomes.as_slice()[usize::from(cave)];
                            cave.floor
                                .filter(|_| {
                                    self.caverns.as_ref().is_some_and(|c| {
                                        let above = [x_m, y_m + self.voxel_size, z_m];
                                        base_m - self.voxel_size >= CAVERN_MIN_DEPTH_M
                                            && self.cavern_open(c, above)
                                    })
                                })
                                .or(cave.rock)
                        })
                        .or_else(|| {
                            let strata = self.strata.then(&biome)??;
                            let strata = &self.params.biomes.strata()[strata.index()];
                            let wave_m =
                                *wave_m.get_or_insert_with(|| strata_wave_m(self.seed, x_m, z_m));
                            strata.at(below_m, wave_m)
                        })
                        .unwrap_or(ground.rock)
                    };
                    chunk.set(x, y, z, Voxel::cube(material));
                }
            }
        }
        let mut chunk = chunk.build();
        if self.smoothing {
            self.smooth(
                &mut chunk,
                &surface_m,
                [cx, cy, cz],
                floor_m,
                columns.as_ref(),
            );
        }
        let margin = self
            .scatter
            .map_or(0, |s| (s.reach_m / self.voxel_size).ceil() as i32 + 1);
        // A tree a margin away reaches a margin further.
        let claims = self.claims_near(self.chunk_columns([cx, cz], 2 * margin));
        if self.structures_may_reach(floor_m, ceiling_m) {
            self.stand_claims(&mut chunk, [cx, cy, cz], &claims);
        }
        if let Some(scatter) = &self.scatter {
            let placed = self.fields_near(scatter, self.chunk_columns([cx, cz], margin));
            let kept = self.footprints(&claims);
            self.scatter(
                &mut chunk,
                scatter,
                &placed,
                &kept,
                &surface_m,
                [cx, cy, cz],
            );
            if scatter.plants {
                self.plants(&mut chunk, scatter, &placed, &kept, [cx, cy, cz]);
            }
            self.crop_fields(&mut chunk, &placed, &kept, [cx, cy, cz]);
        }
        chunk
    }

    /// Each column's own rock where Biomes disagree: the one case that must ask all 1024 columns
    /// for their Biome.
    /// The top of the lava version 3's deepest caverns hold, with a World lava.
    fn lava_m(&self) -> Option<f32> {
        let caverns = self.caverns.as_ref()?;
        self.params.lava.map(|_| caverns.floor_m + LAVA_DEPTH_M)
    }

    /// The water table under `(x_m, z_m)`: the World's a little under Sea level, raised across
    /// a perched aquifer's whole region so its lakes stand level.
    pub fn water_table_m(&self, x_m: f32, z_m: f32) -> f32 {
        let table_m = self.params.sea_level_m - WATER_TABLE_BELOW_SEA_M;
        if self.caverns.is_none() || self.water.is_none() {
            return table_m;
        }
        table_m + aquifer_rise_m(self.seed, aquifer_cell([x_m, z_m]))
    }

    /// Whether a cavern is open at a point: carved, and not in the rock kept over the lava or
    /// along an aquifer's border.
    fn cavern_open(&self, caverns: &Caverns, [x_m, y_m, z_m]: [f32; 3]) -> bool {
        if !caverns.carves(x_m, y_m, z_m) {
            return false;
        }
        if let Some(lava_m) = self.lava_m()
            && (lava_m..lava_m + LAVA_SEAL_M).contains(&y_m)
        {
            return false;
        }
        !self.aquifer_walled([x_m, y_m, z_m])
    }

    /// Whether a point lies in the wall kept along an aquifer region's border, below the higher
    /// of the two tables either side where one is raised.
    fn aquifer_walled(&self, [x_m, y_m, z_m]: [f32; 3]) -> bool {
        if self.water.is_none() {
            return false;
        }
        let cell = aquifer_cell([x_m, z_m]);
        let at = [x_m, z_m];
        let mut highest_m = 0.0f32;
        let mut near = false;
        for axis in 0..2 {
            let within = at[axis] - cell[axis] as f32 * crate::underground::AQUIFER_CELL_M;
            let step = if within < AQUIFER_WALL_M {
                -1
            } else if within > crate::underground::AQUIFER_CELL_M - AQUIFER_WALL_M {
                1
            } else {
                continue;
            };
            let mut other = cell;
            other[axis] += step;
            near = true;
            highest_m = highest_m.max(aquifer_rise_m(self.seed, other));
        }
        if !near {
            return false;
        }
        highest_m = highest_m.max(aquifer_rise_m(self.seed, cell));
        highest_m > 0.0 && y_m < self.params.sea_level_m - WATER_TABLE_BELOW_SEA_M + highest_m
    }

    /// What fills an open cavern Voxel: lava near the band's floor, water under the table, a cave
    /// Biome's feature on a dry floor, or `None`, air.
    fn cavern_fill(
        &self,
        caverns: &Caverns,
        [x_m, y_m, z_m]: [f32; 3],
        table_m: f32,
        cave: impl FnOnce() -> Option<u8>,
    ) -> Option<Voxel> {
        if let (Some(lava), Some(lava_m)) = (self.params.lava, self.lava_m())
            && y_m < lava_m
        {
            return Some(Voxel::cube(lava));
        }
        let water = self.water?;
        if y_m < table_m {
            return Some(Voxel::cube(water.material));
        }
        // Whole metres alone, so a coarser Voxel size stands the same few.
        let whole = |m: f32| m == m.floor();
        if !(whole(x_m) && whole(z_m) && whole(y_m)) {
            return None;
        }
        let draw = |salt: u32, spacing_m: f32| {
            let seed = self.seed ^ salt ^ (y_m as i32 as u32).wrapping_mul(0x9e37_79b1);
            crate::noise::cell_hash(x_m as i32, z_m as i32, seed)[0] < 1.0 / (spacing_m * spacing_m)
        };
        let cave = &self.params.cave_biomes.as_slice()[usize::from(cave()?)];
        if self.hollow_at(caverns, [x_m, y_m - self.voxel_size, z_m]) {
            return None;
        }
        cave.features()
            .iter()
            .enumerate()
            .find(|&(i, f)| draw(SALT_CAVE_FEATURE + i as u32, f.spacing_m))
            .map(|(_, f)| Voxel::new(f.material, f.shape, 0))
    }

    /// Whether a floor Voxel of a perched aquifer's dry cavern wells up as a spring: a pool one
    /// Voxel deep, rock below and on every side, so the water stands rather than pours.
    fn spring_at(&self, [x_m, y_m, z_m]: [f32; 3], base_m: f32, table_m: f32) -> bool {
        let Some(caverns) = &self.caverns else {
            return false;
        };
        let whole = |m: f32| m == m.floor();
        if self.water.is_none()
            || table_m <= self.params.sea_level_m - WATER_TABLE_BELOW_SEA_M
            || y_m < table_m
            || base_m - self.voxel_size < CAVERN_MIN_DEPTH_M
            || !(whole(x_m) && whole(z_m))
        {
            return false;
        }
        let seed = self.seed ^ SALT_SPRING ^ (y_m as i32 as u32).wrapping_mul(0x9e37_79b1);
        let chance = 1.0 / (SPRING_SPACING_M * SPRING_SPACING_M);
        if crate::noise::cell_hash(x_m as i32, z_m as i32, seed)[0] >= chance {
            return false;
        }
        let v = self.voxel_size;
        let open = |[dx, dy, dz]: [f32; 3]| self.hollow_at(caverns, [x_m + dx, y_m + dy, z_m + dz]);
        open([0.0, v, 0.0])
            && ![
                [0.0, -v, 0.0],
                [v, 0.0, 0.0],
                [-v, 0.0, 0.0],
                [0.0, 0.0, v],
                [0.0, 0.0, -v],
            ]
            .into_iter()
            .any(open)
    }

    /// Whether a cavern or a tunnel opens a point, as far as a spring or a cave feature beside it
    /// needs to know.
    fn hollow_at(&self, caverns: &Caverns, [x_m, y_m, z_m]: [f32; 3]) -> bool {
        self.cavern_open(caverns, [x_m, y_m, z_m])
            || self
                .caves
                .as_ref()
                .is_some_and(|c| c.carves(x_m, y_m, z_m, self.cave_scale_at(x_m, z_m)))
    }

    /// The cave Biome of a Voxel `base_m` under the surface, cached by lattice cell for the Chunk.
    fn cave_biome_cached(
        &self,
        cache: &mut HashMap<[i32; 3], Option<u8>>,
        at: [f32; 3],
        base_m: f32,
    ) -> Option<u8> {
        if !self.underground_biomes || base_m < CAVE_BIOME_MIN_DEPTH_M {
            return None;
        }
        let cell = at.map(|m| (m / crate::underground::CAVE_CELL_M).floor() as i32);
        *cache.entry(cell).or_insert_with(|| {
            cave_biome_at(
                &self.params.cave_biomes,
                self.seed,
                self.params.sea_level_m,
                at,
            )
        })
    }

    /// The fog of the cave Biome round a point at least `CAVE_BIOME_MIN_DEPTH_M` under the
    /// surface, where one has fog; `None` leaves the surface's air.
    pub fn cave_fog_at(&self, x_m: f32, y_m: f32, z_m: f32) -> Option<SkyFogBlend> {
        if !self.underground_biomes
            || self.surface_height_m(x_m, z_m) - y_m < CAVE_BIOME_MIN_DEPTH_M
        {
            return None;
        }
        let cave = cave_biome_at(
            &self.params.cave_biomes,
            self.seed,
            self.params.sea_level_m,
            [x_m, y_m, z_m],
        )?;
        self.params.cave_biomes.as_slice()[usize::from(cave)]
            .fog
            .map(|f| f.blend())
    }

    /// The cave Biome round a point, by its place in the World's cave Biome table, as
    /// [`Self::cave_fog_at`] finds it.
    pub fn cave_biome_index_at(&self, x_m: f32, y_m: f32, z_m: f32) -> Option<usize> {
        if !self.underground_biomes
            || self.surface_height_m(x_m, z_m) - y_m < CAVE_BIOME_MIN_DEPTH_M
        {
            return None;
        }
        cave_biome_at(
            &self.params.cave_biomes,
            self.seed,
            self.params.sea_level_m,
            [x_m, y_m, z_m],
        )
        .map(usize::from)
    }

    /// The name of the cave Biome round a point, as [`Self::cave_fog_at`] finds it.
    pub fn cave_biome_name_at(&self, x_m: f32, y_m: f32, z_m: f32) -> Option<&str> {
        let at = self.cave_biome_index_at(x_m, y_m, z_m)?;
        Some(self.params.cave_biomes.as_slice()[at].name.as_str())
    }

    /// How far under the lowest surface every Voxel is the ground's rock: past the soil and the
    /// deepest stratum's undulating bottom.
    fn solid_below_m(&self) -> f32 {
        let strata_m = if self.strata {
            self.params
                .biomes
                .strata()
                .iter()
                .map(Strata::deepest_m)
                .fold(0.0, f32::max)
                + STRATA_WAVE_M
        } else {
            0.0
        };
        self.params.soil_depth_m.max(strata_m)
    }

    fn deep_rock(&self, cx: i32, cz: i32, ground: GroundParams) -> Chunk {
        let columns = self
            .biome_grounds
            .filter(|g| !g.same_rock)
            .and_then(|_| self.column_grounds(cx, cz));
        let Some(columns) = columns else {
            return Chunk::filled(Voxel::cube(ground.rock));
        };
        if columns.iter().all(|g| g.rock == columns[0].rock) {
            return Chunk::filled(Voxel::cube(columns[0].rock));
        }
        let mut chunk = ChunkBuilder::new();
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let rock = Voxel::cube(columns[z * CHUNK_SIZE + x].rock);
                for y in 0..CHUNK_SIZE {
                    chunk.set(x, y, z, rock);
                }
            }
        }
        chunk.build()
    }

    /// Depth is measured from a surface somewhere in the surface band, so this only errs towards
    /// reaching.
    fn ores_reaching(&self, floor_m: f32, ceiling_m: f32) -> Option<(&OreField, OreMask)> {
        let field = self.ores.as_ref()?;
        let (lowest, highest) = self.terrain_band_m();
        let mask = field.reaching((lowest - ceiling_m).max(0.0), highest - floor_m);
        (!mask.is_empty()).then_some((field, mask))
    }

    /// Looked up on first ask, so not at all in a column no ore vein touches.
    fn column_biome(
        &self,
        [cx, cz]: [i32; 2],
        [x, z]: [usize; 2],
    ) -> impl Fn() -> Option<BiomeId> + '_ {
        let cell = std::cell::OnceCell::new();
        move || {
            *cell
                .get_or_init(|| self.biome_at(self.axis_m(cx, x as i32), self.axis_m(cz, z as i32)))
        }
    }

    /// Depth needs each column's own surface.
    fn sow_deep_rock(
        &self,
        chunk: &mut Chunk,
        coord: [i32; 2],
        origin_m: [f32; 3],
        field: &OreField,
        ores: OreMask,
    ) {
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let x_m = origin_m[0] + x as f32 * self.voxel_size;
                let z_m = origin_m[2] + z as f32 * self.voxel_size;
                let surface_m = self.surface_height_m(x_m, z_m);
                let biome = self.column_biome(coord, [x, z]);
                for y in 0..CHUNK_SIZE {
                    let y_m = origin_m[1] + y as f32 * self.voxel_size;
                    if let Some(material) = field.at(ores, [x_m, y_m, z_m], surface_m - y_m, &biome)
                    {
                        chunk.set(x, y, z, Voxel::cube(material));
                    }
                }
            }
        }
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
        columns: Option<&ColumnGrounds>,
    ) {
        let Some(ground) = self.params.ground else {
            return;
        };
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
                    // The ground as it stands, so a cell the overhang density pushed solid or a
                    // cave opened counts as it looks.
                    if self.still_solid(chunk, coord, [nx, y, nz])
                        && !self.still_solid(chunk, coord, [nx, y + 1, nz])
                    {
                        rising += 1;
                        turn = quarter;
                    }
                }
                let cap = columns
                    .map_or(ground, |c| c[z as usize * CHUNK_SIZE + x as usize])
                    .surface;
                let voxel = match rising {
                    0 => continue,
                    1 => Voxel::shaped(cap, SHAPE_STAIR, Orientation::new(POS_Y, turn)),
                    _ => Voxel::shaped(cap, SHAPE_SLAB, Orientation::IDENTITY),
                };
                chunk.set(x as usize, y as usize, z as usize, voxel);
                placed += 1;
                if placed == MAX_SMOOTHED_PER_CHUNK {
                    return;
                }
            }
        }
    }

    /// Stands a Biome's feature in the cell over a column's top Voxel, after smoothing so a stair
    /// keeps its cell. Like a stair, a cell over the last Voxel of the Chunk below is made here.
    fn scatter(
        &self,
        chunk: &mut Chunk,
        scatter: &SurfaceScatter,
        placed: &[fields::Placed],
        kept: &[Footprint],
        surface_m: &[f32; SURFACE_W * SURFACE_W],
        coord: [i32; 3],
    ) {
        let Some(world) = self.params.ground else {
            return;
        };
        let (_, highest) = self.terrain_band_m();
        let size = CHUNK_SIZE as i32;
        let floor_m = (coord[1] * size) as f32 * self.voxel_size;
        for z in 0..size {
            for x in 0..size {
                let top = top_index(surface_m[surface_index(x, z)], floor_m, self.voxel_size);
                let y = top + 1;
                if !(0..size).contains(&y)
                    || floor_m + y as f32 * self.voxel_size > highest
                    || !chunk.get(x as usize, y as usize, z as usize).is_air()
                {
                    continue;
                }
                let (x_m, z_m) = (self.axis_m(coord[0], x), self.axis_m(coord[2], z));
                let draw = SurfaceScatter::draw(self.seed, x_m, z_m);
                let (gx, gz) = (coord[0] * size + x, coord[2] * size + z);
                if draw >= scatter.most
                    || fields::cell_at(placed, gx, gz).is_some()
                    || covers(kept, gx, gz)
                {
                    continue;
                }
                let Some(biome) = self
                    .biome_at(x_m, z_m)
                    .and_then(|id| self.params.biomes.get(id))
                else {
                    continue;
                };
                let Some(feature) =
                    scatter.pick(&biome.surface_features, draw, self.seed, x_m, z_m)
                else {
                    continue;
                };
                // A Plant is stood by `plants`, which reads no neighbour Chunk's cells.
                if feature.plant.is_some() {
                    continue;
                }
                let material = feature.material;
                let surface = biome.ground.unwrap_or(world).surface;
                let on_surface = if top >= 0 {
                    chunk.get(x as usize, top as usize, z as usize) == Voxel::cube(surface)
                } else {
                    self.still_solid(chunk, coord, [x, top, z])
                };
                if on_surface {
                    chunk.set(x as usize, y as usize, z as usize, Voxel::cube(material));
                }
            }
        }
    }

    /// The metre position of the column or layer at global Voxel index `g`, as its owning Chunk
    /// computes it.
    fn global_m(&self, g: i32) -> f32 {
        let size = CHUNK_SIZE as i32;
        self.axis_m(g.div_euclid(size), g.rem_euclid(size))
    }

    /// The footprint of every Structure and Settlement covering or bordering a column of `region`.
    fn footprints_near(&self, region: [[i32; 2]; 2]) -> Vec<Footprint> {
        self.footprints(&self.claims_near(region))
    }

    /// The columns of the Chunk at `[cx, cz]`, widened by `margin` on every side.
    fn chunk_columns(&self, [cx, cz]: [i32; 2], margin: i32) -> [[i32; 2]; 2] {
        let size = CHUNK_SIZE as i32;
        [
            [cx, cz].map(|c| c * size - margin),
            [cx, cz].map(|c| (c + 1) * size + margin),
        ]
    }

    /// Rock below this height stays solid: a cave opening onto water would let
    /// the fluid simulation pour it in, a hole that keeps changing and being saved.
    fn sealed_below_m(&self, x_m: f32, z_m: f32, ground_m: f32) -> f32 {
        /// Higher than any bank this steep is too far from the water to open onto it.
        const CLEAR_M: f32 = 64.0;
        const REACH_M: f32 = 4.0;
        if self.water.is_none() {
            return f32::NEG_INFINITY;
        }
        let wet = |x, z| {
            self.water_level_m(x, z).filter(|_| {
                self.surface_height_m(x, z) < self.water_level_m(x, z).unwrap_or(f32::MIN)
            })
        };
        if self
            .water_level_m(x_m, z_m)
            .is_some_and(|level| ground_m > level + CLEAR_M)
        {
            return f32::NEG_INFINITY;
        }
        // The Voxels beside the column, which a carve here would open onto, and a few metres out,
        // which a tunnel's width reaches.
        let s = self.voxel_size;
        let level_m = [
            (0.0, 0.0),
            (s, 0.0),
            (-s, 0.0),
            (0.0, s),
            (0.0, -s),
            (REACH_M, 0.0),
            (-REACH_M, 0.0),
            (0.0, REACH_M),
            (0.0, -REACH_M),
        ]
        .into_iter()
        .filter_map(|(dx, dz)| wet(x_m + dx, z_m + dz))
        .fold(f32::NEG_INFINITY, f32::max);
        // Water fills the whole Voxel its level reaches into, so the rock is sealed to that
        // Voxel's top and not only to the level.
        if level_m == f32::NEG_INFINITY {
            return level_m;
        }
        (self.level_top(level_m) + 1) as f32 * s
    }

    /// Whether a tunnel or a shaft opens the ground cell at global Voxel `at`, the column's top or
    /// above: what the Chunk holding it decides, so any Chunk can answer for it. A cavern stays
    /// [`CAVERN_MIN_DEPTH_M`] under the ground, so it never reaches the cells this is asked about.
    fn carved(&self, at: [i32; 3]) -> bool {
        let [x_m, y_m, z_m] = at.map(|g| self.global_m(g));
        let tunnel = || {
            self.caves
                .as_ref()
                .is_some_and(|caves| caves.carves(x_m, y_m, z_m, self.cave_scale_at(x_m, z_m)))
        };
        let shaft = || {
            self.caverns.as_ref().is_some_and(|c| {
                c.shafts(
                    [x_m, z_m],
                    [x_m, z_m],
                    |x, z| self.surface_height_m(x, z),
                    |x, z| self.water_surface_m(x, z).is_none(),
                )
                .iter()
                .any(|s| s.cuts(x_m, y_m, z_m))
            })
        };
        if !(tunnel() || shaft()) {
            return false;
        }
        let clear = (CAVE_CLEAR_M / self.voxel_size).ceil() as i32;
        let near = self.footprints_near([
            [at[0] - clear, at[2] - clear],
            [at[0] + 1 + clear, at[2] + 1 + clear],
        ]);
        !near.iter().any(|f| f.widened(clear).covers(at[0], at[2]))
            && y_m >= self.sealed_below_m(x_m, z_m, self.surface_height_m(x_m, z_m))
            && !self.walled_in([x_m, y_m, z_m], self.water_table_m(x_m, z_m))
    }

    /// Whether a tunnel or shaft must leave the rock at a point, which is a flooded cavern's or the
    /// shell round it below the water table.
    fn walled_in(&self, [x_m, y_m, z_m]: [f32; 3], table_m: f32) -> bool {
        let flooded = (self.water.is_some() && y_m < table_m)
            || self
                .lava_m()
                .is_some_and(|lava_m| y_m < lava_m + LAVA_SEAL_M);
        flooded && self.caverns.is_some_and(|c| c.walls_in(x_m, y_m, z_m))
    }

    /// The Plant a Biome stands on the column at global Voxel `(gx, gz)`, if any: a pure function
    /// of the seed and the position, so every Chunk that reaches the column decides alike. It
    /// reads no Chunk, so a tree is the same from any Chunk's viewpoint. Nothing stands on a
    /// planted field's columns, and nothing with a Voxel in a column `kept` covers.
    fn plant_at(
        &self,
        scatter: &SurfaceScatter,
        placed: &[fields::Placed],
        kept: &[Footprint],
        gx: i32,
        gz: i32,
    ) -> Option<Standing> {
        if fields::cell_at(placed, gx, gz).is_some() || covers(kept, gx, gz) {
            return None;
        }
        let standing = self.plant_raw(scatter, gx, gz)?;
        (kept.is_empty() || !self.reaches_into(scatter, &standing, kept)).then_some(standing)
    }

    /// Whether a tree or vine standing beside a Structure reaches a column it covers: it is left
    /// out whole rather than cut, so it is the same from every Chunk's viewpoint.
    fn reaches_into(
        &self,
        scatter: &SurfaceScatter,
        standing: &Standing,
        kept: &[Footprint],
    ) -> bool {
        let [gx, _, gz] = standing.base;
        let margin = (scatter.reach_m / self.voxel_size).ceil() as i32 + 1;
        let near = kept.iter().any(|f| {
            (f.min[0] - margin..f.max[0] + margin).contains(&gx)
                && (f.min[1] - margin..f.max[1] + margin).contains(&gz)
        });
        if !near {
            return false;
        }
        let cells = match standing.plant.kind {
            SurfacePlantKind::Tree(tree) => {
                let mut cells = self.tree_cells(&tree, standing.base);
                cells.extend(self.vine_cells(&tree, standing.base).into_iter().flatten());
                cells
            }
            SurfacePlantKind::Vine(vine) => self.crop_cells(&standing.plant, &vine, standing.base),
            _ => return false,
        };
        cells.iter().any(|(at, _)| covers(kept, at[0], at[2]))
    }

    /// [`Self::plant_at`] before a planted field clears the column: what the column draws, unless
    /// it is a tree with a stronger tree's trunk within [`CROWN_SPACING`] crowns.
    fn plant_raw(&self, scatter: &SurfaceScatter, gx: i32, gz: i32) -> Option<Standing> {
        let standing = self.plant_drawn(scatter, gx, gz)?;
        if let SurfacePlantKind::Tree(tree) = standing.plant.kind
            && self.crowded(scatter, [gx, gz], CROWN_SPACING * tree.canopy_radius_m)
        {
            return None;
        }
        Some(standing)
    }

    /// Whether a tree whose draw beats this column's stands within `radius_m` of it. Each column
    /// asks only draws and the trees they would stand, never whether those stand themselves, so
    /// every Chunk decides alike; a lower draw wins, so of two trees too close the same one stays.
    #[expect(
        clippy::disallowed_methods,
        reason = "generation: generator-determinism ticket 12 makes this a multiplication"
    )]
    fn crowded(&self, scatter: &SurfaceScatter, [gx, gz]: [i32; 2], radius_m: f32) -> bool {
        let own = SurfaceScatter::draw(self.seed, self.global_m(gx), self.global_m(gz));
        let reach = (radius_m / self.voxel_size).ceil() as i32;
        let r2 = (radius_m / self.voxel_size).powi(2);
        (-reach..=reach)
            .flat_map(|dz| (-reach..=reach).map(move |dx| (dx, dz)))
            .filter(|&(dx, dz)| (dx, dz) != (0, 0) && ((dx * dx + dz * dz) as f32) < r2)
            .any(|(dx, dz)| {
                let (nx, nz) = (gx + dx, gz + dz);
                let draw = SurfaceScatter::draw(self.seed, self.global_m(nx), self.global_m(nz));
                draw < own
                    && draw < scatter.most
                    && self
                        .plant_drawn(scatter, nx, nz)
                        .is_some_and(|s| matches!(s.plant.kind, SurfacePlantKind::Tree(_)))
            })
    }

    /// What a column's own draw stands on it, before any neighbour is asked.
    fn plant_drawn(&self, scatter: &SurfaceScatter, gx: i32, gz: i32) -> Option<Standing> {
        self.params.ground?;
        let (x_m, z_m) = (self.global_m(gx), self.global_m(gz));
        let draw = SurfaceScatter::draw(self.seed, x_m, z_m);
        if draw >= scatter.most {
            return None;
        }
        let biome = self
            .biome_at(x_m, z_m)
            .and_then(|id| self.params.biomes.get(id))?;
        let plant = scatter
            .pick(&biome.surface_features, draw, self.seed, x_m, z_m)?
            .plant?;
        let size = self.voxel_size;
        let top = top_index(self.surface_height_m(x_m, z_m), 0.0, size);
        let base = [gx, top + 1, gz];
        let (_, highest) = self.terrain_band_m();
        let y_m = self.global_m(base[1]);
        if y_m > highest || self.carved([gx, top, gz]) {
            return None;
        }
        let under_lake = self.water.filter(|_| {
            self.water_level_m(x_m, z_m)
                .is_some_and(|level_m| y_m < level_m)
        });
        match (plant.kind, under_lake) {
            (
                SurfacePlantKind::Column {
                    stands_in: Some(fluid),
                    ..
                },
                Some(w),
            ) if w.material == fluid => {
                // A Voxel of water is left over the Plant's top.
                let depth_m = self
                    .water_level_m(x_m, z_m)
                    .map_or(0.0, |level_m| level_m - y_m);
                if depth_m <= size {
                    return None;
                }
            }
            (
                SurfacePlantKind::Column {
                    stands_in: Some(_), ..
                },
                _,
            )
            | (_, Some(_)) => return None,
            _ => {}
        }
        if self.smoothing && self.eased([gx, gz], base[1]) {
            return None;
        }
        Some(Standing {
            plant,
            base,
            vary: SurfaceScatter::draw_salted(self.seed, x_m, z_m, PLANT_VARY_SALT),
        })
    }

    /// Whether smoothing would put a stair or slab in the cell over the column's top, as `smooth`
    /// decides it for the Chunk that holds the cell.
    fn eased(&self, [gx, gz]: [i32; 2], y: i32) -> bool {
        CARDINALS.iter().any(|&(dx, dz, _)| {
            let (nx, nz) = (gx + dx, gz + dz);
            self.ground_at([nx, y, nz]) && !self.ground_at([nx, y + 1, nz])
        })
    }

    /// Voxels in a column Plant's run at this column, from its base up.
    fn run_voxels(&self, standing: &Standing) -> i32 {
        let SurfacePlantKind::Column {
            max_height_m,
            stands_in,
            ..
        } = standing.plant.kind
        else {
            return 1;
        };
        let most = ((max_height_m / self.voxel_size + 1e-3).floor() as i32).max(1);
        let mut run = 1 + (standing.vary * f64::from(most)) as i32;
        if stands_in.is_some()
            && let Some(level_m) = self.water_level_m(
                self.global_m(standing.base[0]),
                self.global_m(standing.base[2]),
            )
        {
            let room = ((level_m - self.global_m(standing.base[1])) / self.voxel_size).ceil();
            // The surface Voxel stays water, so a Plant never punches a hole in the top.
            run = run.min((room as i32 - 1).max(1));
        }
        run
    }

    /// The cell over the ground of column `(gx, gz)` where a Plant's Voxel stands: `plant_at`'s
    /// checks without a Biome, a feature or a column's lake, for fruit that stands beside its vine.
    fn ground_base(&self, gx: i32, gz: i32) -> Option<[i32; 3]> {
        self.params.ground?;
        let (x_m, z_m) = (self.global_m(gx), self.global_m(gz));
        let top = top_index(self.surface_height_m(x_m, z_m), 0.0, self.voxel_size);
        let base = [gx, top + 1, gz];
        let (_, highest) = self.terrain_band_m();
        let y_m = self.global_m(base[1]);
        if y_m > highest
            || self.carved([gx, top, gz])
            || self
                .water_level_m(x_m, z_m)
                .is_some_and(|level_m| y_m < level_m)
            || (self.smoothing && self.eased([gx, gz], base[1]))
        {
            return None;
        }
        Some(base)
    }

    /// A vine and its first fruit: the ripe vine at `base`, then fruit on the ground within reach,
    /// never touching. A function of the vine and its column in metres alone, like
    /// [`Self::tree_cells`], so every Chunk that reaches it agrees.
    fn crop_cells(
        &self,
        plant: &SurfacePlant,
        vine: &SurfaceVine,
        base: [i32; 3],
    ) -> Vec<([i32; 3], Voxel)> {
        let seed = u64::from(self.seed);
        let column_m = [self.global_m(base[0]), self.global_m(base[2])];
        let at = |[dx, dz]: [i32; 2]| self.ground_base(base[0] + dx, base[2] + dz);
        let offsets = first_fruit_offsets(
            seed,
            column_m,
            (vine.reach_m, vine.gap_m, vine.max_fruit),
            self.voxel_size,
            |c| at(c).is_some(),
        );
        let mut cells = vec![(base, Voxel::new(vine.material, plant.shape, 0))];
        for fruit in offsets.into_iter().filter_map(at) {
            let stage = fruit_stage(seed, fruit, self.voxel_size, vine.len as usize);
            let shape = vine.stage_shapes[stage].unwrap_or(vine.fruit_shape);
            cells.push((fruit, Voxel::new(vine.fruit[stage], shape, 0)));
        }
        cells
    }

    /// The Plant's tree as the generator grows it: a Tree row holding only what shape needs.
    fn tree_of(tree: &SurfaceTree) -> Tree {
        Tree {
            trunk: tree.trunk,
            canopy: tree.canopy,
            trunk_height_m: tree.trunk_height_m,
            canopy_radius_m: tree.canopy_radius_m,
            branching: tree.branching,
            mean_seconds: 0.0,
            trunk_yield: Vec::new(),
            canopy_yield: Vec::new(),
            sapling_yield: CountRange { min: 1, max: 1 },
            vines: None,
            fruit: tree.fruit.map(|f| Fruit {
                stages: f.stages[..f.len as usize]
                    .iter()
                    .zip(f.stage_shapes)
                    .map(|(&material, shape)| Stage {
                        material,
                        mean_seconds: None,
                        yields: Vec::new(),
                        shape,
                    })
                    .collect(),
                density: f.density,
                regrow_seconds: crate::plant::SAMPLE_INTERVAL_S,
                shape: (f.shape != SHAPE_FULL_CUBE).then_some(f.shape),
            }),
        }
    }

    /// Every Voxel of the tree standing at `base`, fully grown, with the fruit at hash-drawn
    /// Stages. Grown from the ground's height 0 and shifted up, so the tree is a function of its
    /// column in metres and not of which Voxel the ground rounds to at this Voxel size.
    fn tree_cells(&self, tree: &SurfaceTree, base: [i32; 3]) -> Vec<([i32; 3], Voxel)> {
        let row = Self::tree_of(tree);
        let seed = u64::from(self.seed);
        tree_voxels(&row, seed, [base[0], 0, base[2]], self.voxel_size)
            .into_iter()
            .map(|v| {
                let (material, shape) = match (v.part, tree.fruit) {
                    (Part::Fruit, Some(f)) => {
                        let stage = fruit_stage(seed, v.at, self.voxel_size, f.len as usize);
                        (f.stages[stage], f.stage_shapes[stage].unwrap_or(f.shape))
                    }
                    _ => (v.material, v.shape),
                };
                (
                    [v.at[0], v.at[1] + base[1], v.at[2]],
                    Voxel::new(material, shape, v.orientation),
                )
            })
            .collect()
    }

    /// The vines hanging from the tree standing at `base`, one list of cells per vine, each ending
    /// above the ground of its own column. A function of the tree and its column in metres alone,
    /// like [`Self::tree_cells`].
    fn vine_cells(&self, tree: &SurfaceTree, base: [i32; 3]) -> Vec<Vec<([i32; 3], Voxel)>> {
        let Some(vines) = tree.vines else {
            return Vec::new();
        };
        let seed = u64::from(self.seed);
        let grown = tree_voxels(
            &Self::tree_of(tree),
            seed,
            [base[0], 0, base[2]],
            self.voxel_size,
        );
        vine_voxels(&grown, &vines, seed, self.voxel_size)
            .into_iter()
            .map(|vine| {
                let voxel = Voxel::new(vines.material, vines.shape, vine.orientation);
                let ground = top_index(
                    self.surface_height_m(self.global_m(vine.top[0]), self.global_m(vine.top[2])),
                    0.0,
                    self.voxel_size,
                );
                vine.cells()
                    .map(|at| [at[0], at[1] + base[1], at[2]])
                    .take_while(|at| at[1] > ground)
                    .map(|at| (at, voxel))
                    .collect()
            })
            .collect()
    }

    /// Stands the Plants whose Voxels fall in this Chunk. Trees reach over a Chunk's border, so
    /// every column within the widest canopy is asked, in one fixed order, and a Voxel goes only
    /// into an empty cell: the first tree to claim it wins from every Chunk's viewpoint.
    fn plants(
        &self,
        chunk: &mut Chunk,
        scatter: &SurfaceScatter,
        placed: &[fields::Placed],
        kept: &[Footprint],
        coord: [i32; 3],
    ) {
        let size = CHUNK_SIZE as i32;
        let origin = [coord[0] * size, coord[1] * size, coord[2] * size];
        let (lowest, _) = self.terrain_band_m();
        let (_, highest) = self.ground_bounds_m();
        let floor_m = self.global_m(origin[1]);
        let ceiling_m = self.global_m(origin[1] + size - 1);
        if floor_m > highest || ceiling_m < lowest - self.voxel_size {
            return;
        }
        let margin = if scatter.reach_m > 0.0 {
            (scatter.reach_m / self.voxel_size).ceil() as i32 + 1
        } else {
            0
        };
        let inside = |c: [i32; 3]| -> Option<[usize; 3]> {
            let local = [c[0] - origin[0], c[1] - origin[1], c[2] - origin[2]];
            local
                .iter()
                .all(|v| (0..size).contains(v))
                .then(|| local.map(|v| v as usize))
        };
        let put = |chunk: &mut Chunk, at: [i32; 3], voxel: Voxel, into: Option<MaterialId>| {
            if let Some([x, y, z]) = inside(at) {
                let here = chunk.get(x, y, z);
                if here.is_air() || into.is_some_and(|fluid| here == Voxel::cube(fluid)) {
                    chunk.set(x, y, z, voxel);
                }
            }
        };
        let outside = |g: i32, lo: i32| (lo - g).max(g - (lo + size - 1)).max(0);
        let mut trees = Vec::new();
        let mut hung: Vec<Vec<([i32; 3], Voxel)>> = Vec::new();
        let mut crops: Vec<([i32; 3], Voxel)> = Vec::new();
        for gz in origin[2] - margin..origin[2] + size + margin {
            for gx in origin[0] - margin..origin[0] + size + margin {
                let own = (origin[0]..origin[0] + size).contains(&gx)
                    && (origin[2]..origin[2] + size).contains(&gz);
                let Some(standing) = self.plant_at(scatter, placed, kept, gx, gz) else {
                    continue;
                };
                let base = standing.base;
                match standing.plant.kind {
                    SurfacePlantKind::Stages {
                        materials,
                        len,
                        stage_shapes,
                    } if own => {
                        let stage = (standing.vary * f64::from(len)) as usize;
                        let shape = stage_shapes[stage].unwrap_or(standing.plant.shape);
                        let voxel = Voxel::new(materials[stage], shape, 0);
                        put(chunk, base, voxel, None);
                    }
                    SurfacePlantKind::Column {
                        material,
                        stands_in,
                        ..
                    } if own => {
                        let voxel = Voxel::new(material, standing.plant.shape, 0);
                        for dy in 0..self.run_voxels(&standing) {
                            put(chunk, [base[0], base[1] + dy, base[2]], voxel, stands_in);
                        }
                    }
                    SurfacePlantKind::Tree(tree) => {
                        let reach = (tree.reach_m() / self.voxel_size).ceil() as i32
                            + 1
                            + tree.rounding_slack_voxels();
                        // A vine hangs from the canopy, so it can reach below the base on a slope.
                        let hang = tree
                            .vines
                            .map_or(0, |v| (v.length_m / self.voxel_size).ceil() as i32 + 1);
                        if outside(gx, origin[0]) > reach
                            || outside(gz, origin[2]) > reach
                            || base[1] - hang > origin[1] + size - 1
                        {
                            continue;
                        }
                        let rise = (scatter.rise_m / self.voxel_size).ceil() as i32 + 2;
                        if base[1] + rise < origin[1] {
                            continue;
                        }
                        trees.push((tree.trunk, self.tree_cells(&tree, base)));
                        hung.extend(self.vine_cells(&tree, base));
                    }
                    SurfacePlantKind::Vine(vine) => {
                        let reach = (vine.reach_m / self.voxel_size).ceil() as i32 + 1;
                        let slope = reach + 2;
                        if outside(gx, origin[0]) > reach
                            || outside(gz, origin[2]) > reach
                            || base[1] - slope > origin[1] + size - 1
                            || base[1] + slope < origin[1]
                        {
                            continue;
                        }
                        crops.extend(self.crop_cells(&standing.plant, &vine, base));
                    }
                    _ => {}
                }
            }
        }
        // A trunk is never left with a hole where a neighbour's canopy got there first.
        for trunks in [true, false] {
            for (trunk, cells) in &trees {
                for &(at, voxel) in cells {
                    if (voxel.material() == *trunk) == trunks {
                        put(chunk, at, voxel, None);
                    }
                }
            }
        }
        // Vines hang last, so they never take a cell any tree's own Voxel needs.
        for &(at, voxel) in hung.iter().flatten() {
            put(chunk, at, voxel, None);
        }
        // Crops with fruit stand after every tree, so a trunk or crown is never cut by one.
        for &(at, voxel) in &crops {
            put(chunk, at, voxel, None);
        }
    }

    /// The height, in metres, of the tallest Plant Voxel in a footprint's columns, for spawning
    /// clear of them.
    fn plants_clear_m(&self, scatter: &SurfaceScatter, min_m: [f32; 2], max_m: [f32; 2]) -> f32 {
        let size = self.voxel_size;
        let columns =
            |a: usize| (min_m[a] / size).floor() as i32..=(max_m[a] / size).floor() as i32;
        let (xs, zs) = (columns(0), columns(1));
        let margin = (scatter.reach_m / size).ceil() as i32 + 1;
        let placed = self.fields_near(
            scatter,
            [
                [*xs.start() - margin, *zs.start() - margin],
                [*xs.end() + margin + 1, *zs.end() + margin + 1],
            ],
        );
        let kept = self.footprints_near([
            [*xs.start() - 2 * margin, *zs.start() - 2 * margin],
            [*xs.end() + 2 * margin + 1, *zs.end() + 2 * margin + 1],
        ]);
        let mut clear_m = f32::MIN;
        for gz in zs.clone() {
            for gx in xs.clone() {
                if matches!(
                    fields::cell_at(&placed, gx, gz),
                    Some((_, fields::Cell::Plot { planted: true, .. }))
                ) && !covers(&kept, gx, gz)
                {
                    let top = top_index(
                        self.surface_height_m(self.global_m(gx), self.global_m(gz)),
                        0.0,
                        size,
                    );
                    clear_m = clear_m.max(self.global_m(top + 2));
                }
            }
        }
        for gz in *zs.start() - margin..=*zs.end() + margin {
            for gx in *xs.start() - margin..=*xs.end() + margin {
                let Some(standing) = self.plant_at(scatter, &placed, &kept, gx, gz) else {
                    continue;
                };
                let base = standing.base;
                let own = xs.contains(&gx) && zs.contains(&gz);
                let top = match standing.plant.kind {
                    SurfacePlantKind::Tree(tree) => self
                        .tree_cells(&tree, base)
                        .into_iter()
                        .filter(|(at, _)| xs.contains(&at[0]) && zs.contains(&at[2]))
                        .map(|(at, _)| at[1] + 1)
                        .max(),
                    SurfacePlantKind::Vine(vine) => self
                        .crop_cells(&standing.plant, &vine, base)
                        .into_iter()
                        .filter(|(at, _)| xs.contains(&at[0]) && zs.contains(&at[2]))
                        .map(|(at, _)| at[1] + 1)
                        .max(),
                    _ if own => Some(base[1] + self.run_voxels(&standing)),
                    _ => None,
                };
                if let Some(top) = top {
                    clear_m = clear_m.max(self.global_m(top));
                }
            }
        }
        clear_m
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

    /// Whether the cell `at` (Voxel offsets from this Chunk's corner, possibly one outside it) is
    /// ground after the cube pass. Inside the Chunk that is a read, which smoothing's own stairs
    /// do not change; outside, it is asked as the Chunk holding it decides it.
    fn still_solid(&self, chunk: &Chunk, coord: [i32; 3], at: [i32; 3]) -> bool {
        if at.iter().all(|c| (0..CHUNK_SIZE as i32).contains(c)) {
            let v = chunk.get(at[0] as usize, at[1] as usize, at[2] as usize);
            return !v.is_air()
                && !v.is_shaped()
                && self.water.is_none_or(|w| v.material() != w.material);
        }
        let size = CHUNK_SIZE as i32;
        self.ground_at([0, 1, 2].map(|a| coord[a] * size + at[a]))
    }

    /// Whether the cell at global Voxel `at`, the column's top or near it, is ground once caves
    /// are carved, as the Chunk holding it decides.
    fn ground_at(&self, at: [i32; 3]) -> bool {
        self.filled(at) && !self.carved(at)
    }

    /// Whether the cube pass fills the cell at global Voxel `[gx, y, gz]` with ground before
    /// caves: under the column's surface, or past it where the overhang density pushes it. It
    /// asks the column and its neighbours rather than their Chunk's whole Stack, since a
    /// neighbour's Chunk may not be made yet.
    fn filled(&self, [gx, y, gz]: [i32; 3]) -> bool {
        let at_m = |gx, gz| [gx, gz].map(|g| self.global_m(g));
        let [x_m, z_m] = at_m(gx, gz);
        let y_m = self.global_m(y);
        let surface_m = self.surface_height_m(x_m, z_m);
        let base_m = surface_m - y_m;
        if base_m.abs() > self.terrain.overhang_max_m() {
            return base_m >= 0.0;
        }
        // `overhang_amp_at`, column by column.
        let wettest_m = CARDINALS
            .iter()
            .map(|&(dx, dz, _)| at_m(gx + dx, gz + dz))
            .filter(|&[x, z]| {
                self.water_level_m(x, z)
                    .is_some_and(|l| l > self.surface_height_m(x, z))
            })
            .chain([[x_m, z_m]])
            .filter_map(|[x, z]| self.water_level_m(x, z))
            .fold(f32::MIN, f32::max);
        let amp_m = self
            .terrain
            .overhang_amp_m(x_m, z_m)
            .min((surface_m - wettest_m).max(0.0));
        let size = CHUNK_SIZE as i32;
        // Only a banded cell reads the Structures the density keeps off, which cost the most.
        if amp_m <= 0.0
            || base_m.abs() > amp_m
            || covers(
                &self.overhang_kept_off([gx.div_euclid(size), gz.div_euclid(size)]),
                gx,
                gz,
            )
        {
            return base_m >= 0.0;
        }
        base_m + self.terrain.overhang_push_m(x_m, y_m, z_m, amp_m) >= 0.0
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
        scale: f32,
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
                        lo[1] <= hi[1] && caves.may_carve(lo, hi, scale);
                }
            }
        }
        reachable
    }

    /// Carves a Chunk that is solid rock throughout, visiting only the
    /// sub-blocks a tunnel can reach.
    fn carve(
        &self,
        chunk: &mut Chunk,
        caves: &CaveField,
        origin_m: [f32; 3],
        top_m: f32,
        columns: Option<&ColumnCaves>,
    ) {
        let reachable =
            self.reachable_blocks(caves, origin_m, top_m, columns.map_or(1.0, |c| c.max));
        let mut cache = Default::default();
        for z in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    if reachable[block_of(x, y, z)]
                        && caves.carves_near(
                            &mut cache,
                            origin_m[0] + x as f32 * self.voxel_size,
                            origin_m[1] + y as f32 * self.voxel_size,
                            origin_m[2] + z as f32 * self.voxel_size,
                            columns.map_or(1.0, |c| c.scales[z * CHUNK_SIZE + x]),
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
#[allow(
    clippy::disallowed_methods,
    reason = "a test's own maths is not generation"
)]
mod tests {
    use super::*;

    const SEED: u64 = 0x7e55_e2ae;
    /// Under version 3 [`SEED`]'s sweep is flat lowland; this one's holds sea, dry land and
    /// one-Voxel steps.
    const COAST: u64 = 3907;

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
                ..crate::testgen::world_params()
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
                ..crate::testgen::world_params()
            },
        )
    }

    /// Smoothing with no minimum rise, so it fires at any Voxel size: the tests
    /// about where it places things, rather than whether it runs.
    fn smoothed(voxel_size: f32, caves: bool) -> Worldgen {
        smoothed_at(SEED, voxel_size, caves)
    }

    fn smoothed_at(seed: u64, voxel_size: f32, caves: bool) -> Worldgen {
        let defaults = crate::testgen::world_params();
        Worldgen::new(
            seed,
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
    fn no_ground_is_air_without_moving_the_surface() {
        let params = crate::testgen::world_params();
        for size in [1.0, 0.5, 0.25] {
            let solid = Worldgen::new(7, size, params);
            let empty = Worldgen::new(
                7,
                size,
                WorldgenParams {
                    ground: None,
                    ..params
                },
            );
            assert_eq!(
                solid.surface_height_m(0.0, 0.0),
                empty.surface_height_m(0.0, 0.0)
            );
            for y in [-100, -1, 0, 1, 100] {
                assert_eq!(empty.chunk(0, y, 0).uniform(), Some(Voxel::AIR));
            }
        }
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
        let soil_m = crate::testgen::world_params().soil_depth_m;
        for voxel_size in [1.0, 0.5, 0.25] {
            let g = terrain(voxel_size);
            let dirt = column(&g, 0, 0)
                .iter()
                .filter(|v| v.material() == crate::testgen::fixtures().ground.soil)
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
            let grass: Vec<usize> = (0..c.len())
                .filter(|&y| c[y].material() == crate::testgen::fixtures().ground.surface)
                .collect();
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
        let a = Worldgen::new(1, 1.0, crate::testgen::world_params());
        let b = Worldgen::new(2, 1.0, crate::testgen::world_params());
        assert!((0..200).any(|i| {
            let m = i as f32 * 11.3;
            a.surface_height_m(m, -m) != b.surface_height_m(m, -m)
        }));
    }

    #[test]
    fn a_seed_differing_only_in_its_high_bits_is_a_different_world() {
        // Truncating the u64 instead of folding it would make these one world.
        let a = Worldgen::new(1, 1.0, crate::testgen::world_params());
        let b = Worldgen::new(1 | (1 << 40), 1.0, crate::testgen::world_params());
        assert!((0..200).any(|i| {
            let m = i as f32 * 11.3;
            a.surface_height_m(m, -m) != b.surface_height_m(m, -m)
        }));
    }

    /// FNV-1a over surface heights and one surface Chunk, so a pinned value is
    /// stable across toolchains, unlike `DefaultHasher`.
    fn terrain_digest(seed: u64, params: WorldgenParams) -> u64 {
        let g = Worldgen::new(seed, 1.0, params);
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        let mut eat = |word: u32| {
            for byte in word.to_le_bytes() {
                hash = (hash ^ byte as u64).wrapping_mul(0x0100_0000_01b3);
            }
        };
        for i in 0..64 {
            let m = i as f32 * 7.7 - 240.0;
            eat(g.surface_height_m(m, m * 0.5).to_bits());
        }
        let chunk = g.chunk(0, 0, 0);
        for x in 0..crate::CHUNK_SIZE {
            for y in 0..crate::CHUNK_SIZE {
                for z in 0..crate::CHUNK_SIZE {
                    let v = chunk.get(x, y, z);
                    eat(v.material() as u32
                        | (v.shape() as u32) << 16
                        | (v.orientation() as u32) << 24);
                }
            }
        }
        hash
    }

    #[test]
    fn boundary_seeds_generate_without_overflow() {
        for seed in [
            0,
            1,
            u32::MAX as u64 - 3,
            u32::MAX as u64,
            0xffff_ffff_0000_0000,
            0x0000_0001_0000_0001,
            u64::MAX - 1,
            u64::MAX,
        ] {
            terrain_digest(seed, crate::testgen::world_params());
        }
    }

    /// The u64 → u32 fold lets distinct seeds share terrain, which the spec allows.
    #[test]
    fn seeds_whose_halves_cancel_fold_to_the_same_world() {
        let digest = |seed| terrain_digest(seed, crate::testgen::world_params());
        assert_eq!(digest(0x0000_0001_0000_0001), digest(0));
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
            Some(Voxel::cube(crate::testgen::fixtures().ground.rock)),
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
    fn a_version_3_settlement_is_founded_only_where_its_disc_is_level() {
        let g = Worldgen::new(SEED, 1.0, crate::testgen::world_params());
        let reach_m = 40.0;
        let rise = |x: f32, z: f32| {
            let h = g.surface_height_m(x, z);
            let r = (0..16)
                .map(|k| {
                    let a = k as f32 * std::f32::consts::TAU / 16.0;
                    (g.surface_height_m(x + reach_m * a.cos(), z + reach_m * a.sin()) - h).abs()
                })
                .fold(0.0, f32::max);
            (h, r)
        };
        let (mut steep, mut level) = (0, 0);
        for j in -40..40 {
            for i in -40..40 {
                let [x, z] = [i as f32 * 97.0, j as f32 * 89.0];
                let (h, r) = rise(x, z);
                if h < 30.0 {
                    continue;
                }
                if r > 12.0 {
                    steep += 1;
                    assert!(
                        !g.level_disc([x, z], h, reach_m),
                        "founded on a slope at {x},{z}"
                    );
                } else if r < 1.5 {
                    level += 1;
                    assert!(
                        g.level_disc([x, z], h, reach_m),
                        "refused level ground at {x},{z}"
                    );
                }
            }
        }
        assert!(
            steep > 50 && level > 50,
            "{steep} steep and {level} level sites"
        );
    }

    #[test]
    fn shafts_open_a_way_from_the_ground_down_into_a_cavern() {
        let params = WorldgenParams {
            smoothing: None,
            water: None,
            // Tunnels thinner than a Voxel, so the air found is the shaft's.
            caves: Some(CaveParams {
                radius_m: 0.001,
                ..CaveParams::default()
            }),
            ..crate::testgen::world_params()
        };
        let g = Worldgen::new(SEED, 1.0, params);
        let shafts = g.caverns.as_ref().unwrap().shafts(
            [-3000.0, -3000.0],
            [3000.0, 3000.0],
            |x, z| g.surface_height_m(x, z),
            |x, z| g.water_surface_m(x, z).is_none(),
        );
        assert!(shafts.len() > 20, "{} shafts", shafts.len());
        let air = |x: i32, y: i32, z: i32| {
            let s = CHUNK_SIZE as i32;
            g.chunk(x.div_euclid(s), y.div_euclid(s), z.div_euclid(s))
                .get(
                    x.rem_euclid(s) as usize,
                    y.rem_euclid(s) as usize,
                    z.rem_euclid(s) as usize,
                )
                .is_air()
        };
        let open = shafts
            .iter()
            .take(12)
            .filter(|s| {
                let [x, z] = s.centre_m.map(|m| m.round());
                let top = g.surface_height_m(x, z).floor() as i32 - 1;
                let (x, z) = (x as i32, z as i32);
                (s.bottom_m.ceil() as i32..top).all(|y| air(x, y, z))
                    && air(x, s.bottom_m.floor() as i32 - 1, z)
            })
            .count();
        assert!(
            open >= 9,
            "{open} of 12 shafts reach a cavern from the ground"
        );
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
                    if below_m < 0.0
                        || g.caves
                            .as_ref()
                            .is_some_and(|c| c.carves(x_m, y_m, z_m, g.cave_scale_at(x_m, z_m)))
                    {
                        continue;
                    }
                    let material = if below_m < g.voxel_size {
                        crate::testgen::fixtures().ground.surface
                    } else if below_m < g.params.soil_depth_m {
                        crate::testgen::fixtures().ground.soil
                    } else {
                        crate::testgen::fixtures().ground.rock
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

    fn voxels(chunk: &Chunk) -> Vec<Voxel> {
        (0..CHUNK_SIZE)
            .flat_map(|z| {
                (0..CHUNK_SIZE).flat_map(move |y| (0..CHUNK_SIZE).map(move |x| (x, y, z)))
            })
            .map(|(x, y, z)| chunk.get(x, y, z))
            .collect()
    }

    /// Regions small enough that a sweep crosses several borders.
    fn tuned(voxel_size: f32) -> Worldgen {
        let biome = |name: &str, temperature, cave_scale| crate::Biome {
            name: crate::BiomeName::new(name).unwrap(),
            temperature,
            humidity: 0.5,
            ground: None,
            sky_fog: None,
            cave_scale: Some(cave_scale),
            surface_features: Default::default(),
        };
        Worldgen::new(
            SEED,
            voxel_size,
            WorldgenParams {
                smoothing: None,
                water: None,
                climate: ClimateParams {
                    temperature_size_m: 24.0,
                    humidity_size_m: 24.0,
                },
                biomes: BiomeTable::new(&[biome("none", 0.1, 0.0), biome("wide", 0.9, 3.0)])
                    .unwrap(),
                ..crate::testgen::world_params()
            },
        )
    }

    #[test]
    fn ruling_a_box_out_never_changes_a_tuned_chunk() {
        // A bound taken at the World's own scale would rule out boxes a wider
        // Biome carves.
        for voxel_size in [1.0, 0.5] {
            let g = tuned(voxel_size);
            let untuned = caved(voxel_size);
            let mut changed = 0;
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
                changed += usize::from(voxels(&fast) != voxels(&untuned.chunk(cx, cy, cz)));
            }
            assert!(changed > 0, "tuning changed no Chunk at {voxel_size} m");
        }
    }

    #[test]
    fn cave_scale_has_no_step_across_a_border() {
        let g = tuned(1.0);
        let (mut previous, mut largest_step, mut between) = (None::<f32>, 0.0f32, 0);
        let (mut lowest, mut highest) = (f32::MAX, f32::MIN);
        let mut biomes = std::collections::HashSet::new();
        for i in 0..8000 {
            let x_m = -500.0 + i as f32 * 0.25;
            let scale = g.cave_scale_at(x_m, 37.0);
            biomes.insert(g.biome_at(x_m, 37.0));
            if let Some(was) = previous {
                largest_step = largest_step.max((scale - was).abs());
            }
            previous = Some(scale);
            (lowest, highest) = (lowest.min(scale), highest.max(scale));
            between += usize::from(scale > 0.3 && scale < 2.7);
        }
        assert!(biomes.len() == 2, "the line crosses no border");
        // The Biomes differ by 3; a step at the ground's border would be all of it.
        assert!(largest_step < 0.5, "a step of {largest_step}");
        assert!(between > 100, "only {between} samples are a blend");
        assert!(lowest < 0.01 && highest > 2.99, "{lowest}..{highest}");
    }

    #[test]
    fn where_the_scale_is_zero_there_is_no_cave() {
        let g = tuned(1.0);
        let off: Vec<_> = (-60..60)
            .flat_map(|x| (-60..60).map(move |z| (x as f32 * 2.0, z as f32 * 2.0)))
            .filter(|&(x, z)| g.cave_scale_at(x, z) == 0.0)
            .collect();
        assert!(off.len() > 500, "{} columns have caves off", off.len());
        let caves = g.caves.as_ref().unwrap();
        for (x, z) in off {
            for y in -24..20 {
                assert!(
                    !caves.carves(x, y as f32, z, g.cave_scale_at(x, z)),
                    "{x},{y},{z}"
                );
            }
        }
    }

    #[test]
    fn tuning_that_is_not_asked_for_changes_nothing() {
        let tuned_params = tuned(1.0).params;
        let table = |scales: [Option<f32>; 2]| {
            let mut biomes = tuned_params.biomes.as_slice().to_vec();
            for (b, s) in biomes.iter_mut().zip(scales) {
                b.cave_scale = s;
            }
            BiomeTable::new(&biomes).unwrap()
        };
        let chunks = |params: WorldgenParams| -> Vec<Voxel> {
            let g = Worldgen::new(SEED, 1.0, params);
            sweep(1.0)
                .into_iter()
                .flat_map(|(x, y, z)| voxels(&g.chunk(x, y, z)))
                .collect()
        };
        let plain = chunks(WorldgenParams {
            biomes: table([None, None]),
            ..tuned_params
        });
        // Scale 1 is the World's own caves, so it moves not a single Voxel.
        assert!(
            plain
                == chunks(WorldgenParams {
                    biomes: table([Some(1.0), Some(1.0)]),
                    ..tuned_params
                })
        );
        // A World without caves has nothing to tune; coarse LOD levels are built
        // that way.
        let bare = |biomes| WorldgenParams {
            caves: None,
            biomes,
            ..tuned_params
        };
        assert!(chunks(bare(tuned_params.biomes)) == chunks(bare(table([None, None]))));
    }

    #[test]
    fn a_tuned_world_still_only_removes_voxels() {
        let (with, without) = (tuned(1.0), terrain(1.0));
        let mut carved = 0;
        for (cx, cy, cz) in sweep(1.0) {
            let (a, b) = (with.chunk(cx, cy, cz), without.chunk(cx, cy, cz));
            for z in 0..CHUNK_SIZE {
                for y in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        let (got, was) = (a.get(x, y, z), b.get(x, y, z));
                        if got != was {
                            assert!(got.is_air(), "{cx},{cy},{cz} {x},{y},{z}");
                            carved += 1;
                        }
                    }
                }
            }
        }
        assert!(carved > 0);
        assert_eq!(with.surface_bounds_m(), without.surface_bounds_m());
    }

    #[test]
    fn a_biomes_scale_carves_the_same_metre_point_at_any_voxel_size() {
        let (coarse, fine) = (tuned(1.0), tuned(0.5));
        let (mut carved, mut checked) = (0, 0);
        for i in 0..2000 {
            let (x, z) = (i * 7 % 61 - 30, i * 13 % 67 - 33);
            let y = -20 + (i * 5) % 70;
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
                        .carves(x as f32, y as f32, z as f32, 1.0),
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
        let tunnels = crate::testgen::world_params().caves.unwrap().floor_m;
        let floor = tunnels.min(g.caverns.as_ref().unwrap().floor_m);
        let cell = CHUNK_SIZE as f32;
        let below = (floor / cell).floor() as i32 - 1;
        for (cx, cz) in [(0, 0), (4, -3), (-7, 2)] {
            assert_eq!(
                g.chunk(cx, below, cz).uniform(),
                Some(Voxel::cube(crate::testgen::fixtures().ground.rock)),
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
                floor_m: -250.0,
                ..CaveParams::default()
            }),
            ..crate::testgen::world_params()
        };
        let g = Worldgen::new(SEED, 1.0, params);
        // Version 3's band reaches below the tunnels, and a Chunk inside it is seldom stored
        // uniform: so the layers just under the band.
        let below = (g.surface_bounds_m().0 / CHUNK_SIZE as f32).floor() as i32;
        let (mut uniform, mut carved) = (0, 0);
        for cx in -5..5 {
            for cz in -5..5 {
                for cy in below - 4..below {
                    match g.chunk(cx, cy, cz).uniform() {
                        Some(v) => {
                            assert_eq!(v, Voxel::cube(crate::testgen::fixtures().ground.rock));
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
            let grass: Vec<usize> = (0..c.len())
                .filter(|&y| c[y].material() == crate::testgen::fixtures().ground.surface)
                .collect();
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
                            mouths += usize::from(
                                b.get(x, y, z).material()
                                    == crate::testgen::fixtures().ground.surface,
                            );
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
        assert!(
            walls.contains(&crate::testgen::fixtures().ground.soil),
            "{walls:?}"
        );
        assert!(
            walls.contains(&crate::testgen::fixtures().ground.rock),
            "{walls:?}"
        );
    }

    /// The fixture water and a lava that is hot, opaque and keeps out of
    /// Shapes: a lake of either is made the same way.
    fn lake_tables() -> (Tables, [MaterialId; 2]) {
        let mut tables = crate::testgen::fixtures().tables.clone();
        let water = crate::testgen::fixtures().water;
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

    fn flooded(voxel_size: f32, material: MaterialId) -> Worldgen {
        let params = crate::testgen::world_params();
        Worldgen::new(
            COAST,
            voxel_size,
            WorldgenParams {
                water: Some(WaterParams {
                    level_m: params.sea_level_m,
                    material,
                }),
                // Smoothing yields to water, a cavern under the water table holds it and overhangs
                // leave the heightfield; this is about the water alone.
                smoothing: None,
                caves: None,
                landforms: crate::terrain::LandformParams {
                    overhangs: false,
                    ..params.landforms
                },
                ..params
            },
        )
    }

    /// Water fills the air between the ground and the water surface over it, and nothing else.
    #[test]
    fn water_fills_the_open_air_between_the_ground_and_its_level_and_nothing_else() {
        let (_, fluids) = lake_tables();
        for (voxel_size, material) in [1.0f32, 0.5]
            .into_iter()
            .flat_map(|v| fluids.map(|m| (v, m)))
        {
            let g = flooded(voxel_size, material);
            let mut filled = 0;
            for (cx, cy, cz) in sweep(voxel_size) {
                let chunk = g.chunk(cx, cy, cz);
                for z in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        let x_m = g.axis_m(cx, x as i32);
                        let z_m = g.axis_m(cz, z as i32);
                        let surface_m = g.surface_height_m(x_m, z_m);
                        let level_m = g.water_surface_m(x_m, z_m).unwrap_or(f32::MIN);
                        for y in 0..CHUNK_SIZE {
                            let y_m = g.axis_m(cy, y as i32);
                            let wet = chunk.get(x, y, z) == Voxel::cube(material);
                            let open = y_m > surface_m && y_m < level_m;
                            assert_eq!(wet, open, "{x_m} {y_m} {z_m}");
                            filled += usize::from(wet);
                        }
                    }
                }
            }
            assert!(filled > 0, "the sweep floods something at {voxel_size} m");
        }
    }

    #[test]
    fn the_default_parameters_have_no_lakes_and_generate_no_fluid() {
        let params = WorldgenParams {
            ground: crate::testgen::world_params().ground,
            ..WorldgenParams::default()
        };
        assert_eq!(params.water, None);
        let g = Worldgen::new(COAST, 1.0, params);
        for (cx, cy, cz) in sweep(1.0) {
            for v in g.chunk(cx, cy, cz).kinds() {
                assert!(
                    [
                        crate::voxel::AIR,
                        crate::testgen::fixtures().ground.rock,
                        crate::testgen::fixtures().ground.soil,
                        crate::testgen::fixtures().ground.surface
                    ]
                    .contains(&v.material()),
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
            let g = flooded(1.0, material);
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
                COAST,
                1.0,
                WorldgenParams {
                    smoothing,
                    ..crate::testgen::world_params()
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

    #[test]
    fn a_world_without_caves_is_what_it_was() {
        // `caves: None` costs one `Option` check and changes nothing else.
        let g = terrain(1.0);
        assert!(g.caves.is_none());
        let cell = CHUNK_SIZE as f32;
        let cy = (g.surface_height_m(0.0, 0.0) / cell).floor() as i32;
        assert_eq!(g.chunk(0, cy, 0).uniform(), None);
        let (lowest_m, _) = g.surface_bounds_m();
        assert_eq!(
            g.chunk(0, (lowest_m / cell).floor() as i32 - 1, 0)
                .uniform(),
            Some(Voxel::cube(crate::testgen::fixtures().ground.rock))
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
                        chunk.set(
                            x,
                            y,
                            z,
                            Voxel::cube(crate::testgen::fixtures().ground.surface),
                        );
                    }
                }
            }
        }
        g.smooth(&mut chunk, &surface, [0, 0, 0], 0.0, None);
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

    /// At [`COAST`] the overhang density pushes cells past the surface the steps are read from.
    #[test]
    fn smoothing_stairs_face_the_step_they_climb() {
        for seed in [SEED, COAST] {
            for size in [1.0, 0.5] {
                assert_stairs_face_their_step(&smoothed_at(seed, size, false));
            }
        }
    }

    fn assert_stairs_face_their_step(g: &Worldgen) {
        use crate::mesh::{NEG_X, NEG_Z, POS_X, POS_Z};
        use crate::shape::ShapeTable;
        let table = ShapeTable::builtin();
        let mut probe = Probe::new(g);
        let sides = [
            (POS_X, [1, 0, 0]),
            (NEG_X, [-1, 0, 0]),
            (POS_Z, [0, 0, 1]),
            (NEG_Z, [0, 0, -1]),
        ];
        let (mut stairs, mut slabs, mut facings) = (0, 0, std::collections::HashSet::new());
        for (at, v) in shaped_voxels(g, &mut probe) {
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
            assert_eq!(v.material(), crate::testgen::fixtures().ground.surface);
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
        for seed in [SEED, COAST] {
            for size in [1.0, 0.5] {
                let (at_edge, total) = assert_ramps_are_whole(&smoothed_at(seed, size, true));
                assert!(
                    at_edge > 0 && total > at_edge,
                    "{seed} {size} m: {at_edge} of {total}"
                );
            }
        }

        let g = smoothed(1.0, true);

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
            let defaults = crate::testgen::world_params();
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
            let defaults = crate::testgen::world_params();
            let g = Worldgen::new(
                COAST,
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
        let defaults = crate::testgen::world_params();
        let any = |voxel_size: f32| {
            let g = Worldgen::new(COAST, voxel_size, defaults);
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
        let soil = crate::testgen::world_params().soil_depth_m;
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
            let lake = |material| crate::testgen::world_params().with_default_lake(material);
            for material in fluids {
                assert_eq!(lake(material).validate_lake(&tables), Ok(()));
            }
            assert_eq!(
                crate::testgen::world_params().validate_lake(&tables),
                Ok(())
            );
            for (material, rule) in [
                (999, "names no registered Material"),
                (
                    crate::testgen::fixtures().ground.rock,
                    "is solid, not a fluid",
                ),
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
            let d = crate::testgen::world_params();
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
                let d = crate::testgen::world_params();
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
            let d = crate::testgen::world_params();
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
            let d = crate::testgen::world_params();
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
            let d = crate::testgen::world_params();
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
