//! Streaming work that runs off the tick.
//!
//! A job is one Chunk's worth of one stage. It is handed shared references and
//! returns an owned value, so no worker ever holds a `&mut` into World state;
//! `super::streaming` lands the result by calling the Server-private mutators on
//! its own tick. See `docs/adr/0002-client-server-from-day-one.md`.

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::tasks::futures::check_ready;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use tesserae_core::{Chunk, ChunkLight, ChunkMesh, ShapeTable, Worldgen};

use crate::world::ChunkCoord;

// A job closes over these by value or by `Arc`, so a field that stops being
// `Send + Sync` should fail here and not at a spawn site in `streaming`.
const _: () = {
    const fn shareable<T: Send + Sync + 'static>() {}
    shareable::<Chunk>();
    shareable::<ChunkLight>();
    shareable::<ChunkMesh>();
    shareable::<ShapeTable>();
    shareable::<Worldgen>();
};

/// The jobs of one stage: running on a worker, or finished and waiting to land.
///
/// Both count as in flight. A result that has finished but not landed is still a
/// snapshot held and a decision not yet applied.
pub(in crate::server) struct Stage<T> {
    running: HashMap<ChunkCoord, Task<T>>,
    finished: HashMap<ChunkCoord, T>,
    dispatched: usize,
    landed: usize,
}

impl<T> Default for Stage<T> {
    fn default() -> Self {
        Stage {
            running: HashMap::default(),
            finished: HashMap::default(),
            dispatched: 0,
            landed: 0,
        }
    }
}

impl<T: Send + 'static> Stage<T> {
    pub fn in_flight(&self) -> usize {
        self.running.len() + self.finished.len()
    }

    pub fn contains(&self, coord: ChunkCoord) -> bool {
        self.running.contains_key(&coord) || self.finished.contains_key(&coord)
    }

    /// Runs `work` on the async compute pool.
    ///
    /// One job per coord: a second dispatch for a coord already in flight is a
    /// bug in the caller's bookkeeping, and silently replacing the first would
    /// hide it.
    pub fn spawn(&mut self, coord: ChunkCoord, work: impl FnOnce() -> T + Send + 'static) {
        debug_assert!(!self.contains(coord), "{coord} already has a job in flight");
        let task = AsyncComputeTaskPool::get().spawn(async move { work() });
        self.running.insert(coord, task);
        self.dispatched += 1;
    }

    /// Moves every job that has finished to the waiting set, without blocking.
    pub fn poll(&mut self) {
        let mut done = Vec::new();
        for (&coord, task) in &mut self.running {
            if let Some(out) = check_ready(task) {
                done.push((coord, out));
            }
        }
        for (coord, out) in done {
            self.running.remove(&coord);
            self.finished.insert(coord, out);
        }
    }

    /// The coords whose results are waiting to land.
    pub fn finished(&self) -> impl Iterator<Item = ChunkCoord> + '_ {
        self.finished.keys().copied()
    }

    pub fn take(&mut self, coord: ChunkCoord) -> Option<T> {
        let out = self.finished.remove(&coord)?;
        self.landed += 1;
        Some(out)
    }

    /// Drops every job, running or finished, for which `keep` is false. A
    /// running job is cancelled if it has not started; one that has runs to
    /// completion with nowhere to deliver.
    pub fn retain(&mut self, mut keep: impl FnMut(ChunkCoord) -> bool) {
        self.running.retain(|&c, _| keep(c));
        self.finished.retain(|&c, _| keep(c));
    }

    /// Jobs dispatched and landed since the last [`Stage::reset_counts`].
    #[cfg(test)]
    pub fn counts(&self) -> (usize, usize) {
        (self.dispatched, self.landed)
    }

    pub fn reset_counts(&mut self) {
        self.dispatched = 0;
        self.landed = 0;
    }
}

/// Every job in flight, by stage.
#[derive(Resource, Default)]
pub(in crate::server) struct Jobs {
    pub generate: Stage<Chunk>,
    // Landing for these arrives in later tickets; the stages exist so the type
    // the tests and the overlay read does not change shape three times.
    pub relight: Stage<ChunkLight>,
    pub mesh: Stage<ChunkMesh>,
}

impl Jobs {
    #[cfg(any(test, feature = "dev"))]
    pub fn in_flight(&self) -> usize {
        self.generate.in_flight() + self.relight.in_flight() + self.mesh.in_flight()
    }

    pub fn poll(&mut self) {
        self.generate.poll();
        self.relight.poll();
        self.mesh.poll();
    }

    pub fn reset_counts(&mut self) {
        self.generate.reset_counts();
        self.relight.reset_counts();
        self.mesh.reset_counts();
    }
}
