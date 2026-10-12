//! Chunk streaming: which Chunks exist, and which the Client has a mesh for.
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

use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use tesserae_core::mesh::{NEG_X, NEG_Y, NEG_Z, POS_X, POS_Y, POS_Z};
use tesserae_core::{CHUNK_SIZE, ChunkMesh, Tables};

use super::jobs::{Border, Jobs, Shared, build_mesh, solve};
use super::world_state::ChunkStore;
use super::{ContentTables, WorldGenerator};
use crate::world::{ChunkCoord, WorldSettings};

/// The entity the World streams around. The Client puts this on its camera and
/// the Server reads that entity's `Transform`.
///
/// Intent, not state: the camera is a Client entity and its pose is Client
/// data. The component is declared here because the Server is the side that
/// decides what intent it accepts.
#[derive(Component)]
pub struct StreamingAnchor;

/// A Chunk's finished geometry, for the Client to upload.
///
/// Carries a `tesserae-core` `ChunkMesh` with no Bevy in it. Sent even when
/// empty, so the Client has one rule: empty means despawn.
#[derive(Message)]
pub struct ChunkMeshReady {
    pub coord: ChunkCoord,
    pub mesh: ChunkMesh,
}

/// The Client should stop drawing this Chunk.
#[derive(Message)]
pub struct ChunkUnloaded {
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
}

