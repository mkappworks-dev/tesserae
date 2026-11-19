//! The chunk material: `StandardMaterial` plus Game appearance by Material id.
//!
//! Extending `StandardMaterial` rather than writing a bare `Material` is what
//! buys shadows, clustered lighting, fog and tonemapping from `bevy_pbr` — the
//! decision in DESIGN.md's "Bevy depth" row. The texture is shared with item
//! rendering; a later Look pack can replace appearance through this boundary.

use bevy::asset::AssetPath;
use bevy::mesh::{MeshVertexBufferLayoutRef, VertexBufferLayout};
use bevy::pbr::{
    ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline,
};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError, VertexState,
    VertexStepMode,
};
use bevy::render::storage::ShaderBuffer;
use bevy::shader::{ShaderDefVal, ShaderRef};

use crate::client::chunk_mesh;

/// What to spawn in `MeshMaterial3d`.
pub type ChunkMaterial = ExtendedMaterial<StandardMaterial, ChunkMaterialExtension>;

/// Material ids with a colour of their own.
pub const PALETTE_LEN: usize = 256;

/// One bit per palette slot and the flame slot, in `UVec4`s of 128 bits.
pub const TEXTURED_VECS: usize = (PALETTE_LEN + 1).div_ceil(128);

/// The palette slot after the Materials': what a [`tesserae_core::mesh::FLAME`]
/// quad is drawn in, so no Material's appearance row can recolour flames.
pub const FLAME_SLOT: usize = PALETTE_LEN;

/// Mirrors `LodParams` in `chunk_material.wesl`; the two layouts must stay in step.
#[derive(Reflect, ShaderType, Debug, Clone, Copy, Default)]
pub struct LodParams {
    /// Non-zero tints each level above 0, to show the rings in a capture. A
    /// uniform, not a shader def, so it does not specialise a pipeline.
    pub tint: u32,
}

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
#[bind_group_data(ChunkMaterialKey)]
pub struct ChunkMaterialExtension {
    /// Indexed by Material id. Bindings 0 to 99 belong to the base
    /// `StandardMaterial`, so an extension starts at 100.
    #[uniform(100)]
    pub palette: [Vec4; PALETTE_LEN + 1],
    #[uniform(101)]
    pub lod: LodParams,
    /// The Chunks each level draws, in `client::drawn_mask`'s layout. A buffer
    /// of one zero word discards nothing.
    #[storage(102, read_only)]
    pub drawn: Handle<ShaderBuffer>,
    #[texture(103)]
    #[sampler(104)]
    pub texture: Option<Handle<Image>>,
    #[uniform(105)]
    pub textured: [UVec4; TEXTURED_VECS],
    /// Every Chunk layer's quads, in `render::quad_buffer`'s ranges.
    #[storage(106, read_only)]
    pub quads: Handle<ShaderBuffer>,
    /// Whether the pipeline holds the coverage test's `discard`. A Chunk no
    /// finer level draws any of takes a material without it: on Apple's
    /// tile-based GPUs a pipeline that may discard defeats hidden-surface
    /// removal, which cost 35% of the GPU frame at 1440p.
    pub coverage_test: bool,
}

/// The pipeline variant a [`ChunkMaterialExtension`] specialises.
#[repr(C)]
#[derive(Eq, PartialEq, Hash, Copy, Clone)]
pub struct ChunkMaterialKey {
    coverage_test: bool,
}

impl From<&ChunkMaterialExtension> for ChunkMaterialKey {
    fn from(extension: &ChunkMaterialExtension) -> Self {
        Self {
            coverage_test: extension.coverage_test,
        }
    }
}

/// `base_color` is linear in the shader, so a colour picked by eye has to be
/// converted or it renders several stops too bright.
fn srgb(r: f32, g: f32, b: f32) -> Vec4 {
    srgba(r, g, b, 1.0)
}

/// Alpha is linear already and only matters to the blended pass.
fn srgba(r: f32, g: f32, b: f32, a: f32) -> Vec4 {
    Color::srgba(r, g, b, a).to_linear().to_vec4()
}

