//! Running `tesserae_core::flow` on workers and landing what it proposes.
//!
//! One logical step at a time. A step takes its due Voxels from the bounded
//! [`Agenda`], snapshots every Chunk they read, and hands them to up to
//! [`JOBS_IN_FLIGHT`] workers in slices of a quota of evaluations, each slice
//! carrying its own Voxels' timers. Once every slice is back the step lands
//! whole, in one tick, through `edit::set_occupants`, and the next starts: so
//! the quota, the number of workers and the order they finish in change how
//! many ticks a step takes and never what lands. At most one step completes
//! per tick, so a quiet Server simulates in real time and a busy one slows
//! down rather than skipping.
//!
//! **When events enter.** `now` is the step under way, or the empty step the
//! simulation idles at. Every event has the index `now` had when it happened,
//! and every event of index `t` is first read by step `t + 1`:
//!
//! | Event                          | Seen by `simulate`          | Index  |
//! | ------------------------------ | --------------------------- | ------ |
//! | player Edit (`apply_pending`)  | same tick, it runs first    | `now`  |
//! | Mod Command (`mods::Drain`)    | same tick, it runs first    | `now`  |
//! | Chunk admission (first lit)    | next tick's start           | `now`  |
//! | eviction, or a new lifetime    | next tick's start           | `now`  |
//! | a simulated Chunk's relight    | next tick's start           | `now`  |
//!
//! Step `t`'s Snapshot is taken when it starts, before any event of index
//! `t`, and it lands after all of them: its proposals that read anything they
//! changed, an Edit's position or a Chunk that left or arrived anew, are
//! dropped and their Voxels woken for `t + 1`; nothing else is restarted. A
//! Chunk counts as admitted once its first light solve has landed, and while
//! a later solve runs nothing lands in it (its Voxels are parked until the
//! solve lands), because a bulk solve must not race an incremental relight.
//!
//! **Bounds.** The agenda's [`AGENDA`] limits, [`LANDED_PER_STEP`] and the
//! store's journal are rules of the simulation, like its step length: under
//! load they decide when a Voxel changes, deterministically, and never whether
//! it does. A step's changes past the landing bound wait for the next step in
//! turn round the Chunks, from a cursor in each; stale and parked attempts
//! count towards it. Evaluations per call and workers in flight are how a step
//! is divided, and change nothing it computes.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::sync::Arc;

use bevy::prelude::*;
use tesserae_core::agenda::{Agenda, Limits, Span};
use tesserae_core::flow::{
    Advance, Cell, Continuation, DEPENDENTS, Finished, FlowScale, Proposal, Quota, READS, Region,
    Step, Timers, Work, advance, chunk_of, readers_outside, reading_outside,
};
use tesserae_core::{FluidTable, Tables};

use super::ContentTables;
use super::edit::{self, set_occupants};
use super::jobs::Stage;
use super::streaming::MeshState;
use super::world_state::{ChunkStore, Lifetime, VoxelCoord};
use crate::world::{ChunkCoord, WorldSettings};

/// Evaluations one worker call may make before it hands back a continuation.
const EVALUATIONS_PER_JOB: usize = 1024;

const JOBS_IN_FLIGHT: usize = 4;

/// Voxels waiting on a step, and how fast rescans catch up with the rest.
const AGENDA: Limits = Limits {
    capacity: 16_384,
    reserve: 4_096,
    scan_per_step: 4_096,
};

/// Changes one step lands. Each relights where water appears or goes, so this
/// bounds the tick a step lands in.
const LANDED_PER_STEP: usize = 64;

/// Edited positions a step's landing checks one by one before it treats their
/// whole Chunks as edited.
const EDITED_CAP: usize = 4096;

