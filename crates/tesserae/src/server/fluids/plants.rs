//! Plant growth on the simulation's Logical step.
//!
//! The Sample tick shares [`FluidSim`]'s clock and lands through the same
//! [`edit::Batch`], so a crop's Stage is one more change of the one World
//! change. A step's Samples are read after every event of its index and after
//! the step's own fluid landings, so nothing they read can be stale; only a
//! change deferred past [`PLANTS_LANDED_PER_STEP`] is read again next step.

use std::collections::{BTreeMap, BTreeSet};

use tesserae_core::CHUNK_SIZE;
use tesserae_core::plant::{GrowEnv, Growth, PLANTS_LANDED_PER_STEP, Surroundings, grow, samples};
use tesserae_core::{
    Chunk, Climate, FluidTable, Light, MaterialId, Occupants, Tables, Voxel, Worldgen,
};

use super::super::edit;
use super::super::streaming::MeshState;
use super::super::world_state::{ChunkStore, VoxelCoord};
use super::{Cell, FluidSim};

/// Waiting changes kept past the landing bound; the rest Sample again.
const DEFERRED_CAP: usize = 4096;

/// What the World says about where a Plant stands.
pub(super) struct Env<'a> {
    pub seed: u64,
    pub voxel_size: f32,
    pub generator: Option<&'a Worldgen>,
}

/// A growth that passed its roll and its needs and is waiting its turn.
#[derive(Clone, Copy)]
struct Pending {
    from: MaterialId,
    roll: f32,
}

/// A plant Voxel that lost its support and broke, for the Server to drop its
/// Seed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::server) struct Broken {
    pub at: Cell,
    pub material: MaterialId,
}

pub(super) struct PlantSim {
    landed_per_step: usize,
    deferred: BTreeMap<Cell, Pending>,
    /// Voxels whose support is to be read again once their Chunk is lit.
    recheck: BTreeSet<Cell>,
    pub broken: Vec<Broken>,
    pub landed: usize,
    /// Changes past the landing bound, evaluated again next step.
    pub late: usize,
    /// Changes the World change refused or whose Voxel was no longer the Stage
    /// they were computed from.
    pub stale: usize,
}

impl Default for PlantSim {
    fn default() -> Self {
        PlantSim {
            landed_per_step: PLANTS_LANDED_PER_STEP,
            deferred: BTreeMap::new(),
            recheck: BTreeSet::new(),
            broken: Vec::new(),
            landed: 0,
            late: 0,
            stale: 0,
        }
    }
}

impl PlantSim {
    #[cfg(test)]
    pub fn set_bound(&mut self, landed_per_step: usize) {
        self.landed_per_step = landed_per_step;
    }

    #[cfg(test)]
    pub fn waiting(&self) -> usize {
        self.deferred.len()
    }
}

/// Reads only Chunks the simulation has admitted: terrain beyond them is
/// unknown, so a Plant beside it holds rather than reading Air.
struct Reads<'a> {
    sim: &'a FluidSim,
    store: &'a ChunkStore,
    meshes: &'a MeshState,
    env: &'a Env<'a>,
    fluids: &'a FluidTable,
}

impl Surroundings for Reads<'_> {
    fn occupants(&self, at: [i32; 3]) -> Option<Occupants> {
        if !self.sim.simulated.contains_key(&VoxelCoord::of(at).chunk) {
            return None;
        }
        self.store.occupants(at, self.fluids)
    }

    fn light(&self, at: [i32; 3]) -> Option<Light> {
        let chunk = VoxelCoord::of(at).chunk;
        (self.sim.simulated.contains_key(&chunk) && !self.meshes.is_unlit(chunk))
            .then(|| self.store.light_at(at))
    }

    fn climate_at(&self, x_m: f32, z_m: f32) -> Option<Climate> {
        self.env.generator.map(|g| g.climate_at(x_m, z_m))
    }
}

fn holds_plant(chunk: &Chunk, tables: &Tables) -> bool {
    chunk
        .kinds()
        .iter()
        .any(|v| !v.is_air() && tables.plants.of(v.material()).is_some())
}

