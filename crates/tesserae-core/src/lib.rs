//! Chunk storage, world generation and meshing for Tesserae.
//!
//! This crate depends on neither Bevy nor wgpu, and must not acquire such a
//! dependency even transitively: it is what keeps every expensive algorithm
//! headless, unit-testable and benchmarkable. CI enforces it in the
//! `Boundaries` job. See `docs/architecture.md`.

pub mod chunk;
pub mod collide;
pub mod light;
pub mod mesh;
pub mod noise;
pub mod shape;
pub mod testgen;
pub mod voxel;
pub mod worldgen;

pub use chunk::{CHUNK_SIZE, CHUNK_VOLUME, Chunk};
pub use light::{Channel, ChunkLight, Light, LightVolume, MAX_LIGHT, light_chunk, relight};
pub use mesh::{ChunkMesh, LitChunk, Neighborhood, greedy_mesh, mesh_chunk};
pub use shape::{
    MAX_SHAPED_PER_CHUNK, NodeBox, Orientation, SHAPE_POST, SHAPE_SLAB, SHAPE_STAIR, ShapeDef,
    ShapeError, ShapeTable,
};
pub use voxel::{AIR, MaterialId, SHAPE_FULL_CUBE, ShapeId, Voxel};
pub use worldgen::{CaveParams, MAX_SMOOTHED_PER_CHUNK, SmoothingParams, Worldgen, WorldgenParams};
