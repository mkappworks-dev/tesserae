//! `ChunkMesh` → Bevy `Mesh`, packed to 16 bytes a vertex.
//!
//! This conversion lives in `tesserae` and never in `tesserae-core`, which must
//! acquire no `bevy` or `wgpu` dependency even transitively. See
//! `docs/architecture.md` invariant 3.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{
    Indices, MeshVertexAttribute, PrimitiveTopology, VertexAttributeValues, VertexFormat,
};
use bevy::prelude::*;
use bevy::shape::Aabb3d;
use tesserae_core::MeshLayer;
use tesserae_core::shape::SUBDIV;

/// Position in sixteenths of a Voxel, `xyz`, and in `w` the face the vertex
/// belongs to, `NEG_X..=POS_Z`, which `chunk_vertex.wesl` turns back into the
/// normal.
///
/// Sixteenths because every Shape, fluid surface and flame lies on that grid,
/// so the packing is exact. Signed because a flame stands a sixteenth outside
/// the Chunk, and sixteen bits because positions local to a region of Chunks
/// need the range: ±2047 Voxels.
pub const ATTRIBUTE_POSITION_FACE: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_PositionFace", 2_084_155_063, VertexFormat::Sint16x4);

/// UV in sixteenths of a Voxel, on the same grid as the positions.
pub const ATTRIBUTE_UV: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_ChunkUv", 2_084_155_064, VertexFormat::Uint16x2);

/// Which Material the vertex belongs to in the low [`LIGHT_SHIFT`] bits, and
/// the packed sky and block light of the face above them.
///
/// The Material needs 17 bits, for [`tesserae_core::mesh::FLAME`]. Light is
/// baked per vertex rather than sampled on the GPU: thousands of torches as
/// point lights is not viable on any hardware, so light is a per-Voxel
/// flood-fill and this attribute is how it reaches the shader.
pub const ATTRIBUTE_MATERIAL_LIGHT: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_MaterialLight", 2_084_155_065, VertexFormat::Uint32);

/// Packed position and UV steps per Voxel.
pub const STEPS: f32 = SUBDIV as f32;

pub const LIGHT_SHIFT: u32 = 24;

/// The mesh is in Voxels, not metres: the entity's `Transform` scales it by
/// the level's Voxel size. The `Aabb` is explicit because Bevy can only derive
/// one from a `Float32x3` position.
pub fn chunk_mesh_to_mesh(layer: &MeshLayer) -> Mesh {
    // Unchecked per vertex: the mesher emits only whole sixteenths, from `NodeBox`es `register`
    // bounds, and `every_shape_and_fluid_level_decodes_to_the_meshers_own_vertices` holds it.
    let step = |c: f32| (c * STEPS) as i16;
    let position_face: Vec<[i16; 4]> = layer
        .positions
        .iter()
        .zip(&layer.normals)
        .map(|(p, n)| [step(p[0]), step(p[1]), step(p[2]), face_of(*n)])
        .collect();
    let uvs: Vec<[u16; 2]> = layer
        .uvs
        .iter()
        .map(|uv| uv.map(|c| step(c) as u16))
        .collect();
    let material_light: Vec<u32> = layer
        .materials
        .iter()
        .zip(&layer.light)
        .map(|(&material, &light)| {
            debug_assert!(material < 1 << LIGHT_SHIFT && light <= 0xff);
            material | light << LIGHT_SHIFT
        })
        .collect();

    let (min, max) = position_face
        .iter()
        .fold((IVec3::MAX, IVec3::MIN), |(lo, hi), p| {
            let p = IVec3::new(p[0].into(), p[1].into(), p[2].into());
            (lo.min(p), hi.max(p))
        });
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(
        ATTRIBUTE_POSITION_FACE,
        VertexAttributeValues::Sint16x4(position_face),
    )
    .with_inserted_attribute(ATTRIBUTE_UV, VertexAttributeValues::Uint16x2(uvs))
    .with_inserted_attribute(ATTRIBUTE_MATERIAL_LIGHT, material_light)
    .with_inserted_indices(Indices::U32(layer.indices.clone()));
    if !layer.positions.is_empty() {
        mesh.final_aabb = Some(Aabb3d {
            min: (min.as_vec3() / STEPS).into(),
            max: (max.as_vec3() / STEPS).into(),
        });
    }
    mesh
}

