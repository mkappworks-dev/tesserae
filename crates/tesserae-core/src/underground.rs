//! Version 3's underground: cave Biomes chosen in 3D, lava near the band's floor, aquifers that
//! raise the water table in regions and the springs they feed.

use crate::climate::{BiomeName, SkyFog, SkyFogBlend};
use crate::noise::{Fbm3Cache, fbm3_cached};
use crate::voxel::{MaterialId, SHAPE_FULL_CUBE, ShapeId};
use crate::worldgen::ParamError;

pub const MAX_CAVE_BIOMES: usize = 16;
pub const MAX_CAVE_FEATURES: usize = 8;
/// The sparsest a cave feature stands, one in this many metres square of floor.
pub const MAX_CAVE_FEATURE_SPACING_M: f32 = 64.0;

/// A Material standing on a cave Biome's dry floor, about one in every `spacing_m` by `spacing_m`
/// metres of it.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CaveFeature {
    pub material: MaterialId,
    pub shape: ShapeId,
    pub spacing_m: f32,
}

impl CaveFeature {
    const UNUSED: CaveFeature = CaveFeature {
        material: 0,
        shape: SHAPE_FULL_CUBE,
        spacing_m: 1.0,
    };
}

/// A region of the underground with its own rock, floor, features and fog, chosen where the
/// cave Climate, two 3D fields in metres, lies nearest its point.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CaveBiome {
    pub name: BiomeName,
    pub temperature: f32,
    pub humidity: f32,
    /// Metres above Sea level, negative under it; `None` matches every height.
    pub altitude_m: Option<[f32; 2]>,
    /// Replaces the ground's rock and strata in the region; `None` keeps them.
    pub rock: Option<MaterialId>,
    /// The Voxel under a cavern's open floor; `None` keeps the rock.
    pub floor: Option<MaterialId>,
    /// The air in its caverns; `None` keeps the surface's.
    pub fog: Option<SkyFog>,
    features_len: u8,
    features: [CaveFeature; MAX_CAVE_FEATURES],
}

impl CaveBiome {
    const UNUSED: CaveBiome = CaveBiome {
        name: BiomeName::UNUSED,
        temperature: 0.0,
        humidity: 0.0,
        altitude_m: None,
        rock: None,
        floor: None,
        fog: None,
        features_len: 0,
        features: [CaveFeature::UNUSED; MAX_CAVE_FEATURES],
    };

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: BiomeName,
        (temperature, humidity): (f32, f32),
        altitude_m: Option<[f32; 2]>,
        rock: Option<MaterialId>,
        floor: Option<MaterialId>,
        fog: Option<SkyFog>,
        features: &[CaveFeature],
    ) -> Result<Self, ParamError> {
        let refuse = |field, rule| ParamError { field, rule };
        let unit = |v: f32| (0.0..=1.0).contains(&v);
        if !(unit(temperature) && unit(humidity)) {
            return Err(refuse("cave_biomes", "must place every point from 0 to 1"));
        }
        if !altitude_m.is_none_or(|[lo, hi]| lo.is_finite() && hi.is_finite() && lo <= hi) {
            return Err(refuse(
                "cave_biomes.altitude_m",
                "must give a finite low end not above its high end",
            ));
        }
        if features.len() > MAX_CAVE_FEATURES {
            return Err(refuse("cave_biomes.features", "holds at most 8 features"));
        }
        if !features
            .iter()
            .all(|f| (1.0..=MAX_CAVE_FEATURE_SPACING_M).contains(&f.spacing_m))
        {
            return Err(refuse(
                "cave_biomes.features.spacing_m",
                "must be from 1 to 64 m",
            ));
        }
        let mut biome = CaveBiome {
            name,
            temperature,
            humidity,
            altitude_m,
            rock,
            floor,
            fog,
            ..CaveBiome::UNUSED
        };
        biome.features[..features.len()].copy_from_slice(features);
        biome.features_len = features.len() as u8;
        Ok(biome)
    }

    pub fn features(&self) -> &[CaveFeature] {
        &self.features[..usize::from(self.features_len)]
    }

    fn admits(&self, altitude_m: f32) -> bool {
        self.altitude_m
            .is_none_or(|[lo, hi]| (lo..=hi).contains(&altitude_m))
    }
}

