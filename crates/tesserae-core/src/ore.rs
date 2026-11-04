//! The ore record a World declares. Plain `Copy` data like the rest of
//! [`crate::WorldgenParams`]; every distance is in **metres**.

use crate::climate::{BiomeId, MAX_BIOMES};
use crate::voxel::{AIR, MaterialId};
use crate::worldgen::{MAX_HEIGHT_M, ParamError};

/// The most ores a World declares. A fixed bound keeps [`OreTable`] inline so
/// [`crate::WorldgenParams`] stays `Copy`.
pub const MAX_ORES: usize = 32;

// `BiomeSet` is one bit per `BiomeId`.
const _: () = assert!(MAX_BIOMES <= u16::BITS as usize);

/// The Biomes an ore appears in. Empty means every Biome, and a World with
/// none, so an ore is only ever narrowed by naming Biomes.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct BiomeSet(u16);

impl BiomeSet {
    pub fn new(ids: &[BiomeId]) -> Result<Self, ParamError> {
        let mut bits = 0u16;
        for id in ids {
            if id.index() >= MAX_BIOMES {
                return Err(ParamError {
                    field: "ores.biomes",
                    rule: "must name Biomes below 16",
                });
            }
            bits |= 1 << id.index();
        }
        Ok(BiomeSet(bits))
    }

    pub fn is_unrestricted(&self) -> bool {
        self.0 == 0
    }

    pub fn contains(&self, id: BiomeId) -> bool {
        self.is_unrestricted() || (id.index() < MAX_BIOMES && self.0 >> id.index() & 1 == 1)
    }

    /// The Biomes named, lowest first. Nothing for an unrestricted set.
    pub fn ids(&self) -> impl Iterator<Item = BiomeId> + '_ {
        (0..MAX_BIOMES as u8)
            .filter(|&i| self.0 >> i & 1 == 1)
            .map(BiomeId::new)
    }
}

/// A vein of one Material in the rock. Veins replace rock only, never soil,
/// surface, water or air.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Ore {
    pub material: MaterialId,
    /// The band the vein centres lie in, in metres below the surface.
    pub min_depth_m: f32,
    pub max_depth_m: f32,
    /// The edge of a vein's noise cell, in metres.
    pub vein_size_m: f32,
    /// The fraction of veins that are real, in `(0, 1]`: 1 fills every cell.
    pub rarity: f32,
    pub biomes: BiomeSet,
}

impl Ore {
    pub fn validate(&self) -> Result<(), ParamError> {
        let refuse = |field, rule| ParamError { field, rule };
        if self.material == AIR {
            return Err(refuse("ores.material", "must not be Air"));
        }
        let band = 0.0..=MAX_HEIGHT_M;
        if !(band.contains(&self.min_depth_m)
            && band.contains(&self.max_depth_m)
            && self.min_depth_m <= self.max_depth_m)
        {
            return Err(refuse(
                "ores.depth",
                "must be a band from 0 to 65536 m, shallow end first",
            ));
        }
        if !(self.vein_size_m > 0.0 && (1.0 / self.vein_size_m).is_finite()) {
            return Err(refuse(
                "ores.vein_size_m",
                "must be finite and greater than zero",
            ));
        }
        if !(self.rarity > 0.0 && self.rarity <= 1.0) {
            return Err(refuse(
                "ores.rarity",
                "must be greater than 0 and at most 1",
            ));
        }
        Ok(())
    }
}

/// A World's ores in declaration order, at most [`MAX_ORES`], each valid.
/// Empty means no ores. Built only through [`OreTable::new`], so a table in
/// hand is a valid one.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct OreTable {
    len: u8,
    ores: [Ore; MAX_ORES],
}

impl Default for OreTable {
    fn default() -> Self {
        OreTable {
            len: 0,
            ores: [Ore {
                material: AIR,
                min_depth_m: 0.0,
                max_depth_m: 0.0,
                vein_size_m: 1.0,
                rarity: 1.0,
                biomes: BiomeSet::default(),
            }; MAX_ORES],
        }
    }
}

impl OreTable {
    pub fn new(ores: &[Ore]) -> Result<Self, ParamError> {
        if ores.len() > MAX_ORES {
            return Err(ParamError {
                field: "ores",
                rule: "holds at most 32 ores",
            });
        }
        ores.iter().try_for_each(Ore::validate)?;
        let mut table = OreTable::default();
        table.ores[..ores.len()].copy_from_slice(ores);
        table.len = ores.len() as u8;
        Ok(table)
    }

    pub fn as_slice(&self) -> &[Ore] {
        &self.ores[..self.len as usize]
    }

    pub fn len(&self) -> usize {
        self.len as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}
