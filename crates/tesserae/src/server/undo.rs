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
#[derive(Resource, Default)]
pub(super) struct Journal {
    undo: VecDeque<Entry>,
    redo: VecDeque<Entry>,
    requests: VecDeque<HistoryAction>,
}
impl Journal {
    pub(super) fn record(&mut self, entry: Entry) {
        if entry.before == entry.after {
            return;
        }
        if self.undo.len() == LIMIT {
            self.undo.pop_front();
        }
        self.undo.push_back(entry);
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
    let entry = match action {
        HistoryAction::Undo => journal.undo.back(),
        HistoryAction::Redo => journal.redo.back(),
    }
    .copied();
    let Some(entry) = entry else {
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
        restore(&mut store, &mut meshes, &tables, entry, action)
    };
    match result {
        Ok(()) => {
            let mut event = entry.event;
            if action == HistoryAction::Undo {
                std::mem::swap(&mut event.before, &mut event.after);
                event.kind = inverse(event.kind);
                journal.undo.pop_back();
                journal.redo.push_back(entry);
            } else {
                journal.redo.pop_back();
                journal.undo.push_back(entry);
            }
            history.record(event);
            applied.write(HistoryApplied(action));
        }
        Err(reason) => {
            rejected.write(EditRejected { reason });
        }
    }
}
fn restore(
    store: &mut ChunkStore,
    meshes: &mut MeshState,
    tables: &tesserae_core::Tables,
    entry: Entry,
    action: HistoryAction,
) -> Result<(), RejectReason> {
    let chunk = VoxelCoord::of(entry.at).chunk;
    let lifetime = store.lifetime(chunk).ok_or(RejectReason::NotLoaded)?;
    if lifetime != entry.lifetime {
        return Err(RejectReason::HistoryChanged);
    }
    if meshes.is_unlit(chunk) {
        return Err(RejectReason::Busy);
    }
    let (expected, replacement) = if action == HistoryAction::Undo {
        (entry.after, entry.before)
    } else {
        (entry.before, entry.after)
    };
    if store.occupants(entry.at, &tables.fluids) != Some(expected) {
        return Err(RejectReason::HistoryChanged);
    }
    edit::set_occupants(store, meshes, tables, entry.at, replacement)
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
        journal.redo.push_back(entry);
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
}
