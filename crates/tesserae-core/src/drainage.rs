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
/// Catchment from which a line of nodes no other joins carries a stream all the same: down a long
/// even slope.
const SLOPE_STREAM_NODES: u32 = 4 * STREAM_NODES;
/// Catchment, in nodes, from which a stream is a river at its full least width; below it a
/// brook narrows to [`BROOK_WIDTH_M`].
const RIVER_NODES: u32 = 120;
/// How deep the narrowest brook's bed lies under its surface.
const BROOK_DIG_M: f32 = 1.6;
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
const VALLEY_REACH_M: f32 = (STENCIL - 1) as f32 * NODE_M - MEANDER_M - JITTER_M;
/// A pond may stand in each cell this wide, with [`POND_CHANCE`].
const POND_CELL_M: f32 = 128.0;
const POND_CHANCE: f32 = 0.2;
const POND_SEED: u32 = 0x7044_0d5e;
/// A pond's radius, most of them small.
const POND_RADIUS_M: (f32, f32) = (2.5, 10.0);
/// The bank round a pond its bowl reaches out over.
const POND_BANK_M: f32 = 4.0;
/// How deep a pond is at its middle, and how far under its rim's lowest point its water stands.
const POND_DEPTH_M: f32 = 1.8;
const POND_BELOW_RIM_M: f32 = 0.4;
/// A pond stands only where its rim is this level, and this far above the sea.
const POND_LEVEL_M: f32 = 1.5;
const POND_ABOVE_SEA_M: f32 = 2.0;

/// How far a channel's node stands off its lattice point, at most, along each axis: enough that
/// channels stop running in straight lattice lines, little enough to stay in the stencil.
const JITTER_M: f32 = 0.3 * NODE_M;
const JITTER_SEED: u32 = 0x6a17_7e2d;
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
/// In a basin deep enough to be a lake, shallow margins included, so the lake reaches its shore.
const LAKE: u8 = 1;
/// A lake node whose way down leaves its lake by a spill its window sees, not by the window's
/// edge.
const SPILLED: u8 = 2;
/// A stream's first node: no stream runs into it, so its water rises there from a spring.
const HEAD: u8 = 4;
/// A node that carries a stream: see [`Drainage::solve`].
const STREAM: u8 = 8;
/// A spring's pool, round about a stream's head, from which its channel narrows to its own width
/// over as far again.
const SPRING_WIDTH_M: f32 = 5.0;
/// How far along a window's edge either side of a river coming in is kept from being an outlet:
/// a coarse node's spacing, as far off as the coarse river may cross.
const ENTRY_NODES: i32 = 4;
const CAPTURE_ROUNDS: usize = 4;
const INLAND: u8 = u8::MAX;
/// Steps from the sea over which a river's channel widens into its mouth.
const MOUTH_NODES: u8 = 2;
/// How many times its channel's width a river's mouth is at the sea.
const MOUTH_WIDEN: f32 = 3.0;
/// The channel width from which a river reaching the sea fans into distributaries.
const DELTA_WIDTH_M: f32 = 4.0;
/// A distributary's width against its river's.
const DISTRIBUTARY: f32 = 0.5;
/// The farthest any channel cuts from its centre line: a 48 m river's half width and whole
/// reach, which the stencil holds. A mouth wider than that reaches less past its banks.
const CHANNEL_REACH_M: f32 = 24.0 + VALLEY_REACH_M;
const MEANDER_SEED: u32 = 0x1a4d_00b1;
/// A lake's drift, slowest where little of it has passed and fastest near the spill.
const LAKE_DRIFT_M_S: (f32, f32) = (0.02, 0.2);
/// Running water, from a brook's slack pool to a big river's steepest reach.
const STREAM_M_S: (f32, f32) = (0.2, 2.5);
/// A pool's speed over a catchment of one square kilometre. Speed grows with discharge only as
/// its tenth power (Leopold and Maddock's hydraulic geometry), so a brook and a great river pool
/// within a factor of two of each other: the water is drawn as a staircase of flat pools, and a
/// speed from the node-to-node slope ran most rivers at their cap, all white water.
const POOL_M_S_PER_KM2: f32 = 0.6;
const POOL_SPEED_EXPONENT: f32 = 0.1;
/// A reach this steep runs as a riffle, [`RIFFLE_GAIN`] times faster than its pool would.
const RIFFLE_SLOPE: f32 = 0.04;
const RIFFLE_GAIN: f32 = 2.5;
/// Under this share of the speed a shallow wave runs at its depth (Froude 1): a reach faster than
/// that would stand in a hydraulic jump, which the drawn water has no shape for.
const MOST_FROUDE: f32 = 0.9;
const GRAVITY_M_S2: f32 = 9.81;
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
    /// [`LAKE`], and [`SPILLED`] on a lake node.
    lake: u8,
    /// Steps downstream to the sea, 0 at the sea; [`INLAND`] past [`MOUTH_NODES`] or where the
    /// stream leaves the window.
    to_sea: u8,
    /// One bit per step in [`STEPS`] its river also flows down as a delta's distributary.
    delta: u8,
    /// How far its channel's point stands off its lattice point, in 127ths of [`JITTER_M`]: hashed
    /// once in the solve, as every column of a channel asks it.
    off: [i8; 2],
    /// Where along its segment down to its parent a Fall's face starts, in 255ths, where the
    /// segment falls steeply enough for one: found in the solve from the ground under it.
    fall_at: u8,
}

#[cfg(test)]
const POOL_SLOPE: f32 = 0.005;

#[cfg(test)]
struct StreamSegment {
    at: (i32, i32),
    n: Node,
    p: Node,
    pa: (i32, i32),
    below: Option<(Node, (i32, i32))>,
}

pub(crate) struct Region {
    nodes: Vec<Node>,
}

impl Node {
    fn is_lake(&self) -> bool {
        self.lake & LAKE != 0
    }

    fn is_stream(&self) -> bool {
        self.lake & STREAM != 0
    }

