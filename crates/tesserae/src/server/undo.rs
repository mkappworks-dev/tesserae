//! Bounded player-edit inverses, applied through the normal World change path.
use super::{
    ContentTables,
    edit::{self, EditRejected, RejectReason},
    mods::EditHistory,
    streaming::MeshState,
    world_state::{ChunkStore, Lifetime, VoxelCoord},
};
use bevy::prelude::*;
use std::collections::VecDeque;
use tesserae_core::Occupants;
use tesserae_mods::{EditEvent, EditKind};

const LIMIT: usize = 128;
const REQUEST_LIMIT: usize = 16;
/// Voxels the journal holds across all groups, so layouts cannot make 128
/// entries cost 128 times 256 records.
const RECORDED_LIMIT: usize = 1024;

#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryAction {
    Undo,
    Redo,
}
#[derive(Message, Clone, Copy, Debug)]
pub struct HistoryApplied(pub HistoryAction);

#[derive(Clone, Copy)]
pub(super) struct Entry {
    pub at: [i32; 3],
    pub lifetime: Lifetime,
    pub before: Occupants,
    pub after: Occupants,
    pub event: EditEvent,
}
/// The Voxels one Edit or one layout changed, undone and redone together.
type Group = Vec<Entry>;

#[derive(Resource, Default)]
pub(super) struct Journal {
    undo: VecDeque<Group>,
    redo: VecDeque<Group>,
    requests: VecDeque<HistoryAction>,
}
impl Journal {
    #[cfg(test)]
    pub(super) fn is_empty(&self) -> bool {
        self.undo.is_empty() && self.redo.is_empty()
    }

    pub(super) fn cancel_requests(&mut self) {
        self.requests.clear();
    }

    pub(super) fn record(&mut self, entry: Entry) {
        self.record_group(vec![entry]);
    }

