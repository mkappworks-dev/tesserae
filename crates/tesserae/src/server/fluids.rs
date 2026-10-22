//! Running `tesserae_core::flow` on workers and landing what it proposes.
//!
//! One logical step at a time. A step's due Voxels go to a worker with shared
//! Snapshots of the Chunks they read; a quota-bounded call hands back a
//! continuation, which goes straight out again on the next tick. Only a whole
//! step's proposals land, through `edit::set_occupants`, and the step after it
//! starts once they all have: so how often the quota cut a step, and when its
//! worker finished, change when it lands and never what lands.
//!
//! Every World change, an Edit's or a landed proposal's, is journalled by the
//! store, and that journal is what wakes the Voxels around it: a change made
//! while step `t` is current is first read by step `t + 1`. At most one step
//! completes per tick, so a quiet Server simulates in real time and a busy one
//! slows down rather than skipping.
//!
//! A proposal is dropped and its Voxel woken again if anything it read changed
//! by other means since its Snapshot: an Edit near it, or a new Chunk at its
//! coordinate. The kernel never proposes a source, so a stale result cannot
//! bring back one an Edit removed.

use std::collections::VecDeque;
use std::sync::Arc;

use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
use tesserae_core::Tables;
use tesserae_core::flow::{
    Advance, Cell, Continuation, FlowScale, Proposal, Quota, READS, Region, Step, Timers, Wakeups,
    Work, advance, chunk_of,
};

use super::ContentTables;
use super::edit::{self, set_occupants};
use super::jobs::Stage;
use super::streaming::MeshState;
use super::world_state::{ChunkStore, Lifetime, VoxelCoord};
use crate::world::{ChunkCoord, WorldSettings};

/// Evaluations one worker call may make before it hands back a continuation.
const EVALUATIONS_PER_JOB: usize = 1024;

/// Proposals landed per tick. Each relights where water appears or goes, so
/// this bounds a tick; the rest of the step lands on the ticks after.
const LANDED_PER_TICK: usize = 64;

#[derive(Resource)]
pub(in crate::server) struct FluidSim {
    /// The logical step being evaluated or landed, or the next to start.
    now: Step,
    wakeups: Wakeups,
    /// Held here between steps and by the worker during one.
    timers: Option<Timers>,
    job: Stage<Advance, ()>,
    landing: VecDeque<Proposal>,
    /// Edits since the running step's first Snapshot, which its proposals did
    /// not see.
    edited: HashSet<Cell>,
    /// The lifetime of every Chunk the running step read, as first read.
    read: HashMap<ChunkCoord, Lifetime>,
    scale: Option<Arc<FlowScale>>,
    quota: Quota,
    landed_per_tick: usize,
    pub work: Work,
    pub landed: usize,
    /// Proposals dropped because what they read had changed.
    pub stale: usize,
    /// Proposals put off because their Chunk's light was being solved.
    pub deferred: usize,
}

impl Default for FluidSim {
    fn default() -> Self {
        FluidSim {
            now: 0,
            wakeups: Wakeups::default(),
            timers: Some(Timers::default()),
            job: Stage::default(),
            landing: VecDeque::new(),
            edited: HashSet::default(),
            read: HashMap::default(),
            scale: None,
            quota: Quota {
                evaluations: EVALUATIONS_PER_JOB,
            },
            landed_per_tick: LANDED_PER_TICK,
            work: Work::default(),
            landed: 0,
            stale: 0,
            deferred: 0,
        }
    }
}

impl FluidSim {
    #[cfg(test)]
    pub fn now(&self) -> Step {
        self.now
    }

    /// Whether nothing is waiting, running or landing.
    #[cfg(test)]
    pub fn is_idle(&self) -> bool {
        self.wakeups.is_empty() && self.job.in_flight() == 0 && self.landing.is_empty()
    }

    fn stepping(&self) -> bool {
        self.job.in_flight() > 0 || !self.landing.is_empty()
    }

    #[cfg(test)]
    pub fn set_quota(&mut self, evaluations: usize, landed_per_tick: usize) {
        self.quota = Quota { evaluations };
        self.landed_per_tick = landed_per_tick;
    }

