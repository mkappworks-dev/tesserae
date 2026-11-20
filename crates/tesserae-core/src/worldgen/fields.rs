//! Fields: a region of ground that holds mostly one species, and planted plots of one crop.
//!
//! Both are pure functions of the seed, a position in metres and the Voxel size, so every Chunk
//! that reaches a column decides alike and the field is whole across Chunk borders.

use super::{SCATTER_SEED, SurfaceScatter, Worldgen, top_index};
use crate::climate::{BiomeId, CropField, Field, SurfaceFeatures, SurfacePlantKind};
use crate::voxel::Voxel;

pub(super) fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// Uniform in `[0, 1)`.
pub(super) fn unit(h: u64) -> f64 {
    (h >> 11) as f64 / (1u64 << 53) as f64
}

pub(super) fn hashed(seed: u32, salt: u64, parts: &[u64]) -> u64 {
    parts
        .iter()
        .fold(mix(SCATTER_SEED ^ u64::from(seed) ^ salt), |h, &p| {
            mix(h ^ p)
        })
}

/// Keeps the region lattice and the stray draw apart from every other draw of a column.
const REGION_SALT: u64 = 0x7e61_0f1e_1d00_0001;
const STRAY_SALT: u64 = 0x7e61_0f1e_1d00_0002;
const FIELD_SALT: u64 = 0x7e61_0f1e_1d00_0003;

/// The member of a group, as an index among its `members`, that the region holding `(x_m, z_m)`
/// is given: the nearest point of a lattice `scale_m` apart, each moved within its cell by hash.
fn region_member(field: &Field, members: u64, seed: u32, x_m: f32, z_m: f32) -> u64 {
    let (px, pz) = (x_m / field.scale_m, z_m / field.scale_m);
    let (ci, cj) = (px.floor() as i32, pz.floor() as i32);
    let mut nearest = (f32::MAX, 0);
    for j in cj - 1..=cj + 1 {
        for i in ci - 1..=ci + 1 {
            let h = hashed(
                seed,
                REGION_SALT,
                &[u64::from(field.group), i as u32 as u64, j as u32 as u64],
            );
            let jitter = |salt: u64| 0.15 + 0.7 * unit(mix(h ^ salt)) as f32;
            let (dx, dz) = (px - (i as f32 + jitter(1)), pz - (j as f32 + jitter(2)));
            let d = dx * dx + dz * dz;
            if d < nearest.0 {
                nearest = (d, h);
            }
        }
    }
    mix(nearest.1 ^ 3) % members
}

/// Whether the feature at `index`, drawn on the column at `(x_m, z_m)`, may stand: one with no
/// field always may; a member of a group stands in the regions that hold it and, with the field's
/// `strays`, anywhere.
pub(super) fn admits(
    features: &SurfaceFeatures,
    index: usize,
    seed: u32,
    x_m: f32,
    z_m: f32,
) -> bool {
    let all = features.as_slice();
    let Some(field) = all[index].field else {
        return true;
    };
    let same =
        |f: &&crate::climate::SurfaceFeature| f.field.is_some_and(|g| g.group == field.group);
    let members = all.iter().filter(same).count() as u64;
    if members < 2 {
        return true;
    }
    let rank = all[..index].iter().filter(same).count() as u64;
    rank == region_member(&field, members, seed, x_m, z_m)
        || SurfaceScatter::draw_salted(seed, x_m, z_m, STRAY_SALT) < f64::from(field.strays)
}

/// A lattice cell holds a field this often; the cell's side is the spacing, so the mean distance
/// between fields is that over the square root of this, near the spacing.
const FIELD_CHANCE: f64 = 0.5;
/// Kept clear of a lattice cell's border, so two cells' fields never touch.
const INSET_M: f32 = 2.0;
const MAX_PLOTS_PER_SIDE: u64 = 3;
/// Places a held cell tries before it holds no field: ground is rarely level, so a cell looks about.
const ATTEMPTS: u64 = 36;
/// The most the ground may differ across a field.
const LEVEL_M: f32 = 1.0;
/// Columns are asked this far apart when a field is tested for ground, Biome and caves.
const SAMPLE_M: f32 = 1.0;

