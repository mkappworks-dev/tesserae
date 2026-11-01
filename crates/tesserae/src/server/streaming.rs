//! Chunk streaming: which Chunks exist, and which the Client has a mesh for.
//!
//! Streams around the Player's Eye, which the rest of this module calls the
//! anchor: the Server's own idea of where the Player is, not a pose the Client
//! owns.
//!
//! Four nested extents, in Cells, each with a job:
//!
//! | Extent  | Meaning                                                  |
//! | ------- | -------------------------------------------------------- |
//! | `mesh`  | gets a mesh, from `view_distance_m`                      |
//! | `keep`  | keeps the mesh it has — hysteresis at the visible edge    |
//! | `load`  | is generated, so `keep`'s outermost Chunk has neighbours  |
//! | `evict` | is still held — hysteresis on generation                  |
//!
//! The ordering is load-bearing rather than defensive slack. Face visibility
//! reads one Voxel across each face, so a Chunk meshed before its six
//! neighbours exist emits a skin of faces buried in the neighbour's rock —
//! hence `load` is `keep` grown by one Cell. And because `evict` is in turn
//! wider than `load`, an evicted Chunk is never adjacent to a Chunk the Client
//! holds a mesh for, so eviction can never leave a stale face behind and needs
//! no re-mesh of its own.
//!
//! Generation, light solving and meshing run as jobs on a pool (see
//! `super::jobs`); the tick dispatches them from snapshots and lands the
//! results. The same ordering is what makes that safe. A mesh job reads six
//! neighbours, and for one to be evicted before the mesh lands the anchor must
//! have moved far enough that the Chunk being meshed is outside `mesh`; landing
//! checks that extent, so **a mesh lands only if every neighbour it read is still
//! held**. A light solve is validated differently - against Edits, and against
//! the Chunk having been replaced - and not against the extents.

use std::sync::Arc;

use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;

use crate::session::SessionApp;
#[cfg(any(feature = "dev", feature = "profiling"))]
use tesserae_core::Chunk;
use tesserae_core::mesh::{NEG_X, NEG_Y, NEG_Z, POS_X, POS_Y, POS_Z};
#[cfg(any(feature = "dev", feature = "profiling"))]
use tesserae_core::visibility::Connectivity;
use tesserae_core::{CHUNK_SIZE, ChunkMesh, Tables};

use super::jobs::{Border, Jobs, Solved, build_mesh, generate, solve};
use super::lod;
use super::lod_rings::LevelRings;
use super::player::Player;
use super::priority::{Aim, Order, Ring};
use super::world_state::{ChunkStore, Snapshot};
use super::{ContentTables, WorldGenerator};
use crate::world::{ChunkCoord, WorldSettings};

/// A Chunk's finished geometry, for the Client to upload.
///
/// Carries a `tesserae-core` `ChunkMesh` with no Bevy in it. Sent even when
/// empty, so the Client has one rule: empty means despawn.
#[derive(Message)]
pub struct ChunkMeshReady {
    pub coord: ChunkCoord,
    pub mesh: ChunkMesh,
    /// Which of the Chunk's faces see each other through it, for the Client's
    /// visibility walk: the Client never reads the Voxels to work it out.
    #[cfg(any(feature = "dev", feature = "profiling"))]
    pub connectivity: Connectivity,
}

/// The Client should stop drawing this Chunk.
#[derive(Message)]
pub struct ChunkUnloaded {
    pub coord: ChunkCoord,
}

/// `mesh` is in that level's Voxel-index space: the Client scales it by
/// `voxel_size * 2^level` and places the Chunk at `coord * cell_size * 2^level`.
/// Empty means despawn. Level 0 never appears here.
#[derive(Message)]
pub struct LodMeshReady {
    pub level: u32,
    pub coord: ChunkCoord,
    pub mesh: ChunkMesh,
}

#[derive(Message)]
pub struct LodUnloaded {
    pub level: u32,
    pub coord: ChunkCoord,
}

/// How far the Server streams. Not World save data — see `crate::world`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct StreamingSettings {
    /// Horizontal radius, in metres, that the Client gets geometry for.
    pub view_distance_m: f32,
    /// Half-extent above and below the anchor, in metres.
    pub vertical_distance_m: f32,
    /// How far past an extent a Chunk travels before the Server acts on it, so
    /// an anchor sitting on a Cell boundary does not load and evict the same
    /// Chunk every tick.
    pub hysteresis_m: f32,
    /// At or below `view_distance_m` there are no coarse levels.
    pub lod_distance_m: f32,
}

impl Default for StreamingSettings {
    fn default() -> Self {
        StreamingSettings {
            view_distance_m: 192.0,
            vertical_distance_m: 96.0,
            hysteresis_m: 32.0,
            lod_distance_m: 4096.0,
        }
    }
}

/// A count for each stage of the pipeline.
#[derive(Clone, Copy, Debug)]
pub struct StageCounts {
    pub generate: usize,
    pub relight: usize,
    pub mesh: usize,
}

/// How much streaming work is allowed at once, and how much lands per tick.
///
/// Not a count of work done per tick, which is what this used to be and what the
/// tick's cost was once proportional to. The work runs on a pool now, so the
/// tick pays for dispatching and landing, which cost the same however expensive
/// the Chunk is. Two numbers per stage, bounding two different things:
///
/// - **`in_flight`** bounds the snapshots held and how stale a queued job's
///   priority can get: the pool is FIFO, so an anchor that moves cannot reorder
///   what is already queued. Dispatch only happens on the tick, so a cap below
///   what the workers can do in one tick idles them and slows the fill rather
///   than the frame.
/// - **`landed_per_tick`** bounds what the tick itself pays: a hash write and a
///   message per result, and for a mesh the Client's upload (11 us each).
///
/// A bad number here costs fill latency, not frame time. That is the trade
/// against the old count, which was deterministic and had to be sized for the
/// worst frame: this is not, and when work lands varies with the machine.
///
/// Measured 2026-10-12 on an Apple M5 (3 async compute threads), debug at
/// `opt-level = 1`, five runs each, with the worst main-thread time per tick and
/// per frame taken from `MainThreadCost` rather than frame time, because vsync
/// turns the latter into the refresh interval:
///
/// | In flight (gen / relight / mesh) | shown | filled | worst tick |
/// | -------------------------------- | ----- | ------ | ---------- |
/// | 16 / 16 / 16                     | 5.9 s | 6.1 s  | 1.2 ms     |
/// | 32 / 32 / 32                     | 3.1 s | 3.2 s  | 0.5 ms     |
/// | 64 / 128 / 128 (these defaults)  | 1.5 s | 1.6 s  | 0.7 ms     |
///
/// against 1.1 s, 7.2 s and 22 ms before. Doubling the async compute threads
/// made things worse, not better: at 6 the worst tick was 12.7 ms, because the
/// workers took cores the render and ECS threads needed; 4 was within noise of 3.
///
/// The cap times the cost of a Chunk, over the workers, is how far behind the
/// anchor a priority can lag. With caves a surface Chunk a tunnel crosses costs
/// 252 us to generate, 140 us to light and 395 us to mesh: 64 x 252 us + 128 x
/// 140 us + 128 x 395 us = 85 ms of work if every job were such a Chunk, 28 ms
/// across three workers, about two ticks (68 ms and 23 ms before caves). Most
/// jobs are nanosecond early-outs or Chunks no tunnel crosses. Caves moved
/// nothing measured: ten runs each, interleaved, with `TESSERAE_CAVES=0` against
/// the default, give a worst tick of 0.6 ms, a fill of 1.58 s and 8 us a mesh
/// upload either way. A cost ten times higher is when these want to come down or
/// become a worker-time window.
///
/// Slope smoothing put stairs and slabs on the surface Chunks, which is the
/// first time terrain pays for the shaped paths: about +65 us to generate
/// (112 -> 178), +10-20% to mesh and about +80-100% to light (the shaped flood
/// is the slower one), per surface Chunk, and 11 -> 16 us a mesh upload. Added
/// to the worst case above that is roughly 116 ms of work or 39 ms over three
/// workers, 1.4x the window and nowhere near 10x. Nine-run A/B with
/// `TESSERAE_SMOOTHING=0` against the default, on a machine at load 20 (so the
/// absolute figures are worse than the table), moved the worst tick from 1.6 to
/// 1.8 ms and nothing else beyond noise. Defaults unchanged.
///
/// `wait_for_settled_light` holds a mesh back until its neighbourhood has no
/// light solve pending. Off, a Chunk shows 0.2-0.4 s sooner but is built 2.2
/// times as often (2427 meshes against 1111), each one a delivery and an upload.
#[derive(Resource, Clone, Copy, Debug)]
pub struct StreamingBudget {
    /// Jobs allowed in flight, per stage.
    pub in_flight: StageCounts,
    /// Results applied to World state per tick, per stage.
    pub landed_per_tick: StageCounts,
    /// Whether a mesh waits for its Chunk and the six around it to have no light
    /// solve pending or running.
    pub wait_for_settled_light: bool,
}

impl Default for StreamingBudget {
    fn default() -> Self {
        StreamingBudget {
            in_flight: StageCounts {
                generate: 64,
                relight: 128,
                mesh: 128,
            },
            landed_per_tick: StageCounts {
                generate: 64,
                relight: 256,
                mesh: 32,
            },
            wait_for_settled_light: true,
        }
    }
}

#[cfg(any(feature = "dev", feature = "profiling"))]
impl StreamingBudget {
    /// Overrides from the environment, so a sweep needs no rebuild:
    /// `TESSERAE_BUDGET` is six comma-separated counts, the in-flight caps then
    /// the landing caps, each generate, relight, mesh; `TESSERAE_SETTLE_GATE`
    /// is 0 or 1.
    fn with_env_overrides(mut self) -> Self {
        if let Ok(v) = std::env::var("TESSERAE_BUDGET") {
            let n: Vec<usize> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
            if let [a, b, c, d, e, f] = n[..] {
                self.in_flight = StageCounts {
                    generate: a,
                    relight: b,
                    mesh: c,
                };
                self.landed_per_tick = StageCounts {
                    generate: d,
                    relight: e,
                    mesh: f,
                };
            } else {
                warn!("TESSERAE_BUDGET needs six counts, got {v:?}");
            }
        }
        if let Ok(v) = std::env::var("TESSERAE_SETTLE_GATE") {
            self.wait_for_settled_light = v != "0";
        }
        self
    }
}

/// Under `dev`, `TESSERAE_LOD=0` turns coarse streaming off, to measure its cost.
pub(super) fn streaming_settings(view: Option<tesserae_mods::ViewParams>) -> StreamingSettings {
    let mut settings = StreamingSettings::default();
    if let Some(view) = view {
        settings.view_distance_m = view.view_distance_m;
        settings.lod_distance_m = view.lod_distance_m;
    }
    #[cfg(any(feature = "dev", feature = "profiling"))]
    if std::env::var("TESSERAE_LOD").is_ok_and(|v| v == "0") {
        return StreamingSettings {
            lod_distance_m: settings.view_distance_m,
            ..settings
        };
    }
    settings
}

fn streaming_budget() -> StreamingBudget {
    let budget = StreamingBudget::default();
    #[cfg(any(feature = "dev", feature = "profiling"))]
    let budget = budget.with_env_overrides();
    budget
}

/// A cylinder of Cells around the anchor: a radius in x and z, a half-extent in
/// y. Round horizontally because a box gives a visibly square horizon, and
/// terrain is wide and shallow so the two axes do not want the same number.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Extent {
    pub horizontal: i32,
    pub vertical: i32,
}

impl Extent {
    pub fn contains(self, delta: IVec3) -> bool {
        delta.y.abs() <= self.vertical
            && delta.x * delta.x + delta.z * delta.z <= self.horizontal * self.horizontal
    }

    pub fn grown(self, cells: i32) -> Self {
        Extent {
            horizontal: self.horizontal + cells,
            vertical: self.vertical + cells,
        }
    }

    /// How many Cells are inside, without building them.
    #[cfg(any(test, feature = "dev", feature = "profiling"))]
    pub fn count(self) -> usize {
        let h = self.horizontal;
        let columns: i32 = (-h..=h).map(|x| 2 * (h * h - x * x).isqrt() + 1).sum();
        columns as usize * (2 * self.vertical + 1) as usize
    }

    /// Every offset inside this extent, nearest first.
    pub fn offsets(self) -> Vec<IVec3> {
        let h = self.horizontal;
        let v = self.vertical;
        let mut offsets: Vec<IVec3> = (-v..=v)
            .flat_map(|y| (-h..=h).flat_map(move |z| (-h..=h).map(move |x| IVec3::new(x, y, z))))
            .filter(|&d| self.contains(d))
            .collect();
        offsets.sort_unstable_by_key(|d| d.length_squared());
        offsets
    }
}