#[derive(Resource)]
pub(in crate::server) struct FluidSim {
    now: Step,
    agenda: Agenda,
    /// Held here between steps and by the running step's slices during one.
    timers: Timers,
    running: Option<Running>,
    job: Stage<Advance, usize>,
    /// What the simulation reads, by the lifetime it admitted.
    simulated: BTreeMap<ChunkCoord, Lifetime>,
    /// Held but not lit yet, so not yet admitted.
    awaiting: BTreeSet<ChunkCoord>,
    /// Simulated Chunks a light solve is pending for: nothing lands in them.
    suspended: BTreeSet<ChunkCoord>,
    /// What events of index `now` changed, which the running step did not see.
    edited: HashSet<Cell>,
    edited_chunks: BTreeSet<ChunkCoord>,
    /// The Chunk landing served last, and where each Chunk with changes left
    /// over starts next time.
    turn: Option<[i32; 3]>,
    cursors: BTreeMap<[i32; 3], Cell>,
    scale: Option<Arc<FlowScale>>,
    quota: Quota,
    jobs_in_flight: usize,
    landed_per_step: usize,
    pub work: Work,
    pub landed: usize,
    /// Changes dropped because what they read had changed.
    pub stale: usize,
    /// Changes past the landing bound, evaluated again next step.
    pub deferred: usize,
    /// Changes put to sleep because their Chunk's light was being solved.
    pub parked: usize,
    /// Voxels a rescan supplied.
    pub scanned: usize,
    /// Ticks the running step's results are left uncollected, as if its
    /// workers were that much slower.
    #[cfg(test)]
    pub stall: usize,
}

/// A step under way.
struct Running {
    step: Step,
    region: Arc<Region>,
    /// The lifetime of every Chunk in `region`.
    read: HashMap<[i32; 3], Lifetime>,
    slices: Vec<Slice>,
}

enum Slice {
    Waiting(Continuation),
    Out,
    Done(Finished),
}

impl Default for FluidSim {
    fn default() -> Self {
        FluidSim {
            now: 0,
            agenda: Agenda::new(AGENDA),
            timers: Timers::default(),
            running: None,
            job: Stage::default(),
            simulated: BTreeMap::new(),
            awaiting: BTreeSet::new(),
            suspended: BTreeSet::new(),
            edited: HashSet::new(),
            edited_chunks: BTreeSet::new(),
            turn: None,
            cursors: BTreeMap::new(),
            scale: None,
            quota: Quota {
                evaluations: EVALUATIONS_PER_JOB,
            },
            jobs_in_flight: JOBS_IN_FLIGHT,
            landed_per_step: LANDED_PER_STEP,
            work: Work::default(),
            landed: 0,
            stale: 0,
            deferred: 0,
            parked: 0,
            scanned: 0,
            #[cfg(test)]
            stall: 0,
        }
    }
}

fn key(coord: ChunkCoord) -> [i32; 3] {
    [coord.x, coord.y, coord.z]
}

fn coord(chunk: [i32; 3]) -> ChunkCoord {
    ChunkCoord::new(chunk[0], chunk[1], chunk[2])
}

fn offset(at: Cell, d: Cell) -> Cell {
    [at[0] + d[0], at[1] + d[1], at[2] + d[2]]
}

impl FluidSim {
    #[cfg(test)]
    pub fn now(&self) -> Step {
        self.now
    }

    /// Whether nothing is due, waiting on a rescan or running.
    #[cfg(test)]
    pub fn is_idle(&self) -> bool {
        self.running.is_none() && self.agenda.is_empty()
    }

    #[cfg(test)]
    pub fn running(&self) -> bool {
        self.running.is_some()
    }

    #[cfg(test)]
    pub fn agenda(&self) -> &Agenda {
        &self.agenda
    }

    #[cfg(test)]
    pub fn timers(&self) -> usize {
        self.timers.len()
    }

    #[cfg(test)]
    pub fn is_simulated(&self, coord: ChunkCoord) -> bool {
        self.simulated.contains_key(&coord)
    }

    /// How a step is divided: neither changes what it computes.
    #[cfg(test)]
    pub fn set_division(&mut self, evaluations: usize, jobs_in_flight: usize) {
        self.quota = Quota { evaluations };
        self.jobs_in_flight = jobs_in_flight;
    }

