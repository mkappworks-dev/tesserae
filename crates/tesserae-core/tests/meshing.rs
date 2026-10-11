use tesserae_core::mesh::{NEG_X, POS_X};
use tesserae_core::{CHUNK_SIZE, Chunk, Neighborhood, Voxel, greedy_mesh, testgen};

fn mesh_isolated(chunk: &Chunk) -> tesserae_core::ChunkMesh {
    greedy_mesh(&Neighborhood::isolated(chunk))
}

#[test]
fn an_air_chunk_emits_nothing() {
    let mesh = mesh_isolated(&Chunk::air());
    assert!(mesh.is_empty());
    assert_eq!(mesh.quad_count(), 0);
}

#[test]
fn one_voxel_emits_six_quads() {
    let mut c = Chunk::air();
    c.set(16, 16, 16, Voxel::cube(1));
    let mesh = mesh_isolated(&c);
    assert_eq!(mesh.quad_count(), 6);
    assert_eq!(mesh.positions.len(), 24);
    assert_eq!(mesh.indices.len(), 36);
}

/// The whole point of greedy meshing. A naive mesher emits 6 * 32 * 32 = 6144.
#[test]
fn a_solid_chunk_emits_six_quads_not_six_thousand() {
    let mesh = mesh_isolated(&Chunk::filled(Voxel::cube(1)));
    assert_eq!(mesh.quad_count(), 6);
}

#[test]
fn two_adjacent_voxels_merge_their_side_faces() {
    let mut c = Chunk::air();
    c.set(10, 10, 10, Voxel::cube(1));
    c.set(11, 10, 10, Voxel::cube(1));
    // 2 end caps, and each of the 4 sides merges into one 2x1 quad.
    assert_eq!(mesh_isolated(&c).quad_count(), 6);
}

#[test]
fn differing_materials_do_not_merge() {
    let mut c = Chunk::air();
    c.set(10, 10, 10, Voxel::cube(1));
    c.set(11, 10, 10, Voxel::cube(2));
    // 2 end caps plus 2 unmerged quads on each of the 4 sides.
    assert_eq!(mesh_isolated(&c).quad_count(), 10);
}

#[test]
fn an_interior_face_between_two_cubes_is_culled() {
    let mut c = Chunk::air();
    c.set(10, 10, 10, Voxel::cube(1));
    c.set(11, 10, 10, Voxel::cube(2));
    let mesh = mesh_isolated(&c);
    // Side faces of both cubes legitimately have vertices at x == 11; what must
    // not exist is a quad lying *on* that plane.
    let on_seam = mesh
        .positions
        .chunks(4)
        .filter(|quad| quad.iter().all(|p| p[0] == 11.0))
        .count();
    assert_eq!(on_seam, 0, "the shared plane x=11 must emit no face");
}

#[test]
fn a_solid_neighbour_culls_the_boundary_face() {
    let solid = Chunk::filled(Voxel::cube(1));
    let open = greedy_mesh(&Neighborhood::isolated(&solid));
    let closed = greedy_mesh(&Neighborhood::new(
        &solid,
        [None, Some(&solid), None, None, None, None],
    ));
    assert_eq!(open.quad_count(), 6);
    assert_eq!(closed.quad_count(), 5, "+X face should be occluded");
}

#[test]
fn neighbours_are_read_from_the_correct_side() {
    let mut center = Chunk::air();
    center.set(0, 5, 5, Voxel::cube(1));

    let mut left = Chunk::air();
    left.set(CHUNK_SIZE - 1, 5, 5, Voxel::cube(1));

    let open = greedy_mesh(&Neighborhood::isolated(&center));
    let closed = greedy_mesh(&Neighborhood::new(
        &center,
        [Some(&left), None, None, None, None, None],
    ));
    assert_eq!(open.quad_count(), 6);
    assert_eq!(closed.quad_count(), 5, "-X face should be occluded");
    let _ = (NEG_X, POS_X);
}

