//! Settlements: roads branched from a centre and Plots beside them, each laid out from the seed
//! and the Settlement's lattice cell alone (ADR-0029).
//!
//! The layout works in metres, every Plot's corner on whole metres, so it is the same at every
//! Voxel size; every Chunk a Settlement reaches lays it out again and writes only its own Voxels.
//! A Settlement claims ground before single Structures, fields, trees and scatter, and never asks
//! about any of them.

use std::f32::consts::TAU;
use std::sync::Arc;

use super::fields::{hashed, mix, unit};
use super::structures::Structure;
use super::{Footprint, Worldgen, top_index};
use crate::chunk::{CHUNK_SIZE, Chunk};
use crate::climate::{BIOMES_SINCE, BiomeId, BiomeSettlement};
use crate::settlement::{Building, Settlement};
use crate::template::{Template, Turn};
use crate::voxel::Voxel;
use crate::worldgen::WorldgenParams;

const SETTLEMENT_SALT: u64 = 0x5e77_1e00_0000_0001;
/// A road's cells are those whose centre is this near its line.
const ROAD_HALF_M: f32 = 1.5;
const ROAD_STEP_M: f32 = 4.0;
/// The steepest a road's step climbs, rise over run.
const MAX_ROAD_GRADE: f32 = 0.45;
/// The bends a road tries at each step, in radians off its heading, straightest first.
const ROAD_BENDS: [f32; 5] = [0.0, -0.3, 0.3, -0.6, 0.6];
/// Each bend's cost, against a grade, so a road keeps straight on even ground.
const BEND_COST: f32 = 0.05;
/// Ground kept clear around a Plot: between its Structure and a road, and between two.
const APRON_M: f32 = 2.0;
/// How far apart along a road Plots are tried.
const SLOT_M: f32 = 2.0;
/// How far apart the ground rules ask a Plot's columns.
const SAMPLE_M: f32 = 1.0;

/// A cardinal direction's step on the ground plane, indexed as a [`Turn`] takes +Z: +Z, +X, -Z,
/// -X.
const FACINGS: [[f32; 2]; 4] = [[0.0, 1.0], [1.0, 0.0], [0.0, -1.0], [-1.0, 0.0]];

#[derive(Clone, Copy, Debug)]
struct Row {
    biome: BiomeId,
    row: u64,
    entry: BiomeSettlement,
}

/// Built only when some Biome lists a Settlement the World has, so a World without one generates
/// exactly as before.
#[derive(Clone, Debug)]
pub(super) struct SettlementPass {
    settlements: Arc<[Settlement]>,
    /// In claim order.
    rows: Vec<Row>,
    /// By Biome, a bit for each Settlement it lists.
    listed: Vec<u64>,
    /// The most Voxels a Plot's Structure reaches above and below the ground's top Voxel.
    pub(super) above: i32,
    pub(super) below: i32,
    /// The widest disc and margin of any row, in metres.
    reach_m: f32,
}

impl SettlementPass {
    pub(super) fn new(
        params: &WorldgenParams,
        voxel_size: f32,
        templates: &[Template],
        settlements: &[Settlement],
    ) -> Option<Self> {
        if params.generator < BIOMES_SINCE || params.ground.is_none() {
            return None;
        }
        let mut rows = Vec::new();
        let mut listed = vec![0; params.biomes.as_slice().len()];
        let (mut above, mut below, mut reach_m) = (0, 0, 0.0f32);
        for (b, biome) in params.biomes.as_slice().iter().enumerate() {
            for (row, &entry) in biome.surface_features.settlements().iter().enumerate() {
                let Some(settlement) = settlements.get(usize::from(entry.settlement)) else {
                    continue;
                };
                listed[b] |= 1 << entry.settlement;
                rows.push(Row {
                    biome: BiomeId::new(b as u8),
                    row: row as u64,
                    entry,
                });
                reach_m = reach_m.max(settlement.radius_m + settlement.margin_m);
                let buildings = settlement
                    .centre
                    .iter()
                    .chain(settlement.rules.iter().map(|r| &r.building));
                for building in buildings {
                    let sampled = templates
                        .get(usize::from(building.template))?
                        .sample(voxel_size, Turn::NONE);
                    above = above.max(sampled.size()[1] - sampled.ground());
                    below = below.max(sampled.ground());
                }
            }
        }
        if rows.is_empty() {
            return None;
        }
        Some(SettlementPass {
            settlements: settlements.into(),
            rows,
            listed,
            above,
            below,
            reach_m,
        })
    }
}