    pub(super) fn record_group(&mut self, mut group: Group) {
        group.retain(|entry| entry.before != entry.after);
        if group.is_empty() {
            return;
        }
        if self.undo.len() == LIMIT {
            self.undo.pop_front();
        }
        self.undo.push_back(group);
        while self.undo.iter().map(Vec::len).sum::<usize>() > RECORDED_LIMIT && self.undo.len() > 1
        {
            self.undo.pop_front();
        }
        self.redo.clear();
    }
}
pub(super) fn receive(
    mut messages: MessageReader<HistoryAction>,
    mut journal: ResMut<Journal>,
    mut rejected: MessageWriter<EditRejected>,
) {
    for &action in messages.read() {
        if journal.requests.len() == REQUEST_LIMIT {
            rejected.write(EditRejected {
                reason: RejectReason::Busy,
            });
        } else {
            journal.requests.push_back(action);
        }
    }
}
pub(super) fn apply(
    mut journal: ResMut<Journal>,
    pending: Res<edit::PendingEdits>,
    mut store: ResMut<ChunkStore>,
    mut meshes: ResMut<MeshState>,
    tables: Res<ContentTables>,
    mut history: ResMut<EditHistory>,
    (mut rejected, mut applied): (MessageWriter<EditRejected>, MessageWriter<HistoryApplied>),
) {
    let Some(action) = journal.requests.pop_front() else {
        return;
    };
    let group = match action {
        HistoryAction::Undo => journal.undo.back(),
        HistoryAction::Redo => journal.redo.back(),
    }
    .cloned();
    let Some(group) = group else {
        rejected.write(EditRejected {
            reason: if action == HistoryAction::Undo {
                RejectReason::NoUndo
            } else {
                RejectReason::NoRedo
            },
        });
        return;
    };
    let result = if !pending.is_empty() {
        Err(RejectReason::Busy)
    } else {
        restore(&mut store, &mut meshes, &tables, &group, action)
    };
    match result {
        Ok(()) => {
            // The Mods hear the changes in the order they were made, or in
            // reverse when they are taken back.
            let mut events: Vec<EditEvent> = group.iter().map(|entry| entry.event).collect();
            if action == HistoryAction::Undo {
                events.reverse();
                for event in &mut events {
                    std::mem::swap(&mut event.before, &mut event.after);
                    event.kind = inverse(event.kind);
                }
                journal.undo.pop_back();
                journal.redo.push_back(group);
            } else {
                journal.redo.pop_back();
                journal.undo.push_back(group);
            }
            for event in events {
                history.record(event);
            }
            applied.write(HistoryApplied(action));
        }
        Err(reason) => {
            rejected.write(EditRejected { reason });
        }
    }
}
/// Replays a whole group or none of it: every cell is checked before the first
/// is changed, and a change that fails midway is taken back.
fn restore(
    store: &mut ChunkStore,
    meshes: &mut MeshState,
    tables: &tesserae_core::Tables,
    group: &[Entry],
    action: HistoryAction,
) -> Result<(), RejectReason> {
    let undoing = action == HistoryAction::Undo;
    let step = |entry: &Entry| {
        if undoing {
            (entry.after, entry.before)
        } else {
            (entry.before, entry.after)
        }
    };
    for entry in group {
        let chunk = VoxelCoord::of(entry.at).chunk;
        let lifetime = store.lifetime(chunk).ok_or(RejectReason::NotLoaded)?;
        if lifetime != entry.lifetime {
            return Err(RejectReason::HistoryChanged);
        }
        if meshes.is_unlit(chunk) {
            return Err(RejectReason::Busy);
        }
        if store.occupants(entry.at, &tables.fluids) != Some(step(entry).0) {
            return Err(RejectReason::HistoryChanged);
        }
    }
    let order: Vec<&Entry> = if undoing {
        group.iter().rev().collect()
    } else {
        group.iter().collect()
    };
    for (done, entry) in order.iter().enumerate() {
        if let Err(reason) = edit::set_occupants(store, meshes, tables, entry.at, step(entry).1) {
            for undone in order[..done].iter().rev() {
                let _ = edit::set_occupants(store, meshes, tables, undone.at, step(undone).0);
            }
            return Err(reason);
        }
    }
    Ok(())
}
fn inverse(kind: EditKind) -> EditKind {
    match kind {
        EditKind::Solid { before, after } => EditKind::Solid {
            before: after,
            after: before,
        },
        EditKind::SourcePlaced(id) => EditKind::SourceRemoved(id),
        EditKind::SourceRemoved(id) => EditKind::SourcePlaced(id),
        EditKind::Ignited(id) => EditKind::PutOut(id),
        EditKind::PutOut(id) => EditKind::Ignited(id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_undo_history_and_pending_edits_do_not_outlive_their_world() {
        use crate::session::testing::{app, enter, leave};
        let mut app = app();
        app.add_plugins(super::super::edit::plugin);
        enter(&mut app);
        app.world_mut()
            .resource_mut::<Journal>()
            .requests
            .push_back(HistoryAction::Undo);
        leave(&mut app);
        assert!(!app.world().contains_resource::<Journal>());
        assert!(
            !app.world()
                .contains_resource::<super::super::edit::PendingEdits>()
        );
        enter(&mut app);
        assert!(app.world().resource::<Journal>().requests.is_empty());
        assert!(
            app.world()
                .resource::<super::super::edit::PendingEdits>()
                .is_empty()
        );
    }
    #[test]
    fn entries_and_requests_are_bounded_and_fresh_edits_clear_redo() {
        use bevy::ecs::system::RunSystemOnce;
        use tesserae_core::{Chunk, Voxel};
        let mut store = ChunkStore::default();
        let coord = VoxelCoord::of([0, 0, 0]).chunk;
        store.insert(coord, Chunk::air());
        let entry = Entry {
            at: [0, 0, 0],
            lifetime: store.lifetime(coord).unwrap(),
            before: Occupants {
                solid: Voxel::AIR,
                fluid: None,
                burning: None,
            },
            after: Occupants {
                solid: Voxel::cube(8),
                fluid: None,
                burning: None,
            },
            event: EditEvent {
                at: [0.0; 3],
                before: Voxel::AIR,
                after: Voxel::cube(8),
                kind: EditKind::Solid {
                    before: Voxel::AIR,
                    after: Voxel::cube(8),
                },
            },
        };
        let mut journal = Journal::default();
        for _ in 0..LIMIT + 10 {
            journal.record(entry);
        }
        assert_eq!(journal.undo.len(), LIMIT);
        journal.redo.push_back(vec![entry]);
        journal.record(entry);
        assert!(journal.redo.is_empty());
        let mut app = App::new();
        app.insert_resource(journal)
            .add_message::<HistoryAction>()
            .add_message::<EditRejected>();
        for _ in 0..REQUEST_LIMIT + 4 {
            app.world_mut().write_message(HistoryAction::Undo);
        }
        app.world_mut().run_system_once(receive).unwrap();
        assert_eq!(
            app.world().resource::<Journal>().requests.len(),
            REQUEST_LIMIT
        );
        assert_eq!(
            app.world_mut()
                .resource_mut::<Messages<EditRejected>>()
                .drain()
                .count(),
            4
        );
    }

    #[test]
    fn the_journal_bounds_recorded_voxels_across_groups_and_drops_empty_ones() {
        use tesserae_core::{Chunk, Voxel};
        let mut store = ChunkStore::default();
        let coord = VoxelCoord::of([0, 0, 0]).chunk;
        store.insert(coord, Chunk::air());
        let solid = |voxel| Occupants {
            solid: voxel,
            fluid: None,
            burning: None,
        };
        let entry = |before: Voxel, after: Voxel| Entry {
            at: [0, 0, 0],
            lifetime: store.lifetime(coord).unwrap(),
            before: solid(before),
            after: solid(after),
            event: EditEvent {
                at: [0.0; 3],
                before,
                after,
                kind: EditKind::Solid { before, after },
            },
        };
        let mut journal = Journal::default();
        journal.record_group(vec![entry(Voxel::AIR, Voxel::AIR); 4]);
        assert!(journal.undo.is_empty(), "a group that changed nothing");
        for _ in 0..RECORDED_LIMIT / 256 + 1 {
            journal.record_group(vec![entry(Voxel::AIR, Voxel::cube(8)); 256]);
        }
        assert_eq!(journal.undo.len(), RECORDED_LIMIT / 256);
    }
}
