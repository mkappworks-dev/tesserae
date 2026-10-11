//! World parameters and the Chunk lattice.
//!
//! The Server owns everything here from step 3 on; step 2 writes it once at
//! startup and only reads it afterwards, so no Client system ever mutates World
//! state. See `docs/adr/0002-client-server-from-day-one.md`.

use bevy::prelude::*;
use tesserae_core::CHUNK_SIZE;

/// A Chunk's position in the Chunk lattice. Dimensionless: multiply by
/// [`WorldSettings::cell_size`] to get metres.
#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ChunkCoord {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl ChunkCoord {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        ChunkCoord { x, y, z }
    }
}

/// One World's own parameters.
#[derive(Resource, Clone, Copy, Debug)]
pub struct WorldSettings {
    /// The edge length of one Voxel in metres: the single conversion between
    /// Voxel-index space and metres. Nothing else in the engine may assume a
    /// ratio. See `docs/adr/0003-voxel-size-is-per-world.md`.
    pub voxel_size: f32,
}

impl Default for WorldSettings {
    fn default() -> Self {
        WorldSettings { voxel_size: 1.0 }
    }
}

impl WorldSettings {
    /// A Cell's edge length in metres — a Chunk's footprint, which grows with
    /// LOD level while the Chunk's Voxel count does not.
    pub fn cell_size(&self) -> f32 {
        CHUNK_SIZE as f32 * self.voxel_size
    }

    /// Where a Chunk's Voxel (0, 0, 0) sits, in metres.
    pub fn chunk_origin(&self, coord: ChunkCoord) -> Vec3 {
        Vec3::new(coord.x as f32, coord.y as f32, coord.z as f32) * self.cell_size()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_size_scales_with_voxel_size() {
        assert_eq!(WorldSettings { voxel_size: 1.0 }.cell_size(), 32.0);
        assert_eq!(WorldSettings { voxel_size: 0.25 }.cell_size(), 8.0);
    }

    #[test]
    fn chunk_origins_are_metres_not_voxel_indices() {
        let world = WorldSettings { voxel_size: 0.25 };
        assert_eq!(
            world.chunk_origin(ChunkCoord::new(2, 0, -1)),
            Vec3::new(16.0, 0.0, -8.0)
        );
    }
}