    /// The simulation's bounds: these decide when a Voxel under load changes.
    #[cfg(test)]
    pub fn set_bounds(&mut self, limits: Limits, landed_per_step: usize) {
        self.agenda = Agenda::new(limits);
        self.landed_per_step = landed_per_step;
    }

    #[cfg(test)]
    pub fn job(&mut self) -> &mut Stage<Advance, usize> {
        &mut self.job
    }

    fn simulates(&self, at: Cell) -> bool {
        self.simulated.contains_key(&VoxelCoord::of(at).chunk)
    }

    /// Wakes, at `step`, every simulated Voxel whose state reads `at`.
    fn wake_around(&mut self, at: Cell, step: Step) {
        for d in DEPENDENTS {
            let c = offset(at, d);
            if self.simulates(c) {
                self.agenda.wake(c, step);
            }
        }
    }

    /// Rescans what reads anything in `chunk`, inside it and around it.
    fn rescan_around(&mut self, chunk: ChunkCoord) {
        if self.simulated.contains_key(&chunk) {
            self.agenda.rescan(Span::chunk(key(chunk)));
        }
        self.rescan_readers(chunk);
    }

    /// Rescans the simulated Voxels outside `chunk` that read into it.
    fn rescan_readers(&mut self, chunk: ChunkCoord) {
        for span in readers_outside(key(chunk)) {
            if self.simulated.contains_key(&coord(span.home())) {
                self.agenda.rescan(span);
            }
        }
    }

    fn next_step(&mut self) {
        self.now += 1;
        self.edited.clear();
        self.edited_chunks.clear();
    }
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<FluidSim>()
        // After everything that changes the World on this tick, so the step that
        // starts reads all of it, and before `FixedUpdate` meshes what lands.
        .add_systems(
            FixedPreUpdate,
            simulate
                .after(edit::apply_pending)
                .after(super::mods::Drain),
        );
}

pub(in crate::server) fn simulate(
    mut sim: ResMut<FluidSim>,
    mut store: ResMut<ChunkStore>,
    mut meshes: ResMut<MeshState>,
    tables: Res<ContentTables>,
    world: Res<WorldSettings>,
) {
    let sim = &mut *sim;
    let scale = Arc::clone(
        sim.scale
            .get_or_insert_with(|| Arc::new(FlowScale::new(&tables.fluids, world.voxel_size))),
    );
    absorb_chunks(sim, &mut store, &meshes, &tables.fluids);
    absorb_changes(sim, &mut store);
    #[cfg(test)]
    if sim.running.is_some() && sim.stall > 0 {
        sim.stall -= 1;
        return;
    }
    if sim.running.is_some() {
        if !collect(sim, &tables.0, &scale) {
            return;
        }
        land_step(sim, &mut store, &mut meshes, &tables.0);
    } else {
        sim.next_step();
    }
    start_step(sim, &store, &tables.0, &scale);
}

/// Admits Chunks that have their light, drops those that left or came back as
/// a new Chunk, and notes which a light solve is pending for.
fn absorb_chunks(
    sim: &mut FluidSim,
    store: &mut ChunkStore,
    meshes: &MeshState,
    fluids: &FluidTable,
) {
    for coord in store.take_lifecycle() {
        if let Some(&life) = sim.simulated.get(&coord)
            && store.lifetime(coord) != Some(life)
        {
            evict(sim, store, coord, fluids);
        }
        if store.contains(coord) && !sim.simulated.contains_key(&coord) {
            sim.awaiting.insert(coord);
        }
    }
    let ready: Vec<ChunkCoord> = sim
        .awaiting
        .iter()
        .copied()
        .filter(|&c| !store.contains(c) || !meshes.is_unlit(c))
        .collect();
    for coord in ready {
        sim.awaiting.remove(&coord);
        if store.contains(coord) {
            admit(sim, store, coord, fluids);
        }
    }
    let unlit: BTreeSet<ChunkCoord> = meshes
        .unlit_chunks()
        .filter(|c| sim.simulated.contains_key(c))
        .collect();
    let resumed: Vec<ChunkCoord> = sim.suspended.difference(&unlit).copied().collect();
    for coord in resumed {
        sim.agenda.unpark(key(coord));
    }
    sim.suspended = unlit;
}

