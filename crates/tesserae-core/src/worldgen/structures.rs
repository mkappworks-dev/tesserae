//! Structures: Templates a Biome lists, spread by a Poisson-disc draw on a lattice (ADR-0029).
//!
//! Where a Structure stands, how it is turned and how high is a function of the seed and metres
//! alone, so every Chunk it reaches decides it alike and writes only its own Voxels.
//!
//! Claim order: Settlements first, keeping single Structures out of their discs and margins; then
//! earlier rows (by Biome, then by row), and a later row's draw that would touch one gives way.
//! Whatever is laid after Structures (fields, trees, scatter) asks [`Worldgen::claims_near`] for
//! footprints; a Structure never asks about them.

use std::sync::Arc;

use super::fields::{hashed, mix, unit};
use super::ground::SAMPLE_M;
use super::{Footprint, StructurePart, Worldgen, top_index};
use crate::chunk::{CHUNK_SIZE, Chunk};
use crate::climate::{BiomeId, BiomeStructure};
use crate::template::{Cell, Template, Turn};
use crate::voxel::{MaterialId, Voxel};
use crate::worldgen::WorldgenParams;
use crate::worldgen::settlements::LaidSettlement;

pub(super) const STRUCTURE_SALT: u64 = 0x5717_c7e0_0000_0001;
/// Kept clear of a lattice cell's border by a candidate's point, so its footprint's centre, once
/// the corner is on whole metres, is still in the cell.
pub(super) const INSET_M: f32 = 0.5;
/// No Structure or Settlement comes this near the World origin, where Games build their spawn.
pub(super) const ORIGIN_CLEAR_M: f32 = 48.0;
/// A lattice cell's eight neighbours.
pub(super) const NEIGHBOURS: [[i32; 2]; 8] = [
    [-1, -1],
    [0, -1],
    [1, -1],
    [-1, 0],
    [1, 0],
    [-1, 1],
    [0, 1],
    [1, 1],
];

/// One Biome's row, as generation at this Voxel size reads it.
#[derive(Clone, Copy, Debug)]
pub(super) struct Row {
    pub(super) biome: BiomeId,
    pub(super) row: u64,
    pub(super) entry: BiomeStructure,
    /// Voxels along x, y and z, by turn.
    pub(super) sizes: [[i32; 3]; 4],
    /// The highest Voxel layer of the Ground layer, which stands on the ground's top.
    pub(super) ground_top: i32,
    pub(super) level_tolerance_m: f32,
    /// The Template's longer side, in metres.
    longer_m: f32,
    /// Its Template spans a gap: how far above the Ground layer its deck's underside is.
    pub(super) underside_m: Option<f32>,
}

/// Built only when some Biome lists a Structure whose Template the World has, so a World without
/// one generates exactly as before.
#[derive(Clone, Debug)]
pub(super) struct StructurePass {
    /// In claim order.
    rows: Vec<Row>,
    /// By Biome, a bit for each Template it lists; a World holds at most 64.
    pub(super) listed: Vec<u64>,
    /// The most Voxels a Structure reaches above and below the ground's top Voxel.
    pub(super) above: i32,
    pub(super) below: i32,
}

impl StructurePass {
    pub(super) fn new(
        params: &WorldgenParams,
        voxel_size: f32,
        templates: &[Template],
    ) -> Option<Self> {
        params.ground?;
        let mut rows = Vec::new();
        let mut listed = vec![0; params.biomes.as_slice().len()];
        for (b, biome) in params.biomes.as_slice().iter().enumerate() {
            for (row, &entry) in biome.surface_features.structures().iter().enumerate() {
                let Some(template) = templates.get(usize::from(entry.template)) else {
                    continue;
                };
                listed[b] |= 1 << entry.template;
                let sampled = template.sample(voxel_size, Turn::NONE);
                rows.push(Row {
                    biome: BiomeId::new(b as u8),
                    row: row as u64,
                    entry,
                    sizes: [0, 1, 2, 3].map(|q| template.sample(voxel_size, Turn::new(q)).size()),
                    ground_top: sampled.ground_top(),
                    level_tolerance_m: template.level_tolerance_m(),
                    longer_m: template.size_m()[0].max(template.size_m()[2]),
                    underside_m: template.spans().then(|| template.underside_m()),
                });
            }
        }
        if rows.is_empty() {
            return None;
        }
        let above = rows.iter().map(|r| r.sizes[0][1] - r.ground_top).max()?;
        let below = rows.iter().map(|r| r.ground_top).max()?;
        Some(StructurePass {
            rows,
            listed,
            above,
            below,
        })
    }
}

