//! Structures: Templates a Biome lists, stood on a lattice of pure draws (ADR-0029).
//!
//! Where a Structure stands, how it is turned and how high is a function of the seed and metres
//! alone, so every Chunk it reaches decides it alike and writes only its own Voxels.
//!
//! Claim order: earlier rows (by Biome, then by row) claim ground first and a later row's draw that
//! would touch one gives way. Whatever is laid after Structures (fields, trees, scatter) asks
//! [`Worldgen::structures_near`] for footprints; a Structure never asks about them.

use std::sync::Arc;

use super::fields::{hashed, mix, unit};
use super::{Worldgen, top_index};
use crate::chunk::{CHUNK_SIZE, Chunk};
use crate::climate::{BIOMES_SINCE, BiomeId, BiomeStructure};
use crate::template::{Cell, Template, Turn};
use crate::voxel::Voxel;
use crate::worldgen::WorldgenParams;

const STRUCTURE_SALT: u64 = 0x5717_c7e0_0000_0001;
/// Places a held lattice cell tries before it stands nothing.
const ATTEMPTS: u64 = 4;
/// Kept clear of a lattice cell's border, so Structures in two cells never touch.
const INSET_M: f32 = 0.5;

/// One Biome's row, as generation at this Voxel size reads it.
#[derive(Clone, Copy, Debug)]
struct Row {
    biome: BiomeId,
    row: u64,
    entry: BiomeStructure,
    /// Voxels along x, y and z, by turn.
    sizes: [[i32; 3]; 4],
    /// The lowest Voxel layer of the Ground layer.
    ground: i32,
}

/// Built only when some Biome lists a Structure whose Template the World has, so a World without
/// one generates exactly as before.
#[derive(Clone, Debug)]
pub(super) struct StructurePass {
    templates: Arc<[Template]>,
    /// In claim order.
    rows: Vec<Row>,
    /// The most Voxels a Structure reaches above and below the ground's top Voxel.
    above: i32,
    below: i32,
}

impl StructurePass {
    pub(super) fn new(
        params: &WorldgenParams,
        voxel_size: f32,
        templates: &[Template],
    ) -> Option<Self> {
        if params.generator < BIOMES_SINCE || params.ground.is_none() {
            return None;
        }
        let mut rows = Vec::new();
        for (b, biome) in params.biomes.as_slice().iter().enumerate() {
            for (row, &entry) in biome.surface_features.structures().iter().enumerate() {
                let Some(template) = templates.get(usize::from(entry.template)) else {
                    continue;
                };
                let sampled = template.sample(voxel_size, Turn::NONE);
                rows.push(Row {
                    biome: BiomeId::new(b as u8),
                    row: row as u64,
                    entry,
                    sizes: [0, 1, 2, 3].map(|q| template.sample(voxel_size, Turn::new(q)).size()),
                    ground: sampled.ground(),
                });
            }
        }
        if rows.is_empty() {
            return None;
        }
        let above = rows.iter().map(|r| r.sizes[0][1] - r.ground).max()?;
        let below = rows.iter().map(|r| r.ground).max()?;
        Some(StructurePass {
            templates: templates.into(),
            rows,
            above,
            below,
        })
    }
}

/// A Structure that stands, in global Voxel indices.
#[derive(Clone, Copy, Debug)]
pub(super) struct Structure {
    row: usize,
    turn: Turn,
    min: [i32; 3],
    size: [i32; 3],
}

impl Structure {
    /// The columns it covers, as `[min, max)` per axis, x then z.
    pub(super) fn footprint(&self) -> [[i32; 2]; 2] {
        [
            [self.min[0], self.min[2]],
            [self.min[0] + self.size[0], self.min[2] + self.size[2]],
        ]
    }

    /// Whether it covers or borders a column of `region`, `[min, max)`.
    fn touches(&self, [min, max]: [[i32; 2]; 2]) -> bool {
        let [own_min, own_max] = self.footprint();
        (0..2).all(|a| own_min[a] <= max[a] && min[a] <= own_max[a])
    }
}

