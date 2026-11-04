//! The ore record a World declares. Plain `Copy` data like the rest of
//! [`crate::WorldgenParams`]; every distance is in **metres**.

use crate::climate::{BiomeId, MAX_BIOMES};
use crate::noise::lattice3;
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
    /// The band a Voxel is ore in, in metres below the surface, ends included.
    pub min_depth_m: f32,
    pub max_depth_m: f32,
    /// The edge of a vein's noise cell, in metres. A vein is a blob inside its
    /// cell, so it is never wider than this.
    pub vein_size_m: f32,
    /// The fraction of cells that hold a vein, in `(0, 1]`: 1 puts one in
    /// every cell.
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

/// The ores a Chunk can reach, as a bit per [`OreTable`] slot.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct OreMask(u32);

const _: () = assert!(MAX_ORES <= u32::BITS as usize);

impl OreMask {
    pub(crate) fn is_empty(self) -> bool {
        self.0 == 0
    }
}

/// Far from the terrain, cave and Climate seeds, so veins follow none of them.
const ORE_SEED: u32 = 0x0e0e_0001;

/// The salts that turn one cell into the independent numbers a vein is made
/// of: whether it is real, its radius and the three axes of its centre.
const REAL: u32 = 0;
const RADIUS: u32 = 1;
const CENTRE: u32 = 2;

/// A World's veins: a pure function of the seed and a position in **metres**,
/// so a vein is the same at every Voxel size and for every LOD level. Space is
/// cut into cubes of `vein_size_m`; a cube holds a sphere with probability
/// `rarity`, wholly inside it, so a Voxel asks one cube and no neighbour.
#[derive(Clone, Copy, Debug)]
pub(crate) struct OreField {
    seed: u32,
    ores: OreTable,
    inv_vein_size: [f32; MAX_ORES],
}

impl OreField {
    /// `None` for no ores, so a World without any pays nothing.
    pub(crate) fn new(seed: u32, ores: &OreTable) -> Option<Self> {
        if ores.is_empty() {
            return None;
        }
        let mut inv_vein_size = [1.0; MAX_ORES];
        for (inv, ore) in inv_vein_size.iter_mut().zip(ores.as_slice()) {
            *inv = 1.0 / ore.vein_size_m;
        }
        Some(OreField {
            seed: seed.wrapping_add(ORE_SEED),
            ores: *ores,
            inv_vein_size,
        })
    }

    /// The ores with a Voxel at some depth in `lo_m..=hi_m` below the surface.
    pub(crate) fn reaching(&self, lo_m: f32, hi_m: f32) -> OreMask {
        let mut mask = 0;
        for (i, ore) in self.ores.as_slice().iter().enumerate() {
            if ore.min_depth_m <= hi_m && lo_m <= ore.max_depth_m {
                mask |= 1 << i;
            }
        }
        OreMask(mask)
    }

    /// The Material of the first ore in `mask` with a vein at this Voxel, which
    /// is `depth_m` below the surface. `biome` is asked only when a vein is
    /// there and it is restricted, because a Biome is dearer than a vein.
    pub(crate) fn at(
        &self,
        mask: OreMask,
        [x_m, y_m, z_m]: [f32; 3],
        depth_m: f32,
        biome: impl Fn() -> Option<BiomeId>,
    ) -> Option<MaterialId> {
        let mut rest = mask.0;
        while rest != 0 {
            let i = rest.trailing_zeros() as usize;
            rest &= rest - 1;
            let ore = &self.ores.as_slice()[i];
            if depth_m < ore.min_depth_m
                || depth_m > ore.max_depth_m
                || !self.vein(i, ore.rarity, [x_m, y_m, z_m])
            {
                continue;
            }
            if ore.biomes.is_unrestricted() || biome().is_some_and(|b| ore.biomes.contains(b)) {
                return Some(ore.material);
            }
        }
        None
    }

    /// Whether the point is inside the sphere of the cube it is in, working in
    /// units of that cube's edge.
    fn vein(&self, i: usize, rarity: f32, p: [f32; 3]) -> bool {
        let k = self.inv_vein_size[i];
        let scaled = p.map(|m| m * k);
        let cell = scaled.map(|s| s.floor());
        let [cx, cy, cz] = cell.map(|c| c as i32);
        let seed = self.seed.wrapping_add((i as u32).wrapping_mul(0x9e37_79b9));
        let draw = |salt: u32| lattice3(cx, cy, cz, seed.wrapping_add(salt));
        if draw(REAL) >= rarity {
            return false;
        }
        // From a quarter to a half of the cube's edge: the centre is then
        // placed so the sphere stays inside it.
        let radius = 0.25 + 0.25 * draw(RADIUS);
        let mut offset_sq = 0.0;
        for axis in 0..3 {
            let centre = radius + (1.0 - 2.0 * radius) * draw(CENTRE + axis as u32);
            let d = scaled[axis] - cell[axis] - centre;
            offset_sq += d * d;
        }
        offset_sq < radius * radius
    }
}
