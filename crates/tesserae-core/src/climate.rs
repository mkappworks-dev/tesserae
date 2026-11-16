//! Climate and Biome selection: which kind of ground a column is, as a pure
//! function of the seed and a position in **metres**.
//!
//! Nothing here knows a Voxel size, so the answer is the same at every Voxel
//! size and for every LOD level generator
//! (`docs/adr/0003-voxel-size-is-per-world.md`,
//! `docs/adr/0011-coarse-levels-are-derived.md`).

use crate::noise::fbm2;
use crate::voxel::{AIR, MaterialId, SHAPE_FULL_CUBE, ShapeId};
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
pub const MAX_SURFACE_FEATURES: usize = 16;

/// From one in every metre of ground, a carpet of ground cover, to one in a square kilometre.
pub const SURFACE_FEATURE_SPACING_M: std::ops::RangeInclusive<f32> = 1.0..=1024.0;

/// A clump's field varies over this many metres: a thicket's width to a region's.
pub const CLUMP_SCALE_M: std::ops::RangeInclusive<f32> = 4.0..=1024.0;

/// A wild field's regions are this many metres across: a flower bed's width to a meadow's.
pub const FIELD_SCALE_M: std::ops::RangeInclusive<f32> = 8.0..=96.0;

/// The most of a field's ground that ignores its region's species.
pub const MAX_FIELD_STRAYS: f32 = 0.5;

/// Bounded so a [`Biome`] stays inline and `Copy`.
pub const MAX_CROP_FIELDS: usize = 2;

/// Crops one planted field draws a plot's crop from.
pub const MAX_FIELD_CROPS: usize = 4;

/// A plot's side, from a kitchen garden to a farm field.
pub const FIELD_PLOT_M: std::ops::RangeInclusive<f32> = 6.0..=24.0;

/// The mean distance between planted fields, in metres.
pub const FIELD_SPACING_M: std::ops::RangeInclusive<f32> = 32.0..=256.0;

/// Bounded so a [`Biome`] stays inline and `Copy`.
pub const MAX_BIOME_STRUCTURES: usize = 4;

/// The widest lattice a Biome's Structures stand on, in metres.
pub const MAX_STRUCTURE_SPACING_M: f32 = 4096.0;

/// A Template a Biome stands as a Structure: each square lattice cell `spacing_m` on a side holds
/// one with `chance`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct BiomeStructure {
    /// An index into the World's Templates.
    pub template: u16,
    pub spacing_m: f32,
    pub chance: f32,
}

impl BiomeStructure {
    const NONE: BiomeStructure = BiomeStructure {
        template: 0,
        spacing_m: 0.0,
        chance: 0.0,
    };

    /// Checked against the Template it names, since its spacing's floor is that Template's size.
    pub fn validate(&self, template: &crate::template::Template) -> Result<(), ParamError> {
        let [x, _, z] = template.size_m();
        validate_lattice(
            "biomes.structures",
            self.spacing_m,
            self.chance,
            2.0 * x.max(z),
            "spacing must be at least twice the Template's longer side and at most 4096 m",
        )
    }
}

/// The rules a lattice row's `spacing_m` and `chance` share: each lattice cell holds a draw with
/// `chance`, and `spacing_m` is from `floor_m` to [`MAX_STRUCTURE_SPACING_M`].
fn validate_lattice(
    field: &'static str,
    spacing_m: f32,
    chance: f32,
    floor_m: f32,
    spacing_rule: &'static str,
) -> Result<(), ParamError> {
    if !(chance > 0.0 && chance <= 1.0) {
        return Err(ParamError {
            field,
            rule: "chance must be above 0 and at most 1",
        });
    }
    if !(spacing_m >= floor_m && spacing_m <= MAX_STRUCTURE_SPACING_M) {
        return Err(ParamError {
            field,
            rule: spacing_rule,
        });
    }
    Ok(())
}

/// Bounded so a [`Biome`] stays inline and `Copy`.
pub const MAX_BIOME_SETTLEMENTS: usize = 2;

/// A Settlement a Biome lays out: each square lattice cell `spacing_m` on a side holds a candidate
/// centre with `chance`, which stands unless a neighbour's would meet its disc and margin.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct BiomeSettlement {
    /// An index into the World's Settlements.
    pub settlement: u16,
    pub spacing_m: f32,
    pub chance: f32,
    /// The four lattice cells round the origin always draw, win against their neighbours, and
    /// draw their centre clear of the origin's keep-out, so a Player finds one near spawn.
    pub near_spawn: bool,
}

impl BiomeSettlement {
    const NONE: BiomeSettlement = BiomeSettlement {
        settlement: 0,
        spacing_m: 0.0,
        chance: 0.0,
        near_spawn: false,
    };