/// A lattice cell's Settlement whose centre the ground takes, before its roads and Plots are
/// laid out.
#[derive(Clone, Copy, Debug)]
struct Site {
    row: usize,
    hash: u64,
    /// On whole metres.
    centre_m: [f32; 2],
    /// Its disc and margin, in metres.
    reach_m: f32,
    /// The way its first road leaves, and its centre's door faces.
    facing: u8,
    centre: Option<Structure>,
}

impl Site {
    fn overlaps(&self, other: &Site) -> bool {
        let [dx, dz] = [0, 1].map(|a| self.centre_m[a] - other.centre_m[a]);
        (dx * dx + dz * dz).sqrt() < self.reach_m + other.reach_m
    }

    /// Whether its disc and margin reach a point of `[min, max)`, in metres.
    fn reaches(&self, [min, max]: [[f32; 2]; 2], reach_m: f32) -> bool {
        let [dx, dz] = [0, 1].map(|a| {
            let c = self.centre_m[a];
            c - c.clamp(min[a], max[a])
        });
        dx * dx + dz * dz < reach_m * reach_m
    }
}

/// A Settlement laid out: its roads, a metre cell each, and its Plots' Structures.
#[derive(Clone, Debug)]
pub(super) struct Town {
    centre_m: [f32; 2],
    radius_m: f32,
    path: Voxel,
    /// The metre cell at the corner of the square the disc fits in.
    origin_m: [i32; 2],
    side: i32,
    road: Vec<bool>,
    /// The centre's first, if it has one.
    pub(super) plots: Vec<Structure>,
}

impl Town {
    fn road_at(&self, [x, z]: [i32; 2]) -> bool {
        let [i, j] = [x - self.origin_m[0], z - self.origin_m[1]];
        (0..self.side).contains(&i)
            && (0..self.side).contains(&j)
            && self.road[(j * self.side + i) as usize]
    }

    /// Its disc, which fields, trees and scatter keep off.
    pub(super) fn footprint(&self, voxel_size: f32) -> Footprint {
        let centre = self.centre_m.map(|c| c / voxel_size);
        let radius = self.radius_m / voxel_size;
        Footprint {
            min: centre.map(|c| (c - radius).floor() as i32),
            max: centre.map(|c| (c + radius).ceil() as i32),
            disc: Some((centre, radius)),
        }
    }
}

/// A Plot taken, as rectangles in metres `[min, max)`: its Structure, and that widened by the
/// apron.
#[derive(Clone, Copy, Debug)]
struct Lot {
    kept: [[f32; 2]; 2],
}

fn overlap([a_min, a_max]: [[f32; 2]; 2], [b_min, b_max]: [[f32; 2]; 2]) -> bool {
    (0..2).all(|a| a_min[a] < b_max[a] && b_min[a] < a_max[a])
}

fn widened([min, max]: [[f32; 2]; 2], by: f32) -> [[f32; 2]; 2] {
    [min.map(|v| v - by), max.map(|v| v + by)]
}

/// The side of a Template its door is on, as a [`Turn`] takes +Z: the edge nearest the door's
/// cell. A Template with no door turns its +Z edge to the road.
fn door_side(template: &Template, door: Option<[u16; 2]>) -> u8 {
    let Some([x, z]) = door.map(|d| d.map(i32::from)) else {
        return 0;
    };
    let [w, _, d] = template.size().map(|n| n as i32);
    let distances = [d - 1 - z, w - 1 - x, z, x];
    (0..4u8)
        .min_by_key(|&q| distances[usize::from(q)])
        .unwrap_or_default()
}

impl Worldgen {
    fn settlement_pass(&self) -> Option<&SettlementPass> {
        self.settlements.as_ref()
    }