/// The four extents, derived from [`StreamingSettings`] and the Cell size.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rings {
    pub mesh: Extent,
    pub keep: Extent,
    pub load: Extent,
    pub evict: Extent,
}

impl Rings {
    pub fn new(settings: StreamingSettings, cell_size_m: f32) -> Self {
        let level_zero = LevelRings::new(settings, cell_size_m, 0);
        Rings {
            mesh: level_zero.mesh.outer,
            keep: level_zero.keep.outer,
            load: level_zero.load,
            evict: level_zero.evict,
        }
    }
}

/// The load volume's offsets in priority order (`server::priority`), rebuilt
/// only when the Eye's anchor, look or speed moves past a threshold, so a tick
/// does not sort a few thousand coordinates.
#[derive(Resource, Default)]
struct LoadOrder {
    built_for: Option<Extent>,
    /// The load extent, in no particular order.
    extent: Vec<IVec3>,
    priority: Order,
    offsets: Vec<IVec3>,
    /// The same offsets highest layer first, each layer in priority order. Sky
    /// light flows downwards, so solving a Chunk before the one above it means
    /// solving it again when the one above arrives.
    descending: Vec<IVec3>,
}

/// Which loaded Chunks need light solved, which need a mesh, and which the
/// Client holds one for.
#[derive(Resource, Default)]
pub(in crate::server) struct MeshState {
    unlit: HashSet<ChunkCoord>,
    dirty: HashSet<ChunkCoord>,
    delivered: HashSet<ChunkCoord>,
    /// Chunks with a light solve running. Removed from `unlit` when the job
    /// starts, so anything that marks one stale again while it runs is still
    /// recorded when it finishes.
    relighting: HashSet<ChunkCoord>,
    /// Chunks whose running solve read something an Edit has since changed. The
    /// incremental path is not monotone, so a bulk result computed from the old
    /// state must not land over it.
    poisoned: HashSet<ChunkCoord>,
    /// Solves of each Chunk dropped in a row for reading what changed.
    spoilt: HashMap<ChunkCoord, u8>,
    /// Held Chunks whose Voxels an Edit changed: what a level-1 cover derives
    /// from, unlike the light an Edit touches.
    edited_chunks: HashSet<ChunkCoord>,
    /// Drained by `stream_lod`, which alone can see whether the cover exists.
    cover_changed: HashSet<ChunkCoord>,
    /// Each held Chunk's connectivity, and the Voxels it was worked out from:
    /// the Weak makes an Edit's `Arc::make_mut` move the Chunk, so a stale
    /// entry never matches what the store holds.
    #[cfg(any(feature = "dev", feature = "profiling"))]
    connectivity: HashMap<ChunkCoord, (std::sync::Weak<Chunk>, Connectivity)>,
    #[cfg(any(feature = "dev", feature = "profiling"))]
    pub(in crate::server) reach: MeshReach,
}

/// Level-0 mesh jobs and their worker CPU, split by whether a walk from the
/// Eye through open space reaches the Chunk: what moving the unreachable ones
/// to the back of the order would defer. `TESSERAE_PROFILE_VISIBILITY` only,
/// as the walk runs on the tick.
#[cfg(any(feature = "dev", feature = "profiling"))]
#[derive(Clone)]
pub(in crate::server) struct MeshReach {
    pub on: bool,
    /// Reached jobs and nanoseconds, then unreached ones.
    split: Arc<[std::sync::atomic::AtomicU64; 4]>,
}

#[cfg(any(feature = "dev", feature = "profiling"))]
impl Default for MeshReach {
    fn default() -> Self {
        MeshReach {
            on: std::env::var_os("TESSERAE_PROFILE_VISIBILITY").is_some(),
            split: Arc::default(),
        }
    }
}

#[cfg(any(feature = "dev", feature = "profiling"))]
impl MeshReach {
    /// `[(jobs, cpu ms) reached, (jobs, cpu ms) not]` since the World opened.
    pub fn totals(&self) -> [(u64, f64); 2] {
        use std::sync::atomic::Ordering::Relaxed;
        let at = |i: usize| self.split[i].load(Relaxed);
        [(at(0), at(1) as f64 / 1e6), (at(2), at(3) as f64 / 1e6)]
    }

    /// `work`, charging its thread CPU to the reached or the unreached side.
    fn charge<T, W: FnOnce() -> T + Send + 'static>(
        &self,
        reached: bool,
        work: W,
    ) -> impl FnOnce() -> T + Send + 'static + use<T, W> {
        let split = Arc::clone(&self.split);
        let side = if reached { 0 } else { 2 };
        move || {
            use std::sync::atomic::Ordering::Relaxed;
            let before = crate::cost::thread_cpu();
            let out = work();
            split[side].fetch_add(1, Relaxed);
            if let Some((a, b)) = before.zip(crate::cost::thread_cpu()) {
                split[side + 1].fetch_add(((b - a) * 1e9) as u64, Relaxed);
            }
            out
        }
    }
}

/// The Chunks of `load` a walk from `center` through open space enters, from
/// the connectivity each held Chunk was generated or meshed with. One without
/// any counts as open.
#[cfg(any(feature = "dev", feature = "profiling"))]
fn reachable(
    store: &ChunkStore,
    meshes: &MeshState,
    center: ChunkCoord,
    load: Extent,
) -> tesserae_core::visibility::Visited {
    let c = [center.x, center.y, center.z];
    let bounds = tesserae_core::visibility::Bounds::around(
        c,
        [load.horizontal, load.vertical, load.horizontal],
    );
    tesserae_core::visibility::reach(c, bounds, |[x, y, z]| {
        let coord = ChunkCoord::new(x, y, z);
        store
            .shared(coord)
            .and_then(|held| meshes.connectivity_of(coord, &held.chunk))
            .unwrap_or(Connectivity::ALL)
    })
}

impl MeshState {
    /// Whether this Chunk's light is still waiting on a solve. A Server system
    /// that wants to edit a Voxel waits for this: an edit re-floods light
    /// incrementally, and `relight`'s from-scratch solve landing afterwards
    /// would discard the work even though it would reach the same answer.
    pub(in crate::server) fn is_unlit(&self, coord: ChunkCoord) -> bool {
        self.unlit.contains(&coord) || self.relighting.contains(&coord)
    }

    /// Every Chunk [`MeshState::is_unlit`] is true of.
    pub(in crate::server) fn unlit_chunks(&self) -> impl Iterator<Item = ChunkCoord> + '_ {
        self.unlit.union(&self.relighting).copied()
    }

    /// A Voxel in this Chunk changed and its light has already been re-flooded,
    /// so the mesh that bakes both is stale.
    ///
    /// Deliberately not adding to `unlit`. The edit's own incremental pass is
    /// the sound one in both directions; `relight`'s bulk solve converges
    /// upwards only, so routing an edit through it is the mistake
    /// `tesserae_core::relight`'s doc comment describes.
    ///
    /// A running solve of this Chunk is spoilt too. A solve reads its own
    /// Chunk and only the plane of each neighbour that faces it, so a change
    /// elsewhere in a neighbour leaves it standing: the caller marks the
    /// neighbour whose plane it changed as edited itself. Spoiling all seven
    /// would let water that never stops changing next door keep a solve, and
    /// every mesh that waits on it, from ever landing.
    pub(in crate::server) fn edited(&mut self, coord: ChunkCoord) {
        self.dirty.insert(coord);
        if self.relighting.contains(&coord) {
            self.poisoned.insert(coord);
        }
    }

    /// Separate from `edited`, which also fires for Chunks only light or a face
    /// reached; their cover reads neither.
    pub(in crate::server) fn voxels_changed(&mut self, coord: ChunkCoord) {
        self.edited_chunks.insert(coord);
        self.cover_changed.insert(coord);
    }

    #[cfg(any(feature = "dev", feature = "profiling"))]
    fn note_connectivity(&mut self, coord: ChunkCoord, chunk: &Arc<Chunk>, c: Connectivity) {
        self.connectivity.insert(coord, (Arc::downgrade(chunk), c));
    }

    /// The connectivity of `chunk`, if it was worked out from those Voxels.
    #[cfg(any(feature = "dev", feature = "profiling"))]
    fn connectivity_of(&self, coord: ChunkCoord, chunk: &Arc<Chunk>) -> Option<Connectivity> {
        self.connectivity
            .get(&coord)
            .filter(|(from, _)| std::ptr::eq(from.as_ptr(), Arc::as_ptr(chunk)))
            .map(|&(_, c)| c)
    }

    pub(in crate::server) fn take_cover_changes(&mut self) -> Vec<ChunkCoord> {
        self.cover_changed.drain().collect()
    }

    /// Whether a Chunk inside `load` is waiting on a light solve. Outside it
    /// `relight` never looks, so a Chunk the anchor has drifted away from can
    /// stay in `unlit` for good without anything being left to do.
    #[cfg(any(test, feature = "dev", feature = "profiling"))]
    fn unlit_within(&self, center: ChunkCoord, load: Extent) -> bool {
        self.unlit.iter().any(|&c| load.contains(delta(center, c)))
    }

    /// Whether the Client was sent this Chunk's mesh and has not been told to
    /// drop it.
    pub(in crate::server) fn is_delivered(&self, coord: ChunkCoord) -> bool {
        self.delivered.contains(&coord)
    }

    pub(in crate::server) fn is_edited(&self, coord: ChunkCoord) -> bool {
        self.edited_chunks.contains(&coord)
    }

    #[cfg(test)]
    pub(in crate::server) fn is_dirty(&self, coord: ChunkCoord) -> bool {
        self.dirty.contains(&coord)
    }

    #[cfg(test)]
    pub(in crate::server) fn mark_unlit(&mut self, coord: ChunkCoord) {
        self.unlit.insert(coord);
    }

    #[cfg(test)]
    pub(in crate::server) fn mark_lit(&mut self, coord: ChunkCoord) {
        self.unlit.remove(&coord);
    }
}

pub(super) fn plugin(app: &mut App) {
    app.init_session_resource::<StreamingSettings>()
        .init_session_resource_with(streaming_budget)
        .init_session_resource::<LoadOrder>()
        .init_session_resource::<MeshState>()
        .init_session_resource::<Jobs>()
        .add_session_message::<ChunkMeshReady>()
        .add_session_message::<ChunkUnloaded>()
        .add_session_message::<LodMeshReady>()
        .add_session_message::<LodUnloaded>()
        .add_plugins(lod::plugin)
        .add_session_systems(
            FixedUpdate,
            (stream, lod::stream_lod).chain().in_set(Streaming),
        );
    #[cfg(any(feature = "dev", feature = "profiling"))]
    app.add_session_systems(FixedFirst, crate::cost::tick_started)
        .add_session_systems(FixedLast, crate::cost::tick_ended);
    #[cfg(any(feature = "dev", feature = "profiling"))]
    app.add_plugins(stats::plugin);
}

/// The six face neighbours, in the mesher's face order.
pub(super) fn face_neighbors(coord: ChunkCoord) -> [ChunkCoord; 6] {
    let mut n = [coord; 6];
    n[NEG_X] = ChunkCoord::new(coord.x - 1, coord.y, coord.z);
    n[POS_X] = ChunkCoord::new(coord.x + 1, coord.y, coord.z);
    n[NEG_Y] = ChunkCoord::new(coord.x, coord.y - 1, coord.z);
    n[POS_Y] = ChunkCoord::new(coord.x, coord.y + 1, coord.z);
    n[NEG_Z] = ChunkCoord::new(coord.x, coord.y, coord.z - 1);
    n[POS_Z] = ChunkCoord::new(coord.x, coord.y, coord.z + 1);
    n
}

pub(super) fn offset(center: ChunkCoord, delta: IVec3) -> ChunkCoord {
    ChunkCoord::new(center.x + delta.x, center.y + delta.y, center.z + delta.z)
}

pub(super) fn delta(from: ChunkCoord, to: ChunkCoord) -> IVec3 {
    IVec3::new(to.x - from.x, to.y - from.y, to.z - from.z)
}

#[derive(SystemSet, Hash, PartialEq, Eq, Clone, Debug)]
pub(super) struct Streaming;