/// A crop-field lattice cell of one of a Biome's fields.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct FieldKey {
    biome: u8,
    /// Among the Biome's fields.
    entry: u8,
    cell: [i32; 2],
}

/// A planted field's place, in global Voxel indices: a grid of plots set apart by lanes one Voxel
/// wide, which also ring it.
#[derive(Clone, Copy, Debug)]
pub(super) struct Placed {
    biome: BiomeId,
    field: CropField,
    hash: u64,
    min: [i32; 2],
    plots: [i32; 2],
    /// A plot's side in Voxels, along x then z.
    side: [i32; 2],
    /// The edge, `-x`, `+x`, `-z`, `+z`, the ditch runs along.
    ditch: Option<u8>,
    rows_along_x: bool,
}

/// What a field puts in one column.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Cell {
    Path,
    Ditch,
    Plot {
        crop: usize,
        stage: usize,
        /// Every second row of a plot is planted.
        planted: bool,
    },
}

impl Placed {
    fn extent(&self) -> [i32; 2] {
        [0, 1].map(|a| self.plots[a] * (self.side[a] + 1) + 1)
    }

    fn max(&self) -> [i32; 2] {
        let extent = self.extent();
        [0, 1].map(|a| self.min[a] + extent[a])
    }

    /// The columns it covers, as `[min, max)` per axis.
    pub(super) fn footprint(&self) -> [[i32; 2]; 2] {
        [self.min, self.max()]
    }

    pub(super) fn cell(&self, gx: i32, gz: i32) -> Option<Cell> {
        let [wx, wz] = self.extent();
        let (dx, dz) = (gx - self.min[0], gz - self.min[1]);
        if !(0..wx).contains(&dx) || !(0..wz).contains(&dz) {
            return None;
        }
        let in_plot = |d: i32, axis: usize| {
            let period = self.side[axis] + 1;
            (d % period != 0).then(|| (d / period, d % period - 1))
        };
        let (Some((px, ox)), Some((pz, oz))) = (in_plot(dx, 0), in_plot(dz, 1)) else {
            let on_ditch = match self.ditch {
                Some(0) => dx == 0,
                Some(1) => dx == wx - 1,
                Some(2) => dz == 0,
                Some(3) => dz == wz - 1,
                _ => false,
            };
            return Some(if on_ditch { Cell::Ditch } else { Cell::Path });
        };
        let plot = mix(self.hash ^ ((px as u64) << 16 | pz as u64) ^ 0x9107);
        let total: f32 = self.field.crops().iter().map(|c| c.weight).sum();
        let mut at = unit(mix(plot ^ 1)) as f32 * total;
        let crop = self
            .field
            .crops()
            .iter()
            .position(|c| {
                at -= c.weight;
                at < 0.0
            })
            .unwrap_or(self.field.crops().len() - 1);
        let stages = self.field.crops()[crop].len as usize;
        let stage = if unit(mix(plot ^ 2)) < 0.5 {
            stages - 1
        } else {
            (unit(mix(plot ^ 3)) * stages as f64) as usize
        };
        let row = if self.rows_along_x { oz } else { ox };
        Some(Cell::Plot {
            crop,
            stage: stage.min(stages - 1),
            planted: row % 2 == 0,
        })
    }
}

/// The field covering the column, and what it holds there.
pub(super) fn cell_at(placed: &[Placed], gx: i32, gz: i32) -> Option<(&Placed, Cell)> {
    placed.iter().find_map(|p| Some((p, p.cell(gx, gz)?)))
}