    /// Whether a Settlement's disc and margin reach a column of `footprint`, `[min, max)`: what
    /// keeps a single Structure out. Asks only the centres, so it stays cheap per Structure.
    pub(super) fn settled(&self, footprint: [[i32; 2]; 2]) -> bool {
        !self.sites_near(self.region_m(footprint)).is_empty()
    }

    /// Every Settlement laid out whose disc covers or borders a column of `region`, `[min, max)`.
    pub(super) fn towns_near(&self, region: [[i32; 2]; 2]) -> Vec<Town> {
        let region = self.region_m(region);
        self.sites_near(region)
            .into_iter()
            .filter(|s| {
                let radius_m = self.settlement_of(s).radius_m;
                s.reaches(widened(region, self.voxel_size), radius_m)
            })
            .filter_map(|s| self.lay_out(&s))
            .collect()
    }

    fn region_m(&self, [min, max]: [[i32; 2]; 2]) -> [[f32; 2]; 2] {
        [min.map(|g| self.global_m(g)), max.map(|g| self.global_m(g))]
    }

    fn settlement_of(&self, site: &Site) -> &Settlement {
        let pass = self.settlement_pass().expect("a Site has a pass");
        &pass.settlements[usize::from(pass.rows[site.row].entry.settlement)]
    }

    /// The Sites of every row whose disc and margin reach `region`, in metres, in claim order. A
    /// later row's Site gives way to an earlier row's it would overlap.
    fn sites_near(&self, region: [[f32; 2]; 2]) -> Vec<Site> {
        let Some(pass) = self.settlement_pass() else {
            return Vec::new();
        };
        let mut standing = Vec::new();
        for index in 0..pass.rows.len() {
            for site in self.row_sites(index, region) {
                let around = widened([site.centre_m, site.centre_m], site.reach_m + pass.reach_m);
                let claimed = (0..index).any(|before| {
                    self.row_sites(before, around)
                        .iter()
                        .any(|s| s.overlaps(&site))
                });
                if !claimed {
                    standing.push(site);
                }
            }
        }
        standing
    }

    /// The Sites of one row whose disc and margin reach `region`, in metres.
    fn row_sites(&self, index: usize, region: [[f32; 2]; 2]) -> Vec<Site> {
        let Some(pass) = self.settlement_pass() else {
            return Vec::new();
        };
        let row = pass.rows[index];
        let spacing = row.entry.spacing_m;
        let at = |m: f32| (m / spacing).floor() as i32;
        let (low, high) = (
            [0, 1].map(|a| at(region[0][a])),
            [0, 1].map(|a| at(region[1][a])),
        );
        (low[1]..=high[1])
            .flat_map(|j| (low[0]..=high[0]).map(move |i| [i, j]))
            .filter_map(|cell| self.site(index, &row, cell))
            .filter(|s| s.reaches(region, s.reach_m))
            .collect()
    }

    /// The Site a lattice cell holds, if the ground at its centre takes it.
    fn site(&self, index: usize, row: &Row, cell: [i32; 2]) -> Option<Site> {
        let pass = self.settlement_pass()?;
        let settlement = &pass.settlements[usize::from(row.entry.settlement)];
        let hash = hashed(
            self.seed,
            SETTLEMENT_SALT,
            &[
                row.biome.index() as u64,
                row.row,
                cell[0] as u32 as u64,
                cell[1] as u32 as u64,
            ],
        );
        let reach_m = settlement.radius_m + settlement.margin_m;
        let mut centre_m = [0.0; 2];
        for axis in 0..2 {
            let from = cell[axis] as f32 * row.entry.spacing_m;
            let low = (from + reach_m).ceil();
            let high = (from + row.entry.spacing_m - reach_m).floor();
            if high < low {
                return None;
            }
            let free = (high - low) as u64;
            centre_m[axis] = low + (mix(hash ^ (3 + axis as u64)) % (free + 1)) as f32;
        }
        let [x_m, z_m] = centre_m;
        let biome = self.biome_at(x_m, z_m)?;
        if pass.listed[biome.index()] & 1 << row.entry.settlement == 0 {
            return None;
        }
        let height_m = self.surface_height_m(x_m, z_m);
        if self.water.is_some_and(|w| height_m < w.level_m) {
            return None;
        }
        let facing = (mix(hash ^ 2) & 3) as u8;
        let centre = match &settlement.centre {
            Some(building) => {
                let front = [0, 1].map(|a| {
                    centre_m[a] - FACINGS[usize::from(facing)][a] * (ROAD_HALF_M + APRON_M)
                });
                let (structure, _) = self.plot(building, facing, front)?;
                Some(structure)
            }
            None if self.cave_may_cut(x_m, z_m, height_m) => return None,
            None => None,
        };
        Some(Site {
            row: index,
            hash,
            centre_m,
            reach_m,
            facing,
            centre,
        })
    }

