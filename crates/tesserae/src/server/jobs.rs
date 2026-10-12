//! Streaming work that runs off the tick.
//!
//! A job is one Chunk's worth of one stage. It is handed shared references and
//! returns an owned value, so no worker ever holds a `&mut` into World state;
//! `super::streaming` lands the result by calling the Server-private mutators on
//! its own tick. See `docs/adr/0002-client-server-from-day-one.md`.

use std::sync::Arc;
#[cfg(test)]
use std::sync::atomic::{AtomicBool, Ordering};

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::tasks::futures::check_ready;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use tesserae_core::light::{Light, MAX_LIGHT, light_chunk};
use tesserae_core::{Chunk, ChunkLight, ChunkMesh, LitChunk, ShapeTable, Worldgen};

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
    /// Results that finished and were thrown away at landing because what they
    /// were computed from had changed.
    #[cfg(any(test, feature = "dev"))]
    stale: usize,
    /// While true, a job does not start its work, so a test can change the World
    /// under a job it knows is in flight.
    #[cfg(test)]
    pub hold: Arc<AtomicBool>,
    /// Finishes jobs in an order that has nothing to do with dispatch order.
    #[cfg(test)]
    pub scramble: bool,
}

impl<T> Default for Stage<T> {
    fn default() -> Self {
        Stage {
            running: HashMap::default(),
            finished: HashMap::default(),
            dispatched: 0,
            landed: 0,
            #[cfg(any(test, feature = "dev"))]
            stale: 0,
            #[cfg(test)]
            hold: Arc::default(),
            #[cfg(test)]
            scramble: false,
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
        #[cfg(test)]
        let work = {
            let hold = Arc::clone(&self.hold);
            let delay_us = if self.scramble { scramble_us(coord) } else { 0 };
            move || {
                while hold.load(Ordering::Acquire) {
                    std::thread::sleep(std::time::Duration::from_micros(50));
                }
                std::thread::sleep(std::time::Duration::from_micros(delay_us));
                work()
            }
        };
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

    /// Records a result thrown away at landing.
    pub fn discard(&mut self) {
        #[cfg(any(test, feature = "dev"))]
        {
            self.stale += 1;
        }
    }

    #[cfg(test)]
    pub fn stale(&self) -> usize {
        self.stale
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

/// A pseudo-random delay of up to 400 us, derived from the coord alone.
#[cfg(test)]
fn scramble_us(coord: ChunkCoord) -> u64 {
    let h = (coord.x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (coord.y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
        ^ (coord.z as u64).wrapping_mul(0x1656_67B1_9E37_79F9);
    (h >> 20) % 400
}

/// What a relight job reads of one face neighbour.
pub(in crate::server) enum Border {
    /// Not loaded, so a dark boundary.
    Absent,
    /// Above the band the surface can reach, where the generator makes only air
    /// and everything sees full sky. The relaxation cannot discover this: there
    /// is no neighbour to ask.
    OpenSky,
    Loaded(Arc<Chunk>, Arc<ChunkLight>),
}

/// The Voxels of the synthetic Chunk above the surface: air, which is all the
/// generator makes there.
static SKY_CHUNK: Chunk = Chunk::air();

/// A finished light solve.
pub(in crate::server) struct Solved {
    /// The instance that was solved, so landing can tell it is still the one in
    /// the store and has not been evicted and regenerated meanwhile.
    pub chunk: Arc<Chunk>,
    pub light: ChunkLight,
    /// Whether `light` differs from the light the Chunk had when the job began.
    pub changed: bool,
    /// Per face, whether the plane that neighbour reads moved.
    pub outward: [bool; 6],
}

/// Solves one Chunk's light from a snapshot. Equality and the per-face plane
/// comparison run here, not on the tick: they are a 32768-cell compare each.
pub(in crate::server) fn solve(
    chunk: Arc<Chunk>,
    before: &ChunkLight,
    border: &[Border; 6],
    shapes: &ShapeTable,
) -> Solved {
    let open_sky = ChunkLight::filled(Light::new(MAX_LIGHT, 0));
    let lit: [Option<LitChunk>; 6] = std::array::from_fn(|face| match &border[face] {
        Border::Absent => None,
        Border::OpenSky => Some(LitChunk::new(&SKY_CHUNK, &open_sky)),
        Border::Loaded(chunk, light) => Some(LitChunk::new(chunk, light)),
    });
    let light = light_chunk(&chunk, lit, shapes);
    let changed = *before != light;
    let outward = std::array::from_fn(|face| before.boundary_differs(&light, face));
    Solved {
        chunk,
        light,
        changed,
        outward,
    }
}

/// Every job in flight, by stage.
#[derive(Resource, Default)]
pub(in crate::server) struct Jobs {
    pub generate: Stage<Chunk>,
    // Landing for these arrives in later tickets; the stages exist so the type
    // the tests and the overlay read does not change shape three times.
    pub relight: Stage<Solved>,
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
