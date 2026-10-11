//! World state, and the Server's exclusive right to write it.
//!
//! Every mutator here is `pub(in crate::server)`. That is the whole enforcement
//! of `docs/adr/0002-client-server-from-day-one.md`: a Client system can still
//! ask for `ResMut<ChunkStore>`, but with the mutators out of scope there is
//! nothing it can call, so "a Client system that writes World state" fails to
//! compile rather than merely being noticed in review. The Client gets finished
//! meshes through `super::streaming`'s messages and needs no read access either.

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use tesserae_core::light::{Light, LightVolume};
use tesserae_core::{CHUNK_SIZE, Chunk, ChunkLight, Voxel};

use crate::world::ChunkCoord;

/// Every Chunk the Server currently holds, and its light.
///
/// Light is World state like the Voxels are, so it lives here under the same
/// Server-private mutators rather than on the Client that renders it.
#[derive(Resource, Default)]
pub struct ChunkStore {
    chunks: HashMap<ChunkCoord, Chunk>,
    light: HashMap<ChunkCoord, ChunkLight>,
}

impl ChunkStore {
    pub fn get(&self, coord: ChunkCoord) -> Option<&Chunk> {
        self.chunks.get(&coord)
    }

    pub fn light(&self, coord: ChunkCoord) -> Option<&ChunkLight> {
        self.light.get(&coord)
    }

    #[cfg_attr(not(feature = "dev"), allow(dead_code))]
    /// One Voxel's light, in lattice coordinates. `Light::DARK` where the
    /// volume does not extend, which is also what [`LightVolume`] reports.
    pub fn light_at(&self, at: [i32; 3]) -> Light {
        let at = VoxelCoord::of(at);
        self.light
            .get(&at.chunk)
            .map_or(Light::DARK, |light| light.get(at.x, at.y, at.z))
    }

    pub fn contains(&self, coord: ChunkCoord) -> bool {
        self.chunks.contains_key(&coord)
    }

    pub fn coords(&self) -> impl Iterator<Item = ChunkCoord> + '_ {
        self.chunks.keys().copied()
    }

    /// A Chunk arrives dark. `super::streaming`'s relight stage fills it in
    /// from its neighbours, which is also what makes a Chunk arriving next to
    /// it brighten this one.
    pub(in crate::server) fn insert(&mut self, coord: ChunkCoord, chunk: Chunk) {
        self.chunks.insert(coord, chunk);
        self.light.insert(coord, ChunkLight::dark());
    }

    pub(in crate::server) fn set_light(&mut self, coord: ChunkCoord, light: ChunkLight) {
        self.light.insert(coord, light);
    }

    #[cfg_attr(not(feature = "dev"), allow(dead_code))]
    /// Replaces one Voxel. The caller re-floods light around it with
    /// `tesserae_core::relight` — see `super::lightbox`, which is the only
    /// caller until build-order step 5 brings a real edit path.
    pub(in crate::server) fn set_voxel(&mut self, at: VoxelCoord, voxel: Voxel) {
        if let Some(chunk) = self.chunks.get_mut(&at.chunk) {
            chunk.set(at.x, at.y, at.z, voxel);
        }
    }

    pub(in crate::server) fn remove(&mut self, coord: ChunkCoord) {
        self.chunks.remove(&coord);
        self.light.remove(&coord);
    }
}

// The types and mutators below are the Server's Voxel-edit path. Its only
// caller today is the `dev` lightbox, so a default-feature build has none and
// `-D warnings` would reject them; each carries
// `cfg_attr(not(feature = "dev"), allow(dead_code))` rather than a blanket
// allow, so the lint still fires once build-order step 5 brings the real edit
// path and something later removes its last caller.

/// A Voxel's position, split into the Chunk that holds it and its index inside.
///
/// The split is the whole content of the type: `tesserae_core`'s flood-fill
/// addresses Voxels in one continuous lattice and knows nothing about Chunks,
/// so something has to do this arithmetic and it is not core's job.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(not(feature = "dev"), allow(dead_code))]
pub struct VoxelCoord {
    pub chunk: ChunkCoord,
    pub x: usize,
    pub y: usize,
    pub z: usize,
}

#[cfg_attr(not(feature = "dev"), allow(dead_code))]
impl VoxelCoord {
    /// A floor division, not a truncation: `-1 / 32` truncates towards zero and
    /// would put the Voxel one below a Chunk's origin into that same Chunk.
    /// The same trap `WorldSettings::chunk_at` documents, one scale down.
    pub fn of(at: [i32; 3]) -> Self {
        let size = CHUNK_SIZE as i32;
        let chunk = ChunkCoord::new(
            at[0].div_euclid(size),
            at[1].div_euclid(size),
            at[2].div_euclid(size),
        );
        VoxelCoord {
            chunk,
            x: at[0].rem_euclid(size) as usize,
            y: at[1].rem_euclid(size) as usize,
            z: at[2].rem_euclid(size) as usize,
        }
    }
}

/// The Server's Chunk store, seen as the one continuous lattice of Voxels that
/// `tesserae_core`'s incremental flood-fill works over.
///
/// Records which Chunks it wrote to, so `super::streaming` knows what to
/// re-mesh: an edit's light can reach one Chunk in every direction, and the
/// caller has no other way to find out which.
#[cfg_attr(not(feature = "dev"), allow(dead_code))]
pub(in crate::server) struct LightLattice<'a> {
    pub store: &'a mut ChunkStore,
    pub touched: Vec<ChunkCoord>,
}