#[expect(
    clippy::too_many_arguments,
    reason = "one tick, and splitting it would recompute the anchor and the \
              rings in each half or need a resource to carry them"
)]
fn stream(
    mut store: ResMut<ChunkStore>,
    mut order: ResMut<LoadOrder>,
    mut meshes: ResMut<MeshState>,
    mut jobs: ResMut<Jobs>,
    mut ready: MessageWriter<ChunkMeshReady>,
    mut unloaded: MessageWriter<ChunkUnloaded>,
    generator: Res<WorldGenerator>,
    tables: Res<ContentTables>,
    world: Res<WorldSettings>,
    settings: Res<StreamingSettings>,
    budget: Res<StreamingBudget>,
    player: Query<&Player>,
    time: Option<Res<Time>>,
) {
    // The Server spawns exactly one. Streaming around an arbitrary one of
    // several would be worse than streaming around none.
    let Ok(player) = player.single() else {
        return;
    };

    let rings = Rings::new(*settings, world.cell_size());
    let center = world.chunk_at(player.eye());
    if order.built_for != Some(rings.load) {
        order.extent = rings.load.offsets();
        order.built_for = Some(rings.load);
    }
    let aim = Aim::of(player);
    let order = &mut *order;
    let ring = Ring {
        level: 0,
        anchor: center,
        offsets: &order.extent,
        cell_m: world.cell_size(),
        cap_m: settings.view_distance_m,
    };
    let now_s = time.map_or(0.0, |t| t.elapsed_secs_f64());
    {
        #[cfg(any(feature = "dev", feature = "profiling"))]
        let _rescoring = crate::cost::Rescoring::start();
        if order.priority.update(&ring, &aim, now_s) {
            order.offsets = order.priority.offsets().collect();
            order.descending = order.priority.layers_top_down();
        }
    }
    let cell_m = world.cell_size();
    let priority = |c: ChunkCoord| {
        let min = IVec3::new(c.x, c.y, c.z).as_vec3() * cell_m;
        aim.score_m(min, cell_m, settings.view_distance_m)
    };

    jobs.reset_counts();
    jobs.poll();
    retire(
        &mut store,
        &mut meshes,
        &mut jobs,
        &mut unloaded,
        center,
        rings.keep,
        rings.evict,
    );
    land_generated(
        &mut store,
        &mut meshes,
        &mut jobs,
        center,
        rings.evict,
        budget.landed_per_tick.generate,
        &priority,
    );
    land_relight(
        &mut store,
        &mut meshes,
        &mut jobs,
        budget.landed_per_tick.relight,
        &priority,
    );
    land_meshes(
        &mut meshes,
        &mut jobs,
        &mut ready,
        center,
        rings.mesh,
        budget.landed_per_tick.mesh,
        &priority,
    );
    dispatch_generation(
        &store,
        &mut jobs,
        &generator,
        order,
        center,
        budget.in_flight.generate,
        &tables.0,
    );
    if let Some((coord, snapshot, border)) = dispatch_relight(
        &store,
        &mut meshes,
        &mut jobs,
        &generator,
        order,
        &world,
        center,
        budget.in_flight.relight,
        &tables.0,
    ) {
        let solved = solve(snapshot, &border, &tables.0);
        apply_solve(&mut store, &mut meshes, coord, solved);
    }
    dispatch_meshes(
        &store,
        &mut meshes,
        &mut jobs,
        order,
        center,
        (rings.mesh, rings.load),
        budget.in_flight.mesh,
        budget.wait_for_settled_light,
        &tables.0,
    );
}

/// Solves of one Chunk spoilt in a row before the next is made on the tick,
/// where nothing can change under it: a writer changing the plane it reads on
/// every tick would otherwise keep its light, and the meshes waiting on that
/// light, from ever landing. One such solve a tick, at a bulk solve's cost.
const SPOILT_BEFORE_ON_TICK: u8 = 4;

/// Drops meshes that left `keep` and Chunks that left `evict`.
fn retire(
    store: &mut ChunkStore,
    meshes: &mut MeshState,
    jobs: &mut Jobs,
    unloaded: &mut MessageWriter<ChunkUnloaded>,
    center: ChunkCoord,
    keep: Extent,
    evict: Extent,
) {
    let gone: Vec<ChunkCoord> = meshes
        .delivered
        .iter()
        .copied()
        .filter(|&c| !keep.contains(delta(center, c)))
        .collect();
    for coord in gone {
        meshes.delivered.remove(&coord);
        unloaded.write(ChunkUnloaded { coord });
        // The Chunk itself stays loaded, so marking it dirty again is the only
        // thing that will redraw it if the anchor comes back: `dirty` is
        // cleared on delivery and nothing sets it for a Chunk that is already
        // generated.
        meshes.dirty.insert(coord);
    }

    jobs.generate.retain(|c| evict.contains(delta(center, c)));
    jobs.relight.retain(|c| evict.contains(delta(center, c)));
    jobs.mesh.retain(|c| keep.contains(delta(center, c)));
    meshes
        .relighting
        .retain(|&c| evict.contains(delta(center, c)));
    meshes
        .poisoned
        .retain(|&c| evict.contains(delta(center, c)));
    meshes
        .spoilt
        .retain(|&c, _| evict.contains(delta(center, c)));

    store
        .pending_evictions
        .retain(|&c| !evict.contains(delta(center, c)));
    store
        .approved_evictions
        .retain(|&c, _| !evict.contains(delta(center, c)));
    let stale: Vec<ChunkCoord> = store
        .coords()
        .filter(|&c| !evict.contains(delta(center, c)))
        .collect();
    for coord in stale {
        if !store.remove(coord) {
            continue;
        }
        meshes.dirty.remove(&coord);
        meshes.unlit.remove(&coord);
        #[cfg(any(feature = "dev", feature = "profiling"))]
        meshes.connectivity.remove(&coord);
        // An unedited Chunk is what the generator makes again; only an edited one
        // leaves a ghost in its cover.
        if meshes.edited_chunks.remove(&coord) {
            meshes.cover_changed.insert(coord);
        }
    }
}

/// Applies up to `budget` finished generation jobs, nearest first.
///
/// A result lands only if the Chunk is still inside `evict` of the current anchor
/// and nothing else has put one there; otherwise the anchor has moved on and the
/// result is dropped.
fn land_generated(
    store: &mut ChunkStore,
    meshes: &mut MeshState,
    jobs: &mut Jobs,
    center: ChunkCoord,
    evict: Extent,
    budget: usize,
    priority: &dyn Fn(ChunkCoord) -> f32,
) {
    let mut finished: Vec<ChunkCoord> = jobs.generate.finished().collect();
    finished.sort_unstable_by(|&a, &b| priority(a).total_cmp(&priority(b)));
    for coord in finished.into_iter().take(budget) {
        let Some(generated) = jobs.generate.take(coord) else {
            continue;
        };
        if store.contains(coord) || !evict.contains(delta(center, coord)) {
            continue;
        }
        store.insert(coord, generated.chunk);
        // A Chunk restored from the archive is not what the job generated.
        #[cfg(any(feature = "dev", feature = "profiling"))]
        if let Some(held) = store
            .shared(coord)
            .filter(|_| !store.revisions.contains_key(&coord))
        {
            meshes.note_connectivity(coord, &held.chunk, generated.connectivity);
        }
        if store.revisions.contains_key(&coord) {
            meshes.voxels_changed(coord);
        }
        // Only this Chunk's *geometry* is affected, and only its own light is
        // queued. Re-dirtying the six neighbours' meshes here looks obviously
        // necessary and is not: `publish` will not mesh a Chunk until all six
        // neighbours are present, so a neighbour arriving cannot invalidate a
        // mesh the Client already holds by changing a face.
        //
        // It can invalidate one by changing the *light*, and that is what
        // `relight` handles rather than this function. Light crosses a Chunk
        // boundary and keeps going, so a Chunk two Cells away changes the light
        // in between and so changes ours — further than any re-dirtying here
        // could reach. `relight` re-publishes a Chunk whose light moved, and
        // marks its neighbours for a re-solve so the change keeps travelling as
        // far as it really goes.
        meshes.unlit.insert(coord);
        meshes.dirty.insert(coord);
    }
}

/// Starts generation jobs for the missing Chunks of the load extent in priority
/// order, until `in_flight` are outstanding.
fn dispatch_generation(
    store: &ChunkStore,
    jobs: &mut Jobs,
    generator: &WorldGenerator,
    order: &LoadOrder,
    center: ChunkCoord,
    in_flight: usize,
    tables: &Arc<Tables>,
) {
    let mut free = in_flight.saturating_sub(jobs.generate.in_flight());
    for &d in &order.offsets {
        if free == 0 {
            break;
        }
        let coord = offset(center, d);
        if store.contains(coord) || jobs.generate.contains(coord) {
            continue;
        }
        let worldgen = generator.0.clone();
        let tables = Arc::clone(tables);
        jobs.generate
            .spawn(coord, move || generate(&worldgen, coord, &tables));
        free -= 1;
    }
}

/// Applies up to `budget` finished light solves, in dispatch's order.
///
/// The bulk path: a Chunk arriving starts dark, so every change here is an
/// increase and re-solving from the current boundary converges. A Voxel
/// **edit** must not come through here — see `tesserae_core::relight`. A result
/// from before an Edit that touched what it read is dropped for that reason, and
/// the Chunk is queued again.
fn land_relight(
    store: &mut ChunkStore,
    meshes: &mut MeshState,
    jobs: &mut Jobs,
    budget: usize,
    priority: &dyn Fn(ChunkCoord) -> f32,
) {
    // Same order as dispatch: a landing that darkens nothing above it first
    // would be solved again when the one above lands.
    let mut finished: Vec<ChunkCoord> = jobs.relight.finished().collect();
    finished.sort_unstable_by(|&a, &b| {
        b.y.cmp(&a.y)
            .then_with(|| priority(a).total_cmp(&priority(b)))
    });
    for coord in finished.into_iter().take(budget) {
        let Some(solved) = jobs.relight.take(coord) else {
            continue;
        };
        meshes.relighting.remove(&coord);
        let poisoned = meshes.poisoned.remove(&coord);
        if !store.contains(coord) {
            continue;
        }
        if poisoned || !store.unchanged_since(coord, &solved.snapshot) {
            jobs.relight.note_stale();
            *meshes.spoilt.entry(coord).or_default() += 1;
            meshes.unlit.insert(coord);
            continue;
        }
        apply_solve(store, meshes, coord, solved);
    }
}

/// Lands a solve that read what the store still holds.
fn apply_solve(store: &mut ChunkStore, meshes: &mut MeshState, coord: ChunkCoord, solved: Solved) {
    meshes.spoilt.remove(&coord);
    if !solved.changed {
        return;
    }
    store.set_light(coord, solved.light);
    // This Chunk's own mesh bakes its own light, so it is stale either way.
    meshes.dirty.insert(coord);
    // Per face, not once for the Chunk. A neighbour reads only the plane
    // across the face it shares, so re-solving it because something changed
    // deep inside this Chunk is work for nothing — and during a fill it is
    // every Chunk about seven times over, which is enough to stop the
    // relight queue ever draining.
    for (face, neighbor) in face_neighbors(coord).into_iter().enumerate() {
        if solved.outward[face] && store.contains(neighbor) {
            meshes.unlit.insert(neighbor);
            // Its boundary faces are lit by the plane that just moved.
            meshes.dirty.insert(neighbor);
        }
    }
}

/// Starts light solves for Chunks whose light is out of date, highest layer
/// first and each layer in priority order, until `in_flight` are outstanding.
///
/// Top-down because sky light flows downwards: solving a Chunk before the one
/// above it means solving it again when the one above lands.
#[expect(
    clippy::too_many_arguments,
    reason = "one stage of the tick; bundling them would invent a type used once"
)]
fn dispatch_relight(
    store: &ChunkStore,
    meshes: &mut MeshState,
    jobs: &mut Jobs,
    generator: &WorldGenerator,
    order: &LoadOrder,
    world: &WorldSettings,
    center: ChunkCoord,
    in_flight: usize,
    tables: &Arc<Tables>,
) -> Option<(ChunkCoord, Snapshot, [Border; 6])> {
    let mut on_tick = None;
    let mut free = in_flight.saturating_sub(jobs.relight.in_flight());
    for &d in &order.descending {
        if free == 0 {
            break;
        }
        let coord = offset(center, d);
        // A mark made while the Chunk's job runs stays in `unlit` and is acted
        // on once that job has landed.
        if !meshes.unlit.contains(&coord) || jobs.relight.contains(coord) {
            continue;
        }
        let Some(snapshot) = store.shared(coord) else {
            meshes.unlit.remove(&coord);
            continue;
        };
        let border: [Border; 6] = face_neighbors(coord).map(|n| match store.shared(n) {
            Some(snapshot) => Border::Loaded(snapshot),
            None if n.y > coord.y && above_the_surface(generator, world, n) => Border::OpenSky,
            None => Border::Absent,
        });
        let starved = meshes
            .spoilt
            .get(&coord)
            .is_some_and(|&n| n >= SPOILT_BEFORE_ON_TICK);
        if starved {
            if on_tick.is_none() {
                meshes.unlit.remove(&coord);
                on_tick = Some((coord, snapshot, border));
            }
            continue;
        }
        meshes.unlit.remove(&coord);
        meshes.relighting.insert(coord);
        meshes.poisoned.remove(&coord);
        let tables = Arc::clone(tables);
        jobs.relight
            .spawn(coord, move || solve(snapshot, &border, &tables));
        free -= 1;
    }
    on_tick
}

