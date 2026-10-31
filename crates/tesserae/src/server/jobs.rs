//! Streaming work that runs off the tick.
//!
//! A job is one Chunk's worth of one stage. It is handed shared references and
//! returns an owned value, so no worker ever holds a `&mut` into World state;
//! `super::streaming` lands the result by calling the Server-private mutators on
//! its own tick. See `docs/adr/0002-client-server-from-day-one.md`.

use std::fmt::Debug;
use std::hash::Hash;
#[cfg(test)]
use std::sync::Arc;

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::tasks::futures::check_ready;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use tesserae_core::light::{Light, MAX_LIGHT, light_cells};
use tesserae_core::visibility::{Connectivity, connectivity};
use tesserae_core::{Chunk, ChunkLight, ChunkMesh, LitChunk, Tables, Worldgen, mesh_chunk};

use super::world_state::Snapshot;
use crate::world::ChunkCoord;

// A job closes over these by value or by `Arc`, so a field that stops being
// `Send + Sync` should fail here and not at a spawn site in `streaming`.
const _: () = {
    const fn shareable<T: Send + Sync + 'static>() {}
    shareable::<Chunk>();
    shareable::<ChunkLight>();
    shareable::<ChunkMesh>();
    shareable::<Tables>();
    shareable::<Worldgen>();
};

/// The jobs of one stage: running on a worker, or finished and waiting to land.
///
/// Both count as in flight. A result that has finished but not landed is still a
/// snapshot held and a decision not yet applied.
pub(in crate::server) struct Stage<T, K = ChunkCoord> {
    running: HashMap<K, Task<T>>,
    finished: HashMap<K, T>,
    dispatched: usize,
    landed: usize,
    /// Every job landed since the World opened, for the `dev` report.
    #[cfg(any(feature = "dev", feature = "profiling"))]
    landed_total: usize,
    /// Jobs run and their worker thread CPU in nanoseconds since the World
    /// opened, every key together.
    #[cfg(any(feature = "dev", feature = "profiling"))]
    run: std::sync::Arc<[std::sync::atomic::AtomicU64; 2]>,
    /// Results that finished and were thrown away at landing because what they
    /// were computed from had changed.
    #[cfg(any(test, feature = "dev", feature = "profiling"))]
    stale: usize,
    /// While true, a job does not start its work, so a test can change the World
    /// under a job it knows is in flight.
    #[cfg(test)]
    pub hold: Arc<Gate>,
    /// Finishes jobs in an order that has nothing to do with dispatch order.
    #[cfg(test)]
    pub scramble: bool,
    /// Keeps a latency-bound test's jobs from queuing behind other tests' backlog.
    #[cfg(test)]
    pub pool: Option<Arc<bevy::tasks::TaskPool>>,
}

impl<T, K> Default for Stage<T, K> {
    fn default() -> Self {
        Stage {
            running: HashMap::default(),
            finished: HashMap::default(),
            dispatched: 0,
            landed: 0,
            #[cfg(any(feature = "dev", feature = "profiling"))]
            landed_total: 0,
            #[cfg(any(feature = "dev", feature = "profiling"))]
            run: Default::default(),
            #[cfg(any(test, feature = "dev", feature = "profiling"))]
            stale: 0,
            #[cfg(test)]
            hold: Arc::default(),
            #[cfg(test)]
            scramble: false,
            #[cfg(test)]
            pool: None,
        }
    }
}

