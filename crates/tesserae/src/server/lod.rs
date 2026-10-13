//! Coarse LOD levels: Chunks above level 0, derived and never edited.
//!
//! A level-k Chunk is the [`downsample`] of eight level-(k-1) Chunks. Those are
//! generated for the purpose by the level-(k-1) source generator, not read from
//! a store, because the Server holds level 0 only within `evict` of the Eye and
//! the children of a distant coarse Chunk do not exist. The one exception is
//! level 1, which takes the level-0 Chunks the [`ChunkStore`] holds instead of
//! generating them, so an Edit reaches it.
//!
//! A coarse Chunk is meshed by the level-0 mesher, lit as open sky because no
//! light is stored above level 0, and delivered as `LodMeshReady`. The shell it
//! is meshed in has a hole where the next finer level draws.
//!
//! The pipeline is ADR-0007's: a job holds a generator and `Arc`s and returns an
//! owned `Chunk`, and the result is validated where it lands. The store and the
//! bookkeeping here are Server-private for the reason `ChunkStore` is, and the
//! Client never sees either.
//!
//! See `docs/adr/0002-client-server-from-day-one.md` and
//! `docs/adr/0007-streaming-jobs-run-on-bevy-task-pools.md`.

use std::sync::Arc;

use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
use tesserae_core::light::{Light, MAX_LIGHT};
use tesserae_core::{
    Chunk, ChunkLight, ChunkMesh, LitChunk, ShapeTable, Tables, Worldgen, WorldgenParams,
    downsample, mesh_chunk,
};

use super::jobs::Stage;
use super::lod_rings::{LevelRings, Shell, level_count};
use super::player::Player;
use super::streaming::{
    LodMeshReady, LodUnloaded, MeshState, StreamingSettings, delta, face_neighbors, offset,
};
use super::world_state::ChunkStore;
use super::{ContentTables, WorldGenerator};
use crate::world::{ChunkCoord, WorldSettings};

/// A coarse Chunk is named by its level and its coordinate in that level's
/// lattice.
pub(in crate::server) type LodKey = (u32, ChunkCoord);

/// How many coarse jobs of each stage.
#[derive(Clone, Copy, Debug)]
pub(in crate::server) struct LodCounts {
    pub generate: usize,
    pub mesh: usize,
}

/// How much coarse work is allowed at once, shared by every level and separate
/// from level 0's so a burst of it cannot occupy the slots level 0 needs.
///
/// Placeholders, for ticket 07 to replace with measurements. A bad number here
/// is fill latency and never frame time, as for `StreamingBudget`.
#[derive(Resource, Clone, Copy, Debug)]
pub(in crate::server) struct LodBudget {
    pub in_flight: LodCounts,
    pub landed_per_tick: LodCounts,
}

impl Default for LodBudget {
    fn default() -> Self {
        LodBudget {
            in_flight: LodCounts {
                generate: 16,
                mesh: 32,
            },
            landed_per_tick: LodCounts {
                generate: 16,
                mesh: 16,
            },
        }
    }
}

/// The generator of each source level, for the jobs that downsample it.
///
/// `sources[j]` makes level-j Chunks, so a level-k job clones `sources[k - 1]`.
/// There is one fewer source than there are levels: the outermost level is
/// never anyone's child.
#[derive(Resource)]
pub(in crate::server) struct LodGenerators {
    sources: Vec<Worldgen>,
}

impl LodGenerators {
    /// Level j is generated at `voxel_size * 2^j`; everything in `Worldgen` is
    /// tuned in metres, so no other parameter changes.
    pub fn new(world: &WorldSettings, level_zero: &Worldgen, levels: u32) -> Self {
        // The smoothing gate is by metres, so a 2 m Voxel passes it and would
        // place 1 m stairs that the next downsample rounds back to cubes.
        let coarse = WorldgenParams {
            smoothing: None,
            ..level_zero.params()
        };
        let sources = (0..levels.saturating_sub(1))
            .map(|j| {
                if j == 0 {
                    level_zero.clone()
                } else {
                    Worldgen::new(world.seed, world.voxel_size * 2f32.powi(j as i32), coarse)
                }
            })
            .collect();
        LodGenerators { sources }
    }

    /// How many levels there are, level 0 included.
    pub fn levels(&self) -> u32 {
        self.sources.len() as u32 + 1
    }
}

/// Every coarse Chunk the Server currently holds, per level.
///
/// Derived data: nothing writes it but landing, and no Edit ever does.
#[derive(Resource, Default)]
pub(in crate::server) struct LodStore {
    /// Index `level - 1`; grown on first write so the store needs no level
    /// count of its own.
    levels: Vec<HashMap<ChunkCoord, Arc<Chunk>>>,
}

impl LodStore {
    fn at(&self, level: u32) -> Option<&HashMap<ChunkCoord, Arc<Chunk>>> {
        debug_assert!(level >= 1, "level 0 lives in the ChunkStore");
        self.levels.get(level as usize - 1)
    }

    fn at_mut(&mut self, level: u32) -> &mut HashMap<ChunkCoord, Arc<Chunk>> {
        debug_assert!(level >= 1, "level 0 lives in the ChunkStore");
        let index = level as usize - 1;
        if self.levels.len() <= index {
            self.levels.resize_with(index + 1, HashMap::default);
        }
        &mut self.levels[index]
    }

    pub fn contains(&self, level: u32, coord: ChunkCoord) -> bool {
        self.at(level).is_some_and(|l| l.contains_key(&coord))
    }

    #[cfg(test)]
    pub fn get(&self, level: u32, coord: ChunkCoord) -> Option<&Chunk> {
        self.at(level)?.get(&coord).map(Arc::as_ref)
    }

    /// A Chunk as a shared handle, for a job to hold.
    pub fn shared(&self, level: u32, coord: ChunkCoord) -> Option<Arc<Chunk>> {
        self.at(level)?.get(&coord).map(Arc::clone)
    }

    #[cfg(test)]
    pub fn coords(&self, level: u32) -> impl Iterator<Item = ChunkCoord> + '_ {
        self.at(level).into_iter().flat_map(|l| l.keys().copied())
    }

    #[cfg(feature = "dev")]
    pub fn len(&self, level: u32) -> usize {
        self.at(level).map_or(0, HashMap::len)
    }

    pub(in crate::server) fn insert(&mut self, level: u32, coord: ChunkCoord, chunk: Chunk) {
        self.at_mut(level).insert(coord, Arc::new(chunk));
    }

    /// Drops every Chunk of the level for which `keep` is false.
    pub(in crate::server) fn retain(
        &mut self,
        level: u32,
        mut keep: impl FnMut(ChunkCoord) -> bool,
    ) {
        self.at_mut(level).retain(|&c, _| keep(c));
    }
}

/// What the tick keeps about one coarse level.
#[derive(Default)]
struct Level {
    /// The anchor the level was last retired and scanned for.
    anchor: Option<ChunkCoord>,
    /// Whether a scan could find work. Set by an anchor move, a landing, an
    /// invalidation, and by a scan that ran out of budget before it finished.
    pending: bool,
    /// Whether a scan of the mesh shell could find work: a Chunk or a neighbour
    /// of it landed, a mesh slot freed, or the anchor moved.
    mesh_pending: bool,
    /// Held Chunks that need a mesh.
    dirty: HashSet<ChunkCoord>,
    /// Held Chunks that need regenerating.
    stale: HashSet<ChunkCoord>,
    /// Held Chunks with a replacement running. Taken out of `stale` at
    /// dispatch, so a mark made during the flight is still there when the job
    /// lands; this is what tells landing a replacement from an intruder.
    regenerating: HashSet<ChunkCoord>,
    /// Chunks the Client holds a mesh for.
    delivered: HashSet<ChunkCoord>,
}

/// Which coarse Chunks need a mesh, regenerating, or are with the Client.
#[derive(Resource, Default)]
pub(in crate::server) struct LodState {
    /// Index `level - 1`.
    levels: Vec<Level>,
    /// Offsets by shell. Five of six levels share each at the defaults.
    offsets: Vec<(Shell, Arc<Vec<IVec3>>)>,
}

impl LodState {
    fn level_mut(&mut self, level: u32) -> &mut Level {
        let index = level as usize - 1;
        if self.levels.len() <= index {
            self.levels.resize_with(index + 1, Level::default);
        }
        &mut self.levels[index]
    }

    fn offsets_for(&mut self, shell: Shell) -> Arc<Vec<IVec3>> {
        if let Some((_, offsets)) = self.offsets.iter().find(|(s, _)| *s == shell) {
            return Arc::clone(offsets);
        }
        let offsets = Arc::new(shell.offsets());
        self.offsets.push((shell, Arc::clone(&offsets)));
        offsets
    }

    #[cfg(feature = "dev")]
    pub fn delivered_len(&self, level: u32) -> usize {
        self.levels
            .get(level as usize - 1)
            .map_or(0, |l| l.delivered.len())
    }

    /// A held Chunk's inputs changed, so it is regenerated.
    pub(in crate::server) fn invalidate(&mut self, level: u32, coord: ChunkCoord) {
        let level = self.level_mut(level);
        level.stale.insert(coord);
        level.pending = true;
    }

    #[cfg(test)]
    fn dirty(&self, level: u32) -> HashSet<ChunkCoord> {
        self.levels
            .get(level as usize - 1)
            .map_or_else(HashSet::default, |l| l.dirty.clone())
    }