impl Worldgen {
    /// The lattice cells of a row that columns `[min, max)` touch.
    fn structure_cells(&self, entry: &BiomeStructure, [min, max]: [[i32; 2]; 2]) -> Vec<[i32; 2]> {
        let at = |g: i32| (g as f32 * self.voxel_size / entry.spacing_m).floor() as i32;
        let (low, high) = ([0, 1].map(|a| at(min[a])), [0, 1].map(|a| at(max[a] - 1)));
        (low[1]..=high[1])
            .flat_map(|j| (low[0]..=high[0]).map(move |i| [i, j]))
            .collect()
    }

    /// The `attempt`th place a held cell's Structure may stand, before the ground is asked: its
    /// turn, and its corner on whole metres so it stands alike at every Voxel size. `None` where
    /// it would not fit inside the cell's inset.
    fn structure_draw(
        &self,
        index: usize,
        row: &Row,
        cell: [i32; 2],
        held: u64,
        attempt: u64,
    ) -> Option<Structure> {
        let h = mix(held ^ (attempt + 1).wrapping_mul(0x9e37_79b9_7f4a_7c15));
        let turn = Turn::new((mix(h ^ 2) & 3) as u8);
        let template = &self.structures.as_ref()?.templates[usize::from(row.entry.template)];
        let [x_m, _, z_m] = template.size_m();
        let along_m = if turn.quarters().is_multiple_of(2) {
            [x_m, z_m]
        } else {
            [z_m, x_m]
        };
        let size = row.sizes[usize::from(turn.quarters())];
        let mut min = [0; 3];
        for (axis, slot) in [0, 2].into_iter().enumerate() {
            let from = cell[axis] as f32 * row.entry.spacing_m;
            let low = (from + INSET_M).ceil();
            let high = (from + row.entry.spacing_m - INSET_M - along_m[axis]).floor();
            if high < low {
                return None;
            }
            let free = (high - low) as u64;
            let corner_m = low + (mix(h ^ (3 + axis as u64)) % (free + 1)) as f32;
            min[slot] = (corner_m / self.voxel_size).round() as i32;
        }
        Some(Structure {
            row: index,
            turn,
            min,
            size,
        })
    }

    /// Whether the ground under a drawn Structure takes it, and how high it then stands: its
    /// Ground layer on the top Voxel of the ground under its centre, if that column is in the row's
    /// Biome. The rules of where a Structure may stand belong here alone.
    fn structure_accepts(&self, row: &Row, mut s: Structure) -> Option<Structure> {
        let centre = |a: usize| {
            let slot = [0, 2][a];
            self.global_m(s.min[slot]) + s.size[slot] as f32 * self.voxel_size / 2.0
        };
        let (x_m, z_m) = (centre(0), centre(1));
        if self.biome_at(x_m, z_m) != Some(row.biome) {
            return None;
        }
        let top = top_index(self.surface_height_m(x_m, z_m), 0.0, self.voxel_size);
        s.min[1] = top - row.ground;
        Some(s)
    }

    /// The Structures of one row touching columns `region`, each held cell standing at the first
    /// of its draws the ground takes, before any earlier row is asked.
    fn row_structures(&self, index: usize, region: [[i32; 2]; 2]) -> Vec<Structure> {
        let Some(row) = self.structures.as_ref().map(|p| p.rows[index]) else {
            return Vec::new();
        };
        self.structure_cells(&row.entry, region)
            .into_iter()
            .filter_map(|cell| {
                let h = hashed(
                    self.seed,
                    STRUCTURE_SALT,
                    &[
                        row.biome.index() as u64,
                        row.row,
                        cell[0] as u32 as u64,
                        cell[1] as u32 as u64,
                    ],
                );
                if unit(mix(h ^ 1)) >= f64::from(row.entry.chance) {
                    return None;
                }
                (0..ATTEMPTS)
                    .filter_map(|attempt| self.structure_draw(index, &row, cell, h, attempt))
                    .find_map(|s| self.structure_accepts(&row, s))
            })
            .filter(|s| s.touches(region))
            .collect()
    }