    /// Checked against the Settlement it names, since its spacing's floor is that one's reach.
    pub fn validate(&self, settlement: &crate::settlement::Settlement) -> Result<(), ParamError> {
        validate_lattice(
            "biomes.settlements",
            self.spacing_m,
            self.chance,
            2.0 * settlement.reach_m(),
            "spacing must be at least twice the Settlement's radius and margin and at most 4096 m",
        )
    }
}

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

    /// The earlier of two equally near Biomes, `None` for an empty table. Under version 3 the Climate is moved by `site.1` (temperature and
    /// humidity) and only Biomes whose selectors admit `site.0` compete, falling back to all of
    /// them where none does, so a column always has a Biome.
    pub(crate) fn biome_at(
        &self,
        biomes: &BiomeTable,
        x_m: f32,
        z_m: f32,
        site: Option<(Site, (f32, f32))>,
    ) -> Option<BiomeId> {
        if biomes.is_empty() {
            return None;
        }
        let mut c = self.warped(x_m, z_m);
        if let Some((_, (dt, dh))) = site {
            c.temperature += dt;
            c.humidity += dh;
        }
        let distance = |b: &Biome| {
            let (dt, dh) = (b.temperature - c.temperature, b.humidity - c.humidity);
            dt * dt + dh * dh
        };
        let pick = |admit: &dyn Fn(usize) -> bool| {
            let mut nearest: Option<(usize, f32)> = None;
            for (i, b) in biomes.as_slice().iter().enumerate() {
                if !admit(i) {
                    continue;
                }
                let d = distance(b);
                if nearest.is_none_or(|(_, best)| d < best) {
                    nearest = Some((i, d));
                }
            }
            nearest
        };
        let selectors = biomes.selectors();
        let chosen = match site {
            // A Biome that names a range and admits the column beats one that names none, however
            // near the others are in Climate; failing both, the nearest of all.
            Some((s, _)) => pick(&|i| selectors[i].is_ranged() && selectors[i].admits(&s))
                .or_else(|| pick(&|i| !selectors[i].is_ranged()))
                .or_else(|| pick(&|_| true)),
            // Before version 3 there is no altitude or coast to ask, so a Biome of the land's
            // shape is not part of the World.
            None => pick(&|i| !selectors[i].is_ranged()).or_else(|| pick(&|_| true)),
        };
        let (i, _) = chosen?;
        Some(BiomeId(i as u8))
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
/// `spacing_m` metres of it. With `plant` set it is that Plant instead, generated from the data the
/// World froze, and `material` is only its first Material.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SurfaceFeature {
    pub material: MaterialId,
    pub spacing_m: f32,
    pub plant: Option<SurfacePlant>,
    /// Absent scatters evenly, as every feature did before groves.
    pub clump: Option<Clump>,
    /// Absent stands wherever it is drawn, as every feature did before fields.
    pub field: Option<Field>,
}

/// Groups features of one Biome so a region of the ground holds mostly one of them, as a patch of
/// flowers is mostly one species.
///
/// The ground is cut into regions by the nearest of a jittered lattice `scale_m` apart, a pure
/// function of the seed and a position in metres, so it holds across Chunk borders and at every
/// Voxel size. Each region is given one member of the group by hash; a column that draws a member
/// stands it only if it is the region's, or, with chance `strays`, anyway.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Field {
    /// Local to the Biome: features with the same group compete.
    pub group: u8,
    pub scale_m: f32,
    pub strays: f32,
}

impl Field {
    pub fn validate(&self) -> Result<(), ParamError> {
        if !FIELD_SCALE_M.contains(&self.scale_m) {
            return Err(ParamError {
                field: "biomes.surface_features.field.scale_m",
                rule: "must be from 8 to 96 m",
            });
        }
        if !(0.0..=MAX_FIELD_STRAYS).contains(&self.strays) {
            return Err(ParamError {
                field: "biomes.surface_features.field.strays",
                rule: "must be from 0 to 0.5",
            });
        }
        Ok(())
    }
}

/// A low-frequency density field over the ground plane that multiplies a feature's chance, so it
/// stands in patches with bare ground between instead of evenly.
///
/// The field is value noise over a lattice `scale_m` metres apart, a pure function of the seed, the
/// feature's place in its Biome and a position in metres, so it holds across Chunk borders and at
/// every Voxel size.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Clump {
    pub scale_m: f32,
    /// 0 is no clump. 0.6 is groves at up to 1.6 times the chance with thin clearings; 0.95 packs
    /// the feature into patches over a fifth of the ground at up to 5 times the chance, with bare
    /// ground between; 1 leaves nothing between patches at 6.6 times.
    pub strength: f32,
}

