//! Settlements: roads branched from a centre and Plots beside them, each laid out from the seed
//! and the Settlement's lattice cell alone (ADR-0029).
//!
//! The layout works in metres, every Plot's corner on whole metres, so it is the same at every
//! Voxel size; every Chunk a Settlement reaches takes the same layout and writes only its own
//! Voxels.
//! A Settlement claims ground before single Structures, fields, trees and scatter, and never asks
//! about any of them.

use std::f32::consts::TAU;
use std::sync::Arc;

use super::fields::{hashed, mix, unit};
use super::ground::SAMPLE_M;
use super::structures::{NEIGHBOURS, ORIGIN_CLEAR_M, Structure};
use super::{CARDINALS, Footprint, Worldgen, top_index};
use crate::chunk::{CHUNK_SIZE, Chunk};
use crate::climate::{BIOMES_SINCE, BiomeId, BiomeSettlement};
use crate::mesh::POS_Y;
use crate::settlement::{Building, Settlement};
use crate::shape::{Orientation, SHAPE_SLAB, SHAPE_STAIR};
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
/// The gaps, in metres, between the places along a road a Plot is tried, drawn between these.
const SLOT_GAP_M: (f32, f32) = (1.0, 4.0);
/// The most a Plot stands back from its road past its apron.
const SETBACK_M: f32 = 2.0;
/// The clearance a Plot keeps past its apron from another, at the disc's edge; none at the
/// centre, so buildings stand further apart outwards.
const SPREAD_M: f32 = 4.0;
/// A road's half width at the disc's edge, narrowing from [`ROAD_HALF_M`] at the centre.
const EDGE_HALF_M: f32 = 1.0;
/// An approach path's half width past the disc.
const APPROACH_HALF_M: f32 = 0.9;
/// How far an approach path runs past the disc as a trail, drawn between these: past the margin,
/// over ground the Settlement does not claim, stopping short of another Settlement's.
const TRAIL_M: (f32, f32) = (40.0, 90.0);
/// The share of a road's cells worn to the Biome's surface at the disc's edge; none inside half
/// the radius, and all at an approach path's end.
const EDGE_WEAR: f32 = 0.25;
const WEAR_SALT: u64 = 0x5e77_1e00_0000_0002;
/// The most a Plot or a road is cut into or filled over the ground.
const CUT_FILL_M: f32 = 2.0;
/// What the ground rules allow at a Plot's sampled columns, short of [`CUT_FILL_M`] so a column
/// between two samples still keeps within it.
const SAMPLED_CUT_FILL_M: f32 = 1.5;
/// A road forks every this many metres out from the centre, both ways across it, where at least
/// [`FORK_ROOM_M`] of the disc is left past the fork: never in a disc of radius under 66 m.
const FORK_EVERY_M: f32 = 36.0;
const FORK_ROOM_M: f32 = 30.0;
/// The share of the ring a fork's distance traces between two roads from the centre that a fork
/// road runs, drawn between these.
const FORK_LENGTH: (f32, f32) = (0.35, 0.55);
/// The most a road from the centre runs, as a multiple of the furthest it may reach.
const WANDER: f32 = 8.0;
/// Of the two ways a road may fork at each distance, the share that do.
const FORK_CHANCE: f64 = 0.75;
/// How far a fork road turns off square to its road, either way, at most.
const FORK_JITTER: f32 = 0.25;

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
    pub(super) settlements: Arc<[Settlement]>,
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
                    above = above.max(sampled.size()[1] - sampled.ground_top());
                    below = below.max(sampled.ground_top());
                }
            }
        }
        if rows.is_empty() {
            return None;
        }
        // Levelling and grading lift and sink the ground, and a Plot with it.
        let lift = (CUT_FILL_M / voxel_size).ceil() as i32 + 1;
        Some(SettlementPass {
            settlements: settlements.into(),
            rows,
            listed,
            above: above + lift,
            below: below + lift,
            reach_m,
        })
    }
}

/// A lattice cell's Settlement whose centre the ground takes, before its roads and Plots are
/// laid out.
#[derive(Clone, Copy, Debug)]
pub(super) struct Founding {
    row: usize,
    cell: [i32; 2],
    hash: u64,
    /// On whole metres.
    centre_m: [f32; 2],
    /// The disc and the most Plots this copy draws.
    radius_m: f32,
    plots: u8,
    /// Its disc and margin, in metres.
    reach_m: f32,
    /// The way its first road leaves, and its centre's door faces.
    facing: u8,
    centre: Option<(Structure, Plot)>,
}