impl Default for StreamingSettings {
    fn default() -> Self {
        StreamingSettings {
            view_distance_m: 192.0,
            vertical_distance_m: 96.0,
            hysteresis_m: 32.0,
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

#[cfg(feature = "dev")]
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

fn streaming_budget() -> StreamingBudget {
    let budget = StreamingBudget::default();
    #[cfg(feature = "dev")]
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
        // Rounded up, so the configured distance is always reached rather than
        // approached, and at least one Cell so a view distance below one Cell
        // still renders the Chunk the anchor is standing in.
        let cells = |metres: f32| ((metres / cell_size_m).ceil() as i32).max(1);
        let hysteresis = cells(settings.hysteresis_m);
        let mesh = Extent {
            horizontal: cells(settings.view_distance_m),
            vertical: cells(settings.vertical_distance_m),
        };
        let keep = mesh.grown(hysteresis);
        let load = keep.grown(1);
        Rings {
            mesh,
            keep,
            load,
            evict: load.grown(hysteresis),
        }
    }
}

/// The load volume's offsets, sorted by distance, rebuilt only when the extent
/// changes. Walking this from the anchor gives "nearest missing Chunk" without
/// sorting a few thousand coordinates every tick.
#[derive(Resource, Default)]
struct LoadOrder {
    built_for: Option<Extent>,
    offsets: Vec<IVec3>,
    /// The same offsets, highest first. Sky light flows downwards, so solving a
    /// Chunk before the one above it means solving it again when the one above
    /// arrives; nearest-first is right for generation and wrong here.
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
}

impl MeshState {
    /// Whether this Chunk's light is still waiting on a solve. A Server system
    /// that wants to edit a Voxel waits for this: an edit re-floods light
    /// incrementally, and `relight`'s from-scratch solve landing afterwards
    /// would discard the work even though it would reach the same answer.
    pub(in crate::server) fn is_unlit(&self, coord: ChunkCoord) -> bool {
        self.unlit.contains(&coord) || self.relighting.contains(&coord)
    }

    /// A Voxel in this Chunk changed and its light has already been re-flooded,
    /// so the mesh that bakes both is stale.
    ///
    /// Deliberately not adding to `unlit`. The edit's own incremental pass is
    /// the sound one in both directions; `relight`'s bulk solve converges
    /// upwards only, so routing an edit through it is the mistake
    /// `tesserae_core::relight`'s doc comment describes.
    pub(in crate::server) fn edited(&mut self, coord: ChunkCoord) {
        self.dirty.insert(coord);
        // A solve reads its Chunk and its six face neighbours, so the Edit
        // invalidates the solve of any of the seven.
        for c in std::iter::once(coord).chain(face_neighbors(coord)) {
            if self.relighting.contains(&c) {
                self.poisoned.insert(c);
            }
        }
    }

    /// Whether a Chunk inside `load` is waiting on a light solve. Outside it
    /// `relight` never looks, so a Chunk the anchor has drifted away from can
    /// stay in `unlit` for good without anything being left to do.
    #[cfg(any(test, feature = "dev"))]
    fn unlit_within(&self, center: ChunkCoord, load: Extent) -> bool {
        self.unlit.iter().any(|&c| load.contains(delta(center, c)))
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
    app.init_resource::<StreamingSettings>()
        .insert_resource(streaming_budget())
        .init_resource::<LoadOrder>()
        .init_resource::<MeshState>()
        .init_resource::<Jobs>()
        .add_message::<ChunkMeshReady>()
        .add_message::<ChunkUnloaded>()
        .add_systems(FixedUpdate, stream);
    #[cfg(feature = "dev")]
    app.add_systems(
        FixedUpdate,
        (
            crate::cost::tick_started.before(stream),
            crate::cost::tick_ended.after(stream),
        ),
    );
    #[cfg(feature = "dev")]
    app.add_plugins(stats::plugin);
}

/// The six face neighbours, in the mesher's face order.
fn face_neighbors(coord: ChunkCoord) -> [ChunkCoord; 6] {
    let mut n = [coord; 6];
    n[NEG_X] = ChunkCoord::new(coord.x - 1, coord.y, coord.z);
    n[POS_X] = ChunkCoord::new(coord.x + 1, coord.y, coord.z);
    n[NEG_Y] = ChunkCoord::new(coord.x, coord.y - 1, coord.z);
    n[POS_Y] = ChunkCoord::new(coord.x, coord.y + 1, coord.z);
    n[NEG_Z] = ChunkCoord::new(coord.x, coord.y, coord.z - 1);
    n[POS_Z] = ChunkCoord::new(coord.x, coord.y, coord.z + 1);
    n
}

fn offset(center: ChunkCoord, delta: IVec3) -> ChunkCoord {
    ChunkCoord::new(center.x + delta.x, center.y + delta.y, center.z + delta.z)
}

fn delta(from: ChunkCoord, to: ChunkCoord) -> IVec3 {
    IVec3::new(to.x - from.x, to.y - from.y, to.z - from.z)
}

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
    anchor: Query<&Transform, With<StreamingAnchor>>,
) {
    // No anchor, or more than one, is not an error: the Client has not spawned
    // its camera yet on the first tick. Streaming around an arbitrary one of
    // several would be worse than streaming around none.
    let Ok(pose) = anchor.single() else {
        return;
    };

    let rings = Rings::new(*settings, world.cell_size());
    let center = world.chunk_at(pose.translation);
    if order.built_for != Some(rings.load) {
        order.offsets = rings.load.offsets();
        order.descending = order.offsets.clone();
        order.descending.sort_by_key(|d| (-d.y, d.length_squared()));
        order.built_for = Some(rings.load);
    }

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
    );
    land_relight(
        &mut store,
        &mut meshes,
        &mut jobs,
        center,
        budget.landed_per_tick.relight,
    );
    land_meshes(
        &mut meshes,
        &mut jobs,
        &mut ready,
        center,
        rings.mesh,
        budget.landed_per_tick.mesh,
    );
    dispatch_generation(
        &store,
        &mut jobs,
        &generator,
        &order,
        center,
        budget.in_flight.generate,
    );
    dispatch_relight(
        &store,
        &mut meshes,
        &mut jobs,
        &generator,
        &order,
        &world,
        center,
        budget.in_flight.relight,
        &tables.0,
    );
    dispatch_meshes(
        &store,
        &mut meshes,
        &mut jobs,
        &order,
        center,
        rings.mesh,
        budget.in_flight.mesh,
        budget.wait_for_settled_light,
        &tables.0,
    );
}

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

