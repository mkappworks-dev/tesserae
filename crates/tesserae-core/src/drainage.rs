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
const MARGIN: i32 = STENCIL + 2;
const STORED: i32 = REGION_NODES + 2 * MARGIN;
const WINDOW: i32 = REGION_NODES + 2 * HALO_NODES;

/// Added to a node's level for each step downstream across a flat, so water finds its way out
/// without the lake's surface tilting visibly.
const EPS_M: f32 = 1e-4;
/// A basin whose deepest node is this far under its spill level is a lake.
const LAKE_MIN_DEPTH_M: f32 = 2.5;
/// More than the `EPS_M` steps a flat adds up across a window: a node under its fill level by this
/// much is in a basin.
const FLOODED_M: f32 = 0.05;
/// Catchment, in nodes, above which a node carries a stream: a brook of 0.025 km² starts high on
/// a slope, a few nodes below its ridge. Lower, and a brook runs through most lattice cells.
const STREAM_NODES: u32 = 24;
/// Catchment, in nodes, from which a stream is a river at its full least width; below it a
/// brook narrows to [`BROOK_WIDTH_M`].
const RIVER_NODES: u32 = 120;
/// The narrowest brook: wider than a 1 m Voxel's diagonal, so a brook running across the grid
/// still holds two neighbouring columns in each row and its water joins up face to face.
const BROOK_WIDTH_M: f32 = 1.6;
/// How far from a hilltop, in nodes, its first brook starts at most.
#[cfg(test)]
const BROOK_REACH_NODES: i32 = 4;
const MAX_CACHED_REGIONS: usize = 96;
/// How far under the node's level the water surface sits.
const SURFACE_BELOW_M: f32 = 0.6;
/// The valley wall climbs this much per metre from the water's edge.
const BANK_SLOPE: f32 = 0.5;
/// Short enough that every segment reaching a point is in its stencil: a segment outside it is at
/// least `STENCIL - 1` nodes away, less the meander's push. A reach past that dropped a segment at
/// a node line while its valley still applied, and the ground stepped there.
const VALLEY_REACH_M: f32 = (STENCIL - 1) as f32 * NODE_M - MEANDER_M;
/// How far a stream of `width_m` swings to either side, up to [`MEANDER_M`], which the stencil
/// and the valley's reach are bounded by; and how much of its bending is long bends rather than
/// tight wiggles, 0 for the narrowest brook and 1 for a river of [`LONG_BEND_WIDTH_M`].
fn meander_m(width_m: f32) -> (f32, f32) {
    let amount_m = (2.0 * width_m).clamp(3.0, MEANDER_M);
    let long = ((width_m - BROOK_WIDTH_M) / (LONG_BEND_WIDTH_M - BROOK_WIDTH_M)).clamp(0.0, 1.0);
    (amount_m, long)
}

/// The point a stream's channel is drawn from, `amount_m` off `(x_m, z_m)`: tight bends and long
/// ones, each over a fixed size and blended by `long`. A size that changed with the stream would
/// stretch the noise about the origin, so the warp would swing kilometres out.
fn meander_at(x_m: f32, z_m: f32, amount_m: f32, long: f32) -> (f32, f32) {
    let (tx, tz) = crate::noise::warp2(x_m, z_m, MEANDER_SEED, TIGHT_BEND_M, amount_m);
    let (lx, lz) = crate::noise::warp2(x_m, z_m, MEANDER_SEED ^ 1, LONG_BEND_M, amount_m);
    (tx + (lx - tx) * long, tz + (lz - tz) * long)
}

/// How far the narrowest brook's gully cuts the ground: 1 in 2 banks up to 6 m deep.
const BROOK_REACH_M: f32 = 12.0;
/// The nodes on each side of a point whose segments it asks.
const STENCIL: i32 = 4;
/// The size of a brook's tight bends and a large river's long ones.
const TIGHT_BEND_M: f32 = 40.0;
const LONG_BEND_M: f32 = 240.0;
/// The width from which a stream bends only in long bends.
const LONG_BEND_WIDTH_M: f32 = 20.0;
/// The most the meander warp moves a point: a river's swing, and a brook's is less.
const MEANDER_M: f32 = 14.0;

const NO_PARENT: u8 = 8;
const INLAND: u8 = u8::MAX;
/// Steps from the sea over which a river's channel widens into its mouth.
const MOUTH_NODES: u8 = 2;
/// How many times its channel's width a river's mouth is at the sea.
const MOUTH_WIDEN: f32 = 3.0;
/// The channel width from which a river reaching the sea fans into distributaries.
const DELTA_WIDTH_M: f32 = 10.0;
/// A distributary's width against its river's.
const DISTRIBUTARY: f32 = 0.5;
/// The farthest any channel cuts from its centre line: a 48 m river's half width and whole
/// reach, which the stencil holds. A mouth wider than that reaches less past its banks.
const CHANNEL_REACH_M: f32 = 24.0 + VALLEY_REACH_M;
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
    #[cfg(test)]
    height_m: f32,
    /// The fill level: the ground, or the spill level of the basin it lies in.
    level_m: f32,
    catchment: u32,
    down: u8,
    /// In a basin deep enough to be a lake, shallow margins included, so the lake reaches its shore.
    lake: bool,
    /// Steps downstream to the sea, 0 at the sea; [`INLAND`] past [`MOUTH_NODES`] or where the
    /// stream leaves the window.
    to_sea: u8,
    /// One bit per step in [`STEPS`] its river also flows down as a delta's distributary.
    delta: u8,
}

