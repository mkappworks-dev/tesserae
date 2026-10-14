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

/// Mirrors `LodParams` in `chunk_material.wesl`; the two layouts must stay in step.
#[derive(Reflect, ShaderType, Debug, Clone, Copy)]
pub struct LodParams {
    /// Non-zero tints each level above 0, to show the rings in a capture. A
    /// uniform, not a shader def, so it does not specialise a pipeline.
    pub tint: u32,
    /// Per level, packed four to a `Vec4`. See `server::cuts` for why.
    pub cut_radius_m: [Vec4; CUT_VECS],
    pub cut_half_height_m: [Vec4; CUT_VECS],
}

/// Enough for every level the Server can have (32), four to a `Vec4`.
pub const CUT_VECS: usize = 8;

impl LodParams {
    pub fn new(tint: u32, cuts: &[(f32, f32)]) -> Self {
        let mut params = LodParams {
            tint,
            cut_radius_m: [Vec4::ZERO; CUT_VECS],
            cut_half_height_m: [Vec4::ZERO; CUT_VECS],
        };
        for (level, &(radius, half_height)) in cuts.iter().take(CUT_VECS * 4).enumerate() {
            params.cut_radius_m[level / 4][level % 4] = radius;
            params.cut_half_height_m[level / 4][level % 4] = half_height;
        }
        params
    }
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
        let mut palette = [srgb(1.0, 0.0, 1.0); PALETTE_LEN];
        palette[tesserae_core::testgen::STONE as usize] = srgb(0.52, 0.52, 0.55);
        palette[tesserae_core::testgen::DIRT as usize] = srgb(0.42, 0.29, 0.18);
        palette[tesserae_core::testgen::GRASS as usize] = srgb(0.33, 0.56, 0.24);
        // The one Material that emits. Near-white, because the shader tints a
        // block-lit face by its own palette colour and a saturated emitter
        // would wash its own glow with its albedo twice over.
        palette[tesserae_core::worldgen::LANTERN as usize] = srgb(0.96, 0.89, 0.72);
        // The class is Material data and the amount is appearance: a look pack
        // may choose both, so this is the placeholder.
        palette[tesserae_core::material::GLASS as usize] = srgba(0.70, 0.88, 0.92, 0.35);
        palette[tesserae_core::material::WATER as usize] = srgba(0.10, 0.32, 0.62, 0.65);
        palette[tesserae_core::material::LEAVES as usize] = srgb(0.16, 0.42, 0.14);
        ChunkMaterialExtension {
            palette,
            lod: LodParams::new(0, &[]),
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
    fn glass_and_water_are_see_through_and_everything_else_is_not() {
        use tesserae_core::material::{GLASS, LEAVES, WATER};
        let palette = ChunkMaterialExtension::default().palette;
        let missing = palette[0];
        for id in [GLASS, WATER] {
            let colour = palette[id as usize];
            assert!(colour.w > 0.0 && colour.w < 1.0, "Material {id} alpha");
            assert_ne!(colour.truncate(), missing.truncate(), "Material {id} unset");
        }
        assert_ne!(palette[GLASS as usize], palette[WATER as usize]);
        // Leaves draw in the opaque pass until step 9 gives them a texture to
        // cut out, so an alpha here would be ignored and mislead.
        assert_eq!(palette[LEAVES as usize].w, 1.0);
        assert_ne!(palette[LEAVES as usize].truncate(), missing.truncate());
        for id in [
            tesserae_core::testgen::STONE,
            tesserae_core::testgen::DIRT,
            tesserae_core::testgen::GRASS,
            tesserae_core::worldgen::LANTERN,
        ] {
            assert_eq!(palette[id as usize].w, 1.0, "Material {id} alpha");
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