/// At most [`MAX_CAVE_BIOMES`], distinctly named. Empty leaves the underground as the surface
/// Biomes lay it.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CaveBiomeTable {
    len: u8,
    biomes: [CaveBiome; MAX_CAVE_BIOMES],
}

impl Default for CaveBiomeTable {
    fn default() -> Self {
        CaveBiomeTable {
            len: 0,
            biomes: [CaveBiome::UNUSED; MAX_CAVE_BIOMES],
        }
    }
}

impl CaveBiomeTable {
    pub fn new(biomes: &[CaveBiome]) -> Result<Self, ParamError> {
        if biomes.len() > MAX_CAVE_BIOMES {
            return Err(ParamError {
                field: "cave_biomes",
                rule: "holds at most 16 cave Biomes",
            });
        }
        if biomes
            .iter()
            .enumerate()
            .any(|(i, b)| biomes[..i].iter().any(|earlier| earlier.name == b.name))
        {
            return Err(ParamError {
                field: "cave_biomes",
                rule: "must give every cave Biome its own name",
            });
        }
        let mut table = CaveBiomeTable::default();
        table.biomes[..biomes.len()].copy_from_slice(biomes);
        table.len = biomes.len() as u8;
        Ok(table)
    }

    pub fn as_slice(&self) -> &[CaveBiome] {
        &self.biomes[..usize::from(self.len)]
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// The edge of the lattice a cave Biome is chosen on: coarse enough that a chunk asks a few
/// hundred times, fine enough that a border wanders.
pub(crate) const CAVE_CELL_M: f32 = 4.0;
/// The size of the cave Climate's regions.
const CAVE_CLIMATE_SIZE_M: f32 = 160.0;
/// The field gathers round its middle, a tenth of it under 0.32 and a tenth over 0.68; stretched
/// by this, those tenths reach 0.1 and 0.9, so a point declared near a corner is ever chosen.
const CAVE_CLIMATE_STRETCH: f32 = 2.2;
const SALT_CAVE_TEMPERATURE: u32 = 0xca7e_0001;
const SALT_CAVE_HUMIDITY: u32 = 0xca7e_0002;
/// How far under the surface a cave Biome's rock, floor and fog begin, so the ground a Player
/// walks on stays the surface Biome's.
pub(crate) const CAVE_BIOME_MIN_DEPTH_M: f32 = 12.0;

/// The cave Biome at the centre of the [`CAVE_CELL_M`] lattice cell holding a point, by index:
/// the nearest point to the cave Climate there among those admitting its height. A pure function
/// of the seed and metres, so every Voxel size and Chunk order chooses the same.
pub(crate) fn cave_biome_at(
    table: &CaveBiomeTable,
    seed: u32,
    sea_level_m: f32,
    at: [f32; 3],
) -> Option<u8> {
    cave_biome_near(table, seed, sea_level_m, at, &mut Default::default())
}

/// [`cave_biome_at`], bit for bit, for a run of nearby points sharing the corners of their
/// cave Climate's cells.
fn cave_biome_near(
    table: &CaveBiomeTable,
    seed: u32,
    sea_level_m: f32,
    [x_m, y_m, z_m]: [f32; 3],
    [temperature, humidity]: &mut [Fbm3Cache; 2],
) -> Option<u8> {
    if table.is_empty() {
        return None;
    }
    let centre = [x_m, y_m, z_m].map(|m| ((m / CAVE_CELL_M).floor() + 0.5) * CAVE_CELL_M);
    let [x, y, z] = centre.map(|m| m / CAVE_CLIMATE_SIZE_M);
    let unit = |salt, cache| {
        let n = fbm3_cached(x, y, z, seed ^ salt, 2, cache);
        ((n - 0.5) * CAVE_CLIMATE_STRETCH + 0.5).clamp(0.0, 1.0)
    };
    let (t, h) = (
        unit(SALT_CAVE_TEMPERATURE, temperature),
        unit(SALT_CAVE_HUMIDITY, humidity),
    );
    let altitude_m = centre[1] - sea_level_m;
    table
        .as_slice()
        .iter()
        .enumerate()
        .filter(|(_, b)| b.admits(altitude_m))
        .min_by(|(_, a), (_, b)| {
            let d = |c: &CaveBiome| {
                let (dt, dh) = (c.temperature - t, c.humidity - h);
                (dt * dt + dh * dh).sqrt()
            };
            d(a).total_cmp(&d(b))
        })
        .map(|(i, _)| i as u8)
}

/// The cave Biome of each [`CAVE_CELL_M`] cell a Chunk spans, asked once each, indexed from the
/// Chunk's first cell. Empty where a Voxel is at least a cell wide, since no two Voxels then share
/// a cell.
pub(crate) struct CaveBiomeCells {
    origin: [i32; 3],
    span: [usize; 3],
    cells: Vec<Option<Option<u8>>>,
    noise: [Fbm3Cache; 2],
}

impl CaveBiomeCells {
    /// For `voxels` Voxels a side from `origin_m`, the `i`th at `origin_m + i * voxel_size`: a cell
    /// is monotone in metres, so every Voxel placed so falls in the span.
    pub(crate) fn new(kept: bool, origin_m: [f32; 3], voxel_size: f32, voxels: usize) -> Self {
        let last_m = (voxels - 1) as f32 * voxel_size;
        let origin = origin_m.map(cave_cell);
        let span =
            std::array::from_fn(|i| (cave_cell(origin_m[i] + last_m) - origin[i] + 1) as usize);
        let len = if kept && voxel_size < CAVE_CELL_M {
            span.iter().product()
        } else {
            0
        };
        CaveBiomeCells {
            origin,
            span,
            cells: vec![None; len],
            noise: Default::default(),
        }
    }

    /// [`cave_biome_at`] for a Voxel of the Chunk.
    pub(crate) fn get(
        &mut self,
        table: &CaveBiomeTable,
        seed: u32,
        sea_level_m: f32,
        at: [f32; 3],
    ) -> Option<u8> {
        let mut ask = || cave_biome_near(table, seed, sea_level_m, at, &mut self.noise);
        if self.cells.is_empty() {
            return ask();
        }
        let [x, y, z] = std::array::from_fn(|i| (cave_cell(at[i]) - self.origin[i]) as usize);
        let [w, h, _] = self.span;
        *self.cells[(z * h + y) * w + x].get_or_insert_with(ask)
    }
}

fn cave_cell(m: f32) -> i32 {
    (m / CAVE_CELL_M).floor() as i32
}

impl SkyFog {
    pub(crate) fn blend(self) -> SkyFogBlend {
        let unit = |c: [u8; 3]| c.map(|v| f32::from(v) / 255.0);
        SkyFogBlend {
            sky: unit(self.sky),
            fog: unit(self.fog),
        }
    }
}

/// How far above the band's floor caverns hold lava.
pub(crate) const LAVA_DEPTH_M: f32 = 10.0;
/// The rock kept between the lava caverns and every cavern above them, so no lava meets water.
pub(crate) const LAVA_SEAL_M: f32 = 4.0;

/// Square regions, each with one water table, so a cavern lake is level wherever it stands.
pub(crate) const AQUIFER_CELL_M: f32 = 192.0;
/// The rock kept along a region's border wherever a raised table stands beside it, so no lake
/// drains into its neighbour.
pub(crate) const AQUIFER_WALL_M: f32 = 4.0;
/// The share of regions whose table is raised, and how far.
const AQUIFER_SHARE: f32 = 0.35;
pub(crate) const AQUIFER_RISE_M: (f32, f32) = (6.0, 30.0);
const SALT_AQUIFER: u32 = 0xa9f1_0001;

/// How far above the World's water table a region's own table stands: 0 for most, a perched
/// aquifer for some.
pub(crate) fn aquifer_rise_m(seed: u32, [ix, iz]: [i32; 2]) -> f32 {
    let [chance, depth, _] = crate::noise::cell_hash(ix, iz, seed ^ SALT_AQUIFER);
    if chance >= AQUIFER_SHARE {
        return 0.0;
    }
    let (lo, hi) = AQUIFER_RISE_M;
    (lo + depth * (hi - lo)).round()
}

pub(crate) fn aquifer_cell([x_m, z_m]: [f32; 2]) -> [i32; 2] {
    [x_m, z_m].map(|m| (m / AQUIFER_CELL_M).floor() as i32)
}

/// One in this many square metres of an aquifer's cavern ceiling above its lake drips.
pub(crate) const SPRING_SPACING_M: f32 = 24.0;
