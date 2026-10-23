//! Chunk storage, world generation and meshing for Tesserae.
//!
//! This crate depends on neither Bevy nor wgpu, and must not acquire such a
//! dependency even transitively: it is what keeps every expensive algorithm
//! headless, unit-testable and benchmarkable. CI enforces it in the
//! `Boundaries` job. See `docs/architecture.md`.

pub mod agenda;
pub mod chunk;
pub mod collide;
pub mod flow;
pub mod fluid;
pub mod light;
pub mod lod;
pub mod material;
pub mod mesh;
pub mod noise;
pub mod occupants;
pub mod shape;
pub mod tables;
pub mod testgen;
pub mod voxel;
pub mod worldgen;

pub use chunk::{CHUNK_SIZE, CHUNK_VOLUME, Chunk};
pub use fluid::{FluidDef, FluidError, FluidTable};
pub use light::{
    Channel, ChunkLight, Light, LightVolume, MAX_LIGHT, light_cells, light_chunk, relight,
};
pub use lod::{COVERED_SKY, CoarseLight, Surface, coarse_light, downsample};
pub use material::{MaterialDef, MaterialError, MaterialTable, Occlusion, RenderClass};
pub use mesh::{ChunkMesh, LitChunk, MeshLayer, Neighborhood, greedy_mesh, mesh_chunk};
pub use occupants::{
    Burning, ChunkOverlay, Fluid, Level, Occupants, Overlay, optical, optical_chunk,
};
pub use shape::{
    Arm, CollisionBox, ConnectDef, Connections, MAX_SHAPED_PER_CHUNK, NodeBox, Orientation,
    SHAPE_POST, SHAPE_SLAB, SHAPE_STAIR, ShapeDef, ShapeError, ShapeTable,
};
pub use tables::{PlaceFault, Tables};
pub use voxel::{AIR, MaterialId, SHAPE_FULL_CUBE, ShapeId, Voxel};
pub use worldgen::{
    CaveParams, MAX_HEIGHT_M, MAX_OCTAVES, MAX_SMOOTHED_PER_CHUNK, ParamError, SmoothingParams,
    WaterParams, Worldgen, WorldgenParams,
};