pub(crate) struct Region {
    nodes: Vec<Node>,
}

impl Node {
    fn is_lake(&self) -> bool {
        self.lake
    }
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
    /// How far rivers' and brooks' valleys cut ground standing at the height asked about.
    pub cut_m: f32,
    /// [`Self::cut_m`] by rivers alone: what a Biome reads as a river's bank, so a brook's gully
    /// keeps its slope's own ground.
    pub river_cut_m: f32,
}

pub(crate) struct Drainage {
    id: u64,
    sea_m: f32,
    /// This solve's lattice spacing: [`NODE_M`], or [`COARSE_NODE_M`] for the coarse one.
    node_m: f32,
    base: Arc<BaseRow>,
    cache: RwLock<HashMap<[i32; 2], Arc<Region>>>,
    /// A solve on a coarser lattice over a wider window, whose large lakes set the level at the
    /// edge of this one's windows. Without it a lake wider than a window is filled to whatever
    /// spill each window happens to see, and two regions draw it at two levels.
    coarse: Option<Box<Drainage>>,
}

/// The coarse solve's spacing: its region is 8 km and its halo 4 km.
const COARSE_NODE_M: f32 = NODE_M * 4.0;

/// The base ground at four points, as a window's rows ask for it.
type BaseRow = dyn Fn([f32; 4], [f32; 4]) -> [f32; 4] + Send + Sync;

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// The region this thread asked for last and whose drainage it was. A stale entry is harmless: a
/// region is a pure function of its Drainage, so this only skips the lock and the map.
type LastRegion = Option<(u64, [i32; 2], Arc<Region>)>;

thread_local! {
    static LAST: RefCell<LastRegion> = const { RefCell::new(None) };
}

fn sortable(f: f32) -> u32 {
    let b = f.to_bits();
    if b >> 31 == 1 { !b } else { b | 0x8000_0000 }
}

impl Drainage {
    pub(crate) fn new(
        sea_m: f32,
        base: impl Fn([f32; 4], [f32; 4]) -> [f32; 4] + Send + Sync + 'static,
    ) -> Self {
        let base: Arc<BaseRow> = Arc::new(base);
        Drainage {
            coarse: Some(Box::new(Self::lattice(sea_m, COARSE_NODE_M, base.clone()))),
            ..Self::lattice(sea_m, NODE_M, base)
        }
    }

    fn lattice(sea_m: f32, node_m: f32, base: Arc<BaseRow>) -> Self {
        Drainage {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            sea_m,
            node_m,
            base,
            cache: RwLock::new(HashMap::new()),
            coarse: None,
        }
    }

    /// The level of the highest lake on a corner of the lattice cell holding `(x_m, z_m)`; `None`
    /// where no corner is lake, so a river crossing a window's edge away from any lake is never
    /// dammed by ground this coarse lattice is too wide to see the valley of.
    fn lake_level_at(&self, x_m: f32, z_m: f32) -> Option<f32> {
        let (i0, j0) = (
            (x_m / self.node_m).floor() as i32,
            (z_m / self.node_m).floor() as i32,
        );
        let key = [i0.div_euclid(REGION_NODES), j0.div_euclid(REGION_NODES)];
        let region = self.region(key);
        [(0, 0), (1, 0), (0, 1), (1, 1)]
            .map(|(di, dj)| region.node(i0 + di, j0 + dj, key))
            .iter()
            .filter(|n| n.is_lake())
            .map(|n| n.level_m)
            .reduce(f32::max)
    }

