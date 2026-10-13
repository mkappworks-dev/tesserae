//! Coarse LOD levels: Chunks above level 0, derived and never edited.
//!
//! A level-k Chunk is the [`downsample`] of eight level-(k-1) Chunks. Those are
//! generated for the purpose by the level-(k-1) source generator, not read from
//! a store, because the Server holds level 0 only within `evict` of the Eye and
//! the children of a distant coarse Chunk do not exist. The one exception is
//! level 1, which takes the level-0 Chunks the [`ChunkStore`] holds instead of
//! generating them, so an Edit reaches it.
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
use tesserae_core::{Chunk, ShapeTable, Worldgen, WorldgenParams, downsample};

use super::jobs::Stage;
use super::lod_rings::{LevelRings, level_count};
use super::player::Player;
use super::streaming::{Extent, StreamingSettings, delta, face_neighbors, offset};
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
            in_flight: LodCounts { generate: 16 },
            landed_per_tick: LodCounts { generate: 16 },
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

    #[cfg_attr(not(test), allow(dead_code))] // Stats and meshing read these (ticket 04).
    pub fn get(&self, level: u32, coord: ChunkCoord) -> Option<&Chunk> {
        self.at(level)?.get(&coord).map(Arc::as_ref)
    }

    /// A Chunk as a shared handle, for a job to hold.
    #[cfg_attr(not(test), allow(dead_code))] // Mesh jobs hold these (ticket 04).
    pub fn shared(&self, level: u32, coord: ChunkCoord) -> Option<Arc<Chunk>> {
        self.at(level)?.get(&coord).map(Arc::clone)
    }

    #[cfg_attr(not(test), allow(dead_code))] // Stats and meshing read these (ticket 04).
    pub fn coords(&self, level: u32) -> impl Iterator<Item = ChunkCoord> + '_ {
        self.at(level).into_iter().flat_map(|l| l.keys().copied())
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
    /// Load offsets by extent. Five of six levels share one at the defaults.
    offsets: Vec<(Extent, Arc<Vec<IVec3>>)>,
}

impl LodState {
    fn level_mut(&mut self, level: u32) -> &mut Level {
        let index = level as usize - 1;
        if self.levels.len() <= index {
            self.levels.resize_with(index + 1, Level::default);
        }
        &mut self.levels[index]
    }

    fn offsets_for(&mut self, extent: Extent) -> Arc<Vec<IVec3>> {
        if let Some((_, offsets)) = self.offsets.iter().find(|(e, _)| *e == extent) {
            return Arc::clone(offsets);
        }
        let offsets = Arc::new(extent.offsets());
        self.offsets.push((extent, Arc::clone(&offsets)));
        offsets
    }

    /// A held Chunk's inputs changed, so it is regenerated.
    #[cfg_attr(not(test), allow(dead_code))] // Edits call this (ticket 05).
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
        self.levels.iter().any(|l| l.pending)
    }

    #[cfg(test)]
    fn is_stale(&self, level: u32, coord: ChunkCoord) -> bool {
        self.levels
            .get(level as usize - 1)
            .is_some_and(|l| l.stale.contains(&coord))
    }
}

/// The coarse generate jobs in flight, keyed by level and coord.
#[derive(Resource, Default)]
pub(in crate::server) struct LodJobs {
    pub generate: Stage<Chunk, LodKey>,
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

/// Where one level stands this tick.
struct Plan {
    level: u32,
    anchor: ChunkCoord,
    rings: LevelRings,
}

#[expect(
    clippy::too_many_arguments,
    reason = "one tick, as `stream`; a SystemParam bundle would be a type used once"
)]
pub(in crate::server) fn stream_lod(
    chunks: Res<ChunkStore>,
    mut store: ResMut<LodStore>,
    mut state: ResMut<LodState>,
    mut jobs: ResMut<LodJobs>,
    generators: Res<LodGenerators>,
    tables: Res<ContentTables>,
    world: Res<WorldSettings>,
    settings: Res<StreamingSettings>,
    budget: Res<LodBudget>,
    player: Query<&Player>,
) {
    let levels = generators.levels();
    if levels <= 1 {
        return;
    }
    let Ok(player) = player.single() else {
        return;
    };

    jobs.generate.reset_counts();
    jobs.generate.poll();

    let plans: Vec<Plan> = (1..levels)
        .map(|level| Plan {
            level,
            anchor: world.chunk_at_level(player.eye(), level),
            rings: LevelRings::new(*settings, world.cell_size(), level),
        })
        .collect();

    retire(&plans, &mut store, &mut state, &mut jobs);
    land(&plans, &mut store, &mut state, &mut jobs, &budget);
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
}

