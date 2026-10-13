//! `ChunkMesh` → Bevy `Mesh`. The one place Voxel-index space becomes metres.
//!
//! This conversion lives in `tesserae` and never in `tesserae-core`, which must
//! acquire no `bevy` or `wgpu` dependency even transitively. See
//! `docs/architecture.md` invariant 3.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, MeshVertexAttribute, PrimitiveTopology, VertexFormat};
use bevy::prelude::*;
use tesserae_core::MeshLayer;

/// Which Material each vertex belongs to, for the chunk [`MaterialExtension`].
///
/// A high id keeps custom attributes sorted after Bevy's built-ins and away from
/// their ids. `Uint32` with the Material in the low 16 bits.
///
/// The high 16 are spare and stay that way. A Shape's geometry is already in the
/// mesh, so nothing in the shader varies by it; what would earn the bits is a
/// look pack that shades by Shape (wind in a fence, say), which is step 9's
/// decision and not worth bytes per vertex before then.
///
/// [`MaterialExtension`]: bevy::pbr::MaterialExtension
pub const ATTRIBUTE_MATERIAL: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_Material", 2_084_155_061, VertexFormat::Uint32);

/// The packed sky and block light of the face each vertex belongs to.
///
/// Baked per vertex rather than sampled on the GPU: thousands of torches as
/// point lights is not viable on any hardware, so light is a per-Voxel
/// flood-fill and this attribute is how it reaches the shader.
///
/// `Uint32` for one byte of payload. Folding it into [`ATTRIBUTE_MATERIAL`]'s
/// spare 16 bits would save four bytes a vertex, which is noise against the 32 in
/// position, normal and UV and not worth entangling two attributes for.
pub const ATTRIBUTE_LIGHT: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_Light", 2_084_155_062, VertexFormat::Uint32);