/// Whether a Chunk is entirely above the band the surface can reach, so the
/// generator makes nothing but air there and everything in it sees full sky.
///
/// The comparison is in metres, as `surface_bounds_m` and the Chunk floor both
/// are. In Voxel indices it would be calibrated to one Voxel size, which is the
/// mistake `docs/adr/0003-voxel-size-is-per-world.md` exists to prevent.
fn above_the_surface(generator: &WorldGenerator, world: &WorldSettings, coord: ChunkCoord) -> bool {
    let (_, highest_m) = generator.surface_bounds_m();
    let floor_m = (coord.y * CHUNK_SIZE as i32) as f32 * world.voxel_size;
    floor_m > highest_m
}

/// Applies up to `budget` finished meshes, in priority order, and tells the
/// Client.
///
/// A mesh lands only if its Chunk is still inside the `mesh` extent. That is
/// also what keeps the module doc's eviction argument true for a job in flight:
/// a Chunk inside `mesh` has all six neighbours inside `load`, so every
/// neighbour the job read is still held.
///
/// A result is delivered even if the Chunk was dirtied again during the flight.
/// It is no staler than the mesh the Client already holds, the dirty mark is
/// still there so a fresh job follows, and dropping it instead would starve a
/// Chunk that is edited faster than a mesh takes to build.
fn land_meshes(
    meshes: &mut MeshState,
    jobs: &mut Jobs,
    ready: &mut MessageWriter<ChunkMeshReady>,
    center: ChunkCoord,
    mesh: Extent,
    budget: usize,
    priority: &dyn Fn(ChunkCoord) -> f32,
) {
    let mut finished: Vec<ChunkCoord> = jobs.mesh.finished().collect();
    finished.sort_unstable_by(|&a, &b| priority(a).total_cmp(&priority(b)));
    for coord in finished.into_iter().take(budget) {
        let Some(built) = jobs.mesh.take(coord) else {
            continue;
        };
        if !mesh.contains(delta(center, coord)) {
            jobs.mesh.note_stale();
            // Dirty again so it is redrawn if the anchor comes back.
            meshes.dirty.insert(coord);
            continue;
        }
        if meshes.dirty.contains(&coord) {
            jobs.mesh.note_stale();
        }
        #[cfg(any(feature = "dev", feature = "profiling"))]
        meshes
            .connectivity
            .insert(coord, (built.from, built.connectivity));
        ready.write(ChunkMeshReady {
            coord,
            mesh: built.mesh,
            #[cfg(any(feature = "dev", feature = "profiling"))]
            connectivity: built.connectivity,
        });
        meshes.delivered.insert(coord);
    }
}

/// Starts mesh jobs for dirty Chunks inside the mesh extent, in priority order,
/// until `in_flight` are outstanding.
///
/// Requires all six neighbours' Voxels **and** light: a boundary face is lit by
/// the Voxel across it, so a neighbour without light would bake a black seam.
/// And requires the Chunk and its neighbours to be settled, neither stale nor
/// being solved: a mesh built against light that is about to move is built twice.
/// Requiring all six neighbours is also what lets generation get away with not
/// re-dirtying them; see `land_generated`.
#[expect(
    clippy::too_many_arguments,
    reason = "one stage of the tick; bundling them would invent a type used once"
)]
fn dispatch_meshes(
    store: &ChunkStore,
    meshes: &mut MeshState,
    jobs: &mut Jobs,
    order: &LoadOrder,
    center: ChunkCoord,
    (mesh, load): (Extent, Extent),
    in_flight: usize,
    wait_for_settled_light: bool,
    tables: &Arc<Tables>,
) {
    #[cfg(not(any(feature = "dev", feature = "profiling")))]
    let _ = load;
    #[cfg(any(feature = "dev", feature = "profiling"))]
    let mut reached = None;
    let mut free = in_flight.saturating_sub(jobs.mesh.in_flight());
    for &d in &order.offsets {
        if free == 0 {
            break;
        }
        if !mesh.contains(d) {
            continue;
        }
        let coord = offset(center, d);
        if !meshes.dirty.contains(&coord) || jobs.mesh.contains(coord) {
            continue;
        }
        let Some(own) = store.shared(coord) else {
            continue;
        };
        let faces = face_neighbors(coord);
        let around: [Option<Snapshot>; 6] = faces.map(|n| store.shared(n));
        let Some(around) = around.into_iter().collect::<Option<Vec<_>>>() else {
            continue;
        };
        let settled = |c: &ChunkCoord| !meshes.is_unlit(*c);
        if wait_for_settled_light && !(settled(&coord) && faces.iter().all(settled)) {
            continue;
        }
        let around: [Snapshot; 6] = around
            .try_into()
            .unwrap_or_else(|_| unreachable!("six neighbours were collected"));
        meshes.dirty.remove(&coord);
        let tables = Arc::clone(tables);
        #[cfg(any(feature = "dev", feature = "profiling"))]
        let work = {
            let known = meshes.connectivity_of(coord, &own.chunk);
            move || build_mesh(&own, &around, known, &tables)
        };
        #[cfg(not(any(feature = "dev", feature = "profiling")))]
        let work = move || build_mesh(&own, &around, &tables);
        #[cfg(any(feature = "dev", feature = "profiling"))]
        if meshes.reach.on {
            let walk = reached.get_or_insert_with(|| reachable(store, meshes, center, load));
            let work = meshes
                .reach
                .charge(walk.contains([coord.x, coord.y, coord.z]), work);
            jobs.mesh.spawn(coord, work);
            free -= 1;
            continue;
        }
        jobs.mesh.spawn(coord, work);
        free -= 1;
    }
}

/// A summary of what the Server is holding, for the `dev` overlay.
///
/// `dev` only, because a shipped build should not put engine internals on
/// screen — and because the alternative is a resource nothing reads. It exists
/// so the Client can show streaming working while reading a published summary
/// rather than the Chunk store itself; the invariant in
/// `docs/adr/0002-client-server-from-day-one.md` is that the Client has no
/// access to World state, and a count is not a window onto it.
#[cfg(any(feature = "dev", feature = "profiling"))]
pub mod stats {
    use super::*;
    use crate::server::lod_rings::level_count;

    /// `non_uniform` onward are measured for levels above 0 only; zero at level 0.
    #[derive(Default, Clone, Copy, Debug)]
    pub struct LevelStats {
        /// Chunks held, including the margin no mesh is built for.
        pub held: usize,
        /// Chunks the Client holds a mesh for. Counts empty meshes.
        pub meshed: usize,
        /// What `held` is once the level has filled around an Eye that holds
        /// still: its load extent.
        pub steady: usize,
        /// What `meshed` is then: the Chunks of its mesh shell.
        pub shell: usize,
        pub non_uniform: usize,
        pub mesh: lod::measure::MeshSize,
        /// Mean job time to generate, and to mesh, a Chunk of this level.
        pub generate: lod::measure::JobCost,
        pub mesh_job: lod::measure::JobCost,
        #[cfg_attr(not(feature = "profiling"), allow(dead_code, reason = "profiler only"))]
        pub flow: Flow,
    }

    /// Generate, relight and mesh work of one level; relight is level 0 only.
    #[derive(Default, Clone, Copy, Debug)]
    #[cfg_attr(not(feature = "profiling"), allow(dead_code, reason = "profiler only"))]
    pub struct Flow {
        /// Due and not dispatched: a Chunk of the load extent not held, a lit
        /// Chunk whose light is out of date, a Chunk of the mesh shell whose mesh
        /// is.
        pub waiting: [usize; 3],
        pub in_flight: [usize; 3],
        /// Since the World opened: jobs run on a worker, and their worker CPU.
        pub run: [u64; 3],
        pub cpu_ms: [f64; 3],
    }

    #[derive(Resource, Default, Clone, Debug)]
    pub struct StreamingStats {
        /// Chunks the Server holds, including the margin the Client never sees.
        pub loaded: usize,
        /// Chunks the Server has sent the Client a mesh for. Not the number
        /// drawn: most of a 3D ring is sky, and an empty mesh is a despawn.
        pub meshed: usize,
        /// The Chunk the anchor is standing in.
        pub anchor: ChunkCoord,
        /// Nothing is left to do: the load volume is held, every Chunk's light
        /// is solved, and every Chunk inside the mesh extent has a current
        /// mesh. Chunks the Server holds only as margin stay dirty for ever and
        /// are not waited for.
        pub settled: bool,
        /// Every Chunk inside the mesh extent has been sent a mesh at least
        /// once. Earlier than `settled`, which also waits for the margin's
        /// light, and the one that matches what a player sees fill in.
        pub shown: bool,
        /// Jobs landed, and results that were out of date when they landed, per
        /// stage: generate, relight, mesh. A stale relight was dropped; a stale
        /// mesh was delivered anyway and followed by a fresh one.
        pub landed: [usize; 3],
        pub stale: [usize; 3],
        /// Indexed by level. Level 0 repeats `loaded` and `meshed`.
        pub levels: Vec<LevelStats>,
        /// Fluid steps' jobs run and their worker CPU since the World opened:
        /// they share the workers with streaming.
        #[cfg_attr(not(feature = "profiling"), allow(dead_code, reason = "profiler only"))]
        pub fluid_run: (u64, f64),
        /// Level-0 mesh jobs and worker CPU ms that a walk from the Eye
        /// reached, then those it did not, since the World opened. `None`
        /// unless `TESSERAE_PROFILE_VISIBILITY` is set.
        #[cfg_attr(not(feature = "profiling"), allow(dead_code, reason = "profiler only"))]
        pub mesh_reach: Option<[(u64, f64); 2]>,
    }

    /// Where [`StreamingStats`] is written, for a reader that must see this tick's.
    #[derive(SystemSet, Hash, PartialEq, Eq, Clone, Debug)]
    pub struct Published;

    pub(super) fn plugin(app: &mut App) {
        app.init_session_resource::<StreamingStats>()
            .add_session_systems(
                FixedPostUpdate,
                publish_stats
                    .run_if(crate::cost::collecting)
                    .in_set(Published),
            );
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "a Bevy system, and a SystemParam bundle would be a type used once"
    )]
    pub(super) fn publish_stats(
        mut stats: ResMut<StreamingStats>,
        store: Res<ChunkStore>,
        meshes: Res<MeshState>,
        jobs: Res<Jobs>,
        order: Res<LoadOrder>,
        settings: Res<StreamingSettings>,
        world: Res<WorldSettings>,
        lod_store: Res<lod::LodStore>,
        lod_state: Res<lod::LodState>,
        lod_jobs: Res<lod::LodJobs>,
        fluids: Option<Res<crate::server::fluids::FluidSim>>,
        player: Query<&Player>,
    ) {
        let eye = player.single().map(Player::eye).ok();
        let center = player
            .single()
            .map(|player| world.chunk_at(player.eye()))
            .ok();
        let rings = Rings::new(*settings, world.cell_size());
        let mesh = rings.mesh;
        let shown = center.is_some_and(|center| {
            mesh.offsets()
                .into_iter()
                .all(|d| meshes.delivered.contains(&offset(center, d)))
        });
        let settled = center.is_some_and(|center| {
            // An empty order is the tick before the first `stream`, which would
            // otherwise read as a held load volume.
            !order.offsets.is_empty()
                && jobs.in_flight() == 0
                && order
                    .offsets
                    .iter()
                    .all(|&d| store.contains(offset(center, d)))
                && !meshes.unlit_within(center, rings.load)
                && !meshes
                    .dirty
                    .iter()
                    .any(|&c| mesh.contains(delta(center, c)))
        });
        let (generated, relit, meshed) = (jobs.generate.run(), jobs.relight.run(), jobs.mesh.run());
        let level_zero = Flow {
            waiting: center.map_or([0; 3], |center| {
                [
                    order
                        .offsets
                        .iter()
                        .filter(|&&d| {
                            let c = offset(center, d);
                            !store.contains(c) && !jobs.generate.contains(c)
                        })
                        .count(),
                    meshes.unlit.len(),
                    meshes
                        .dirty
                        .iter()
                        .filter(|&&c| mesh.contains(delta(center, c)) && !jobs.mesh.contains(c))
                        .count(),
                ]
            }),
            in_flight: [
                jobs.generate.in_flight(),
                jobs.relight.in_flight(),
                jobs.mesh.in_flight(),
            ],
            run: [generated.0, relit.0, meshed.0],
            cpu_ms: [generated.1, relit.1, meshed.1],
        };
        let mut levels = vec![LevelStats {
            held: store.coords().count(),
            meshed: meshes.delivered.len(),
            steady: rings.load.count(),
            shell: mesh.count(),
            flow: level_zero,
            ..default()
        }];
        let (lod_generating, lod_meshing) = (lod_jobs.generate.keys(), lod_jobs.mesh.keys());
        let lod_in_flight = |keys: &[(u32, ChunkCoord)], level: u32| {
            keys.iter().filter(|(l, _)| *l == level).count()
        };
        levels.extend((1..level_count(&settings)).map(|level| {
            let rings = LevelRings::new(*settings, world.cell_size(), level);
            let anchor = eye.map(|eye| world.chunk_at_level(eye, level));
            let [generate_cpu, mesh_cpu] = lod_jobs.times.cpu_ms(level);
            let flow = Flow {
                waiting: anchor.map_or([0; 3], |anchor| {
                    [
                        rings
                            .load
                            .offsets()
                            .into_iter()
                            .filter(|&d| {
                                let c = offset(anchor, d);
                                !lod_store.contains(level, c)
                                    && !lod_jobs.generate.contains((level, c))
                            })
                            .count(),
                        0,
                        lod_state.dirty_within(level, anchor, rings.mesh),
                    ]
                }),
                in_flight: [
                    lod_in_flight(&lod_generating, level),
                    0,
                    lod_in_flight(&lod_meshing, level),
                ],
                run: [
                    lod_jobs.times.generate_cost(level).jobs,
                    0,
                    lod_jobs.times.mesh_cost(level).jobs,
                ],
                cpu_ms: [generate_cpu, 0.0, mesh_cpu],
            };
            LevelStats {
                flow,
                held: lod_store.len(level),
                meshed: lod_state.delivered_len(level),
                steady: rings.load.count(),
                shell: rings.mesh.count(),
                non_uniform: lod_store.non_uniform(level),
                mesh: lod_state.mesh_sizes(level),
                generate: lod_jobs.times.generate_cost(level),
                mesh_job: lod_jobs.times.mesh_cost(level),
            }
        }));
        *stats = StreamingStats {
            loaded: store.coords().count(),
            meshed: meshes.delivered.len(),
            anchor: center.unwrap_or_default(),
            settled,
            shown,
            landed: [
                jobs.generate.landed_total(),
                jobs.relight.landed_total(),
                jobs.mesh.landed_total(),
            ],
            stale: [
                jobs.generate.stale(),
                jobs.relight.stale(),
                jobs.mesh.stale(),
            ],
            levels,
            fluid_run: fluids.map_or((0, 0.0), |f| f.jobs_run()),
            mesh_reach: meshes.reach.on.then(|| meshes.reach.totals()),
        };
    }
}

