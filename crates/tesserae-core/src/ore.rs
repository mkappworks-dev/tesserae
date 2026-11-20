//! The ore record a World declares; every distance is in **metres**.

use crate::climate::{BiomeId, MAX_BIOMES};
use crate::noise::lattice3;
use crate::voxel::{AIR, MaterialId};
use crate::worldgen::{MAX_HEIGHT_M, ParamError};

/// Bounded so [`OreTable`] stays inline and [`crate::WorldgenParams`] stays `Copy`.
pub const MAX_ORES: usize = 32;

// `BiomeSet` is one bit per `BiomeId`.
const _: () = assert!(MAX_BIOMES <= u16::BITS as usize);

/// Empty means every Biome, and a World with none: an ore is only narrowed by naming Biomes.
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

    pub fn ids(&self) -> impl Iterator<Item = BiomeId> + '_ {
        (0..MAX_BIOMES as u8)
            .filter(|&i| self.0 >> i & 1 == 1)
            .map(BiomeId::new)
    }
}

/// Veins replace rock only, never soil, surface, water or air.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Ore {
    pub material: MaterialId,
    /// Metres below the surface, ends included.
    pub min_depth_m: f32,
    pub max_depth_m: f32,
    /// Edge of a vein's noise cell: a vein is a blob inside it, so never wider.
    pub vein_size_m: f32,
    /// Fraction of cells that hold a vein, in `(0, 1]`.
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

/// In declaration order, at most [`MAX_ORES`]. Empty means no ores.
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

/// Salts that turn one cell into the independent numbers a vein is made of.
const REAL: u32 = 0;
const RADIUS: u32 = 1;
const CENTRE: u32 = 2;

/// A pure function of the seed and a position in **metres**, so a vein is the same at every Voxel
/// size and LOD level. A cube holds its sphere wholly inside it, so a Voxel asks one cube and no
/// neighbour.
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

    pub(crate) fn reaching(&self, lo_m: f32, hi_m: f32) -> OreMask {
        let mut mask = 0;
        for (i, ore) in self.ores.as_slice().iter().enumerate() {
            if ore.min_depth_m <= hi_m && lo_m <= ore.max_depth_m {
                mask |= 1 << i;
            }
        }
        OreMask(mask)
    }

    /// `biome` is asked only when a restricted vein is there: a Biome is dearer than a vein.
    pub(crate) fn at(
        &self,
        cells: &mut OreCells,
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
                || !self.vein(&mut cells.0[i], i, ore.rarity, [x_m, y_m, z_m])
            {
                continue;
            }
            if ore.biomes.is_unrestricted() || biome().is_some_and(|b| ore.biomes.contains(b)) {
                return Some(ore.material);
            }
        }
        None
    }

    fn vein(&self, last: &mut Option<VeinCell>, i: usize, rarity: f32, p: [f32; 3]) -> bool {
        let k = self.inv_vein_size[i];
        let scaled = p.map(|m| m * k);
        let cell = scaled.map(|s| s.floor());
        let at = cell.map(|c| c as i32);
        let sphere = match *last {
            Some(VeinCell { at: was, sphere }) if was == at => sphere,
            _ => {
                let sphere = self.sphere(i, rarity, at);
                *last = Some(VeinCell { at, sphere });
                sphere
            }
        };
        let Some(Sphere { radius, centre }) = sphere else {
            return false;
        };
        let mut offset_sq = 0.0;
        for axis in 0..3 {
            let d = scaled[axis] - cell[axis] - centre[axis];
            offset_sq += d * d;
        }
        offset_sq < radius * radius
    }

    /// The vein ore `i` holds in cell `[cx, cy, cz]`, if any, in the cell's own units.
    fn sphere(&self, i: usize, rarity: f32, [cx, cy, cz]: [i32; 3]) -> Option<Sphere> {
        let seed = self.seed.wrapping_add((i as u32).wrapping_mul(0x9e37_79b9));
        let draw = |salt: u32| lattice3(cx, cy, cz, seed.wrapping_add(salt));
        if draw(REAL) >= rarity {
            return None;
        }
        // Placing the centre `radius` in from each face keeps the sphere inside its cube.
        let radius = 0.25 + 0.25 * draw(RADIUS);
        let centre =
            std::array::from_fn(|axis| radius + (1.0 - 2.0 * radius) * draw(CENTRE + axis as u32));
        Some(Sphere { radius, centre })
    }
}

