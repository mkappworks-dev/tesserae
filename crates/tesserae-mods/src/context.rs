//! What a Mod is told when it is asked for a batch.

use tesserae_core::Voxel;

use crate::command::BatchReport;

/// One player Edit since the last call, in metres.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct EditEvent {
    pub at: [f32; 3],
    pub before: Voxel,
    pub after: Voxel,
}

#[derive(Clone, PartialEq, Debug)]
pub struct TickContext {
    pub tick: u64,
    pub dt_s: f32,
    pub edits: Vec<EditEvent>,
    pub last: BatchReport,
}
