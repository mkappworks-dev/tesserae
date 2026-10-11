//! `ChunkMesh` → Bevy `Mesh`. The one place Voxel-index space becomes metres.
//!
//! This conversion lives in `tesserae` and never in `tesserae-core`, which must
//! acquire no `bevy` or `wgpu` dependency even transitively. See
//! `docs/architecture.md` invariant 3.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, MeshVertexAttribute, PrimitiveTopology, VertexFormat};
use bevy::prelude::*;
use tesserae_core::ChunkMesh;

/// Which Material each vertex belongs to, for the chunk [`MaterialExtension`].
///
/// A high id keeps custom attributes sorted after Bevy's built-ins and away from
/// their ids. `Uint32` rather than a narrower type because the Voxel format
/// already budgets 16 bits for Material and shaped Voxels will want the Shape
/// alongside it.
///
/// [`MaterialExtension`]: bevy::pbr::MaterialExtension
pub const ATTRIBUTE_MATERIAL: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_Material", 2_084_155_061, VertexFormat::Uint32);

/// Positions are scaled by `voxel_size` here, so everything downstream — the
/// `Aabb` Bevy derives, frustum culling, shadow bounds — is already in metres.
pub fn chunk_mesh_to_mesh(chunk_mesh: &ChunkMesh, voxel_size: f32) -> Mesh {
    let positions: Vec<[f32; 3]> = chunk_mesh
        .positions
        .iter()
        .map(|p| [p[0] * voxel_size, p[1] * voxel_size, p[2] * voxel_size])
        .collect();

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, chunk_mesh.normals.clone())
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, chunk_mesh.uvs.clone())
    .with_inserted_attribute(ATTRIBUTE_MATERIAL, chunk_mesh.materials.clone())
    .with_inserted_indices(Indices::U32(chunk_mesh.indices.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::mesh::VertexAttributeValues;
    use tesserae_core::{Chunk, Neighborhood, Voxel, greedy_mesh, testgen};

    fn terrain_mesh(voxel_size: f32) -> (ChunkMesh, Mesh) {
        let chunk = testgen::terrain_chunk(0, 0, 0);
        let source = greedy_mesh(&Neighborhood::isolated(&chunk));
        let mesh = chunk_mesh_to_mesh(&source, voxel_size);
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
        ] {
            assert!(mesh.attribute(attribute).is_some(), "{attribute:?} missing");
        }
        assert_eq!(mesh.count_vertices(), source.positions.len());
        assert_eq!(mesh.indices().unwrap().len(), source.indices.len());
    }

    #[test]
    fn positions_are_metres_so_voxel_size_scales_the_chunk() {
        let (source, mesh) = terrain_mesh(0.25);
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("positions are Float32x3");
        };
        for (scaled, unscaled) in positions.iter().zip(&source.positions) {
            assert_eq!(scaled, &unscaled.map(|c| c * 0.25));
        }
        // A 32-Voxel Chunk at 0.25m is an 8m Cell, whatever the terrain inside.
        let widest = positions.iter().map(|p| p[0]).fold(0.0f32, f32::max);
        assert!(widest <= 8.0, "{widest} m exceeds the Cell");
    }

    #[test]
    fn normals_and_uvs_pass_through_unscaled() {
        let (source, mesh) = terrain_mesh(0.25);
        let Some(VertexAttributeValues::Float32x3(normals)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("normals are Float32x3");
        };
        assert_eq!(normals, &source.normals);
        for n in normals {
            assert!((Vec3::from_array(*n).length() - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn material_indices_survive_as_uint32() {
        let mut chunk = Chunk::air();
        chunk.set(0, 0, 0, Voxel::cube(testgen::GRASS));
        let source = greedy_mesh(&Neighborhood::isolated(&chunk));
        let mesh = chunk_mesh_to_mesh(&source, 1.0);

        let Some(VertexAttributeValues::Uint32(materials)) = mesh.attribute(ATTRIBUTE_MATERIAL)
        else {
            panic!("the material index must stay an integer, not become a float");
        };
        assert_eq!(materials.len(), source.positions.len());
        assert!(materials.iter().all(|m| *m == testgen::GRASS as u32));
    }
}
