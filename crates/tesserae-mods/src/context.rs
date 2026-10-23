//! What a Mod is told when it is asked for a batch.

use tesserae_core::{MaterialId, Voxel};

use crate::command::BatchReport;

/// What a player meant by an Edit.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EditKind {
    /// The solid layer, before and after: a Break or a Place.
    Solid {
        before: Voxel,
        after: Voxel,
    },
    SourcePlaced(MaterialId),
    SourceRemoved(MaterialId),
}

/// One player Edit since the last call, in metres.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct EditEvent {
    pub at: [f32; 3],
    pub kind: EditKind,
    /// The packed Voxel before and after, which is all a 0.1 Mod is shown.
    pub before: Voxel,
    pub after: Voxel,
}

impl EditEvent {
    /// Whether the 0.1 projection reports it: every solid Edit, and a fluid
    /// one only where it changed the packed Voxel.
    pub fn in_voxel_projection(&self) -> bool {
        matches!(self.kind, EditKind::Solid { .. }) || self.before != self.after
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct TickContext {
    pub tick: u64,
    pub dt_s: f32,
    pub edits: Vec<EditEvent>,
    pub last: BatchReport,
}
