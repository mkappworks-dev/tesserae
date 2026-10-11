use tesserae_core::light::{Channel, Light, MAX_LIGHT, transmitted};
use tesserae_core::mesh::{NEG_X, NEG_Y, POS_X};
use tesserae_core::{
    CHUNK_SIZE, Chunk, ChunkLight, LitChunk, Neighborhood, Voxel, greedy_mesh, mesh_chunk, testgen,
};
use tesserae_core::{Worldgen, WorldgenParams};

/// The light the tests that are about *geometry* use. Lent from a `static`
/// because `LitChunk` borrows it; the light-specific tests below build their
/// own.
static DARK: ChunkLight = ChunkLight::dark();

fn unlit(chunk: &Chunk) -> LitChunk<'_> {
    LitChunk::new(chunk, &DARK)
}

fn mesh_isolated(chunk: &Chunk) -> tesserae_core::ChunkMesh {
    greedy_mesh(&Neighborhood::isolated(unlit(chunk)))
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
    let open = greedy_mesh(&Neighborhood::isolated(unlit(&solid)));
    let closed = greedy_mesh(&Neighborhood::new(
        unlit(&solid),
        [None, Some(unlit(&solid)), None, None, None, None],
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

    let open = greedy_mesh(&Neighborhood::isolated(unlit(&center)));
    let closed = greedy_mesh(&Neighborhood::new(
        unlit(&center),
        [Some(unlit(&left)), None, None, None, None, None],
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
    assert_eq!(mesh.light.len(), n);
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

// `mesh_chunk`: the streaming entry point, which declines where it provably can.

/// Six of the same Chunk, for the enclosed case.
fn enclosed_by(n: &Chunk) -> [Option<LitChunk<'_>>; 6] {
    [Some(unlit(n)); 6]
}

#[test]
fn a_uniform_air_chunk_declines_whatever_its_neighbours_hold() {
    let solid = Chunk::filled(Voxel::cube(1));
    let air = Chunk::air();
    assert!(mesh_chunk(unlit(&air), enclosed_by(&solid)).is_empty());
    assert!(mesh_chunk(unlit(&air), [None; 6]).is_empty());
}

#[test]
fn a_fully_enclosed_solid_chunk_declines() {
    let solid = Chunk::filled(Voxel::cube(1));
    assert!(mesh_chunk(unlit(&solid), enclosed_by(&solid)).is_empty());
}

#[test]
fn an_unknown_neighbour_is_not_an_occluder() {
    let solid = Chunk::filled(Voxel::cube(1));
    let mut neighbors = enclosed_by(&solid);
    neighbors[0] = None;
    // Unknown, not air: the face has to be emitted or the Chunk is open to a
    // hole once that neighbour streams in.
    assert_eq!(mesh_chunk(unlit(&solid), neighbors).quad_count(), 1);
}

#[test]
fn an_air_neighbour_opens_exactly_one_face() {
    let solid = Chunk::filled(Voxel::cube(1));
    let air = Chunk::air();
    let mut neighbors = enclosed_by(&solid);
    neighbors[2] = Some(unlit(&air));
    assert_eq!(mesh_chunk(unlit(&solid), neighbors).quad_count(), 1);
}

#[test]
fn a_shaped_uniform_chunk_is_not_an_occluder() {
    // Pass 1 treats a shaped Voxel as transparent, so a uniform Chunk of
    // stairs must not satisfy the enclosed case.
    let stairs = Chunk::filled(Voxel::new(1, 1, 0));
    let solid = Chunk::filled(Voxel::cube(1));
    assert!(mesh_chunk(unlit(&stairs), enclosed_by(&solid)).is_empty());
    assert!(mesh_chunk(unlit(&solid), enclosed_by(&stairs)).quad_count() > 0);
}

/// The fast path is an optimisation, so where it does not fire the output must
/// be the same triangle soup `greedy_mesh` would have produced.
#[test]
fn declining_never_changes_the_mesh_it_would_have_produced() {
    let g = Worldgen::new(0x7e55, 1.0, WorldgenParams::default());
    let center = g.chunk(0, 0, 0);
    let neighbors = [
        g.chunk(-1, 0, 0),
        g.chunk(1, 0, 0),
        g.chunk(0, -1, 0),
        g.chunk(0, 1, 0),
        g.chunk(0, 0, -1),
        g.chunk(0, 0, 1),
    ];
    let refs: [Option<LitChunk>; 6] = std::array::from_fn(|i| Some(unlit(&neighbors[i])));

    let fast = mesh_chunk(unlit(&center), refs);
    let slow = greedy_mesh(&Neighborhood::new(unlit(&center), refs));
    assert_eq!(fast.positions, slow.positions);
    assert_eq!(fast.normals, slow.normals);
    assert_eq!(fast.uvs, slow.uvs);
    assert_eq!(fast.materials, slow.materials);
    assert_eq!(fast.light, slow.light);
    assert_eq!(fast.indices, slow.indices);
    assert_eq!(fast.skipped_shaped, slow.skipped_shaped);
}

// Light in the merge key. A merged quad has four vertices and cannot carry a
// different level per Voxel it spans, so light has to be part of what two
// coplanar faces agree on - which is also why there is no light-only re-mesh
// path: a light change can change the quad decomposition.

/// A solid Chunk with air above it, so there is one unobstructed top face per
/// column to put light on.
fn floor() -> Chunk {
    let mut chunk = Chunk::air();
    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            chunk.set(x, 0, z, Voxel::cube(1));
        }
    }
    chunk
}

#[test]
fn uniform_light_costs_the_merge_key_nothing() {
    let chunk = floor();
    let flat = ChunkLight::filled(Light::new(MAX_LIGHT, 0));
    let lit = greedy_mesh(&Neighborhood::isolated(LitChunk::new(&chunk, &flat)));
    let unlit = mesh_isolated(&chunk);
    assert_eq!(
        lit.quad_count(),
        unlit.quad_count(),
        "flat light must not split a quad"
    );
}

#[test]
fn two_faces_differing_only_in_light_do_not_merge() {
    let chunk = floor();

    let mut split = ChunkLight::filled(Light::new(MAX_LIGHT, 0));
    // One column of the air directly above the floor is dimmer, so the top face
    // under it cannot join the run beside it.
    for z in 0..CHUNK_SIZE {
        split.set(16, 1, z, Light::new(MAX_LIGHT - 1, 0));
    }

    let flat = ChunkLight::filled(Light::new(MAX_LIGHT, 0));
    let merged = greedy_mesh(&Neighborhood::isolated(LitChunk::new(&chunk, &flat)));
    let shredded = greedy_mesh(&Neighborhood::isolated(LitChunk::new(&chunk, &split)));
    assert!(
        shredded.quad_count() > merged.quad_count(),
        "{} quads with a seam vs {} without",
        shredded.quad_count(),
        merged.quad_count()
    );
}

#[test]
fn a_face_carries_the_light_of_the_voxel_in_front_of_it() {
    let chunk = floor();
    let mut light = ChunkLight::dark();
    // The air above the floor, which is what a +Y face looks out at. The floor
    // itself stays dark, as an opaque Voxel does.
    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            light.set(x, 1, z, Light::new(9, 4));
        }
    }

    let mesh = greedy_mesh(&Neighborhood::isolated(LitChunk::new(&chunk, &light)));
    let top: Vec<u32> = (0..mesh.positions.len())
        .filter(|&i| mesh.normals[i] == [0.0, 1.0, 0.0])
        .map(|i| mesh.light[i])
        .collect();
    assert!(!top.is_empty(), "the floor has a top face");
    assert!(
        top.iter().all(|&l| l == Light::new(9, 4).packed() as u32),
        "a top face must be lit by the air above it, not by its own Voxel: {top:?}"
    );

    // And every vertex of one quad agrees, since they share the key.
    for quad in mesh.light.chunks(4) {
        assert!(quad.iter().all(|l| *l == quad[0]));
    }
}

#[test]
fn a_boundary_face_is_lit_from_the_neighbour_and_not_from_zero() {
    let mut center = Chunk::air();
    center.set(CHUNK_SIZE - 1, 5, 5, Voxel::cube(1));
    let center_light = ChunkLight::dark();

    let above = Chunk::air();
    let mut above_light = ChunkLight::dark();
    above_light.set(0, 5, 5, Light::new(MAX_LIGHT, 0));

    let mut neighbors = [None; 6];
    neighbors[POS_X] = Some(LitChunk::new(&above, &above_light));
    let mesh = greedy_mesh(&Neighborhood::new(
        LitChunk::new(&center, &center_light),
        neighbors,
    ));

    let across: Vec<u32> = (0..mesh.positions.len())
        .filter(|&i| mesh.normals[i] == [1.0, 0.0, 0.0])
        .map(|i| mesh.light[i])
        .collect();
    assert_eq!(
        across,
        vec![Light::new(MAX_LIGHT, 0).packed() as u32; 4],
        "the +X face reads the neighbour's light shell"
    );
}

/// Light can only split a quad that would otherwise merge, never create or
/// remove a face, because face visibility is a question about geometry alone.
/// Both fast paths therefore still hold - checked against deliberately
/// non-uniform light rather than argued.
#[test]
fn light_does_not_revive_either_declined_case() {
    let air = Chunk::air();
    let solid = Chunk::filled(Voxel::cube(1));

    let mut lumpy = ChunkLight::dark();
    lumpy.set(3, 4, 5, Light::new(MAX_LIGHT, MAX_LIGHT));
    lumpy.set(31, 31, 31, Light::new(1, 2));

    let neighbors: [Option<LitChunk>; 6] = [Some(LitChunk::new(&solid, &lumpy)); 6];
    assert!(mesh_chunk(LitChunk::new(&air, &lumpy), neighbors).is_empty());
    assert!(mesh_chunk(LitChunk::new(&solid, &lumpy), neighbors).is_empty());
}

/// The mesher bakes whatever light it is handed, so it must not quietly reject
/// a level the solver can legitimately produce.
#[test]
fn every_level_the_solver_can_produce_survives_to_a_vertex() {
    let chunk = floor();
    let mut light = ChunkLight::dark();
    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            let sky = transmitted(Channel::Sky, MAX_LIGHT, NEG_Y);
            light.set(x, 1, z, Light::new(sky, (x % 16) as u8));
        }
    }
    let mesh = greedy_mesh(&Neighborhood::isolated(LitChunk::new(&chunk, &light)));
    let seen: std::collections::BTreeSet<u8> = (0..mesh.positions.len())
        .filter(|&i| mesh.normals[i] == [0.0, 1.0, 0.0])
        .map(|i| (mesh.light[i] & 0x0F) as u8)
        .collect();
    assert_eq!(seen.len(), 16, "lost a block level: {seen:?}");
}
