//! The chunk material: `StandardMaterial` plus Game appearance by Material id.
//!
//! Extending `StandardMaterial` rather than writing a bare `Material` is what
//! buys shadows, clustered lighting, fog and tonemapping from `bevy_pbr` — the
//! decision in DESIGN.md's "Bevy depth" row. A later Look pack can replace
//! appearance through this boundary.

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

use super::water;
use crate::client::chunk_mesh;

/// What to spawn in `MeshMaterial3d`.
pub type ChunkMaterial = ExtendedMaterial<StandardMaterial, ChunkMaterialExtension>;

/// Material ids with a colour of their own.
pub const PALETTE_LEN: usize = 256;

/// One face word per palette slot and the flame slot, four to a `UVec4`.
pub const FACE_WORD_VECS: usize = (PALETTE_LEN + 1).div_ceil(4);

/// The palette slot after the Materials': what a [`tesserae_core::mesh::FLAME`]
/// quad is drawn in, so no Material's appearance row can recolour flames.
pub const FLAME_SLOT: usize = PALETTE_LEN;

/// The share of the sky's ambient light a block face gets by which way it faces, a top's being 1.
/// Bevy's ambient is the same from every direction, which lit a shaded step's side as brightly as a
/// tree's shadow on a top, so terraces read flat. The sun and cast shadows are left as they are.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkyView {
    /// A wall.
    pub side: f32,
    /// An underside.
    pub bottom: f32,
}

pub const SKY_VIEW: SkyView = SkyView {
    side: 0.45,
    bottom: 0.4,
};

const _: () =
    assert!(0.0 < SKY_VIEW.bottom && SKY_VIEW.bottom <= SKY_VIEW.side && SKY_VIEW.side < 1.0);

/// [`SKY_VIEW`] as the chunk shader's `constants::CHUNK_SKY_VIEW_*`: a shader def holds no float, so
/// each is its bit pattern, which the shader bitcasts back.
fn sky_view_shader_defs() -> Vec<ShaderDefVal> {
    let bits = |name: &'static str, value: f32| ShaderDefVal::UInt(name.into(), value.to_bits());
    vec![
        bits("CHUNK_SKY_VIEW_SIDE", SKY_VIEW.side),
        bits("CHUNK_SKY_VIEW_BOTTOM", SKY_VIEW.bottom),
    ]
}

/// The noon floor a face no sky light reaches keeps, as `constants::CHUNK_CAVE_FLOOR` (a share)
/// and `constants::CHUNK_NOON_AMBIENT` (the noon ambient it is a share of), bit patterns as above.
fn cave_floor_shader_defs() -> Vec<ShaderDefVal> {
    let bits = |name: &'static str, value: f32| ShaderDefVal::UInt(name.into(), value.to_bits());
    vec![
        bits("CHUNK_CAVE_FLOOR", tesserae_core::sky::CAVE_FLOOR),
        bits("CHUNK_NOON_AMBIENT", crate::client::chunk_view::SKY_AMBIENT),
    ]
}

/// Mirrors `LodParams` in `chunk_material.wesl`; the two layouts must stay in step.
#[derive(Reflect, ShaderType, Debug, Clone, Copy, Default)]
pub struct LodParams {
    /// Non-zero tints each level above 0, to show the rings in a capture. A
    /// uniform, not a shader def, so it does not specialise a pipeline.
    pub tint: u32,
}