    /// The Structure `building` stands as on a Plot, turned so its door faces `facing` from
    /// `door_m`, the point before its door, if the ground there takes it; and the rectangle it
    /// covers, in metres.
    fn plot(
        &self,
        building: &Building,
        facing: u8,
        door_m: [f32; 2],
    ) -> Option<(Structure, [[f32; 2]; 2])> {
        let (turn, corner_m, size_m) = self.turned(building, facing, door_m);
        let rect = [corner_m, [0, 1].map(|a| corner_m[a] + size_m[a])];
        let template = &self.templates[usize::from(building.template)];
        let height_m = self.plot_ground(template, corner_m, size_m)?;
        let sampled = template.sample(self.voxel_size, turn);
        let size = sampled.size();
        let [x, z] = corner_m.map(|m| (m / self.voxel_size).round() as i32);
        let y = top_index(height_m, 0.0, self.voxel_size) - sampled.ground();
        Some((
            Structure {
                template: building.template,
                turn,
                min: [x, y, z],
                size,
            },
            rect,
        ))
    }

    /// The turn that faces `building`'s door to `facing`, and the whole-metre corner and size in
    /// metres it then has with its door's front edge at `door_m`.
    fn turned(
        &self,
        building: &Building,
        facing: u8,
        door_m: [f32; 2],
    ) -> (Turn, [f32; 2], [f32; 2]) {
        let template = &self.templates[usize::from(building.template)];
        let turn = Turn::new(facing + 4 - door_side(template, building.door));
        let [w_m, _, d_m] = template.size_m();
        let cell_m = template.cell_m();
        let mut point = match building.door {
            Some([x, z]) => [(f32::from(x) + 0.5) * cell_m, (f32::from(z) + 0.5) * cell_m],
            None => [w_m / 2.0, d_m],
        };
        let mut size_m = [w_m, d_m];
        for _ in 0..turn.quarters() {
            point = [point[1], size_m[0] - point[0]];
            size_m = [size_m[1], size_m[0]];
        }
        // The door's front is on the edge it faces.
        match facing {
            0 => point[1] = size_m[1],
            1 => point[0] = size_m[0],
            2 => point[1] = 0.0,
            _ => point[0] = 0.0,
        }
        let corner_m = [0, 1].map(|a| (door_m[a] - point[a]).round());
        (turn, corner_m, size_m)
    }

    /// The height a Plot's Structure stands at, the lower median of its footprint's ground, if the
    /// ground takes it: dry, level to within the Template's tolerance and no cave in its top metre.
    fn plot_ground(
        &self,
        template: &Template,
        corner_m: [f32; 2],
        size_m: [f32; 2],
    ) -> Option<f32> {
        let along = |a: usize| {
            (0..)
                .map(|k| k as f32 * SAMPLE_M)
                .take_while(move |&k_m| k_m < size_m[a])
                .map(move |k_m| corner_m[a] + k_m)
        };
        let mut heights = Vec::new();
        let (mut lowest_m, mut highest_m) = (f32::MAX, f32::MIN);
        for z_m in along(1) {
            for x_m in along(0) {
                let height_m = self.surface_height_m(x_m, z_m);
                if self.water.is_some_and(|w| height_m < w.level_m) {
                    return None;
                }
                (lowest_m, highest_m) = (lowest_m.min(height_m), highest_m.max(height_m));
                if highest_m - lowest_m > template.level_tolerance_m() {
                    return None;
                }
                heights.push([x_m, z_m, height_m]);
            }
        }
        if heights
            .iter()
            .any(|&[x_m, z_m, height_m]| self.cave_may_cut(x_m, z_m, height_m))
        {
            return None;
        }
        let mut sorted: Vec<f32> = heights.iter().map(|c| c[2]).collect();
        sorted.sort_by(f32::total_cmp);
        Some(sorted[(sorted.len() - 1) / 2])
    }