impl Founding {
    fn overlaps(&self, other: &Founding) -> bool {
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
pub(super) struct LaidSettlement {
    centre_m: [f32; 2],
    radius_m: f32,
    path: Voxel,
    /// The metre cell at the corner of the square the disc fits in.
    origin_m: [i32; 2],
    side: i32,
    road: Vec<Lane>,
    /// The height, in metres, a road is graded to at each cell's centre.
    grade: Vec<f32>,
    /// An index into the World's Settlements.
    pub(super) settlement: u16,
    /// The centre's first, if it has one.
    pub(super) structures: Vec<Structure>,
    /// The rule each of [`Self::structures`] was drawn by; the centre's is `None`.
    pub(super) rules: Vec<Option<usize>>,
    /// Each one's Plot, levelled; by [`Self::structures`].
    plots: Vec<Plot>,
    /// The points its approach paths step through past the disc.
    approach: Vec<[f32; 2]>,
}

/// What a metre cell of a Settlement's square holds.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Lane {
    Open,
    /// Laid in the Settlement's path Material.
    Laid,
    /// Graded like a road, but showing the Biome's surface.
    Worn,
}

impl LaidSettlement {
    fn lane_at(&self, [x, z]: [i32; 2]) -> Lane {
        let [i, j] = [x - self.origin_m[0], z - self.origin_m[1]];
        if (0..self.side).contains(&i) && (0..self.side).contains(&j) {
            self.road[(j * self.side + i) as usize]
        } else {
            Lane::Open
        }
    }

    fn road_at(&self, at: [i32; 2]) -> bool {
        self.lane_at(at) != Lane::Open
    }

    /// Its disc and the ground along its approach paths, which fields, trees and scatter keep
    /// off.
    pub(super) fn footprints(&self, voxel_size: f32) -> impl Iterator<Item = Footprint> + '_ {
        let around = ROAD_STEP_M / 2.0 + APPROACH_HALF_M + 0.5;
        std::iter::once((self.centre_m, self.radius_m))
            .chain(self.approach.iter().map(move |&p| (p, around)))
            .map(move |(centre_m, radius_m)| {
                let centre = centre_m.map(|c| c / voxel_size);
                let radius = radius_m / voxel_size;
                Footprint {
                    min: centre.map(|c| (c - radius).floor() as i32),
                    max: centre.map(|c| (c + radius).ceil() as i32),
                    disc: Some((centre, radius)),
                }
            })
    }
}

/// A road from a Settlement's centre: its points [`ROAD_STEP_M`] apart, and the furthest any is
/// from the centre.
struct Road {
    points: Vec<[f32; 2]>,
    end_m: f32,
}

/// Where a road may step: within `limit_m` of `centre`, and only outwards past `inner_m`.
#[derive(Clone, Copy)]
struct Reach {
    centre: [f32; 2],
    inner_m: f32,
    limit_m: f32,
}

/// A place along a road a Plot is tried.
struct Slot {
    key: u64,
    road: usize,
    along: f32,
    /// Which side of the road, as `1` or `-1`.
    side: f32,
    setback_m: f32,
}

/// How far from its centre a Settlement's roads and trails may reach.
fn paths_m(radius_m: f32) -> f32 {
    radius_m + TRAIL_M.1 + APPROACH_HALF_M
}

fn lerp((low, high): (f32, f32), t: f32) -> f32 {
    low + (high - low) * t
}

/// A Plot taken, as rectangles in metres `[min, max)`: its Structure, and that widened by the
/// apron; and the height its ground is levelled to.
#[derive(Clone, Copy, Debug)]
struct Plot {
    rect: [[f32; 2]; 2],
    kept: [[f32; 2]; 2],
    height_m: f32,
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
    pub(super) fn settlement_pass(&self) -> Option<&SettlementPass> {
        self.settlements.as_ref()
    }

    /// Whether a Settlement's disc and margin reach a column of `footprint`, `[min, max)`: what
    /// keeps a single Structure out. Asks only the centres, so it stays cheap per Structure.
    pub(super) fn settled(&self, footprint: [[i32; 2]; 2]) -> bool {
        !self.foundings_near(self.region_m(footprint)).is_empty()
    }

    /// Every Settlement laid out whose disc covers or borders a column of `region`, `[min, max)`.
    pub(super) fn laid_settlements_near(&self, region: [[i32; 2]; 2]) -> Vec<Arc<LaidSettlement>> {
        let region = self.region_m(region);
        self.foundings_within(region, TRAIL_M.1 + APPROACH_HALF_M)
            .into_iter()
            .filter(|s| s.reaches(widened(region, self.voxel_size), paths_m(s.radius_m)))
            .filter_map(|s| self.layouts.laid((s.row, s.cell), || self.lay_out(&s)))
            .collect()
    }

    fn region_m(&self, [min, max]: [[i32; 2]; 2]) -> [[f32; 2]; 2] {
        [min.map(|g| self.global_m(g)), max.map(|g| self.global_m(g))]
    }

    fn settlement_of(&self, founding: &Founding) -> &Settlement {
        let pass = self.settlement_pass().expect("a founding has a pass");
        self.settlement_of_row(&pass.rows[founding.row])
    }

    fn settlement_of_row(&self, row: &Row) -> &Settlement {
        let pass = self.settlement_pass().expect("a row has a pass");
        &pass.settlements[usize::from(row.entry.settlement)]
    }

    /// The foundings of every row whose disc and margin reach `region`, in metres, in claim order. A
    /// later row's founding gives way to an earlier row's it would overlap.
    fn foundings_near(&self, region: [[f32; 2]; 2]) -> Vec<Founding> {
        self.foundings_within(region, 0.0)
    }