/// Where patches start and how much ground they hold, per strength in hundredths.
const CLUMP_STEPS: usize = 100;
/// The share of the ground a full-strength clump leaves bare.
const CLUMP_BARE_AT_FULL: f64 = 0.85;
/// Half the width of the ramp from clearing to patch, in noise units.
const CLUMP_RAMP: f32 = 0.05;

struct ClumpShape {
    /// The noise value a patch starts at.
    cut: [f32; CLUMP_STEPS + 1],
    /// The mean of the patch ramp over the plane, which the ramp is divided by so the factor
    /// averages 1.
    mean: [f32; CLUMP_STEPS + 1],
}

fn ramp(n: f32, cut: f32) -> f32 {
    let t = ((n - (cut - CLUMP_RAMP)) / (2.0 * CLUMP_RAMP)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Value noise is not uniform, so the cut that leaves a share of the ground bare is read off the
/// noise's own distribution, measured once over a few thousand lattice cells.
fn clump_shape() -> &'static ClumpShape {
    static SHAPE: std::sync::OnceLock<ClumpShape> = std::sync::OnceLock::new();
    SHAPE.get_or_init(|| {
        const CELLS: usize = 192;
        const PER_CELL: usize = 4;
        let side = CELLS * PER_CELL;
        let mut values: Vec<f32> = (0..side * side)
            .map(|i| {
                let (x, z) = (i % side, i / side);
                let at = |c: usize| (c as f32 + 0.5) / PER_CELL as f32;
                crate::noise::noise2(at(x), at(z), 0x5eed_c1a9)
            })
            .collect();
        values.sort_by(f32::total_cmp);
        let mut shape = ClumpShape {
            cut: [0.0; CLUMP_STEPS + 1],
            mean: [1.0; CLUMP_STEPS + 1],
        };
        for step in 0..=CLUMP_STEPS {
            let bare = CLUMP_BARE_AT_FULL * step as f64 / CLUMP_STEPS as f64;
            let cut = values[((bare * (values.len() - 1) as f64) as usize).min(values.len() - 1)];
            let first = values.partition_point(|&v| v < cut - CLUMP_RAMP);
            let last = values.partition_point(|&v| v < cut + CLUMP_RAMP);
            let sum: f64 = values[first..last]
                .iter()
                .map(|&v| f64::from(ramp(v, cut)))
                .sum::<f64>()
                + (values.len() - last) as f64;
            shape.cut[step] = cut;
            shape.mean[step] = (sum / values.len() as f64) as f32;
        }
        shape
    })
}

impl Clump {
    /// `(cut, mean)` at this strength, interpolated between hundredths.
    fn shape(&self) -> (f32, f32) {
        let shape = clump_shape();
        let at = self.strength.clamp(0.0, 1.0) * CLUMP_STEPS as f32;
        let low = (at.floor() as usize).min(CLUMP_STEPS - 1);
        let t = at - low as f32;
        let lerp = |v: &[f32; CLUMP_STEPS + 1]| v[low] + (v[low + 1] - v[low]) * t;
        (lerp(&shape.cut), lerp(&shape.mean))
    }

    /// The factor on a column's chance, from `1 - strength` in a clearing to [`Self::max_factor`] in
    /// a patch, with a mean of 1 over the plane, so the declared spacing stays the mean spacing.
    ///
    /// A patch is where the noise passes a cut chosen to leave `0.85 * strength` of the ground bare,
    /// ramped over a short distance; its factor is divided by the ramp's mean over the plane. A
    /// strength below 1 lets `1 - strength` of the chance fall evenly on the clearings.
    pub fn factor(&self, seed: u32, x_m: f32, z_m: f32) -> f64 {
        let n = crate::noise::noise2(x_m / self.scale_m, z_m / self.scale_m, seed);
        let (cut, mean) = self.shape();
        let strength = f64::from(self.strength);
        1.0 - strength + strength * f64::from(ramp(n, cut) / mean)
    }

    /// The most [`Self::factor`] reaches.
    pub fn max_factor(&self) -> f64 {
        let (_, mean) = self.shape();
        let strength = f64::from(self.strength);
        1.0 - strength + strength / f64::from(mean)
    }

    pub fn validate(&self) -> Result<(), ParamError> {
        if !CLUMP_SCALE_M.contains(&self.scale_m) {
            return Err(ParamError {
                field: "biomes.surface_features.clump.scale_m",
                rule: "must be from 4 to 1024 m",
            });
        }
        if !(0.0..=1.0).contains(&self.strength) {
            return Err(ParamError {
                field: "biomes.surface_features.clump.strength",
                rule: "must be from 0 to 1",
            });
        }
        Ok(())
    }
}

impl SurfaceFeature {
    pub const fn cube(material: MaterialId, spacing_m: f32) -> Self {
        SurfaceFeature {
            material,
            spacing_m,
            plant: None,
            clump: None,
            field: None,
        }
    }
}

/// Most Stages a generated crop draws from.
pub const MAX_SURFACE_STAGES: usize = 8;
/// Most fruit Stages a generated tree draws from.
pub const MAX_SURFACE_FRUIT_STAGES: usize = 4;

/// What generation needs of one Plant, owned by the Biome so [`crate::WorldgenParams`] stays `Copy`
/// and a Game that later changes the Plant cannot reshape a saved World.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SurfacePlant {
    /// The Shape every generated Voxel but a tree's takes, as a Player's Place of the seed would
    /// have chosen it.
    pub shape: ShapeId,
    pub kind: SurfacePlantKind,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum SurfacePlantKind {
    /// A crop's Stages in order, or the one Material of a spreader. One Voxel at a hash-drawn Stage.
    Stages {
        materials: [MaterialId; MAX_SURFACE_STAGES],
        len: u8,
        /// The Shape of each Stage that declares its own, over the Plant's.
        stage_shapes: [Option<ShapeId>; MAX_SURFACE_STAGES],
    },
    /// A run standing on the ground, or in the lake when `stands_in` names its fluid.
    Column {
        material: MaterialId,
        max_height_m: f32,
        stands_in: Option<MaterialId>,
    },
    Tree(SurfaceTree),
    /// A vine crop standing ripe, with its first fruit on the ground round it.
    Vine(SurfaceVine),
}

/// A vine crop and the fruit it bears, frozen with the Biome like a tree's.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SurfaceVine {
    /// The ripe vine's Material, in the Plant's Shape.
    pub material: MaterialId,
    pub fruit: [MaterialId; MAX_SURFACE_FRUIT_STAGES],
    pub len: u8,
    /// The Shape of every fruit Voxel; the cube is `SHAPE_FULL_CUBE`.
    pub fruit_shape: ShapeId,
    pub stage_shapes: [Option<ShapeId>; MAX_SURFACE_FRUIT_STAGES],
    pub reach_m: f32,
    pub gap_m: f32,
    pub max_fruit: u32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SurfaceTree {
    pub trunk: MaterialId,
    pub canopy: MaterialId,
    pub trunk_height_m: (f32, f32),
    pub canopy_radius_m: f32,
    pub fruit: Option<SurfaceFruit>,
    pub vines: Option<SurfaceVines>,
    /// Frozen with the tree, like the vines, so a Game that later edits its row cannot reshape
    /// a saved World.
    pub branching: Option<crate::plant::Branching>,
}

impl SurfaceTree {
    /// How far the tree reaches from its column, in metres.
    pub fn reach_m(&self) -> f32 {
        self.branching
            .map_or(self.canopy_radius_m, |b| b.extent_m(self.canopy_radius_m))
    }

    /// Voxels a branched tree's cells can stand past its extent in metres: the lean, spread, limb
    /// and lobe each round on their own.
    pub fn rounding_slack_voxels(&self) -> i32 {
        if self.branching.is_some() { 4 } else { 0 }
    }

    pub fn rounding_slack_m(&self, voxel_size: f32) -> f32 {
        self.rounding_slack_voxels() as f32 * voxel_size
    }
}

/// The vines worldgen hangs from a tree's outer canopy, frozen with the climber they are made of
/// so a Game that later changes it cannot reshape a saved World.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SurfaceVines {
    pub material: MaterialId,
    /// The climber's `plane` Shape.
    pub shape: ShapeId,
    /// The Orientation that lays the plane against each horizontal face, in the order of
    /// [`VINE_FACES`]; resolved when the World is made because worldgen holds no Shape table.
    pub orientations: [u8; 4],
    /// Share of the outer canopy cells that hang a vine, from 0 to 0.5.
    pub density: f32,
    /// The longest a vine hangs, above 0 and at most 8.
    pub length_m: f32,
}