#[cfg(test)]
pub(in crate::server) mod tests {
    use super::*;
    use crate::server::edit;
    use crate::server::lod_rings::level_count;
    use crate::server::world_state::VoxelCoord;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::tasks::{AsyncComputeTaskPool, TaskPoolBuilder};
    use tesserae_core::light::{Light, MAX_LIGHT, light_chunk};
    use tesserae_core::{Chunk, ChunkLight, LitChunk, MeshLayer, mesh_chunk};
    use tesserae_core::{Voxel, Worldgen, WorldgenParams};

    fn rings(voxel_size: f32) -> Rings {
        let world = WorldSettings {
            voxel_size,
            ..default()
        };
        Rings::new(StreamingSettings::default(), world.cell_size())
    }

    #[test]
    fn the_extents_nest_outwards() {
        for view_distance_m in [1.0, 31.0, 32.0, 33.0, 100.0, 192.0, 1000.0] {
            let settings = StreamingSettings {
                view_distance_m,
                ..default()
            };
            let r = Rings::new(settings, 32.0);
            assert!(r.keep.horizontal >= r.mesh.horizontal, "{r:?}");
            assert!(r.load.horizontal > r.keep.horizontal, "{r:?}");
            assert!(r.evict.horizontal > r.load.horizontal, "{r:?}");
            assert!(r.load.vertical > r.keep.vertical, "{r:?}");
            assert!(r.evict.vertical > r.load.vertical, "{r:?}");
        }
    }

    /// The property eviction's correctness rests on. If an evicted Chunk could
    /// be adjacent to one the Client has a mesh for, that mesh's boundary faces
    /// would be stale and eviction would have to trigger a re-mesh.
    #[test]
    fn no_evicted_chunk_is_adjacent_to_one_that_keeps_its_mesh() {
        let r = rings(1.0);
        let reach = r.evict.horizontal.max(r.evict.vertical) + 2;
        for y in -reach..=reach {
            for z in -reach..=reach {
                for x in -reach..=reach {
                    let d = IVec3::new(x, y, z);
                    if r.evict.contains(d) {
                        continue;
                    }
                    for step in [IVec3::X, IVec3::Y, IVec3::Z] {
                        for sign in [1, -1] {
                            let n = d + step * sign;
                            assert!(!r.keep.contains(n), "evicted {d} touches kept {n}");
                        }
                    }
                }
            }
        }
    }

    /// Why `load` is `keep` grown by one: the outermost Chunk that keeps a mesh
    /// must have all six neighbours generated, or it meshes a skin of faces
    /// buried in rock that has not arrived yet.
    #[test]
    fn every_neighbour_of_a_kept_chunk_is_inside_the_load_extent() {
        let r = rings(1.0);
        for d in r.keep.offsets() {
            for step in [IVec3::X, IVec3::Y, IVec3::Z] {
                for sign in [1, -1] {
                    let n = d + step * sign;
                    assert!(r.load.contains(n), "kept {d} needs {n}");
                }
            }
        }
    }

    #[test]
    fn a_metre_view_distance_doubles_in_cells_when_voxel_size_halves() {
        // ADR-0003: the setting is in metres, so the same World streams the
        // same distance whatever a Voxel is worth.
        let coarse = rings(1.0);
        let fine = rings(0.5);
        assert_eq!(fine.mesh.horizontal, coarse.mesh.horizontal * 2);
        assert_eq!(fine.mesh.vertical, coarse.mesh.vertical * 2);
    }

    #[test]
    fn offsets_are_the_extent_exactly_and_nearest_first() {
        let extent = Extent {
            horizontal: 5,
            vertical: 2,
        };
        let offsets = extent.offsets();

        let unique: HashSet<IVec3> = offsets.iter().copied().collect();
        assert_eq!(unique.len(), offsets.len(), "duplicates");
        assert!(offsets.iter().all(|&d| extent.contains(d)));

        // Nothing inside the extent is missing.
        for y in -3..=3 {
            for z in -6..=6 {
                for x in -6..=6 {
                    let d = IVec3::new(x, y, z);
                    assert_eq!(extent.contains(d), unique.contains(&d), "{d}");
                }
            }
        }

        assert!(
            offsets[0] == IVec3::ZERO,
            "the anchor's own Cell comes first"
        );
        assert!(
            offsets
                .windows(2)
                .all(|w| w[0].length_squared() <= w[1].length_squared()),
            "not sorted by distance"
        );
    }

    #[test]
    fn a_view_distance_below_one_cell_still_renders_the_anchors_own_chunk() {
        let r = Rings::new(
            StreamingSettings {
                view_distance_m: 1.0,
                vertical_distance_m: 1.0,
                hysteresis_m: 1.0,
                ..default()
            },
            32.0,
        );
        assert!(r.mesh.contains(IVec3::ZERO));
    }

    // The loop itself, driven headlessly. `run_system_once` rather than letting
    // `FixedUpdate` fire, so a tick is a function call and nothing depends on
    // simulated time.

    /// Small enough to fill in one tick: mesh (1, 1), keep (2, 2),
    /// load (3, 3), evict (4, 4).
    pub(in crate::server) fn small() -> StreamingSettings {
        StreamingSettings {
            view_distance_m: 32.0,
            vertical_distance_m: 32.0,
            hysteresis_m: 32.0,
            ..default()
        }
    }

    pub(in crate::server) const UNLIMITED: StreamingBudget = StreamingBudget {
        in_flight: StageCounts {
            generate: usize::MAX,
            relight: usize::MAX,
            mesh: usize::MAX,
        },
        landed_per_tick: StageCounts {
            generate: usize::MAX,
            relight: usize::MAX,
            mesh: usize::MAX,
        },
        wait_for_settled_light: true,
    };

    /// How long `settle` waits before it calls the streaming stuck. Wall time,
    /// not ticks: the tests share one pool, and a fill that is slow because
    /// twenty other tests are filling is not a hang.
    pub(in crate::server) const SETTLE_CEILING: std::time::Duration =
        std::time::Duration::from_secs(60);

    pub(in crate::server) struct Harness {
        pub(in crate::server) app: App,
        pub(in crate::server) anchor: Entity,
    }

    impl Harness {
        pub(in crate::server) fn new(settings: StreamingSettings, budget: StreamingBudget) -> Self {
            Self::with_world(
                settings,
                budget,
                WorldSettings::default(),
                tesserae_core::testgen::world_params(),
            )
        }

        pub(in crate::server) fn with_world(
            settings: StreamingSettings,
            budget: StreamingBudget,
            world_settings: WorldSettings,
            params: WorldgenParams,
        ) -> Self {
            // One pool for the whole test process; `get_or_init` is a no-op after
            // the first test, and the thread count is fixed so nothing a test
            // asserts depends on the machine.
            AsyncComputeTaskPool::get_or_init(|| TaskPoolBuilder::new().num_threads(4).build());
            let generator = Worldgen::new(world_settings.seed, world_settings.voxel_size, params);
            let lod_generators =
                lod::LodGenerators::new(&world_settings, &generator, level_count(&settings));
            let mut app = App::new();
            app.insert_resource(world_settings)
                .insert_resource(settings)
                .insert_resource(budget)
                .insert_resource(WorldGenerator(generator))
                .insert_resource(lod_generators)
                .init_resource::<lod::LodStore>()
                .init_resource::<lod::LodState>()
                .init_resource::<lod::LodJobs>()
                .init_resource::<lod::LodOrders>()
                .init_resource::<lod::LodBudget>()
                .insert_resource(ContentTables(Arc::new(crate::server::fixture_tables())))
                .init_resource::<ChunkStore>()
                .init_resource::<LoadOrder>()
                .init_resource::<MeshState>()
                .init_resource::<Jobs>()
                .init_resource::<crate::server::fluids::FluidSim>()
                .add_message::<ChunkMeshReady>()
                .add_message::<ChunkUnloaded>()
                .add_message::<LodMeshReady>()
                .add_message::<LodUnloaded>();
            let anchor = app.world_mut().spawn(Player::at(Vec3::ZERO)).id();
            Harness { app, anchor }
        }

        pub(in crate::server) fn anchor_at(&mut self, metres: Vec3) {
            self.app
                .world_mut()
                .get_mut::<Player>(self.anchor)
                .unwrap()
                .set_eye(metres);
        }

        pub(in crate::server) fn tick(&mut self) -> (Vec<ChunkMeshReady>, Vec<ChunkCoord>) {
            self.app
                .world_mut()
                .run_system_once(stream)
                .expect("the tick ran");
            let ready = self
                .app
                .world_mut()
                .resource_mut::<Messages<ChunkMeshReady>>()
                .drain()
                .collect();
            let unloaded = self
                .app
                .world_mut()
                .resource_mut::<Messages<ChunkUnloaded>>()
                .drain()
                .map(|m| m.coord)
                .collect();
            (ready, unloaded)
        }

        /// Ticks until nothing is in flight, nothing was dispatched or landed,
        /// no light is stale and no message was written, and returns every
        /// message written on the way. The honest form of "run until done" now
        /// that a tick starts work instead of finishing it.
        pub(in crate::server) fn settle(&mut self) -> (Vec<ChunkMeshReady>, Vec<ChunkCoord>) {
            let (mut ready, mut unloaded) = (Vec::new(), Vec::new());
            let started = std::time::Instant::now();
            while started.elapsed() < SETTLE_CEILING {
                let (r, u) = self.tick();
                let quiet = r.is_empty() && u.is_empty() && self.quiet();
                ready.extend(r);
                unloaded.extend(u);
                if quiet {
                    return (ready, unloaded);
                }
                if self.jobs().in_flight() > 0 {
                    std::thread::sleep(std::time::Duration::from_micros(100));
                }
            }
            panic!("streaming was still busy after {SETTLE_CEILING:?}");
        }

        fn quiet(&self) -> bool {
            let jobs = self.jobs();
            let (dispatched, landed) = jobs.generate.counts();
            jobs.in_flight() == 0
                && dispatched == 0
                && landed == 0
                && !self
                    .app
                    .world()
                    .resource::<MeshState>()
                    .unlit_within(self.center(), self.rings().load)
        }

        fn center(&self) -> ChunkCoord {
            let player = self.app.world().get::<Player>(self.anchor);
            let world = self.app.world().resource::<WorldSettings>();
            world.chunk_at(player.expect("the Player exists").eye())
        }

        fn jobs(&self) -> &Jobs {
            self.app.world().resource::<Jobs>()
        }

        fn jobs_mut(&mut self) -> Mut<'_, Jobs> {
            self.app.world_mut().resource_mut::<Jobs>()
        }

        pub(in crate::server) fn store(&self) -> &ChunkStore {
            self.app.world().resource::<ChunkStore>()
        }

        fn delivered(&self) -> HashSet<ChunkCoord> {
            self.app.world().resource::<MeshState>().delivered.clone()
        }