    /// [`Self::foundings_near`], for a region `extra_m` past each disc and margin: where trails
    /// run.
    fn foundings_within(&self, region: [[f32; 2]; 2], extra_m: f32) -> Vec<Founding> {
        let Some(pass) = self.settlement_pass() else {
            return Vec::new();
        };
        let mut standing = Vec::new();
        for index in 0..pass.rows.len() {
            for founding in self.row_foundings(index, region, extra_m) {
                let around = widened(
                    [founding.centre_m, founding.centre_m],
                    founding.reach_m + pass.reach_m,
                );
                let claimed = (0..index).any(|before| {
                    self.row_foundings(before, around, 0.0)
                        .iter()
                        .any(|s| s.overlaps(&founding))
                });
                if !claimed {
                    standing.push(founding);
                }
            }
        }
        standing
    }

    /// The foundings of one row whose disc and margin, widened by `extra_m`, reach `region`, in
    /// metres.
    fn row_foundings(&self, index: usize, region: [[f32; 2]; 2], extra_m: f32) -> Vec<Founding> {
        let Some(pass) = self.settlement_pass() else {
            return Vec::new();
        };
        let row = pass.rows[index];
        let spacing = row.entry.spacing_m;
        let at = |m: f32| (m / spacing).floor() as i32;
        let reach_m = self.settlement_of_row(&row).reach_m() + extra_m;
        let (low, high) = (
            [0, 1].map(|a| at(region[0][a] - reach_m)),
            [0, 1].map(|a| at(region[1][a] + reach_m)),
        );
        (low[1]..=high[1])
            .flat_map(|j| (low[0]..=high[0]).map(move |i| [i, j]))
            .filter_map(|cell| {
                self.layouts
                    .founding((index, cell), || self.founding(index, &row, cell))
            })
            .filter(|s| s.reaches(region, s.reach_m + extra_m))
            .collect()
    }

    /// A held cell's candidate centre, on whole metres anywhere in the cell, and its priority
    /// over its neighbours', from hashes alone.
    fn founding_draw(&self, row: &Row, cell: [i32; 2]) -> Option<(u64, [f32; 2], u64)> {
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
        if let Some(centre_m) = self.near_spawn_draw(row, cell, hash) {
            return Some((hash, centre_m, u64::MAX));
        }
        if unit(mix(hash ^ 1)) >= f64::from(row.entry.chance) {
            return None;
        }
        let mut centre_m = [0.0; 2];
        for axis in 0..2 {
            let from = cell[axis] as f32 * row.entry.spacing_m;
            let low = from.ceil();
            let high = (from + row.entry.spacing_m).ceil() - 1.0;
            let free = (high - low).max(0.0) as u64;
            centre_m[axis] = low + (mix(hash ^ (3 + axis as u64)) % (free + 1)) as f32;
        }
        Some((hash, centre_m, mix(hash ^ 6)))
    }

    /// A `near-spawn` row's draw in one of the four cells round the origin: a whole-metre centre
    /// a seeded distance out, from just past the origin's keep-out for the largest copy to the
    /// cell's side, at a seeded angle across the cell's quarter, so it always lies in its cell.
    /// `None` for any other cell, or where the cell is too small to hold one past the keep-out.
    fn near_spawn_draw(&self, row: &Row, cell: [i32; 2], hash: u64) -> Option<[f32; 2]> {
        if !row.entry.near_spawn || !cell.iter().all(|c| (-1..=0).contains(c)) {
            return None;
        }
        let side_m = row.entry.spacing_m;
        // A metre spare each way for the rounding to whole metres.
        let keep_m = self.settlement_of_row(row).reach_m() + ORIGIN_CLEAR_M + 2.0;
        if keep_m >= side_m - 1.0 {
            return None;
        }
        let out_m = keep_m + (side_m - 1.0 - keep_m) * unit(mix(hash ^ 10)) as f32;
        let angle = TAU / 4.0 * unit(mix(hash ^ 11)) as f32;
        let along = [out_m * angle.cos(), out_m * angle.sin()];
        Some([0, 1].map(|a| {
            let m = along[a].round();
            if cell[a] == 0 {
                m.min(side_m - 1.0)
            } else {
                -m.max(1.0)
            }
        }))
    }