    #[cfg(test)]
    fn clear_dirty(&mut self) {
        self.levels.iter_mut().for_each(|l| l.dirty.clear());
    }

    #[cfg(test)]
    fn any_pending(&self) -> bool {
        self.levels.iter().any(|l| l.pending || l.mesh_pending)
    }

    #[cfg(test)]
    pub(in crate::server) fn delivered(&self, level: u32) -> HashSet<ChunkCoord> {
        self.levels
            .get(level as usize - 1)
            .map_or_else(HashSet::default, |l| l.delivered.clone())
    }

    #[cfg(test)]
    fn is_stale(&self, level: u32, coord: ChunkCoord) -> bool {
        self.levels
            .get(level as usize - 1)
            .is_some_and(|l| l.stale.contains(&coord))
    }
}

/// The coarse jobs in flight, keyed by level and coord.
#[derive(Resource, Default)]
pub(in crate::server) struct LodJobs {
    pub generate: Stage<Chunk, LodKey>,
    pub mesh: Stage<ChunkMesh, LodKey>,
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<LodStore>()
        .init_resource::<LodState>()
        .init_resource::<LodJobs>()
        .init_resource::<LodBudget>()
        // After `open_world`, whose generator the sources are derived from.
        .add_systems(Startup, open_lod_generators.after(super::open_world));
}

fn open_lod_generators(
    mut commands: Commands,
    world: Res<WorldSettings>,
    settings: Res<StreamingSettings>,
    generator: Res<WorldGenerator>,
) {
    commands.insert_resource(LodGenerators::new(
        &world,
        &generator,
        level_count(&settings),
    ));
}

/// Child `i` of a coarse Chunk, in the lattice one level finer: the octant
/// `x + 2y + 4z`.
fn child(coord: ChunkCoord, i: usize) -> ChunkCoord {
    ChunkCoord::new(
        2 * coord.x + (i & 1) as i32,
        2 * coord.y + (i >> 1 & 1) as i32,
        2 * coord.z + (i >> 2) as i32,
    )
}

/// The level-1 Chunk that covers a level-0 one: the inverse of [`child`].
fn cover(coord: ChunkCoord) -> ChunkCoord {
    ChunkCoord::new(
        coord.x.div_euclid(2),
        coord.y.div_euclid(2),
        coord.z.div_euclid(2),
    )
}

/// A coarse Chunk from the level below. A `held` child is used as it is and the
/// others are generated, which is the same Chunk for any child nothing has
/// edited. That is the property that lets a coarse Chunk be derived data.
pub(in crate::server) fn generate_coarse(
    source: &Worldgen,
    coord: ChunkCoord,
    held: &[Option<Arc<Chunk>>; 8],
    shapes: &ShapeTable,
) -> Chunk {
    let made: [Option<Chunk>; 8] = std::array::from_fn(|i| {
        held[i].is_none().then(|| {
            let c = child(coord, i);
            source.chunk(c.x, c.y, c.z)
        })
    });
    let children: [&Chunk; 8] = std::array::from_fn(|i| {
        held[i]
            .as_deref()
            .or(made[i].as_ref())
            .expect("each child is held or was just generated")
    });
    downsample(children, shapes)
}

/// All the light a coarse Chunk has. Full sky matches the level-0 faces beside
/// it and merges maximally; the cost is a cave mouth lit like open ground.
static OPEN_SKY: ChunkLight = ChunkLight::filled(Light::new(MAX_LIGHT, 0));

/// Meshes a coarse Chunk from itself and its six same-level neighbours.
fn mesh_coarse(center: &Chunk, around: &[Arc<Chunk>; 6], tables: &Tables) -> ChunkMesh {
    let neighbors: [Option<LitChunk>; 6] =
        std::array::from_fn(|face| Some(LitChunk::new(&around[face], &OPEN_SKY)));
    mesh_chunk(LitChunk::new(center, &OPEN_SKY), neighbors, tables)
}

/// Where one level stands this tick.
struct Plan {
    level: u32,
    anchor: ChunkCoord,
    rings: LevelRings,
}

impl Plan {
    fn index(level: u32) -> usize {
        level as usize - 1
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "one tick, as `stream`; a SystemParam bundle would be a type used once"
)]
pub(in crate::server) fn stream_lod(
    chunks: Res<ChunkStore>,
    mut meshes: ResMut<MeshState>,
    mut store: ResMut<LodStore>,
    mut state: ResMut<LodState>,
    mut jobs: ResMut<LodJobs>,
    mut ready: MessageWriter<LodMeshReady>,
    mut unloaded: MessageWriter<LodUnloaded>,
    generators: Res<LodGenerators>,
    tables: Res<ContentTables>,
    world: Res<WorldSettings>,
    settings: Res<StreamingSettings>,
    budget: Res<LodBudget>,
    player: Query<&Player>,
) {
    let levels = generators.levels();
    if levels <= 1 {
        // Nobody will read the marks, and they would pile up.
        meshes.take_cover_changes();
        return;
    }
    let Ok(player) = player.single() else {
        return;
    };
    let changed = meshes.take_cover_changes();

    jobs.generate.reset_counts();
    jobs.generate.poll();
    jobs.mesh.reset_counts();
    jobs.mesh.poll();

    // Level 1 only: an Edit is within EDIT_REACH_M of the Eye, level 2 starts at
    // 256 m and level 0 is evicted at 288 m, so no higher level can hold one.
    for coord in changed {
        let covering = cover(coord);
        // A job in flight read its children before the change and lands stale.
        if store.contains(1, covering) || jobs.generate.contains((1, covering)) {
            state.invalidate(1, covering);
        }
    }

    let plans: Vec<Plan> = (1..levels)
        .map(|level| Plan {
            level,
            anchor: world.chunk_at_level(player.eye(), level),
            rings: LevelRings::new(*settings, world.cell_size(), level),
        })
        .collect();

    retire(&plans, &mut store, &mut state, &mut jobs, &mut unloaded);
    land(&plans, &mut store, &mut state, &mut jobs, &budget);
    land_meshes(&plans, &mut state, &mut jobs, &mut ready, &budget);
    dispatch(
        &plans,
        &chunks,
        &store,
        &mut state,
        &mut jobs,
        &generators,
        &tables,
        &budget,
    );
    dispatch_meshes(&plans, &store, &mut state, &mut jobs, &tables.0, &budget);
}

/// Releases the meshes that left `keep` and drops what left `evict`, for each
/// level whose anchor Chunk moved. Nothing else can move a Chunk out of either,
/// so a level whose anchor held still is not looked at.
fn retire(
    plans: &[Plan],
    store: &mut LodStore,
    state: &mut LodState,
    jobs: &mut LodJobs,
    unloaded: &mut MessageWriter<LodUnloaded>,
) {
    let mut moved = false;
    for plan in plans {
        let level = state.level_mut(plan.level);
        if level.anchor == Some(plan.anchor) {
            continue;
        }
        moved = true;
        level.anchor = Some(plan.anchor);
        level.pending = true;
        level.mesh_pending = true;

        // Before `delivered` is pruned by `evict`: a Chunk the Client holds a
        // mesh for must be unloaded, not forgotten.
        let released: Vec<ChunkCoord> = level
            .delivered
            .iter()
            .copied()
            .filter(|&c| !plan.rings.keep.contains(delta(plan.anchor, c)))
            .collect();
        for coord in released {
            level.delivered.remove(&coord);
            unloaded.write(LodUnloaded {
                level: plan.level,
                coord,
            });
            // Still held, and nothing else will mesh it again if the Eye comes
            // back.
            level.dirty.insert(coord);
        }

        let inside = |c: ChunkCoord| plan.rings.evict.contains(delta(plan.anchor, c));
        level.dirty.retain(|&c| inside(c));
        level.stale.retain(|&c| inside(c));
        level.regenerating.retain(|&c| inside(c));
        // Everything left in `delivered` is inside `keep`, so inside `evict`.
        store.retain(plan.level, inside);
    }
    if moved {
        jobs.generate.retain(|(level, coord)| {
            plans
                .get(Plan::index(level))
                .is_some_and(|p| p.rings.evict.contains(delta(p.anchor, coord)))
        });
        let kept = |(level, coord): LodKey| {
            let p = &plans[Plan::index(level)];
            p.rings.keep.contains(delta(p.anchor, coord))
        };
        let cancelled: Vec<LodKey> = jobs.mesh.keys().into_iter().filter(|&k| !kept(k)).collect();
        jobs.mesh.retain(kept);
        // Dispatch took these out of `dirty`, and the Chunk is still held.
        for (level, coord) in cancelled {
            let p = &plans[Plan::index(level)];
            if p.rings.evict.contains(delta(p.anchor, coord)) {
                state.level_mut(level).dirty.insert(coord);
            }
        }
    }
}