/// The face an axis-aligned unit normal belongs to, in `FACE_STEPS` order.
fn face_of(normal: [f32; 3]) -> i16 {
    let axis = normal.iter().position(|&c| c != 0.0).unwrap_or(0);
    (axis * 2 + usize::from(normal[axis] > 0.0)) as i16
}

/// What `chunk_vertex.wesl` reads back from a mesh [`chunk_mesh_to_mesh`]
/// made, in Voxels: the layer it was made from.
pub fn decoded(mesh: &Mesh) -> Option<MeshLayer> {
    let (
        Some(VertexAttributeValues::Sint16x4(position_face)),
        Some(VertexAttributeValues::Uint16x2(uvs)),
        Some(VertexAttributeValues::Uint32(material_light)),
        Some(Indices::U32(indices)),
    ) = (
        mesh.attribute(ATTRIBUTE_POSITION_FACE),
        mesh.attribute(ATTRIBUTE_UV),
        mesh.attribute(ATTRIBUTE_MATERIAL_LIGHT),
        mesh.indices(),
    )
    else {
        return None;
    };
    Some(MeshLayer {
        positions: position_face
            .iter()
            .map(|p| [p[0], p[1], p[2]].map(|c| f32::from(c) / STEPS))
            .collect(),
        normals: position_face
            .iter()
            .map(|p| {
                let face = p[3] as usize;
                let mut normal = [0.0; 3];
                normal[face / 2] = if face % 2 == 1 { 1.0 } else { -1.0 };
                normal
            })
            .collect(),
        uvs: uvs
            .iter()
            .map(|uv| uv.map(|c| f32::from(c) / STEPS))
            .collect(),
        materials: material_light
            .iter()
            .map(|w| w & ((1 << LIGHT_SHIFT) - 1))
            .collect(),
        light: material_light.iter().map(|w| w >> LIGHT_SHIFT).collect(),
        indices: indices.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tesserae_core::light::{Light, MAX_LIGHT};
    use tesserae_core::shape::examples;
    use tesserae_core::testgen::fixtures;
    use tesserae_core::{
        Burning, CHUNK_SIZE, Chunk, ChunkLight, ChunkOverlay, Fluid, Level, LitChunk, Neighborhood,
        Orientation, Overlay, Tables, Voxel, greedy_mesh,
    };

    /// Both channels non-zero, so a nibble lost on the way shows.
    static LIT: ChunkLight = ChunkLight::filled(Light::new(MAX_LIGHT, 7));

    /// The builtin Shapes, then a fence and a pipe, which connect.
    fn tables() -> Tables {
        let mut tables = fixtures().tables.clone();
        tables.shapes.register(examples::fence()).unwrap();
        tables.shapes.register(examples::pipe()).unwrap();
        tables
    }

    fn wet(fluid: Fluid) -> Overlay {
        Overlay {
            fluid: Some(fluid),
            burning: None,
        }
    }

    /// A floor the full width of the Chunk, fluid standing at every surface
    /// level, every Shape in every orientation soaked in a source and in flowing
    /// fluid at every level, and flames off both far corners.
    fn every_kind_of_face(tables: &Tables) -> tesserae_core::ChunkMesh {
        let (rock, water) = (fixtures().ground.rock, fixtures().water);
        let mut chunk = Chunk::air();
        let mut overlay = ChunkOverlay::none();
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                chunk.set(x, 0, z, Voxel::cube(rock));
            }
        }
        for surface in 1..=Fluid::FULL_SURFACE {
            let fluid = Fluid {
                source: false,
                surface,
                reach: 1,
                ..Fluid::source(water)
            };
            overlay.set(2 * surface as usize - 1, 2, 1, wet(fluid));
        }
        let mut cell = 0;
        for shape in 1..tables.shapes.len() as u8 {
            for orientation in Orientation::all() {
                for surface in 1..=Fluid::FULL_SURFACE {
                    let at = [
                        1 + 2 * (cell % 15),
                        4 + 2 * (cell / 225),
                        1 + 2 * (cell / 15 % 15),
                    ];
                    cell += 1;
                    chunk.set(at[0], at[1], at[2], Voxel::shaped(rock, shape, orientation));
                    let level = Level {
                        surface,
                        reach: 1,
                        descent: 0,
                    };
                    let fluid = if surface == Fluid::FULL_SURFACE {
                        Fluid::source(water)
                    } else {
                        Fluid::flowing(water, [level; 4]).unwrap()
                    };
                    overlay.set(at[0], at[1], at[2], wet(fluid));
                }
            }
        }
        for at in [0, CHUNK_SIZE - 1] {
            chunk.set(at, CHUNK_SIZE - 1, at, Voxel::cube(rock));
            overlay.set(
                at,
                CHUNK_SIZE - 1,
                at,
                Overlay {
                    fluid: None,
                    burning: Some(Burning { fuel: 1 }),
                },
            );
        }
        let lit = LitChunk::new(&chunk, &LIT).with_overlay(&overlay);
        greedy_mesh(&Neighborhood::isolated(lit), tables)
    }

    fn assert_decodes_exactly(layer: &MeshLayer, what: &str) {
        let decoded = decoded(&chunk_mesh_to_mesh(layer)).expect("a packed Chunk mesh");
        assert_eq!(decoded.positions, layer.positions, "{what}: positions");
        assert_eq!(decoded.normals, layer.normals, "{what}: normals");
        assert_eq!(decoded.uvs, layer.uvs, "{what}: uvs");
        assert_eq!(decoded.materials, layer.materials, "{what}: materials");
        assert_eq!(decoded.light, layer.light, "{what}: light");
        assert_eq!(decoded.indices, layer.indices, "{what}: indices");
    }

    /// Bevy derives an `Aabb` only from `Float32x3` positions; without one a
    /// Chunk is never frustum culled.
    #[test]
    fn the_aabb_is_exactly_the_bounds_of_the_decoded_positions() {
        use bevy::camera::primitives::MeshAabb;
        let mesh = every_kind_of_face(&tables());
        let packed = chunk_mesh_to_mesh(&mesh.cutout);
        let aabb = packed.get_aabb().expect("an Aabb Bevy can find");
        let positions = mesh.cutout.positions.iter().map(|&p| Vec3::from(p));
        let lo = positions.clone().fold(Vec3::MAX, Vec3::min);
        let hi = positions.fold(Vec3::MIN, Vec3::max);
        assert_eq!(Vec3::from(aabb.min()), lo);
        assert_eq!(Vec3::from(aabb.max()), hi);
        assert_eq!(lo.x, -1.0 / 16.0, "a flame outside the Chunk");

        let empty = chunk_mesh_to_mesh(&MeshLayer::default());
        assert!(empty.get_aabb().is_none());
    }

    #[test]
    fn every_shape_and_fluid_level_decodes_to_the_meshers_own_vertices() {
        let tables = tables();
        for shape in 0..tables.shapes.len() as u8 {
            for (orientation, quads) in tables.shapes.preview_geometry(shape).iter().enumerate() {
                let layer = tesserae_core::mesh::mesh_shape(quads, fixtures().ground.rock);
                assert_decodes_exactly(&layer, &format!("Shape {shape} turned {orientation}"));
            }
        }

        let mesh = every_kind_of_face(&tables);
        for (layer, what) in [
            (&mesh.opaque, "opaque"),
            (&mesh.blended, "blended"),
            (&mesh.cutout, "cutout"),
        ] {
            assert_decodes_exactly(layer, what);
        }

        // What the scene is for: the extremes the packing has to hold.
        let all = || {
            [&mesh.opaque, &mesh.blended, &mesh.cutout]
                .into_iter()
                .flat_map(|l| l.positions.iter())
        };
        let lowest = all().flat_map(|p| p.iter()).copied().fold(0.0, f32::min);
        let highest = all().flat_map(|p| p.iter()).copied().fold(0.0, f32::max);
        assert_eq!(lowest, -1.0 / 16.0, "a flame outside the Chunk's low faces");
        assert_eq!(highest, 32.0 + 1.0 / 16.0, "and outside its high ones");
        assert!(mesh.cutout.materials.contains(&tesserae_core::mesh::FLAME));
        let widest = mesh
            .opaque
            .uvs
            .iter()
            .flatten()
            .copied()
            .fold(0.0, f32::max);
        assert_eq!(widest, 32.0, "the floor's UVs span the Chunk");
        for surface in 1..=Fluid::FULL_SURFACE {
            let height = 2.0 + f32::from(surface) / 16.0;
            assert!(
                mesh.blended.positions.iter().any(|p| p[1] == height),
                "no fluid surface at {height}"
            );
        }
    }
}