#[cfg_attr(not(feature = "dev"), allow(dead_code))]
impl<'a> LightLattice<'a> {
    pub fn new(store: &'a mut ChunkStore) -> Self {
        LightLattice {
            store,
            touched: Vec::new(),
        }
    }
}

impl LightVolume for LightLattice<'_> {
    fn voxel(&self, at: [i32; 3]) -> Option<Voxel> {
        let at = VoxelCoord::of(at);
        self.store
            .get(at.chunk)
            .map(|chunk| chunk.get(at.x, at.y, at.z))
    }

    fn light(&self, at: [i32; 3]) -> Light {
        let at = VoxelCoord::of(at);
        self.store
            .light(at.chunk)
            .map_or(Light::DARK, |light| light.get(at.x, at.y, at.z))
    }

    fn set_light(&mut self, at: [i32; 3], light: Light) {
        let at = VoxelCoord::of(at);
        let Some(stored) = self.store.light.get_mut(&at.chunk) else {
            return;
        };
        stored.set(at.x, at.y, at.z, light);
        if self.touched.last() != Some(&at.chunk) {
            // Last-write check rather than a set: the fill has strong locality,
            // so this collapses nearly every run, and `streaming` de-duplicates
            // what is left when it marks Chunks dirty.
            self.touched.push(at.chunk);
        }
    }
}

/// Where a player enters the World, in metres.
///
/// Private field with a Server-only constructor, for the reason in the module
/// doc. The Client reads it to frame its camera; framing is a Client decision,
/// the spawn point is not.
#[derive(Resource, Clone, Copy, Debug)]
pub struct SpawnPoint(Vec3);

impl SpawnPoint {
    pub(in crate::server) fn new(metres: Vec3) -> Self {
        SpawnPoint(metres)
    }

    pub fn metres(self) -> Vec3 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tesserae_core::light::MAX_LIGHT;
    use tesserae_core::{CHUNK_SIZE, relight, worldgen};

    #[test]
    fn a_voxel_coord_floors_rather_than_truncating_below_the_origin() {
        let last = CHUNK_SIZE - 1;
        // Truncation would put all three of these in Chunk 0.
        for (at, chunk, local) in [
            ([0, 0, 0], ChunkCoord::new(0, 0, 0), (0, 0, 0)),
            (
                [-1, -1, -1],
                ChunkCoord::new(-1, -1, -1),
                (last, last, last),
            ),
            ([31, 32, -32], ChunkCoord::new(0, 1, -1), (last, 0, 0)),
        ] {
            let v = VoxelCoord::of(at);
            assert_eq!(v.chunk, chunk, "{at:?}");
            assert_eq!((v.x, v.y, v.z), local, "{at:?}");
        }
    }

    /// The `LightVolume` impl is what lets core's flood-fill cross a Chunk
    /// boundary without knowing Chunks exist. A light placed near a face has to
    /// reach into the next Chunk, and a Chunk that is not loaded has to stop
    /// it — `None`, not air.
    #[test]
    fn the_lattice_crosses_a_chunk_boundary_and_stops_at_an_unloaded_one() {
        let mut store = ChunkStore::default();
        let here = ChunkCoord::new(0, 0, 0);
        let next = ChunkCoord::new(1, 0, 0);
        store.insert(here, Chunk::air());
        store.insert(next, Chunk::air());

        let last = CHUNK_SIZE as i32 - 1;
        let at = [last, 5, 5];
        let mut lattice = LightLattice::new(&mut store);
        lattice
            .store
            .set_voxel(VoxelCoord::of(at), Voxel::cube(worldgen::LANTERN));
        relight(&mut lattice, at, Light::DARK);

        assert_eq!(store.light_at(at).block(), MAX_LIGHT);
        assert_eq!(
            store.light_at([last + 1, 5, 5]).block(),
            MAX_LIGHT - 1,
            "light did not cross into the next Chunk"
        );
        // Four Chunks away is not loaded, so the fill stopped rather than
        // treating the gap as air.
        assert_eq!(
            store.light_at([last + 1 + CHUNK_SIZE as i32, 5, 5]),
            Light::DARK
        );
    }

    #[test]
    fn the_lattice_reports_which_chunks_it_wrote_to() {
        let mut store = ChunkStore::default();
        let here = ChunkCoord::new(0, 0, 0);
        let next = ChunkCoord::new(1, 0, 0);
        store.insert(here, Chunk::air());
        store.insert(next, Chunk::air());

        let at = [CHUNK_SIZE as i32 - 1, 5, 5];
        let mut lattice = LightLattice::new(&mut store);
        lattice
            .store
            .set_voxel(VoxelCoord::of(at), Voxel::cube(worldgen::LANTERN));
        relight(&mut lattice, at, Light::DARK);
        let touched = lattice.touched;

        // Both, because the caller has no other way to learn that the Chunk
        // next door needs re-meshing.
        for coord in [here, next] {
            assert!(touched.contains(&coord), "{coord} not reported");
        }
    }

    #[test]
    fn a_chunk_arrives_dark_and_eviction_takes_its_light_with_it() {
        let mut store = ChunkStore::default();
        let coord = ChunkCoord::new(2, 0, -1);
        store.insert(coord, Chunk::air());
        assert_eq!(
            store.light(coord).and_then(ChunkLight::uniform),
            Some(Light::DARK)
        );

        store.remove(coord);
        assert!(store.light(coord).is_none(), "light outlived its Chunk");
    }
}
