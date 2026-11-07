//! Climate and Biome selection: which kind of ground a column is, as a pure
//! function of the seed and a position in **metres**.
//!
//! Nothing here knows a Voxel size, so the answer is the same at every Voxel
//! size and for every LOD level generator
//! (`docs/adr/0003-voxel-size-is-per-world.md`,
//! `docs/adr/0011-coarse-levels-are-derived.md`).

use crate::noise::fbm2;
use crate::voxel::{AIR, MaterialId};
use crate::worldgen::{GroundParams, ParamError};

/// Fixed, not a parameter: a declaration sizes the regions, the border warp adds detail.
const OCTAVES: u32 = 3;

/// Far from the terrain octaves' `seed + 0..octaves` and the cave fields, so Climate does not
/// track the surface.
const TEMPERATURE_SEED: u32 = 0x0c11_a7e0;
const HUMIDITY_SEED: u32 = 0x0c11_a8e0;
const WARP_TEMPERATURE_SEED: u32 = 0x0c11_a9e0;
const WARP_HUMIDITY_SEED: u32 = 0x0c11_aae0;

/// Version 1 stays frozen as it was before Biomes existed.
pub const BIOMES_SINCE: u32 = 2;

/// Bounded so [`BiomeTable`] stays inline and [`crate::WorldgenParams`] stays `Copy`.
pub const MAX_BIOMES: usize = 16;

pub const MAX_BIOME_NAME_BYTES: usize = 32;

/// A Biome point further than this (Climate units) from a straight border always wins its side, so
/// a Game can place points knowing where the warp stops.
pub const BORDER_WARP: f32 = 0.1;

/// The warp must vary faster than the Climate it bends, or borders would only drift smoothly.
const WARP_SIZE_FRACTION: f32 = 0.25;

/// Wider than [`BORDER_WARP`], so a blend is already under way where the warp has pushed the hard
/// border to.
const BLEND_RANGE: f32 = 0.15;

/// A tunnel four times as wide carves sixteen times the rock: the most before the ground is lace.
pub const MAX_CAVE_SCALE: f32 = 4.0;

/// Bounded so a [`Biome`] stays inline and `Copy`.
pub const MAX_SURFACE_FEATURES: usize = 4;

/// From one in every 4 by 4 m of ground, already a carpet, to one in a square kilometre.
pub const SURFACE_FEATURE_SPACING_M: std::ops::RangeInclusive<f32> = 4.0..=1024.0;

/// Each field in `[0, 1)`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Climate {
    pub temperature: f32,
    pub humidity: f32,
}

/// Horizontal period, in metres, of the lowest octave of each field.
#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClimateParams {
    pub temperature_size_m: f32,
    pub humidity_size_m: f32,
}

