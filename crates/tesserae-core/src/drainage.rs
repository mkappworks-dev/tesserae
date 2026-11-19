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
/// Catchment, in nodes, above which a node carries a stream.
const STREAM_NODES: u32 = 120;
const MAX_CACHED_REGIONS: usize = 96;
/// How far under the node's level the water surface sits.
const SURFACE_BELOW_M: f32 = 0.6;
/// The valley wall climbs this much per metre from the water's edge.
const BANK_SLOPE: f32 = 0.5;
/// Short enough that every segment reaching a point is in its stencil: a segment outside it is at
/// least `STENCIL - 1` nodes away, less the meander's push. A reach past that dropped a segment at
/// a node line while its valley still applied, and the ground stepped there.
const VALLEY_REACH_M: f32 = (STENCIL - 1) as f32 * NODE_M - MEANDER_M;
/// The nodes on each side of a point whose segments it asks.
const STENCIL: i32 = 4;
/// The most the meander warp moves a point.
const MEANDER_M: f32 = 14.0;

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
    #[cfg(test)]
    height_m: f32,
    /// The fill level: the ground, or the spill level of the basin it lies in.
    level_m: f32,
    catchment: u32,
    down: u8,
    /// In a basin deep enough to be a lake, shallow margins included, so the lake reaches its shore.
    lake: bool,
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
    /// How far rivers' valleys cut ground standing at the height asked about.
    pub cut_m: f32,
}

pub(crate) struct Drainage {
    id: u64,
    sea_m: f32,
    /// This solve's lattice spacing: [`NODE_M`], or [`COARSE_NODE_M`] for the coarse one.
    node_m: f32,
    base: Arc<dyn Fn(f32, f32) -> f32 + Send + Sync>,
    cache: RwLock<HashMap<[i32; 2], Arc<Region>>>,
    /// A solve on a coarser lattice over a wider window, whose large lakes set the level at the
    /// edge of this one's windows. Without it a lake wider than a window is filled to whatever
    /// spill each window happens to see, and two regions draw it at two levels.
    coarse: Option<Box<Drainage>>,
}

/// The coarse solve's spacing: its region is 8 km and its halo 4 km.
const COARSE_NODE_M: f32 = NODE_M * 4.0;

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
    pub(crate) fn new(sea_m: f32, base: impl Fn(f32, f32) -> f32 + Send + Sync + 'static) -> Self {
        let base: Arc<dyn Fn(f32, f32) -> f32 + Send + Sync> = Arc::new(base);
        Drainage {
            coarse: Some(Box::new(Self::lattice(sea_m, COARSE_NODE_M, base.clone()))),
            ..Self::lattice(sea_m, NODE_M, base)
        }
    }

    fn lattice(sea_m: f32, node_m: f32, base: Arc<dyn Fn(f32, f32) -> f32 + Send + Sync>) -> Self {
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
        for j in 0..WINDOW {
            for i in 0..WINDOW {
                height[idx(i, j)] = (self.base)(
                    (origin[0] + i) as f32 * self.node_m,
                    (origin[1] + j) as f32 * self.node_m,
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
                    #[cfg(test)]
                    height_m: height[at],
                    level_m: level[at],
                    catchment: catchment[at],
                    down: parent[at],
                    lake: lake[at],
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

        // Bends the lattice's straight 8-way segments into meanders; bounded under a node's width so
        // the stencil below still holds every segment that can reach the point.
        let (qx, qz) = crate::noise::warp2(x_m, z_m, MEANDER_SEED, 90.0, MEANDER_M);
        let mut cut_m = 0.0f32;
        for dj in -STENCIL + 1..=STENCIL {
            for di in -STENCIL + 1..=STENCIL {
                let (ni, nj) = (i0 + di, j0 + dj);
                let n = region.node(ni, nj, key);
                if n.down == NO_PARENT || n.catchment < STREAM_NODES {
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
                let t = (((qx - a[0]) * ab[0] + (qz - a[1]) * ab[1])
                    / (ab[0] * ab[0] + ab[1] * ab[1]))
                    .clamp(0.0, 1.0);
                let (cx, cz) = (a[0] + ab[0] * t, a[1] + ab[1] * t);
                let (dx, dz) = (qx - cx, qz - cz);
                let d = (dx * dx + dz * dz).sqrt();
                let width = river_width_m(n.catchment) * (1.0 - t) + river_width_m(p.catchment) * t;
                let surface = (n.level_m * (1.0 - t) + p.level_m * t) - SURFACE_BELOW_M;
                let edge = (d - width / 2.0).max(0.0);
                if edge >= VALLEY_REACH_M {
                    continue;
                }
                let dig = 1.2 + 0.25 * width.sqrt();
                let target = surface - dig + edge * BANK_SLOPE;
                let fade = 1.0 - smoothstep(0.66 * VALLEY_REACH_M, VALLEY_REACH_M, edge);
                cut_m = cut_m.max((ground_m - target).max(0.0) * fade);
                if edge * BANK_SLOPE < dig * 2.0 {
                    level_m = Some(level_m.map_or(surface, |l| l.max(surface)));
                }
            }
        }
        Wet { level_m, cut_m }
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

fn river_width_m(catchment: u32) -> f32 {
    (1.5 + 0.0045 * (catchment as f32 * NODE_M * NODE_M).sqrt()).min(48.0)
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
        Drainage::new(SEA_M, |x, z| {
            let n = crate::noise::fbm2(x / 900.0, z / 900.0, 7, 4);
            SEA_M - 30.0 + n * 140.0 - (x + z).max(0.0) * 0.004
        })
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
        Drainage::new(SEA_M, |x, z| {
            let (dx, dz) = (x - 2048.0, z - 1024.0);
            let r = (dx * dx + dz * dz).sqrt();
            let notch = dx < 0.0 && dz.abs() < 200.0;
            match r {
                r if r < 2400.0 => SEA_M + 16.0 + r / 2400.0 * 5.0,
                r if notch => SEA_M + 26.0 - (r - 2400.0) * 0.01,
                r if r < 2700.0 => SEA_M + 56.0,
                r => SEA_M + 56.0 - (r - 2700.0) * 0.01,
            }
        })
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
}