/// A Structure that stands, alone or on a Settlement's Plot, in global Voxel indices.
#[derive(Clone, Copy, Debug)]
pub(super) struct Structure {
    /// An index into the World's Templates.
    pub(super) template: u16,
    pub(super) turn: Turn,
    pub(super) min: [i32; 3],
    pub(super) size: [i32; 3],
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
    pub(super) fn touches(&self, [min, max]: [[i32; 2]; 2]) -> bool {
        let [own_min, own_max] = self.footprint();
        (0..2).all(|a| own_min[a] <= max[a] && min[a] <= own_max[a])
    }
}

impl Worldgen {
    /// The lattice cells of a row whose Structure may touch columns `[min, max)`: a cell's
    /// Structure is centred in it and reaches half its Template's longer side past it.
    fn structure_cells(&self, row: &Row, [min, max]: [[i32; 2]; 2]) -> Vec<[i32; 2]> {
        let spacing = row.entry.spacing_m;
        let reach_m = row.longer_m / 2.0 + self.voxel_size;
        let at = |m: f32| (m / spacing).floor() as i32;
        let low = [0, 1].map(|a| at(self.global_m(min[a]) - reach_m));
        let high = [0, 1].map(|a| at(self.global_m(max[a]) + reach_m));
        (low[1]..=high[1])
            .flat_map(|j| (low[0]..=high[0]).map(move |i| [i, j]))
            .collect()
    }

    /// A held cell's candidate, from hashes alone: its Structure, turned, its corner on whole
    /// metres so it stands alike at every Voxel size; its footprint's centre in metres, inside the
    /// cell; and its priority over its neighbours'.
    fn structure_draw(&self, row: &Row, cell: [i32; 2]) -> Option<(Structure, [f32; 2], u64)> {
        let h = self.held(row, cell)?;
        let turn = Turn::new((mix(h ^ 2) & 3) as u8);
        let template = &self.templates[usize::from(row.entry.template)];
        let [x_m, _, z_m] = template.size_m();
        let along_m = if turn.quarters().is_multiple_of(2) {
            [x_m, z_m]
        } else {
            [z_m, x_m]
        };
        let size = row.sizes[usize::from(turn.quarters())];
        let (mut min, mut centre_m) = ([0; 3], [0.0; 2]);
        for (axis, slot) in [0, 2].into_iter().enumerate() {
            let spacing = row.entry.spacing_m;
            // Rounding the corner moves the centre at most half a metre, so it stays in the cell.
            let point_m = cell[axis] as f32 * spacing
                + INSET_M
                + unit(mix(h ^ (3 + axis as u64))) as f32 * (spacing - 2.0 * INSET_M);
            let corner_m = (point_m - along_m[axis] / 2.0).round();
            centre_m[axis] = corner_m + along_m[axis] / 2.0;
            min[slot] = (corner_m / self.voxel_size).round() as i32;
        }
        let s = Structure {
            template: row.entry.template,
            turn,
            min,
            size,
        };
        Some((s, centre_m, mix(h ^ 6)))
    }

    /// A cell's hash, if the row's chance holds it.
    pub(super) fn held(&self, row: &Row, cell: [i32; 2]) -> Option<u64> {
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
        (unit(mix(h ^ 1)) < f64::from(row.entry.chance)).then_some(h)
    }

    /// A held cell's candidate, if no neighbouring cell's candidate with a higher priority is
    /// within the row's spacing. Asks candidates alone, never the ground, so the answer is the
    /// same from every Chunk; the spacing is at least the cell's side, so only the eight
    /// neighbours can be that near.
    fn structure_candidate(&self, row: &Row, cell: [i32; 2]) -> Option<Structure> {
        let (s, centre_m, priority) = self.structure_draw(row, cell)?;
        let spacing = row.entry.spacing_m;
        let crowded = NEIGHBOURS.iter().any(|&[di, dj]| {
            let near = [cell[0] + di, cell[1] + dj];
            self.structure_draw(row, near)
                .is_some_and(|(_, other_m, other)| {
                    let [dx, dz] = [0, 1].map(|a| other_m[a] - centre_m[a]);
                    (other, near) > (priority, cell) && dx * dx + dz * dz < spacing * spacing
                })
        });
        (!crowded && self.clear_of_origin(&s)).then_some(s)
    }