    #[cfg(test)]
    pub fn job(&mut self) -> &mut Stage<Advance, ()> {
        &mut self.job
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

/// Wakes the Voxels around every change since the last look. Edits made while
/// a step is running are also what its proposals must not overwrite.
fn absorb(sim: &mut FluidSim, store: &mut ChunkStore, edits: bool) {
    let next = sim.now + 1;
    let stepping = sim.stepping();
    for at in store.take_changed() {
        sim.wakeups.wake_around(at, next);
        if edits && stepping {
            sim.edited.insert(at);
        }
    }
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
    absorb(sim, &mut store, true);

    // At most one step completes per tick.
    let mut finished = false;
    if sim.job.in_flight() > 0 {
        sim.job.poll();
        let Some(result) = sim.job.take(()) else {
            return;
        };
        match result {
            Advance::Partial(cont) => {
                dispatch(sim, &store, &tables.0, &scale, cont);
                return;
            }
            Advance::Complete(done) => {
                for (step, at) in done.wakeups {
                    sim.wakeups.wake(at, step);
                }
                sim.timers = Some(done.timers);
                sim.work += done.work;
                sim.landing.extend(done.proposals);
                if sim.landing.is_empty() {
                    finish_step(sim);
                    finished = true;
                }
            }
        }
    }

    if !sim.landing.is_empty() {
        land(sim, &mut store, &mut meshes, &tables.0);
        absorb(sim, &mut store, false);
        if !sim.landing.is_empty() {
            return;
        }
        finish_step(sim);
        finished = true;
    }

    let due = sim.wakeups.take_due(sim.now);
    if due.is_empty() {
        if !finished {
            sim.now += 1;
        }
        return;
    }
    let timers = sim.timers.take().expect("held between steps");
    let cont = Continuation::start(sim.now, due, timers);
    dispatch(sim, &store, &tables.0, &scale, cont);
}

fn finish_step(sim: &mut FluidSim) {
    sim.now += 1;
    sim.edited.clear();
    sim.read.clear();
}

/// Hands the next part of a step to a worker, with every Chunk it reads.
fn dispatch(
    sim: &mut FluidSim,
    store: &ChunkStore,
    tables: &Arc<Tables>,
    scale: &Arc<FlowScale>,
    cont: Continuation,
) {
    let mut region = Region::default();
    let mut last = None;
    for at in cont.upcoming(sim.quota) {
        let (chunk, _) = chunk_of(at);
        if last == Some(chunk) || region.contains(chunk) {
            continue;
        }
        last = Some(chunk);
        let coord = ChunkCoord::new(chunk[0], chunk[1], chunk[2]);
        let Some(snapshot) = store.shared(coord) else {
            continue;
        };
        sim.read.entry(coord).or_insert(snapshot.lifetime);
        region.insert(chunk, snapshot.chunk, snapshot.overlay);
    }
    let tables = Arc::clone(tables);
    let scale = Arc::clone(scale);
    let quota = sim.quota;
    sim.job
        .spawn((), move || advance(&region, &tables, &scale, cont, quota));
}

/// Lands up to the per-tick cap of the step's proposals, in the order the step
/// made them.
fn land(sim: &mut FluidSim, store: &mut ChunkStore, meshes: &mut MeshState, tables: &Tables) {
    let next = sim.now + 1;
    for _ in 0..sim.landed_per_tick {
        let Some(p) = sim.landing.pop_front() else {
            return;
        };
        debug_assert!(
            !p.next.fluid.is_some_and(|f| f.source),
            "flow proposed a source"
        );
        let chunk = VoxelCoord::of(p.at).chunk;
        if meshes.is_unlit(chunk) {
            sim.deferred += 1;
            sim.wakeups.wake(p.at, next);
            continue;
        }
        let read_elsewhere = READS.iter().any(|d| {
            sim.edited
                .contains(&[p.at[0] + d[0], p.at[1] + d[1], p.at[2] + d[2]])
        });
        let reborn = sim.read.get(&chunk).copied() != store.lifetime(chunk);
        if read_elsewhere || reborn || store.occupants(p.at, &tables.fluids) != Some(p.read) {
            sim.stale += 1;
            sim.wakeups.wake(p.at, next);
            continue;
        }
        if set_occupants(store, meshes, tables, p.at, p.next).is_ok() {
            sim.landed += 1;
        } else {
            sim.stale += 1;
        }
    }
}

#[cfg(test)]
mod tests;
