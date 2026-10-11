//! Places a plan of Voxels through the edit path, a few a tick. `dev` only.
//!
//! What the `dev` structures have in common: they stand near the spawn point so
//! the default camera frames them, they wait for the Chunks they fall in to be
//! loaded and lit, and they place through [`edit::set_voxel`] - the function a
//! player's click ends in - rather than writing the store.

use bevy::prelude::*;
use tesserae_core::{ShapeTable, Voxel};

use super::edit;
use super::streaming::MeshState;
use super::world_state::{ChunkStore, VoxelCoord};

/// Voxels placed per tick.
///
/// A plan is hundreds of Voxels and each one re-floods light, so placing it all
/// in one tick costs more than a frame. A budget for the same reason
/// `StreamingBudget` is one - and the cost is this tool building a building, not
/// what a single edit costs: that is bounded by the 15 Voxels light reaches.
const EDITS_PER_TICK: usize = 24;

/// How far through its plan a builder has got.
#[derive(Default)]
pub(super) struct Progress {
    placed: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Step {
    /// Nothing to do: still waiting for Chunks, or already finished.
    Idle,
    Placed,
    /// This call placed the last Voxel.
    Completed,
}

/// Places the next few Voxels of `plan` once every Chunk it falls in is loaded
/// and its light has settled.
///
/// Waiting on the light is not politeness: an edit re-floods incrementally, and
/// the bulk solve `streaming::relight` runs for a Chunk that is still `unlit`
/// would land afterwards and overwrite that work.
pub(super) fn step(
    plan: &[([i32; 3], Voxel)],
    progress: &mut Progress,
    store: &mut ChunkStore,
    meshes: &mut MeshState,
    shapes: &ShapeTable,
) -> Step {
    if progress.placed >= plan.len() {
        return Step::Idle;
    }
    let ready = plan.iter().all(|(at, _)| {
        let chunk = VoxelCoord::of(*at).chunk;
        store.contains(chunk) && !meshes.is_unlit(chunk)
    });
    if !ready {
        return Step::Idle;
    }

    let until = (progress.placed + EDITS_PER_TICK).min(plan.len());
    for (at, voxel) in &plan[progress.placed..until] {
        if let Err(reason) = edit::set_voxel(store, meshes, shapes, *at, *voxel) {
            warn!("{at:?}: {}", reason.message());
        }
    }
    progress.placed = until;
    if until == plan.len() {
        Step::Completed
    } else {
        Step::Placed
    }
}
