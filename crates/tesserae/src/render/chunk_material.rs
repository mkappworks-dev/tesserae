//! The chunk material: `StandardMaterial` plus Game appearance by Material id.
//!
//! Extending `StandardMaterial` rather than writing a bare `Material` is what
//! buys shadows, clustered lighting, fog and tonemapping from `bevy_pbr` — the
//! decision in DESIGN.md's "Bevy depth" row. The texture is shared with item
//! rendering; a later Look pack can replace appearance through this boundary.

use bevy::asset::AssetPath;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{
    ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline,
    MeshPipelineKey,
};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, FragmentState, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
};
use bevy::render::storage::ShaderBuffer;
use bevy::shader::{ShaderDefVal, ShaderRef};

use crate::client::chunk_mesh::{
    ATTRIBUTE_MATERIAL_LIGHT, ATTRIBUTE_POSITION_FACE, ATTRIBUTE_UV, LIGHT_SHIFT, STEPS,
};

/// What to spawn in `MeshMaterial3d`.
pub type ChunkMaterial = ExtendedMaterial<StandardMaterial, ChunkMaterialExtension>;

/// Material ids with a colour of their own.
pub const PALETTE_LEN: usize = 64;

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
    pub textured: UVec4,
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
            textured: UVec4::ZERO,
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

    fn prepass_fragment_shader() -> ShaderRef {
        embedded("chunk_prepass.wesl")
    }

    fn prepass_vertex_shader() -> ShaderRef {
        embedded("chunk_prepass.wesl")
    }

    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // A packed Chunk mesh has none of `bevy_pbr`'s attributes, so the base
        // material's layout is empty and the packed one has to be declared
        // here. Spelling the set out is safe because this material is only
        // ever used on meshes from `chunk_mesh_to_mesh`.
        descriptor.vertex.buffers = vec![layout.0.get_layout(&[
            ATTRIBUTE_POSITION_FACE.at_shader_location(0),
            ATTRIBUTE_UV.at_shader_location(1),
            ATTRIBUTE_MATERIAL_LIGHT.at_shader_location(2),
        ])?];

        // Reach the shader as `constants::*`, so neither the uniform's length
        // nor the light scale can drift from the Rust side.
        let defs = [
            ShaderDefVal::UInt("CHUNK_PALETTE_LEN".into(), PALETTE_LEN as u32),
            ShaderDefVal::UInt("CHUNK_PALETTE_SLOTS".into(), PALETTE_LEN as u32 + 1),
            ShaderDefVal::UInt("CHUNK_FLAME".into(), tesserae_core::mesh::FLAME),
            ShaderDefVal::UInt("CHUNK_MAX_LIGHT".into(), tesserae_core::MAX_LIGHT as u32),
            ShaderDefVal::UInt("CHUNK_VERTEX_STEPS".into(), STEPS as u32),
            ShaderDefVal::UInt("CHUNK_LIGHT_SHIFT".into(), LIGHT_SHIFT),
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
        cut_coverage_in_prepass(descriptor, key.mesh_key);
        descriptor.vertex.shader_defs.extend_from_slice(&defs);
        if let Some(fragment) = descriptor.fragment.as_mut() {
            fragment.shader_defs.extend_from_slice(&defs);
        }
        Ok(())
    }
}

/// Runs the coverage rule in a prepass, which `bevy_pbr` leaves depth-only for
/// an opaque material. Without it a coarse level writes depth where a finer one
/// draws, and the main pass, which loads that depth, loses the finer surface.
/// The sun's orthographic shadow views are skipped: coarse levels cast no
/// shadow, and a fragment stage would slow a depth-only pass for nothing.
fn cut_coverage_in_prepass(descriptor: &mut RenderPipelineDescriptor, key: MeshPipelineKey) {
    let prepass: ShaderDefVal = "PREPASS_PIPELINE".into();
    let orthographic = key.intersection(MeshPipelineKey::VIEW_PROJECTION_RESERVED_BITS)
        == MeshPipelineKey::VIEW_PROJECTION_ORTHOGRAPHIC;
    if !descriptor.vertex.shader_defs.contains(&prepass) || orthographic {
        return;
    }
    let fragment = descriptor.fragment.get_or_insert_with(|| FragmentState {
        shader: descriptor.vertex.shader.clone(),
        shader_defs: descriptor.vertex.shader_defs.clone(),
        ..default()
    });
    fragment.shader_defs.push("CHUNK_COVERAGE_CUT".into());
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
    use bevy::render::render_resource::VertexState;

    fn pipeline(defs: &[&'static str], fragment: bool) -> RenderPipelineDescriptor {
        let defs: Vec<ShaderDefVal> = defs.iter().map(|&d| d.into()).collect();
        let shader = Handle::<bevy::shader::Shader>::default();
        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: shader.clone(),
                shader_defs: defs.clone(),
                ..default()
            },
            fragment: fragment.then(|| FragmentState {
                shader,
                shader_defs: defs,
                ..default()
            }),
            ..default()
        }
    }

    fn cuts(descriptor: &RenderPipelineDescriptor) -> bool {
        let cut: ShaderDefVal = "CHUNK_COVERAGE_CUT".into();
        descriptor
            .fragment
            .as_ref()
            .is_some_and(|f| f.shader_defs.contains(&cut))
    }

    #[test]
    fn a_depth_only_camera_prepass_gains_a_fragment_stage_that_cuts_coverage() {
        let mut descriptor = pipeline(&["PREPASS_PIPELINE"], false);
        cut_coverage_in_prepass(
            &mut descriptor,
            MeshPipelineKey::DEPTH_PREPASS | MeshPipelineKey::VIEW_PROJECTION_NONSTANDARD,
        );
        assert!(cuts(&descriptor));
        let fragment = descriptor.fragment.unwrap();
        assert!(fragment.targets.is_empty(), "depth only");
        assert_eq!(fragment.shader, descriptor.vertex.shader);
    }

    #[test]
    fn a_cutout_prepass_keeps_its_fragment_stage_and_also_cuts_coverage() {
        let mut descriptor = pipeline(&["PREPASS_PIPELINE", "MAY_DISCARD"], true);
        cut_coverage_in_prepass(&mut descriptor, MeshPipelineKey::DEPTH_PREPASS);
        assert!(cuts(&descriptor));
        let may_discard: ShaderDefVal = "MAY_DISCARD".into();
        assert!(
            descriptor
                .fragment
                .unwrap()
                .shader_defs
                .contains(&may_discard)
        );
    }

    #[test]
    fn the_suns_shadow_views_and_the_main_pass_are_left_alone() {
        let shadow = pipeline(&["PREPASS_PIPELINE"], false);
        let mut cut = shadow.clone();
        cut_coverage_in_prepass(
            &mut cut,
            MeshPipelineKey::DEPTH_PREPASS | MeshPipelineKey::VIEW_PROJECTION_ORTHOGRAPHIC,
        );
        assert_eq!(cut, shadow);

        let main = pipeline(&["MESH_PIPELINE"], true);
        let mut cut = main.clone();
        cut_coverage_in_prepass(&mut cut, MeshPipelineKey::DEPTH_PREPASS);
        assert_eq!(cut, main);
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
        assert_eq!(palette.len(), 64 + 1, "the Materials' and the flames'");
        assert_ne!(palette[FLAME_SLOT], palette[0], "flames unset");
        assert!((tesserae_core::mesh::FLAME as usize) >= PALETTE_LEN);
    }
}
