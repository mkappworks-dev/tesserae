//! World parameters and the Chunk lattice.
//!
//! What a World owns: its Voxel size, its seed and its Chunks. How far the
//! Server streams is not in here — that is a runtime setting and lives with the
//! Server, because it is not save data. See `CONTEXT.md`.

use bevy::prelude::*;
use tesserae_core::CHUNK_SIZE;

/// A Chunk's position in the Chunk lattice. Dimensionless: multiply by
/// [`WorldSettings::cell_size`] to get metres.
#[derive(Component, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
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

impl std::fmt::Display for ChunkCoord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}", self.x, self.y, self.z)
    }
}

/// One World's own parameters.
#[derive(Resource, Clone, Copy, Debug)]
pub struct WorldSettings {
    /// The edge length of one Voxel in metres: the single conversion between
    /// Voxel-index space and metres. Nothing else in the engine may assume a
    /// ratio. See `docs/adr/0003-voxel-size-is-per-world.md`.
    pub voxel_size: f32,
    /// What the World generates from. Save data, so a World regenerates
    /// identically after a Chunk is evicted.
    pub seed: u64,
}

impl Default for WorldSettings {
    fn default() -> Self {
        WorldSettings {
            voxel_size: 1.0,
            seed: 0x7e55_e2ae_0000_0001,
        }
    }
}

/// The Voxel sizes every physical fixture is tested at.
#[cfg(feature = "dev")]
const DEV_VOXEL_SIZES: [f32; 3] = [1.0, 0.5, 0.25];

impl WorldSettings {
    /// The defaults, except that under `dev` `TESSERAE_VOXEL_SIZE_M` picks one
    /// of [`DEV_VOXEL_SIZES`], so one binary opens a World at each of them.
    /// Save data does not exist yet, so nothing else chooses it.
    #[cfg(feature = "dev")]
    pub fn from_env(env: impl Fn(&str) -> Option<String>) -> Self {
        let defaults = WorldSettings::default();
        let Some(asked) = env("TESSERAE_VOXEL_SIZE_M") else {
            return defaults;
        };
        match asked.parse::<f32>() {
            Ok(size) if DEV_VOXEL_SIZES.contains(&size) => WorldSettings {
                voxel_size: size,
                ..defaults
            },
            _ => {
                eprintln!(
                    "TESSERAE_VOXEL_SIZE_M={asked} is not one of {DEV_VOXEL_SIZES:?}; using {}",
                    defaults.voxel_size
                );
                defaults
            }
        }
    }

    /// A Cell's edge length in metres — a Chunk's footprint, which grows with
    /// LOD level while the Chunk's Voxel count does not.
    pub fn cell_size(&self) -> f32 {
        CHUNK_SIZE as f32 * self.voxel_size
    }