impl Default for ChunkMaterialExtension {
    fn default() -> Self {
        // Air never emits a face, so magenta at index 0 means the mesher or the
        // vertex attribute is wrong rather than that the palette is short.
        let mut palette = [srgb(1.0, 0.0, 1.0); PALETTE_LEN + 1];
        palette[FLAME_SLOT] = srgb(1.0, 0.86, 0.55);
        ChunkMaterialExtension {
            palette,
            lod: LodParams::default(),
            drawn: Handle::default(),
            texture: None,
            textured: [UVec4::ZERO; TEXTURED_VECS],
            quads: Handle::default(),
            coverage_test: true,
        }
    }
}

// deferred_vertex_shader and deferred_fragment_shader are deliberately left at
// the base material's: forward is Bevy's default and DESIGN.md leaves
// forward-vs-deferred open until there are real scenes to measure. Chunks would
// lose their Material colour in a deferred pass until that is settled.
impl MaterialExtension for ChunkMaterialExtension {
    fn vertex_shader() -> ShaderRef {
        shader()
    }

    fn fragment_shader() -> ShaderRef {
        shader()
    }

    // Defining the prepass shaders is also what keeps the material bind group,
    // and the quad buffer in it, bound in the shadow and depth passes.
    fn prepass_fragment_shader() -> ShaderRef {
        embedded("chunk_prepass.wesl")
    }

    fn prepass_vertex_shader() -> ShaderRef {
        embedded("chunk_prepass.wesl")
    }

    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        read_no_vertex_attribute(&mut descriptor.vertex);

        // Reach the shader as `constants::*`, so neither the uniform's length,
        // the light scale nor the quad layout can drift from the Rust side.
        let mut defs = vec![
            ShaderDefVal::UInt("CHUNK_PALETTE_LEN".into(), PALETTE_LEN as u32),
            ShaderDefVal::UInt("CHUNK_PALETTE_SLOTS".into(), PALETTE_LEN as u32 + 1),
            ShaderDefVal::UInt("CHUNK_FLAME".into(), tesserae_core::mesh::FLAME),
            ShaderDefVal::UInt("CHUNK_MAX_LIGHT".into(), tesserae_core::MAX_LIGHT as u32),
            ShaderDefVal::UInt(
                "CHUNK_TEXTURE_EDGE".into(),
                super::texture_atlas::EDGE as u32,
            ),
            ShaderDefVal::UInt(
                "CHUNK_TEXTURE_WIDTH".into(),
                super::texture_atlas::WIDTH as u32,
            ),
            ShaderDefVal::UInt(
                "CHUNK_TEXTURE_HEIGHT".into(),
                super::texture_atlas::HEIGHT as u32,
            ),
        ];
        defs.extend(quad_layout_defs());
        #[cfg(feature = "profiling")]
        let switched = crate::profiling::switch("TESSERAE_DIAG_NO_COVERAGE_DISCARD");
        #[cfg(not(feature = "profiling"))]
        let switched = false;
        if !key.bind_group_data.coverage_test || switched {
            defs.push(ShaderDefVal::Bool("CHUNK_NO_COVERAGE_DISCARD".into(), true));
        }
        descriptor.vertex.shader_defs.extend_from_slice(&defs);
        if let Some(fragment) = descriptor.fragment.as_mut() {
            fragment.shader_defs.extend_from_slice(&defs);
        }
        Ok(())
    }
}