/// Most vines per outer canopy cell, and longest hang in metres.
pub const MAX_VINE_DENSITY: f32 = 0.5;
pub const MAX_VINE_LENGTH_M: f32 = 8.0;

/// The faces a vine can lie against, as [`crate::mesh::FACE_STEPS`] indices: the four sides.
pub const VINE_FACES: [usize; 4] = [0, 1, 4, 5];

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SurfaceFruit {
    pub stages: [MaterialId; MAX_SURFACE_FRUIT_STAGES],
    pub len: u8,
    pub density: f32,
    /// The Shape of every fruit Voxel; the cube is `SHAPE_FULL_CUBE`.
    pub shape: ShapeId,
    /// The Shape of each Stage that declares its own, over `shape`.
    pub stage_shapes: [Option<ShapeId>; MAX_SURFACE_FRUIT_STAGES],
}

impl SurfacePlant {
    /// Every Material it generates, first one first.
    pub fn materials(&self) -> Vec<MaterialId> {
        match self.kind {
            SurfacePlantKind::Stages { materials, len, .. } => materials[..len as usize].to_vec(),
            SurfacePlantKind::Column {
                material,
                stands_in,
                ..
            } => [Some(material), stands_in].into_iter().flatten().collect(),
            SurfacePlantKind::Vine(v) => {
                let mut all = vec![v.material];
                all.extend_from_slice(&v.fruit[..v.len as usize]);
                all
            }
            SurfacePlantKind::Tree(t) => {
                let mut all = vec![t.trunk, t.canopy];
                if let Some(f) = t.fruit {
                    all.extend_from_slice(&f.stages[..f.len as usize]);
                }
                all.extend(t.vines.map(|v| v.material));
                all
            }
        }
    }

