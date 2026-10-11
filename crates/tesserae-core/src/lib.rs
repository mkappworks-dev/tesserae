//! Chunk storage, world generation and meshing for Tesserae.
//!
//! This crate depends on neither Bevy nor wgpu, and must not acquire such a
//! dependency even transitively: it is what keeps every expensive algorithm
//! headless, unit-testable and benchmarkable. CI enforces it in the
//! `Boundaries` job. See `docs/architecture.md`.

pub mod chunk;
pub mod mesh;
pub mod noise;
pub mod testgen;
pub mod voxel;
pub mod worldgen;

pub use chunk::{CHUNK_SIZE, CHUNK_VOLUME, Chunk};
pub use mesh::{ChunkMesh, Neighborhood, greedy_mesh, mesh_chunk};
pub use voxel::{AIR, MaterialId, SHAPE_FULL_CUBE, ShapeId, Voxel};
pub use worldgen::{Worldgen, WorldgenParams};
