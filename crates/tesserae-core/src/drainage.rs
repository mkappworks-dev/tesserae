//! Version 3's water on the land: lakes in basins and rivers found by a drainage solve over a coarse
//! lattice. The World is cut into regions; each region's solve reads a halo of its neighbours and is
//! cached, so a point's answer is a pure function of the seed and a fixed window, in any Chunk order
//! and at any cache state. See ADR-0030.

use std::cell::RefCell;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

/// Lattice spacing of the solve, in metres.
pub(crate) const NODE_M: f32 = 32.0;
/// Nodes along one side of a region (2048 m).
const REGION_NODES: i32 = 64;
/// Nodes of neighbouring regions the solve also reads on every side (1024 m); a river that would
/// need more upstream than this is cut at the window's edge.
const HALO_NODES: i32 = 32;
/// Nodes stored past the region's own on every side, so a query near the border never asks the
/// next region.
const MARGIN: i32 = 3;
const STORED: i32 = REGION_NODES + 2 * MARGIN;
const WINDOW: i32 = REGION_NODES + 2 * HALO_NODES;

/// Added to a node's level for each step downstream across a flat, so water finds its way out
/// without the lake's surface tilting visibly.
const EPS_M: f32 = 1e-4;
/// A node this far under its basin's spill level is lake.
const LAKE_MIN_DEPTH_M: f32 = 2.5;
/// Catchment, in nodes, above which a node carries a stream.
const STREAM_NODES: u32 = 120;
const MAX_CACHED_REGIONS: usize = 96;
/// How far under the node's level the water surface sits.
const SURFACE_BELOW_M: f32 = 0.6;
/// The valley wall climbs this much per metre from the water's edge.
const BANK_SLOPE: f32 = 0.5;
const VALLEY_REACH_M: f32 = 120.0;

const NO_PARENT: u8 = 8;
const MEANDER_SEED: u32 = 0x1a4d_00b1;
const STEPS: [(i32, i32); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

#[derive(Clone, Copy, Debug)]
struct Node {
    /// The base ground, before any river carves it.
    height_m: f32,
    /// The fill level: the ground, or the spill level of the basin it lies in.
    level_m: f32,
    catchment: u32,
    down: u8,
}

pub(crate) struct Region {
    nodes: Vec<Node>,
}

impl Region {
    fn node(&self, i: i32, j: i32, region: [i32; 2]) -> Node {
        let (li, lj) = (
            i - region[0] * REGION_NODES + MARGIN,
            j - region[1] * REGION_NODES + MARGIN,
        );
        debug_assert!((0..STORED).contains(&li) && (0..STORED).contains(&lj));
        self.nodes[(lj * STORED + li) as usize]
    }
}

/// What a column sees of the water on the land.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Wet {
    /// The level water stands at over this column, if a lake or river reaches it.
    pub level_m: Option<f32>,
    /// How far the river valley lowers the ground here, applied as a ceiling.
    pub ground_ceiling_m: Option<(f32, f32)>,
}

pub(crate) struct Drainage {
    id: u64,
    sea_m: f32,
    base: Box<dyn Fn(f32, f32) -> f32 + Send + Sync>,
    cache: RwLock<HashMap<[i32; 2], Arc<Region>>>,
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

thread_local! {
    static LAST: RefCell<Option<(u64, [i32; 2], Arc<Region>)>> = const { RefCell::new(None) };
}

fn sortable(f: f32) -> u32 {
    let b = f.to_bits();
    if b >> 31 == 1 { !b } else { b | 0x8000_0000 }
}

impl Drainage {
    pub(crate) fn new(sea_m: f32, base: impl Fn(f32, f32) -> f32 + Send + Sync + 'static) -> Self {
        Drainage {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            sea_m,
            base: Box::new(base),
            cache: RwLock::new(HashMap::new()),
        }
    }

    pub(crate) fn forget(&self) {
        self.cache.write().unwrap().clear();
        LAST.with(|l| *l.borrow_mut() = None);
    }

    fn region(&self, key: [i32; 2]) -> Arc<Region> {
        let hit = LAST.with(|l| {
            l.borrow()
                .as_ref()
                .filter(|(id, k, _)| *id == self.id && *k == key)
                .map(|(_, _, r)| r.clone())
        });
        if let Some(r) = hit {
            return r;
        }
        let cached = self.cache.read().unwrap().get(&key).cloned();
        let region = cached.unwrap_or_else(|| {
            let solved = Arc::new(self.solve(key));
            let mut cache = self.cache.write().unwrap();
            if cache.len() >= MAX_CACHED_REGIONS {
                cache.clear();
            }
            cache.entry(key).or_insert(solved).clone()
        });
        LAST.with(|l| *l.borrow_mut() = Some((self.id, key, region.clone())));
        region
    }

