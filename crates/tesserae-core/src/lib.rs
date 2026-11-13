//! Chunk storage, world generation and meshing for Tesserae.
//!
//! This crate depends on neither Bevy nor wgpu, and must not acquire such a
//! dependency even transitively: it is what keeps every expensive algorithm
//! headless, unit-testable and benchmarkable. CI enforces it in the
//! `Boundaries` job. See `docs/architecture.md`.

pub mod agenda;
pub mod audio;
pub mod biome_edits;
pub mod chunk;
pub mod climate;
pub mod collide;
pub mod combustion;
pub mod craft;
pub mod entity;
pub mod fire;
pub mod flow;
pub mod fluid;
pub mod immersion;
pub mod inventory;
pub mod layout;
pub mod light;
pub mod lod;
pub mod material;
pub mod mesh;
pub mod noise;
pub mod occupants;
pub mod ore;
pub mod reaction;
pub mod save;
pub mod seed;
pub mod shape;
pub mod survival;
pub mod tables;
pub mod testgen;
pub mod usable;
pub mod visibility;
pub mod voxel;
pub mod worldgen;

pub use chunk::{CHUNK_SIZE, CHUNK_VOLUME, Chunk};
pub use climate::{
    Biome, BiomeId, BiomeName, BiomeTable, Climate, ClimateParams, MAX_BIOME_NAME_BYTES,
    MAX_BIOMES, MAX_CAVE_SCALE, MAX_SURFACE_FEATURES, SURFACE_FEATURE_SPACING_M, SkyFog,
    SkyFogBlend, SurfaceFeature, SurfaceFeatures,
};
pub use combustion::{Burn, CombustionDef, CombustionError, CombustionTable, Heat, MAX_HEAT};
pub use fluid::{FluidDef, FluidError, FluidTable};
pub use light::{
    Channel, ChunkLight, Light, LightVolume, MAX_LIGHT, light_cells, light_chunk, relight,
    relight_many,
};
pub use lod::{
    COVERED_SKY, CoarseLight, SeamMargin, Surface, coarse_light, downsample, seam_openings,
};
pub use material::{MaterialDef, MaterialError, MaterialTable, Occlusion, RenderClass};
pub use mesh::{
    ChunkMesh, LitChunk, MeshLayer, Neighborhood, SeamOpenings, greedy_mesh, mesh_chunk,
    mesh_chunk_with_seams,
};
pub use occupants::{
    Burning, ChunkOverlay, Fluid, Level, Occupants, Overlay, optical, optical_chunk,
};
pub use ore::{BiomeSet, MAX_ORES, Ore, OreTable};
pub use reaction::{
    Outcome, Participant, Phase, Reactant, ReactionDef, ReactionError, ReactionTable, Side,
};
pub use shape::{
    Arm, CollisionBox, ConnectDef, Connections, MAX_SHAPED_PER_CHUNK, NodeBox, Orientation,
    SHAPE_POST, SHAPE_SLAB, SHAPE_STAIR, ShapeDef, ShapeError, ShapeTable,
};
pub use survival::SurvivalRules;
pub use tables::{PlaceFault, Tables};
pub use usable::{CONTAINER_SLOTS, Door, MAX_CONTAINER_SLOTS, UseError, VoxelUse, VoxelUses};
pub use voxel::{AIR, MaterialId, SHAPE_FULL_CUBE, ShapeId, Voxel};
pub use worldgen::{
    CaveParams, GroundParams, MAX_HEIGHT_M, MAX_OCTAVES, MAX_SMOOTHED_PER_CHUNK, ParamError,
    SmoothingParams, WaterParams, Worldgen, WorldgenParams,
};