    /// Drops the cache and this thread's last region; another thread keeps its own until it next
    /// misses.
    pub(crate) fn forget(&self) {
        self.cache.write().unwrap().clear();
        LAST.with(|l| *l.borrow_mut() = None);
        if let Some(coarse) = &self.coarse {
            coarse.forget();
        }
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
        const { assert!(WINDOW % 4 == 0) };
        for j in 0..WINDOW {
            for i in (0..WINDOW).step_by(4) {
                let row = (self.base)(
                    std::array::from_fn(|k| (origin[0] + i + k as i32) as f32 * self.node_m),
                    [(origin[1] + j) as f32 * self.node_m; 4],
                );
                let at = idx(i, j);
                height[at..at + 4].copy_from_slice(&row);
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
                    let lake = self.coarse.as_ref().and_then(|c| {
                        c.lake_level_at(
                            (origin[0] + i) as f32 * self.node_m,
                            (origin[1] + j) as f32 * self.node_m,
                        )
                    });
                    seed(i, j, lake.map_or(h, |l| l.max(h)), &mut level);
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
                parent[n] = opposite(k);
                heap.push(Reverse((sortable(f), n)));
            }
        }
        let lake = lakes(&height, &level);
        // In flood order a node's parent comes first, so one pass counts every node's steps.
        let mut to_sea = vec![INLAND; size];
        for &at in &order {
            to_sea[at] = if parent[at] == NO_PARENT {
                if height[at] <= self.sea_m { 0 } else { INLAND }
            } else {
                let (di, dj) = STEPS[parent[at] as usize];
                let p = idx((at as i32 % WINDOW) + di, (at as i32 / WINDOW) + dj);
                match to_sea[p] {
                    INLAND => INLAND,
                    k if k >= MOUTH_NODES => INLAND,
                    k => k + 1,
                }
            };
        }
        let mut catchment = vec![1u32; size];
        for &at in order.iter().rev() {
            if parent[at] != NO_PARENT {
                let (di, dj) = STEPS[parent[at] as usize];
                let p = idx((at as i32 % WINDOW) + di, (at as i32 / WINDOW) + dj);
                catchment[p] += catchment[at];
            }
        }
        let inside = |i: i32, j: i32| (0..WINDOW).contains(&i) && (0..WINDOW).contains(&j);
        let mut delta = vec![0u8; size];
        for at in 0..size {
            let (ci, cj) = ((at as i32) % WINDOW, (at as i32) / WINDOW);
            if to_sea[at] == INLAND
                || to_sea[at] == 0
                || parent[at] == NO_PARENT
                || river_width_m(catchment[at]) < DELTA_WIDTH_M
            {
                continue;
            }
            for (k, (di, dj)) in STEPS.iter().enumerate() {
                let (qi, qj) = (ci + di, cj + dj);
                if k != parent[at] as usize
                    && inside(qi, qj)
                    && to_sea[idx(qi, qj)] <= 1
                    && level[idx(qi, qj)] < level[at]
                {
                    delta[at] |= 1 << k;
                }
            }
        }
        let mut nodes = Vec::with_capacity((STORED * STORED) as usize);
        for lj in 0..STORED {
            for li in 0..STORED {
                let at = idx(li - MARGIN + HALO_NODES, lj - MARGIN + HALO_NODES);
                nodes.push(Node {
                    #[cfg(test)]
                    height_m: height[at],
                    level_m: level[at],
                    catchment: catchment[at],
                    down: parent[at],
                    lake: lake[at],
                    to_sea: to_sea[at],
                    delta: delta[at],
                });
            }
        }
        Region { nodes }
    }

    /// The lake and river water over `(x_m, z_m)`, and how far rivers' valleys cut ground standing
    /// at `ground_m` there. Each segment in reach cuts its own valley and the deepest wins, so the
    /// ground is continuous where two segments are equally near.
    pub(crate) fn wet(&self, x_m: f32, z_m: f32, ground_m: f32) -> Wet {
        let (u, v) = (x_m / self.node_m, z_m / self.node_m);
        let (i0, j0) = (u.floor() as i32, v.floor() as i32);
        let key = [i0.div_euclid(REGION_NODES), j0.div_euclid(REGION_NODES)];
        let region = self.region(key);
        let mut level_m: Option<f32> = None;

        for (di, dj) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            let n = region.node(i0 + di, j0 + dj, key);
            if n.is_lake() && n.level_m > self.sea_m {
                level_m = Some(level_m.map_or(n.level_m, |l: f32| l.max(n.level_m)));
            }
        }