    /// How far above its base it can stand, in metres, before rounding to Voxels.
    pub fn rise_m(&self) -> f32 {
        match self.kind {
            SurfacePlantKind::Stages { .. } => 0.0,
            SurfacePlantKind::Column { max_height_m, .. } => max_height_m,
            SurfacePlantKind::Vine(_) => 0.0,
            SurfacePlantKind::Tree(t) => {
                t.trunk_height_m.1
                    + t.canopy_radius_m
                        .max(t.branching.map_or(0.0, |b| b.lobe_radius_m))
            }
        }
    }

    fn validate(&self) -> Result<(), ParamError> {
        let bad = |rule| ParamError {
            field: "biomes.surface_features.plant",
            rule,
        };
        let finite_pos = |v: f32| v.is_finite() && v > 0.0;
        match self.kind {
            SurfacePlantKind::Stages { materials, len, .. } => {
                if !(1..=MAX_SURFACE_STAGES as u8).contains(&len)
                    || materials[..len as usize].contains(&AIR)
                {
                    return Err(bad("must hold 1 to 8 Stages, none Air"));
                }
            }
            SurfacePlantKind::Column {
                material,
                max_height_m,
                stands_in,
            } => {
                if material == AIR || stands_in == Some(AIR) || !finite_pos(max_height_m) {
                    return Err(bad("a column needs a Material and a height"));
                }
            }
            SurfacePlantKind::Vine(v) => {
                let fruit = &v.fruit[..(v.len as usize).min(MAX_SURFACE_FRUIT_STAGES)];
                if v.material == AIR
                    || !(1..=MAX_SURFACE_FRUIT_STAGES as u8).contains(&v.len)
                    || fruit.contains(&AIR)
                    || !(v.reach_m.is_finite() && v.reach_m > 0.0 && v.reach_m <= 8.0)
                    || !(v.gap_m.is_finite() && (0.0..=v.reach_m).contains(&v.gap_m))
                    || !(1..=crate::plant::MAX_VINE_FRUIT).contains(&v.max_fruit)
                {
                    return Err(bad(
                        "a vine needs a Material, 1 to 4 fruit Stages, a reach to 8 m, a gap within it and 1 to 16 fruit",
                    ));
                }
            }
            SurfacePlantKind::Tree(t) => {
                let (low, high) = t.trunk_height_m;
                let fruit = t.fruit.is_none_or(|f| {
                    (1..=MAX_SURFACE_FRUIT_STAGES as u8).contains(&f.len)
                        && (0.0..=1.0).contains(&f.density)
                });
                let vines = t.vines.is_none_or(|v| {
                    v.material != AIR
                        && v.orientations.iter().all(|&o| o < 24)
                        && (0.0..=MAX_VINE_DENSITY).contains(&v.density)
                        && v.length_m > 0.0
                        && v.length_m <= MAX_VINE_LENGTH_M
                });
                if !vines {
                    return Err(bad(
                        "vines need a climber, a density to 0.5 and a length to 8 m",
                    ));
                }
                if let Some(b) = t.branching
                    && b.check(t.canopy_radius_m, low).is_err()
                {
                    return Err(bad("a tree's branching must be one the generator can draw"));
                }
                if t.trunk == AIR
                    || t.canopy == AIR
                    || !finite_pos(low)
                    || !matches!(
                        low.partial_cmp(&high),
                        Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
                    )
                    || !finite_pos(t.canopy_radius_m)
                    || !fruit
                {
                    return Err(bad("a tree needs a trunk, canopy, height and radius"));
                }
            }
        }
        Ok(())
    }
}

/// A crop a planted field draws a plot's crop from, frozen like a Plant feature's.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct FieldCrop {
    /// The Shape every Stage takes that declares none, as a Player's Place of the seed would have
    /// chosen it.
    pub shape: ShapeId,
    pub materials: [MaterialId; MAX_SURFACE_STAGES],
    /// Stages in order; the last is ripe.
    pub len: u8,
    pub stage_shapes: [Option<ShapeId>; MAX_SURFACE_STAGES],
    pub weight: f32,
}