impl<T: Send + 'static, K: Copy + Eq + Hash + Debug> Stage<T, K> {
    pub fn in_flight(&self) -> usize {
        self.running.len() + self.finished.len()
    }

    pub fn contains(&self, key: K) -> bool {
        self.running.contains_key(&key) || self.finished.contains_key(&key)
    }

    /// Runs `work` on the async compute pool.
    ///
    /// One job per key: a second dispatch for a key already in flight is a
    /// bug in the caller's bookkeeping, and silently replacing the first would
    /// hide it.
    pub fn spawn(&mut self, key: K, work: impl FnOnce() -> T + Send + 'static) {
        debug_assert!(!self.contains(key), "{key:?} already has a job in flight");
        #[cfg(any(feature = "dev", feature = "profiling"))]
        let work = {
            let run = std::sync::Arc::clone(&self.run);
            move || {
                use std::sync::atomic::Ordering::Relaxed;
                let before = crate::cost::thread_cpu();
                let out = work();
                run[0].fetch_add(1, Relaxed);
                if let Some((a, b)) = before.zip(crate::cost::thread_cpu()) {
                    run[1].fetch_add(((b - a) * 1e9) as u64, Relaxed);
                }
                out
            }
        };
        #[cfg(not(test))]
        let pool = AsyncComputeTaskPool::get();
        #[cfg(test)]
        let pool: &bevy::tasks::TaskPool =
            self.pool.as_deref().unwrap_or(AsyncComputeTaskPool::get());
        #[cfg(not(test))]
        let task = pool.spawn(async move { work() });
        // Held and scrambled by awaiting, never by blocking: a job that parked
        // its worker would starve every other test sharing the process's pool.
        #[cfg(test)]
        let task = {
            let hold = Arc::clone(&self.hold);
            let yields = if self.scramble {
                scramble_yields(key)
            } else {
                0
            };
            pool.spawn(async move {
                hold.passed().await;
                for _ in 0..yields {
                    bevy::tasks::futures_lite::future::yield_now().await;
                }
                work()
            })
        };
        self.running.insert(key, task);
        self.dispatched += 1;
    }

    /// Moves every job that has finished to the waiting set, without blocking.
    pub fn poll(&mut self) {
        let mut done = Vec::new();
        for (&key, task) in &mut self.running {
            if let Some(out) = check_ready(task) {
                done.push((key, out));
            }
        }
        for (key, out) in done {
            self.running.remove(&key);
            self.finished.insert(key, out);
        }
    }

    pub fn keys(&self) -> Vec<K> {
        self.running
            .keys()
            .chain(self.finished.keys())
            .copied()
            .collect()
    }

    /// The keys whose results are waiting to land.
    pub fn finished(&self) -> impl Iterator<Item = K> + '_ {
        self.finished.keys().copied()
    }

    pub fn take(&mut self, key: K) -> Option<T> {
        let out = self.finished.remove(&key)?;
        self.landed += 1;
        #[cfg(any(feature = "dev", feature = "profiling"))]
        {
            self.landed_total += 1;
        }
        Some(out)
    }

    /// Drops every job, running or finished, for which `keep` is false. A
    /// running job is cancelled if it has not started; one that has runs to
    /// completion with nowhere to deliver.
    pub fn retain(&mut self, mut keep: impl FnMut(K) -> bool) {
        self.running.retain(|&c, _| keep(c));
        self.finished.retain(|&c, _| keep(c));
    }

    /// Records a result that was out of date when it landed, whether it was then
    /// thrown away or delivered anyway.
    pub fn note_stale(&mut self) {
        #[cfg(any(test, feature = "dev", feature = "profiling"))]
        {
            self.stale += 1;
        }
    }

    #[cfg(any(test, feature = "dev", feature = "profiling"))]
    pub fn stale(&self) -> usize {
        self.stale
    }

    #[cfg(any(feature = "dev", feature = "profiling"))]
    pub fn landed_total(&self) -> usize {
        self.landed_total
    }

    /// Jobs run, and their worker thread CPU in milliseconds, since the World
    /// opened. The CPU is zero without the `profiling` thread clock.
    #[cfg(any(feature = "dev", feature = "profiling"))]
    pub fn run(&self) -> (u64, f64) {
        use std::sync::atomic::Ordering::Relaxed;
        (
            self.run[0].load(Relaxed),
            self.run[1].load(Relaxed) as f64 / 1e6,
        )
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

/// A pseudo-random number of times a job yields before it starts, derived from
/// the key alone, so jobs finish in an order unrelated to dispatch order.
#[cfg(test)]
fn scramble_yields(key: impl Hash) -> u64 {
    use std::hash::Hasher;
    // `DefaultHasher::new` has fixed keys, so a key yields the same every run.
    let mut hasher = std::hash::DefaultHasher::new();
    key.hash(&mut hasher);
    (hasher.finish() >> 20) % 40
}

/// Keeps the jobs of a stage from starting until released.
#[cfg(test)]
#[derive(Default)]
pub(in crate::server) struct Gate {
    held: std::sync::atomic::AtomicBool,
    waiting: std::sync::Mutex<Vec<std::task::Waker>>,
}

#[cfg(test)]
impl Gate {
    pub fn hold(&self) {
        self.held.store(true, std::sync::atomic::Ordering::Release);
    }

    pub fn release(&self) {
        self.held.store(false, std::sync::atomic::Ordering::Release);
        for waker in self.waiting.lock().expect("not poisoned").drain(..) {
            waker.wake();
        }
    }

    /// Ready once the gate is open.
    async fn passed(&self) {
        std::future::poll_fn(|cx| {
            let mut waiting = self.waiting.lock().expect("not poisoned");
            // Checked under the lock `release` takes, so a release between the
            // check and the push cannot be missed.
            if !self.held.load(std::sync::atomic::Ordering::Acquire) {
                return std::task::Poll::Ready(());
            }
            waiting.push(cx.waker().clone());
            std::task::Poll::Pending
        })
        .await
    }
}

/// What a relight job reads of one face neighbour.
pub(in crate::server) enum Border {
    /// Not loaded, so a dark boundary.
    Absent,
    /// Above the band the surface can reach, where the generator makes only air
    /// and everything sees full sky. The relaxation cannot discover this: there
    /// is no neighbour to ask.
    OpenSky,
    Loaded(Snapshot),
}

/// The Voxels of the synthetic Chunk above the surface: air, which is all the
/// generator makes there.
static SKY_CHUNK: Chunk = Chunk::air();

/// A finished light solve.
pub(in crate::server) struct Solved {
    /// What was solved, so landing can tell it is still what the store holds
    /// and has not been evicted, regenerated or changed under it meanwhile.
    pub snapshot: Snapshot,
    pub light: ChunkLight,
    /// Whether `light` differs from the light the Chunk had when the job began.
    pub changed: bool,
    /// Per face, whether the plane that neighbour reads moved.
    pub outward: [bool; 6],
}

/// Solves one Chunk's light from a snapshot. Equality and the per-face plane
/// comparison run here, not on the tick: they are a 32768-cell compare each.
pub(in crate::server) fn solve(
    snapshot: Snapshot,
    border: &[Border; 6],
    tables: &Tables,
) -> Solved {
    let before: &ChunkLight = &snapshot.light;
    let open_sky = ChunkLight::filled(Light::new(MAX_LIGHT, 0));
    let lit: [Option<LitChunk>; 6] = std::array::from_fn(|face| match &border[face] {
        Border::Absent => None,
        Border::OpenSky => Some(LitChunk::new(&SKY_CHUNK, &open_sky)),
        Border::Loaded(s) => Some(LitChunk::new(&s.chunk, &s.light)),
    });
    let light = light_cells(&snapshot.chunk, &snapshot.overlay, lit, tables);
    let changed = *before != light;
    let outward = std::array::from_fn(|face| before.boundary_differs(&light, face));
    Solved {
        snapshot,
        light,
        changed,
        outward,
    }
}

/// A generated Chunk and which of its faces see each other through it.
pub(in crate::server) struct Generated {
    pub chunk: Chunk,
    pub connectivity: Connectivity,
}

/// Generates one Chunk, and its connectivity while the worker holds it.
pub(in crate::server) fn generate(
    worldgen: &Worldgen,
    coord: ChunkCoord,
    tables: &Tables,
) -> Generated {
    let chunk = worldgen.chunk(coord.x, coord.y, coord.z);
    let connectivity = connectivity(&chunk, &tables.materials);
    Generated {
        chunk,
        connectivity,
    }
}

/// A mesh and the connectivity of the Chunk it was built from.
pub(in crate::server) struct Built {
    pub mesh: ChunkMesh,
    pub connectivity: Connectivity,
    pub from: std::sync::Weak<Chunk>,
}

/// Meshes one Chunk from a snapshot of it and its six face neighbours. Its
/// connectivity is worked out again only if `known` is not that Chunk's.
pub(in crate::server) fn build_mesh(
    center: &Snapshot,
    around: &[Snapshot; 6],
    known: Option<Connectivity>,
    tables: &Tables,
) -> Built {
    let neighbors: [Option<LitChunk>; 6] = std::array::from_fn(|face| Some(lit(&around[face])));
    Built {
        mesh: mesh_chunk(lit(center), neighbors, tables),
        connectivity: known.unwrap_or_else(|| connectivity(&center.chunk, &tables.materials)),
        from: std::sync::Arc::downgrade(&center.chunk),
    }
}

fn lit(snapshot: &Snapshot) -> LitChunk<'_> {
    LitChunk::new(&snapshot.chunk, &snapshot.light).with_overlay(&snapshot.overlay)
}

/// Every job in flight, by stage.
#[derive(Resource, Default)]
pub(in crate::server) struct Jobs {
    pub generate: Stage<Generated>,
    // Landing for these arrives in later tickets; the stages exist so the type
    // the tests and the overlay read does not change shape three times.
    pub relight: Stage<Solved>,
    pub mesh: Stage<Built>,
}

impl Jobs {
    #[cfg(any(test, feature = "dev", feature = "profiling"))]
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