        let (mut cut_m, mut river_cut_m) = (0.0f32, 0.0f32);
        for dj in -STENCIL + 1..=STENCIL {
            for di in -STENCIL + 1..=STENCIL {
                let (ni, nj) = (i0 + di, j0 + dj);
                let n = region.node(ni, nj, key);
                if n.down == NO_PARENT {
                    continue;
                }
                if n.catchment < STREAM_NODES {
                    // Too small for a stream of its own, but a step from the sea a delta beside
                    // it may still feed it a distributary.
                    if n.to_sea == 1 {
                        if let Some(w) = delta_fed_m(&region, key, (ni, nj)) {
                            let (si, sj) = STEPS[n.down as usize];
                            let p = region.node(ni + si, nj + sj, key);
                            let c = self.side_channel(
                                (ni, nj),
                                n,
                                (si, sj),
                                p,
                                w,
                                (x_m, z_m),
                                ground_m,
                            );
                            take_channel(c, true, &mut cut_m, &mut river_cut_m, &mut level_m);
                        }
                    }
                    continue;
                }
                let (si, sj) = STEPS[n.down as usize];
                let p = region.node(ni + si, nj + sj, key);
                if p.is_lake() && n.is_lake() {
                    continue;
                }
                let a = [ni as f32 * self.node_m, nj as f32 * self.node_m];
                let b = [
                    (ni + si) as f32 * self.node_m,
                    (nj + sj) as f32 * self.node_m,
                ];
                let ab = [b[0] - a[0], b[1] - a[1]];
                let along = |x: f32, z: f32| {
                    (((x - a[0]) * ab[0] + (z - a[1]) * ab[1]) / (ab[0] * ab[0] + ab[1] * ab[1]))
                        .clamp(0.0, 1.0)
                };
                // Bends the lattice's straight 8-way segments into meanders sized by the stream,
                // blended between its two ends as its width is, so a tributary and the river it
                // joins bend alike where they meet.
                let s0 = along(x_m, z_m);
                let w0 = river_width_m(n.catchment) * (1.0 - s0) + river_width_m(p.catchment) * s0;
                let (amount_m, long) = meander_m(w0);
                let (qx, qz) = meander_at(x_m, z_m, amount_m, long);
                let t = along(qx, qz);
                let (cx, cz) = (a[0] + ab[0] * t, a[1] + ab[1] * t);
                let (dx, dz) = (qx - cx, qz - cz);
                let d = (dx * dx + dz * dz).sqrt();
                let mut width =
                    river_width_m(n.catchment) * (1.0 - t) + river_width_m(p.catchment) * t;
                let near_sea = n.to_sea != INLAND;
                if near_sea {
                    width = mouth_width_m(width, sea_steps(n, p, t));
                }
                let surface = (n.level_m * (1.0 - t) + p.level_m * t) - SURFACE_BELOW_M;
                let edge = (d - width / 2.0).max(0.0);
                let mut reach_m = valley_reach_m(width);
                if near_sea {
                    reach_m = reach_m.min(CHANNEL_REACH_M - width / 2.0);
                }
                if n.delta != 0 {
                    let branch_m = river_width_m(n.catchment) * DISTRIBUTARY;
                    for (di, dj) in distributaries(n) {
                        let q = region.node(ni + di, nj + dj, key);
                        let c = self.side_channel(
                            (ni, nj),
                            n,
                            (di, dj),
                            q,
                            branch_m,
                            (x_m, z_m),
                            ground_m,
                        );
                        take_channel(c, true, &mut cut_m, &mut river_cut_m, &mut level_m);
                    }
                }
                if edge >= reach_m {
                    continue;
                }
                let dig = dig_m(width);
                let target = surface - dig + edge * BANK_SLOPE;
                let fade = 1.0 - smoothstep(0.66 * reach_m, reach_m, edge);
                let cut = (ground_m - target).max(0.0) * fade;
                cut_m = cut_m.max(cut);
                if n.catchment >= RIVER_NODES {
                    river_cut_m = river_cut_m.max(cut);
                }
                // A brook's water stays in the channel it cut: past it the slope stands a few
                // centimetres either side of the surface, and its Voxels would chequer wet and dry.
                let band = if n.catchment >= RIVER_NODES { 2.0 } else { 1.0 };
                if edge * BANK_SLOPE < dig * band {
                    level_m = Some(level_m.map_or(surface, |l| l.max(surface)));
                }
            }
        }
        Wet {
            level_m,
            cut_m,
            river_cut_m,
        }
    }

    /// The cut and water level a delta's distributary of `width_m`, from node `n` at `at` down
    /// `step` to `p`, makes at `(x_m, z_m)` over ground at `ground_m`; the water level is
    /// `f32::MIN` outside its channel. Out of line and cold: few nodes have one, and the stream
    /// loop above stays as small as it was without deltas.
    #[allow(clippy::too_many_arguments, reason = "one segment's own description")]
    #[cold]
    #[inline(never)]
    fn side_channel(
        &self,
        at: (i32, i32),
        n: Node,
        (si, sj): (i32, i32),
        p: Node,
        width_m: f32,
        (x_m, z_m): (f32, f32),
        ground_m: f32,
    ) -> (f32, f32) {
        let a = [at.0 as f32 * self.node_m, at.1 as f32 * self.node_m];
        let ab = [si as f32 * self.node_m, sj as f32 * self.node_m];
        let along = |x: f32, z: f32| {
            (((x - a[0]) * ab[0] + (z - a[1]) * ab[1]) / (ab[0] * ab[0] + ab[1] * ab[1]))
                .clamp(0.0, 1.0)
        };
        let (amount_m, long) = meander_m(width_m);
        let (qx, qz) = meander_at(x_m, z_m, amount_m, long);
        let t = along(qx, qz);
        let (cx, cz) = (a[0] + ab[0] * t, a[1] + ab[1] * t);
        let d = (qx - cx).hypot(qz - cz);
        let width = mouth_width_m(width_m, sea_steps(n, p, t));
        let surface = (n.level_m * (1.0 - t) + p.level_m * t) - SURFACE_BELOW_M;
        let edge = (d - width / 2.0).max(0.0);
        let reach_m = valley_reach_m(width).min(CHANNEL_REACH_M - width / 2.0);
        if edge >= reach_m {
            return (0.0, f32::MIN);
        }
        let dig = dig_m(width);
        let target = surface - dig + edge * BANK_SLOPE;
        let fade = 1.0 - smoothstep(0.66 * reach_m, reach_m, edge);
        let wet = edge * BANK_SLOPE < dig * 2.0;
        (
            (ground_m - target).max(0.0) * fade,
            if wet { surface } else { f32::MIN },
        )
    }
}