fn wet(store: &ChunkStore, coord: ChunkCoord, fluids: &FluidTable) -> bool {
    store
        .get(coord)
        .is_some_and(|c| c.kinds().iter().any(|v| fluids.contains(v.material())))
        || store.overlay(coord).is_some_and(|o| o.may_contain_fluid())
}

/// Whether any fluid is held close enough to `chunk` for its arrival or
/// departure to change something: what keeps dry terrain streaming in free.
fn wet_near(sim: &FluidSim, store: &ChunkStore, chunk: ChunkCoord, fluids: &FluidTable) -> bool {
    (-1..=1).any(|dx| {
        (-1..=1).any(|dy| {
            (-1..=1).any(|dz| {
                let c = ChunkCoord::new(chunk.x + dx, chunk.y + dy, chunk.z + dz);
                (c == chunk || sim.simulated.contains_key(&c)) && wet(store, c, fluids)
            })
        })
    })
}

/// Wakes both sides of a Chunk's boundary once it can be read, and the whole
/// of it if it holds fluid that may not be settled. Generated lakes are
/// rescanned once on arrival, then sleep like any settled water.
fn admit(sim: &mut FluidSim, store: &ChunkStore, coord: ChunkCoord, fluids: &FluidTable) {
    sim.simulated.insert(
        coord,
        store.lifetime(coord).expect("admitting a held Chunk"),
    );
    if !wet_near(sim, store, coord, fluids) {
        return;
    }
    let uniform = store.get(coord).is_some_and(|c| c.uniform().is_some())
        && store.overlay(coord).is_some_and(|o| o.uniform().is_some());
    if wet(store, coord, fluids) && !uniform {
        sim.agenda.rescan(Span::chunk(key(coord)));
    } else {
        for span in reading_outside(key(coord)) {
            sim.agenda.rescan(span);
        }
    }
    sim.rescan_readers(coord);
}

/// Drops a Chunk's waiting work and timers with it, and has what read it look
/// again: those Voxels now read unknown terrain, so they pause and cancel any
/// change they were waiting to make.
fn evict(sim: &mut FluidSim, store: &ChunkStore, coord: ChunkCoord, fluids: &FluidTable) {
    sim.simulated.remove(&coord);
    sim.suspended.remove(&coord);
    let home = key(coord);
    sim.agenda.forget(home);
    sim.timers.retain(|c| chunk_of(c).0 != home);
    sim.cursors.remove(&home);
    if wet_near(sim, store, coord, fluids) {
        sim.rescan_readers(coord);
    }
}

/// Wakes the Voxels around every World change since the last look, which the
/// running step's proposals must also not overwrite.
fn absorb_changes(sim: &mut FluidSim, store: &mut ChunkStore) {
    let next = sim.now + 1;
    let changed = store.take_changed();
    for at in changed.cells {
        if sim.edited.len() < EDITED_CAP {
            sim.edited.insert(at);
        } else {
            sim.edited_chunks.insert(VoxelCoord::of(at).chunk);
        }
        sim.wake_around(at, next);
    }
    for chunk in changed.chunks {
        sim.edited_chunks.insert(chunk);
        sim.rescan_around(chunk);
    }
}

/// Takes every slice that has come back and sends out more, up to the bound.
/// True once the whole step is back.
fn collect(sim: &mut FluidSim, tables: &Arc<Tables>, scale: &Arc<FlowScale>) -> bool {
    sim.job.poll();
    let running = sim.running.as_mut().expect("a step is running");
    let finished: Vec<usize> = sim.job.finished().collect();
    for i in finished {
        running.slices[i] = match sim.job.take(i).expect("finished") {
            Advance::Complete(done) => Slice::Done(done),
            Advance::Partial(cont) => Slice::Waiting(cont),
        };
    }
    dispatch(sim, tables, scale);
    let running = sim.running.as_ref().expect("a step is running");
    running.slices.iter().all(|s| matches!(s, Slice::Done(_)))
}