/// Applies up to the budget of finished jobs, nearest level and then nearest
/// Chunk first.
///
/// A result lands if its Chunk is inside `evict` of its level's current anchor
/// and is either absent or the replacement `dispatch` asked for. Anything else
/// is dropped: the anchor has moved on, or the slot was filled by other means.
fn land(
    plans: &[Plan],
    store: &mut LodStore,
    state: &mut LodState,
    jobs: &mut LodJobs,
    budget: &LodBudget,
) {
    let anchor_of = |level: u32| plans[Plan::index(level)].anchor;
    let mut finished: Vec<LodKey> = jobs.generate.finished().collect();
    finished
        .sort_unstable_by_key(|&(level, c)| (level, delta(anchor_of(level), c).length_squared()));
    for key in finished.into_iter().take(budget.landed_per_tick.generate) {
        let Some(chunk) = jobs.generate.take(key) else {
            continue;
        };
        let (level, coord) = key;
        let plan = &plans[Plan::index(level)];
        let held = store.contains(level, coord);
        let lv = state.level_mut(level);
        // A slot has freed whether or not the result is used.
        lv.pending = true;
        // Chunks and their neighbours are what a mesh waits for.
        lv.mesh_pending = true;
        let replacing = lv.regenerating.remove(&coord);
        if !plan.rings.evict.contains(delta(plan.anchor, coord)) || (held && !replacing) {
            jobs.generate.note_stale();
            continue;
        }
        store.insert(level, coord, chunk);
        lv.dirty.insert(coord);
        if held {
            // A first generation cannot change a mesh whose six neighbours are
            // required to exist; a replacement can.
            for neighbor in face_neighbors(coord) {
                if store.contains(level, neighbor) {
                    lv.dirty.insert(neighbor);
                }
            }
        }
    }
}

/// Delivers up to the budget of finished meshes, nearest level and then nearest
/// Chunk first.
///
/// A mesh lands only if its Chunk is still inside `keep`, which also keeps the
/// eviction argument true for a job in flight: a Chunk inside `keep` has all six
/// neighbours inside `load`, so every neighbour the job read is still held. As
/// at level 0 it is delivered even if the Chunk was dirtied again during the
/// flight - no staler than the mesh the Client holds, the mark is still there,
/// and dropping it would starve a Chunk that changes faster than a mesh builds.
fn land_meshes(
    plans: &[Plan],
    state: &mut LodState,
    jobs: &mut LodJobs,
    ready: &mut MessageWriter<LodMeshReady>,
    budget: &LodBudget,
) {
    let anchor_of = |level: u32| plans[Plan::index(level)].anchor;
    let mut finished: Vec<LodKey> = jobs.mesh.finished().collect();
    finished
        .sort_unstable_by_key(|&(level, c)| (level, delta(anchor_of(level), c).length_squared()));
    for key in finished.into_iter().take(budget.landed_per_tick.mesh) {
        let Some(mesh) = jobs.mesh.take(key) else {
            continue;
        };
        let (level, coord) = key;
        let plan = &plans[Plan::index(level)];
        let lv = state.level_mut(level);
        lv.mesh_pending = true;
        if !plan.rings.keep.contains(delta(plan.anchor, coord)) {
            jobs.mesh.note_stale();
            lv.dirty.insert(coord);
            continue;
        }
        if lv.dirty.contains(&coord) {
            jobs.mesh.note_stale();
        }
        ready.write(LodMeshReady { level, coord, mesh });
        lv.delivered.insert(coord);
    }
}

/// Starts generate jobs, nearest level first and nearest Chunk first within it,
/// until the budget is spent. A level is scanned only if it has `pending` work.
#[expect(
    clippy::too_many_arguments,
    reason = "one stage of the tick; bundling them would invent a type used once"
)]
fn dispatch(
    plans: &[Plan],
    chunks: &ChunkStore,
    store: &LodStore,
    state: &mut LodState,
    jobs: &mut LodJobs,
    generators: &LodGenerators,
    tables: &ContentTables,
    budget: &LodBudget,
) {
    let mut free = budget
        .in_flight
        .generate
        .saturating_sub(jobs.generate.in_flight());
    for plan in plans {
        // A level left `pending` by the budget runs out of it here, and the
        // ones after it are not scanned either.
        if free == 0 {
            break;
        }
        if !state.level_mut(plan.level).pending {
            continue;
        }
        // Generation has no hole: a mesh needs the Chunks under it too.
        let offsets = state.offsets_for(Shell {
            outer: plan.rings.load,
            hole: None,
        });
        let level = state.level_mut(plan.level);
        // Cleared first and set again if the budget runs out before the scan
        // ends, so the work it left is found next tick without a landing.
        level.pending = false;
        for &d in offsets.iter() {
            let coord = offset(plan.anchor, d);
            let held = store.contains(plan.level, coord);
            let stale = level.stale.contains(&coord);
            if (held && !stale) || jobs.generate.contains((plan.level, coord)) {
                continue;
            }
            if free == 0 {
                level.pending = true;
                break;
            }
            free -= 1;
            level.stale.remove(&coord);
            if held {
                level.regenerating.insert(coord);
            }
            let held_children: [Option<Arc<Chunk>>; 8] = if plan.level == 1 {
                std::array::from_fn(|i| chunks.chunk_arc(child(coord, i)))
            } else {
                Default::default()
            };
            let source = generators.sources[Plan::index(plan.level)].clone();
            let tables = Arc::clone(&tables.0);
            jobs.generate.spawn((plan.level, coord), move || {
                generate_coarse(&source, coord, &held_children, &tables.shapes)
            });
        }
    }
}