    /// Whether every column of its footprint is at least [`ORIGIN_CLEAR_M`] from the origin.
    pub(super) fn clear_of_origin(&self, s: &Structure) -> bool {
        let [min, max] = s.footprint();
        let [dx, dz] = [0, 1].map(|a| {
            (min[a] as f32 * self.voxel_size)
                .max(-(max[a] as f32 * self.voxel_size))
                .max(0.0)
        });
        dx * dx + dz * dz >= ORIGIN_CLEAR_M * ORIGIN_CLEAR_M
    }

    /// Whether the ground under a drawn Structure takes it, and how high it then stands: its
    /// Ground layer on the top Voxel of the footprint's median ground. The rules of where a
    /// Structure may stand belong here alone, and ask only metres, so every Voxel size and every
    /// Chunk decides alike.
    fn structure_accepts(&self, row: &Row, mut s: Structure) -> Option<Structure> {
        let listed = &self.structures.as_ref()?.listed;
        let points: Vec<_> = self
            .sampled(&s, 2)
            .into_iter()
            .flat_map(|gz| self.sampled(&s, 0).into_iter().map(move |gx| (gx, gz)))
            .map(|(gx, gz)| ([self.global_m(gx), self.global_m(gz)], true))
            .collect();
        let (mut lowest_m, mut highest_m) = (f32::MAX, f32::MIN);
        let ground = self.sample_ground(points, |c| {
            let listed_here = self
                .biome_at(c.x_m, c.z_m)
                .is_some_and(|b| listed[b.index()] & 1 << row.entry.template != 0);
            (lowest_m, highest_m) = (lowest_m.min(c.height_m), highest_m.max(c.height_m));
            listed_here && highest_m - lowest_m <= row.level_tolerance_m
        })?;
        if !self.uncut(&ground) {
            return None;
        }
        let median_m = ground.lower_median_m()?;
        s.min[1] = top_index(median_m, 0.0, self.voxel_size) - row.ground_top;
        Some(s)
    }

    /// The columns of a footprint's `slot` axis the ground rules ask: a metre apart from its
    /// corner, which is on whole metres, so the same metre points at every Voxel size.
    fn sampled(&self, s: &Structure, slot: usize) -> Vec<i32> {
        let side_m = s.size[slot] as f32 * self.voxel_size;
        (0..)
            .map(|k| k as f32 * SAMPLE_M)
            .take_while(|&k_m| k_m < side_m)
            .map(|k_m| s.min[slot] + ((k_m / self.voxel_size).round() as i32).min(s.size[slot] - 1))
            .collect()
    }

    /// The Structures of one row touching columns `region`, each a cell's candidate that stands
    /// where the ground takes it, before any earlier row is asked.
    fn row_structures(&self, index: usize, region: [[i32; 2]; 2]) -> Vec<Structure> {
        let Some(row) = self.structures.as_ref().map(|p| p.rows[index]) else {
            return Vec::new();
        };
        self.structure_cells(&row, region)
            .into_iter()
            .filter_map(|cell| {
                self.layouts.structure((index, cell), || {
                    // A crossing keeps inside its own cell, so it needs no neighbour's draw.
                    if row.underside_m.is_some() {
                        return self.crossing(&row, cell);
                    }
                    let s = self.structure_candidate(&row, cell)?;
                    self.structure_accepts(&row, s)
                })
            })
            .filter(|s| s.touches(region))
            .collect()
    }

    /// Every Structure that covers or borders a column of `region`, `[min, max)`, in claim order.
    /// A later row's Structure gives way to an earlier row's it would touch, so a column belongs to
    /// one Structure, and every one gives way to a Settlement.
    pub(super) fn structures_near(&self, region: [[i32; 2]; 2]) -> Vec<Structure> {
        let Some(pass) = &self.structures else {
            return Vec::new();
        };
        let mut standing = Vec::new();
        for index in 0..pass.rows.len() {
            for s in self.row_structures(index, region) {
                let claimed =
                    (0..index).any(|before| !self.row_structures(before, s.footprint()).is_empty());
                if !claimed && !self.settled(s.footprint()) {
                    standing.push(s);
                }
            }
        }
        standing
    }

    /// The floors, in metres, a Structure's Voxels lie between, from the terrain band and the
    /// tallest and deepest listed Template alone. `None` where no Structure stands.
    pub(super) fn structure_band_m(&self) -> Option<(f32, f32)> {
        let reaches = [
            self.structures.as_ref().map(|p| (p.above, p.below)),
            self.settlements.as_ref().map(|p| (p.above, p.below)),
        ];
        let (above, below) = reaches
            .into_iter()
            .flatten()
            .reduce(|a, b| (a.0.max(b.0), a.1.max(b.1)))?;
        let (lowest, highest) = self.terrain_band_m();
        let size = self.voxel_size;
        Some((
            lowest - (below + 1) as f32 * size,
            highest + (above + 1) as f32 * size,
        ))
    }