    fn solve(&self, key: [i32; 2]) -> Region {
        let origin = [
            key[0] * REGION_NODES - HALO_NODES,
            key[1] * REGION_NODES - HALO_NODES,
        ];
        let idx = |i: i32, j: i32| (j * WINDOW + i) as usize;
        let size = (WINDOW * WINDOW) as usize;
        let mut height = vec![0.0f32; size];
        for j in 0..WINDOW {
            for i in 0..WINDOW {
                height[idx(i, j)] = (self.base)(
                    (origin[0] + i) as f32 * NODE_M,
                    (origin[1] + j) as f32 * NODE_M,
                );
            }
        }
        let mut level = vec![f32::NAN; size];
        let mut parent = vec![NO_PARENT; size];
        let mut order = Vec::with_capacity(size);
        let mut heap = BinaryHeap::new();
        let mut seed = |i: i32, j: i32, f: f32, level: &mut Vec<f32>| {
            level[idx(i, j)] = f;
            heap.push(Reverse((sortable(f), idx(i, j))));
        };
        for j in 0..WINDOW {
            for i in 0..WINDOW {
                let h = height[idx(i, j)];
                let edge = i == 0 || j == 0 || i == WINDOW - 1 || j == WINDOW - 1;
                if h <= self.sea_m {
                    seed(i, j, self.sea_m, &mut level);
                } else if edge {
                    seed(i, j, h, &mut level);
                }
            }
        }
        while let Some(Reverse((_, at))) = heap.pop() {
            order.push(at);
            let (ci, cj) = ((at as i32) % WINDOW, (at as i32) / WINDOW);
            let here = level[at];
            for (k, (di, dj)) in STEPS.iter().enumerate() {
                let (ni, nj) = (ci + di, cj + dj);
                if !(0..WINDOW).contains(&ni) || !(0..WINDOW).contains(&nj) {
                    continue;
                }
                let n = idx(ni, nj);
                if !level[n].is_nan() {
                    continue;
                }
                let f = height[n].max(here + EPS_M);
                level[n] = f;
                // The step from `n` back to `at` is the opposite direction.
                parent[n] = (7 - k) as u8;
                heap.push(Reverse((sortable(f), n)));
            }
        }
        let mut catchment = vec![1u32; size];
        for &at in order.iter().rev() {
            if parent[at] != NO_PARENT {
                let (di, dj) = STEPS[parent[at] as usize];
                let p = idx((at as i32 % WINDOW) + di, (at as i32 / WINDOW) + dj);
                catchment[p] += catchment[at];
            }
        }
        let mut nodes = Vec::with_capacity((STORED * STORED) as usize);
        for lj in 0..STORED {
            for li in 0..STORED {
                let at = idx(li - MARGIN + HALO_NODES, lj - MARGIN + HALO_NODES);
                nodes.push(Node {
                    height_m: height[at],
                    level_m: level[at],
                    catchment: catchment[at],
                    down: parent[at],
                });
            }
        }
        Region { nodes }
    }

    /// The lake and river water over `(x_m, z_m)`, and the valley a river cuts there.
    pub(crate) fn wet(&self, x_m: f32, z_m: f32) -> Wet {
        let (u, v) = (x_m / NODE_M, z_m / NODE_M);
        let (i0, j0) = (u.floor() as i32, v.floor() as i32);
        let key = [i0.div_euclid(REGION_NODES), j0.div_euclid(REGION_NODES)];
        let region = self.region(key);
        let mut level_m: Option<f32> = None;

        for (di, dj) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            let n = region.node(i0 + di, j0 + dj, key);
            if n.level_m - n.height_m > LAKE_MIN_DEPTH_M && n.level_m > self.sea_m {
                level_m = Some(level_m.map_or(n.level_m, |l: f32| l.max(n.level_m)));
            }
        }

        // Bends the lattice's straight 8-way segments into meanders; bounded under a node's width so
        // the stencil below still holds every segment that can reach the point.
        let (qx, qz) = crate::noise::warp2(x_m, z_m, MEANDER_SEED, 90.0, 14.0);
        let mut best: Option<(f32, f32, f32)> = None;
        for dj in -1..=2 {
            for di in -1..=2 {
                let (ni, nj) = (i0 + di, j0 + dj);
                let n = region.node(ni, nj, key);
                if n.down == NO_PARENT || n.catchment < STREAM_NODES {
                    continue;
                }
                let (si, sj) = STEPS[n.down as usize];
                let p = region.node(ni + si, nj + sj, key);
                if p.level_m - p.height_m > LAKE_MIN_DEPTH_M
                    && n.level_m - n.height_m > LAKE_MIN_DEPTH_M
                {
                    continue;
                }
                let a = [ni as f32 * NODE_M, nj as f32 * NODE_M];
                let b = [(ni + si) as f32 * NODE_M, (nj + sj) as f32 * NODE_M];
                let ab = [b[0] - a[0], b[1] - a[1]];
                let t = (((qx - a[0]) * ab[0] + (qz - a[1]) * ab[1])
                    / (ab[0] * ab[0] + ab[1] * ab[1]))
                    .clamp(0.0, 1.0);
                let (cx, cz) = (a[0] + ab[0] * t, a[1] + ab[1] * t);
                let d = ((qx - cx).powi(2) + (qz - cz).powi(2)).sqrt();
                let width = river_width_m(n.catchment) * (1.0 - t) + river_width_m(p.catchment) * t;
                let surface = (n.level_m * (1.0 - t) + p.level_m * t) - SURFACE_BELOW_M;
                let edge = (d - width / 2.0).max(0.0);
                if best.is_none_or(|(e, _, _)| edge < e) {
                    best = Some((edge, surface, width));
                }
            }
        }
        let mut ground_ceiling_m = None;
        if let Some((edge, surface, width)) = best
            && edge < VALLEY_REACH_M
        {
            let dig = 1.2 + 0.25 * width.sqrt();
            let target = surface - dig + edge * BANK_SLOPE;
            let fade = 1.0 - smoothstep(0.66 * VALLEY_REACH_M, VALLEY_REACH_M, edge);
            ground_ceiling_m = Some((target, fade));
            if edge * BANK_SLOPE < dig * 2.0 {
                level_m = Some(level_m.map_or(surface, |l| l.max(surface)));
            }
        }
        Wet {
            level_m,
            ground_ceiling_m,
        }
    }
}

fn river_width_m(catchment: u32) -> f32 {
    (1.5 + 0.0045 * (catchment as f32 * NODE_M * NODE_M).sqrt()).min(48.0)
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