fn dispatch(sim: &mut FluidSim, tables: &Arc<Tables>, scale: &Arc<FlowScale>) {
    let FluidSim {
        running,
        job,
        quota,
        jobs_in_flight,
        ..
    } = sim;
    let Some(running) = running.as_mut() else {
        return;
    };
    for (i, slice) in running.slices.iter_mut().enumerate() {
        if job.in_flight() >= *jobs_in_flight {
            return;
        }
        if !matches!(slice, Slice::Waiting(_)) {
            continue;
        }
        let Slice::Waiting(cont) = std::mem::replace(slice, Slice::Out) else {
            unreachable!("matched above");
        };
        let region = Arc::clone(&running.region);
        let tables = Arc::clone(tables);
        let scale = Arc::clone(scale);
        let quota = *quota;
        job.spawn(i, move || advance(&*region, &tables, &scale, cont, quota));
    }
}

/// Starts step `now`: its due Voxels, a Snapshot of every simulated Chunk
/// they read, and their timers, divided into slices.
fn start_step(
    sim: &mut FluidSim,
    store: &ChunkStore,
    tables: &Arc<Tables>,
    scale: &Arc<FlowScale>,
) {
    let step = sim.now;
    let taken = sim.agenda.take_due(step);
    sim.scanned += taken.scanned;
    let due = taken.cells;
    if due.is_empty() {
        sim.agenda.end_step();
        return;
    }
    let mut region = Region::default();
    let mut read = HashMap::new();
    let mut seen = HashSet::new();
    for &at in &due {
        for d in READS {
            let (chunk, _) = chunk_of(offset(at, d));
            if !seen.insert(chunk) {
                continue;
            }
            let c = coord(chunk);
            let Some(&life) = sim.simulated.get(&c) else {
                continue;
            };
            let Some(snapshot) = store.shared(c).filter(|s| s.lifetime == life) else {
                continue;
            };
            read.insert(chunk, life);
            region.insert(chunk, snapshot.chunk, snapshot.overlay);
        }
    }
    let slices = due
        .chunks(sim.quota.evaluations.max(1))
        .map(|part| {
            let timers = sim.timers.split_off(part);
            Slice::Waiting(Continuation::start(step, part.to_vec(), timers))
        })
        .collect();
    sim.running = Some(Running {
        step,
        region: Arc::new(region),
        read,
        slices,
    });
    dispatch(sim, tables, scale);
}

/// Lands the finished step within its bound and ends it.
fn land_step(sim: &mut FluidSim, store: &mut ChunkStore, meshes: &mut MeshState, tables: &Tables) {
    let Running {
        step, read, slices, ..
    } = sim.running.take().expect("a step is running");
    let next = step + 1;
    let mut proposals = Vec::new();
    for slice in slices {
        let Slice::Done(done) = slice else {
            unreachable!("landing a step before all of it is back");
        };
        for (ready, at) in done.wakeups {
            if sim.simulates(at) {
                sim.agenda.rewake(at, ready);
            }
        }
        sim.timers.merge(done.timers);
        sim.work += done.work;
        proposals.extend(done.proposals);
    }
    // Timers of Voxels whose Chunk left while the step ran.
    let simulated = &sim.simulated;
    sim.timers
        .retain(|c| simulated.contains_key(&VoxelCoord::of(c).chunk));

    let (chosen, deferred) = choose(sim, proposals);
    for p in deferred {
        sim.deferred += 1;
        if sim.simulates(p.at) {
            sim.timers.hold(p.at, next, p.next.fluid);
            sim.agenda.rewake(p.at, next);
        }
    }
    for p in chosen {
        debug_assert!(
            !p.next.fluid.is_some_and(|f| f.source),
            "flow proposed a source"
        );
        let chunk = VoxelCoord::of(p.at).chunk;
        if !still_valid(sim, &read, store, tables, &p) {
            sim.stale += 1;
            if sim.simulates(p.at) {
                sim.agenda.rewake(p.at, next);
            }
            continue;
        }
        if sim.suspended.contains(&chunk) {
            sim.parked += 1;
            sim.agenda.park(p.at);
            continue;
        }
        if set_occupants(store, meshes, tables, p.at, p.next).is_ok() {
            sim.landed += 1;
        } else {
            sim.stale += 1;
            sim.agenda.rewake(p.at, next);
        }
    }
    let changed = store.take_changed();
    debug_assert!(changed.chunks.is_empty(), "a step outgrew the journal");
    for at in changed.cells {
        sim.wake_around(at, next);
    }
    sim.agenda.end_step();
    sim.next_step();
}