    /// Whether a Structure may reach a Chunk whose Voxels span `floor_m..=ceiling_m`.
    pub(super) fn structures_may_reach(&self, floor_m: f32, ceiling_m: f32) -> bool {
        self.structure_band_m()
            .is_some_and(|(lowest, highest)| floor_m <= highest && ceiling_m >= lowest)
    }

    /// The top, in metres, of the highest Structure covering a column of `region`, `[min, max)`,
    /// alone or on a Plot.
    pub(super) fn structures_clear_m(&self, [min, max]: [[i32; 2]; 2]) -> Option<f32> {
        let claims = self.claims_near([min, max]);
        claims
            .settlements
            .iter()
            .flat_map(|t| t.structures.iter())
            .chain(&claims.structures)
            .filter(|s| {
                let [own_min, own_max] = s.footprint();
                (0..2).all(|a| own_min[a] < max[a] && min[a] < own_max[a])
            })
            .map(|s| self.global_m(s.min[1] + s.size[1]))
            .reduce(f32::max)
    }

    /// Everything that claims ground ahead of fields, trees and scatter and covers or borders a
    /// column of `region`: the Settlements, then the single Structures.
    pub(super) fn claims_near(&self, region: [[i32; 2]; 2]) -> Claims {
        if self.structure_band_m().is_none() {
            return Claims::default();
        }
        Claims {
            settlements: self.laid_settlements_near(region),
            structures: self.structures_near(region),
        }
    }

    /// [`Self::claims_near`] for `region`, from `claims`, its answer for a region holding it: in
    /// the same order, since both keep claim order.
    pub(super) fn claims_within(&self, claims: &Claims, region: [[i32; 2]; 2]) -> Claims {
        Claims {
            settlements: claims
                .settlements
                .iter()
                .filter(|laid| self.laid_near(laid, region))
                .cloned()
                .collect(),
            structures: claims
                .structures
                .iter()
                .filter(|s| s.touches(region))
                .copied()
                .collect(),
        }
    }

    /// What fields, trees and scatter keep off.
    pub(super) fn footprints(&self, claims: &Claims) -> Vec<Footprint> {
        claims
            .settlements
            .iter()
            .flat_map(|t| t.footprints(self.voxel_size))
            .chain(
                claims
                    .structures
                    .iter()
                    .map(|s| Footprint::rect(s.footprint())),
            )
            .collect()
    }

    /// Writes every claim that falls in the Chunk at `coord`: Settlements, then single Structures.
    pub(super) fn stand_claims(&self, chunk: &mut Chunk, coord: [i32; 3], claims: &Claims) {
        for laid in &claims.settlements {
            self.stand_settlement(chunk, coord, laid);
        }
        for s in &claims.structures {
            self.stand_structure(chunk, coord, s, true);
        }
    }

    /// Writes a Structure's cells that fall in the Chunk at `coord`: a Keep cell is left, anything
    /// else, Air included, overwrites what is there. Under a Voxel in its lowest layer, the
    /// foundation Material fills down to the ground, unless `foundation` is false, where the
    /// ground has been levelled to it.
    pub(super) fn stand_structure(
        &self,
        chunk: &mut Chunk,
        coord: [i32; 3],
        s: &Structure,
        foundation: bool,
    ) {
        let size = CHUNK_SIZE as i32;
        let origin = coord.map(|c| c * size);
        self.structure_voxels(
            s,
            foundation,
            [origin, origin.map(|o| o + size)],
            |at, voxel| {
                let [lx, ly, lz] = at.map(|g| g.rem_euclid(size) as usize);
                chunk.set(lx, ly, lz, voxel);
            },
        );
    }