/// Starts mesh jobs for dirty Chunks in each level's mesh shell, nearest level
/// first and nearest Chunk first within it, until the budget is spent.
///
/// Requires all six neighbours held. There is no light to settle, so unlike
/// level 0 nothing else gates a mesh.
fn dispatch_meshes(
    plans: &[Plan],
    store: &LodStore,
    state: &mut LodState,
    jobs: &mut LodJobs,
    tables: &Arc<Tables>,
    budget: &LodBudget,
) {
    let mut free = budget.in_flight.mesh.saturating_sub(jobs.mesh.in_flight());
    for plan in plans {
        if free == 0 {
            break;
        }
        if !state.level_mut(plan.level).mesh_pending {
            continue;
        }
        let offsets = state.offsets_for(plan.rings.mesh);
        let level = state.level_mut(plan.level);
        level.mesh_pending = false;
        for &d in offsets.iter() {
            let coord = offset(plan.anchor, d);
            if !level.dirty.contains(&coord) || jobs.mesh.contains((plan.level, coord)) {
                continue;
            }
            let Some(own) = store.shared(plan.level, coord) else {
                continue;
            };
            let around: Option<Vec<Arc<Chunk>>> = face_neighbors(coord)
                .into_iter()
                .map(|n| store.shared(plan.level, n))
                .collect();
            let Some(around) = around else {
                continue;
            };
            let around: [Arc<Chunk>; 6] = around
                .try_into()
                .unwrap_or_else(|_| unreachable!("six neighbours were collected"));
            if free == 0 {
                level.mesh_pending = true;
                break;
            }
            free -= 1;
            level.dirty.remove(&coord);
            let tables = Arc::clone(tables);
            jobs.mesh.spawn((plan.level, coord), move || {
                mesh_coarse(&own, &around, &tables)
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::edit::set_voxel;
    use crate::server::streaming::tests::{Harness, SETTLE_CEILING, UNLIMITED, same_mesh, small};
    use bevy::ecs::system::RunSystemOnce;
    use tesserae_core::{CHUNK_SIZE, SHAPE_STAIR, Tables, Voxel, worldgen};

    /// Past this many ticks a fill is stuck, however much wall time is left: a
    /// livelock that never waits on a job would otherwise spin until the ceiling.
    const SETTLE_TICKS: usize = 200_000;

    /// Four levels, each one Cell of its own thick: mesh (1, 1), load (3, 3) and
    /// evict (4, 4) at every level, so a fill is about 200 Chunks a level.
    fn lod_settings() -> StreamingSettings {
        StreamingSettings {
            lod_distance_m: 256.0,
            ..small()
        }
    }

    const LEVELS: u32 = 4;

    fn unlimited() -> LodBudget {
        LodBudget {
            in_flight: LodCounts {
                generate: usize::MAX,
                mesh: usize::MAX,
            },
            landed_per_tick: LodCounts {
                generate: usize::MAX,
                mesh: usize::MAX,
            },
        }
    }

    fn harness_with(settings: StreamingSettings, budget: LodBudget) -> Harness {
        let mut h = Harness::new(settings, UNLIMITED);
        h.app.insert_resource(budget);
        h
    }

    fn harness() -> Harness {
        harness_with(lod_settings(), unlimited())
    }

    impl Harness {
        fn lod_tick(&mut self) -> (Vec<LodMeshReady>, Vec<LodUnloaded>) {
            self.app
                .world_mut()
                .run_system_once(stream_lod)
                .expect("the tick ran");
            let world = self.app.world_mut();
            let ready = world
                .resource_mut::<Messages<LodMeshReady>>()
                .drain()
                .collect();
            let unloaded = world
                .resource_mut::<Messages<LodUnloaded>>()
                .drain()
                .collect();
            (ready, unloaded)
        }

        fn lod_jobs(&self) -> &LodJobs {
            self.app.world().resource::<LodJobs>()
        }

        fn lod_store(&self) -> &LodStore {
            self.app.world().resource::<LodStore>()
        }

        fn lod_state(&self) -> &LodState {
            self.app.world().resource::<LodState>()
        }

        fn lod_in_flight(&self) -> usize {
            self.lod_jobs().generate.in_flight() + self.lod_jobs().mesh.in_flight()
        }

        fn lod_quiet(&self) -> bool {
            self.lod_in_flight() == 0
                && self.lod_jobs().generate.counts() == (0, 0)
                && self.lod_jobs().mesh.counts() == (0, 0)
                && !self.lod_state().any_pending()
        }

        /// Ticks until nothing is in flight, pending or landing, and returns
        /// every message written on the way.
        pub(in crate::server) fn lod_settle(&mut self) -> (Vec<LodMeshReady>, Vec<LodUnloaded>) {
            let (ready, unloaded, _) = self.lod_settle_counting();
            (ready, unloaded)
        }

        /// `lod_settle`, and how many generate jobs were dispatched on the way.
        fn lod_settle_counting(&mut self) -> (Vec<LodMeshReady>, Vec<LodUnloaded>, usize) {
            let (mut ready, mut unloaded, mut dispatched) = (Vec::new(), Vec::new(), 0);
            let started = std::time::Instant::now();
            for _ in 0..SETTLE_TICKS {
                if started.elapsed() > SETTLE_CEILING {
                    break;
                }
                let (r, u) = self.lod_tick();
                dispatched += self.lod_jobs().generate.counts().0;
                let quiet = r.is_empty() && u.is_empty() && self.lod_quiet();
                ready.extend(r);
                unloaded.extend(u);
                if quiet {
                    return (ready, unloaded, dispatched);
                }
                if self.lod_in_flight() > 0 {
                    std::thread::sleep(std::time::Duration::from_micros(100));
                }
            }
            panic!("coarse streaming was still busy after {SETTLE_TICKS} ticks");
        }

        fn anchor_of(&self, level: u32) -> ChunkCoord {
            let eye = self.app.world().get::<Player>(self.anchor).unwrap().eye();
            self.app
                .world()
                .resource::<WorldSettings>()
                .chunk_at_level(eye, level)
        }

        fn rings_of(&self, level: u32) -> LevelRings {
            let world = self.app.world().resource::<WorldSettings>();
            LevelRings::new(
                *self.app.world().resource::<StreamingSettings>(),
                world.cell_size(),
                level,
            )
        }

        /// Every Chunk `level` should hold when it has settled around the Eye.
        fn load_set(&self, level: u32) -> HashSet<ChunkCoord> {
            let anchor = self.anchor_of(level);
            self.rings_of(level)
                .load
                .offsets()
                .into_iter()
                .map(|d| offset(anchor, d))
                .collect()
        }

        fn held(&self, level: u32) -> HashSet<ChunkCoord> {
            self.lod_store().coords(level).collect()
        }

        /// Every Chunk of `level` the Eye's mesh shell covers.
        fn mesh_shell(&self, level: u32) -> HashSet<ChunkCoord> {
            let anchor = self.anchor_of(level);
            self.rings_of(level)
                .mesh
                .offsets()
                .into_iter()
                .map(|d| offset(anchor, d))
                .collect()
        }

        fn has_six_neighbours(&self, level: u32, coord: ChunkCoord) -> bool {
            face_neighbors(coord)
                .into_iter()
                .all(|n| self.lod_store().contains(level, n))
        }

        /// What `mesh_chunk` makes of the store's Chunks, lit the way decision 3
        /// says, built without going through the job under test.
        fn fresh_mesh(&self, level: u32, coord: ChunkCoord) -> ChunkMesh {
            let sky = ChunkLight::filled(Light::new(MAX_LIGHT, 0));
            let store = self.lod_store();
            let lit = |c: ChunkCoord| LitChunk::new(store.get(level, c).expect("held"), &sky);
            let neighbors = face_neighbors(coord).map(|n| Some(lit(n)));
            mesh_chunk(lit(coord), neighbors, &Tables::builtin())
        }

        fn delivered_at(&self, level: u32) -> HashSet<ChunkCoord> {
            self.lod_state().delivered(level)
        }

        /// An Edit, through the function a player's click ends in.
        fn edit(&mut self, at: [i32; 3], voxel: Voxel) {
            self.app
                .world_mut()
                .run_system_once(
                    move |mut store: ResMut<ChunkStore>,
                          mut meshes: ResMut<MeshState>,
                          tables: Res<ContentTables>| {
                        set_voxel(&mut store, &mut meshes, &tables, at, voxel).expect("loaded");
                    },
                )
                .unwrap();
        }

        /// Fills the 2x2x2 Voxels at `local` (even in every axis) with `voxel`:
        /// the footprint of one coarse Voxel, so the cover is sure to change.
        fn edit_block(&mut self, chunk: ChunkCoord, local: [i32; 3], voxel: Voxel) {
            let side = CHUNK_SIZE as i32;
            for (dx, dy, dz) in (0..8).map(|i| (i & 1, i >> 1 & 1, i >> 2)) {
                self.edit(
                    [
                        chunk.x * side + local[0] + dx,
                        chunk.y * side + local[1] + dy,
                        chunk.z * side + local[2] + dz,
                    ],
                    voxel,
                );
            }
        }

        /// A held, empty level-0 Chunk whose cover is meshed and which leaves
        /// `evict` when the Eye walks 160 m along +x, while its cover stays.
        fn quiet_chunk(&self) -> ChunkCoord {
            let shell = self.mesh_shell(1);
            let mut found: Vec<ChunkCoord> = self
                .store()
                .coords()
                .filter(|c| (-1..=0).contains(&c.x))
                .filter(|&c| self.store().get(c).unwrap().uniform() == Some(Voxel::AIR))
                .filter(|&c| shell.contains(&cover(c)))
                .collect();
            found.sort_by_key(|c| (c.y, c.z));
            *found.last().expect("an empty Chunk under the Eye")
        }

        fn cover_arc(&self, covering: ChunkCoord) -> Arc<Chunk> {
            self.lod_store().shared(1, covering).expect("cover held")
        }
    }

    fn octant(chunk: ChunkCoord) -> usize {
        let bit = |v: i32| v.rem_euclid(2) as usize;
        let i = bit(chunk.x) + 2 * bit(chunk.y) + 4 * bit(chunk.z);
        assert_eq!(child(cover(chunk), i), chunk);
        i
    }

    fn stone() -> Voxel {
        Voxel::cube(worldgen::STONE)
    }

    /// What the cover of `chunk` is with the store's copy of it: the one
    /// non-generated child.
    fn expected_cover(h: &Harness, chunk: ChunkCoord) -> Chunk {
        let held = h.store().get(chunk).expect("held");
        expected(1, cover(chunk), Some((octant(chunk), held)))
    }

    /// The last mesh written for each Chunk.
    fn latest(ready: Vec<LodMeshReady>) -> HashMap<LodKey, ChunkMesh> {
        ready
            .into_iter()
            .map(|m| ((m.level, m.coord), m.mesh))
            .collect()
    }

    fn same_chunk(a: &Chunk, b: &Chunk) -> bool {
        match (a.uniform(), b.uniform()) {
            (Some(x), Some(y)) => x == y,
            (None, None) => (0..CHUNK_SIZE).all(|z| {
                (0..CHUNK_SIZE).all(|y| (0..CHUNK_SIZE).all(|x| a.get(x, y, z) == b.get(x, y, z)))
            }),
            _ => false,
        }
    }

    fn world() -> WorldSettings {
        WorldSettings::default()
    }

    /// The generator of source level `j`, built on its own so a mistake in
    /// `LodGenerators` is not shared with the expectation.
    fn source(j: u32) -> Worldgen {
        let params = WorldgenParams::default();
        let params = if j == 0 {
            params
        } else {
            WorldgenParams {
                smoothing: None,
                ..params
            }
        };
        Worldgen::new(
            world().seed,
            world().voxel_size * 2f32.powi(j as i32),
            params,
        )
    }

    /// What a level-`level` Chunk is: the downsample of eight Chunks of the
    /// level-below generator, with `override_child` standing in for one of them.
    fn expected(level: u32, coord: ChunkCoord, override_child: Option<(usize, &Chunk)>) -> Chunk {
        let source = source(level - 1);
        let made: Vec<Chunk> = (0..8)
            .map(|i| {
                let c = child(coord, i);
                source.chunk(c.x, c.y, c.z)
            })
            .collect();
        let refs: [&Chunk; 8] = std::array::from_fn(|i| match override_child {
            Some((at, chunk)) if at == i => chunk,
            _ => &made[i],
        });
        downsample(refs, &Tables::builtin().shapes)
    }

    fn assert_holds_exactly_the_load_set(h: &Harness) {
        for level in 1..LEVELS {
            assert_eq!(h.held(level), h.load_set(level), "level {level}");
        }
    }

    #[test]
    fn the_children_of_a_chunk_are_its_octants() {
        let parent = ChunkCoord::new(-3, 2, 5);
        for i in 0..8 {
            let c = child(parent, i);
            let d = (c.x - 2 * parent.x, c.y - 2 * parent.y, c.z - 2 * parent.z);
            assert_eq!(d, ((i & 1) as i32, (i >> 1 & 1) as i32, (i >> 2) as i32));
        }
    }

    #[test]
    fn the_source_generators_sit_at_doubling_voxel_sizes_and_only_level_zero_smooths() {
        let level_zero = Worldgen::new(world().seed, 1.0, WorldgenParams::default());
        let sources = LodGenerators::new(&world(), &level_zero, LEVELS);
        assert_eq!(sources.levels(), LEVELS);
        assert!(sources.sources[0].params().smoothing.is_some());
        for (j, generator) in sources.sources.iter().enumerate().skip(1) {
            assert!(generator.params().smoothing.is_none(), "source {j}");
            // The same Chunk as an independently built generator makes.
            assert!(same_chunk(
                &generator.chunk(1, 0, 2),
                &source(j as u32).chunk(1, 0, 2)
            ));
        }
        assert_eq!(LodGenerators::new(&world(), &level_zero, 1).levels(), 1);
    }

    #[test]
    fn levels_one_two_and_three_are_the_downsample_of_their_generated_children() {
        let mut h = harness();
        h.lod_settle();
        assert_holds_exactly_the_load_set(&h);

        for level in 1..LEVELS {
            let mut varied = 0;
            for coord in h.held(level) {
                let held = h.lod_store().get(level, coord).unwrap();
                assert!(
                    same_chunk(held, &expected(level, coord, None)),
                    "level {level} {coord}"
                );
                varied += usize::from(held.uniform().is_none());
            }
            assert!(varied > 0, "level {level} had no terrain to get wrong");
        }
    }

    #[test]
    fn held_level_zero_children_give_the_result_generating_them_would() {
        let mut h = harness();
        h.settle();
        let held_children = h.store().coords().count();
        assert!(held_children > 0);
        h.lod_settle();

        // Some of level 1's children were held, and some were not.
        let covers_held = |coord: ChunkCoord| (0..8).any(|i| h.store().contains(child(coord, i)));
        let covers_absent =
            |coord: ChunkCoord| (0..8).any(|i| !h.store().contains(child(coord, i)));
        assert!(h.held(1).into_iter().any(covers_held));
        assert!(h.held(1).into_iter().any(covers_absent));
        for coord in h.held(1) {
            assert!(
                same_chunk(
                    h.lod_store().get(1, coord).unwrap(),
                    &expected(1, coord, None)
                ),
                "{coord}"
            );
        }
    }

    #[test]
    fn a_held_level_zero_child_is_used_in_its_octant() {
        let mut h = harness();
        // Octant 5: x and z in the upper half, y in the lower.
        let lantern = Chunk::filled(Voxel::cube(worldgen::LANTERN));
        let origin = ChunkCoord::new(0, 0, 0);
        let edited = child(origin, 5);
        assert_eq!(edited, ChunkCoord::new(1, 0, 1));
        h.app
            .world_mut()
            .resource_mut::<ChunkStore>()
            .insert(edited, lantern.clone());
        h.lod_settle();

        let got = h.lod_store().get(1, origin).unwrap();
        let half = CHUNK_SIZE / 2;
        assert_eq!(
            got.get(half + 2, 2, half + 2),
            Voxel::cube(worldgen::LANTERN)
        );
        assert!(same_chunk(got, &expected(1, origin, Some((5, &lantern)))));
        assert!(!same_chunk(got, &expected(1, origin, None)));
    }

    #[test]
    fn the_job_uses_what_it_is_handed_and_generates_the_rest() {
        let shapes = Tables::builtin().shapes;
        let source = source(0);
        let coord = ChunkCoord::new(2, -1, 3);
        let made = |i: usize| {
            let c = child(coord, i);
            source.chunk(c.x, c.y, c.z)
        };
        let all = generate_coarse(&source, coord, &Default::default(), &shapes);

        let held_even: [Option<Arc<Chunk>>; 8] =
            std::array::from_fn(|i| (i % 2 == 0).then(|| Arc::new(made(i))));
        assert!(same_chunk(
            &generate_coarse(&source, coord, &held_even, &shapes),
            &all
        ));

        let lantern = Arc::new(Chunk::filled(Voxel::cube(worldgen::LANTERN)));
        let mut edited = held_even;
        edited[2] = Some(Arc::clone(&lantern));
        let with_edit = generate_coarse(&source, coord, &edited, &shapes);
        assert!(!same_chunk(&with_edit, &all));
        assert!(same_chunk(
            &with_edit,
            &expected_with(&source, coord, 2, &lantern)
        ));
    }

    fn expected_with(source: &Worldgen, coord: ChunkCoord, at: usize, chunk: &Chunk) -> Chunk {
        let made: Vec<Chunk> = (0..8)
            .map(|i| {
                let c = child(coord, i);
                source.chunk(c.x, c.y, c.z)
            })
            .collect();
        let refs: [&Chunk; 8] = std::array::from_fn(|i| if i == at { chunk } else { &made[i] });
        downsample(refs, &Tables::builtin().shapes)
    }

    #[test]
    fn in_flight_never_exceeds_the_cap_and_landings_never_exceed_theirs() {
        let mut h = harness_with(
            lod_settings(),
            LodBudget {
                in_flight: LodCounts {
                    generate: 5,
                    ..unlimited().in_flight
                },
                landed_per_tick: LodCounts {
                    generate: 2,
                    ..unlimited().landed_per_tick
                },
            },
        );

        // Dispatch is synchronous, so the first tick's count does not depend on
        // how fast the workers are, and nothing has landed yet to be capped.
        h.lod_tick();
        assert_eq!(h.lod_jobs().generate.counts().0, 5);
        let keys = h.lod_jobs().generate.keys();
        assert!(keys.iter().all(|&(level, _)| level == 1), "{keys:?}");
        // Nearest first: nothing farther than the fifth-nearest Chunk was taken.
        let anchor = h.anchor_of(1);
        let dist = |c: ChunkCoord| delta(anchor, c).length_squared();
        let mut all: Vec<i32> = h.load_set(1).into_iter().map(dist).collect();
        all.sort_unstable();
        assert!(keys.iter().all(|&(_, c)| dist(c) <= all[4]));

        let started = std::time::Instant::now();
        for _ in 0..SETTLE_TICKS {
            assert!(started.elapsed() < SETTLE_CEILING, "never went quiet");
            h.lod_tick();
            let (dispatched, landed) = h.lod_jobs().generate.counts();
            assert!(
                h.lod_jobs().generate.in_flight() <= 5,
                "over the in-flight cap"
            );
            assert!(landed <= 2, "landed more than allowed");
            assert!(dispatched <= 5);
            if h.lod_quiet() {
                // Budget-limited levels must still all be reached.
                assert_holds_exactly_the_load_set(&h);
                return;
            }
            std::thread::sleep(std::time::Duration::from_micros(100));
        }
        panic!("never went quiet");
    }

    #[test]
    fn moving_the_anchor_loads_ahead_and_evicts_behind_on_every_level() {
        let mut h = harness();
        h.lod_settle();
        let before: Vec<HashSet<ChunkCoord>> = (1..LEVELS).map(|l| h.held(l)).collect();

        // Far past `evict` of the coarsest level.
        h.anchor_at(Vec3::new(3000.0, 0.0, 0.0));
        h.lod_tick();
        for level in 1..LEVELS {
            let evict = h.rings_of(level).evict;
            let anchor = h.anchor_of(level);
            for coord in h.held(level) {
                assert!(
                    evict.contains(delta(anchor, coord)),
                    "level {level} kept {coord}"
                );
            }
        }
        h.lod_settle();
        assert_holds_exactly_the_load_set(&h);
        for (level, old) in (1..LEVELS).zip(before) {
            assert!(
                old.is_disjoint(&h.held(level)),
                "level {level} still holds what it held at the origin"
            );
        }
    }

    #[test]
    fn a_level_whose_anchor_chunk_did_not_move_is_left_alone() {
        let mut h = harness();
        h.lod_settle();
        let (two, three) = (h.held(2), h.held(3));

        // One level-1 Cell: level 1's anchor moves, level 2's and 3's do not.
        let cell = h.app.world().resource::<WorldSettings>().cell_size_at(1);
        h.anchor_at(Vec3::new(cell, 0.0, 0.0));
        assert_eq!(h.anchor_of(2), ChunkCoord::new(0, 0, 0));
        h.lod_tick();
        let keys = h.lod_jobs().generate.keys();
        assert!(!keys.is_empty() && keys.iter().all(|&(level, _)| level == 1));
        h.lod_settle();
        assert_eq!((h.held(2), h.held(3)), (two, three));
        let anchor = h.anchor_of(1);
        let evict = h.rings_of(1).evict;
        assert!(h.held(1).iter().all(|&c| evict.contains(delta(anchor, c))));
        assert!(h.load_set(1).is_subset(&h.held(1)));
    }

    /// Hysteresis, per level. Without it an Eye on a Cell boundary would evict
    /// and regenerate the same Chunks every tick.
    #[test]
    fn crossing_a_cell_boundary_does_not_thrash() {
        let mut h = harness();
        h.lod_settle();
        let settled: Vec<HashSet<ChunkCoord>> = (1..LEVELS).map(|l| h.held(l)).collect();

        // Each level's own Cell edge, one by one.
        for level in 1..LEVELS {
            let cell = h
                .app
                .world()
                .resource::<WorldSettings>()
                .cell_size_at(level);
            for x in [cell - 0.01, cell, cell + 0.01, cell - 0.01, 0.0] {
                h.anchor_at(Vec3::new(x, 0.0, 0.0));
                h.lod_tick();
                // A finer level moves several of its own Cells over this
                // distance, and rightly evicts.
                for (l, old) in (1..LEVELS).zip(&settled).filter(|&(l, _)| l >= level) {
                    assert!(
                        old.is_subset(&h.held(l)),
                        "level {l} lost a Chunk at {x} m, crossing level {level}'s edge"
                    );
                }
            }
            // Back where it started, everything is already held.
            h.lod_settle();
        }
    }

    #[test]
    fn a_job_in_flight_for_a_chunk_the_anchor_leaves_lands_nothing() {
        let mut h = harness_with(
            lod_settings(),
            LodBudget {
                in_flight: LodCounts {
                    generate: 8,
                    ..unlimited().in_flight
                },
                ..unlimited()
            },
        );
        h.lod_tick();
        let started: Vec<LodKey> = h.lod_jobs().generate.keys();
        assert_eq!(started.len(), 8);

        h.anchor_at(Vec3::new(3000.0, 0.0, 0.0));
        h.lod_settle();
        for (level, coord) in started {
            assert!(!h.lod_store().contains(level, coord), "{level} {coord}");
        }
        assert_holds_exactly_the_load_set(&h);
    }

    /// The landing check on its own: the anchor has not moved, so `retire` does
    /// not run and nothing else stands between a result and the store.
    #[test]
    fn a_result_outside_evict_is_dropped_at_landing() {
        let mut h = harness();
        h.lod_settle();
        let far = ChunkCoord::new(100, 0, 0);
        h.app
            .world_mut()
            .resource_mut::<LodJobs>()
            .generate
            .spawn((1, far), || Chunk::filled(Voxel::cube(worldgen::LANTERN)));
        h.lod_settle();
        assert!(!h.lod_store().contains(1, far), "the far Chunk landed");
    }

    #[test]
    fn a_result_for_a_held_chunk_nothing_asked_to_replace_is_dropped() {
        let mut h = harness();
        h.lod_settle();
        let coord = ChunkCoord::new(0, 0, 0);
        let before = h.lod_store().shared(1, coord).unwrap();
        h.app
            .world_mut()
            .resource_mut::<LodJobs>()
            .generate
            .spawn((1, coord), || Chunk::filled(Voxel::cube(worldgen::LANTERN)));
        h.lod_settle();
        let after = h.lod_store().shared(1, coord).unwrap();
        assert!(
            Arc::ptr_eq(&before, &after),
            "an intruder replaced the Chunk"
        );
    }

    #[test]
    fn an_invalidated_chunk_is_regenerated_and_it_and_its_neighbours_are_meshed_again() {
        let mut h = harness();
        h.lod_settle();
        let coord = ChunkCoord::new(0, 0, 0);
        let before = h.lod_store().shared(1, coord).unwrap();

        // An Edit would have changed a child; what the cover should become.
        let lantern = Chunk::filled(Voxel::cube(worldgen::LANTERN));
        h.app
            .world_mut()
            .resource_mut::<ChunkStore>()
            .insert(child(coord, 7), lantern.clone());
        h.app
            .world_mut()
            .resource_mut::<LodState>()
            .invalidate(1, coord);
        let (ready, unloaded) = h.lod_settle();

        let after = h.lod_store().shared(1, coord).unwrap();
        assert!(!Arc::ptr_eq(&before, &after), "never replaced");
        assert!(same_chunk(&after, &expected(1, coord, Some((7, &lantern)))));
        assert!(!h.lod_state().is_stale(1, coord));

        // All inside level 1's mesh shell, so all meshed again, once each.
        let mut want: Vec<LodKey> = std::iter::once(coord)
            .chain(face_neighbors(coord))
            .map(|c| (1, c))
            .collect();
        let mut got: Vec<LodKey> = ready.iter().map(|m| (m.level, m.coord)).collect();
        want.sort_unstable_by_key(|&(l, c)| (l, c.x, c.y, c.z));
        got.sort_unstable_by_key(|&(l, c)| (l, c.x, c.y, c.z));
        assert_eq!(got, want);
        assert!(unloaded.is_empty());
    }

    #[test]
    fn a_first_generation_marks_only_itself_for_a_mesh() {
        let mut h = harness();
        h.lod_settle();
        assert_eq!(
            h.lod_state()
                .dirty(1)
                .union(&h.lod_state().delivered(1))
                .copied()
                .collect::<HashSet<_>>(),
            h.held(1)
        );

        // A Chunk that arrives next to held ones leaves their meshes alone. It
        // is outside the mesh shell, so nothing meshes it either.
        let edge = ChunkCoord::new(3, 0, 0);
        h.app.world_mut().resource_mut::<LodState>().clear_dirty();
        h.app
            .world_mut()
            .resource_mut::<LodStore>()
            .retain(1, |c| c != edge);
        h.app
            .world_mut()
            .resource_mut::<LodState>()
            .invalidate(1, edge);
        let (ready, _) = h.lod_settle();
        assert!(ready.is_empty(), "a margin Chunk's arrival re-meshed one");
        assert_eq!(h.lod_state().dirty(1), HashSet::from([edge]));
    }

    #[test]
    fn with_one_level_no_coarse_job_is_ever_spawned() {
        let settings = StreamingSettings {
            lod_distance_m: small().view_distance_m,
            ..small()
        };
        let mut h = harness_with(settings, LodBudget::default());
        for _ in 0..50 {
            h.lod_tick();
            assert_eq!(h.lod_jobs().generate.counts(), (0, 0));
            assert_eq!(h.lod_jobs().generate.in_flight(), 0);
        }
        assert!(h.lod_store().coords(1).next().is_none());
        // A moved Eye makes no difference.
        h.anchor_at(Vec3::new(500.0, 0.0, 0.0));
        h.lod_tick();
        assert_eq!(h.lod_jobs().generate.counts(), (0, 0));
    }

    #[test]
    fn scrambled_landing_order_reaches_the_same_state() {
        let mut state = Vec::new();
        for (scramble, cap) in [(false, usize::MAX), (true, usize::MAX), (true, 3)] {
            let mut h = harness_with(
                lod_settings(),
                LodBudget {
                    in_flight: LodCounts {
                        generate: cap,
                        ..unlimited().in_flight
                    },
                    ..unlimited()
                },
            );
            h.app
                .world_mut()
                .resource_mut::<LodJobs>()
                .generate
                .scramble = scramble;
            h.lod_settle();
            assert_holds_exactly_the_load_set(&h);
            state.push(h);
        }
        let reference = &state[0];
        for h in &state[1..] {
            for level in 1..LEVELS {
                for coord in reference.held(level) {
                    assert!(
                        same_chunk(
                            reference.lod_store().get(level, coord).unwrap(),
                            h.lod_store().get(level, coord).unwrap()
                        ),
                        "{level} {coord}"
                    );
                }
            }
        }
    }

    /// The `mesh_shell` Chunks of every coarse level, tagged with their level.
    fn all_shells(h: &Harness) -> HashSet<LodKey> {
        (1..LEVELS)
            .flat_map(|l| h.mesh_shell(l).into_iter().map(move |c| (l, c)))
            .collect()
    }

    #[test]
    fn at_quiescence_each_level_has_delivered_its_mesh_shell_and_every_mesh_is_fresh() {
        let mut h = harness();
        let (ready, unloaded) = h.lod_settle();
        assert!(unloaded.is_empty());

        // A Chunk is meshed once: nothing but a replacement re-dirties it.
        assert_eq!(ready.len(), all_shells(&h).len());
        let meshes = latest(ready);
        assert_eq!(
            meshes.keys().copied().collect::<HashSet<_>>(),
            all_shells(&h)
        );

        for level in 1..LEVELS {
            let shell = h.mesh_shell(level);
            let meshable: HashSet<ChunkCoord> = shell
                .iter()
                .copied()
                .filter(|&c| h.has_six_neighbours(level, c))
                .collect();
            assert_eq!(
                meshable, shell,
                "level {level} has a shell Chunk without a neighbour"
            );
            assert_eq!(h.delivered_at(level), meshable, "level {level}");

            let mut non_empty = 0;
            for &coord in &shell {
                let mesh = &meshes[&(level, coord)];
                assert!(
                    same_mesh(mesh, &h.fresh_mesh(level, coord)),
                    "level {level} {coord} differs from a fresh mesh"
                );
                non_empty += usize::from(!mesh.is_empty());
            }
            assert!(non_empty > 0, "level {level} had nothing to mesh");
        }
    }

    #[test]
    fn no_mesh_is_delivered_before_its_six_neighbours_are_held() {
        let mut h = harness_with(
            lod_settings(),
            LodBudget {
                in_flight: LodCounts {
                    generate: 3,
                    ..unlimited().in_flight
                },
                ..unlimited()
            },
        );
        {
            let mut jobs = h.app.world_mut().resource_mut::<LodJobs>();
            jobs.generate.scramble = true;
            jobs.mesh.scramble = true;
        }
        let mut delivered = 0;
        let started = std::time::Instant::now();
        loop {
            assert!(started.elapsed() < SETTLE_CEILING, "never went quiet");
            let (ready, _) = h.lod_tick();
            for m in &ready {
                assert!(
                    h.has_six_neighbours(m.level, m.coord),
                    "level {} {} was delivered with a neighbour missing",
                    m.level,
                    m.coord
                );
            }
            delivered += ready.len();
            if h.lod_quiet() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_micros(100));
        }
        assert_eq!(delivered, all_shells(&h).len());
    }

    #[test]
    fn every_vertex_light_is_full_sky_and_no_block_light() {
        let mut h = harness();
        let (ready, _) = h.lod_settle();
        let want = u32::from(Light::new(MAX_LIGHT, 0).packed());
        let mut vertices = 0;
        for m in &ready {
            for layer in [&m.mesh.opaque, &m.mesh.blended] {
                assert_eq!(layer.light.len(), layer.positions.len());
                assert!(
                    layer.light.iter().all(|&l| l == want),
                    "level {} {} has a vertex not lit as open sky",
                    m.level,
                    m.coord
                );
                vertices += layer.light.len();
            }
        }
        assert!(vertices > 0, "no vertex to check");
    }

    #[test]
    fn a_coarse_mesh_has_no_shaped_voxel_geometry_though_its_children_do() {
        let mut h = harness();
        // A held child of stairs, which level 0 would give shaped geometry.
        let stairs = Chunk::filled(Voxel::new(worldgen::STONE, SHAPE_STAIR, 5));
        let tables = Tables::builtin();
        let level_zero = mesh_chunk(
            LitChunk::new(&stairs, &OPEN_SKY),
            Default::default(),
            &tables,
        );
        assert!(
            level_zero.shaped_voxels > 0,
            "level 0 would not mesh a stair"
        );
        h.app
            .world_mut()
            .resource_mut::<ChunkStore>()
            .insert(child(ChunkCoord::new(0, 0, 0), 3), stairs);

        let (ready, _) = h.lod_settle();
        let mut level_one = 0;
        for m in &ready {
            assert_eq!(m.mesh.shaped_voxels, 0, "level {} {}", m.level, m.coord);
            level_one += usize::from(m.level == 1);
        }
        assert!(level_one > 0);
        for coord in h.held(1) {
            assert_eq!(h.lod_store().get(1, coord).unwrap().shaped_count(), 0);
        }
    }

    #[test]
    fn a_teleport_unloads_every_delivered_mesh_before_its_chunk_is_forgotten() {
        let mut h = harness();
        h.lod_settle();
        let before = all_shells(&h);
        assert!(!before.is_empty());

        h.anchor_at(Vec3::new(3000.0, 0.0, 0.0));
        let (_, unloaded) = h.lod_tick();
        let unloaded: HashSet<LodKey> = unloaded.iter().map(|u| (u.level, u.coord)).collect();
        assert_eq!(unloaded, before);
        for level in 1..LEVELS {
            assert!(h.delivered_at(level).is_empty(), "level {level}");
        }

        // And the new neighbourhood fills in with nothing left over from the old.
        let (ready, again) = h.lod_settle();
        assert!(again.is_empty());
        assert_eq!(
            latest(ready).keys().copied().collect::<HashSet<_>>(),
            all_shells(&h)
        );
        assert!(before.is_disjoint(&all_shells(&h)));
    }

    /// The module doc's argument, per level: a mesh lands only if its Chunk is
    /// inside `keep`, so every neighbour it read is inside `load` and still held.
    /// Two distances: past `keep`, where `retire` cancels the job, and between
    /// `mesh` and `keep`, where the Chunk is still wanted.
    #[test]
    fn a_fly_by_with_meshes_outstanding_never_delivers_a_mesh_with_an_evicted_neighbour() {
        for cells in [1, 2, 10] {
            let mut h = harness();
            h.app.world_mut().resource_mut::<LodJobs>().mesh.hold.hold();
            let shells = all_shells(&h).len();
            let started = std::time::Instant::now();
            while h.lod_jobs().mesh.in_flight() < shells {
                assert!(
                    started.elapsed() < SETTLE_CEILING,
                    "meshes never dispatched"
                );
                h.lod_tick();
                std::thread::sleep(std::time::Duration::from_micros(100));
            }

            // Level 1 moves `cells` of its own Cells; the coarser levels move
            // less, or not at all.
            let cell = h.app.world().resource::<WorldSettings>().cell_size_at(1);
            h.anchor_at(Vec3::new(cells as f32 * cell, 0.0, 0.0));
            h.lod_tick();
            if cells > 5 {
                let keys = h.lod_jobs().mesh.keys();
                assert!(
                    !keys.contains(&(1, ChunkCoord::new(0, 0, 0))),
                    "not cancelled"
                );
            }
            h.app
                .world_mut()
                .resource_mut::<LodJobs>()
                .mesh
                .hold
                .release();

            let mut ready = Vec::new();
            let started = std::time::Instant::now();
            loop {
                assert!(started.elapsed() < SETTLE_CEILING, "never went quiet");
                let (r, _) = h.lod_tick();
                for m in &r {
                    let keep = h.rings_of(m.level).keep;
                    assert!(
                        keep.contains(delta(h.anchor_of(m.level), m.coord)),
                        "level {} {} was delivered from outside keep, {cells} Cells away",
                        m.level,
                        m.coord
                    );
                    assert!(
                        h.has_six_neighbours(m.level, m.coord),
                        "level {} {} was delivered with an evicted neighbour",
                        m.level,
                        m.coord
                    );
                }
                ready.extend(r);
                if h.lod_quiet() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_micros(100));
            }

            // What the Client ends up holding is the shell, current.
            for level in 1..LEVELS {
                assert!(h.mesh_shell(level).is_subset(&h.delivered_at(level)));
                let keep = h.rings_of(level).keep;
                let anchor = h.anchor_of(level);
                for coord in h.delivered_at(level) {
                    assert!(keep.contains(delta(anchor, coord)), "level {level} {coord}");
                }
            }
            for ((level, coord), mesh) in latest(ready) {
                assert!(
                    same_mesh(&mesh, &h.fresh_mesh(level, coord)),
                    "level {level} {coord} is stale, {cells} Cells away"
                );
            }
        }
    }

    /// Dispatch takes a Chunk out of `dirty`, so a job `retire` cancels would
    /// otherwise leave a held Chunk that nothing ever meshes again.
    #[test]
    fn a_mesh_cancelled_by_a_move_is_built_when_the_eye_comes_back() {
        let mut h = harness();
        h.app.world_mut().resource_mut::<LodJobs>().mesh.hold.hold();
        let shells = all_shells(&h).len();
        let started = std::time::Instant::now();
        while h.lod_jobs().mesh.in_flight() < shells {
            assert!(
                started.elapsed() < SETTLE_CEILING,
                "meshes never dispatched"
            );
            h.lod_tick();
            std::thread::sleep(std::time::Duration::from_micros(100));
        }
        let shell = h.mesh_shell(1);
        let behind = ChunkCoord::new(-1, 0, 0);
        assert!(h.lod_jobs().mesh.keys().contains(&(1, behind)));

        // Out of keep for the Chunks behind, but not out of evict.
        let cell = h.app.world().resource::<WorldSettings>().cell_size_at(1);
        h.anchor_at(Vec3::new(3.0 * cell, 0.0, 0.0));
        h.lod_tick();
        assert!(
            !h.lod_jobs().mesh.keys().contains(&(1, behind)),
            "not cancelled"
        );
        assert!(
            h.lod_store().contains(1, behind),
            "evicted, not just released"
        );
        h.app
            .world_mut()
            .resource_mut::<LodJobs>()
            .mesh
            .hold
            .release();
        h.lod_settle();
        assert!(!h.delivered_at(1).contains(&behind));

        h.anchor_at(Vec3::ZERO);
        h.lod_settle();
        // A superset: what was meshed on the way is still inside `keep`.
        assert!(shell.is_subset(&h.delivered_at(1)));
    }

    #[test]
    fn a_result_outside_keep_is_dropped_at_landing() {
        let mut h = harness();
        h.lod_settle();
        let far = ChunkCoord::new(100, 0, 0);
        h.app
            .world_mut()
            .resource_mut::<LodJobs>()
            .mesh
            .spawn((1, far), ChunkMesh::default);
        let (ready, _) = h.lod_settle();
        assert!(ready.is_empty(), "a mesh from outside keep was delivered");
        assert!(!h.delivered_at(1).contains(&far));
    }

    /// The starvation argument: a mark made while the job runs must not cost the
    /// Chunk its mesh, and must still produce the fresh one.
    #[test]
    fn a_chunk_dirtied_in_flight_is_delivered_and_meshed_again() {
        let mut h = harness();
        h.app.world_mut().resource_mut::<LodJobs>().mesh.hold.hold();
        let shells = all_shells(&h).len();
        let started = std::time::Instant::now();
        while h.lod_jobs().mesh.in_flight() < shells {
            assert!(
                started.elapsed() < SETTLE_CEILING,
                "meshes never dispatched"
            );
            h.lod_tick();
            std::thread::sleep(std::time::Duration::from_micros(100));
        }

        let marked = ChunkCoord::new(0, 0, 0);
        h.app
            .world_mut()
            .resource_mut::<LodState>()
            .level_mut(1)
            .dirty
            .insert(marked);
        h.app
            .world_mut()
            .resource_mut::<LodJobs>()
            .mesh
            .hold
            .release();
        let (ready, _) = h.lod_settle();

        let count = |c: ChunkCoord| {
            ready
                .iter()
                .filter(|m| (m.level, m.coord) == (1, c))
                .count()
        };
        assert_eq!(count(marked), 2, "delivered once, then again");
        assert_eq!(count(ChunkCoord::new(1, 0, 0)), 1);
        assert!(h.delivered_at(1).contains(&marked));
        assert!(h.lod_jobs().mesh.stale() > 0);
    }

    /// A hole with room for a hysteresis around it: level 0 is six Cells across
    /// and two high, so level 1 has a hole of two Cells and one Cell of keep.
    fn hole_settings() -> StreamingSettings {
        StreamingSettings {
            view_distance_m: 192.0,
            vertical_distance_m: 64.0,
            hysteresis_m: 32.0,
            lod_distance_m: 384.0,
        }
    }

    fn d(x: i32, y: i32, z: i32) -> IVec3 {
        IVec3::new(x, y, z)
    }

    #[test]
    fn the_hole_is_not_meshed_and_the_overlap_around_it_is() {
        let mut h = harness_with(hole_settings(), unlimited());
        let rings = h.rings_of(1);
        assert_eq!(
            rings.mesh.hole.map(|e| (e.horizontal, e.vertical)),
            Some((2, 0))
        );
        let (ready, _) = h.lod_settle();
        let anchor = h.anchor_of(1);
        let coord_at = |delta: IVec3| offset(anchor, delta);

        let delivered = h.delivered_at(1);
        // Inside the hole: level 0 draws it.
        for delta in [d(0, 0, 0), d(1, 0, 0), d(0, 0, -2), d(1, 0, 1)] {
            assert!(
                !delivered.contains(&coord_at(delta)),
                "{delta} is in the hole"
            );
        }
        // The Cell of overlap, inside what level 0 covers (3 Cells here) and
        // outside the hole.
        for delta in [d(2, 0, 1), d(3, 0, 0), d(0, 1, 0), d(0, -1, 2)] {
            assert!(
                delivered.contains(&coord_at(delta)),
                "{delta} is the overlap"
            );
        }
        let expected: HashSet<ChunkCoord> = h
            .mesh_shell(1)
            .into_iter()
            .filter(|&c| !rings.mesh.hole.unwrap().contains(delta(anchor, c)))
            .collect();
        assert_eq!(delivered, expected);
        assert_eq!(latest(ready).len(), expected.len());
    }

    #[test]
    fn walking_onto_a_coarse_chunk_releases_it_only_past_the_hysteresis() {
        let mut h = harness_with(hole_settings(), unlimited());
        h.lod_settle();
        let cell = h.app.world().resource::<WorldSettings>().cell_size_at(1);
        let target = ChunkCoord::new(4, 0, 0);
        assert!(h.delivered_at(1).contains(&target));

        // Two Cells away: inside the mesh hole, but still inside keep's.
        h.anchor_at(Vec3::new(2.0 * cell, 0.0, 0.0));
        let (_, unloaded) = h.lod_tick();
        assert!(
            !unloaded.iter().any(|u| (u.level, u.coord) == (1, target)),
            "released inside the hysteresis"
        );
        assert!(h.delivered_at(1).contains(&target));
        h.lod_settle();
        assert!(h.delivered_at(1).contains(&target));

        // One Cell away: past it.
        h.anchor_at(Vec3::new(3.0 * cell, 0.0, 0.0));
        let (_, unloaded) = h.lod_tick();
        assert!(
            unloaded.iter().any(|u| (u.level, u.coord) == (1, target)),
            "kept past the hysteresis"
        );
        h.lod_settle();
        assert!(!h.delivered_at(1).contains(&target));

        // And walking back draws it again.
        h.anchor_at(Vec3::ZERO);
        let (ready, _) = h.lod_settle();
        assert!(ready.iter().any(|m| (m.level, m.coord) == (1, target)));
        assert!(h.delivered_at(1).contains(&target));
    }

    #[test]
    fn an_edit_under_a_held_cover_regenerates_it_and_delivers_the_new_mesh() {
        let mut h = harness();
        h.settle();
        h.lod_settle();
        let chunk = h.quiet_chunk();
        let covering = cover(chunk);
        let before = h.cover_arc(covering);

        h.edit_block(chunk, [10, 10, 10], stone());
        let (ready, _) = h.lod_settle();

        let after = h.cover_arc(covering);
        assert!(!same_chunk(&before, &after), "the Edit changed nothing");
        assert!(same_chunk(&after, &expected_cover(&h, chunk)));
        let meshes = latest(ready);
        let delivered = meshes
            .get(&(1, covering))
            .expect("the cover was redelivered");
        assert!(same_mesh(delivered, &h.fresh_mesh(1, covering)));
        assert!(!h.lod_state().is_stale(1, covering));
    }

    #[test]
    fn an_edit_under_a_cover_that_is_not_held_dispatches_nothing_and_is_in_it_when_it_is_made() {
        let mut h = harness();
        h.settle();
        let chunk = h.quiet_chunk();
        let covering = cover(chunk);

        h.edit_block(chunk, [10, 10, 10], stone());
        let (_, _, dispatched) = h.lod_settle_counting();

        let held: usize = (1..LEVELS).map(|level| h.held(level).len()).sum();
        assert_eq!(dispatched, held, "something was generated twice");
        assert!(same_chunk(
            &h.cover_arc(covering),
            &expected_cover(&h, chunk)
        ));
    }

    #[test]
    fn an_edit_while_a_cover_is_first_generated_is_not_lost_when_it_lands() {
        let mut h = harness();
        h.settle();
        let chunk = h.quiet_chunk();
        let covering = cover(chunk);

        h.lod_tick();
        assert!(
            h.lod_jobs().generate.contains((1, covering)),
            "no job to race"
        );
        h.edit_block(chunk, [10, 10, 10], stone());
        h.lod_settle();

        assert!(same_chunk(
            &h.cover_arc(covering),
            &expected_cover(&h, chunk)
        ));
    }

    /// Dispatch takes the cover out of `stale`, so a mark made while its job
    /// runs is the only thing that asks for the job after it.
    #[test]
    fn a_mark_made_while_a_cover_regenerates_survives_its_landing() {
        let mut h = harness();
        h.settle();
        h.lod_settle();
        let chunk = h.quiet_chunk();
        let covering = cover(chunk);

        h.edit_block(chunk, [10, 10, 10], stone());
        h.lod_tick();
        assert!(
            h.lod_jobs().generate.contains((1, covering)),
            "no job to race"
        );
        let with_first = h.store().get(chunk).unwrap().clone();
        h.edit_block(chunk, [20, 20, 20], stone());
        h.lod_settle();

        let got = h.cover_arc(covering);
        assert!(same_chunk(&got, &expected_cover(&h, chunk)));
        assert!(!same_chunk(
            &got,
            &expected(1, covering, Some((octant(chunk), &with_first)))
        ));
        assert!(!h.lod_state().is_stale(1, covering));
    }

    #[test]
    fn an_edited_chunk_leaving_the_store_takes_its_edit_out_of_the_cover() {
        let mut h = harness();
        h.settle();
        h.lod_settle();
        let chunk = h.quiet_chunk();
        let covering = cover(chunk);
        h.edit_block(chunk, [10, 10, 10], stone());
        h.lod_settle();
        let with_edit = h.cover_arc(covering);
        assert!(!same_chunk(&with_edit, &expected(1, covering, None)));
        assert!(h.app.world().resource::<MeshState>().is_edited(chunk));

        h.anchor_at(Vec3::new(160.0, 0.0, 0.0));
        h.settle();
        assert!(!h.store().contains(chunk), "the Chunk was not evicted");
        assert!(!h.app.world().resource::<MeshState>().is_edited(chunk));
        h.lod_settle();

        let got = h.cover_arc(covering);
        assert!(!Arc::ptr_eq(&got, &with_edit));
        assert!(same_chunk(&got, &expected(1, covering, None)));
    }

    #[test]
    fn an_unedited_chunk_leaving_the_store_regenerates_nothing() {
        let mut h = harness();
        h.settle();
        h.lod_settle();
        let before: Vec<(ChunkCoord, Arc<Chunk>)> =
            h.held(1).into_iter().map(|c| (c, h.cover_arc(c))).collect();
        let chunk = h.quiet_chunk();

        h.anchor_at(Vec3::new(160.0, 0.0, 0.0));
        h.settle();
        assert!(!h.store().contains(chunk), "the Chunk was not evicted");
        h.lod_settle();

        let mut kept = 0;
        for (coord, was) in before {
            if h.lod_store().contains(1, coord) {
                kept += 1;
                assert!(
                    Arc::ptr_eq(&h.cover_arc(coord), &was),
                    "{coord} was rebuilt"
                );
            }
        }
        assert!(kept > 0);
    }

    /// A Chunk edited faster than its cover is rebuilt must still show the
    /// Client a cover, as at level 0.
    #[test]
    fn sustained_edits_under_a_cover_still_deliver_its_mesh() {
        let mut h = harness();
        h.settle();
        h.lod_settle();
        let chunk = h.quiet_chunk();
        let covering = cover(chunk);

        let mut delivered = 0;
        let started = std::time::Instant::now();
        let mut i = 0;
        while delivered == 0 && started.elapsed() < SETTLE_CEILING {
            i += 1;
            h.edit_block(
                chunk,
                [10, 10, 10],
                if i % 2 == 0 { stone() } else { Voxel::AIR },
            );
            let (ready, _) = h.lod_tick();
            delivered += ready
                .iter()
                .filter(|m| (m.level, m.coord) == (1, covering))
                .count();
            std::thread::sleep(std::time::Duration::from_micros(20));
        }
        assert!(
            delivered > 0,
            "{covering} was edited every tick and never redrawn"
        );
    }
}