/// Runs step `step`'s Sample tick over every admitted Chunk and lands what
/// grew. The caller wakes the fluid simulation around the changes.
pub(super) fn sample_step(
    sim: &mut FluidSim,
    store: &mut ChunkStore,
    meshes: &mut MeshState,
    tables: &Tables,
    env: &Env,
    step: u64,
) {
    if tables.plants.is_empty() {
        return;
    }
    let waiting = std::mem::take(&mut sim.plants.deferred);
    let mut grown: BTreeMap<Cell, (Pending, Growth)> = BTreeMap::new();
    {
        let reads = Reads {
            sim,
            store,
            meshes,
            env,
            fluids: &tables.fluids,
        };
        let rules = GrowEnv {
            plants: &tables.plants,
            shapes: &tables.shapes,
            seed: env.seed,
            voxel_size: env.voxel_size,
        };
        let reread = |at: Cell, from: MaterialId, roll: f32| {
            let here = VoxelCoord::of(at);
            let voxel = store.get(here.chunk)?.get(here.x, here.y, here.z);
            if voxel.material() != from {
                return None;
            }
            grow(&rules, &reads, step, at, voxel, roll).map(|growth| (from, growth))
        };
        for (&at, p) in &waiting {
            if let Some((from, growth)) = reread(at, p.from, p.roll) {
                grown.insert(at, (Pending { from, roll: p.roll }, growth));
            }
        }
        for &chunk in sim.simulated.keys() {
            if meshes.is_unlit(chunk) {
                continue;
            }
            let Some(held) = store.get(chunk) else {
                continue;
            };
            if !holds_plant(held, tables) {
                continue;
            }
            let corner = [
                chunk.x * CHUNK_SIZE as i32,
                chunk.y * CHUNK_SIZE as i32,
                chunk.z * CHUNK_SIZE as i32,
            ];
            for s in samples(env.seed, [chunk.x, chunk.y, chunk.z], step) {
                let [x, y, z] = s.local;
                let from = held.get(x, y, z).material();
                let at = [
                    corner[0] + x as i32,
                    corner[1] + y as i32,
                    corner[2] + z as i32,
                ];
                if grown.contains_key(&at) || tables.plants.of(from).is_none() {
                    continue;
                }
                if let Some((from, growth)) = reread(at, from, s.roll) {
                    grown.insert(at, (Pending { from, roll: s.roll }, growth));
                }
            }
        }
    }
    let mut batch = edit::Batch::default();
    let mut landed = 0;
    // Two growths that write one Voxel in a step: the later Samples again.
    let mut touched: BTreeSet<Cell> = BTreeSet::new();
    for (at, (pending, growth)) in grown {
        if landed >= sim.plants.landed_per_step {
            sim.plants.late += 1;
            if sim.plants.deferred.len() < DEFERRED_CAP {
                sim.plants.deferred.insert(at, pending);
            }
            continue;
        }
        if growth.changes.iter().any(|c| touched.contains(&c.at)) {
            sim.plants.stale += 1;
            continue;
        }
        let mut refused = false;
        for change in &growth.changes {
            let Some(here) = store.occupants(change.at, &tables.fluids) else {
                refused = true;
                break;
            };
            // A new Voxel in Air keeps the fluid it is waterlogged in; a Stage
            // swap keeps what the old Voxel carried.
            let next = if here.solid.is_air() {
                here.place_solid(change.voxel, tables)
            } else {
                Occupants {
                    solid: change.voxel,
                    ..here
                }
            };
            if batch
                .set_occupants(store, meshes, tables, change.at, next)
                .is_err()
            {
                refused = true;
                break;
            }
            touched.insert(change.at);
        }
        if refused {
            sim.plants.stale += 1;
        } else {
            sim.plants.landed += 1;
            landed += 1;
        }
    }
    batch.finish(store, meshes, tables);
}

const FACES: [[i32; 3]; 6] = [
    [1, 0, 0],
    [-1, 0, 0],
    [0, 1, 0],
    [0, -1, 0],
    [0, 0, 1],
    [0, 0, -1],
];

/// Reads the support of every plant Voxel on or beside a changed cell and
/// breaks the ones whose soil or attachment is gone, in the step that changed
/// it. Only what changed is read, so a World with no change reads nothing.
/// Returns whether a Voxel broke: its own change may unsupport another.
pub(super) fn break_unsupported(
    sim: &mut FluidSim,
    store: &mut ChunkStore,
    meshes: &mut MeshState,
    tables: &Tables,
    changed: &[Cell],
) -> bool {
    if tables.plants.is_empty() || (changed.is_empty() && sim.plants.recheck.is_empty()) {
        return false;
    }
    let mut candidates = std::mem::take(&mut sim.plants.recheck);
    for &at in changed {
        candidates.insert(at);
        for d in FACES {
            candidates.insert([at[0] + d[0], at[1] + d[1], at[2] + d[2]]);
        }
    }
    let mut batch = edit::Batch::default();
    let mut broke = false;
    for at in candidates {
        let Some(here) = store.occupants(at, &tables.fluids) else {
            continue;
        };
        let material = here.solid.material();
        if here.solid.is_air() || tables.plants.of(material).is_none() {
            continue;
        }
        let lacks = {
            let held = |c: Cell| store.occupants(c, &tables.fluids);
            tables
                .plants
                .lacks_support(&tables.shapes, here.solid, at, &held)
                .is_some()
        };
        if !lacks {
            continue;
        }
        if meshes.is_unlit(VoxelCoord::of(at).chunk) {
            sim.plants.recheck.insert(at);
            continue;
        }
        let next = Occupants {
            solid: Voxel::AIR,
            ..here
        };
        if batch.set_occupants(store, meshes, tables, at, next).is_ok() {
            sim.plants.broken.push(Broken { at, material });
            broke = true;
        }
    }
    batch.finish(store, meshes, tables);
    broke
}

#[cfg(test)]
mod tests;