/// A flipped quad is invisible under backface culling and the bug is easy to
/// miss in a screenshot, so check the geometry rather than the pixels.
#[test]
fn triangle_winding_matches_the_declared_normal() {
    let mesh = mesh_isolated(&testgen::terrain_chunk(0, 0, 0));
    assert!(mesh.quad_count() > 0);

    for tri in mesh.indices.chunks(3) {
        let p: Vec<[f32; 3]> = tri.iter().map(|&i| mesh.positions[i as usize]).collect();
        let e1 = [p[1][0] - p[0][0], p[1][1] - p[0][1], p[1][2] - p[0][2]];
        let e2 = [p[2][0] - p[0][0], p[2][1] - p[0][1], p[2][2] - p[0][2]];
        let cross = [
            e1[1] * e2[2] - e1[2] * e2[1],
            e1[2] * e2[0] - e1[0] * e2[2],
            e1[0] * e2[1] - e1[1] * e2[0],
        ];
        let n = mesh.normals[tri[0] as usize];
        let dot = cross[0] * n[0] + cross[1] * n[1] + cross[2] * n[2];
        assert!(
            dot > 0.0,
            "winding flipped: cross {cross:?} vs normal {n:?}"
        );
    }
}

#[test]
fn every_quad_is_planar_and_axis_aligned() {
    let mesh = mesh_isolated(&testgen::terrain_chunk(1, 0, 1));
    for quad in mesh.positions.chunks(4) {
        let axes = (0..3)
            .filter(|&ax| quad.iter().all(|p| p[ax] == quad[0][ax]))
            .count();
        assert_eq!(axes, 1, "quad {quad:?} is not on one axis-aligned plane");
    }
}

#[test]
fn shaped_voxels_are_counted_and_emit_nothing() {
    let mut c = Chunk::air();
    c.set(4, 4, 4, Voxel::new(1, 1, 0));
    c.set(5, 4, 4, Voxel::new(1, 2, 0));
    let mesh = mesh_isolated(&c);
    assert_eq!(mesh.skipped_shaped, 2);
    assert_eq!(mesh.quad_count(), 0, "pass 1 ignores shaped voxels");
}

/// A shaped Voxel must not cull its neighbour's face: it does not fill its cell,
/// so occluding would punch a visible hole until per-Shape opacity exists.
#[test]
fn a_shaped_voxel_does_not_occlude_its_neighbour() {
    let mut c = Chunk::air();
    c.set(10, 10, 10, Voxel::cube(1));
    c.set(11, 10, 10, Voxel::new(1, 1, 0));
    assert_eq!(mesh_isolated(&c).quad_count(), 6);
}

#[test]
fn vertex_attributes_stay_in_lockstep() {
    let mesh = mesh_isolated(&testgen::terrain_chunk(0, 0, 0));
    let n = mesh.positions.len();
    assert_eq!(mesh.normals.len(), n);
    assert_eq!(mesh.uvs.len(), n);
    assert_eq!(mesh.materials.len(), n);
    assert_eq!(mesh.indices.len(), n / 4 * 6);
    assert!(mesh.indices.iter().all(|&i| (i as usize) < n));
}

#[test]
fn greedy_meshing_beats_the_naive_count_on_real_terrain() {
    let chunk = testgen::terrain_chunk(0, 0, 0);
    let mesh = mesh_isolated(&chunk);
    let naive = (0..CHUNK_SIZE)
        .flat_map(|y| (0..CHUNK_SIZE).flat_map(move |z| (0..CHUNK_SIZE).map(move |x| (x, y, z))))
        .filter(|&(x, y, z)| chunk.get(x, y, z).is_full_cube())
        .count()
        * 6;
    assert!(
        mesh.quad_count() * 4 < naive,
        "merged {} quads vs {naive} naive faces",
        mesh.quad_count()
    );
}