/// Drops what left `evict` of a level whose anchor Chunk moved. Nothing else
/// can put a Chunk outside it, so a level whose anchor held still is not looked
/// at.
fn retire(plans: &[Plan], store: &mut LodStore, state: &mut LodState, jobs: &mut LodJobs) {
    let mut moved = false;
    for plan in plans {
        let level = state.level_mut(plan.level);
        if level.anchor == Some(plan.anchor) {
            continue;
        }
        moved = true;
        level.anchor = Some(plan.anchor);
        level.pending = true;
        let inside = |c: ChunkCoord| plan.rings.evict.contains(delta(plan.anchor, c));
        level.dirty.retain(|&c| inside(c));
        level.stale.retain(|&c| inside(c));
        level.regenerating.retain(|&c| inside(c));
        level.delivered.retain(|&c| inside(c));
        store.retain(plan.level, inside);
    }
    if moved {
        jobs.generate.retain(|(level, coord)| {
            plans
                .get(level as usize - 1)
                .is_some_and(|p| p.rings.evict.contains(delta(p.anchor, coord)))
        });
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
    let anchor_of = |level: u32| plans[level as usize - 1].anchor;
    let mut finished: Vec<LodKey> = jobs.generate.finished().collect();
    finished
        .sort_unstable_by_key(|&(level, c)| (level, delta(anchor_of(level), c).length_squared()));
    for key in finished.into_iter().take(budget.landed_per_tick.generate) {
        let Some(chunk) = jobs.generate.take(key) else {
            continue;
        };
        let (level, coord) = key;
        let plan = &plans[level as usize - 1];
        let held = store.contains(level, coord);
        let lv = state.level_mut(level);
        // A slot has freed whether or not the result is used.
        lv.pending = true;
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
        let offsets = state.offsets_for(plan.rings.load);
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
            let source = generators.sources[plan.level as usize - 1].clone();
            let tables = Arc::clone(&tables.0);
            jobs.generate.spawn((plan.level, coord), move || {
                generate_coarse(&source, coord, &held_children, &tables.shapes)
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::streaming::tests::{Harness, SETTLE_CEILING, UNLIMITED, small};
    use bevy::ecs::system::RunSystemOnce;
    use tesserae_core::{CHUNK_SIZE, Tables, Voxel, worldgen};

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
            },
            landed_per_tick: LodCounts {
                generate: usize::MAX,
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
        fn lod_tick(&mut self) {
            self.app
                .world_mut()
                .run_system_once(stream_lod)
                .expect("the tick ran");
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

        fn lod_quiet(&self) -> bool {
            let (dispatched, landed) = self.lod_jobs().generate.counts();
            self.lod_jobs().generate.in_flight() == 0
                && (dispatched, landed) == (0, 0)
                && !self.lod_state().any_pending()
        }

        fn lod_settle(&mut self) {
            let started = std::time::Instant::now();
            for _ in 0..SETTLE_TICKS {
                if started.elapsed() > SETTLE_CEILING {
                    break;
                }
                self.lod_tick();
                if self.lod_quiet() {
                    return;
                }
                if self.lod_jobs().generate.in_flight() > 0 {
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
                in_flight: LodCounts { generate: 5 },
                landed_per_tick: LodCounts { generate: 2 },
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
                in_flight: LodCounts { generate: 8 },
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
    fn an_invalidated_chunk_is_regenerated_and_its_neighbours_are_marked_for_a_new_mesh() {
        let mut h = harness();
        h.lod_settle();
        let coord = ChunkCoord::new(0, 0, 0);
        let before = h.lod_store().shared(1, coord).unwrap();
        h.app.world_mut().resource_mut::<LodState>().clear_dirty();

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
        h.lod_settle();

        let after = h.lod_store().shared(1, coord).unwrap();
        assert!(!Arc::ptr_eq(&before, &after), "never replaced");
        assert!(same_chunk(&after, &expected(1, coord, Some((7, &lantern)))));
        assert!(!h.lod_state().is_stale(1, coord));

        let want: HashSet<ChunkCoord> = std::iter::once(coord)
            .chain(face_neighbors(coord))
            .collect();
        assert_eq!(h.lod_state().dirty(1), want);
        assert!(h.lod_state().dirty(2).is_empty());
    }

    #[test]
    fn a_first_generation_marks_only_itself_for_a_mesh() {
        let mut h = harness();
        h.lod_settle();
        assert_eq!(h.lod_state().dirty(1), h.held(1));

        // A Chunk that arrives next to held ones leaves their meshes alone.
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
        h.lod_settle();
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
                    in_flight: LodCounts { generate: cap },
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
}