impl Default for ClimateParams {
    fn default() -> Self {
        ClimateParams {
            temperature_size_m: 512.0,
            humidity_size_m: 512.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ClimateField {
    seed: u32,
    inv_temperature_size: f32,
    inv_humidity_size: f32,
}

impl ClimateField {
    pub(crate) fn new(seed: u32, p: ClimateParams) -> Self {
        ClimateField {
            seed,
            inv_temperature_size: 1.0 / p.temperature_size_m,
            inv_humidity_size: 1.0 / p.humidity_size_m,
        }
    }

    pub(crate) fn at(&self, x_m: f32, z_m: f32) -> Climate {
        Climate {
            temperature: fbm2(
                x_m * self.inv_temperature_size,
                z_m * self.inv_temperature_size,
                self.seed.wrapping_add(TEMPERATURE_SEED),
                OCTAVES,
            ),
            humidity: fbm2(
                x_m * self.inv_humidity_size,
                z_m * self.inv_humidity_size,
                self.seed.wrapping_add(HUMIDITY_SEED),
                OCTAVES,
            ),
        }
    }
}

impl ClimateField {
    /// The Climate pushed by a second, faster pair of fields, so a border follows no contour of
    /// either.
    fn warped(&self, x_m: f32, z_m: f32) -> Climate {
        let c = self.at(x_m, z_m);
        let offset = |n: f32| (n - 0.5) * 2.0 * BORDER_WARP;
        let (kt, kh) = (
            self.inv_temperature_size / WARP_SIZE_FRACTION,
            self.inv_humidity_size / WARP_SIZE_FRACTION,
        );
        Climate {
            temperature: c.temperature
                + offset(fbm2(
                    x_m * kt,
                    z_m * kt,
                    self.seed.wrapping_add(WARP_TEMPERATURE_SEED),
                    2,
                )),
            humidity: c.humidity
                + offset(fbm2(
                    x_m * kh,
                    z_m * kh,
                    self.seed.wrapping_add(WARP_HUMIDITY_SEED),
                    2,
                )),
        }
    }

    /// The mean of `values` (indexed by [`BiomeId`]) weighted by nearness to the warped Climate: it
    /// follows the borders of [`ClimateField::biome_at`] but moves continuously across them. `0.0`
    /// for an empty table.
    pub(crate) fn blend(
        &self,
        biomes: &BiomeTable,
        x_m: f32,
        z_m: f32,
        values: &[f32; MAX_BIOMES],
    ) -> f32 {
        if biomes.is_empty() {
            return 0.0;
        }
        let c = self.warped(x_m, z_m);
        let distances: [f32; MAX_BIOMES] = std::array::from_fn(|i| {
            biomes.as_slice().get(i).map_or(f32::INFINITY, |b| {
                let (dt, dh) = (b.temperature - c.temperature, b.humidity - c.humidity);
                (dt * dt + dh * dh).sqrt()
            })
        });
        let nearest = distances.iter().copied().fold(f32::INFINITY, f32::min);
        let (mut sum, mut weights) = (0.0, 0.0);
        for (d, v) in distances.iter().zip(values) {
            let w = (1.0 - (d - nearest) / BLEND_RANGE).max(0.0);
            sum += w * v;
            weights += w;
        }
        sum / weights
    }

    /// The earlier of two equally near Biomes. `None` for an empty table.
    pub(crate) fn biome_at(&self, biomes: &BiomeTable, x_m: f32, z_m: f32) -> Option<BiomeId> {
        if biomes.is_empty() {
            return None;
        }
        let c = self.warped(x_m, z_m);
        let distance = |b: &Biome| {
            let (dt, dh) = (b.temperature - c.temperature, b.humidity - c.humidity);
            dt * dt + dh * dh
        };
        let mut nearest = (0, distance(&biomes.as_slice()[0]));
        for (i, b) in biomes.as_slice().iter().enumerate().skip(1) {
            let d = distance(b);
            if d < nearest.1 {
                nearest = (i, d);
            }
        }
        Some(BiomeId(nearest.0 as u8))
    }

    /// Continuous in position, so colour never steps where [`Self::biome_at`] does. `None` where no
    /// Biome has colours.
    pub(crate) fn sky_fog_at(
        &self,
        biomes: &BiomeTable,
        x_m: f32,
        z_m: f32,
    ) -> Option<SkyFogBlend> {
        let coloured = || {
            biomes
                .as_slice()
                .iter()
                .filter_map(|b| Some((b, b.sky_fog?)))
        };
        // Unwarped: the warp varies fast to roughen a border, and the air would shimmer with it.
        let c = self.at(x_m, z_m);
        let distance = |b: &Biome| {
            let (dt, dh) = (b.temperature - c.temperature, b.humidity - c.humidity);
            dt.hypot(dh)
        };
        let nearest = coloured().map(|(b, _)| distance(b)).reduce(f32::min)?;
        let (mut sky, mut fog, mut total) = ([0.0; 3], [0.0; 3], 0.0);
        for (b, colours) in coloured() {
            // Relative to the nearest so nothing underflows.
            let weight = (-(distance(b) - nearest) / SKY_BLEND_WIDTH).exp();
            total += weight;
            for i in 0..3 {
                sky[i] += weight * f32::from(colours.sky[i]);
                fog[i] += weight * f32::from(colours.fog[i]);
            }
        }
        let unit = |sum: [f32; 3]| sum.map(|v| v / total / 255.0);
        Some(SkyFogBlend {
            sky: unit(sky),
            fog: unit(fog),
        })
    }
}

/// An index, not a name: only meaningful beside the [`BiomeTable`] that made it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct BiomeId(u8);

impl BiomeId {
    pub const fn new(index: u8) -> Self {
        BiomeId(index)
    }

    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// 1 to [`MAX_BIOME_NAME_BYTES`] bytes of UTF-8, held inline.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct BiomeName {
    len: u8,
    bytes: [u8; MAX_BIOME_NAME_BYTES],
}

impl BiomeName {
    const UNUSED: BiomeName = BiomeName {
        len: 0,
        bytes: [0; MAX_BIOME_NAME_BYTES],
    };

    pub fn new(name: &str) -> Result<Self, ParamError> {
        if name.is_empty() || name.len() > MAX_BIOME_NAME_BYTES {
            return Err(ParamError {
                field: "biomes.name",
                rule: "must be 1 to 32 bytes",
            });
        }
        let mut bytes = [0; MAX_BIOME_NAME_BYTES];
        bytes[..name.len()].copy_from_slice(name.as_bytes());
        Ok(BiomeName {
            len: name.len() as u8,
            bytes,
        })
    }

    pub fn as_str(&self) -> &str {
        // Built only from a `&str` and cut at its whole length.
        std::str::from_utf8(&self.bytes[..self.len as usize]).unwrap_or_default()
    }
}

impl std::fmt::Debug for BiomeName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self.as_str(), f)
    }
}

impl std::fmt::Display for BiomeName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl serde::Serialize for BiomeName {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for BiomeName {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let name = String::deserialize(d)?;
        BiomeName::new(&name).map_err(serde::de::Error::custom)
    }
}

