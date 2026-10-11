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
use tesserae_core::Chunk;

use crate::world::ChunkCoord;

/// Every Chunk the Server currently holds.
#[derive(Resource, Default)]
pub struct ChunkStore {
    chunks: HashMap<ChunkCoord, Chunk>,
}

impl ChunkStore {
    pub fn get(&self, coord: ChunkCoord) -> Option<&Chunk> {
        self.chunks.get(&coord)
    }

    pub fn contains(&self, coord: ChunkCoord) -> bool {
        self.chunks.contains_key(&coord)
    }

    pub fn coords(&self) -> impl Iterator<Item = ChunkCoord> + '_ {
        self.chunks.keys().copied()
    }

    pub(in crate::server) fn insert(&mut self, coord: ChunkCoord, chunk: Chunk) {
        self.chunks.insert(coord, chunk);
    }

    pub(in crate::server) fn remove(&mut self, coord: ChunkCoord) {
        self.chunks.remove(&coord);
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
