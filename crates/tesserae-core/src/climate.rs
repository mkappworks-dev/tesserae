//! Climate and Biome selection: which kind of ground a column is, as a pure
//! function of the seed and a position in **metres**.
//!
//! Nothing here knows a Voxel size, so the answer is the same at every Voxel
//! size and for every LOD level generator
//! (`docs/adr/0003-voxel-size-is-per-world.md`,
//! `docs/adr/0011-coarse-levels-are-derived.md`).

use crate::noise::fbm2;
use crate::worldgen::{GroundParams, ParamError};

/// Octaves of each Climate field. Fixed rather than a parameter: a declaration
/// sizes the regions, and the border warp supplies the finer detail.
const OCTAVES: u32 = 3;

/// Far from the terrain octaves' `seed + 0..octaves` and the cave fields, so
/// Climate is not correlated with the surface.
const TEMPERATURE_SEED: u32 = 0x0c11_a7e0;
const HUMIDITY_SEED: u32 = 0x0c11_a8e0;
const WARP_TEMPERATURE_SEED: u32 = 0x0c11_a9e0;
const WARP_HUMIDITY_SEED: u32 = 0x0c11_aae0;

/// The first Generator version that selects Biomes. Version 1 is frozen as it
/// was before they existed.
pub const BIOMES_SINCE: u32 = 2;

/// The most Biomes a World declares. A fixed bound keeps [`BiomeTable`] inline
/// so [`crate::WorldgenParams`] stays `Copy` and a per-level generator costs no
/// allocation.
pub const MAX_BIOMES: usize = 16;

/// The longest [`BiomeName`], in bytes.
pub const MAX_BIOME_NAME_BYTES: usize = 32;

/// How far, in Climate units, the border between two Biomes is pushed either
/// way. A Biome point further than this from a straight border always wins its
/// side of it, so a Game can place points knowing where the warp stops.
pub const BORDER_WARP: f32 = 0.1;

/// The warp varies faster than the Climate it bends, as a fraction of the
/// Climate's own feature size, or borders would only ever drift smoothly.
const WARP_SIZE_FRACTION: f32 = 0.25;

/// Temperature and humidity at one point, each in `[0, 1)`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Climate {
    pub temperature: f32,
    pub humidity: f32,
}

/// How large the Climate regions are, in metres: the horizontal period of the
/// lowest octave of each field. Plain `Copy` data like the rest of
/// [`crate::WorldgenParams`].
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

/// The two Climate fields, with their frequencies already inverted.
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
    /// The Climate a column is judged by when a Biome is chosen: the real
    /// Climate pushed by a second, faster pair of fields, so a border follows
    /// neither a contour of temperature nor of humidity.
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

    /// The Biome nearest the warped Climate at a point; the earlier of two
    /// equally near. `None` for an empty table.
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
}

/// Which entry of a [`BiomeTable`] a column belongs to. An index and not a
/// name: it is only meaningful beside the table that made it.
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

/// A Biome's name, 1 to [`MAX_BIOME_NAME_BYTES`] bytes of UTF-8, held inline.
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

/// A region type: a point in the unit square of temperature and humidity. The
/// nearest point to a column's Climate wins it. What a Biome owns beyond that
/// (ores, caves, colours) arrives with the tickets that use it.
///
/// Not serde: ground is held by Material id, which only means something beside
/// the tables that made it, so the save writes names (`save::SavedBiome`).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Biome {
    pub name: BiomeName,
    pub temperature: f32,
    pub humidity: f32,
    /// `None` takes the World's own ground.
    pub ground: Option<GroundParams>,
}

impl Biome {
    const UNUSED: Biome = Biome {
        name: BiomeName::UNUSED,
        temperature: 0.0,
        humidity: 0.0,
        ground: None,
    };
}

/// A World's Biomes in declaration order, at most [`MAX_BIOMES`] with distinct
/// names and points inside the unit square. Empty means a World with no Biomes,
/// which generates exactly as it did before they existed. Order is identity:
/// [`BiomeId`] is a position here, and a tie goes to the earlier entry.
///
/// Built only through [`BiomeTable::new`], which checks those rules, so a table
/// in hand is a valid one.
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