/// What `chunk_vertex.wesl` decodes a layer's words by.
fn quad_layout_defs() -> Vec<ShaderDefVal> {
    use chunk_mesh::{
        HEADER_LEVEL, HEADER_STRIDE, HEADER_TABLE_LEN, HEADER_WORDS, LIGHT_SHIFT,
        VERTICES_PER_QUAD, WIDE,
    };
    use tesserae_core::mesh::layout::*;
    let uint = |name: &str, value: u32| ShaderDefVal::UInt(format!("CHUNK_{name}").into(), value);
    vec![
        uint("VERTICES_PER_QUAD", VERTICES_PER_QUAD),
        uint("HEADER_WORDS", HEADER_WORDS),
        uint("HEADER_LEVEL", HEADER_LEVEL),
        uint("HEADER_TABLE_LEN", HEADER_TABLE_LEN),
        uint("HEADER_STRIDE", HEADER_STRIDE),
        uint("WIDE_STRIDE", WIDE),
        uint("LIGHT_SHIFT", LIGHT_SHIFT),
        uint("PAINT_SHIFT", PAINT_SHIFT),
        uint("SUBDIV", tesserae_core::shape::SUBDIV as u32),
        uint("FACES", FACES),
        uint("KIND_BITS", KIND_BITS),
        uint("KIND_CROSS", KIND_CROSS),
        ShaderDefVal::Int("CHUNK_PLANE_BIAS".into(), PLANE_BIAS),
        uint("PLANE_SHIFT", PLANE_SHIFT),
        uint("PLANE_BITS", PLANE_BITS),
        uint("CORNER_BITS", CORNER_BITS),
        uint("U0_SHIFT", U0_SHIFT),
        uint("V0_SHIFT", V0_SHIFT),
        uint("DV_SHIFT", DV_SHIFT),
        uint("CELL_SHIFT", CELL_SHIFT),
        uint("CELL_BITS", CELL_BITS),
        uint("DIAGONAL_SHIFT", DIAGONAL_SHIFT),
        uint("AXIS_SHIFT", AXIS_SHIFT),
        uint("AXIS_BITS", AXIS_BITS),
        uint("REVERSED_SHIFT", REVERSED_SHIFT),
        uint("OFFSET_SHIFT", OFFSET_SHIFT),
        uint("OFFSET_BITS", OFFSET_BITS),
        uint("SPAN_BITS", SPAN_BITS),
    ]
}

/// The vertex shader reads the quad buffer, never the mesh, which is a
/// placeholder from `chunk_mesh::placeholder` for its vertex count. Bevy's
/// specializer keys pipelines by `buffers[0]`, so the slot stays, empty; a
/// zero stride, as wgpu wants a multiple of four and the placeholder's is one.
fn read_no_vertex_attribute(vertex: &mut VertexState) {
    vertex.buffers = vec![VertexBufferLayout {
        array_stride: 0,
        step_mode: VertexStepMode::Vertex,
        attributes: Vec::new(),
    }];
}

fn shader() -> ShaderRef {
    embedded("chunk_material.wesl")
}

fn embedded(file: &str) -> ShaderRef {
    ShaderRef::Path(
        AssetPath::from_path_buf(bevy::asset::embedded_path!(file)).with_source("embedded"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::render::render_resource::VertexFormat;

    #[test]
    fn the_pipeline_keeps_the_vertex_buffer_bevy_reads_but_no_attribute() {
        let mut vertex = VertexState {
            buffers: vec![VertexBufferLayout::from_vertex_formats(
                VertexStepMode::Vertex,
                [VertexFormat::Uint8],
            )],
            ..default()
        };
        read_no_vertex_attribute(&mut vertex);
        let [buffer] = vertex.buffers.as_slice() else {
            panic!(
                "Bevy's specializer indexes buffers[0]: {:?}",
                vertex.buffers
            );
        };
        assert!(buffer.attributes.is_empty());
        assert_eq!(buffer.array_stride % 4, 0, "wgpu's VERTEX_ALIGNMENT");
    }

    #[test]
    fn only_flames_have_a_builtin_colour() {
        let palette = ChunkMaterialExtension::default().palette;
        assert!(
            palette[..PALETTE_LEN]
                .iter()
                .all(|colour| *colour == palette[0])
        );
        assert_ne!(palette[FLAME_SLOT], palette[0]);
    }

    #[test]
    fn every_material_id_the_shader_can_index_has_a_colour() {
        let palette = ChunkMaterialExtension::default().palette;
        assert_eq!(
            palette.len(),
            PALETTE_LEN + 1,
            "the Materials' and the flames'"
        );
        assert_ne!(palette[FLAME_SLOT], palette[0], "flames unset");
        assert!((tesserae_core::mesh::FLAME as usize) >= PALETTE_LEN);
    }
}
