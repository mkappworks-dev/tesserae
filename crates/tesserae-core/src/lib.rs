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
pub(crate) mod drainage;
pub mod entity;
pub mod fire;
pub mod flow;
pub mod fluid;
pub mod immersion;
pub mod inventory;
pub mod layout;
pub mod light;
pub mod lod;
pub mod loot;
pub mod map;
pub mod marker;
pub mod material;
pub mod mesh;
pub mod noise;
pub mod occupants;
pub mod ore;
pub mod plant;
pub mod reaction;
pub mod save;
pub mod seed;
pub mod settlement;
pub mod shape;
pub mod survival;
pub mod tables;
pub mod template;
pub mod terrain;
pub mod testgen;
pub mod underground;
pub mod usable;
pub mod visibility;
pub mod voxel;
pub mod worldgen;

pub use chunk::{CHUNK_SIZE, CHUNK_VOLUME, Chunk};
pub use climate::{
    Biome, BiomeId, BiomeName, BiomeSelectors, BiomeSettlement, BiomeStructure, BiomeTable,
    CLUMP_SCALE_M, Climate, ClimateParams, Clump, CropField, FIELD_PLOT_M, FIELD_SCALE_M,
    FIELD_SPACING_M, Field, FieldCrop, MAX_BIOME_NAME_BYTES, MAX_BIOME_SETTLEMENTS,
    MAX_BIOME_STRUCTURES, MAX_BIOMES, MAX_CAVE_SCALE, MAX_CROP_FIELDS, MAX_FIELD_CROPS,
    MAX_FIELD_STRAYS, MAX_RELIEF_SCALE, MAX_STRATA, MAX_STRATUM_DEPTH_M, MAX_STRUCTURE_SPACING_M,
    MAX_SURFACE_FEATURES, MAX_SURFACE_FRUIT_STAGES, MAX_SURFACE_STAGES, MAX_VINE_DENSITY,
    MAX_VINE_LENGTH_M, SURFACE_FEATURE_SPACING_M, Site, SkyFog, SkyFogBlend, Strata, Stratum,
    SurfaceFeature, SurfaceFeatures, SurfaceFruit, SurfacePlant, SurfacePlantKind, SurfaceTree,
    SurfaceVine, SurfaceVines, VINE_FACES,
};
pub use combustion::{Burn, CombustionDef, CombustionError, CombustionTable, Heat, MAX_HEAT};
pub use fluid::{FluidDef, FluidError, FluidTable};
pub use light::{
    Channel, ChunkLight, Light, LightVolume, MAX_LIGHT, light_cells, light_chunk, relight,
    relight_many,
};
pub use lod::{
    COVERED_SKY, CoarseLight, SeamMargin, Surface, agree_with_level_zero, coarse_light, downsample,
    seam_openings,
};
pub use material::{MaterialDef, MaterialError, MaterialTable, Occlusion, RenderClass};
pub use mesh::{
    ChunkMesh, LitChunk, MeshLayer, Neighborhood, SeamOpenings, emits_no_faces, greedy_mesh,
    mesh_chunk, mesh_chunk_with_seams,
};
pub use occupants::{
    Burning, ChunkOverlay, Fluid, Level, Occupants, Overlay, optical, optical_chunk,
};
pub use ore::{BiomeSet, MAX_ORES, Ore, OreTable};
pub use plant::{PlantError, PlantRef, PlantTable, Role as PlantRole};
pub use reaction::{
    Outcome, Participant, Phase, Reactant, ReactionDef, ReactionError, ReactionTable, Side,
};
pub use shape::{
    Arm, CollisionBox, ConnectDef, Connections, Form, MAX_SHAPED_PER_CHUNK, NodeBox, Orientation,
    SHAPE_POST, SHAPE_SLAB, SHAPE_STAIR, ShapeDef, ShapeError, ShapeTable,
};
pub use survival::SurvivalRules;
pub use tables::{PlaceFault, Tables};
pub use underground::{
    CaveBiome, CaveBiomeTable, CaveFeature, MAX_CAVE_BIOMES, MAX_CAVE_FEATURE_SPACING_M,
    MAX_CAVE_FEATURES,
};
pub use usable::{
    Across, CONTAINER_SLOTS, DOOR_HEIGHT_M, DOOR_HEIGHTS_M, DOOR_WIDTH_M, DOOR_WIDTHS_M, Door,
    MAX_CONTAINER_SLOTS, Rise, UseError, VoxelUse, VoxelUses, door_faces, facing, placed_in,
};
pub use voxel::{AIR, MaterialId, SHAPE_FULL_CUBE, ShapeId, Voxel};
pub use worldgen::{
    CaveParams, ColumnTop, GroundParams, MAX_HEIGHT_M, MAX_OCTAVES, MAX_SMOOTHED_PER_CHUNK,
    ParamError, SmoothingParams, WaterParams, Worldgen, WorldgenParams,
};