/// Positions are scaled by `voxel_size` here, so everything downstream — the
/// `Aabb` Bevy derives, frustum culling, shadow bounds — is already in metres.
pub fn chunk_mesh_to_mesh(layer: &MeshLayer, voxel_size: f32) -> Mesh {
    let positions: Vec<[f32; 3]> = layer
        .positions
        .iter()
        .map(|p| [p[0] * voxel_size, p[1] * voxel_size, p[2] * voxel_size])
        .collect();

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, layer.normals.clone())
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, layer.uvs.clone())
    .with_inserted_attribute(ATTRIBUTE_MATERIAL, layer.materials.clone())
    // Unscaled, unlike the positions above: a light level counts attenuation
    // steps and is not a distance, so `voxel_size` has no business touching it.
    .with_inserted_attribute(ATTRIBUTE_LIGHT, layer.light.clone())
    .with_inserted_indices(Indices::U32(layer.indices.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::mesh::VertexAttributeValues;
    use tesserae_core::light::{Light, MAX_LIGHT};
    use tesserae_core::{
        Chunk, ChunkLight, ChunkMesh, LitChunk, Neighborhood, Tables, Voxel, greedy_mesh, testgen,
    };

    /// Full sky, which is what the solver answers for open terrain.
    static SKY: ChunkLight = ChunkLight::filled(Light::new(MAX_LIGHT, 0));

    fn terrain_mesh(voxel_size: f32) -> (ChunkMesh, Mesh) {
        let chunk = testgen::terrain_chunk(0, 0, 0);
        let source = greedy_mesh(
            &Neighborhood::isolated(LitChunk::new(&chunk, &SKY)),
            &Tables::builtin(),
        );
        let mesh = chunk_mesh_to_mesh(&source.opaque, voxel_size);
        (source, mesh)
    }

    #[test]
    fn every_attribute_the_chunk_material_reads_is_present() {
        let (source, mesh) = terrain_mesh(1.0);
        for attribute in [
            Mesh::ATTRIBUTE_POSITION.id,
            Mesh::ATTRIBUTE_NORMAL.id,
            Mesh::ATTRIBUTE_UV_0.id,
            ATTRIBUTE_MATERIAL.id,
            ATTRIBUTE_LIGHT.id,
        ] {
            assert!(mesh.attribute(attribute).is_some(), "{attribute:?} missing");
        }
        assert_eq!(mesh.count_vertices(), source.opaque.positions.len());
        assert_eq!(mesh.indices().unwrap().len(), source.opaque.indices.len());
    }

    #[test]
    fn positions_are_metres_so_voxel_size_scales_the_chunk() {
        let (source, mesh) = terrain_mesh(0.25);
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("positions are Float32x3");
        };
        for (scaled, unscaled) in positions.iter().zip(&source.opaque.positions) {
            assert_eq!(scaled, &unscaled.map(|c| c * 0.25));
        }
        // A 32-Voxel Chunk at 0.25m is an 8m Cell, whatever the terrain inside.
        let widest = positions.iter().map(|p| p[0]).fold(0.0f32, f32::max);
        assert!(widest <= 8.0, "{widest} m exceeds the Cell");
    }

    #[test]
    fn a_coarse_level_scales_by_its_own_voxel_size() {
        use crate::world::WorldSettings;
        let world = WorldSettings {
            voxel_size: 0.25,
            ..default()
        };
        // Level 3 is 2 m Voxels, so a 32-Voxel Chunk is a 64 m Cell.
        let (source, mesh) = terrain_mesh(world.voxel_size_at(3));
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("positions are Float32x3");
        };
        for (scaled, unscaled) in positions.iter().zip(&source.opaque.positions) {
            assert_eq!(scaled, &unscaled.map(|c| c * 2.0));
        }
        let widest = positions.iter().map(|p| p[0]).fold(0.0f32, f32::max);
        assert!(
            widest <= world.cell_size_at(3),
            "{widest} m exceeds the Cell"
        );
    }

    #[test]
    fn normals_and_uvs_pass_through_unscaled() {
        let (source, mesh) = terrain_mesh(0.25);
        let Some(VertexAttributeValues::Float32x3(normals)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("normals are Float32x3");
        };
        assert_eq!(normals, &source.opaque.normals);
        for n in normals {
            assert!((Vec3::from_array(*n).length() - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn material_indices_survive_as_uint32() {
        let mut chunk = Chunk::air();
        chunk.set(0, 0, 0, Voxel::cube(testgen::GRASS));
        let source = greedy_mesh(
            &Neighborhood::isolated(LitChunk::new(&chunk, &SKY)),
            &Tables::builtin(),
        );
        let mesh = chunk_mesh_to_mesh(&source.opaque, 1.0);

        let Some(VertexAttributeValues::Uint32(materials)) = mesh.attribute(ATTRIBUTE_MATERIAL)
        else {
            panic!("the material index must stay an integer, not become a float");
        };
        assert_eq!(materials.len(), source.opaque.positions.len());
        assert!(materials.iter().all(|m| *m == testgen::GRASS as u32));
    }

    /// The same trap as the Material index: a light level is two packed
    /// nibbles, so becoming a float loses them.
    #[test]
    fn light_levels_survive_as_uint32_and_voxel_size_does_not_scale_them() {
        let mut chunk = Chunk::air();
        // In the middle, so all six faces look out at air inside the Chunk.
        // At a corner, three of them look into the isolated Neighborhood's dark
        // shell and the assertion below is about the shell instead.
        chunk.set(16, 16, 16, Voxel::cube(testgen::GRASS));
        let source = greedy_mesh(
            &Neighborhood::isolated(LitChunk::new(&chunk, &SKY)),
            &Tables::builtin(),
        );

        for voxel_size in [1.0, 0.25] {
            let mesh = chunk_mesh_to_mesh(&source.opaque, voxel_size);
            let Some(VertexAttributeValues::Uint32(light)) = mesh.attribute(ATTRIBUTE_LIGHT) else {
                panic!("a light level must stay an integer, not become a float");
            };
            assert_eq!(light, &source.opaque.light, "at {voxel_size} m Voxels");
            assert!(
                light
                    .iter()
                    .all(|&l| l == Light::new(MAX_LIGHT, 0).packed() as u32),
                "the faces of a lone Voxel all look out at full sky"
            );
        }
    }
}