    let stale: Vec<ChunkCoord> = store
        .coords()
        .filter(|&c| !evict.contains(delta(center, c)))
        .collect();
    for coord in stale {
        store.remove(coord);
        meshes.dirty.remove(&coord);
        meshes.unlit.remove(&coord);
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
) {
    let mut finished: Vec<ChunkCoord> = jobs.generate.finished().collect();
    finished.sort_unstable_by_key(|&c| delta(center, c).length_squared());
    for coord in finished.into_iter().take(budget) {
        let Some(chunk) = jobs.generate.take(coord) else {
            continue;
        };
        if store.contains(coord) || !evict.contains(delta(center, coord)) {
            continue;
        }
        store.insert(coord, chunk);
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

/// Starts generation jobs for the nearest missing Chunks in the load extent,
/// until `in_flight` are outstanding.
fn dispatch_generation(
    store: &ChunkStore,
    jobs: &mut Jobs,
    generator: &WorldGenerator,
    order: &LoadOrder,
    center: ChunkCoord,
    in_flight: usize,
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
        jobs.generate
            .spawn(coord, move || worldgen.chunk(coord.x, coord.y, coord.z));
        free -= 1;
    }
}

/// Applies up to `budget` finished light solves, highest first.
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
    center: ChunkCoord,
    budget: usize,
) {
    // Same order as dispatch: a landing that darkens nothing above it first
    // would be solved again when the one above lands.
    let mut finished: Vec<ChunkCoord> = jobs.relight.finished().collect();
    finished.sort_unstable_by_key(|&c| {
        let d = delta(center, c);
        (-d.y, d.length_squared())
    });
    for coord in finished.into_iter().take(budget) {
        let Some(solved) = jobs.relight.take(coord) else {
            continue;
        };
        meshes.relighting.remove(&coord);
        let poisoned = meshes.poisoned.remove(&coord);
        let Some((current, _)) = store.shared(coord) else {
            continue;
        };
        if poisoned || !Arc::ptr_eq(&current, &solved.chunk) {
            jobs.relight.note_stale();
            meshes.unlit.insert(coord);
            continue;
        }
        if !solved.changed {
            continue;
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
}

/// Starts light solves for Chunks whose light is out of date, highest first,
/// until `in_flight` are outstanding.
///
/// Highest first because sky light flows downwards: solving a Chunk before the
/// one above it means solving it again when the one above lands.
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
) {
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
        let Some((chunk, before)) = store.shared(coord) else {
            meshes.unlit.remove(&coord);
            continue;
        };
        let border: [Border; 6] = face_neighbors(coord).map(|n| match store.shared(n) {
            Some((chunk, light)) => Border::Loaded(chunk, light),
            None if n.y > coord.y && above_the_surface(generator, world, n) => Border::OpenSky,
            None => Border::Absent,
        });
        meshes.unlit.remove(&coord);
        meshes.relighting.insert(coord);
        meshes.poisoned.remove(&coord);
        let tables = Arc::clone(tables);
        jobs.relight
            .spawn(coord, move || solve(chunk, &before, &border, &tables));
        free -= 1;
    }
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

/// Applies up to `budget` finished meshes, nearest first, and tells the Client.
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
) {
    let mut finished: Vec<ChunkCoord> = jobs.mesh.finished().collect();
    finished.sort_unstable_by_key(|&c| delta(center, c).length_squared());
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
        ready.write(ChunkMeshReady { coord, mesh: built });
        meshes.delivered.insert(coord);
    }
}