    /// Every Structure that covers or borders a column of `region`, `[min, max)`, in claim order.
    /// A later row's Structure gives way to an earlier row's it would touch, so a column belongs to
    /// one Structure.
    pub(super) fn structures_near(&self, region: [[i32; 2]; 2]) -> Vec<Structure> {
        let Some(pass) = &self.structures else {
            return Vec::new();
        };
        let mut standing = Vec::new();
        for index in 0..pass.rows.len() {
            for s in self.row_structures(index, region) {
                let claimed =
                    (0..index).any(|before| !self.row_structures(before, s.footprint()).is_empty());
                if !claimed {
                    standing.push(s);
                }
            }
        }
        standing
    }

    /// The floors, in metres, a Structure's Voxels lie between, from the terrain band and the
    /// tallest and deepest listed Template alone. `None` where no Structure stands.
    pub(super) fn structure_band_m(&self) -> Option<(f32, f32)> {
        let pass = self.structures.as_ref()?;
        let (lowest, highest) = self.terrain_band_m();
        let size = self.voxel_size;
        Some((
            lowest - (pass.below + 1) as f32 * size,
            highest + (pass.above + 1) as f32 * size,
        ))
    }

    /// Whether a Structure may reach a Chunk whose Voxels span `floor_m..=ceiling_m`.
    pub(super) fn structures_may_reach(&self, floor_m: f32, ceiling_m: f32) -> bool {
        self.structure_band_m()
            .is_some_and(|(lowest, highest)| floor_m <= highest && ceiling_m >= lowest)
    }

    /// The top, in metres, of the highest Structure covering a column of `region`, `[min, max)`.
    pub(super) fn structures_clear_m(&self, [min, max]: [[i32; 2]; 2]) -> Option<f32> {
        self.structures_near([min, max])
            .into_iter()
            .filter(|s| {
                let [own_min, own_max] = s.footprint();
                (0..2).all(|a| own_min[a] < max[a] && min[a] < own_max[a])
            })
            .map(|s| self.global_m(s.min[1] + s.size[1]))
            .reduce(f32::max)
    }

    /// Writes every Structure's cells that fall in the Chunk at `coord`: a Keep cell is left,
    /// anything else, Air included, overwrites what is there.
    pub(super) fn stand_structures(&self, chunk: &mut Chunk, coord: [i32; 3]) {
        let Some(pass) = &self.structures else {
            return;
        };
        let size = CHUNK_SIZE as i32;
        let origin = coord.map(|c| c * size);
        let region = [[origin[0], origin[2]], [origin[0] + size, origin[2] + size]];
        for s in self.structures_near(region) {
            let low = [0, 1, 2].map(|a| s.min[a].max(origin[a]));
            let high = [0, 1, 2].map(|a| (s.min[a] + s.size[a]).min(origin[a] + size));
            if (0..3).any(|a| low[a] >= high[a]) {
                continue;
            }
            let template = &pass.templates[usize::from(pass.rows[s.row].entry.template)];
            let sampled = template.sample(self.voxel_size, s.turn);
            for y in low[1]..high[1] {
                for z in low[2]..high[2] {
                    for x in low[0]..high[0] {
                        let voxel = match sampled.cell([x - s.min[0], y - s.min[1], z - s.min[2]]) {
                            Cell::Keep => continue,
                            Cell::Air => Voxel::AIR,
                            Cell::Voxel(v) => v,
                        };
                        let [lx, ly, lz] = [x, y, z].map(|g| g.rem_euclid(size) as usize);
                        chunk.set(lx, ly, lz, voxel);
                    }
                }
            }
        }
    }
}
