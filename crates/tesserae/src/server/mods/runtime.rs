//! Mod calls own their guest on workers; only the tick lands and drains Commands.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use bevy::tasks::futures::check_ready;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use tesserae_core::Voxel;
use tesserae_mods::{
    BatchError, BatchReport, BoxCursor, CommandBatch, EditEvent, ModFault, ModInstance, Registry,
    Resolved, TickContext, validate,
};

use super::super::ContentTables;
use super::super::edit;
use super::super::streaming::MeshState;
use super::super::world_state::{ChunkStore, VoxelCoord};
use crate::world::WorldSettings;

const MOD_VOXELS_PER_TICK: usize = 2;
const MAX_BACKLOG_VOXELS: u64 = 262_144;
const MAX_EDIT_EVENTS: usize = 1_024;

#[derive(Message, Debug)]
pub(in crate::server) struct ModDisabled {
    pub name: String,
    pub reason: String,
}

#[derive(Resource, Default)]
pub(in crate::server) struct EditHistory {
    events: VecDeque<EditEvent>,
    next: u64,
    dropped: u64,
}

impl EditHistory {
    pub(in crate::server) fn record(&mut self, event: EditEvent) {
        if self.events.len() == MAX_EDIT_EVENTS {
            self.events.pop_front();
            self.dropped = self.dropped.saturating_add(1);
        }
        self.events.push_back(event);
        self.next += 1;
    }

    fn since(&self, cursor: &mut u64, dropped: &mut u64) -> Vec<EditEvent> {
        let first = self.next - self.events.len() as u64;
        *dropped = dropped.saturating_add(first.saturating_sub(*cursor));
        let start = cursor.saturating_sub(first) as usize;
        *cursor = self.next;
        self.events.iter().skip(start).copied().collect()
    }
}

struct Landed {
    instance: ModInstance,
    batch: Result<CommandBatch, BatchError>,
    commands: usize,
}

type Call = Result<Landed, ModFault>;

struct Slot {
    name: String,
    idle: Option<ModInstance>,
    running: Option<Task<Call>>,
    cursor: u64,
    dropped_events: u64,
    throttled: bool,
}

#[derive(Resource)]
pub(in crate::server) struct ModJobs {
    #[expect(
        dead_code,
        reason = "the ordered Registry is retained for save and Mod diagnostics"
    )]
    registry: Registry,
    // Stores are Send but not Sync; exclusive ECS access uses get_mut, never a lock.
    slots: Mutex<Vec<Slot>>,
    tick: u64,
}

impl ModJobs {
    pub(super) fn new(registry: Registry, instances: Vec<ModInstance>) -> Self {
        Self {
            registry,
            slots: Mutex::new(
                instances
                    .into_iter()
                    .map(|instance| Slot {
                        name: instance.id().to_owned(),
                        idle: Some(instance),
                        running: None,
                        cursor: 0,
                        dropped_events: 0,
                        throttled: false,
                    })
                    .collect(),
            ),
            tick: 0,
        }
    }
}

enum Queued {
    Fill(BoxCursor, Voxel),
    Place(std::vec::IntoIter<([i32; 3], Voxel)>),
}

impl Queued {
    fn peek(&self) -> Option<([i32; 3], Voxel)> {
        match self {
            Self::Fill(cursor, voxel) => {
                let mut cursor = *cursor;
                cursor.next().map(|at| (at, *voxel))
            }
            Self::Place(placements) => placements.as_slice().first().copied(),
        }
    }

    fn advance(&mut self) {
        match self {
            Self::Fill(cursor, _) => {
                cursor.next();
            }
            Self::Place(placements) => {
                placements.next();
            }
        }
    }
}

#[derive(Default)]
struct Queue {
    commands: VecDeque<Queued>,
    voxels: u64,
    report: BatchReport,
}

impl Queue {
    fn push(&mut self, batch: CommandBatch) {
        self.voxels += batch.voxels;
        for command in batch.commands {
            match command {
                Resolved::Fill { region, voxel } => {
                    self.commands
                        .push_back(Queued::Fill(region.cursor(), voxel));
                }
                Resolved::Place(placements) if !placements.is_empty() => {
                    self.commands
                        .push_back(Queued::Place(placements.into_iter()));
                }
                Resolved::Place(_) => {}
            }
        }
    }

    fn advance(&mut self) {
        let front = self.commands.front_mut().expect("a Voxel was peeked");
        front.advance();
        if front.peek().is_none() {
            self.commands.pop_front();
        }
        self.voxels -= 1;
    }
}

#[derive(Resource, Default)]
pub(in crate::server) struct PendingCommands {
    queues: Vec<Queue>,
    next: usize,
}