/// Starts mesh jobs for dirty Chunks inside the mesh extent, nearest first,
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
    mesh: Extent,
    in_flight: usize,
    wait_for_settled_light: bool,
    tables: &Arc<Tables>,
) {
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
        let around: [Option<Shared>; 6] = faces.map(|n| store.shared(n));
        let Some(around) = around.into_iter().collect::<Option<Vec<_>>>() else {
            continue;
        };
        let settled = |c: &ChunkCoord| !meshes.is_unlit(*c);
        if wait_for_settled_light && !(settled(&coord) && faces.iter().all(settled)) {
            continue;
        }
        let around: [Shared; 6] = around
            .try_into()
            .unwrap_or_else(|_| unreachable!("six neighbours were collected"));
        meshes.dirty.remove(&coord);
        let tables = Arc::clone(tables);
        jobs.mesh
            .spawn(coord, move || build_mesh(&own, &around, &tables));
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
#[cfg(feature = "dev")]
pub mod stats {
    use super::*;

    #[derive(Resource, Default, Clone, Copy, Debug)]
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
    }

    pub(super) fn plugin(app: &mut App) {
        app.init_resource::<StreamingStats>()
            .add_systems(FixedPostUpdate, publish_stats);
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
        anchor: Query<&Transform, With<StreamingAnchor>>,
    ) {
        let center = anchor
            .single()
            .map(|pose| world.chunk_at(pose.translation))
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
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::edit;
    use crate::server::world_state::VoxelCoord;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::tasks::{AsyncComputeTaskPool, TaskPoolBuilder};
    use tesserae_core::light::{Light, MAX_LIGHT, light_chunk};
    use tesserae_core::{Chunk, ChunkLight, LitChunk, ShapeTable, mesh_chunk};
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
    fn small() -> StreamingSettings {
        StreamingSettings {
            view_distance_m: 32.0,
            vertical_distance_m: 32.0,
            hysteresis_m: 32.0,
        }
    }

    const UNLIMITED: StreamingBudget = StreamingBudget {
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
    const SETTLE_CEILING: std::time::Duration = std::time::Duration::from_secs(60);

    struct Harness {
        app: App,
        anchor: Entity,
    }

    impl Harness {
        fn new(settings: StreamingSettings, budget: StreamingBudget) -> Self {
            // One pool for the whole test process; `get_or_init` is a no-op after
            // the first test, and the thread count is fixed so nothing a test
            // asserts depends on the machine.
            AsyncComputeTaskPool::get_or_init(|| TaskPoolBuilder::new().num_threads(4).build());
            let world_settings = WorldSettings::default();
            let generator = Worldgen::new(
                world_settings.seed,
                world_settings.voxel_size,
                WorldgenParams::default(),
            );
            let mut app = App::new();
            app.insert_resource(world_settings)
                .insert_resource(settings)
                .insert_resource(budget)
                .insert_resource(WorldGenerator(generator))
                .init_resource::<ContentTables>()
                .init_resource::<ChunkStore>()
                .init_resource::<LoadOrder>()
                .init_resource::<MeshState>()
                .init_resource::<Jobs>()
                .add_message::<ChunkMeshReady>()
                .add_message::<ChunkUnloaded>();
            let anchor = app
                .world_mut()
                .spawn((Transform::default(), StreamingAnchor))
                .id();
            Harness { app, anchor }
        }

        fn anchor_at(&mut self, metres: Vec3) {
            self.app
                .world_mut()
                .entity_mut(self.anchor)
                .insert(Transform::from_translation(metres));
        }

        fn tick(&mut self) -> (Vec<ChunkMeshReady>, Vec<ChunkCoord>) {
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
        fn settle(&mut self) -> (Vec<ChunkMeshReady>, Vec<ChunkCoord>) {
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
            let pose = self.app.world().get::<Transform>(self.anchor);
            let world = self.app.world().resource::<WorldSettings>();
            world.chunk_at(pose.expect("the anchor exists").translation)
        }

        fn jobs(&self) -> &Jobs {
            self.app.world().resource::<Jobs>()
        }

        fn jobs_mut(&mut self) -> Mut<'_, Jobs> {
            self.app.world_mut().resource_mut::<Jobs>()
        }

        fn store(&self) -> &ChunkStore {
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

    fn same_mesh(a: &ChunkMesh, b: &ChunkMesh) -> bool {
        a.positions == b.positions
            && a.normals == b.normals
            && a.uvs == b.uvs
            && a.materials == b.materials
            && a.light == b.light
            && a.indices == b.indices
    }

    /// The light of the Chunk a mesh was built from, which `publish` requires
    /// to be present before it meshes at all.
    fn lit<'a>(store: &'a ChunkStore, coord: ChunkCoord) -> LitChunk<'a> {
        LitChunk::new(
            store.get(coord).expect("loaded"),
            store.light(coord).expect("lit"),
        )
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
            let fresh = mesh_chunk(lit(store, *coord), neighbors, &ShapeTable::builtin());
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
            let fresh = mesh_chunk(lit(store, coord), neighbors, &ShapeTable::builtin());
            assert!(same_mesh(&republished.mesh, &fresh), "{coord} is stale");
        }
        assert_eq!(ready.len(), 2, "nothing else was touched");
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
    fn a_tick_with_no_anchor_does_nothing() {
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
            WorldgenParams::default(),
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
            WorldgenParams::default(),
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
            lit_faces += message.mesh.light.iter().filter(|&&l| l != 0).count();
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
        let tables = Tables::builtin();
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
                        Voxel::cube(tesserae_core::worldgen::LANTERN),
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
                Voxel::cube(tesserae_core::worldgen::STONE)
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
                let fresh = mesh_chunk(lit(store, *coord), neighbors, &ShapeTable::builtin());
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

    #[cfg(feature = "dev")]
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