    /// The Settlement a Site grows to: roads branched from its centre, then Plots beside them.
    /// `None` when fewer Plots stand than it needs.
    fn lay_out(&self, site: &Site) -> Option<Town> {
        let settlement = self.settlement_of(site);
        let radius_m = settlement.radius_m;
        let centre = site.centre_m;
        let mut lots = Vec::new();
        let mut plots = Vec::new();
        if let Some(s) = site.centre {
            let rect = [
                [0, 2].map(|a| s.min[a] as f32 * self.voxel_size),
                [0, 2].map(|a| (s.min[a] + s.size[a]) as f32 * self.voxel_size),
            ];
            lots.push(Lot {
                kept: widened(rect, APRON_M),
            });
            plots.push(s);
        }
        let roads = self.roads(site, radius_m, &lots);
        let reach = radius_m.ceil() as i32;
        let origin_m = centre.map(|c| c as i32 - reach);
        let side = 2 * reach + 1;
        let mut town = Town {
            centre_m: centre,
            radius_m,
            path: settlement.path,
            origin_m,
            side,
            road: vec![false; (side * side) as usize],
            plots,
        };
        for road in &roads {
            for pair in road.windows(2) {
                self.rasterise(&mut town, pair[0], pair[1], &lots);
            }
        }

        let mins: usize = settlement.rules.iter().map(|r| usize::from(r.min)).sum();
        let queue = self.plot_queue(settlement, site.hash);
        let mut next = 0;
        let lengths: Vec<f32> = roads.iter().map(|r| length(r)).collect();
        let longest = lengths.iter().copied().fold(0.0, f32::max);
        let mut along = SLOT_M;
        'slots: while along <= longest {
            for (road, &len) in roads.iter().zip(&lengths) {
                if along > len {
                    continue;
                }
                let (point, heading) = point_along(road, along);
                for side in [1.0, -1.0] {
                    let Some(&rule) = queue.get(next) else {
                        break 'slots;
                    };
                    let normal = [-heading[1] * side, heading[0] * side];
                    // The door faces back across the road's side to its line.
                    let facing = (0..4u8)
                        .max_by(|&a, &b| {
                            let dot = |q: u8| {
                                let f = FACINGS[usize::from(q)];
                                -(f[0] * normal[0] + f[1] * normal[1])
                            };
                            dot(a).total_cmp(&dot(b))
                        })
                        .unwrap_or_default();
                    let step = FACINGS[usize::from(facing)];
                    let door_m = [0, 1].map(|a| point[a] - step[a] * (ROAD_HALF_M + APRON_M));
                    let building = &settlement.rules[rule].building;
                    let (_, corner_m, size_m) = self.turned(building, facing, door_m);
                    let rect = [corner_m, [0, 1].map(|a| corner_m[a] + size_m[a])];
                    let kept = widened(rect, APRON_M);
                    let inside = [
                        kept[0],
                        kept[1],
                        [kept[0][0], kept[1][1]],
                        [kept[1][0], kept[0][1]],
                    ]
                    .iter()
                    .all(|p| {
                        let [dx, dz] = [0, 1].map(|a| p[a] - centre[a]);
                        dx * dx + dz * dz <= radius_m * radius_m
                    });
                    if !inside
                        || lots.iter().any(|l: &Lot| overlap(l.kept, kept))
                        || town.any_road(kept)
                    {
                        continue;
                    }
                    match self.plot(building, facing, door_m) {
                        Some((s, _)) => {
                            lots.push(Lot { kept });
                            town.plots.push(s);
                            next += 1;
                        }
                        // The Plot stays empty; a Template still short of its minimum tries again.
                        None if next >= mins => next += 1,
                        None => {}
                    }
                }
            }
            along += SLOT_M;
        }
        let placed = town.plots.len() - usize::from(site.centre.is_some());
        (next >= mins && placed >= usize::from(settlement.minimum)).then_some(town)
    }

    /// Which rule each Plot draws, in the order they are tried: every minimum first, then weighted
    /// draws within each maximum.
    fn plot_queue(&self, settlement: &Settlement, hash: u64) -> Vec<usize> {
        let mut queue = Vec::new();
        let mut counts = vec![0u8; settlement.rules.len()];
        for (i, rule) in settlement.rules.iter().enumerate() {
            queue.extend(std::iter::repeat_n(i, usize::from(rule.min)));
            counts[i] = rule.min;
        }
        while queue.len() < usize::from(settlement.plots) {
            let open: Vec<usize> = (0..settlement.rules.len())
                .filter(|&i| counts[i] < settlement.rules[i].max)
                .collect();
            let Some(&last) = open.last() else {
                break;
            };
            let total: f32 = open.iter().map(|&i| settlement.rules[i].weight).sum();
            let mut draw = unit(mix(hash ^ (0x100 + queue.len() as u64))) as f32 * total;
            let pick = open
                .iter()
                .copied()
                .find(|&i| {
                    draw -= settlement.rules[i].weight;
                    draw < 0.0
                })
                .unwrap_or(last);
            queue.push(pick);
            counts[pick] += 1;
        }
        queue
    }

    /// Each road from the centre: points [`ROAD_STEP_M`] apart, each step the gentlest of its
    /// bends that stays in the disc, dry, clear of `lots` and no steeper than a road climbs.
    fn roads(&self, site: &Site, radius_m: f32, lots: &[Lot]) -> Vec<Vec<[f32; 2]>> {
        let centre = site.centre_m;
        let count = 3 + (mix(site.hash ^ 5) % 2) as usize;
        let first = FACINGS[usize::from(site.facing)];
        let base = first[1].atan2(first[0]);
        let mut roads = Vec::with_capacity(count);
        for branch in 0..count {
            let jitter = if branch == 0 {
                0.0
            } else {
                (unit(mix(site.hash ^ (16 + branch as u64))) as f32 - 0.5) * 0.5 * TAU
                    / count as f32
            };
            let mut heading = base + branch as f32 * TAU / count as f32 + jitter;
            let mut point = centre;
            let mut height_m = self.surface_height_m(point[0], point[1]);
            let mut road = vec![point];
            loop {
                let mut best: Option<(f32, f32, [f32; 2], f32)> = None;
                for (k, bend) in ROAD_BENDS.into_iter().enumerate() {
                    let angle = heading + bend;
                    let next = [
                        point[0] + ROAD_STEP_M * angle.cos(),
                        point[1] + ROAD_STEP_M * angle.sin(),
                    ];
                    let [dx, dz] = [0, 1].map(|a| next[a] - centre[a]);
                    if (dx * dx + dz * dz).sqrt() > radius_m - ROAD_HALF_M
                        || lots
                            .iter()
                            .any(|l| overlap(widened(l.kept, ROAD_HALF_M), [next, next]))
                    {
                        continue;
                    }
                    let next_m = self.surface_height_m(next[0], next[1]);
                    if self.water.is_some_and(|w| next_m < w.level_m) {
                        continue;
                    }
                    let grade = (next_m - height_m).abs() / ROAD_STEP_M;
                    if grade > MAX_ROAD_GRADE {
                        continue;
                    }
                    let cost = grade + BEND_COST * k as f32;
                    if best.is_none_or(|b| cost < b.0) {
                        best = Some((cost, angle, next, next_m));
                    }
                }
                let Some((_, angle, next, next_m)) = best else {
                    break;
                };
                (heading, point, height_m) = (angle, next, next_m);
                road.push(point);
            }
            roads.push(road);
        }
        roads
    }

    /// Marks the cells of the disc whose centre is within [`ROAD_HALF_M`] of the segment `a..b`,
    /// outside every one of `lots`.
    fn rasterise(&self, town: &mut Town, a: [f32; 2], b: [f32; 2], lots: &[Lot]) {
        let low = [0, 1].map(|i| (a[i].min(b[i]) - ROAD_HALF_M).floor() as i32);
        let high = [0, 1].map(|i| (a[i].max(b[i]) + ROAD_HALF_M).ceil() as i32);
        let [ex, ez] = [b[0] - a[0], b[1] - a[1]];
        let len2 = ex * ex + ez * ez;
        for z in low[1]..=high[1] {
            for x in low[0]..=high[0] {
                let p = [x as f32 + 0.5, z as f32 + 0.5];
                let t = (((p[0] - a[0]) * ex + (p[1] - a[1]) * ez) / len2).clamp(0.0, 1.0);
                let [dx, dz] = [p[0] - a[0] - t * ex, p[1] - a[1] - t * ez];
                let [cx, cz] = [0, 1].map(|i| p[i] - town.centre_m[i]);
                if dx * dx + dz * dz > ROAD_HALF_M * ROAD_HALF_M
                    || cx * cx + cz * cz > town.radius_m * town.radius_m
                    || lots.iter().any(|l| overlap(l.kept, [p, p]))
                {
                    continue;
                }
                let [i, j] = [x - town.origin_m[0], z - town.origin_m[1]];
                if (0..town.side).contains(&i) && (0..town.side).contains(&j) {
                    town.road[(j * town.side + i) as usize] = true;
                }
            }
        }
    }

    /// Writes a Settlement's roads and Plots that fall in the Chunk at `coord`.
    pub(super) fn stand_town(&self, chunk: &mut Chunk, coord: [i32; 3], town: &Town) {
        let size = CHUNK_SIZE as i32;
        let origin = coord.map(|c| c * size);
        let inside = |y: i32| (origin[1]..origin[1] + size).contains(&y);
        let half = self.voxel_size / 2.0;
        for gz in origin[2]..origin[2] + size {
            for gx in origin[0]..origin[0] + size {
                let (x_m, z_m) = (self.global_m(gx), self.global_m(gz));
                let cell = [(x_m + half).floor() as i32, (z_m + half).floor() as i32];
                if !town.road_at(cell) {
                    continue;
                }
                let height_m = self.surface_height_m(x_m, z_m);
                if self.water.is_some_and(|w| height_m < w.level_m) {
                    continue;
                }
                let top = top_index(height_m, 0.0, self.voxel_size);
                let [lx, lz] = [gx - origin[0], gz - origin[2]].map(|l| l as usize);
                if inside(top) {
                    chunk.set(lx, (top - origin[1]) as usize, lz, town.path);
                }
                // A smoothing stair on the road would trip whoever walks it.
                if inside(top + 1) {
                    chunk.set(lx, (top + 1 - origin[1]) as usize, lz, Voxel::AIR);
                }
            }
        }
        for s in &town.plots {
            self.stand_structure(chunk, coord, s);
        }
    }
}