impl FieldCrop {
    pub const NONE: FieldCrop = FieldCrop {
        shape: SHAPE_FULL_CUBE,
        materials: [AIR; MAX_SURFACE_STAGES],
        len: 0,
        stage_shapes: [None; MAX_SURFACE_STAGES],
        weight: 0.0,
    };

    /// A crop of the Stages `materials`, in order, each in the Shape `stage_shapes` gives it or
    /// else `shape`.
    pub fn new(
        shape: ShapeId,
        materials: &[MaterialId],
        stage_shapes: &[Option<ShapeId>],
        weight: f32,
    ) -> Result<Self, ParamError> {
        if materials.len() > MAX_SURFACE_STAGES || stage_shapes.len() > MAX_SURFACE_STAGES {
            return Err(ParamError {
                field: "biomes.crop_fields.crops",
                rule: "holds at most 8 Stages",
            });
        }
        let mut crop = FieldCrop {
            shape,
            len: materials.len() as u8,
            weight,
            ..FieldCrop::NONE
        };
        crop.materials[..materials.len()].copy_from_slice(materials);
        crop.stage_shapes[..stage_shapes.len()].copy_from_slice(stage_shapes);
        Ok(crop)
    }

    /// The Voxel of Stage `stage`.
    pub fn voxel(&self, stage: usize) -> crate::Voxel {
        let shape = self.stage_shapes[stage].unwrap_or(self.shape);
        crate::Voxel::new(self.materials[stage], shape, 0)
    }

    fn stages(&self) -> &[MaterialId] {
        &self.materials[..(self.len as usize).min(MAX_SURFACE_STAGES)]
    }
}

/// Rectangular plots of one crop in rows, with paths between them and a ditch along the low edge,
/// scattered over a Biome as a pure function of the seed and a position in metres.
///
/// A lattice cell `spacing_m` on a side draws whether it holds a field; a field is a small grid of
/// plots `plot_m` on a side, each one crop planted every second row, set apart by lanes one Voxel
/// wide. It stands only on level ground that is wholly in its Biome, clear of the lake, caves and
/// trees.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CropField {
    pub crops: [FieldCrop; MAX_FIELD_CROPS],
    pub len: u8,
    /// A plot's side is drawn from this range, in metres.
    pub plot_m: (f32, f32),
    /// The mean distance between fields, in metres.
    pub spacing_m: f32,
    /// A solid Material the lanes are laid in; none leaves the ground bare.
    pub path: Option<MaterialId>,
    /// A fluid the lane along the field's low edge is filled with; none cuts no ditch.
    pub ditch: Option<MaterialId>,
    /// The field stands only within this distance of the lake; none is anywhere.
    pub near_water_m: Option<f32>,
}

impl CropField {
    const NONE: CropField = CropField {
        crops: [FieldCrop::NONE; MAX_FIELD_CROPS],
        len: 0,
        plot_m: (0.0, 0.0),
        spacing_m: 0.0,
        path: None,
        ditch: None,
        near_water_m: None,
    };

    /// An empty field to add crops to with [`Self::push`]; checked by [`SurfaceFeatures::with_crop_fields`].
    pub fn new(
        plot_m: (f32, f32),
        spacing_m: f32,
        path: Option<MaterialId>,
        ditch: Option<MaterialId>,
        near_water_m: Option<f32>,
    ) -> Self {
        CropField {
            plot_m,
            spacing_m,
            path,
            ditch,
            near_water_m,
            ..CropField::NONE
        }
    }

    pub fn push(&mut self, crop: FieldCrop) -> Result<(), ParamError> {
        if self.len as usize == MAX_FIELD_CROPS {
            return Err(ParamError {
                field: "biomes.crop_fields",
                rule: "needs 1 to 4 crops",
            });
        }
        self.crops[self.len as usize] = crop;
        self.len += 1;
        Ok(())
    }

    pub fn crops(&self) -> &[FieldCrop] {
        &self.crops[..(self.len as usize).min(MAX_FIELD_CROPS)]
    }

    /// Every Material the field puts in the World.
    pub fn materials(&self) -> Vec<MaterialId> {
        let mut all: Vec<MaterialId> = self
            .crops()
            .iter()
            .flat_map(|c| c.stages())
            .copied()
            .collect();
        all.extend(self.path);
        all.extend(self.ditch);
        all
    }