impl Worldgen {
    /// Whether the lattice cell holds a field at all, from the seed and the cell alone.
    fn field_held(&self, biome: BiomeId, entry: usize, cell: [i32; 2]) -> Option<u64> {
        let h = hashed(
            self.seed,
            FIELD_SALT,
            &[
                biome.index() as u64,
                entry as u64,
                cell[0] as u32 as u64,
                cell[1] as u32 as u64,
            ],
        );
        (unit(mix(h ^ 1)) < FIELD_CHANCE).then_some(h)
    }

    /// The `try`th place a held cell's field may stand: how big and where in the cell, before the
    /// ground is asked. `None` where it would not fit the cell.
    fn field_draw(
        &self,
        biome: BiomeId,
        field: &CropField,
        cell: [i32; 2],
        held: u64,
        attempt: u64,
    ) -> Option<Placed> {
        let size = self.voxel_size;
        let h = mix(held ^ attempt.wrapping_mul(0x9e37_79b9_7f4a_7c15));
        let mut min = [0; 2];
        let mut side = [0; 2];
        let mut plots = [0; 2];
        for axis in 0..2 {
            let from = cell[axis] as f32 * field.spacing_m;
            let low = ((from + INSET_M) / size).ceil() as i32;
            let high = ((from + field.spacing_m - INSET_M) / size).floor() as i32;
            let room = high - low + 1;
            let a = axis as u64;
            let (short, long) = field.plot_m;
            let side_m = short + (long - short) * unit(mix(h ^ (2 + a))) as f32;
            side[axis] = ((side_m / size).round() as i32).max(1);
            let fit = (room - 1) / (side[axis] + 1);
            if fit < 1 {
                return None;
            }
            // The first places try the biggest fields and the last just one plot, so level ground
            // holds a big field and rough ground a small one.
            let cap = MAX_PLOTS_PER_SIDE.saturating_sub(attempt * MAX_PLOTS_PER_SIDE / ATTEMPTS);
            let most = fit.min(cap.max(1) as i32) as u64;
            plots[axis] = 1 + (mix(h ^ (4 + a)) % most) as i32;
            let free = room - (plots[axis] * (side[axis] + 1) + 1);
            min[axis] = low + (mix(h ^ (6 + a)) % (free as u64 + 1)) as i32;
        }
        Some(Placed {
            biome,
            field: *field,
            hash: h,
            min,
            plots,
            side,
            ditch: None,
            rows_along_x: mix(h ^ 8) & 1 == 1,
        })
    }

    /// The lattice cells of `field` that the Voxel columns `[min, max)` touch.
    fn field_cells(&self, field: &CropField, [min, max]: [[i32; 2]; 2]) -> Vec<[i32; 2]> {
        let at = |g: i32| (g as f32 * self.voxel_size / field.spacing_m).floor() as i32;
        let (low, high) = ([0, 1].map(|a| at(min[a])), [0, 1].map(|a| at(max[a] - 1)));
        (low[1]..=high[1])
            .flat_map(|j| (low[0]..=high[0]).map(move |i| [i, j]))
            .collect()
    }

    /// The field a lattice cell of a Biome's entry holds: none unless held, else the first of its
    /// drawn places the ground takes. Decided once, since no region it is asked from changes it.
    fn cell_field(
        &self,
        scatter: &SurfaceScatter,
        biome: BiomeId,
        entry: usize,
        field: &CropField,
        cell: [i32; 2],
    ) -> Option<Placed> {
        let key = FieldKey {
            biome: biome.index() as u8,
            entry: entry as u8,
            cell,
        };
        *self.fields.get_or_build(key, || {
            let held = self.field_held(biome, entry, cell)?;
            (0..ATTEMPTS)
                .filter_map(|attempt| self.field_draw(biome, field, cell, held, attempt))
                .find_map(|p| self.field_accepts(scatter, p))
        })
    }