/// The cell each ore last asked about, with its vein. A column walked down stays in a cell for
/// several Voxels, and the cell's draws are most of what a vein costs.
#[derive(Clone, Copy, Default)]
pub(crate) struct OreCells([Option<VeinCell>; MAX_ORES]);

#[derive(Clone, Copy)]
struct VeinCell {
    at: [i32; 3],
    sphere: Option<Sphere>,
}

#[derive(Clone, Copy)]
struct Sphere {
    radius: f32,
    centre: [f32; 3],
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::noise::cell_hash;

    /// Ores of every vein size from a Voxel's width to many, common and rare, some narrowed to a
    /// Biome, with overlapping depth bands.
    fn table(i: u32) -> OreTable {
        let ores: Vec<Ore> = (0..1 + i % 6)
            .map(|k| {
                let h = cell_hash(i as i32, k as i32, 0x0e5e);
                Ore {
                    material: 1 + k as MaterialId,
                    min_depth_m: h[0] * 20.0,
                    max_depth_m: 20.0 + h[1] * 300.0,
                    vein_size_m: [0.5, 2.0, 4.0, 5.0, 13.0][(h[2] * 5.0) as usize],
                    rarity: [0.06, 0.25, 1.0][k as usize % 3],
                    biomes: if k % 4 == 3 {
                        BiomeSet::new(&[BiomeId::new(1)]).unwrap()
                    } else {
                        BiomeSet::default()
                    },
                }
            })
            .collect();
        OreTable::new(&ores).unwrap()
    }

    /// Columns walked bottom-up at every LOD level's Voxel size, as a Chunk walks them, beside and
    /// either side of zero, each answer checked against a cache that has seen nothing.
    #[test]
    fn an_ore_walking_a_column_answers_as_it_does_per_voxel() {
        let mut veins = 0;
        for i in 0..96u32 {
            let field = OreField::new(i.wrapping_mul(0x9e37_79b9), &table(i)).unwrap();
            let mask = field.reaching(0.0, 400.0);
            let biome = || Some(BiomeId::new((i % 3) as u8));
            let mut cells = OreCells::default();
            for column in 0..24u32 {
                let h = cell_hash(i as i32, column as i32, 0x0c01);
                let voxel_m = [0.25, 0.5, 1.0, 2.0, 4.0, 8.0, 16.0, 32.0][(h[0] * 8.0) as usize];
                let [x_m, z_m] =
                    [h[1], h[2]].map(|v| ((v - 0.5) * 2000.0 / voxel_m).floor() * voxel_m);
                let lift = cell_hash(i as i32, column as i32, 0x0c02)[0];
                let floor_m = ((lift - 0.5) * 600.0 / voxel_m).floor() * voxel_m;
                let surface_m = floor_m + 32.0 * voxel_m;
                for y in 0..32 {
                    let y_m = floor_m + y as f32 * voxel_m;
                    let p = [x_m, y_m, z_m];
                    let depth_m = surface_m - y_m;
                    let want = field.at(&mut OreCells::default(), mask, p, depth_m, biome);
                    let got = field.at(&mut cells, mask, p, depth_m, biome);
                    assert_eq!(got, want, "{p:?} seed {i}, {voxel_m} m Voxels");
                    veins += usize::from(got.is_some());
                }
            }
        }
        assert!(veins > 1000, "{veins} Voxels of vein to compare");
    }
}