    fn is_spilled(&self) -> bool {
        self.lake & SPILLED != 0
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

/// The pond cell this thread asked last, by Drainage, and its rim's lowest point if it holds one.
type LastPond = Option<(u64, [i32; 2], Option<f32>)>;

thread_local! {
    static LAST: RefCell<LastRegion> = const { RefCell::new(None) };
    static LAST_POND: RefCell<LastPond> = const { RefCell::new(None) };
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

    /// What this coarse solve's rivers carry into a finer window at `origin`, by the finer
    /// lattice's index, in its nodes: each river that crosses the window's edge inward puts what it
    /// gathered outside on the lowest edge node near where it crosses. Without it a river's width
    /// is the area inside each window, and two regions draw it at two widths across their border.
    fn inflow(&self, origin: [i32; 2], fine_m: f32, height: &[f32]) -> Vec<u32> {
        let mut inflow = vec![0u32; (WINDOW * WINDOW) as usize];
        let lo = origin.map(|o| o as f32 * fine_m);
        let hi = origin.map(|o| (o + WINDOW - 1) as f32 * fine_m);
        let inside =
            |c: [i32; 2]| (0..2).all(|k| (lo[k]..=hi[k]).contains(&(c[k] as f32 * self.node_m)));
        let node = |c: [i32; 2]| {
            let key = c.map(|v| v.div_euclid(REGION_NODES));
            self.region(key).node(c[0], c[1], key)
        };
        let down = |c: [i32; 2], n: Node| {
            let (di, dj) = STEPS[n.down as usize];
            [c[0] + di, c[1] + dj]
        };
        let per_node = ((self.node_m / fine_m) * (self.node_m / fine_m)) as u32;
        let first = lo.map(|m| (m / self.node_m).floor() as i32 - 1);
        let last = hi.map(|m| (m / self.node_m).ceil() as i32 + 1);
        // A river the coarse lattice takes out of the window and back in brings back what the
        // finer solve already gathered inside: that is not new water where it comes back.
        let mut returned: HashMap<[i32; 2], u32> = HashMap::new();
        let mut crossings = Vec::new();
        for cj in first[1]..=last[1] {
            for ci in first[0]..=last[0] {
                let c = [ci, cj];
                let n = node(c);
                if n.down == NO_PARENT || n.is_lake() {
                    continue;
                }
                let next = down(c, n);
                match (inside(c), inside(next)) {
                    (false, true) => crossings.push((c, n.catchment)),
                    (true, false) => {
                        let (mut at, mut on) = (next, node(next));
                        for _ in 0..REGION_NODES {
                            if on.down == NO_PARENT {
                                break;
                            }
                            let ahead = down(at, on);
                            if inside(ahead) {
                                *returned.entry(at).or_default() += n.catchment;
                                break;
                            }
                            (at, on) = (ahead, node(ahead));
                        }
                    }
                    _ => {}
                }
            }
        }
        for (c, catchment) in crossings {
            let gathered = catchment.saturating_sub(returned.get(&c).copied().unwrap_or(0));
            if gathered * per_node < STREAM_NODES {
                continue;
            }
            let at = c.map(|v| v as f32 * self.node_m);
            let cross = std::array::from_fn::<i32, 2, _>(|k| {
                ((at[k].clamp(lo[k], hi[k]) / fine_m).round() as i32 - origin[k])
                    .clamp(0, WINDOW - 1)
            });
            let along = usize::from(cross[0] == 0 || cross[0] == WINDOW - 1);
            let lowest = (-2..=2)
                .map(|d| {
                    let mut p = cross;
                    p[along] = (p[along] + d).clamp(0, WINDOW - 1);
                    (p[1] * WINDOW + p[0]) as usize
                })
                .min_by(|&a, &b| height[a].total_cmp(&height[b]))
                .unwrap();
            inflow[lowest] += gathered * per_node;
        }
        inflow
    }

    /// Where a channel's node `[i, j]` stands, in metres: off its lattice point by a hash of it, so
    /// channels joining nodes bend at each instead of running along the lattice.
    fn node_at(&self, n: Node, i: i32, j: i32) -> [f32; 2] {
        let off = |k: usize| JITTER_M * f32::from(n.off[k]) / 127.0;
        [
            i as f32 * self.node_m + off(0),
            j as f32 * self.node_m + off(1),
        ]
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

    /// Stream nodes stored by both `key` and its east or south neighbour: how many, how many of
    /// them differ in width by more than a fifth, and the worst share with its two widths.
    #[cfg(test)]
    pub(crate) fn border_widths(&self, key: [i32; 2]) -> (u32, u32, (f32, f32, f32)) {
        let mut worst = (0.0f32, 0.0, 0.0);
        let (mut compared, mut apart) = (0, 0);
        let a = self.solve(key);
        for across in [true, false] {
            let b_key = if across {
                [key[0] + 1, key[1]]
            } else {
                [key[0], key[1] + 1]
            };
            let b = self.solve(b_key);
            for along in -MARGIN..REGION_NODES + MARGIN {
                for over in REGION_NODES..REGION_NODES + MARGIN {
                    let (i, j) = if across { (over, along) } else { (along, over) };
                    let (i, j) = (i + key[0] * REGION_NODES, j + key[1] * REGION_NODES);
                    let (na, nb) = (a.node(i, j, key), b.node(i, j, b_key));
                    if !na.is_stream() || !nb.is_stream() || na.is_lake() || nb.is_lake() {
                        continue;
                    }
                    let (wa, wb) = (river_width_m(na.catchment), river_width_m(nb.catchment));
                    let off = (wa - wb).abs() / wa.max(wb);
                    if off > worst.0 {
                        worst = (off, wa, wb);
                    }
                    compared += 1;
                    apart += (off > 0.2) as u32;
                }
            }
        }
        (compared, apart, worst)
    }

    /// A region's stream nodes, and how many run beside another stream a node away that neither
    /// flows into it nor meets it at the next node: two streams side by side down one slope.
    #[cfg(test)]
    pub(crate) fn parallel_streams(&self, key: [i32; 2]) -> (u32, u32) {
        let region = self.solve(key);
        let at = |i: i32, j: i32| region.node(i, j, key);
        let down = |i: i32, j: i32, n: Node| {
            let (di, dj) = STEPS[n.down as usize];
            (i + di, j + dj)
        };
        let (mut streams, mut beside) = (0, 0);
        let base = key.map(|k| k * REGION_NODES);
        for j in base[1]..base[1] + REGION_NODES {
            for i in base[0]..base[0] + REGION_NODES {
                let n = at(i, j);
                if !n.is_stream() || n.is_lake() || n.down == NO_PARENT {
                    continue;
                }
                streams += 1;
                let nd = down(i, j, n);
                beside += STEPS.iter().any(|&(di, dj)| {
                    let (qi, qj) = (i + di, j + dj);
                    let q = at(qi, qj);
                    if !q.is_stream() || q.is_lake() || q.down == NO_PARENT || (qi, qj) == nd {
                        return false;
                    }
                    let qd = down(qi, qj, q);
                    qd != (i, j) && qd != nd && (qd.0 - nd.0).abs().max((qd.1 - nd.1).abs()) == 1
                }) as u32;
            }
        }
        (streams, beside)
    }

    /// Every flowing stream segment of a region's own nodes, with the segment below it where that
    /// one flows too: `(at, n, p, below)`.
    #[cfg(test)]
    fn segments(&self, key: [i32; 2]) -> Vec<StreamSegment> {
        let region = self.solve(key);
        let flowing = |n: Node| n.is_stream() && !n.is_lake() && n.down != NO_PARENT;
        let down = |(i, j): (i32, i32), n: Node| {
            let (di, dj) = STEPS[n.down as usize];
            (i + di, j + dj)
        };
        let base = key.map(|k| k * REGION_NODES);
        let mut out = Vec::new();
        for j in base[1]..base[1] + REGION_NODES {
            for i in base[0]..base[0] + REGION_NODES {
                let n = region.node(i, j, key);
                if !flowing(n) {
                    continue;
                }
                let pa = down((i, j), n);
                let p = region.node(pa.0, pa.1, key);
                let below = flowing(p).then(|| {
                    let qa = down(pa, p);
                    (pa, region.node(qa.0, qa.1, key), qa)
                });
                out.push(StreamSegment {
                    at: (i, j),
                    n,
                    p,
                    pa,
                    below: below.map(|(_, q, qa)| (q, qa)),
                });
            }
        }
        out
    }

    /// The most a stream's surface rises downstream, within a segment or where one segment hands
    /// on to the next, over a region's streams, and how many segments were checked.
    #[cfg(test)]
    pub(crate) fn surface_rise_m(&self, key: [i32; 2]) -> (u32, f32) {
        let length = |a: [f32; 2], b: [f32; 2]| libm::hypotf(b[0] - a[0], b[1] - a[1]);
        let (mut checked, mut rise) = (0, f32::MIN);
        for s in self.segments(key) {
            let (a, b) = (
                self.node_at(s.n, s.at.0, s.at.1),
                self.node_at(s.p, s.pa.0, s.pa.1),
            );
            let len = length(a, b);
            let at = |t: f32| water_surface_m(s.n, s.p, t, len);
            for k in 0..16 {
                rise = rise.max(at((k + 1) as f32 / 16.0) - at(k as f32 / 16.0));
            }
            if let Some((q, qa)) = s.below {
                let c = self.node_at(q, qa.0, qa.1);
                let next = water_surface_m(s.p, q, 0.0, length(b, c));
                rise = rise.max(next - at(1.0));
            }
            checked += 1;
        }
        (checked, rise)
    }

    /// The speed of each pool (a reach under 0.5% steep) and each delta's head under 1%, over a
    /// region's streams; a mouth down a steeper reach is a rapid.
    #[cfg(test)]
    pub(crate) fn pool_speeds_m_s(&self, key: [i32; 2]) -> Vec<f32> {
        self.segments(key)
            .into_iter()
            .filter_map(|s| {
                let (a, b) = (
                    self.node_at(s.n, s.at.0, s.at.1),
                    self.node_at(s.p, s.pa.0, s.pa.1),
                );
                let slope =
                    (s.n.level_m - s.p.level_m).max(0.0) / libm::hypotf(b[0] - a[0], b[1] - a[1]);
                (slope < POOL_SLOPE || (s.n.delta != 0 && slope < 2.0 * POOL_SLOPE)).then(|| {
                    let area_km2 = s.n.catchment as f32 * self.node_m * self.node_m * 1e-6;
                    stream_m_s(area_km2, slope, dig_m(river_width_m(s.n.catchment)))
                })
            })
            .collect()
    }

    /// River mouths on a low coast (a stream wide enough for a delta, a step from the sea and
    /// under 3 m above it), and how many of them fan into a delta.
    #[cfg(test)]
    pub(crate) fn low_mouths(&self, key: [i32; 2]) -> (u32, u32) {
        let (mut mouths, mut deltas) = (0, 0);
        for s in self.segments(key) {
            if s.n.to_sea == 1
                && s.n.level_m < self.sea_m + 3.0
                && river_width_m(s.n.catchment) >= DELTA_WIDTH_M
            {
                mouths += 1;
                deltas += (s.n.delta != 0) as u32;
            }
        }
        (mouths, deltas)
    }

    /// Bends where a channel passes through a node, and how many run straight on within 2 degrees.
    #[cfg(test)]
    pub(crate) fn straight_bends(&self, key: [i32; 2]) -> (u32, u32) {
        let (mut bends, mut straight) = (0, 0);
        for s in self.segments(key) {
            let Some((q, qa)) = s.below else { continue };
            let a = self.node_at(s.n, s.at.0, s.at.1);
            let b = self.node_at(s.p, s.pa.0, s.pa.1);
            let c = self.node_at(q, qa.0, qa.1);
            let (u, v) = ([b[0] - a[0], b[1] - a[1]], [c[0] - b[0], c[1] - b[1]]);
            let sin =
                (u[0] * v[1] - u[1] * v[0]) / (libm::hypotf(u[0], u[1]) * libm::hypotf(v[0], v[1]));
            bends += 1;
            straight += (sin.abs() < 0.035 && u[0] * v[0] + u[1] * v[1] > 0.0) as u32;
        }
        (bends, straight)
    }

    /// Puts each falling segment's face where the ground under it drops through the middle of
    /// the fall, so the upper pool ends at the real edge: a face at a hashed point left the pool
    /// running on over lower ground, walled in by the containment.
    fn place_falls(&self, nodes: &mut [Node], key: [i32; 2]) {
        let base = key.map(|k| k * REGION_NODES - MARGIN);
        let at = |li: i32, lj: i32| (lj * STORED + li) as usize;
        for lj in 0..STORED {
            for li in 0..STORED {
                let n = nodes[at(li, lj)];
                if !n.is_stream() || n.down == NO_PARENT {
                    continue;
                }
                let (di, dj) = STEPS[n.down as usize];
                let (pi, pj) = (li + di, lj + dj);
                if !(0..STORED).contains(&pi) || !(0..STORED).contains(&pj) {
                    continue;
                }
                let p = nodes[at(pi, pj)];
                let a = self.node_at(n, base[0] + li, base[1] + lj);
                let b = self.node_at(p, base[0] + pi, base[1] + pj);
                let length_m = libm::hypotf(b[0] - a[0], b[1] - a[1]);
                let drop = n.level_m - p.level_m;
                if drop < FALL_DROP_M || drop / length_m < FALL_SLOPE {
                    continue;
                }
                let middle = n.level_m - drop / 2.0;
                let ts: [f32; FALL_SAMPLES] =
                    std::array::from_fn(|k| (k as f32 + 0.5) / FALL_SAMPLES as f32);
                let mut ground = [0.0f32; FALL_SAMPLES];
                for (out, t) in ground.chunks_mut(4).zip(ts.chunks(4)) {
                    let x = std::array::from_fn(|k| a[0] + (b[0] - a[0]) * t[k]);
                    let z = std::array::from_fn(|k| a[1] + (b[1] - a[1]) * t[k]);
                    out.copy_from_slice(&(self.base)(x, z));
                }
                let face = drop / FALL_FACE_STEEPNESS / length_m;
                let crossing = ts
                    .iter()
                    .zip(ground)
                    .find(|&(_, g)| g < middle)
                    .map_or(1.0, |(&t, _)| t);
                let start = (crossing - face / 2.0).clamp(0.02, (0.98 - face).max(0.02));
                nodes[at(li, lj)].fall_at = (start * 255.0).round() as u8;
            }
        }
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
        let inflow = self
            .coarse
            .as_ref()
            .map_or_else(|| vec![0; size], |c| c.inflow(origin, self.node_m, &height));
        let on_edge = |i: i32, j: i32| i == 0 || j == 0 || i == WINDOW - 1 || j == WINDOW - 1;
        // The edge either side of where a river comes in is no outlet either, or the river runs
        // straight back out along the edge's lowest seed beside it.
        let mut entry = vec![false; size];
        for at in (0..size).filter(|&at| inflow[at] > 0) {
            let (ci, cj) = (at as i32 % WINDOW, at as i32 / WINDOW);
            for dj in -ENTRY_NODES..=ENTRY_NODES {
                for di in -ENTRY_NODES..=ENTRY_NODES {
                    let (i, j) = (ci + di, cj + dj);
                    if (0..WINDOW).contains(&i) && (0..WINDOW).contains(&j) && on_edge(i, j) {
                        entry[idx(i, j)] = true;
                    }
                }
            }
        }
        for j in 0..WINDOW {
            for i in 0..WINDOW {
                let h = height[idx(i, j)];
                if h <= self.sea_m {
                    seed(i, j, self.sea_m, &mut level);
                } else if entry[idx(i, j)] {
                    // A river coming in is no outlet: the flood reaches it from inside, so it
                    // drains inward carrying what the coarse solve gathered beyond the window.
                } else if on_edge(i, j) {
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
                if !level[n].is_nan() || (entry[n] && on_edge(ci, cj)) {
                    continue;
                }
                let f = height[n].max(here + EPS_M);
                level[n] = f;
                parent[n] = opposite(k);
                heap.push(Reverse((sortable(f), n)));
            }
        }
        let lake = lakes(&height, &level);
        for _ in 0..CAPTURE_ROUNDS {
            if !capture(&order, &mut parent, &level, &lake, &inflow) {
                break;
            }
        }
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
        let mut spilled = vec![false; size];
        for &at in &order {
            if lake[at] && parent[at] != NO_PARENT {
                let (di, dj) = STEPS[parent[at] as usize];
                let p = idx((at as i32 % WINDOW) + di, (at as i32 / WINDOW) + dj);
                spilled[at] = !lake[p] || spilled[p];
            }
        }
        let (catchment, stream, fed) = gather(&order, &parent, &inflow);
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
                let (gi, gj) = (
                    key[0] * REGION_NODES + li - MARGIN,
                    key[1] * REGION_NODES + lj - MARGIN,
                );
                let off = |salt: i32| {
                    let h = crate::noise::lattice3(gi, gj, salt, JITTER_SEED);
                    (254.0 * h - 127.0).round() as i8
                };
                nodes.push(Node {
                    #[cfg(test)]
                    height_m: height[at],
                    level_m: level[at],
                    catchment: catchment[at],
                    down: parent[at],
                    lake: if lake[at] {
                        LAKE | (spilled[at] as u8 * SPILLED)
                    } else {
                        0
                    } | if stream[at] && !fed[at] { HEAD } else { 0 }
                        | if stream[at] { STREAM } else { 0 },
                    to_sea: to_sea[at],
                    delta: delta[at],
                    off: [off(0), off(1)],
                    fall_at: 0,
                });
            }
        }
        self.place_falls(&mut nodes, key);
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

        // The water that may stand here, each with how far above the lowest bed a channel leaves
        // here it may stand: a column takes no water whose own bed another channel has cut far
        // below, or it stood tens of metres deep over that channel, walled in by the containment.
        let mut held = Held::default();
        for (di, dj) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            let n = region.node(i0 + di, j0 + dj, key);
            if n.is_lake() && n.level_m > self.sea_m {
                held.push(n.level_m, LAKE_HOLD_M, false);
            }
        }
        let mut lowest_bed_m = f32::MAX;

        let (mut cut_m, mut river_cut_m) = (0.0f32, 0.0f32);
        for dj in -STENCIL + 1..=STENCIL {
            for di in -STENCIL + 1..=STENCIL {
                let (ni, nj) = (i0 + di, j0 + dj);
                let n = region.node(ni, nj, key);
                if n.down == NO_PARENT {
                    continue;
                }
                if !n.is_stream() {
                    // Too small for a stream of its own, but a step from the sea a delta beside
                    // it may still feed it a distributary.
                    if n.to_sea == 1
                        && let Some(w) = delta_fed_m(&region, key, (ni, nj))
                    {
                        let (si, sj) = STEPS[n.down as usize];
                        let p = region.node(ni + si, nj + sj, key);
                        let c =
                            self.side_channel((ni, nj), n, (si, sj), p, w, (x_m, z_m), ground_m);
                        take_channel(c, true, &mut cut_m, &mut river_cut_m, &mut held);
                    }
                    continue;
                }
                let (si, sj) = STEPS[n.down as usize];
                let p = region.node(ni + si, nj + sj, key);
                if p.is_lake() && n.is_lake() {
                    continue;
                }
                let a = self.node_at(n, ni, nj);
                let b = self.node_at(p, ni + si, nj + sj);
                let ab = [b[0] - a[0], b[1] - a[1]];
                let along = |x: f32, z: f32| {
                    (((x - a[0]) * ab[0] + (z - a[1]) * ab[1]) / (ab[0] * ab[0] + ab[1] * ab[1]))
                        .clamp(0.0, 1.0)
                };
                if n.to_sea == INLAND
                    && n.delta == 0
                    && out_of_reach(n, p, a, ab, along(x_m, z_m), (x_m, z_m))
                {
                    continue;
                }
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
                if n.lake & HEAD != 0 {
                    width = width.max(spring_width_m(t, ab));
                }
                let length_m = libm::hypotf(ab[0], ab[1]);
                let surface = water_surface_m(n, p, t, length_m);
                let (dug, wet) = fall_bed(n, p, t, length_m);
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
                        take_channel(c, true, &mut cut_m, &mut river_cut_m, &mut held);
                    }
                }
                if edge >= reach_m {
                    continue;
                }
                let full_dig = dig_m(width);
                let dig = full_dig * dug;
                let target = surface - dig + edge * BANK_SLOPE;
                let fade = 1.0 - smoothstep(0.66 * reach_m, reach_m, edge);
                if edge < 0.66 * reach_m {
                    lowest_bed_m = lowest_bed_m.min(target);
                }
                held.bed(target, fade);
                let cut = (ground_m - target).max(0.0) * fade;
                cut_m = cut_m.max(cut);
                if n.catchment >= RIVER_NODES {
                    river_cut_m = river_cut_m.max(cut);
                }
                // A brook's water stays in the channel it cut: past it the slope stands a few
                // centimetres either side of the surface, and its Voxels would chequer wet and dry.
                let band = if n.catchment >= RIVER_NODES { 2.0 } else { 1.0 };
                // The full depth's band, so the water keeps its width over a Fall's sill.
                if wet && edge * BANK_SLOPE < full_dig * band {
                    held.push(surface, full_dig * band + HOLD_SLACK_M, true);
                }
            }
        }
        if self.coarse.is_some()
            && let Some((cut, surface)) = self.pond(x_m, z_m, ground_m)
        {
            cut_m = cut_m.max(cut);
            held.push(surface, POND_DEPTH_M + HOLD_SLACK_M, true);
        }
        // A river or a pond also stands no deeper than its own over the ground itself, asked
        // here and not of the caller, so the level is the same whatever ground is asked about:
        // between two lattice nodes the ground may fall away under a river the lattice saw level.
        let ground_here_m = if held.iter().any(|&(_, _, river)| river) {
            held.dug((self.base)([x_m; 4], [z_m; 4])[0])
        } else {
            f32::MAX
        };
        for &(surface, depth_m, river) in held.iter() {
            let bed_m = if river { ground_here_m } else { lowest_bed_m };
            if surface - bed_m <= depth_m {
                level_m = Some(level_m.map_or(surface, |l| l.max(surface)));
            }
        }
        Wet {
            level_m,
            cut_m,
            river_cut_m,
        }
    }

    /// The cut and water level of the pond over `(x_m, z_m)`, if one stands there. Each
    /// [`POND_CELL_M`] cell may hold one, at a hashed point and size, where the ground round it is
    /// near level and well above the sea: a bowl whose water stands just under the lowest of its
    /// rim, so it needs no lip. Rivers' and lakes' own water wins where they meet one.
    fn pond(&self, x_m: f32, z_m: f32, ground_m: f32) -> Option<(f32, f32)> {
        let cell = [
            (x_m / POND_CELL_M).floor() as i32,
            (z_m / POND_CELL_M).floor() as i32,
        ];
        let hash = |salt: i32| crate::noise::lattice3(cell[0], cell[1], salt, POND_SEED);
        if hash(0) >= POND_CHANCE {
            return None;
        }
        let radius = POND_RADIUS_M.0 + (POND_RADIUS_M.1 - POND_RADIUS_M.0) * hash(1) * hash(1);
        let margin = POND_CELL_M / 2.0 - POND_RADIUS_M.1 - POND_BANK_M;
        let centre = [
            (cell[0] as f32 + 0.5) * POND_CELL_M + margin * (2.0 * hash(2) - 1.0),
            (cell[1] as f32 + 0.5) * POND_CELL_M + margin * (2.0 * hash(3) - 1.0),
        ];
        let d = libm::hypotf(x_m - centre[0], z_m - centre[1]);
        if d >= radius + POND_BANK_M {
            return None;
        }
        // Every column of a pond asks its rim; this thread keeps the last cell's.
        let hit = LAST_POND.with(|l| {
            l.borrow()
                .filter(|(id, c, _)| *id == self.id && *c == cell)
                .map(|(_, _, low)| low)
        });
        let low = hit.unwrap_or_else(|| {
            let rim = radius + POND_BANK_M / 2.0;
            let ring = (self.base)(
                [centre[0] + rim, centre[0] - rim, centre[0], centre[0]],
                [centre[1], centre[1], centre[1] + rim, centre[1] - rim],
            );
            let (low, high) = ring
                .iter()
                .fold((f32::MAX, f32::MIN), |(l, h), &v| (l.min(v), h.max(v)));
            let low =
                (high - low <= POND_LEVEL_M && low >= self.sea_m + POND_ABOVE_SEA_M).then_some(low);
            LAST_POND.with(|l| *l.borrow_mut() = Some((self.id, cell, low)));
            low
        })?;
        let surface = low - POND_BELOW_RIM_M;
        let r = d / radius;
        let depth = POND_DEPTH_M * (1.0 - r * r).max(0.0);
        let bank = (d - radius).max(0.0) * BANK_SLOPE;
        let target = surface - depth + bank;
        // Faded out over the bank, as a stream's valley is, so the ground meets its own unbroken.
        let fade = 1.0 - smoothstep(radius, radius + POND_BANK_M, d);
        let cut = (ground_m - target).max(0.0) * fade;
        Some((cut, if d < radius { surface } else { f32::MIN }))
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
        let a = self.node_at(n, at.0, at.1);
        let b = self.node_at(p, at.0 + si, at.1 + sj);
        let ab = [b[0] - a[0], b[1] - a[1]];
        let along = |x: f32, z: f32| {
            (((x - a[0]) * ab[0] + (z - a[1]) * ab[1]) / (ab[0] * ab[0] + ab[1] * ab[1]))
                .clamp(0.0, 1.0)
        };
        let (amount_m, long) = meander_m(width_m);
        let (qx, qz) = meander_at(x_m, z_m, amount_m, long);
        let t = along(qx, qz);
        let (cx, cz) = (a[0] + ab[0] * t, a[1] + ab[1] * t);
        let d = libm::hypotf(qx - cx, qz - cz);
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

    /// Where water moves over `(x_m, z_m)`, in m/s along x and z: down the stream whose water
    /// stands highest there, or drifting across a lake toward its outlet. Zero where neither
    /// reaches. Walks the same segments as [`Self::wet`], out of its loop, which every column
    /// asks: only a wet column's mesh asks this.
    pub(crate) fn flow(&self, x_m: f32, z_m: f32) -> [f32; 2] {
        let (u, v) = (x_m / self.node_m, z_m / self.node_m);
        let (i0, j0) = (u.floor() as i32, v.floor() as i32);
        let key = [i0.div_euclid(REGION_NODES), j0.div_euclid(REGION_NODES)];
        // Only the water `wet` lets stand here runs: a channel it turned away for standing over a
        // lower bed has no water here to move.
        let Some(level) = self.wet(x_m, z_m, f32::MIN).level_m else {
            return [0.0; 2];
        };
        let region = self.region(key);
        let mut best: Option<(f32, [f32; 2])> = None;
        let mut take = |found: Option<(f32, [f32; 2])>| {
            if let Some((surface, _)) = found
                && surface <= level + 1e-3
                && best.is_none_or(|(l, _)| surface > l)
            {
                best = found;
            }
        };
        take(self.lake_drift(&region, key, (u, v)));
        for dj in -STENCIL + 1..=STENCIL {
            for di in -STENCIL + 1..=STENCIL {
                let at = (i0 + di, j0 + dj);
                let n = region.node(at.0, at.1, key);
                if n.down == NO_PARENT {
                    continue;
                }
                let step = STEPS[n.down as usize];
                let p = region.node(at.0 + step.0, at.1 + step.1, key);
                if !n.is_stream() {
                    if n.to_sea == 1
                        && let Some(w) = delta_fed_m(&region, key, at)
                    {
                        let side = Channel::side(w);
                        take(self.channel_flow(at, n, step, p, side, (x_m, z_m)));
                    }
                    continue;
                }
                if p.is_lake() && n.is_lake() {
                    continue;
                }
                let main = Channel {
                    widths_m: (river_width_m(n.catchment), river_width_m(p.catchment)),
                    mouth: n.to_sea != INLAND,
                    band: if n.catchment >= RIVER_NODES { 2.0 } else { 1.0 },
                };
                take(self.channel_flow(at, n, step, p, main, (x_m, z_m)));
                let side = Channel::side(river_width_m(n.catchment) * DISTRIBUTARY);
                for step in distributaries(n) {
                    let q = region.node(at.0 + step.0, at.1 + step.1, key);
                    take(self.channel_flow(at, n, step, q, side, (x_m, z_m)));
                }
            }
        }
        best.map_or([0.0; 2], |(_, flow)| flow)
    }

    /// [`Self::lake_drift`] at `(u, v)` on the coarse lattice, without its level.
    fn wide_drift(&self, (u, v): (f32, f32)) -> Option<[f32; 2]> {
        let coarse = self.coarse.as_ref()?;
        let scale = self.node_m / coarse.node_m;
        let (u, v) = (u * scale, v * scale);
        let key = [
            (u.floor() as i32).div_euclid(REGION_NODES),
            (v.floor() as i32).div_euclid(REGION_NODES),
        ];
        Some(coarse.lake_drift(&coarse.region(key), key, (u, v))?.1)
    }

    /// The level and drift of the highest lake on a corner of the lattice cell holding `(u, v)`,
    /// in nodes: each lake corner's way down its flood tree, which leads out by the spill, blended
    /// across the cell, and faster the more of the lake has drained through it.
    fn lake_drift(
        &self,
        region: &Region,
        key: [i32; 2],
        (u, v): (f32, f32),
    ) -> Option<(f32, [f32; 2])> {
        let (i0, j0) = (u.floor() as i32, v.floor() as i32);
        let (fu, fv) = (u - i0 as f32, v - j0 as f32);
        let corners = [(0, 0), (1, 0), (0, 1), (1, 1)].map(|(di, dj)| {
            let w = (if di == 0 { 1.0 - fu } else { fu }) * (if dj == 0 { 1.0 - fv } else { fv });
            (region.node(i0 + di, j0 + dj, key), w)
        });
        let level_m = corners
            .iter()
            .filter(|(n, _)| n.is_lake() && n.level_m > self.sea_m)
            .map(|(n, _)| n.level_m)
            .reduce(f32::max)?;
        // A lake wider than a window leaves it by whichever edge each window sees as nearest
        // its spill, so two regions would drift it apart; the coarse solve sees such a lake whole.
        let wide = std::cell::OnceCell::new();
        let (mut sum, mut weight) = ([0.0f32; 2], 0.0f32);
        for (n, w) in corners {
            if !n.is_lake() {
                continue;
            }
            let drift = if n.is_spilled() || self.coarse.is_none() {
                if n.down == NO_PARENT {
                    continue;
                }
                let (di, dj) = STEPS[n.down as usize];
                let speed = (LAKE_DRIFT_M_S.0 * (n.catchment as f32).sqrt()).min(LAKE_DRIFT_M_S.1);
                let scale = speed / ((di * di + dj * dj) as f32).sqrt();
                [scale * di as f32, scale * dj as f32]
            } else {
                match *wide.get_or_init(|| self.wide_drift((u, v))) {
                    Some(d) => d,
                    None => continue,
                }
            };
            sum = [sum[0] + w * drift[0], sum[1] + w * drift[1]];
            weight += w;
        }
        let drift = if weight > 0.0 {
            sum.map(|c| c / weight)
        } else {
            [0.0; 2]
        };
        Some((level_m, drift))
    }

    /// The water level and Flow of `channel` from node `n` at `at` down `step` to `p`, at
    /// `(x_m, z_m)`, if its water reaches there, as [`Self::wet`] draws that water: along the
    /// meandered centre line, faster down a steeper and deeper channel.
    fn channel_flow(
        &self,
        at: (i32, i32),
        n: Node,
        (si, sj): (i32, i32),
        p: Node,
        channel: Channel,
        (x_m, z_m): (f32, f32),
    ) -> Option<(f32, [f32; 2])> {
        let a = self.node_at(n, at.0, at.1);
        let b = self.node_at(p, at.0 + si, at.1 + sj);
        let ab = [b[0] - a[0], b[1] - a[1]];
        let length_m = (ab[0] * ab[0] + ab[1] * ab[1]).sqrt();
        let along = |x: f32, z: f32| {
            (((x - a[0]) * ab[0] + (z - a[1]) * ab[1]) / (length_m * length_m)).clamp(0.0, 1.0)
        };
        let (wn, wp) = channel.widths_m;
        let s0 = along(x_m, z_m);
        let (amount_m, long) = meander_m(wn * (1.0 - s0) + wp * s0);
        let (qx, qz) = meander_at(x_m, z_m, amount_m, long);
        let t = along(qx, qz);
        let (dx, dz) = (qx - (a[0] + ab[0] * t), qz - (a[1] + ab[1] * t));
        let mut width = wn * (1.0 - t) + wp * t;
        let mut reach_m = valley_reach_m(width);
        if channel.mouth {
            width = mouth_width_m(width, sea_steps(n, p, t));
            reach_m = valley_reach_m(width).min(CHANNEL_REACH_M - width / 2.0);
        }
        if n.lake & HEAD != 0 {
            width = width.max(spring_width_m(t, ab));
        }
        let edge = ((dx * dx + dz * dz).sqrt() - width / 2.0).max(0.0);
        let (_, wet) = fall_bed(n, p, t, length_m);
        let dig = dig_m(width);
        if !wet || edge >= reach_m || edge * BANK_SLOPE >= dig * channel.band {
            return None;
        }
        let surface = water_surface_m(n, p, t, length_m);
        let slope = (n.level_m - p.level_m).max(0.0) / length_m;
        let catchment = n.catchment as f32 * (1.0 - t) + p.catchment as f32 * t;
        let area_km2 = catchment * self.node_m * self.node_m * 1e-6;
        let speed = stream_m_s(area_km2, slope, dig);
        let [ux, uz] = meandered_along(x_m, z_m, (amount_m, long), ab);
        Some((surface, [ux * speed, uz * speed]))
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
/// Where a stream runs beside another a node away that stands no higher, it joins it, the larger
/// of two: an even slope gathers every line of nodes alike, and D8 drew them side by side down it.
/// A node only joins one the flood reached first, so every parent still comes first in `order`.
fn capture(
    order: &[usize],
    parent: &mut [u8],
    level: &[f32],
    lake: &[bool],
    inflow: &[u32],
) -> bool {
    let (catchment, stream, _) = gather(order, parent, inflow);
    let mut rank = vec![0u32; parent.len()];
    for (k, &at) in order.iter().enumerate() {
        rank[at] = k as u32;
    }
    let step = |at: usize, k: usize| {
        let (di, dj) = STEPS[k];
        let (i, j) = (at as i32 % WINDOW + di, at as i32 / WINDOW + dj);
        ((0..WINDOW).contains(&i) && (0..WINDOW).contains(&j)).then(|| (j * WINDOW + i) as usize)
    };
    let flowing: Vec<bool> = (0..parent.len())
        .map(|at| stream[at] && !lake[at] && parent[at] != NO_PARENT)
        .collect();
    let mut moved = vec![false; parent.len()];
    let mut joined = false;
    for &at in order.iter().rev() {
        if !flowing[at] || moved[at] {
            continue;
        }
        let down = step(at, parent[at] as usize);
        let mut best: Option<(u32, usize)> = None;
        for k in 0..STEPS.len() {
            let Some(q) = step(at, k) else { continue };
            if Some(q) == down || !flowing[q] || rank[q] > rank[at] || level[q] > level[at] {
                continue;
            }
            let q_down = step(q, parent[q] as usize);
            if q_down == Some(at) || q_down == down {
                continue;
            }
            if best.is_none_or(|(c, _)| catchment[q] > c) {
                best = Some((catchment[q], k));
            }
        }
        if let Some((_, k)) = best {
            // The line below no longer carries this water: it joins nothing else this round, as
            // what it still gathers is only known once the catchment is counted again.
            let mut below = down;
            while let Some(b) = below.filter(|&b| !moved[b] && parent[b] != NO_PARENT) {
                moved[b] = true;
                below = step(b, parent[b] as usize);
            }
            parent[at] = k as u8;
            joined = true;
        }
    }
    joined
}

/// Each node's catchment in nodes, with what comes in at the window's edge, and which nodes carry
/// a stream and which of those are fed by one above.
fn gather(order: &[usize], parent: &[u8], inflow: &[u32]) -> (Vec<u32>, Vec<bool>, Vec<bool>) {
    let size = parent.len();
    let up_of = |at: usize| {
        let (di, dj) = STEPS[parent[at] as usize];
        ((at as i32 / WINDOW + dj) * WINDOW + at as i32 % WINDOW + di) as usize
    };
    let mut catchment: Vec<u32> = inflow.iter().map(|&n| 1 + n).collect();
    for &at in order.iter().rev() {
        if parent[at] != NO_PARENT {
            catchment[up_of(at)] += catchment[at];
        }
    }
    // A stream starts only where water gathers: in a hollow two or more nodes drain into, or
    // once a whole slope's worth has run down one line. On an even slope every line of nodes
    // gathers alike, and starting them all at one catchment drew brooks side by side.
    let mut inflows = vec![0u8; size];
    for at in 0..size {
        if parent[at] != NO_PARENT {
            inflows[up_of(at)] = inflows[up_of(at)].saturating_add(1);
        }
    }
    let mut stream = vec![false; size];
    let mut fed = vec![false; size];
    for &at in order.iter().rev() {
        stream[at] = fed[at]
            || (catchment[at] >= STREAM_NODES
                && (inflows[at] >= 2 || catchment[at] >= SLOPE_STREAM_NODES));
        if stream[at] && parent[at] != NO_PARENT {
            fed[up_of(at)] = true;
        }
    }
    (catchment, stream, fed)
}

fn opposite(step: usize) -> u8 {
    (STEPS.len() - 1 - step) as u8
}

/// A stream's width from its spring pool, a fraction `t` down its first segment `ab`.
/// A stream's speed from its catchment, its reach's slope and its depth: a pool's from discharge,
/// faster down a riffle, never past Froude 0.9.
fn stream_m_s(area_km2: f32, slope: f32, dig_m: f32) -> f32 {
    let pool = POOL_M_S_PER_KM2 * libm::powf(area_km2, POOL_SPEED_EXPONENT);
    let riffle = 1.0 + (RIFFLE_GAIN - 1.0) * (slope / RIFFLE_SLOPE).min(1.0);
    let froude = MOST_FROUDE * libm::sqrtf(GRAVITY_M_S2 * dig_m);
    (pool * riffle)
        .min(froude)
        .clamp(STREAM_M_S.0, STREAM_M_S.1)
}

/// Whether an inland segment from `n` to `p` is too far from `(x, z)` to cut it however its
/// meander bends, at `s` along it unbent: the meander's noise is the costliest part of a column,
/// and most segments in the stencil are far from it.
fn out_of_reach(n: Node, p: Node, a: [f32; 2], ab: [f32; 2], s: f32, (x, z): (f32, f32)) -> bool {
    let mut widest_m = river_width_m(n.catchment).max(river_width_m(p.catchment));
    if n.lake & HEAD != 0 {
        widest_m = widest_m.max(SPRING_WIDTH_M);
    }
    let bend_m = meander_m(widest_m).0 * core::f32::consts::SQRT_2;
    let (dx, dz) = (x - a[0] - ab[0] * s, z - a[1] - ab[1] * s);
    let far_m = valley_reach_m(widest_m) + widest_m / 2.0 + bend_m;
    dx * dx + dz * dz >= far_m * far_m
}

fn spring_width_m(t: f32, ab: [f32; 2]) -> f32 {
    let from_m = t * libm::hypotf(ab[0], ab[1]);
    SPRING_WIDTH_M * (1.0 - from_m / SPRING_WIDTH_M).max(0.0)
}

/// A segment dropping this far and this steeply drops all at once, as one Fall over a cliff,
/// instead of a staircase of one-Voxel steps each held by a lip.
const FALL_DROP_M: f32 = 4.0;
const FALL_SLOPE: f32 = 0.1;
/// How steep a Fall's face is, rise over run: steep enough that its Voxels stand as a wall of
/// several, while the heightfield stays a slope, as every cliff it draws is.
const FALL_FACE_STEEPNESS: f32 = 3.5;
/// How much deeper than its own channel's band a stream's water may stand over a lower bed.
const HOLD_SLACK_M: f32 = 1.0;
/// How deep a delta's side channel may stand over the ground: a distributary's depth and slack.
const SIDE_HOLD_M: f32 = 4.0;
/// How far above the lowest channel bed in reach a lake's water may stand: an inflow's bed lies a
/// stream's depth under the lake, an outlet's canyon far more.
const LAKE_HOLD_M: f32 = 4.0;

/// The water a column may take, as surfaces with how deep each may stand over the lowest bed.
struct Held {
    /// Surface, how deep it may stand, and whether it is a river's, which the ground also bounds.
    items: [(f32, f32, bool); 32],
    len: usize,
    /// Each valley in reach as its bed here and how much of its cut reaches here.
    beds: [(f32, f32); 64],
    beds_len: usize,
}

impl Default for Held {
    fn default() -> Self {
        Held {
            items: [(0.0, 0.0, false); 32],
            len: 0,
            beds: [(0.0, 0.0); 64],
            beds_len: 0,
        }
    }
}

impl Held {
    fn push(&mut self, surface_m: f32, depth_m: f32, river: bool) {
        if self.len < self.items.len() {
            self.items[self.len] = (surface_m, depth_m, river);
            self.len += 1;
        }
    }

    fn bed(&mut self, target_m: f32, fade: f32) {
        if self.beds_len < self.beds.len() {
            self.beds[self.beds_len] = (target_m, fade);
            self.beds_len += 1;
        }
    }

    /// Ground standing at `base_m` once every valley in reach has cut it.
    fn dug(&self, base_m: f32) -> f32 {
        let cut = self.beds[..self.beds_len]
            .iter()
            .map(|&(target, fade)| (base_m - target).max(0.0) * fade)
            .fold(0.0f32, f32::max);
        base_m - cut
    }

    fn iter(&self) -> impl Iterator<Item = &(f32, f32, bool)> {
        self.items[..self.len].iter()
    }
}

/// Over how far a stream's bed rises to a Fall's lip and sinks again past its foot.
const SILL_M: f32 = 3.0;
/// How many points along a falling segment the solve samples the ground at, a multiple of four.
const FALL_SAMPLES: usize = 16;

/// The water's surface a fraction `t` down the segment from `n` at node `at` to `p`: straight
/// between their levels, or held at the upper level and dropped down one steep face where the
/// segment falls steeply enough to make a waterfall.
fn water_surface_m(n: Node, p: Node, t: f32, length_m: f32) -> f32 {
    let level = match fall_face(n, p, length_m) {
        Some((at_t, face)) => {
            let s = ((t - at_t) / face).clamp(0.0, 1.0);
            n.level_m + (p.level_m - n.level_m) * s
        }
        None => n.level_m * (1.0 - t) + p.level_m * t,
    };
    level - SURFACE_BELOW_M
}

/// Where a segment that falls steeply enough has its Fall: how far along it the face starts and
/// how much of it the face spans, as shares of the segment.
fn fall_face(n: Node, p: Node, length_m: f32) -> Option<(f32, f32)> {
    let drop = n.level_m - p.level_m;
    if drop < FALL_DROP_M || drop / length_m < FALL_SLOPE {
        return None;
    }
    Some((
        f32::from(n.fall_at) / 255.0,
        drop / FALL_FACE_STEEPNESS / length_m,
    ))
}

/// How much of a stream's bed is dug under its surface `t` along the segment, and whether the
/// water is there at all. A Fall's face is bare rock the water pours over, not water stepping
/// down it: the bed rises to a sill flush with the pool over [`SILL_M`] before the lip, stands
/// dry down the face, and sinks to the full depth again over as much past its foot, so the
/// ground stays a slope.
fn fall_bed(n: Node, p: Node, t: f32, length_m: f32) -> (f32, bool) {
    let Some((at_t, face)) = fall_face(n, p, length_m) else {
        return (1.0, true);
    };
    let along_m = t * length_m;
    let (top_m, foot_m) = (at_t * length_m, (at_t + face) * length_m);
    if along_m < top_m {
        (smoothstep(0.0, SILL_M, top_m - along_m), true)
    } else if along_m <= foot_m {
        (0.0, false)
    } else {
        (smoothstep(0.0, SILL_M, along_m - foot_m), true)
    }
}

/// Widens with the square root of catchment, as rivers do; a brook's banks start at the river's
/// least width and fall away toward its head, never below [`BROOK_WIDTH_M`].
fn river_width_m(catchment: u32) -> f32 {
    let bank_m = 1.5 * (catchment as f32 / RIVER_NODES as f32).min(1.0);
    (bank_m + 0.0045 * (catchment as f32 * NODE_M * NODE_M).sqrt()).clamp(BROOK_WIDTH_M, 48.0)
}

/// How far under its surface a stream's bed lies: a river's as it was, and a brook's shallower
/// with its width, down to [`BROOK_DIG_M`] at the narrowest, so its banks meet the ground close
/// in and a mountain stream still runs as a stream and not a film.
fn dig_m(width_m: f32) -> f32 {
    let river = |w: f32| 1.2 + 0.25 * w.sqrt();
    let least_m = river_width_m(RIVER_NODES);
    if width_m >= least_m {
        return river(width_m);
    }
    let t = (width_m - BROOK_WIDTH_M) / (least_m - BROOK_WIDTH_M);
    BROOK_DIG_M + t * (river(least_m) - BROOK_DIG_M)
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
    held: &mut Held,
) {
    *cut_m = cut_m.max(cut);
    if river {
        *river_cut_m = river_cut_m.max(cut);
    }
    if surface > f32::MIN {
        held.push(surface, SIDE_HOLD_M, true);
    }
}

/// A stream's channel down one segment, as [`Drainage::channel_flow`] reads it.
#[derive(Clone, Copy)]
struct Channel {
    /// Its width at the segment's upper and lower node, before any mouth widens it.
    widths_m: (f32, f32),
    /// Whether it widens into a mouth near the sea.
    mouth: bool,
    /// How far up its banks its water stands, against its dig.
    band: f32,
}

impl Channel {
    /// A delta's distributary of `width_m`, as [`Drainage::side_channel`] cuts it.
    fn side(width_m: f32) -> Self {
        Channel {
            widths_m: (width_m, width_m),
            mouth: true,
            band: 2.0,
        }
    }
}

/// The unit direction down segment `ab` at `(x_m, z_m)`, turned to follow the meander: a channel
/// is where the warped point lies on the segment, so it runs across the gradient of the warped
/// point's distance from the segment's line.
fn meandered_along(x_m: f32, z_m: f32, (amount_m, long): (f32, f32), ab: [f32; 2]) -> [f32; 2] {
    const H_M: f32 = 0.5;
    let q = |x: f32, z: f32| meander_at(x, z, amount_m, long);
    let off = |(x, z): (f32, f32)| x * -ab[1] + z * ab[0];
    let gx = (off(q(x_m + H_M, z_m)) - off(q(x_m - H_M, z_m))) / (2.0 * H_M);
    let gz = (off(q(x_m, z_m + H_M)) - off(q(x_m, z_m - H_M))) / (2.0 * H_M);
    let (mut tx, mut tz) = (gz, -gx);
    if tx * ab[0] + tz * ab[1] < 0.0 {
        (tx, tz) = (-tx, -tz);
    }
    let len = (tx * tx + tz * tz).sqrt();
    if len < 1e-6 {
        let len = (ab[0] * ab[0] + ab[1] * ab[1]).sqrt();
        return [ab[0] / len, ab[1] / len];
    }
    [tx / len, tz / len]
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
            if !n.is_stream() {
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
                    .any(|di| region.node(i + di, j + dj, key).is_stream())
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
    fn every_stream_rises_from_a_spring_pool_at_its_head() {
        let d = drainage();
        let key = [0, 0];
        let region = d.solve(key);
        let inner = |i: i32, j: i32| {
            (8..REGION_NODES - 8).contains(&i) && (8..REGION_NODES - 8).contains(&j)
        };
        let (mut heads, mut pooled) = (0, 0);
        for (i, j, n) in region_nodes(&d, key) {
            if !inner(i, j) || !n.is_stream() || n.is_lake() {
                continue;
            }
            let fed = STEPS.iter().any(|&(di, dj)| {
                let c = region.node(i + di, j + dj, key);
                c.down != NO_PARENT && c.is_stream() && STEPS[c.down as usize] == (-di, -dj)
            });
            assert_eq!(n.lake & HEAD != 0, !fed, "the head flag at {i},{j}");
            if fed {
                continue;
            }
            heads += 1;
            // Wet a pool's radius out from the head on every side, where a brook alone is not.
            let [x, z] = d.node_at(n, i, j);
            let r = SPRING_WIDTH_M / 2.0 - 0.5;
            pooled += usize::from(
                [(r, 0.0), (-r, 0.0), (0.0, r), (0.0, -r)]
                    .iter()
                    .filter(|(dx, dz)| d.wet(x + dx, z + dz, f32::MAX).level_m.is_some())
                    .count()
                    >= 3,
            );
        }
        assert!(heads > 10, "{heads} heads");
        assert!(
            pooled * 10 >= heads * 9,
            "{pooled} of {heads} heads stand in a spring pool"
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
        // A brook's wet band, from bank to bank, stays narrow: its 1-in-2 banks reach out twice
        // its depth each side.
        let w = river_width_m(STREAM_NODES);
        assert!(
            w + 2.0 * dig_m(w) / BANK_SLOPE <= 8.5,
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
            assert!(libm::hypotf(qx - x, qz - z) <= MEANDER_M * 2f32.sqrt() + 1e-2);
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

    /// Points jittered off the lattice over the four regions round the origin.
    fn jittered(n: i32, step_m: f32) -> impl Iterator<Item = (f32, f32)> {
        (-n..n).flat_map(move |j| {
            (-n..n).map(move |i| {
                let h = crate::noise::cell_hash(i, j, 0x5f1);
                ((i as f32 + h[0]) * step_m, (j as f32 + h[1]) * step_m)
            })
        })
    }

    fn speed(f: [f32; 2]) -> f32 {
        (f[0] * f[0] + f[1] * f[1]).sqrt()
    }

    #[test]
    fn flow_runs_down_every_stream_and_dry_ground_has_none() {
        let d = drainage();
        let (mut streams, mut down) = (0, 0);
        for (x, z) in jittered(400, 5.0) {
            let f = d.flow(x, z);
            let Some(level) = d.wet(x, z, f32::MIN).level_m else {
                assert_eq!(f, [0.0; 2], "flow on dry ground at {x},{z}");
                continue;
            };
            let pond = d.pond(x, z, f32::MIN).is_some_and(|(_, l)| l == level);
            if d.lake_level_at(x, z) == Some(level) || pond {
                continue;
            }
            let v = speed(f);
            assert!(
                (STREAM_M_S.0 - 1e-3..=STREAM_M_S.1 + 1e-3).contains(&v),
                "{v} m/s at {x},{z}"
            );
            let (ux, uz) = (f[0] / v, f[1] / v);
            let ahead = d.wet(x + ux, z + uz, f32::MIN).level_m;
            let behind = d.wet(x - ux, z - uz, f32::MIN).level_m;
            if let (Some(a), Some(b)) = (ahead, behind) {
                streams += 1;
                down += usize::from(a <= level + 1e-4 && b >= level - 1e-4);
            }
        }
        assert!(
            streams > 1000 && down * 100 >= streams * 98,
            "{down} of {streams} stream points flow downhill"
        );
    }

    /// Whether every lake corner of the lattice cell holding `(x_m, z_m)` drains by a spill its
    /// window sees.
    fn seen_whole(d: &Drainage, x_m: f32, z_m: f32) -> bool {
        let (i0, j0) = ((x_m / NODE_M).floor() as i32, (z_m / NODE_M).floor() as i32);
        let key = [i0.div_euclid(REGION_NODES), j0.div_euclid(REGION_NODES)];
        let region = d.region(key);
        [(0, 0), (1, 0), (0, 1), (1, 1)]
            .map(|(di, dj)| region.node(i0 + di, j0 + dj, key))
            .iter()
            .all(|n| !n.is_lake() || n.is_spilled())
    }

    #[test]
    fn a_lake_drifts_slowly_toward_its_outlet() {
        let d = drainage();
        let outlets: Vec<[f32; 2]> = (-2..=2)
            .flat_map(|j| (-2..=2).map(move |i| [i, j]))
            .flat_map(|key| region_nodes(&d, key))
            .filter(|&(i, j, n)| {
                n.is_lake() && n.down != NO_PARENT && {
                    let (di, dj) = STEPS[n.down as usize];
                    let key = [
                        (i + di).div_euclid(REGION_NODES),
                        (j + dj).div_euclid(REGION_NODES),
                    ];
                    !d.region(key).node(i + di, j + dj, key).is_lake()
                }
            })
            .map(|(i, j, _)| [i as f32 * NODE_M, j as f32 * NODE_M])
            .collect();
        let (mut lake, mut whole, mut reached) = (0, 0, 0);
        for (x0, z0) in jittered(40, 50.0) {
            let Some(level) = d.lake_level_at(x0, z0).filter(|&l| l > SEA_M) else {
                continue;
            };
            if d.wet(x0, z0, f32::MIN).level_m != Some(level) {
                continue;
            }
            lake += 1;
            let v = speed(d.flow(x0, z0));
            assert!(v > 0.0 && v <= LAKE_DRIFT_M_S.1 + 1e-3, "{v} m/s on a lake");
            let (mut x, mut z) = (x0, z0);
            for _ in 0..4000 {
                if d.lake_level_at(x, z).is_none() {
                    break;
                }
                let f = d.flow(x, z);
                let v = speed(f).max(1e-6);
                (x, z) = (x + 4.0 * f[0] / v, z + 4.0 * f[1] / v);
            }
            // A drift that turns back, as across a border two windows drain apart, never leaves.
            assert!(
                d.lake_level_at(x, z).is_none(),
                "the drift from {x0},{z0} is held at {x},{z}"
            );
            // A lake held at the coarse solve's level drifts toward that solve's outlet instead.
            if seen_whole(&d, x0, z0) {
                whole += 1;
                let near = |o: &[f32; 2]| libm::hypotf(o[0] - x, o[1] - z) < 2.0 * NODE_M;
                reached += usize::from(outlets.iter().any(near));
            }
        }
        assert!(
            lake > 20 && whole * 2 > lake && reached * 100 >= whole * 95,
            "{reached} of {whole} lake points seen whole, of {lake}, drift out by an outlet"
        );
    }
}