    /// The fields of one entry of a Biome that stand on any column of `region`, `[min, max)`: each
    /// held cell stands at the first of its drawn places the ground takes.
    fn entry_fields(
        &self,
        scatter: &SurfaceScatter,
        biome: BiomeId,
        entry: usize,
        region: [[i32; 2]; 2],
    ) -> Vec<Placed> {
        let Some(field) = self
            .params
            .biomes
            .get(biome)
            .and_then(|b| b.surface_features.crop_fields().get(entry))
        else {
            return Vec::new();
        };
        let crosses = |p: &Placed| {
            let [min, max] = p.footprint();
            (0..2).all(|a| min[a] < region[1][a] && region[0][a] < max[a])
        };
        self.field_cells(field, region)
            .into_iter()
            .filter_map(|cell| self.cell_field(scatter, biome, entry, field, cell))
            .filter(crosses)
            .collect()
    }

    /// Every planted field that stands on any column of `region`, `[min, max)`, in a fixed order.
    /// A later entry of a Biome gives way to an earlier one it would cross, so a column belongs to
    /// one field.
    pub(super) fn fields_near(
        &self,
        scatter: &SurfaceScatter,
        region: [[i32; 2]; 2],
    ) -> Vec<Placed> {
        let mut placed = Vec::new();
        if !scatter.fields {
            return placed;
        }
        for (b, biome) in self.params.biomes.as_slice().iter().enumerate() {
            let id = BiomeId::new(b as u8);
            for entry in 0..biome.surface_features.crop_fields().len() {
                for p in self.entry_fields(scatter, id, entry, region) {
                    let crossed = (0..entry).any(|before| {
                        !self
                            .entry_fields(scatter, id, before, p.footprint())
                            .is_empty()
                    });
                    if !crossed {
                        placed.push(p);
                    }
                }
            }
        }
        placed
    }

    /// Whether the ground under a drawn field takes it, and which edge is its ditch. The same
    /// answer from every Chunk, since it asks only metres: columns a metre apart for the Biome,
    /// the lake, caves and the lie of the land, every column for a tree.
    fn field_accepts(&self, scatter: &SurfaceScatter, mut p: Placed) -> Option<Placed> {
        let size = self.voxel_size;
        let (_, highest) = self.terrain_band_m();
        let [wx, wz] = p.extent();
        if let Some(reach_m) = p.field.near_water_m {
            self.water?;
            let [min, max] = p.footprint();
            let centre = |a: usize| (self.global_m(min[a]) + self.global_m(max[a])) / 2.0;
            let (cx, cz) = (centre(0), centre(1));
            let near = [0.5, 1.0].into_iter().any(|k: f32| {
                (0..8).any(|d| {
                    let angle = d as f32 * std::f32::consts::FRAC_PI_4;
                    let (x, z) = (
                        cx + reach_m * k * libm::cosf(angle),
                        cz + reach_m * k * libm::sinf(angle),
                    );
                    self.water_surface_m(x, z).is_some()
                })
            });
            if !near {
                return None;
            }
        }
        let stride = ((SAMPLE_M / size).round() as usize).max(1);
        let axis = |w: i32| {
            let mut all: Vec<i32> = (0..w).step_by(stride).collect();
            if all.last() != Some(&(w - 1)) {
                all.push(w - 1);
            }
            all
        };
        let (xs, zs) = (axis(wx), axis(wz));
        // The corners, middles and centre first, so ground that is not level fails after few asks.
        let (mx, mz) = (wx / 2, wz / 2);
        let mut samples: Vec<[i32; 2]> = [0, mx, wx - 1]
            .into_iter()
            .flat_map(|x| [0, mz, wz - 1].map(move |z| [x, z]))
            .collect();
        samples.extend(zs.iter().flat_map(|&z| xs.iter().map(move |&x| [x, z])));
        let level = ((LEVEL_M / size).ceil() as i32).max(1);
        let (mut lowest, mut highest_top) = (i32::MAX, i32::MIN);
        let mut edges = [(0i64, 0i64); 4];
        for [dx, dz] in samples {
            let (gx, gz) = (p.min[0] + dx, p.min[1] + dz);
            let (x_m, z_m) = (self.global_m(gx), self.global_m(gz));
            if self.column_biome_id(gx, gz) != Some(p.biome) {
                return None;
            }
            let top = top_index(self.surface_height_m(x_m, z_m), 0.0, size);
            let y_m = self.global_m(top + 1);
            if y_m > highest
                || self.carved([gx, top, gz], None)
                || self
                    .water_level_m(x_m, z_m)
                    .is_some_and(|level_m| y_m < level_m)
            {
                return None;
            }
            lowest = lowest.min(top);
            highest_top = highest_top.max(top);
            if highest_top - lowest > level {
                return None;
            }
            for (edge, on) in [dx == 0, dx == wx - 1, dz == 0, dz == wz - 1]
                .into_iter()
                .enumerate()
            {
                if on {
                    edges[edge].0 += i64::from(top);
                    edges[edge].1 += 1;
                }
            }
        }
        for gz in p.min[1]..p.min[1] + wz {
            for gx in p.min[0]..p.min[0] + wx {
                let tree = self
                    .plant_raw(scatter, gx, gz)
                    .is_some_and(|s| matches!(s.plant.kind, SurfacePlantKind::Tree(_)));
                if tree {
                    return None;
                }
            }
        }
        if p.field.ditch.is_some() {
            let mean = |(sum, n): (i64, i64)| sum as f64 / n.max(1) as f64;
            p.ditch = (0..4u8)
                .min_by(|&a, &b| mean(edges[a as usize]).total_cmp(&mean(edges[b as usize])));
        }
        Some(p)
    }

