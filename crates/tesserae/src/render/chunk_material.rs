//! The chunk material: `StandardMaterial` plus a colour per Material index.
//!
//! Extending `StandardMaterial` rather than writing a bare `Material` is what
//! buys shadows, clustered lighting, fog and tonemapping from `bevy_pbr` — the
//! decision in DESIGN.md's "Bevy depth" row. Step 9 replaces the flat palette
//! with a look-pack's texture array and keeps this shape.

use bevy::asset::AssetPath;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{
    ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline,
};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
};
use bevy::shader::{ShaderDefVal, ShaderRef};

use crate::client::chunk_mesh::{ATTRIBUTE_LIGHT, ATTRIBUTE_MATERIAL};

/// What to spawn in `MeshMaterial3d`.
pub type ChunkMaterial = ExtendedMaterial<StandardMaterial, ChunkMaterialExtension>;

/// Locations 0 to 7 are `bevy_pbr`'s own vertex attributes.
const MATERIAL_SHADER_LOCATION: u32 = 10;
const LIGHT_SHADER_LOCATION: u32 = 11;

/// Enough for `testgen`'s three Materials with room to look at them. The real
/// count is a Mod's business and arrives with the look-pack API in step 9.
pub const PALETTE_LEN: usize = 16;

/// What the vertex stage needs to know that is not in the mesh. Mirrors
/// `LodParams` in `chunk_material.wesl`; the two layouts must stay in step.
#[derive(Reflect, ShaderType, Debug, Clone, Copy)]
pub struct LodParams {
    /// The World's level-0 edge length, in metres. The per-level depth bias is a
    /// fraction of a Voxel, and a mesh does not carry its Voxel size.
    pub voxel_size: f32,
    /// Non-zero tints every level above 0 by its level, so a capture can show
    /// the rings as rings. A uniform and not a shader def, so it does not
    /// specialise a pipeline.
    pub tint: u32,
}

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct ChunkMaterialExtension {
    /// Indexed by Material id. Bindings 0 to 99 belong to the base
    /// `StandardMaterial`, so an extension starts at 100.
    #[uniform(100)]
    pub palette: [Vec4; PALETTE_LEN],
    #[uniform(101)]
    pub lod: LodParams,
}

/// `base_color` is linear in the shader, so a colour picked by eye has to be
/// converted or it renders several stops too bright.
fn srgb(r: f32, g: f32, b: f32) -> Vec4 {
    Color::srgb(r, g, b).to_linear().to_vec4()
}

impl Default for ChunkMaterialExtension {
    fn default() -> Self {
        // Air never emits a face, so magenta at index 0 means the mesher or the
        // vertex attribute is wrong rather than that the palette is short.
        let mut palette = [srgb(1.0, 0.0, 1.0); PALETTE_LEN];
        palette[tesserae_core::testgen::STONE as usize] = srgb(0.52, 0.52, 0.55);
        palette[tesserae_core::testgen::DIRT as usize] = srgb(0.42, 0.29, 0.18);
        palette[tesserae_core::testgen::GRASS as usize] = srgb(0.33, 0.56, 0.24);
        // The one Material that emits. Near-white, because the shader tints a
        // block-lit face by its own palette colour and a saturated emitter
        // would wash its own glow with its albedo twice over.
        palette[tesserae_core::worldgen::LANTERN as usize] = srgb(0.96, 0.89, 0.72);
        ChunkMaterialExtension {
            palette,
            lod: LodParams {
                voxel_size: 1.0,
                tint: 0,
            },
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

    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // The base material's layout stops at `bevy_pbr`'s attributes, so the
        // custom one has to be declared here or the vertex shader reads
        // garbage. Spelling the set out is safe because this material is only
        // ever used on meshes from `chunk_mesh_to_mesh`.
        descriptor.vertex.buffers = vec![layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
            ATTRIBUTE_MATERIAL.at_shader_location(MATERIAL_SHADER_LOCATION),
            ATTRIBUTE_LIGHT.at_shader_location(LIGHT_SHADER_LOCATION),
        ])?];

        // Reach the shader as `constants::*`, so neither the uniform's length
        // nor the light scale can drift from the Rust side.
        let defs = [
            ShaderDefVal::UInt("CHUNK_PALETTE_LEN".into(), PALETTE_LEN as u32),
            ShaderDefVal::UInt("CHUNK_MAX_LIGHT".into(), tesserae_core::MAX_LIGHT as u32),
        ];
        descriptor.vertex.shader_defs.extend_from_slice(&defs);
        if let Some(fragment) = descriptor.fragment.as_mut() {
            fragment.shader_defs.extend_from_slice(&defs);
        }
        Ok(())
    }
}

fn shader() -> ShaderRef {
    ShaderRef::Path(
        AssetPath::from_path_buf(bevy::asset::embedded_path!("chunk_material.wesl"))
            .with_source("embedded"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn testgen_materials_are_distinguishable_and_not_the_missing_colour() {
        let palette = ChunkMaterialExtension::default().palette;
        let missing = palette[0];
        let used = [
            tesserae_core::testgen::STONE,
            tesserae_core::testgen::DIRT,
            tesserae_core::testgen::GRASS,
            tesserae_core::worldgen::LANTERN,
        ];
        for id in used {
            assert_ne!(palette[id as usize], missing, "Material {id} unset");
        }
        for (i, a) in used.iter().enumerate() {
            for b in &used[i + 1..] {
                assert_ne!(palette[*a as usize], palette[*b as usize]);
            }
        }
    }

    #[test]
    fn every_material_id_the_shader_can_index_has_a_colour() {
        assert_eq!(ChunkMaterialExtension::default().palette.len(), PALETTE_LEN);
    }

    /// The shader clamps the Material index to the palette's last entry, so a
    /// Material past the end silently renders as whatever sits there. Every id
    /// `worldgen` can emit has to be inside it.
    #[test]
    fn no_material_worldgen_emits_falls_outside_the_palette() {
        assert!((tesserae_core::worldgen::LANTERN as usize) < PALETTE_LEN);
    }
}