    fn validate(&self) -> Result<(), ParamError> {
        let bad = |rule| ParamError {
            field: "biomes.crop_fields",
            rule,
        };
        if !(1..=MAX_FIELD_CROPS as u8).contains(&self.len) {
            return Err(bad("needs 1 to 4 crops"));
        }
        for crop in self.crops() {
            if !(2..=MAX_SURFACE_STAGES as u8).contains(&crop.len)
                || crop.stages().contains(&AIR)
                || !(crop.weight.is_finite() && crop.weight > 0.0)
            {
                return Err(bad("a crop needs 2 to 8 Stages and a weight above 0"));
            }
        }
        let (low, high) = self.plot_m;
        if !(FIELD_PLOT_M.contains(&low) && FIELD_PLOT_M.contains(&high) && low <= high) {
            return Err(bad("a plot side must be from 6 to 24 m, low to high"));
        }
        if !FIELD_SPACING_M.contains(&self.spacing_m) {
            return Err(bad("spacing must be from 32 to 256 m"));
        }
        if self.path == Some(AIR) || self.ditch == Some(AIR) {
            return Err(bad("a path or ditch must not be Air"));
        }
        if self
            .near_water_m
            .is_some_and(|m| !(m.is_finite() && m > 0.0 && m <= self.spacing_m))
        {
            return Err(bad("near water must be above 0 and at most the spacing"));
        }
        Ok(())
    }
}

/// At most [`MAX_SURFACE_FEATURES`], in declaration order, and at most [`MAX_CROP_FIELDS`] planted
/// fields.
///
/// Held inline so a Biome stays `Copy`: 16 features and 2 fields measure a Biome at about 2.7 KiB,
/// and generation reads them by reference, so the size is paid only when a table is copied, never
/// per column.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SurfaceFeatures {
    len: u8,
    features: [SurfaceFeature; MAX_SURFACE_FEATURES],
    crop_len: u8,
    crop_fields: [CropField; MAX_CROP_FIELDS],
    structure_len: u8,
    structures: [BiomeStructure; MAX_BIOME_STRUCTURES],
    settlement_len: u8,
    settlements: [BiomeSettlement; MAX_BIOME_SETTLEMENTS],
}

impl Default for SurfaceFeatures {
    fn default() -> Self {
        Self::NONE
    }
}

impl SurfaceFeatures {
    pub const NONE: SurfaceFeatures = SurfaceFeatures {
        len: 0,
        features: [SurfaceFeature::cube(AIR, 0.0); MAX_SURFACE_FEATURES],
        crop_len: 0,
        crop_fields: [CropField::NONE; MAX_CROP_FIELDS],
        structure_len: 0,
        structures: [BiomeStructure::NONE; MAX_BIOME_STRUCTURES],
        settlement_len: 0,
        settlements: [BiomeSettlement::NONE; MAX_BIOME_SETTLEMENTS],
    };

    pub fn new(features: &[SurfaceFeature]) -> Result<Self, ParamError> {
        if features.len() > MAX_SURFACE_FEATURES {
            return Err(ParamError {
                field: "biomes.surface_features",
                rule: "holds at most 16 features",
            });
        }
        if features.iter().any(|f| f.material == AIR) {
            return Err(ParamError {
                field: "biomes.surface_features.material",
                rule: "must not be Air",
            });
        }
        for plant in features.iter().filter_map(|f| f.plant) {
            plant.validate()?;
        }
        for clump in features.iter().filter_map(|f| f.clump) {
            clump.validate()?;
        }
        for field in features.iter().filter_map(|f| f.field) {
            field.validate()?;
            let mismatched = features.iter().filter_map(|f| f.field).any(|g| {
                g.group == field.group && (g.scale_m, g.strays) != (field.scale_m, field.strays)
            });
            if mismatched {
                return Err(ParamError {
                    field: "biomes.surface_features.field",
                    rule: "must give every member of a group the same scale and strays",
                });
            }
        }
        if !features
            .iter()
            .all(|f| SURFACE_FEATURE_SPACING_M.contains(&f.spacing_m))
        {
            return Err(ParamError {
                field: "biomes.surface_features.spacing_m",
                rule: "must be from 1 to 1024 m",
            });
        }
        let mut table = Self::NONE;
        table.features[..features.len()].copy_from_slice(features);
        table.len = features.len() as u8;
        Ok(table)
    }

    /// This table with `fields` as its planted fields.
    pub fn with_crop_fields(mut self, fields: &[CropField]) -> Result<Self, ParamError> {
        if fields.len() > MAX_CROP_FIELDS {
            return Err(ParamError {
                field: "biomes.crop_fields",
                rule: "holds at most 2 fields",
            });
        }
        for field in fields {
            field.validate()?;
        }
        self.crop_fields[..fields.len()].copy_from_slice(fields);
        self.crop_len = fields.len() as u8;
        Ok(self)
    }

    /// This table with `structures` as its Structures, each checked against its Template where
    /// the World's Templates are known.
    pub fn with_structures(mut self, structures: &[BiomeStructure]) -> Result<Self, ParamError> {
        if structures.len() > MAX_BIOME_STRUCTURES {
            return Err(ParamError {
                field: "biomes.structures",
                rule: "holds at most 4 Structures",
            });
        }
        self.structures[..structures.len()].copy_from_slice(structures);
        self.structure_len = structures.len() as u8;
        Ok(self)
    }