/// Per window node, whether it lies in a basin some node of which is deeper than a lake's least
/// depth: each 8-connected group of flooded nodes is one basin, filled to one spill level.
fn lakes(height: &[f32], level: &[f32]) -> Vec<bool> {
    let flooded = |at: usize| level[at] - height[at] > FLOODED_M;
    let mut lake = vec![false; height.len()];
    let mut seen = vec![false; height.len()];
    let mut group = Vec::new();
    for start in 0..height.len() {
        if seen[start] || !flooded(start) {
            continue;
        }
        seen[start] = true;
        group.clear();
        group.push(start);
        let (mut i, mut deep) = (0, false);
        while let Some(&at) = group.get(i) {
            i += 1;
            deep |= level[at] - height[at] > LAKE_MIN_DEPTH_M;
            let (ci, cj) = ((at as i32) % WINDOW, (at as i32) / WINDOW);
            for (di, dj) in STEPS {
                let (ni, nj) = (ci + di, cj + dj);
                if !(0..WINDOW).contains(&ni) || !(0..WINDOW).contains(&nj) {
                    continue;
                }
                let n = (nj * WINDOW + ni) as usize;
                if !seen[n] && flooded(n) {
                    seen[n] = true;
                    group.push(n);
                }
            }
        }
        if deep {
            for &at in &group {
                lake[at] = true;
            }
        }
    }
    lake
}

/// `STEPS` is ordered so that a step and its reverse sum to 7.
fn opposite(step: usize) -> u8 {
    (STEPS.len() - 1 - step) as u8
}

/// Widens with the square root of catchment, as rivers do; a brook's banks start at the river's
/// least width and fall away toward its head, never below [`BROOK_WIDTH_M`].
fn river_width_m(catchment: u32) -> f32 {
    let bank_m = 1.5 * (catchment as f32 / RIVER_NODES as f32).min(1.0);
    (bank_m + 0.0045 * (catchment as f32 * NODE_M * NODE_M).sqrt()).clamp(BROOK_WIDTH_M, 48.0)
}

/// How far under its surface a stream's bed lies: a river's as it was, and a brook's shallower
/// with its width, down to a metre at the narrowest, so its banks meet the ground close in and
/// every Voxel of its channel still holds water at 1 m.
fn dig_m(width_m: f32) -> f32 {
    let river = |w: f32| 1.2 + 0.25 * w.sqrt();
    let least_m = river_width_m(RIVER_NODES);
    if width_m >= least_m {
        return river(width_m);
    }
    let t = (width_m - BROOK_WIDTH_M) / (least_m - BROOK_WIDTH_M);
    1.0 + t * (river(least_m) - 1.0)
}

/// How far from its water a stream's valley cuts the ground: a river's whole reach, and a brook's
/// gully narrower with its width, so brooks high on a slope do not carve a river's valley.
fn valley_reach_m(width_m: f32) -> f32 {
    let least_m = river_width_m(RIVER_NODES);
    let t = ((width_m - BROOK_WIDTH_M) / (least_m - BROOK_WIDTH_M)).min(1.0);
    BROOK_REACH_M + t * (VALLEY_REACH_M - BROOK_REACH_M)
}

/// A channel of `width_m` widened into its mouth, `steps` downstream from the sea: as it was
/// [`MOUTH_NODES`] out, [`MOUTH_WIDEN`] times as wide at the sea.
#[inline]
fn mouth_width_m(width_m: f32, steps: f32) -> f32 {
    let near = 1.0 - (steps / MOUTH_NODES as f32).clamp(0.0, 1.0);
    width_m * (1.0 + (MOUTH_WIDEN - 1.0) * near)
}

/// Steps to the sea at `t` along the segment from `n` to `p`.
#[inline]
fn sea_steps(n: Node, p: Node, t: f32) -> f32 {
    let steps = |k: u8| if k == INLAND { f32::INFINITY } else { k as f32 };
    if n.to_sea == INLAND {
        return f32::INFINITY;
    }
    steps(n.to_sea) * (1.0 - t) + steps(p.to_sea) * t
}

/// The steps a node's river also flows down as a delta: past its own, to each lower neighbour at
/// the sea or a step from it, while it is wide enough and near enough to fan out. A neighbour a
/// step out carries the distributary on to the sea itself ([`delta_fed_m`]).
fn distributaries(n: Node) -> impl Iterator<Item = (i32, i32)> {
    (0..STEPS.len())
        .filter(move |&k| n.delta & (1 << k) != 0)
        .map(|k| STEPS[k])
}