impl Town {
    /// Whether a road cell's centre lies in `rect`, in metres.
    fn any_road(&self, [min, max]: [[f32; 2]; 2]) -> bool {
        let (low, high) = (min.map(|m| m.floor() as i32), max.map(|m| m.ceil() as i32));
        (low[1]..high[1]).any(|z| {
            (low[0]..high[0]).any(|x| {
                let p = [x as f32 + 0.5, z as f32 + 0.5];
                (0..2).all(|a| min[a] < p[a] && p[a] < max[a]) && self.road_at([x, z])
            })
        })
    }
}

fn length(road: &[[f32; 2]]) -> f32 {
    road.windows(2)
        .map(|p| ((p[1][0] - p[0][0]).powi(2) + (p[1][1] - p[0][1]).powi(2)).sqrt())
        .sum()
}

/// The point `along` metres down a road, and the unit heading of its segment there.
fn point_along(road: &[[f32; 2]], mut along: f32) -> ([f32; 2], [f32; 2]) {
    for p in road.windows(2) {
        let [dx, dz] = [p[1][0] - p[0][0], p[1][1] - p[0][1]];
        let len = (dx * dx + dz * dz).sqrt();
        let heading = [dx / len, dz / len];
        if along <= len {
            return (
                [p[0][0] + heading[0] * along, p[0][1] + heading[1] * along],
                heading,
            );
        }
        along -= len;
    }
    (road[road.len() - 1], [1.0, 0.0])
}