    /// Where a Chunk's Voxel (0, 0, 0) sits, in metres.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn chunk_origin(&self, coord: ChunkCoord) -> Vec3 {
        self.chunk_origin_at(coord, 0)
    }

    pub fn chunk_origin_at(&self, coord: ChunkCoord, level: u32) -> Vec3 {
        Vec3::new(coord.x as f32, coord.y as f32, coord.z as f32) * self.cell_size_at(level)
    }

    pub fn voxel_size_at(&self, level: u32) -> f32 {
        self.voxel_size * 2f32.powi(level as i32)
    }

    /// Which Voxel of the lattice holds a point, in the one continuous
    /// coordinate space `tesserae_core`'s flood-fill works over.
    ///
    /// Only the `dev` gallery converts a metre position to a Voxel today, so
    /// a default-feature build has no caller; see the note in
    /// `server/world_state.rs`.
    ///
    /// The single conversion from metres to a Voxel index, so everything
    /// physical stays in metres: `docs/adr/0003-voxel-size-is-per-world.md`.
    /// A floor division for the same reason as [`WorldSettings::chunk_at`].
    #[cfg_attr(not(feature = "dev"), allow(dead_code))]
    pub fn voxel_at(&self, metres: Vec3) -> [i32; 3] {
        let v = (metres / self.voxel_size).floor();
        [v.x as i32, v.y as i32, v.z as i32]
    }

    pub fn cell_size_at(&self, level: u32) -> f32 {
        self.cell_size() * 2f32.powi(level as i32)
    }

    /// Which Chunk holds a point. The inverse of [`WorldSettings::chunk_origin`],
    /// and a floor division rather than a truncation: `-1.0 / 32.0` truncates
    /// towards zero and would put everything in the Cell below the origin into
    /// Chunk 0 alongside everything above it.
    pub fn chunk_at(&self, metres: Vec3) -> ChunkCoord {
        self.chunk_at_level(metres, 0)
    }

    pub fn chunk_at_level(&self, metres: Vec3, level: u32) -> ChunkCoord {
        let c = (metres / self.cell_size_at(level)).floor();
        ChunkCoord::new(c.x as i32, c.y as i32, c.z as i32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "dev")]
    #[test]
    fn the_dev_voxel_size_is_one_of_the_tested_sizes_or_the_default() {
        let with = |v: Option<&str>| {
            WorldSettings::from_env(|name| {
                assert_eq!(name, "TESSERAE_VOXEL_SIZE_M");
                v.map(String::from)
            })
            .voxel_size
        };
        assert_eq!(with(None), 1.0);
        assert_eq!(with(Some("0.5")), 0.5);
        assert_eq!(with(Some("0.25")), 0.25);
        for refused in ["0.3", "0", "-1", "nan", "two"] {
            assert_eq!(with(Some(refused)), 1.0, "{refused}");
        }
    }

    #[test]
    fn cell_size_scales_with_voxel_size() {
        let at = |voxel_size| {
            WorldSettings {
                voxel_size,
                ..default()
            }
            .cell_size()
        };
        assert_eq!(at(1.0), 32.0);
        assert_eq!(at(0.25), 8.0);
    }

    #[test]
    fn chunk_origins_are_metres_not_voxel_indices() {
        let world = WorldSettings {
            voxel_size: 0.25,
            ..default()
        };
        assert_eq!(
            world.chunk_origin(ChunkCoord::new(2, 0, -1)),
            Vec3::new(16.0, 0.0, -8.0)
        );
    }

    #[test]
    fn chunk_at_is_the_inverse_of_chunk_origin() {
        for voxel_size in [1.0, 0.25] {
            let world = WorldSettings {
                voxel_size,
                ..default()
            };
            for coord in [
                ChunkCoord::new(0, 0, 0),
                ChunkCoord::new(3, -2, 7),
                ChunkCoord::new(-11, 4, -1),
            ] {
                assert_eq!(world.chunk_at(world.chunk_origin(coord)), coord);
                // And anywhere inside the Cell, not just at its corner.
                let inside = world.chunk_origin(coord) + Vec3::splat(world.cell_size() * 0.5);
                assert_eq!(world.chunk_at(inside), coord);
            }
        }
    }

    #[test]
    fn a_coarse_levels_voxel_and_origin_double_per_level() {
        let world = WorldSettings {
            voxel_size: 0.25,
            ..default()
        };
        assert_eq!(world.voxel_size_at(0), 0.25);
        assert_eq!(world.voxel_size_at(3), 2.0);
        assert_eq!(
            world.chunk_origin_at(ChunkCoord::new(3, -1, 0), 2),
            Vec3::new(96.0, -32.0, 0.0)
        );
        assert_eq!(
            world.chunk_origin_at(ChunkCoord::new(3, -1, 0), 0),
            world.chunk_origin(ChunkCoord::new(3, -1, 0))
        );
    }

    #[test]
    fn voxel_at_scales_with_voxel_size_and_floors() {
        let coarse = WorldSettings {
            voxel_size: 1.0,
            ..default()
        };
        let fine = WorldSettings {
            voxel_size: 0.25,
            ..default()
        };
        // The same point in metres is four times as many Voxels in.
        assert_eq!(coarse.voxel_at(Vec3::new(3.0, 0.0, 0.0)), [3, 0, 0]);
        assert_eq!(fine.voxel_at(Vec3::new(3.0, 0.0, 0.0)), [12, 0, 0]);
        // Truncation would put this Voxel and the one above it both at 0.
        assert_eq!(coarse.voxel_at(Vec3::new(-0.5, -0.5, -0.5)), [-1, -1, -1]);
    }

    #[test]
    fn chunk_at_floors_rather_than_truncating_below_the_origin() {
        let world = WorldSettings::default();
        // Truncation would put both of these in Chunk 0.
        assert_eq!(
            world.chunk_at(Vec3::new(-1.0, -1.0, -1.0)),
            ChunkCoord::new(-1, -1, -1)
        );
        assert_eq!(
            world.chunk_at(Vec3::new(1.0, 1.0, 1.0)),
            ChunkCoord::new(0, 0, 0)
        );
    }
}