impl PendingCommands {
    fn queue(&mut self, index: usize) -> &mut Queue {
        if self.queues.len() <= index {
            self.queues.resize_with(index + 1, Queue::default);
        }
        &mut self.queues[index]
    }

    fn drain(
        &mut self,
        store: &mut ChunkStore,
        meshes: &mut MeshState,
        tables: &ContentTables,
        budget: usize,
    ) {
        let count = self.queues.len();
        if count == 0 {
            return;
        }
        let mut skipped = 0;
        let mut spent = 0;
        while spent < budget && skipped < count {
            let index = self.next;
            self.next = (index + 1) % count;
            let queue = &mut self.queues[index];
            let Some((at, voxel)) = queue.commands.front().and_then(Queued::peek) else {
                skipped += 1;
                continue;
            };
            // A bulk solve must settle before incremental edits can remove light safely.
            if store.contains(VoxelCoord::of(at).chunk) && meshes.is_unlit(VoxelCoord::of(at).chunk)
            {
                skipped += 1;
                continue;
            }
            let result = edit::set_voxel(store, meshes, tables, at, voxel);
            let counter = if result.is_ok() {
                &mut queue.report.applied
            } else {
                &mut queue.report.rejected
            };
            *counter = counter.saturating_add(1);
            queue.advance();
            spent += 1;
            skipped = 0;
        }
    }
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<EditHistory>()
        .init_resource::<PendingCommands>()
        .add_message::<ModDisabled>()
        .add_systems(FixedPreUpdate, land.before(edit::apply_pending))
        .add_systems(FixedPreUpdate, drain.after(edit::apply_pending).after(land))
        .add_systems(FixedPostUpdate, dispatch);
}

fn land(
    mut jobs: ResMut<ModJobs>,
    mut pending: ResMut<PendingCommands>,
    mut disabled: MessageWriter<ModDisabled>,
) {
    for (index, slot) in jobs
        .slots
        .get_mut()
        .expect("no locking or poisoning")
        .iter_mut()
        .enumerate()
    {
        let Some(result) = slot.running.as_mut().and_then(check_ready) else {
            continue;
        };
        slot.running = None;
        match result {
            Ok(landed) => {
                slot.idle = Some(landed.instance);
                let queue = pending.queue(index);
                match landed.batch {
                    Ok(batch) => queue.push(batch),
                    Err(error) => {
                        let refused = BatchReport::refused(landed.commands);
                        queue.report.rejected =
                            queue.report.rejected.saturating_add(refused.rejected);
                        warn!("mod `{}`: refused batch: {error}", slot.name);
                    }
                }
            }
            Err(fault) => {
                let message = ModDisabled {
                    name: slot.name.clone(),
                    reason: fault.to_string(),
                };
                warn!("mod `{}` disabled: {}", message.name, message.reason);
                disabled.write(message);
            }
        }
    }
}

fn drain(
    mut pending: ResMut<PendingCommands>,
    mut store: ResMut<ChunkStore>,
    mut meshes: ResMut<MeshState>,
    tables: Res<ContentTables>,
) {
    pending.drain(&mut store, &mut meshes, &tables, MOD_VOXELS_PER_TICK);
}

fn dispatch(
    mut jobs: ResMut<ModJobs>,
    mut pending: ResMut<PendingCommands>,
    history: Res<EditHistory>,
    tables: Res<ContentTables>,
    world: Res<WorldSettings>,
    time: Res<Time<Fixed>>,
) {
    jobs.tick += 1;
    let tick = jobs.tick;
    for (index, slot) in jobs
        .slots
        .get_mut()
        .expect("no locking or poisoning")
        .iter_mut()
        .enumerate()
    {
        if slot.idle.is_none() {
            continue;
        }
        let queue = pending.queue(index);
        let throttled = queue.voxels > MAX_BACKLOG_VOXELS;
        if throttled && !slot.throttled {
            warn!(
                "mod `{}` paused at {} queued Voxels",
                slot.name, queue.voxels
            );
        }
        slot.throttled = throttled;
        if throttled {
            continue;
        }
        let instance = slot.idle.take().expect("an idle Mod");
        let context = TickContext {
            tick,
            dt_s: time.timestep().as_secs_f32(),
            edits: history.since(&mut slot.cursor, &mut slot.dropped_events),
            last: std::mem::take(&mut queue.report),
        };
        let tables = Arc::clone(&tables.0);
        let size = world.voxel_size;
        slot.running = Some(AsyncComputeTaskPool::get().spawn(async move {
            let (instance, commands) = instance.tick(context).await?;
            let batch = validate(&commands, &tables, size);
            Ok(Landed {
                instance,
                batch,
                commands: commands.len(),
            })
        }));
    }
}

#[cfg(test)]
pub(in crate::server) mod tests;