        fn rings(&self) -> Rings {
            let world = self.app.world().resource::<WorldSettings>();
            Rings::new(
                *self.app.world().resource::<StreamingSettings>(),
                world.cell_size(),
            )
        }
    }

    /// The Server tick's streaming half, level 0 then the coarse levels, on a
    /// schedule a measurement runs by itself.
    #[cfg(feature = "dev")]
    pub(in crate::server) fn add_stream_systems(
        app: &mut App,
        schedule: impl bevy::ecs::schedule::ScheduleLabel,
    ) {
        app.add_systems(schedule, (stream, lod::stream_lod).chain());
    }

    pub(in crate::server) fn same_mesh(a: &ChunkMesh, b: &ChunkMesh) -> bool {
        let layer = |a: &MeshLayer, b: &MeshLayer| {
            a.positions == b.positions
                && a.normals == b.normals
                && a.uvs == b.uvs
                && a.materials == b.materials
                && a.light == b.light
                && a.indices == b.indices
        };
        layer(&a.opaque, &b.opaque) && layer(&a.blended, &b.blended) && layer(&a.cutout, &b.cutout)
    }

    /// The light of the Chunk a mesh was built from, which `publish` requires
    /// to be present before it meshes at all.
    pub(in crate::server) fn lit<'a>(store: &'a ChunkStore, coord: ChunkCoord) -> LitChunk<'a> {
        LitChunk::new(
            store.get(coord).expect("loaded"),
            store.light(coord).expect("lit"),
        )
        .with_overlay(store.overlay(coord).expect("held"))
    }

    #[test]
    fn settling_fills_the_load_volume_and_meshes_only_what_has_neighbours() {
        let mut h = Harness::new(small(), UNLIMITED);
        let (ready, unloaded) = h.settle();
        assert!(unloaded.is_empty(), "nothing to unload on the first tick");

        let rings = h.rings();
        let want: HashSet<ChunkCoord> = rings
            .load
            .offsets()
            .into_iter()
            .map(|d| offset(ChunkCoord::new(0, 0, 0), d))
            .collect();
        assert_eq!(h.store().coords().collect::<HashSet<_>>(), want);

        for message in &ready {
            let d = delta(ChunkCoord::new(0, 0, 0), message.coord);
            assert!(rings.mesh.contains(d), "{} is outside mesh", message.coord);
            for n in face_neighbors(message.coord) {
                assert!(
                    h.store().contains(n),
                    "sent {} without neighbour {n}",
                    message.coord
                );
            }
        }
        // A Chunk can be sent more than once on the way, as its light settles.
        assert_eq!(
            h.delivered(),
            ready.iter().map(|m| m.coord).collect::<HashSet<_>>()
        );
    }

    /// The property dirty-marking exists for, and the test `generate`'s comment
    /// names. Work arrives over many ticks, so a Chunk meshed early has
    /// neighbours that did not exist yet and light that had not settled. Faces
    /// are covered by `publish` refusing to mesh without all six neighbours;
    /// light is not, because it crosses a Chunk boundary and keeps going, so
    /// `relight` has to re-publish a Chunk whose light moved. If it did not,
    /// the Client would be left holding a mesh baked against light that is no
    /// longer true — a visible seam rather than a buried face.
    #[test]
    fn no_mesh_the_client_still_holds_is_stale() {
        let mut h = Harness::new(
            small(),
            StreamingBudget {
                in_flight: StageCounts {
                    generate: 3,
                    relight: 3,
                    mesh: 2,
                },
                landed_per_tick: StageCounts {
                    generate: 3,
                    relight: 3,
                    mesh: 2,
                },
                wait_for_settled_light: true,
            },
        );

        let mut latest: bevy::platform::collections::HashMap<ChunkCoord, ChunkMesh> =
            Default::default();
        for message in h.settle().0 {
            latest.insert(message.coord, message.mesh);
        }

        assert!(!latest.is_empty(), "nothing was ever delivered");
        assert_eq!(
            h.delivered(),
            latest.keys().copied().collect::<HashSet<_>>(),
            "delivered set and messages disagree"
        );

        for (coord, held) in &latest {
            let store = h.store();
            let neighbors: [Option<LitChunk>; 6] =
                face_neighbors(*coord).map(|n| Some(lit(store, n)));
            let fresh = mesh_chunk(
                lit(store, *coord),
                neighbors,
                &crate::server::fixture_tables(),
            );
            assert!(
                same_mesh(held, &fresh),
                "{coord}: the Client holds a stale mesh ({} quads, should be {})",
                held.quad_count(),
                fresh.quad_count()
            );
        }
    }

    /// The Edit's end to end: a Voxel removed in the dark moves no light, yet the
    /// Chunk it was in and the one across the boundary both get a new face, so
    /// both must be republished, and republished as what `mesh_chunk` makes of the
    /// store as it now stands. The neighbour is the one only the boundary rule in
    /// `edit::set_voxel` reaches.
    #[test]
    fn an_edit_on_a_chunk_boundary_republishes_both_chunks_with_fresh_meshes() {
        let mut h = Harness::new(small(), UNLIMITED);
        h.settle();
        // Deep rock, a boundary apart: both inside the mesh extent and solid.
        let (here, next) = (ChunkCoord::new(0, -1, 0), ChunkCoord::new(1, -1, 0));
        assert!(h.delivered().contains(&here) && h.delivered().contains(&next));

        let at = [CHUNK_SIZE as i32 - 1, -16, 16];
        h.app
            .world_mut()
            .run_system_once(
                move |mut store: ResMut<ChunkStore>,
                      mut meshes: ResMut<MeshState>,
                      tables: Res<ContentTables>| {
                    edit::set_voxel(&mut store, &mut meshes, &tables, at, Voxel::AIR)
                        .expect("the Chunk is loaded");
                },
            )
            .unwrap();
        let (ready, _) = h.settle();

        let store = h.store();
        for coord in [here, next] {
            let republished = ready
                .iter()
                .find(|m| m.coord == coord)
                .unwrap_or_else(|| panic!("{coord} was not republished"));
            assert!(!republished.mesh.is_empty(), "{coord} has a new face");
            let neighbors: [Option<LitChunk>; 6] =
                face_neighbors(coord).map(|n| Some(lit(store, n)));
            let fresh = mesh_chunk(
                lit(store, coord),
                neighbors,
                &crate::server::fixture_tables(),
            );
            assert!(same_mesh(&republished.mesh, &fresh), "{coord} is stale");
        }
        assert_eq!(ready.len(), 2, "nothing else was touched");
    }

    /// The Client walks these and never reads World storage, so what ships
    /// with a mesh has to be the Chunk as it stands, Edits included.
    #[cfg(any(feature = "dev", feature = "profiling"))]
    #[test]
    fn every_mesh_carries_the_sight_lines_of_its_chunk_as_it_stands() {
        use tesserae_core::visibility::connectivity;
        let mut h = Harness::new(small(), UNLIMITED);
        let (ready, _) = h.settle();
        let tables = crate::server::fixture_tables();
        let now = |h: &Harness, c| connectivity(h.store().get(c).unwrap(), &tables.materials);
        assert!(!ready.is_empty());
        for m in &ready {
            assert_eq!(m.connectivity, now(&h, m.coord), "{}", m.coord);
        }

        let deep = ChunkCoord::new(0, -1, 0);
        let before = now(&h, deep);
        h.app
            .world_mut()
            .run_system_once(
                |mut store: ResMut<ChunkStore>,
                 mut meshes: ResMut<MeshState>,
                 tables: Res<ContentTables>| {
                    for x in 0..CHUNK_SIZE as i32 {
                        edit::set_voxel(&mut store, &mut meshes, &tables, [x, -16, 16], Voxel::AIR)
                            .expect("the Chunk is loaded");
                    }
                },
            )
            .unwrap();
        let (ready, _) = h.settle();
        let tunnel = ready
            .iter()
            .rev()
            .find(|m| m.coord == deep)
            .expect("the tunnelled Chunk is republished");
        assert!(tunnel.connectivity.connects(NEG_X, POS_X));
        assert_ne!(tunnel.connectivity, before);
        assert_eq!(tunnel.connectivity, now(&h, deep));
    }

    #[cfg(any(feature = "dev", feature = "profiling"))]
    #[test]
    fn every_level_zero_mesh_job_is_charged_to_reached_or_unreached() {
        let mut h = Harness::new(small(), UNLIMITED);
        h.app.world_mut().resource_mut::<MeshState>().reach.on = true;
        h.settle();
        let [(reached, _), (unreached, _)] = h.app.world().resource::<MeshState>().reach.totals();
        let (run, _) = h.jobs().mesh.run();
        assert!(run > 0);
        assert_eq!(reached + unreached, run);
    }

    #[test]
    fn moving_the_anchor_loads_ahead_and_unloads_behind() {
        let mut h = Harness::new(small(), UNLIMITED);
        h.settle();
        let was_delivered = h.delivered();
        let was_loaded: HashSet<ChunkCoord> = h.store().coords().collect();
        assert!(!was_delivered.is_empty());

        // Ten Cells, well past the evict extent, so everything behind goes.
        let cell = h.app.world().resource::<WorldSettings>().cell_size();
        h.anchor_at(Vec3::new(10.0 * cell, 0.0, 0.0));
        let (_, unloaded) = h.tick();

        assert_eq!(
            unloaded.iter().copied().collect::<HashSet<_>>(),
            was_delivered,
            "exactly the Chunks the Client was drawing should be dropped"
        );
        for coord in &was_loaded {
            assert!(!h.store().contains(*coord), "{coord} was not evicted");
        }
        let (ready, _) = h.settle();
        assert!(
            ready.iter().all(|m| m.coord.x > 0),
            "new meshes should be ahead of the anchor"
        );
        assert!(!ready.is_empty(), "nothing streamed in ahead");
    }

    /// Hysteresis. Without it, an anchor hovering on a Cell boundary would
    /// evict and regenerate the same Chunks every tick.
    #[test]
    fn crossing_a_cell_boundary_does_not_thrash() {
        let mut h = Harness::new(small(), UNLIMITED);
        h.settle();
        let settled: HashSet<ChunkCoord> = h.store().coords().collect();

        let cell = h.app.world().resource::<WorldSettings>().cell_size();
        for step in [cell - 0.01, cell, cell + 0.01, cell - 0.01, 0.0] {
            h.anchor_at(Vec3::new(step, 0.0, 0.0));
            let (_, unloaded) = h.tick();
            assert!(unloaded.is_empty(), "thrashed at {step} m: {unloaded:?}");
        }
        assert!(
            settled.is_subset(&h.store().coords().collect()),
            "a Chunk was evicted and had to be regenerated"
        );
    }

    /// A Chunk can leave the visible extent and come back without being
    /// evicted, because `keep` is narrower than `evict`. Its mesh is dropped on
    /// the way out, so something has to ask for it again on the way back.
    #[test]
    fn a_chunk_that_leaves_the_visible_extent_is_redrawn_when_it_returns() {
        let mut h = Harness::new(small(), UNLIMITED);
        h.settle();
        let home = ChunkCoord::new(0, 0, 0);
        assert!(h.delivered().contains(&home));

        // Three Cells: past `keep` (2) so the mesh is dropped, inside `evict`
        // (4) so the Chunk itself stays loaded.
        let cell = h.app.world().resource::<WorldSettings>().cell_size();
        h.anchor_at(Vec3::new(3.0 * cell, 0.0, 0.0));
        let (_, unloaded) = h.tick();
        assert!(
            unloaded.contains(&home),
            "the mesh should have been dropped"
        );
        assert!(h.store().contains(home), "the Chunk should still be loaded");

        h.anchor_at(Vec3::ZERO);
        let (ready, _) = h.settle();
        assert!(
            ready.iter().any(|m| m.coord == home),
            "{home} came back into view and was never redrawn"
        );
    }

    #[test]
    fn a_tick_with_no_player_does_nothing() {
        let mut h = Harness::new(small(), UNLIMITED);
        h.app.world_mut().entity_mut(h.anchor).despawn();
        let (ready, unloaded) = h.tick();
        assert!(ready.is_empty() && unloaded.is_empty());
        assert_eq!(h.store().coords().count(), 0);
    }

    #[test]
    fn in_flight_never_exceeds_the_cap_and_landings_never_exceed_theirs() {
        let mut h = Harness::new(
            small(),
            StreamingBudget {
                in_flight: StageCounts {
                    generate: 5,
                    ..UNLIMITED.in_flight
                },
                landed_per_tick: StageCounts {
                    generate: 2,
                    relight: 2,
                    mesh: 1,
                },
                wait_for_settled_light: true,
            },
        );

        // Dispatch is synchronous, so the first tick's count does not depend on
        // how fast the workers are, and nothing has landed yet to be capped.
        h.tick();
        assert_eq!(h.jobs().generate.counts().0, 5);
        assert_eq!(h.store().coords().count(), 0);

        let started = std::time::Instant::now();
        while started.elapsed() < SETTLE_CEILING {
            let (ready, _) = h.tick();
            let jobs = h.jobs();
            assert!(jobs.generate.in_flight() <= 5, "over the in-flight cap");
            assert!(jobs.generate.counts().1 <= 2, "landed more than allowed");
            assert!(ready.len() <= 1, "meshed more than allowed");
            if h.quiet() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_micros(100));
        }
        panic!("never went quiet");
    }

    /// The anchor can move while a job is out. Nothing it was generating for the
    /// old position may land, or the store would hold Chunks `retire` would only
    /// remove again next tick.
    #[test]
    fn a_job_in_flight_for_a_chunk_the_anchor_leaves_lands_nothing() {
        let mut h = Harness::new(
            small(),
            StreamingBudget {
                in_flight: StageCounts {
                    generate: 8,
                    ..UNLIMITED.in_flight
                },
                ..UNLIMITED
            },
        );
        h.tick();
        let started: Vec<ChunkCoord> = h
            .rings()
            .load
            .offsets()
            .into_iter()
            .take(8)
            .map(|d| offset(ChunkCoord::new(0, 0, 0), d))
            .collect();
        assert_eq!(h.jobs().generate.counts().0, 8);

        let cell = h.app.world().resource::<WorldSettings>().cell_size();
        h.anchor_at(Vec3::new(20.0 * cell, 0.0, 0.0));
        h.settle();

        for coord in started {
            assert!(!h.store().contains(coord), "{coord} landed after the move");
        }
        let center = ChunkCoord::new(20, 0, 0);
        let evict = h.rings().evict;
        for coord in h.store().coords() {
            assert!(
                evict.contains(delta(center, coord)),
                "{coord} is outside the evict extent of the new anchor"
            );
        }
    }

    // Light. The Server owns it, so these tests live here rather than on the
    // Client that renders it.

    /// Everything above the band the surface can reach is air the generator
    /// made, so the Chunk at the top of the load extent sees full sky. The
    /// relaxation cannot discover this: there is no neighbour above to ask, and
    /// guessing dark would darken every Voxel in the column below, because sky
    /// light at full strength falls straight down undimmed.
    #[test]
    fn the_open_sky_boundary_is_taken_from_the_generator() {
        let mut h = Harness::new(small(), UNLIMITED);
        let generator = Worldgen::new(
            WorldSettings::default().seed,
            WorldSettings::default().voxel_size,
            tesserae_core::testgen::world_params(),
        );
        let (_, highest_m) = generator.surface_bounds_m();
        let cell = WorldSettings::default().cell_size();

        // Well above the band, so every Chunk the extent holds is open air.
        h.anchor_at(Vec3::new(0.0, highest_m + 4.0 * cell, 0.0));
        h.settle();

        let center = h
            .app
            .world()
            .resource::<WorldSettings>()
            .chunk_at(Vec3::new(0.0, highest_m + 4.0 * cell, 0.0));
        let light = h.store().light(center).expect("solved");
        assert_eq!(
            light.uniform().map(|l| l.sky()),
            Some(MAX_LIGHT),
            "a Chunk of open air must be fully sky-lit"
        );
    }

    #[test]
    fn deep_rock_stays_dark() {
        let mut h = Harness::new(small(), UNLIMITED);
        let generator = Worldgen::new(
            WorldSettings::default().seed,
            WorldSettings::default().voxel_size,
            tesserae_core::testgen::world_params(),
        );
        let (lowest_m, _) = generator.surface_bounds_m();
        let cell = WorldSettings::default().cell_size();
        let under = Vec3::new(0.0, lowest_m - 5.0 * cell, 0.0);

        h.anchor_at(under);
        h.settle();

        let center = h.app.world().resource::<WorldSettings>().chunk_at(under);
        assert_eq!(
            h.store().light(center).and_then(ChunkLight::uniform),
            Some(Light::DARK),
            "the generator makes solid stone down here"
        );
    }

    /// Light is baked into the vertex attribute, so a Chunk that reaches the
    /// Client must already be lit — a mesh published before its light settled
    /// would carry a black seam.
    #[test]
    fn nothing_is_published_before_its_light_is_solved() {
        let mut h = Harness::new(small(), UNLIMITED);
        let (ready, _) = h.settle();
        assert!(!ready.is_empty());
        // A Chunk, once lit, stays lit until it is evicted, so the end state is
        // enough to say every message was preceded by its light.
        for message in ready {
            assert!(
                h.store().light(message.coord).is_some(),
                "{} was published unlit",
                message.coord
            );
            for n in face_neighbors(message.coord) {
                assert!(
                    h.store().light(n).is_some(),
                    "{} was published while neighbour {n} was unlit",
                    message.coord
                );
            }
        }
    }

    /// A relight that changes nothing must not re-publish, or the surface would
    /// re-mesh every tick forever at 371 us a Chunk.
    #[test]
    fn a_relight_that_changes_nothing_does_not_republish() {
        let mut h = Harness::new(small(), UNLIMITED);
        h.settle();
        let (ready, unloaded) = h.tick();
        assert!(ready.is_empty(), "re-published {} Chunks", ready.len());
        assert!(unloaded.is_empty());
        assert!(h.quiet(), "the relaxation never settled");
    }

    /// Visible light on visible faces, at the surface. The whole step is
    /// pointless if the Client is handed zeroes.
    #[test]
    fn a_surface_chunk_is_published_with_light_on_it() {
        let mut h = Harness::new(small(), UNLIMITED);
        let mut lit_faces = 0;
        for message in h.settle().0 {
            lit_faces += message
                .mesh
                .opaque
                .light
                .iter()
                .filter(|&&l| l != 0)
                .count();
        }
        assert!(lit_faces > 0, "every published face was baked dark");
    }

    /// The light the bulk solver converges to, found the plainest way there is:
    /// sweep every Chunk against its neighbours' current light, highest first,
    /// until a whole pass changes nothing. What `relight` did tick by tick when
    /// it ran on the tick, and so what the jobs must agree with whatever order
    /// they finish in.
    fn fixed_point(
        store: &ChunkStore,
        generator: &WorldGenerator,
        world: &WorldSettings,
    ) -> bevy::platform::collections::HashMap<ChunkCoord, ChunkLight> {
        let tables = crate::server::fixture_tables();
        let sky_chunk = Chunk::air();
        let open_sky = ChunkLight::filled(Light::new(MAX_LIGHT, 0));
        let mut coords: Vec<ChunkCoord> = store.coords().collect();
        coords.sort_by_key(|c| -c.y);
        let mut lights: bevy::platform::collections::HashMap<ChunkCoord, ChunkLight> =
            coords.iter().map(|&c| (c, ChunkLight::dark())).collect();
        loop {
            let mut changed = false;
            for &coord in &coords {
                let border: [Option<LitChunk>; 6] =
                    face_neighbors(coord).map(|n| match (store.get(n), lights.get(&n)) {
                        (Some(chunk), Some(light)) => Some(LitChunk::new(chunk, light)),
                        _ => (n.y > coord.y && above_the_surface(generator, world, n))
                            .then_some(LitChunk::new(&sky_chunk, &open_sky)),
                    });
                let solved = light_chunk(store.get(coord).expect("loaded"), border, &tables);
                if solved != lights[&coord] {
                    lights.insert(coord, solved);
                    changed = true;
                }
            }
            if !changed {
                return lights;
            }
        }
    }

    fn assert_lit_like(
        h: &Harness,
        expected: &bevy::platform::collections::HashMap<ChunkCoord, ChunkLight>,
    ) {
        for coord in h.store().coords() {
            assert_eq!(
                h.store().light(coord).expect("lit"),
                &expected[&coord],
                "{coord} is not at the fixed point"
            );
        }
    }

    /// Spec decision 5, checked rather than argued: whatever order the jobs
    /// finish in, and however many are allowed at once, the relaxation ends at
    /// the light the synchronous sweep finds. A stale read costs another pass,
    /// not a different answer.
    #[test]
    fn relight_in_scrambled_landing_order_reaches_the_synchronous_fixed_point() {
        for cap in [1, 4, usize::MAX] {
            let mut h = Harness::new(
                small(),
                StreamingBudget {
                    in_flight: StageCounts {
                        relight: cap,
                        ..UNLIMITED.in_flight
                    },
                    ..UNLIMITED
                },
            );
            h.app.world_mut().resource_mut::<Jobs>().relight.scramble = true;
            h.settle();

            let expected = fixed_point(
                h.store(),
                h.app.world().resource::<WorldGenerator>(),
                h.app.world().resource::<WorldSettings>(),
            );
            assert_lit_like(&h, &expected);
            let dense = expected.values().filter(|l| l.uniform().is_none()).count();
            assert!(dense > 0, "cap {cap}: no Chunk had a gradient to get wrong");
        }
    }

    /// Light solves run per generated Chunk while a fresh region fills, at the
    /// default budget on a pool the size of the engine's, as after a teleport.
    fn solves_per_chunk_filling_a_fresh_region() -> f32 {
        let mut h = Harness::new(
            StreamingSettings {
                view_distance_m: 96.0,
                vertical_distance_m: 64.0,
                ..small()
            },
            StreamingBudget::default(),
        );
        let pool = Arc::new(TaskPoolBuilder::new().num_threads(3).build());
        {
            let mut jobs = h.jobs_mut();
            jobs.generate.pool = Some(pool.clone());
            jobs.relight.pool = Some(pool.clone());
            jobs.mesh.pool = Some(pool);
        }
        let ground_m = h
            .app
            .world()
            .resource::<WorldGenerator>()
            .0
            .surface_height_m(16.0, 16.0);
        h.anchor_at(Vec3::new(16.0, ground_m + 2.0, 16.0));
        let (mut generated, mut solves) = (0, 0);
        let started = std::time::Instant::now();
        loop {
            let (ready, unloaded) = h.tick();
            generated += h.jobs().generate.counts().1;
            solves += h.jobs().relight.counts().0;
            if ready.is_empty() && unloaded.is_empty() && h.quiet() {
                return solves as f32 / generated as f32;
            }
            assert!(started.elapsed() < SETTLE_CEILING, "the fill never settled");
            std::thread::sleep(std::time::Duration::from_secs_f64(1.0 / 64.0));
        }
    }

    /// Here the pre-03 order, highest layer first by distance, measured 3.3 and
    /// column by column 4.2; the bound leaves room for when the pool's jobs
    /// finish.
    #[test]
    fn a_fresh_region_is_lit_without_solving_chunks_again_for_light_from_above() {
        let solves = solves_per_chunk_filling_a_fresh_region();
        assert!(solves <= 3.6, "{solves} solves per generated Chunk");
    }

    /// Where a lantern next to a Chunk boundary lights two Chunks, and what
    /// places it.
    fn lantern_on_a_boundary(h: &mut Harness) -> ([i32; 3], ChunkCoord) {
        let generator = h.app.world().resource::<WorldGenerator>();
        let ground = generator.surface_height_m(16.0, 16.0).floor() as i32;
        let at = [CHUNK_SIZE as i32 - 1, ground + 2, 16];
        let neighbour = VoxelCoord::of([at[0] + 1, at[1], at[2]]).chunk;
        (at, neighbour)
    }

    fn place_lantern(h: &mut Harness, at: [i32; 3]) {
        h.app
            .world_mut()
            .run_system_once(
                move |mut store: ResMut<ChunkStore>,
                      mut meshes: ResMut<MeshState>,
                      tables: Res<ContentTables>| {
                    edit::set_voxel(
                        &mut store,
                        &mut meshes,
                        &tables,
                        at,
                        Voxel::cube(tesserae_core::testgen::fixtures().lantern),
                    )
                    .expect("the Chunk is loaded and lit");
                },
            )
            .unwrap();
    }

    /// A mesh built against light that is about to move is built twice, so a
    /// Chunk waits until it and its six neighbours are neither stale nor being
    /// solved.
    #[test]
    fn a_mesh_waits_for_its_neighbourhood_to_settle() {
        let mut h = Harness::new(small(), UNLIMITED);
        h.settle();
        let (home, next) = (ChunkCoord::new(0, 0, 0), ChunkCoord::new(1, 0, 0));

        h.jobs_mut().relight.hold.hold();
        {
            let mut meshes = h.app.world_mut().resource_mut::<MeshState>();
            meshes.mark_unlit(next);
            meshes.edited(home);
        }
        h.tick();
        assert!(
            h.jobs().relight.contains(next),
            "the neighbour was not solved"
        );
        assert!(
            !h.jobs().mesh.contains(home),
            "{home} was meshed while {next} was being solved"
        );

        h.jobs_mut().relight.hold.release();
        let (ready, _) = h.settle();
        assert!(
            ready.iter().any(|m| m.coord == home),
            "{home} was never meshed once its neighbourhood settled"
        );
    }

    /// A Chunk edited faster than a mesh takes to build must still show the
    /// Client something: each result is already behind the next edit, and
    /// dropping it for that would mean never delivering at all.
    #[test]
    fn sustained_edits_to_one_chunk_still_deliver_meshes() {
        let mut h = Harness::new(small(), UNLIMITED);
        h.settle();
        h.jobs_mut().mesh.scramble = true;
        let ground = h
            .app
            .world()
            .resource::<WorldGenerator>()
            .surface_height_m(16.0, 16.0)
            .floor() as i32;
        let at = [16, ground + 1, 16];
        let chunk = VoxelCoord::of(at).chunk;

        let mut delivered = 0;
        let started = std::time::Instant::now();
        let mut i = 0;
        // Until a mesh lands behind an edit, which only needs the pool to get to
        // it; other tests share it, so how long that takes is not this test's to
        // know.
        while (delivered == 0 || h.jobs().mesh.stale() == 0) && started.elapsed() < SETTLE_CEILING {
            i += 1;
            let voxel = if i % 2 == 0 {
                Voxel::cube(tesserae_core::testgen::fixtures().ground.rock)
            } else {
                Voxel::AIR
            };
            h.app
                .world_mut()
                .run_system_once(
                    move |mut store: ResMut<ChunkStore>,
                          mut meshes: ResMut<MeshState>,
                          tables: Res<ContentTables>| {
                        edit::set_voxel(&mut store, &mut meshes, &tables, at, voxel)
                            .expect("loaded");
                    },
                )
                .unwrap();
            let (ready, _) = h.tick();
            delivered += ready.iter().filter(|m| m.coord == chunk).count();
            std::thread::sleep(std::time::Duration::from_micros(20));
        }
        assert!(
            delivered > 0,
            "{chunk} was edited every tick and never redrawn"
        );
        assert!(
            h.jobs().mesh.stale() > 0,
            "no mesh landed behind an edit, so the test did not exercise it"
        );
    }

    /// The anchor can fly off while meshes are being built for where it was. No
    /// result may reach the Client for a Chunk outside the mesh extent it now
    /// has, and what the Client ends up holding is current. Two distances: past
    /// `keep`, where `retire` cancels the job, and between `mesh` and `keep`,
    /// where only the check at landing stands in the way.
    #[test]
    fn a_fly_by_with_meshes_outstanding_delivers_nothing_outside_the_new_extent() {
        for cells in [2.0, 10.0] {
            let mut h = Harness::new(small(), UNLIMITED);
            h.settle();

            h.jobs_mut().mesh.hold.hold();
            let delivered: Vec<ChunkCoord> = h.delivered().into_iter().collect();
            {
                let mut meshes = h.app.world_mut().resource_mut::<MeshState>();
                for &coord in &delivered {
                    meshes.edited(coord);
                }
            }
            h.tick();
            assert!(h.jobs().mesh.in_flight() > 0, "no mesh was in flight");

            let cell = h.app.world().resource::<WorldSettings>().cell_size();
            h.anchor_at(Vec3::new(cells * cell, 0.0, 0.0));
            let (_, unloaded) = h.tick();
            if cells > 5.0 {
                assert_eq!(
                    unloaded.len(),
                    delivered.len(),
                    "old meshes were not dropped"
                );
            }
            h.jobs_mut().mesh.hold.release();
            let (ready, _) = h.settle();

            let mesh = h.rings().mesh;
            let center = h.center();
            let mut latest: bevy::platform::collections::HashMap<ChunkCoord, ChunkMesh> =
                Default::default();
            for message in ready {
                assert!(
                    mesh.contains(delta(center, message.coord)),
                    "{} was sent from outside the mesh extent, {cells} Cells away",
                    message.coord
                );
                latest.insert(message.coord, message.mesh);
            }
            for coord in h.delivered() {
                assert!(
                    mesh.contains(delta(center, coord)) || !latest.contains_key(&coord),
                    "{coord} is held but outside the mesh extent"
                );
            }
            for (coord, held) in &latest {
                let store = h.store();
                let neighbors: [Option<LitChunk>; 6] =
                    face_neighbors(*coord).map(|n| Some(lit(store, n)));
                let fresh = mesh_chunk(
                    lit(store, *coord),
                    neighbors,
                    &crate::server::fixture_tables(),
                );
                assert!(
                    same_mesh(held, &fresh),
                    "{coord} is stale, {cells} Cells away"
                );
            }
        }
    }

    /// The fact spec decision 5 needs from the implementation: a neighbour's
    /// landing marks this Chunk stale while this Chunk's own solve is running,
    /// and that mark has to outlive the solve landing, or the Chunk is left
    /// solved against a plane that has since moved.
    #[test]
    fn a_mark_made_while_a_solve_runs_survives_its_landing() {
        let mut h = Harness::new(small(), UNLIMITED);
        h.settle();
        let home = ChunkCoord::new(0, 0, 0);

        h.jobs_mut().relight.hold.hold();
        h.app
            .world_mut()
            .resource_mut::<MeshState>()
            .mark_unlit(home);
        h.tick();
        assert!(h.jobs().relight.contains(home), "no solve started");
        // What a neighbour's landing does to it mid-flight.
        h.app
            .world_mut()
            .resource_mut::<MeshState>()
            .mark_unlit(home);
        h.jobs_mut().relight.hold.release();

        let mut solves = 0;
        let started = std::time::Instant::now();
        while started.elapsed() < SETTLE_CEILING {
            h.tick();
            solves += h.jobs().relight.counts().0;
            if h.quiet() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_micros(100));
        }
        assert!(h.quiet(), "never went quiet");
        assert!(
            solves >= 1,
            "the mark was lost and {home} was not solved again"
        );
    }

    /// An Edit's incremental relight is not monotone, so a bulk solve begun
    /// before it must not land over it. The neighbour across the boundary is the
    /// Chunk whose solve read the plane the Edit then changed.
    #[test]
    fn an_edit_while_a_relight_is_in_flight_drops_that_result() {
        let mut h = Harness::new(small(), UNLIMITED);
        h.settle();
        let (at, next) = lantern_on_a_boundary(&mut h);
        assert!(h.store().contains(next));

        h.app.world_mut().resource_mut::<Jobs>().relight.hold.hold();
        h.app
            .world_mut()
            .resource_mut::<MeshState>()
            .mark_unlit(next);
        h.tick();
        assert!(
            h.app.world().resource::<MeshState>().is_unlit(next),
            "the solve for {next} never started"
        );
        place_lantern(&mut h, at);
        h.app.world().resource::<Jobs>().relight.hold.release();
        h.settle();

        assert!(
            h.jobs().relight.stale() >= 1,
            "the stale solve landed over the Edit"
        );
        let expected = fixed_point(
            h.store(),
            h.app.world().resource::<WorldGenerator>(),
            h.app.world().resource::<WorldSettings>(),
        );
        assert_lit_like(&h, &expected);
        assert!(
            h.store().light_at([at[0] + 1, at[1], at[2]]).block() > 0,
            "the lantern's light never crossed the boundary"
        );
    }

    /// A Chunk evicted and generated again at the same coord is a different
    /// instance; a solve of the old one is not a solve of it.
    #[test]
    fn a_solve_of_a_chunk_that_was_replaced_meanwhile_does_not_land() {
        let mut h = Harness::new(small(), UNLIMITED);
        h.settle();
        let home = ChunkCoord::new(0, 0, 0);

        h.jobs_mut().relight.hold.hold();
        h.app
            .world_mut()
            .resource_mut::<MeshState>()
            .mark_unlit(home);
        h.tick();
        h.app
            .world_mut()
            .run_system_once(
                move |mut store: ResMut<ChunkStore>, mut meshes: ResMut<MeshState>| {
                    let again = store.get(home).expect("loaded").clone();
                    store.remove(home);
                    store.insert(home, again);
                    meshes.mark_unlit(home);
                },
            )
            .unwrap();
        h.jobs_mut().relight.hold.release();
        h.settle();

        assert!(
            h.jobs().relight.stale() >= 1,
            "the old instance's solve landed"
        );
        let expected = fixed_point(
            h.store(),
            h.app.world().resource::<WorldGenerator>(),
            h.app.world().resource::<WorldSettings>(),
        );
        assert_lit_like(&h, &expected);
    }

    /// A solve reads the overlay as it reads the Voxels: one that changed under
    /// it, with no Voxel moving and nothing marked, is not a solve of what the
    /// store now holds.
    #[test]
    fn a_solve_whose_overlay_changed_meanwhile_does_not_land() {
        let mut h = Harness::new(small(), UNLIMITED);
        h.settle();
        let home = ChunkCoord::new(0, 0, 0);

        h.jobs_mut().relight.hold.hold();
        h.app
            .world_mut()
            .resource_mut::<MeshState>()
            .mark_unlit(home);
        h.tick();
        assert!(h.jobs().relight.contains(home), "no solve started");
        h.app
            .world_mut()
            .run_system_once(move |mut store: ResMut<ChunkStore>| {
                store.set_overlay(
                    VoxelCoord::of([1, 1, 1]),
                    tesserae_core::Overlay {
                        fluid: None,
                        burning: Some(tesserae_core::Burning { fuel: 1 }),
                    },
                );
            })
            .unwrap();
        h.jobs_mut().relight.hold.release();
        h.settle();

        assert!(
            h.jobs().relight.stale() >= 1,
            "the solve of the old overlay landed"
        );
        let expected = fixed_point(
            h.store(),
            h.app.world().resource::<WorldGenerator>(),
            h.app.world().resource::<WorldSettings>(),
        );
        assert_lit_like(&h, &expected);
    }

    #[cfg(any(feature = "dev", feature = "profiling"))]
    #[test]
    fn stats_report_settled_only_when_the_server_has_nothing_left_to_do() {
        fn settled(h: &mut Harness) -> bool {
            h.app
                .world_mut()
                .run_system_once(stats::publish_stats)
                .expect("the summary ran");
            h.app.world().resource::<stats::StreamingStats>().settled
        }

        let mut h = Harness::new(small(), UNLIMITED);
        h.app.init_resource::<stats::StreamingStats>();
        assert!(!settled(&mut h), "settled before the first tick");
        assert!(
            !h.app.world().resource::<stats::StreamingStats>().shown,
            "shown before the first tick"
        );

        h.settle();
        assert!(settled(&mut h), "the fill never settled");
        assert!(
            h.app.world().resource::<stats::StreamingStats>().shown,
            "settled but the mesh extent was not all sent"
        );

        let cell = h.app.world().resource::<WorldSettings>().cell_size();
        h.anchor_at(Vec3::new(10.0 * cell, 0.0, 0.0));
        let mut slow = Harness::new(
            small(),
            StreamingBudget {
                in_flight: StageCounts {
                    generate: 1,
                    ..UNLIMITED.in_flight
                },
                ..UNLIMITED
            },
        );
        slow.app.init_resource::<stats::StreamingStats>();
        slow.tick();
        assert!(
            !settled(&mut slow),
            "settled with the load volume barely started"
        );
    }

    #[cfg(any(feature = "dev", feature = "profiling"))]
    #[test]
    fn stats_count_what_each_level_holds_and_has_meshed() {
        let mut h = Harness::new(
            StreamingSettings {
                lod_distance_m: 128.0,
                ..small()
            },
            UNLIMITED,
        );
        h.app.init_resource::<stats::StreamingStats>();
        h.settle();
        h.lod_settle();
        h.app
            .world_mut()
            .run_system_once(stats::publish_stats)
            .expect("the summary ran");

        let stats = h.app.world().resource::<stats::StreamingStats>();
        let lod_store = h.app.world().resource::<lod::LodStore>();
        let lod_state = h.app.world().resource::<lod::LodState>();
        assert_eq!(stats.levels.len(), 3);
        assert_eq!(
            (stats.levels[0].held, stats.levels[0].meshed),
            (stats.loaded, stats.meshed)
        );
        for level in 1..3 {
            let l = stats.levels[level as usize];
            assert_eq!(l.held, lod_store.coords(level).count());
            assert_eq!(l.meshed, lod_state.delivered(level).len());
            assert!(l.meshed > 0 && l.held > l.meshed, "level {level}: {l:?}");
        }
    }

    mod progress;

    #[test]
    fn face_neighbours_sit_on_the_faces_the_mesher_expects() {
        let n = face_neighbors(ChunkCoord::new(0, 0, 0));
        assert_eq!(n[NEG_X], ChunkCoord::new(-1, 0, 0));
        assert_eq!(n[POS_X], ChunkCoord::new(1, 0, 0));
        assert_eq!(n[NEG_Y], ChunkCoord::new(0, -1, 0));
        assert_eq!(n[POS_Y], ChunkCoord::new(0, 1, 0));
        assert_eq!(n[NEG_Z], ChunkCoord::new(0, 0, -1));
        assert_eq!(n[POS_Z], ChunkCoord::new(0, 0, 1));
    }
}