    /// Each cell [`Self::stand_structure`] writes in the global cells `[min, max)`, in the order
    /// it writes them.
    pub(super) fn structure_voxels(
        &self,
        s: &Structure,
        foundation: bool,
        [min, max]: [[i32; 3]; 2],
        mut put: impl FnMut([i32; 3], Voxel),
    ) {
        let low = [0, 1, 2].map(|a| s.min[a].max(min[a]));
        let high = [0, 1, 2].map(|a| (s.min[a] + s.size[a]).min(max[a]));
        if low[0] >= high[0] || low[2] >= high[2] {
            return;
        }
        let template = &self.templates[usize::from(s.template)];
        let sampled = template.sample(self.voxel_size, s.turn);
        for z in low[2]..high[2] {
            for x in low[0]..high[0] {
                let landing = !template.spans() || self.on_landing(s, [x, z]);
                let ground = std::cell::OnceCell::new();
                for y in low[1]..high[1] {
                    let cell = sampled.cell([x - s.min[0], y - s.min[1], z - s.min[2]]);
                    let voxel = match cell {
                        // Under a span the gap stays as it is, ground, water or air.
                        Cell::Air | Cell::Ground if !landing => continue,
                        Cell::Keep => continue,
                        Cell::Air => Voxel::AIR,
                        Cell::Voxel(v) => v,
                        Cell::Ground => {
                            let ground = ground.get_or_init(|| self.column_ground(x, z));
                            let Some(ground) = *ground else {
                                continue;
                            };
                            sampled.ground_voxel(y - s.min[1], ground)
                        }
                    };
                    put([x, y, z], voxel);
                }
            }
        }
        let Some(foundation) = template.foundation().filter(|_| foundation) else {
            return;
        };
        if min[1] >= s.min[1] {
            return;
        }
        for z in low[2]..high[2] {
            for x in low[0]..high[0] {
                if !matches!(
                    sampled.cell([x - s.min[0], 0, z - s.min[2]]),
                    Cell::Voxel(_) | Cell::Ground
                ) || (template.spans() && !self.on_landing(s, [x, z]))
                {
                    continue;
                }
                let height_m = self.surface_height_m(self.global_m(x), self.global_m(z));
                let ground = top_index(height_m, 0.0, self.voxel_size);
                let from = (ground + 1).max(min[1]);
                for y in from..s.min[1].min(max[1]) {
                    put([x, y, z], Voxel::cube(foundation));
                }
            }
        }
    }
}

impl Worldgen {
    /// Every solid Voxel a Structure or a Settlement Plot's Structure stands in the global cells
    /// `[min, max)`, in the order [`Self::chunk`] writes them: what a coarse level samples its
    /// Structure Impostors from (ADR-0035). It reads the draws and the ground's heights alone,
    /// never a Chunk.
    pub fn structure_parts(&self, min: [i32; 3], max: [i32; 3]) -> Vec<StructurePart> {
        let Some((lowest_m, highest_m)) = self.structure_band_m() else {
            return Vec::new();
        };
        if self.global_m(min[1]) > highest_m || self.global_m(max[1]) < lowest_m {
            return Vec::new();
        }
        let claims = self.claims_near([[min[0], min[2]], [max[0], max[2]]]);
        let mut parts = Vec::new();
        let mut put = |at: [i32; 3], voxel: Voxel| {
            if !voxel.is_air() {
                parts.push(StructurePart {
                    at,
                    material: voxel.material(),
                });
            }
        };
        let plots = claims.settlements.iter().flat_map(|t| t.structures.iter());
        for s in plots {
            self.structure_voxels(s, false, [min, max], &mut put);
        }
        for s in &claims.structures {
            self.structure_voxels(s, true, [min, max], &mut put);
        }
        parts
    }

    /// The Material each Template stands that no Biome lays as ground, sorted: what a coarse
    /// level keeps by any solid over a changed Chunk, where ground keeps to the vote.
    pub fn built_materials(&self) -> Vec<MaterialId> {
        let mut ground: Vec<MaterialId> = self
            .params
            .biomes
            .as_slice()
            .iter()
            .filter_map(|b| b.ground)
            .chain(self.params.ground)
            .flat_map(|g| [g.rock, g.soil, g.surface])
            .chain(self.water.map(|w| w.material))
            .collect();
        ground.sort_unstable();
        let mut built = Vec::new();
        for template in self.templates.iter() {
            let [sx, sy, sz] = template.size();
            for y in 0..sy {
                for z in 0..sz {
                    for x in 0..sx {
                        if let Cell::Voxel(v) = template.cell([x, y, z])
                            && !v.is_air()
                        {
                            built.push(v.material());
                        }
                    }
                }
            }
            built.extend(template.foundation());
        }
        built.sort_unstable();
        built.dedup();
        built.retain(|m| ground.binary_search(m).is_err());
        built
    }
}

/// What claims ground near a region, worked out once per Chunk.
#[derive(Default)]
pub(super) struct Claims {
    pub(super) settlements: Vec<Arc<LaidSettlement>>,
    pub(super) structures: Vec<Structure>,
}