/// The width of the distributary node `at` carries down its last step to the sea, if a delta
/// beside it fans into it. Asked only a step from the sea, so the node keeps no field for it:
/// the stencil reads 64 nodes a column, and a wider one costs every column.
fn delta_fed_m(region: &Region, key: [i32; 2], at: (i32, i32)) -> Option<f32> {
    STEPS
        .iter()
        .enumerate()
        .filter_map(|(k, &(di, dj))| {
            let m = region.node(at.0 + di, at.1 + dj, key);
            (m.delta & (1 << opposite(k)) != 0).then(|| river_width_m(m.catchment) * DISTRIBUTARY)
        })
        .reduce(f32::max)
}

/// Folds one channel's cut and water level into a column's.
#[inline]
fn take_channel(
    (cut, surface): (f32, f32),
    river: bool,
    cut_m: &mut f32,
    river_cut_m: &mut f32,
    level_m: &mut Option<f32>,
) {
    *cut_m = cut_m.max(cut);
    if river {
        *river_cut_m = river_cut_m.max(cut);
    }
    if surface > f32::MIN {
        *level_m = Some(level_m.map_or(surface, |l| l.max(surface)));
    }
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEA_M: f32 = 24.0;

    /// Hills and a sea in one corner, so water has somewhere to go.
    fn drainage() -> Drainage {
        Drainage::new(
            SEA_M,
            each(|x, z| {
                let n = crate::noise::fbm2(x / 900.0, z / 900.0, 7, 4);
                SEA_M - 30.0 + n * 140.0 - (x + z).max(0.0) * 0.004
            }),
        )
    }

    /// A ground of one point at a time, asked four at a time.
    fn each(f: impl Fn(f32, f32) -> f32) -> impl Fn([f32; 4], [f32; 4]) -> [f32; 4] {
        move |x, z| std::array::from_fn(|k| f(x[k], z[k]))
    }

    fn region_nodes(d: &Drainage, key: [i32; 2]) -> Vec<(i32, i32, Node)> {
        let region = d.solve(key);
        (-MARGIN..REGION_NODES + MARGIN)
            .flat_map(|j| (-MARGIN..REGION_NODES + MARGIN).map(move |i| (i, j)))
            .map(|(i, j)| {
                let at = [i + key[0] * REGION_NODES, j + key[1] * REGION_NODES];
                (at[0], at[1], region.node(at[0], at[1], key))
            })
            .collect()
    }

    #[test]
    fn every_stream_runs_downhill_and_ends_at_the_sea_or_the_edge_of_what_is_stored() {
        let d = drainage();
        let key = [0, 0];
        let region = d.solve(key);
        let stored = |i: i32, j: i32| {
            (-MARGIN..REGION_NODES + MARGIN).contains(&i)
                && (-MARGIN..REGION_NODES + MARGIN).contains(&j)
        };
        let mut checked = 0;
        for (i, j, n) in region_nodes(&d, key) {
            if n.catchment < STREAM_NODES {
                continue;
            }
            let (mut at, mut node) = ((i, j), n);
            for _ in 0..(STORED * STORED) {
                if node.down == NO_PARENT {
                    break;
                }
                let (di, dj) = STEPS[node.down as usize];
                let next = (at.0 + di, at.1 + dj);
                if !stored(next.0, next.1) {
                    break;
                }
                let down = region.node(next.0, next.1, key);
                assert!(
                    down.level_m <= node.level_m + 1e-3,
                    "water ran uphill at {at:?}"
                );
                assert!(
                    down.catchment > node.catchment,
                    "a stream lost water at {at:?}"
                );
                (at, node) = (next, down);
            }
            // A stream stopped for a reason: it reached the sea, the only outlet inside the stored
            // area, or it left that area.
            if node.down == NO_PARENT {
                assert!(
                    node.level_m <= SEA_M + 1e-3,
                    "a stream ends on dry land at {at:?}"
                );
            } else {
                let (di, dj) = STEPS[node.down as usize];
                assert!(
                    !stored(at.0 + di, at.1 + dj),
                    "a stream stopped short at {at:?}"
                );
            }
            checked += 1;
        }
        assert!(checked > 20, "{checked} stream nodes");
    }

    /// Most hilltops well above the sea have a brook starting below them, so a Player on a ridge
    /// sees water running down into the valley. At today's river threshold 4 of 14 did; a knoll
    /// whose slopes drain away on every side may still have none.
    #[test]
    fn brooks_start_near_every_high_hilltop() {
        let d = drainage();
        let key = [0, 0];
        let nodes = region_nodes(&d, key);
        let region = d.solve(key);
        let inner = |i: i32, j: i32| {
            (8..REGION_NODES - 8).contains(&i) && (8..REGION_NODES - 8).contains(&j)
        };
        let (mut tops, mut fed) = (0, 0);
        for &(i, j, n) in &nodes {
            if !inner(i, j) || n.height_m < SEA_M + 40.0 {
                continue;
            }
            let top = STEPS
                .iter()
                .all(|(di, dj)| region.node(i + di, j + dj, key).height_m < n.height_m);
            if !top {
                continue;
            }
            tops += 1;
            let near = (-BROOK_REACH_NODES..=BROOK_REACH_NODES).any(|dj| {
                (-BROOK_REACH_NODES..=BROOK_REACH_NODES)
                    .any(|di| region.node(i + di, j + dj, key).catchment >= STREAM_NODES)
            });
            fed += near as u32;
        }
        assert!(tops > 5, "{tops} hilltops");
        assert!(
            fed * 4 >= tops * 3,
            "{fed} of {tops} hilltops have a brook within reach"
        );
    }

    #[test]
    fn a_brook_is_a_voxel_wide_and_widens_downstream_into_the_rivers() {
        assert!(river_width_m(STREAM_NODES) >= std::f32::consts::SQRT_2);
        assert!(
            river_width_m(STREAM_NODES) < 2.0,
            "a brook is narrower than a river"
        );
        let widths: Vec<f32> = (STREAM_NODES..20_000).map(river_width_m).collect();
        assert!(
            widths.windows(2).all(|w| w[1] >= w[0]),
            "a stream narrows downstream"
        );
        // From today's least river on, the widths are the rivers' as they were.
        for c in [120u32, 400, 4000, 40_000] {
            let was = (1.5 + 0.0045 * (c as f32 * NODE_M * NODE_M).sqrt()).min(48.0);
            assert_eq!(river_width_m(c), was, "catchment {c}");
        }
        // A brook's wet band, from bank to bank, stays narrow.
        let w = river_width_m(STREAM_NODES);
        assert!(
            w + 2.0 * dig_m(w) / BANK_SLOPE <= 6.0,
            "a {w} m brook draws a wide band"
        );
        let r = river_width_m(RIVER_NODES);
        assert_eq!(dig_m(r), 1.2 + 0.25 * r.sqrt(), "a river digs as it did");
    }

    #[test]
    fn a_brooks_gully_is_no_rivers_bank() {
        let d = drainage();
        let (mut brook, mut river) = (0, 0);
        for j in -160..160 {
            for i in -160..160 {
                let (x, z) = (i as f32 * 12.7, j as f32 * 12.9);
                let w = d.wet(x, z, 1e4);
                assert!(w.river_cut_m <= w.cut_m, "{x},{z}");
                brook += usize::from(w.cut_m > 1.5 && w.river_cut_m == 0.0);
                river += usize::from(w.river_cut_m > 1.5);
            }
        }
        assert!(
            brook > 100 && river > 100,
            "{brook} brook cuts, {river} river cuts"
        );
    }

    #[test]
    fn meanders_grow_with_the_stream_and_stay_inside_the_stencil() {
        let widths: Vec<f32> = (STREAM_NODES..40_000)
            .step_by(7)
            .map(river_width_m)
            .collect();
        let bends: Vec<(f32, f32)> = widths.iter().map(|&w| meander_m(w)).collect();
        assert!(
            bends
                .windows(2)
                .all(|b| b[1].0 >= b[0].0 && b[1].1 >= b[0].1),
            "a wider stream bends less"
        );
        let (brook, river) = (bends[0], *bends.last().unwrap());
        assert!(brook.0 * 3.0 <= river.0, "brook {brook:?}, river {river:?}");
        assert_eq!(
            (brook.1, river.1),
            (0.0, 1.0),
            "brook {brook:?}, river {river:?}"
        );
        assert!(
            bends.iter().all(|&(a, _)| a <= MEANDER_M),
            "a bend leaves the stencil"
        );
        // Never further than the swing, wherever the point is.
        for k in 0..2000 {
            let (x, z) = (k as f32 * 977.3 - 9e5, k as f32 * -611.9 + 4e5);
            let (qx, qz) = meander_at(x, z, MEANDER_M, (k % 11) as f32 / 10.0);
            assert!((qx - x).hypot(qz - z) <= MEANDER_M * 2f32.sqrt() + 1e-2);
        }
    }

    #[test]
    fn rivers_widen_into_mouths_and_wide_ones_fan_into_deltas() {
        let d = drainage();
        let (mut mouths, mut deltas) = (0, 0);
        for key in [[0, 0], [1, 0], [0, 1], [1, 1]] {
            let region = d.solve(key);
            for (i, j, n) in region_nodes(&d, key) {
                let inner =
                    |k: i32, o: i32| (2..REGION_NODES - 2).contains(&(k - o * REGION_NODES));
                if !inner(i, key[0]) || !inner(j, key[1]) {
                    continue;
                }
                if n.catchment < RIVER_NODES || n.down == NO_PARENT {
                    continue;
                }
                let (si, sj) = STEPS[n.down as usize];
                let p = region.node(i + si, j + sj, key);
                if p.to_sea != 0 {
                    continue;
                }
                mouths += 1;
                let channel = river_width_m(n.catchment);
                assert!(
                    mouth_width_m(channel, sea_steps(n, p, 1.0)) > channel,
                    "the river at {i},{j} meets the sea no wider than its channel"
                );
                if channel >= DELTA_WIDTH_M {
                    deltas += 1;
                    assert!(
                        distributaries(n).next().is_some(),
                        "the {channel} m river at {i},{j} meets the sea in one channel"
                    );
                }
            }
        }
        assert!(mouths > 3 && deltas > 0, "{mouths} mouths, {deltas} deltas");
    }

    #[test]
    fn a_basin_fills_to_its_spill_and_no_water_stands_below_the_ground() {
        let d = drainage();
        let mut lakes = 0;
        for (_, _, n) in region_nodes(&d, [0, 0]) {
            assert!(n.level_m >= n.height_m - 1e-3, "water under the ground");
            lakes += n.is_lake() as u32;
        }
        assert!(lakes > 0, "no basin in the test land");
    }

    #[test]
    fn a_lake_takes_in_the_shallow_margins_of_its_basin() {
        let d = drainage();
        let (mut shallow, mut lakes) = (0, 0);
        for (_, _, n) in region_nodes(&d, [0, 0]) {
            let depth = n.level_m - n.height_m;
            if n.is_lake() {
                lakes += 1;
                shallow += (depth < LAKE_MIN_DEPTH_M) as u32;
            } else {
                assert!(
                    depth <= LAKE_MIN_DEPTH_M,
                    "a node {depth} m deep is not lake"
                );
            }
        }
        assert!(
            lakes > 0 && shallow > 0,
            "{lakes} lake nodes, {shallow} shallow"
        );
    }

    /// A bowl 5 km across, centred on the border between regions `[0, 0]` and `[1, 0]`, whose
    /// one low notch is inside the first region's window and outside the second's.
    fn bowl() -> Drainage {
        Drainage::new(
            SEA_M,
            each(|x, z| {
                let (dx, dz) = (x - 2048.0, z - 1024.0);
                let r = (dx * dx + dz * dz).sqrt();
                let notch = dx < 0.0 && dz.abs() < 200.0;
                match r {
                    r if r < 2400.0 => SEA_M + 16.0 + r / 2400.0 * 5.0,
                    r if notch => SEA_M + 26.0 - (r - 2400.0) * 0.01,
                    r if r < 2700.0 => SEA_M + 56.0,
                    r => SEA_M + 56.0 - (r - 2700.0) * 0.01,
                }
            }),
        )
    }

    #[test]
    fn a_lake_wider_than_a_window_has_one_level_on_both_sides_of_a_border() {
        let d = bowl();
        let (a, b) = (d.solve([0, 0]), d.solve([1, 0]));
        let mut compared = 0;
        for j in 24..40 {
            for i in REGION_NODES..REGION_NODES + MARGIN {
                let (na, nb) = (a.node(i, j, [0, 0]), b.node(i, j, [1, 0]));
                assert!(
                    na.is_lake() && nb.is_lake(),
                    "node {i},{j} is lake on one side only"
                );
                assert!(
                    (na.level_m - nb.level_m).abs() < 0.05,
                    "node {i},{j}: {} against {}",
                    na.level_m,
                    nb.level_m
                );
                compared += 1;
            }
        }
        assert!(compared > 30);
    }

    #[test]
    fn neighbouring_regions_agree_where_their_stored_nodes_overlap() {
        let d = drainage();
        let (a, b) = (d.solve([0, 0]), d.solve([1, 0]));
        let (mut same, mut total) = (0, 0);
        for j in 0..REGION_NODES {
            for i in REGION_NODES..REGION_NODES + MARGIN {
                let (na, nb) = (a.node(i, j, [0, 0]), b.node(i, j, [1, 0]));
                total += 1;
                same += ((na.level_m - nb.level_m).abs() < 1e-3 && na.down == nb.down) as u32;
            }
        }
        // Two windows differ only where a far boundary reaches them: nearly every node agrees.
        assert!(
            same * 100 >= total * 95,
            "{same} of {total} border nodes agree"
        );
    }

    #[test]
    fn the_water_level_is_the_same_whatever_ground_is_asked_about() {
        let d = drainage();
        let grounds = [
            f32::MIN,
            -1e4,
            SEA_M - 30.0,
            SEA_M,
            SEA_M + 12.0,
            400.0,
            1e6,
        ];
        let (mut wet, mut cut) = (0, 0);
        // Across the four regions round the origin, and their borders.
        for j in -160..160 {
            for i in -160..160 {
                let h = crate::noise::cell_hash(i, j, 0x3e7);
                let (x, z) = ((i as f32 + h[0]) * 12.8, (j as f32 + h[1]) * 12.8);
                let want = d.wet(x, z, f32::MIN).level_m.map(f32::to_bits);
                wet += usize::from(want.is_some());
                for ground_m in grounds {
                    let w = d.wet(x, z, ground_m);
                    assert_eq!(
                        w.level_m.map(f32::to_bits),
                        want,
                        "{x},{z} under {ground_m}"
                    );
                    cut += usize::from(w.cut_m > 0.0);
                }
            }
        }
        assert!(wet > 1000 && cut > 1000, "{wet} wet points, {cut} cuts");
    }
}