/// Mirrors `WaterView` in `water.wesl`: what water reflects, kept in step with the sky's, and
/// where its Flow fades.
#[derive(Reflect, ShaderType, Debug, Clone, Copy, Default, PartialEq)]
pub struct WaterView {
    /// The sky material's zenith, linear.
    pub zenith: Vec4,
    /// x and y: the distances in metres over which Flow fades to calm water.
    pub flow_fade_m: Vec4,
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
    /// One layer per distinct Material face, found through `face_words`.
    #[texture(103, dimension = "2d_array")]
    #[sampler(104)]
    pub texture: Option<Handle<Image>>,
    /// Per slot, `texture_atlas::face_word`: zero is untextured.
    #[uniform(105)]
    pub face_words: [UVec4; FACE_WORD_VECS],
    /// Every Chunk layer's quads, in `render::quad_buffer`'s ranges.
    #[storage(106, read_only)]
    pub quads: Handle<ShaderBuffer>,
    /// `water::table`'s rows: which slots are water and how each looks.
    #[uniform(107)]
    pub water: [Vec4; water::TABLE_LEN],
    /// `water::normal_map`; `None` until the Client has built it.
    #[texture(108)]
    #[sampler(109)]
    pub water_normals: Option<Handle<Image>>,
    #[uniform(110)]
    pub water_view: WaterView,
    /// `water::foam_mask`; `None` until the Client has built it.
    #[texture(111)]
    #[sampler(112)]
    pub water_foam: Option<Handle<Image>>,
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
            face_words: [UVec4::ZERO; FACE_WORD_VECS],
            quads: Handle::default(),
            water: [Vec4::ZERO; water::TABLE_LEN],
            water_normals: None,
            water_view: WaterView::default(),
            water_foam: None,
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
        // the light scale, the face words nor the quad layout can drift from the Rust side.
        let mut defs = vec![
            ShaderDefVal::UInt("CHUNK_PALETTE_LEN".into(), PALETTE_LEN as u32),
            ShaderDefVal::UInt("CHUNK_PALETTE_SLOTS".into(), PALETTE_LEN as u32 + 1),
            ShaderDefVal::UInt("CHUNK_FLAME".into(), tesserae_core::mesh::FLAME),
            ShaderDefVal::UInt("CHUNK_MAX_LIGHT".into(), tesserae_core::MAX_LIGHT as u32),
            ShaderDefVal::UInt(
                "CHUNK_TEXTURE_EDGE".into(),
                super::texture_atlas::EDGE as u32,
            ),
            ShaderDefVal::UInt("CHUNK_WATER_ROWS".into(), water::TABLE_LEN as u32),
        ];
        defs.extend(water::ripple_shader_defs());
        defs.extend(water::column_shader_defs());
        defs.extend(super::texture_atlas::face_word_shader_defs());
        defs.extend(quad_layout_defs());
        defs.extend(super::fog::haze_shader_defs());
        defs.extend(sky_view_shader_defs());
        defs.extend(cave_floor_shader_defs());
        #[cfg(feature = "profiling")]
        let switched = crate::profiling::switch("TESSERAE_DIAG_NO_COVERAGE_DISCARD");
        #[cfg(not(feature = "profiling"))]
        let switched = false;
        #[cfg(feature = "profiling")]
        defs.extend(held_water_clock());
        if casts_the_suns_shadow(descriptor) {
            defs.push(ShaderDefVal::Bool("CHUNK_SUN_SHADOW".into(), true));
        }
        if !key.bind_group_data.coverage_test || switched {
            defs.push(ShaderDefVal::Bool("CHUNK_NO_COVERAGE_DISCARD".into(), true));
        }
        descriptor.vertex.shader_defs.extend_from_slice(&defs);
        if let Some(fragment) = descriptor.fragment.as_mut() {
            fragment.shader_defs.extend_from_slice(&defs);
            // AO's prepass draws level 0 only, and water is not in it, so its normal would be
            // empty or the floor's; a Voxel face's own normal is exact.
            fragment
                .shader_defs
                .retain(|def| def != &ShaderDefVal::from("LOAD_PREPASS_NORMALS"));
        }
        Ok(())
    }
}

/// `TESSERAE_WATER_CLOCK_S` holds water's clock at that many seconds, for a capture pair.
#[cfg(feature = "profiling")]
fn held_water_clock() -> Vec<ShaderDefVal> {
    let Some(seconds) = std::env::var("TESSERAE_WATER_CLOCK_S")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
    else {
        return Vec::new();
    };
    vec![
        ShaderDefVal::Bool("CHUNK_WATER_CLOCK_HELD".into(), true),
        ShaderDefVal::UInt("CHUNK_WATER_CLOCK_S".into(), seconds.to_bits()),
    ]
}

/// Bevy draws a directional light's cascades with unclipped depth, natively or
/// emulated; no other pass does.
fn casts_the_suns_shadow(descriptor: &RenderPipelineDescriptor) -> bool {
    descriptor.primitive.unclipped_depth
        || descriptor
            .vertex
            .shader_defs
            .contains(&ShaderDefVal::from("UNCLIPPED_DEPTH_ORTHO_EMULATION"))
}

/// What `chunk_vertex.wesl` decodes a layer's words by.
fn quad_layout_defs() -> Vec<ShaderDefVal> {
    use chunk_mesh::{
        HEADER_LEVEL, HEADER_QUADS, HEADER_STRIDE, HEADER_TABLE_LEN, HEADER_WATER, HEADER_WORDS,
        LIGHT_SHIFT, VERTICES_PER_QUAD, WIDE,
    };
    use tesserae_core::mesh::layout::*;
    let uint = |name: &str, value: u32| ShaderDefVal::UInt(format!("CHUNK_{name}").into(), value);
    vec![
        uint("VERTICES_PER_QUAD", VERTICES_PER_QUAD),
        uint("HEADER_WORDS", HEADER_WORDS),
        uint("HEADER_LEVEL", HEADER_LEVEL),
        uint("HEADER_TABLE_LEN", HEADER_TABLE_LEN),
        uint("HEADER_STRIDE", HEADER_STRIDE),
        uint("HEADER_QUADS", HEADER_QUADS),
        uint("HEADER_WATER", HEADER_WATER),
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
/// placeholder from `chunk_mesh::placeholder` for its bucket's vertex count. Bevy's
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
    fn the_shader_reads_the_sky_view_it_is_given() {
        let defs = sky_view_shader_defs();
        let read = |name: &str| {
            defs.iter()
                .find_map(|def| match def {
                    ShaderDefVal::UInt(key, bits) if key == name => Some(f32::from_bits(*bits)),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("{name} missing"))
        };
        assert_eq!(read("CHUNK_SKY_VIEW_SIDE"), SKY_VIEW.side);
        assert_eq!(read("CHUNK_SKY_VIEW_BOTTOM"), SKY_VIEW.bottom);
    }

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