    pub fn with_settlements(mut self, settlements: &[BiomeSettlement]) -> Result<Self, ParamError> {
        if settlements.len() > MAX_BIOME_SETTLEMENTS {
            return Err(ParamError {
                field: "biomes.settlements",
                rule: "holds at most 2 Settlements",
            });
        }
        self.settlements[..settlements.len()].copy_from_slice(settlements);
        self.settlement_len = settlements.len() as u8;
        Ok(self)
    }

    pub fn as_slice(&self) -> &[SurfaceFeature] {
        &self.features[..self.len as usize]
    }

    pub fn structures(&self) -> &[BiomeStructure] {
        &self.structures[..self.structure_len as usize]
    }

    pub fn settlements(&self) -> &[BiomeSettlement] {
        &self.settlements[..self.settlement_len as usize]
    }

    pub fn crop_fields(&self) -> &[CropField] {
        &self.crop_fields[..self.crop_len as usize]
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0 && self.crop_len == 0
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
    /// Version 3's selectors, beside the Biomes so a Biome literal needs no new field.
    selectors: [BiomeSelectors; MAX_BIOMES],
}

impl Default for BiomeTable {
    fn default() -> Self {
        BiomeTable {
            len: 0,
            biomes: [Biome::UNUSED; MAX_BIOMES],
            selectors: [BiomeSelectors::default(); MAX_BIOMES],
        }
    }
}

/// Where a Biome may stand beyond its Climate, read by Generator version 3 only. A range left
/// `None` matches every value on that axis, so a Biome declared before version 3 is unchanged.
/// Ranges are inclusive.
#[derive(Clone, Copy, PartialEq, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct BiomeSelectors {
    /// Metres above Sea level; negative is under the sea.
    pub altitude_m: Option<[f32; 2]>,
    /// Metres inland of the coast; negative is offshore.
    pub coast_m: Option<[f32; 2]>,
    /// 0 rugged to 1 worn flat.
    pub erosion: Option<[f32; 2]>,
    /// Multiplies the detail relief of the Biome's landform, from 0 to 4; `None` leaves it.
    pub relief: Option<f32>,
}

/// The land a column stands on, as far as a Biome's selectors ask.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Site {
    pub altitude_m: f32,
    pub coast_m: f32,
    pub erosion: f32,
}

pub const MAX_RELIEF_SCALE: f32 = 4.0;

impl BiomeSelectors {
    pub fn admits(&self, site: &Site) -> bool {
        let within = |r: Option<[f32; 2]>, v: f32| r.is_none_or(|[lo, hi]| (lo..=hi).contains(&v));
        within(self.altitude_m, site.altitude_m)
            && within(self.coast_m, site.coast_m)
            && within(self.erosion, site.erosion)
    }

    /// Whether the Biome names any range, which makes it a Biome of the land's shape: version 3
    /// prefers it where it admits the column, and versions 1 and 2 never choose it.
    pub fn is_ranged(&self) -> bool {
        self.altitude_m.is_some() || self.coast_m.is_some() || self.erosion.is_some()
    }

    pub fn validate(&self) -> Result<(), ParamError> {
        let ordered = |r: Option<[f32; 2]>| {
            r.is_none_or(|[lo, hi]| lo.is_finite() && hi.is_finite() && lo <= hi)
        };
        if !(ordered(self.altitude_m) && ordered(self.coast_m) && ordered(self.erosion)) {
            return Err(ParamError {
                field: "biomes.selectors",
                rule: "must give every range a finite low end not above its high end",
            });
        }
        if !self
            .relief
            .is_none_or(|r| (0.0..=MAX_RELIEF_SCALE).contains(&r))
        {
            return Err(ParamError {
                field: "biomes.selectors.relief",
                rule: "must be from 0 to 4",
            });
        }
        Ok(())
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

    /// This table with `selectors` for its Biomes, in order; fewer than the Biomes leaves the rest
    /// matching everything.
    pub fn with_selectors(mut self, selectors: &[BiomeSelectors]) -> Result<Self, ParamError> {
        if selectors.len() > self.len() {
            return Err(ParamError {
                field: "biomes.selectors",
                rule: "must not name more Biomes than the table holds",
            });
        }
        for s in selectors {
            s.validate()?;
        }
        self.selectors[..selectors.len()].copy_from_slice(selectors);
        Ok(self)
    }

    pub fn selectors(&self) -> &[BiomeSelectors] {
        &self.selectors[..self.len as usize]
    }

    /// Whether any Biome says anything about relief.
    pub fn shapes_relief(&self) -> bool {
        self.selectors().iter().any(|s| s.relief.is_some())
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