/// A point in the unit square of temperature and humidity; the nearest to a column's Climate wins.
///
/// Not serde: ground is held by Material id, which only means something beside the tables that
/// made it, so the save writes names (`save::SavedBiome`).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Biome {
    pub name: BiomeName,
    pub temperature: f32,
    pub humidity: f32,
    /// `None` takes the World's own ground.
    pub ground: Option<GroundParams>,
    /// `None` has no say in the sky and fog.
    pub sky_fog: Option<SkyFog>,
    /// Multiplies the World's cave radius: `None` leaves the caves alone, `Some(0.0)` removes them.
    pub cave_scale: Option<f32>,
    pub surface_features: SurfaceFeatures,
}

impl Biome {
    const UNUSED: Biome = Biome {
        name: BiomeName::UNUSED,
        temperature: 0.0,
        humidity: 0.0,
        ground: None,
        sky_fog: None,
        cave_scale: None,
        surface_features: SurfaceFeatures::NONE,
    };
}

/// A Material standing one Voxel above a Biome's surface, about one in every `spacing_m` by
/// `spacing_m` metres of it.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SurfaceFeature {
    pub material: MaterialId,
    pub spacing_m: f32,
}

/// At most [`MAX_SURFACE_FEATURES`], in declaration order.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SurfaceFeatures {
    len: u8,
    features: [SurfaceFeature; MAX_SURFACE_FEATURES],
}

impl Default for SurfaceFeatures {
    fn default() -> Self {
        Self::NONE
    }
}

impl SurfaceFeatures {
    pub const NONE: SurfaceFeatures = SurfaceFeatures {
        len: 0,
        features: [SurfaceFeature {
            material: AIR,
            spacing_m: 0.0,
        }; MAX_SURFACE_FEATURES],
    };

    pub fn new(features: &[SurfaceFeature]) -> Result<Self, ParamError> {
        if features.len() > MAX_SURFACE_FEATURES {
            return Err(ParamError {
                field: "biomes.surface_features",
                rule: "holds at most 4 features",
            });
        }
        if features.iter().any(|f| f.material == AIR) {
            return Err(ParamError {
                field: "biomes.surface_features.material",
                rule: "must not be Air",
            });
        }
        if !features
            .iter()
            .all(|f| SURFACE_FEATURE_SPACING_M.contains(&f.spacing_m))
        {
            return Err(ParamError {
                field: "biomes.surface_features.spacing_m",
                rule: "must be from 4 to 1024 m",
            });
        }
        let mut table = Self::NONE;
        table.features[..features.len()].copy_from_slice(features);
        table.len = features.len() as u8;
        Ok(table)
    }

    pub fn as_slice(&self) -> &[SurfaceFeature] {
        &self.features[..self.len as usize]
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// sRGB bytes. A pair, so a Biome carries both or neither.
///
/// `fog` is what the shader fogs toward and the tonemapper darkens, so it is a little lighter than
/// `sky` (`render::fog::HAZE_FOG_SRGB`).
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkyFog {
    pub sky: [u8; 3],
    pub fog: [u8; 3],
}

/// sRGB in `[0, 1]`, as the Client's fog settings hold it.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SkyFogBlend {
    pub sky: [f32; 3],
    pub fog: [f32; 3],
}

/// Wide enough that a walk never sees a step, narrow enough that a region's middle is its own.
const SKY_BLEND_WIDTH: f32 = 0.1;

/// At most [`MAX_BIOMES`], distinctly named, points inside the unit square. Empty generates exactly
/// as before Biomes existed. Order is identity: [`BiomeId`] is a position, and a tie goes to the
/// earlier entry.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct BiomeTable {
    len: u8,
    biomes: [Biome; MAX_BIOMES],
}

impl Default for BiomeTable {
    fn default() -> Self {
        BiomeTable {
            len: 0,
            biomes: [Biome::UNUSED; MAX_BIOMES],
        }
    }
}

impl BiomeTable {
    pub fn new(biomes: &[Biome]) -> Result<Self, ParamError> {
        let refuse = |rule| ParamError {
            field: "biomes",
            rule,
        };
        if biomes.len() > MAX_BIOMES {
            return Err(refuse("holds at most 16 Biomes"));
        }
        let in_square = |v: f32| (0.0..=1.0).contains(&v);
        if !biomes
            .iter()
            .all(|b| in_square(b.temperature) && in_square(b.humidity))
        {
            return Err(refuse("must place every point from 0 to 1"));
        }
        if biomes
            .iter()
            .enumerate()
            .any(|(i, b)| biomes[..i].iter().any(|earlier| earlier.name == b.name))
        {
            return Err(refuse("must give every Biome its own name"));
        }
        if !biomes.iter().all(|b| {
            b.cave_scale
                .is_none_or(|s| (0.0..=MAX_CAVE_SCALE).contains(&s))
        }) {
            return Err(ParamError {
                field: "biomes.cave_scale",
                rule: "must be from 0 to 4",
            });
        }
        let mut table = BiomeTable::default();
        table.biomes[..biomes.len()].copy_from_slice(biomes);
        table.len = biomes.len() as u8;
        Ok(table)
    }

    pub fn as_slice(&self) -> &[Biome] {
        &self.biomes[..self.len as usize]
    }

    pub fn len(&self) -> usize {
        self.len as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn get(&self, id: BiomeId) -> Option<&Biome> {
        self.as_slice().get(id.index())
    }
}