/// Whether nothing `p` read has changed since its Snapshot: no event touched
/// it, and every Chunk it read is the one it read, or still absent.
fn still_valid(
    sim: &FluidSim,
    read: &HashMap<[i32; 3], Lifetime>,
    store: &ChunkStore,
    tables: &Tables,
    p: &Proposal,
) -> bool {
    READS.iter().all(|&d| {
        let at = offset(p.at, d);
        let (chunk, _) = chunk_of(at);
        let c = coord(chunk);
        // Absent then and now is the same terrain: the kernel reads lazily, so
        // a Voxel beside a missing Chunk may still have decided.
        let same = read.get(&chunk) == sim.simulated.get(&c)
            && sim
                .simulated
                .get(&c)
                .is_none_or(|life| store.lifetime(c) == Some(*life));
        same && !sim.edited.contains(&at) && !sim.edited_chunks.contains(&c)
    }) && store.occupants(p.at, &tables.fluids) == Some(p.read)
}

/// Picks the changes a step lands: all of them within the bound; past it, one
/// per Chunk in turn, starting after the Chunk served last and, in each, from
/// where its last turn stopped, so every Chunk with work keeps landing it.
fn choose(sim: &mut FluidSim, proposals: Vec<Proposal>) -> (Vec<Proposal>, Vec<Proposal>) {
    let bound = sim.landed_per_step;
    if proposals.len() <= bound {
        sim.turn = None;
        sim.cursors.clear();
        return (proposals, Vec::new());
    }
    let mut by_chunk: BTreeMap<[i32; 3], Vec<Proposal>> = BTreeMap::new();
    for p in proposals {
        by_chunk.entry(chunk_of(p.at).0).or_default().push(p);
    }
    let mut queues: Vec<([i32; 3], VecDeque<Proposal>)> = by_chunk
        .into_iter()
        .map(|(c, mut ps)| {
            let from = sim
                .cursors
                .get(&c)
                .map_or(0, |cursor| ps.partition_point(|p| p.at < *cursor));
            ps.rotate_left(from);
            (c, ps.into())
        })
        .collect();
    let from = sim
        .turn
        .map_or(0, |t| queues.partition_point(|(c, _)| *c <= t));
    queues.rotate_left(from);
    let mut chosen = Vec::with_capacity(bound);
    'rounds: loop {
        let mut any = false;
        for (c, queue) in &mut queues {
            if chosen.len() == bound {
                break 'rounds;
            }
            if let Some(p) = queue.pop_front() {
                chosen.push(p);
                sim.turn = Some(*c);
                any = true;
            }
        }
        if !any {
            break;
        }
    }
    sim.cursors.clear();
    let mut deferred = Vec::new();
    for (c, queue) in queues {
        if let Some(first) = queue.front() {
            sim.cursors.insert(c, first.at);
        }
        deferred.extend(queue);
    }
    chosen.sort_unstable_by_key(|p| p.at);
    (chosen, deferred)
}

#[cfg(test)]
mod tests;