    /// Lays the paths and ditches of `placed` in the ground and stands their crops, for the
    /// columns of the Chunk at `coord` that `kept` leaves free.
    pub(super) fn crop_fields(
        &self,
        chunk: &mut crate::chunk::Chunk,
        placed: &[Placed],
        kept: &[super::Footprint],
        coord: [i32; 3],
    ) {
        let size = crate::chunk::CHUNK_SIZE as i32;
        let origin = coord.map(|c| c * size);
        let inside = |y: i32| (origin[1]..origin[1] + size).contains(&y);
        for p in placed {
            let [min, max] = p.footprint();
            for gz in min[1].max(origin[2])..max[1].min(origin[2] + size) {
                for gx in min[0].max(origin[0])..max[0].min(origin[0] + size) {
                    let Some(cell) = p.cell(gx, gz).filter(|_| !super::covers(kept, gx, gz)) else {
                        continue;
                    };
                    let (x_m, z_m) = (self.global_m(gx), self.global_m(gz));
                    let top = top_index(self.surface_height_m(x_m, z_m), 0.0, self.voxel_size);
                    let local = |y: i32| {
                        [
                            (gx - origin[0]) as usize,
                            (y - origin[1]) as usize,
                            (gz - origin[2]) as usize,
                        ]
                    };
                    let lay = |chunk: &mut crate::chunk::Chunk, material| {
                        if inside(top) {
                            let [x, y, z] = local(top);
                            if !chunk.get(x, y, z).is_air() {
                                chunk.set(x, y, z, Voxel::cube(material));
                            }
                        }
                    };
                    match cell {
                        Cell::Path => {
                            if let Some(path) = p.field.path {
                                lay(chunk, path);
                            }
                        }
                        Cell::Ditch => {
                            if let Some(fluid) = p.field.ditch {
                                lay(chunk, fluid);
                            }
                        }
                        Cell::Plot {
                            crop,
                            stage,
                            planted: true,
                        } if inside(top + 1) => {
                            let [x, y, z] = local(top + 1);
                            if chunk.get(x, y, z).is_air() {
                                chunk.set(x, y, z, p.field.crops()[crop].voxel(stage));
                            }
                        }
                        Cell::Plot { .. } => {}
                    }
                }
            }
        }
    }
}