    /// The founding a lattice cell holds: its candidate, if no neighbouring cell's candidate with a
    /// higher priority is near enough for their discs and margins to meet, and the ground at its
    /// centre takes it. The neighbours are asked for candidates alone, never the ground, so the
    /// answer is the same from every Chunk; the spacing is at least twice the reach, so only the
    /// eight neighbours can be that near.
    fn founding(&self, index: usize, row: &Row, cell: [i32; 2]) -> Option<Founding> {
        let pass = self.settlement_pass()?;
        let settlement = &pass.settlements[usize::from(row.entry.settlement)];
        let (hash, centre_m, priority) = self.founding_draw(row, cell)?;
        // Neighbours are kept apart at the largest disc, which the lattice's spacing allows for.
        let largest_m = settlement.reach_m();
        let (plots, radius_m) = settlement.size_at(unit(mix(hash ^ 9)));
        let reach_m = radius_m + settlement.margin_m;
        let crowded = NEIGHBOURS.iter().any(|&[di, dj]| {
            let near = [cell[0] + di, cell[1] + dj];
            self.founding_draw(row, near)
                .is_some_and(|(_, other_m, other)| {
                    let [dx, dz] = [0, 1].map(|a| other_m[a] - centre_m[a]);
                    (other, near) > (priority, cell) && (dx * dx + dz * dz).sqrt() < 2.0 * largest_m
                })
        });
        if crowded || distance(centre_m, [0.0, 0.0]) < reach_m + ORIGIN_CLEAR_M {
            return None;
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
                Some(self.plot(building, facing, front)?)
            }
            None if self.cave_may_cut(x_m, z_m, height_m) => return None,
            None => None,
        };
        Some(Founding {
            row: index,
            cell,
            hash,
            centre_m,
            radius_m,
            plots,
            reach_m,
            facing,
            centre,
        })
    }

    /// The Structure `building` stands as on a Plot, turned so its door faces `facing` from
    /// `door_m`, the point before its door, if the ground there takes it; and its Plot.
    fn plot(&self, building: &Building, facing: u8, door_m: [f32; 2]) -> Option<(Structure, Plot)> {
        let (turn, corner_m, size_m) = self.turned(building, facing, door_m);
        let rect = [corner_m, [0, 1].map(|a| corner_m[a] + size_m[a])];
        let template = &self.templates[usize::from(building.template)];
        let height_m = self.plot_ground(rect)?;
        let sampled = template.sample(self.voxel_size, turn);
        let size = sampled.size();
        let [x, z] = corner_m.map(|m| (m / self.voxel_size).round() as i32);
        let y = top_index(height_m, 0.0, self.voxel_size) - sampled.ground_top();
        Some((
            Structure {
                template: building.template,
                turn,
                min: [x, y, z],
                size,
            },
            Plot {
                rect,
                kept: widened(rect, APRON_M),
                height_m,
            },
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
        match facing {
            0 => point[1] = size_m[1],
            1 => point[0] = size_m[0],
            2 => point[1] = 0.0,
            _ => point[0] = 0.0,
        }
        let corner_m = [0, 1].map(|a| (door_m[a] - point[a]).round());
        (turn, corner_m, size_m)
    }

    /// The height a Plot's ground is levelled to, the lower median of its footprint's ground, if
    /// the ground takes it: dry under the footprint and its apron, no cave in the footprint's top
    /// metre, and every sampled column within [`SAMPLED_CUT_FILL_M`] of that height.
    fn plot_ground(&self, [min, max]: [[f32; 2]; 2]) -> Option<f32> {
        let along = |a: usize| {
            let from = min[a] - APRON_M;
            (0..)
                .map(move |k| from + k as f32 * SAMPLE_M)
                .take_while(move |&m| m < max[a] + APRON_M)
        };
        let points = along(1).flat_map(|z_m| {
            along(0).map(move |x_m| {
                let under = (min[0]..max[0]).contains(&x_m) && (min[1]..max[1]).contains(&z_m);
                ([x_m, z_m], under)
            })
        });
        let ground = self.sample_ground(points, |_| true)?;
        let level_m = ground.lower_median_m()?;
        if ground
            .columns()
            .any(|c| (c.height_m - level_m).abs() > SAMPLED_CUT_FILL_M)
        {
            return None;
        }
        self.uncut(&ground).then_some(level_m)
    }

    /// The Settlement a founding grows to: roads branched from its centre, then Plots beside them,
    /// tried in a seeded order. `None` when fewer Plots stand than it needs.
    fn lay_out(&self, founding: &Founding) -> Option<LaidSettlement> {
        let settlement = self.settlement_of(founding);
        let radius_m = founding.radius_m;
        let centre = founding.centre_m;
        let (structures, plots): (Vec<_>, Vec<_>) = founding.centre.into_iter().unzip();
        let roads = self.roads(founding, &plots);
        let reach = paths_m(radius_m).ceil() as i32 + 1;
        let origin_m = centre.map(|c| c as i32 - reach);
        let side = 2 * reach + 1;
        let approach = roads
            .iter()
            .flat_map(|r| &r.points)
            .filter(|p| distance(**p, centre) > radius_m)
            .copied()
            .collect();
        let pass = self.settlement_pass().expect("a founding has a pass");
        let mut laid = LaidSettlement {
            centre_m: centre,
            radius_m,
            path: settlement.path,
            origin_m,
            side,
            road: vec![Lane::Open; (side * side) as usize],
            grade: Vec::new(),
            settlement: pass.rows[founding.row].entry.settlement,
            rules: vec![None; structures.len()],
            structures,
            plots,
            approach,
        };
        for road in &roads {
            for pair in road.points.windows(2) {
                self.rasterise(&mut laid, road, pair[0], pair[1]);
            }
        }
        self.grade(&mut laid);

        let mins: usize = settlement.rules.iter().map(|r| usize::from(r.min)).sum();
        let mut queue = self.plot_queue(settlement, founding.plots, founding.hash);
        let ranged = settlement.plots_min != settlement.plots;
        let mut next = 0;
        for slot in slots(founding, &roads) {
            let Some(&rule) = queue.get(next) else {
                break;
            };
            let (point, heading) = point_along(&roads[slot.road].points, slot.along);
            let normal = [-heading[1] * slot.side, heading[0] * slot.side];
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
            let back_m = ROAD_HALF_M + APRON_M + slot.setback_m;
            let door_m = [0, 1].map(|a| point[a] - step[a] * back_m);
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
            .all(|&p| distance(p, centre) <= radius_m);
            let middle = [0, 1].map(|a| (rect[0][a] + rect[1][a]) / 2.0);
            let clear = widened(kept, SPREAD_M * distance(middle, centre) / radius_m);
            if !inside
                || laid.plots.iter().any(|plot| overlap(plot.kept, clear))
                || laid.any_road(kept)
            {
                continue;
            }
            match self.plot(building, facing, door_m) {
                Some((s, plot)) => {
                    laid.plots.push(plot);
                    laid.structures.push(s);
                    laid.rules.push(Some(rule));
                    next += 1;
                }
                // A Template still short of its minimum tries again.
                None if next < mins => {}
                // A ranged copy tries the Template again after the rest, so its size follows its
                // draw on rough ground. A fixed size keeps the layout saved Worlds pin.
                None if ranged => {
                    queue.push(rule);
                    next += 1;
                }
                // The Plot stays empty.
                None => next += 1,
            }
        }
        let placed = laid.structures.len() - usize::from(founding.centre.is_some());
        (next >= mins && placed >= usize::from(settlement.minimum)).then_some(laid)
    }

    /// Which rule each Plot draws, in the order they are tried: every minimum first, then weighted
    /// draws within each maximum.
    fn plot_queue(&self, settlement: &Settlement, plots: u8, hash: u64) -> Vec<usize> {
        let mut queue = Vec::new();
        let mut counts = vec![0u8; settlement.rules.len()];
        for (i, rule) in settlement.rules.iter().enumerate() {
            queue.extend(std::iter::repeat_n(i, usize::from(rule.min)));
            counts[i] = rule.min;
        }
        while queue.len() < usize::from(plots) {
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
    /// bends that stays in the disc, dry, clear of `plots` and no steeper than a road climbs. Two
    /// or three run on past the disc as approach paths, heading only outwards there. In a wide
    /// disc each then forks, after them.
    fn roads(&self, founding: &Founding, plots: &[Plot]) -> Vec<Road> {
        let centre = founding.centre_m;
        let radius_m = founding.radius_m;
        let inner_m = radius_m - ROAD_HALF_M;
        let count = 3 + (mix(founding.hash ^ 5) % 2) as usize;
        let approaches = 2 + (mix(founding.hash ^ 7) % 2) as usize;
        let first_approach = (mix(founding.hash ^ 8) % count as u64) as usize;
        let first = FACINGS[usize::from(founding.facing)];
        let base = first[1].atan2(first[0]);
        // Another Settlement's disc and margin, which a road stops short of.
        let others: Vec<([f32; 2], f32)> = self
            .foundings_near(widened([centre, centre], paths_m(radius_m)))
            .into_iter()
            .filter(|f| (f.row, f.cell) != (founding.row, founding.cell))
            .map(|f| (f.centre_m, f.reach_m + APPROACH_HALF_M))
            // Nor into the origin's keep-out, where Games build their spawn.
            .chain([([0.0; 2], ORIGIN_CLEAR_M + APPROACH_HALF_M)])
            .collect();
        let mut roads = Vec::with_capacity(count);
        for branch in 0..count {
            let jitter = if branch == 0 {
                0.0
            } else {
                (unit(mix(founding.hash ^ (16 + branch as u64))) as f32 - 0.5) * 0.5 * TAU
                    / count as f32
            };
            let heading = base + branch as f32 * TAU / count as f32 + jitter;
            let limit_m = if (branch + count - first_approach) % count < approaches {
                radius_m
                    + lerp(
                        TRAIL_M,
                        unit(mix(founding.hash ^ (32 + branch as u64))) as f32,
                    )
            } else {
                inner_m
            }
            .max(inner_m);
            let reach = Reach {
                centre,
                inner_m,
                limit_m,
            };
            // A road hemmed in by hills could otherwise wander round inside the disc for
            // thousands of steps.
            let steps = (WANDER * limit_m / ROAD_STEP_M).ceil() as usize;
            roads.push(self.walk(centre, heading, reach, steps, (plots, &others)));
        }
        let rings = ((radius_m - FORK_ROOM_M) / FORK_EVERY_M).floor().max(0.0) as usize;
        for spoke in 0..count {
            for ring in 1..=rings {
                let out_m = ring as f32 * FORK_EVERY_M;
                let points = &roads[spoke].points;
                let Some(at) = points.iter().position(|&p| distance(p, centre) >= out_m) else {
                    continue;
                };
                let (from, ahead) = (points[at.saturating_sub(1)], points[at]);
                let along = (ahead[1] - from[1]).atan2(ahead[0] - from[0]);
                for (side, turn) in [-1.0f32, 1.0].into_iter().enumerate() {
                    let key =
                        mix(founding.hash ^ (0x300 + ((spoke * 16 + ring) * 2 + side) as u64));
                    if unit(mix(key ^ 1)) >= FORK_CHANCE {
                        continue;
                    }
                    let skew = (unit(mix(key ^ 2)) as f32 - 0.5) * 2.0 * FORK_JITTER;
                    let length_m =
                        lerp(FORK_LENGTH, unit(mix(key ^ 3)) as f32) * out_m * TAU / count as f32;
                    let steps = (length_m / ROAD_STEP_M).ceil() as usize;
                    let reach = Reach {
                        centre,
                        inner_m,
                        limit_m: inner_m,
                    };
                    let fork = self.walk(
                        ahead,
                        along + turn * (TAU / 4.0 + skew),
                        reach,
                        steps,
                        (plots, &others),
                    );
                    if fork.points.len() > 1 {
                        roads.push(fork);
                    }
                }
            }
        }
        roads
    }

    /// A road from `start` heading `heading`, at most `steps` steps, each the gentlest of its
    /// bends that stays in `reach`, dry, clear of `plots` and of `others`' discs and margins, and
    /// no steeper than a road climbs.
    fn walk(
        &self,
        start: [f32; 2],
        mut heading: f32,
        reach: Reach,
        steps: usize,
        (plots, others): (&[Plot], &[([f32; 2], f32)]),
    ) -> Road {
        let Reach {
            centre,
            inner_m,
            limit_m,
        } = reach;
        let mut point = start;
        let mut height_m = self.surface_height_m(point[0], point[1]);
        let mut road = vec![point];
        while road.len() <= steps {
            let mut best: Option<(f32, f32, [f32; 2], f32)> = None;
            for (k, bend) in ROAD_BENDS.into_iter().enumerate() {
                let angle = heading + bend;
                let next = [
                    point[0] + ROAD_STEP_M * angle.cos(),
                    point[1] + ROAD_STEP_M * angle.sin(),
                ];
                let out_m = distance(next, centre);
                if out_m > limit_m
                    || (out_m > inner_m && out_m <= distance(point, centre))
                    || plots
                        .iter()
                        .any(|plot| overlap(widened(plot.kept, ROAD_HALF_M), [next, next]))
                    || others
                        .iter()
                        .any(|&(c, reach_m)| distance(next, c) < reach_m)
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
        let end_m = road
            .iter()
            .map(|&p| distance(p, centre))
            .fold(0.0, f32::max);
        Road {
            points: road,
            end_m,
        }
    }

    /// Marks the cells whose centre is near the segment `a..b` of `road`, outside every one of
    /// `plots`: within [`ROAD_HALF_M`] at the centre, narrowing to [`EDGE_HALF_M`] at the disc's
    /// edge and [`APPROACH_HALF_M`] past it, out along a trail. Cells towards the edge wear, by a
    /// draw of their own, more often the further out, so a trail fades into the ground at its end.
    fn rasterise(&self, laid: &mut LaidSettlement, road: &Road, a: [f32; 2], b: [f32; 2]) {
        let radius_m = laid.radius_m;
        let low = [0, 1].map(|i| (a[i].min(b[i]) - ROAD_HALF_M).floor() as i32);
        let high = [0, 1].map(|i| (a[i].max(b[i]) + ROAD_HALF_M).ceil() as i32);
        let [ex, ez] = [b[0] - a[0], b[1] - a[1]];
        let len2 = ex * ex + ez * ez;
        for z in low[1]..=high[1] {
            for x in low[0]..=high[0] {
                let p = [x as f32 + 0.5, z as f32 + 0.5];
                let t = (((p[0] - a[0]) * ex + (p[1] - a[1]) * ez) / len2).clamp(0.0, 1.0);
                let [dx, dz] = [p[0] - a[0] - t * ex, p[1] - a[1] - t * ez];
                let out_m = distance(p, laid.centre_m);
                let (half_m, wear) = if out_m <= radius_m {
                    let edge = out_m / radius_m;
                    let half_m = lerp((ROAD_HALF_M, EDGE_HALF_M), edge);
                    (half_m, EDGE_WEAR * (2.0 * edge - 1.0).clamp(0.0, 1.0))
                } else {
                    let end = ((out_m - radius_m) / (road.end_m - radius_m)).clamp(0.0, 1.0);
                    (APPROACH_HALF_M, lerp((EDGE_WEAR, 1.0), end * end))
                };
                if dx * dx + dz * dz > half_m * half_m
                    || laid.plots.iter().any(|plot| overlap(plot.kept, [p, p]))
                {
                    continue;
                }
                let [i, j] = [x - laid.origin_m[0], z - laid.origin_m[1]];
                if !((0..laid.side).contains(&i) && (0..laid.side).contains(&j)) {
                    continue;
                }
                let worn = wear > 0.0
                    && unit(hashed(
                        self.seed,
                        WEAR_SALT,
                        &[x as u32 as u64, z as u32 as u64],
                    )) < f64::from(wear);
                let cell = &mut laid.road[(j * laid.side + i) as usize];
                if *cell != Lane::Laid {
                    *cell = if worn { Lane::Worn } else { Lane::Laid };
                }
            }
        }
    }

    /// Grades the roads: the height at each cell's centre midway between the highest surface no
    /// steeper than [`MAX_ROAD_GRADE`] under the road's ground and the lowest over it, so it never
    /// climbs faster than that between any two points. A road cell that would be cut or filled by
    /// more than [`CUT_FILL_M`] is not laid.
    fn grade(&self, laid: &mut LaidSettlement) {
        let side = laid.side as usize;
        let at =
            |i: usize, j: usize| [0, 1].map(|a| laid.origin_m[a] as f32 + [i, j][a] as f32 + 0.5);
        let natural: Vec<f32> = (0..side * side)
            .map(|c| {
                let [x, z] = at(c % side, c / side);
                if laid.road[c] != Lane::Open {
                    self.surface_height_m(x, z)
                } else {
                    f32::NAN
                }
            })
            .collect();
        let mut over: Vec<f32> = natural
            .iter()
            .map(|h| if h.is_nan() { f32::MAX } else { *h })
            .collect();
        let mut under: Vec<f32> = natural
            .iter()
            .map(|h| if h.is_nan() { f32::MIN } else { *h })
            .collect();
        let step = MAX_ROAD_GRADE * SAMPLE_M;
        let mut relax = |c: usize, n: usize| {
            over[c] = over[c].min(over[n] + step);
            under[c] = under[c].max(under[n] - step);
        };
        for c in 0..side * side {
            if c % side > 0 {
                relax(c, c - 1);
            }
            if c >= side {
                relax(c, c - side);
            }
        }
        for c in (0..side * side).rev() {
            if c % side + 1 < side {
                relax(c, c + 1);
            }
            if c + side < side * side {
                relax(c, c + side);
            }
        }
        laid.grade = over
            .iter()
            .zip(&under)
            .map(|(o, u)| (o + u) / 2.0)
            .collect();
        for (c, h) in natural.iter().enumerate() {
            if laid.road[c] != Lane::Open && (laid.grade[c] - h).abs() > CUT_FILL_M {
                laid.road[c] = Lane::Open;
            }
        }
    }

    /// Writes a Settlement that falls in the Chunk at `coord`: each Plot's ground levelled, then
    /// its roads graded, then its Plots' Structures.
    pub(super) fn stand_settlement(
        &self,
        chunk: &mut Chunk,
        coord: [i32; 3],
        laid: &LaidSettlement,
    ) {
        let size = CHUNK_SIZE as i32;
        let origin = coord.map(|c| c * size);
        let half = self.voxel_size / 2.0;
        for plot in &laid.plots {
            let [min, max] = widened(plot.rect, APRON_M);
            let low =
                [0, 1].map(|a| ((min[a] / self.voxel_size).floor() as i32).max(origin[a * 2]));
            let high = [0, 1]
                .map(|a| ((max[a] / self.voxel_size).ceil() as i32).min(origin[a * 2] + size));
            for gz in low[1]..high[1] {
                for gx in low[0]..high[0] {
                    let (x_m, z_m) = (self.global_m(gx), self.global_m(gz));
                    let centre = [x_m + half, z_m + half];
                    let away = (0..2)
                        .map(|a| (plot.rect[0][a] - centre[a]).max(centre[a] - plot.rect[1][a]))
                        .fold(0.0f32, f32::max);
                    if away > APRON_M {
                        continue;
                    }
                    let natural_m = self.surface_height_m(x_m, z_m);
                    if self.water.is_some_and(|w| natural_m < w.level_m) {
                        continue;
                    }
                    // The apron eases from the Plot's height to the ground just past it.
                    let blend = away / (APRON_M + SAMPLE_M);
                    let target_m = (plot.height_m + (natural_m - plot.height_m) * blend)
                        .clamp(natural_m - CUT_FILL_M, natural_m + CUT_FILL_M);
                    let Some(ground) = self.column_ground(gx, gz) else {
                        continue;
                    };
                    let top = top_index(target_m, 0.0, self.voxel_size);
                    self.reshape(
                        chunk,
                        origin,
                        [gx, gz],
                        natural_m,
                        top,
                        Voxel::cube(ground.surface),
                    );
                }
            }
        }
        for gz in origin[2]..origin[2] + size {
            for gx in origin[0]..origin[0] + size {
                let (x_m, z_m) = (self.global_m(gx), self.global_m(gz));
                let Some(half_voxels) = self.road_height(laid, [gx, gz]) else {
                    continue;
                };
                let cap = match laid.lane_at(metre_cell(x_m, z_m, half)) {
                    Lane::Worn => match self.column_ground(gx, gz) {
                        Some(ground) => Voxel::cube(ground.surface),
                        None => continue,
                    },
                    _ => laid.path,
                };
                let natural_m = self.surface_height_m(x_m, z_m);
                let top = half_voxels.div_euclid(2) - 1;
                self.reshape(chunk, origin, [gx, gz], natural_m, top, cap);
                if half_voxels % 2 == 0 || !(origin[1]..origin[1] + size).contains(&(top + 1)) {
                    continue;
                }
                let rising: Vec<u8> = CARDINALS
                    .iter()
                    .filter(|&&(dx, dz, _)| {
                        self.road_height(laid, [gx + dx, gz + dz]) == Some(half_voxels + 1)
                    })
                    .map(|c| c.2)
                    .collect();
                let material = cap.material();
                let voxel = match rising[..] {
                    [turn] => Voxel::shaped(material, SHAPE_STAIR, Orientation::new(POS_Y, turn)),
                    _ => Voxel::shaped(material, SHAPE_SLAB, Orientation::IDENTITY),
                };
                let local =
                    [gx - origin[0], top + 1 - origin[1], gz - origin[2]].map(|l| l as usize);
                chunk.set(local[0], local[1], local[2], voxel);
            }
        }
        for s in &laid.structures {
            self.stand_structure(chunk, coord, s, false);
        }
    }

    /// The height, in half Voxels, a road's walked surface is at in the column at `(gx, gz)`:
    /// above the Voxel whose floor is at or under the graded height, as the ground's own top is.
    /// `None` where no road is laid.
    fn road_height(&self, laid: &LaidSettlement, [gx, gz]: [i32; 2]) -> Option<i32> {
        let half = self.voxel_size / 2.0;
        let (x_m, z_m) = (self.global_m(gx), self.global_m(gz));
        if !laid.road_at(metre_cell(x_m, z_m, half)) {
            return None;
        }
        if self
            .water
            .is_some_and(|w| self.surface_height_m(x_m, z_m) < w.level_m)
        {
            return None;
        }
        let graded_m = laid.graded([x_m + half, z_m + half]);
        Some((graded_m / half).floor() as i32 + 2)
    }

    /// Moves the top of the column at `(gx, gz)` from the ground's to Voxel `top`, which becomes
    /// `cap`: soil fills beneath it and air clears above it, through the ground's old top and the
    /// smoothing stair that may stand on it.
    fn reshape(
        &self,
        chunk: &mut Chunk,
        origin: [i32; 3],
        [gx, gz]: [i32; 2],
        natural_m: f32,
        top: i32,
        cap: Voxel,
    ) {
        let size = CHUNK_SIZE as i32;
        let Some(ground) = self.column_ground(gx, gz) else {
            return;
        };
        let natural = top_index(natural_m, 0.0, self.voxel_size);
        let low = natural.min(top).max(origin[1]);
        let high = (natural.max(top) + 1).min(origin[1] + size - 1);
        let [lx, lz] = [gx - origin[0], gz - origin[2]].map(|l| l as usize);
        for y in low..=high {
            let voxel = match y.cmp(&top) {
                std::cmp::Ordering::Less => Voxel::cube(ground.soil),
                std::cmp::Ordering::Equal => cap,
                std::cmp::Ordering::Greater => Voxel::AIR,
            };
            chunk.set(lx, (y - origin[1]) as usize, lz, voxel);
        }
    }
}

impl LaidSettlement {
    /// The graded height at a point, in metres, between the cells' centres.
    fn graded(&self, point: [f32; 2]) -> f32 {
        let side = self.side as usize;
        let [u, v] = [0, 1].map(|a| point[a] - self.origin_m[a] as f32 - 0.5);
        let [i, j] = [u, v].map(|t| (t.floor().max(0.0) as usize).min(side - 2));
        let [s, t] = [u - i as f32, v - j as f32].map(|f| f.clamp(0.0, 1.0));
        let g = |i: usize, j: usize| self.grade[j * side + i];
        let near = g(i, j) * (1.0 - s) + g(i + 1, j) * s;
        let far = g(i, j + 1) * (1.0 - s) + g(i + 1, j + 1) * s;
        near * (1.0 - t) + far * t
    }

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

/// The metre cell holding the centre of the column whose corner is at `(x_m, z_m)`.
fn metre_cell(x_m: f32, z_m: f32, half: f32) -> [i32; 2] {
    [(x_m + half).floor() as i32, (z_m + half).floor() as i32]
}

fn distance(a: [f32; 2], b: [f32; 2]) -> f32 {
    let [dx, dz] = [0, 1].map(|i| a[i] - b[i]);
    (dx * dx + dz * dz).sqrt()
}

fn length(road: &[[f32; 2]]) -> f32 {
    road.windows(2).map(|p| distance(p[0], p[1])).sum()
}

/// The places along `roads` a Plot is tried, at seeded gaps on each side of each road and a seeded
/// distance back from it, in a seeded order, so Plots spread over the disc.
fn slots(founding: &Founding, roads: &[Road]) -> Vec<Slot> {
    let mut slots = Vec::new();
    for (r, road) in roads.iter().enumerate() {
        let len = length(&road.points);
        for (k, side) in [1.0, -1.0].into_iter().enumerate() {
            let lane = mix(founding.hash ^ (0x200 + 2 * r + k) as u64);
            let mut along = 0.0;
            for n in 0.. {
                let key = mix(lane ^ n);
                along += lerp(SLOT_GAP_M, unit(mix(key ^ 1)) as f32);
                if along > len {
                    break;
                }
                slots.push(Slot {
                    key,
                    road: r,
                    along,
                    side,
                    setback_m: SETBACK_M * unit(mix(key ^ 2)) as f32,
                });
            }
        }
    }
    slots.sort_unstable_by_key(|s| s.key);
    slots
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
