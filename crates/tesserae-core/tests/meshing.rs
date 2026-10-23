use std::sync::OnceLock;
use tesserae_core::light::{Channel, Light, MAX_LIGHT, transmitted};
use tesserae_core::mesh::{NEG_X, NEG_Y, NEG_Z, POS_X, POS_Y, POS_Z};
use tesserae_core::shape::{NodeBox, examples};
use tesserae_core::{
    CHUNK_SIZE, Chunk, ChunkLight, ConnectDef, Connections, LitChunk, Neighborhood, Voxel,
    greedy_mesh, mesh_chunk, testgen,
};
use tesserae_core::{
    Orientation, SHAPE_POST, SHAPE_SLAB, SHAPE_STAIR, ShapeTable, Tables, Worldgen, WorldgenParams,
};

const FENCE: u8 = 4;
const PIPE: u8 = 5;

/// The builtin Tables, then a fence and a six-way pipe registered as a Mod would.
fn tables() -> &'static Tables {
    static TABLES: OnceLock<Tables> = OnceLock::new();
    TABLES.get_or_init(|| {
        let mut t = Tables::builtin();
        assert_eq!(t.shapes.register(examples::fence()), Ok(FENCE));
        assert_eq!(t.shapes.register(examples::pipe()), Ok(PIPE));
        t
    })
}

fn shapes() -> &'static ShapeTable {
    &tables().shapes
}

/// The light the tests that are about *geometry* use. Lent from a `static`
/// because `LitChunk` borrows it; the light-specific tests below build their
/// own.
static DARK: ChunkLight = ChunkLight::dark();

fn unlit(chunk: &Chunk) -> LitChunk<'_> {
    LitChunk::new(chunk, &DARK)
}

fn mesh_isolated(chunk: &Chunk) -> tesserae_core::ChunkMesh {
    greedy_mesh(&Neighborhood::isolated(unlit(chunk)), tables())
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
    assert_eq!(mesh.opaque.positions.len(), 24);
    assert_eq!(mesh.opaque.indices.len(), 36);
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
        .opaque
        .positions
        .chunks(4)
        .filter(|quad| quad.iter().all(|p| p[0] == 11.0))
        .count();
    assert_eq!(on_seam, 0, "the shared plane x=11 must emit no face");
}

#[test]
fn a_solid_neighbour_culls_the_boundary_face() {
    let solid = Chunk::filled(Voxel::cube(1));
    let open = greedy_mesh(&Neighborhood::isolated(unlit(&solid)), tables());
    let closed = greedy_mesh(
        &Neighborhood::new(
            unlit(&solid),
            [None, Some(unlit(&solid)), None, None, None, None],
        ),
        tables(),
    );
    assert_eq!(open.quad_count(), 6);
    assert_eq!(closed.quad_count(), 5, "+X face should be occluded");
}

#[test]
fn neighbours_are_read_from_the_correct_side() {
    let mut center = Chunk::air();
    center.set(0, 5, 5, Voxel::cube(1));

    let mut left = Chunk::air();
    left.set(CHUNK_SIZE - 1, 5, 5, Voxel::cube(1));

    let open = greedy_mesh(&Neighborhood::isolated(unlit(&center)), tables());
    let closed = greedy_mesh(
        &Neighborhood::new(
            unlit(&center),
            [Some(unlit(&left)), None, None, None, None, None],
        ),
        tables(),
    );
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

    for tri in mesh.opaque.indices.chunks(3) {
        let p: Vec<[f32; 3]> = tri
            .iter()
            .map(|&i| mesh.opaque.positions[i as usize])
            .collect();
        let e1 = [p[1][0] - p[0][0], p[1][1] - p[0][1], p[1][2] - p[0][2]];
        let e2 = [p[2][0] - p[0][0], p[2][1] - p[0][1], p[2][2] - p[0][2]];
        let cross = [
            e1[1] * e2[2] - e1[2] * e2[1],
            e1[2] * e2[0] - e1[0] * e2[2],
            e1[0] * e2[1] - e1[1] * e2[0],
        ];
        let n = mesh.opaque.normals[tri[0] as usize];
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
    for quad in mesh.opaque.positions.chunks(4) {
        let axes = (0..3)
            .filter(|&ax| quad.iter().all(|p| p[ax] == quad[0][ax]))
            .count();
        assert_eq!(axes, 1, "quad {quad:?} is not on one axis-aligned plane");
    }
}

#[test]
fn shaped_voxels_are_counted_and_emit_their_own_geometry() {
    let slab = shaped(SHAPE_SLAB, POS_Y, 0);
    let stair = shaped(SHAPE_STAIR, POS_Y, 0);
    let mut c = Chunk::air();
    // Apart, so neither hides a face of the other.
    c.set(4, 4, 4, slab);
    c.set(8, 4, 4, stair);
    let mesh = mesh_isolated(&c);
    assert_eq!(mesh.shaped_voxels, 2);
    assert_eq!(
        mesh.quad_count(),
        shapes().quads(slab, Connections::NONE).len()
            + shapes().quads(stair, Connections::NONE).len(),
        "pass 2 emits exactly the Shapes' own quads"
    );
}

/// A slab covers only half of its side, so it must not cull the face of the cube
/// beside it: that would punch a visible hole.
#[test]
fn a_shaped_voxel_does_not_occlude_its_neighbour() {
    let mut c = Chunk::air();
    c.set(10, 10, 10, Voxel::cube(1));
    c.set(11, 10, 10, shaped(SHAPE_SLAB, POS_Y, 0));
    assert!(cube_face_drawn(&mesh_isolated(&c), POS_X));
}

// Pass 1 culls a cube's face by what the Voxel across it *seals*, per face.

fn shaped(shape: u8, up: usize, turn: u8) -> Voxel {
    Voxel::shaped(1, shape, Orientation::new(up, turn))
}

/// Whether the cube at (10, 10, 10) has its `face` drawn, judged by the sub-cell
/// face in the middle of it. Pass 1 emits a face whole or not at all, so one
/// sample decides.
fn cube_face_drawn(mesh: &tesserae_core::ChunkMesh, face: usize) -> bool {
    let (d, positive) = (face / 2, face % 2 == 1);
    let mut cell = [168i32; 3];
    cell[d] = if positive { 160 + 15 } else { 160 };
    drawn_faces(mesh).contains_key(&(face, cell))
}

/// The sides on which `neighbour` hides the face of a cube it stands beside.
fn sides_it_hides(neighbour: Voxel) -> Vec<usize> {
    let offsets = [
        (NEG_X, [-1, 0, 0]),
        (POS_X, [1, 0, 0]),
        (NEG_Y, [0, -1, 0]),
        (POS_Y, [0, 1, 0]),
        (NEG_Z, [0, 0, -1]),
        (POS_Z, [0, 0, 1]),
    ];
    offsets
        .into_iter()
        .filter(|&(face, d)| {
            let mut c = Chunk::air();
            c.set(10, 10, 10, Voxel::cube(1));
            c.set(
                (10 + d[0]) as usize,
                (10 + d[1]) as usize,
                (10 + d[2]) as usize,
                neighbour,
            );
            !cube_face_drawn(&mesh_isolated(&c), face)
        })
        .map(|(face, _)| face)
        .collect()
}

#[test]
fn a_cube_loses_a_face_only_where_the_neighbour_seals_the_matching_one() {
    // A bottom slab seals its underside, so it hides the cube only from above.
    assert_eq!(sides_it_hides(shaped(SHAPE_SLAB, POS_Y, 0)), [POS_Y]);
    // A top slab seals its top, so it hides the cube only from below.
    assert_eq!(sides_it_hides(shaped(SHAPE_SLAB, NEG_Y, 0)), [NEG_Y]);
    // A stair seals its underside and its tall back (+Z): from above, and from
    // the -Z side, where that back faces the cube.
    assert_eq!(
        sides_it_hides(shaped(SHAPE_STAIR, POS_Y, 0)),
        [POS_Y, NEG_Z]
    );
    // Turned a quarter, the back faces +X and the cube sees it from -X's side.
    assert_eq!(
        sides_it_hides(shaped(SHAPE_STAIR, POS_Y, 1)),
        [NEG_X, POS_Y]
    );
    assert!(sides_it_hides(shaped(SHAPE_POST, POS_Y, 0)).is_empty());
}

#[test]
fn vertex_attributes_stay_in_lockstep() {
    let mesh = mesh_isolated(&testgen::terrain_chunk(0, 0, 0));
    let n = mesh.opaque.positions.len();
    assert_eq!(mesh.opaque.normals.len(), n);
    assert_eq!(mesh.opaque.uvs.len(), n);
    assert_eq!(mesh.opaque.materials.len(), n);
    assert_eq!(mesh.opaque.light.len(), n);
    assert_eq!(mesh.opaque.indices.len(), n / 4 * 6);
    assert!(mesh.opaque.indices.iter().all(|&i| (i as usize) < n));
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
    assert!(mesh_chunk(unlit(&air), enclosed_by(&solid), tables()).is_empty());
    assert!(mesh_chunk(unlit(&air), [None; 6], tables()).is_empty());
}

#[test]
fn a_fully_enclosed_solid_chunk_declines() {
    let solid = Chunk::filled(Voxel::cube(1));
    assert!(mesh_chunk(unlit(&solid), enclosed_by(&solid), tables()).is_empty());
}

#[test]
fn an_unknown_neighbour_is_not_an_occluder() {
    let solid = Chunk::filled(Voxel::cube(1));
    let mut neighbors = enclosed_by(&solid);
    neighbors[0] = None;
    // Unknown, not air: the face has to be emitted or the Chunk is open to a
    // hole once that neighbour streams in.
    assert_eq!(
        mesh_chunk(unlit(&solid), neighbors, tables()).quad_count(),
        1
    );
}

#[test]
fn an_air_neighbour_opens_exactly_one_face() {
    let solid = Chunk::filled(Voxel::cube(1));
    let air = Chunk::air();
    let mut neighbors = enclosed_by(&solid);
    neighbors[2] = Some(unlit(&air));
    assert_eq!(
        mesh_chunk(unlit(&solid), neighbors, tables()).quad_count(),
        1
    );
}

#[test]
fn a_shaped_uniform_chunk_is_not_an_occluder() {
    // A stair does not seal every face, so a Chunk of them neither satisfies the
    // enclosed case nor is hidden by it: each tread is inside its own cell.
    let stairs = Chunk::filled(shaped(SHAPE_STAIR, POS_Y, 0));
    let solid = Chunk::filled(Voxel::cube(1));
    assert!(mesh_chunk(unlit(&stairs), enclosed_by(&solid), tables()).quad_count() > 0);
    assert!(mesh_chunk(unlit(&solid), enclosed_by(&stairs), tables()).quad_count() > 0);
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

    let fast = mesh_chunk(unlit(&center), refs, tables());
    let slow = greedy_mesh(&Neighborhood::new(unlit(&center), refs), tables());
    assert_eq!(fast.opaque.positions, slow.opaque.positions);
    assert_eq!(fast.opaque.normals, slow.opaque.normals);
    assert_eq!(fast.opaque.uvs, slow.opaque.uvs);
    assert_eq!(fast.opaque.materials, slow.opaque.materials);
    assert_eq!(fast.opaque.light, slow.opaque.light);
    assert_eq!(fast.opaque.indices, slow.opaque.indices);
    assert_eq!(fast.shaped_voxels, slow.shaped_voxels);
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
    let lit = greedy_mesh(
        &Neighborhood::isolated(LitChunk::new(&chunk, &flat)),
        tables(),
    );
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
    let merged = greedy_mesh(
        &Neighborhood::isolated(LitChunk::new(&chunk, &flat)),
        tables(),
    );
    let shredded = greedy_mesh(
        &Neighborhood::isolated(LitChunk::new(&chunk, &split)),
        tables(),
    );
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

    let mesh = greedy_mesh(
        &Neighborhood::isolated(LitChunk::new(&chunk, &light)),
        tables(),
    );
    let top: Vec<u32> = (0..mesh.opaque.positions.len())
        .filter(|&i| mesh.opaque.normals[i] == [0.0, 1.0, 0.0])
        .map(|i| mesh.opaque.light[i])
        .collect();
    assert!(!top.is_empty(), "the floor has a top face");
    assert!(
        top.iter().all(|&l| l == Light::new(9, 4).packed() as u32),
        "a top face must be lit by the air above it, not by its own Voxel: {top:?}"
    );

    // And every vertex of one quad agrees, since they share the key.
    for quad in mesh.opaque.light.chunks(4) {
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
    let mesh = greedy_mesh(
        &Neighborhood::new(LitChunk::new(&center, &center_light), neighbors),
        tables(),
    );

    let across: Vec<u32> = (0..mesh.opaque.positions.len())
        .filter(|&i| mesh.opaque.normals[i] == [1.0, 0.0, 0.0])
        .map(|i| mesh.opaque.light[i])
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
    assert!(mesh_chunk(LitChunk::new(&air, &lumpy), neighbors, tables()).is_empty());
    assert!(mesh_chunk(LitChunk::new(&solid, &lumpy), neighbors, tables()).is_empty());
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
    let mesh = greedy_mesh(
        &Neighborhood::isolated(LitChunk::new(&chunk, &light)),
        tables(),
    );
    let seen: std::collections::BTreeSet<u8> = (0..mesh.opaque.positions.len())
        .filter(|&i| mesh.opaque.normals[i] == [0.0, 1.0, 0.0])
        .map(|i| (mesh.opaque.light[i] & 0x0F) as u8)
        .collect();
    assert_eq!(seen.len(), 16, "lost a block level: {seen:?}");
}

// Pass 2: shaped Voxels.

/// A tiny seeded generator; core has no dependencies, so no `rand`.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// The Voxels in `[LO, LO + SPAN)` on each axis, away from the Chunk edge so
/// `Neighborhood::isolated`'s air shell is not what the scene meets.
const LO: usize = 10;
const SPAN: usize = 4;

/// A face of a solid sub-cell, in sixteenths: which face, and which sub-cell.
type SubFace = (usize, [i32; 3]);

const DIRS: [[i32; 3]; 6] = [
    [-1, 0, 0],
    [1, 0, 0],
    [0, -1, 0],
    [0, 1, 0],
    [0, 0, -1],
    [0, 0, 1],
];

/// What a Voxel is drawn with, from the test's own copy of the definitions and
/// **not** from `ShapeTable::connections`: the rule that grows an arm is
/// restated here from the same declarations, so a mistake in the table's is
/// something this can disagree with.
fn declared(chunk: &Chunk, at: [i32; 3], arms: bool) -> Vec<NodeBox> {
    let v = chunk.get(at[0] as usize, at[1] as usize, at[2] as usize);
    let def = match v.shape() {
        _ if v.is_air() => return vec![],
        0 => return vec![NodeBox::new([0; 3], [16; 3])],
        FENCE => examples::fence(),
        PIPE => examples::pipe(),
        _ => return shapes().selection(v, Connections::NONE).to_vec(),
    };
    let connect = def.connect.as_ref().expect("a connected example");
    let mut boxes = def.boxes.clone();
    for arm in connect.arms.iter().filter(|_| arms) {
        let d = DIRS[arm.face];
        let across = [at[0] + d[0], at[1] + d[1], at[2] + d[2]];
        if connects(connect, chunk, across, arm.face) {
            boxes.extend_from_slice(&arm.boxes);
        }
    }
    boxes
}

fn connects(connect: &ConnectDef, chunk: &Chunk, across: [i32; 3], face: usize) -> bool {
    let there = chunk.get(across[0] as usize, across[1] as usize, across[2] as usize);
    let group = match there.shape() {
        _ if there.is_air() => return false,
        FENCE => examples::FENCE_GROUP,
        PIPE => examples::PIPE_GROUP,
        _ => 0,
    };
    // Connected Shapes seal nothing, so the neighbour's bare boxes decide.
    group & connect.connects_to != 0
        || (connect.to_sealed && covers_whole_face(&declared(chunk, across, false), face ^ 1))
}

fn covers_whole_face(boxes: &[NodeBox], face: usize) -> bool {
    let (d, layer) = (face / 2, if face % 2 == 1 { 15 } else { 0 });
    (0..16).all(|u| {
        (0..16).all(|v| {
            let mut c = [0u8; 3];
            c[d] = layer;
            c[(d + 1) % 3] = u;
            c[(d + 2) % 3] = v;
            boxes
                .iter()
                .any(|b| (0..3).all(|a| b.min[a] <= c[a] && c[a] < b.max[a]))
        })
    })
}

/// Every face between a solid sub-cell and an empty one, from the declared
/// boxes alone - a different route to the geometry than the table's own grid.
fn exposed_faces(chunk: &Chunk) -> std::collections::BTreeSet<SubFace> {
    exposed_faces_unless(chunk, |_, _| true)
}

/// [`exposed_faces`] where a solid sub-cell of Material `across` hides the face
/// of one of Material `here` only if `hides(across, here)`. The caller states the
/// rule itself, so the mesher's is not what is marking its own work.
fn exposed_faces_unless(
    chunk: &Chunk,
    hides: impl Fn(u16, u16) -> bool,
) -> std::collections::BTreeSet<SubFace> {
    let mut solid = std::collections::HashMap::new();
    for z in LO..LO + SPAN {
        for y in LO..LO + SPAN {
            for x in LO..LO + SPAN {
                let v = chunk.get(x, y, z);
                for b in declared(chunk, [x as i32, y as i32, z as i32], true) {
                    for sz in b.min[2]..b.max[2] {
                        for sy in b.min[1]..b.max[1] {
                            for sx in b.min[0]..b.max[0] {
                                solid.insert(
                                    [
                                        (x * 16 + sx as usize) as i32,
                                        (y * 16 + sy as usize) as i32,
                                        (z * 16 + sz as usize) as i32,
                                    ],
                                    v.material(),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    let mut faces = std::collections::BTreeSet::new();
    for (s, here) in &solid {
        for (f, d) in DIRS.iter().enumerate() {
            let n = [s[0] + d[0], s[1] + d[1], s[2] + d[2]];
            // Inside one Voxel a sub-cell always hides its neighbour; the
            // Material's say starts at the Voxel's boundary.
            let same_voxel = (0..3).all(|a| s[a].div_euclid(16) == n[a].div_euclid(16));
            let hidden = solid
                .get(&n)
                .is_some_and(|across| same_voxel || hides(*across, *here));
            if !hidden {
                faces.insert((f, *s));
            }
        }
    }
    faces
}

/// Every sub-cell face the mesh draws, counted: a quad covers the squares of its
/// rectangle, each belonging to the solid sub-cell behind it.
fn drawn_faces(mesh: &tesserae_core::ChunkMesh) -> std::collections::BTreeMap<SubFace, u32> {
    let mut drawn = std::collections::BTreeMap::new();
    for layer in [&mesh.opaque, &mesh.blended, &mesh.cutout] {
        for (q, normal) in layer.positions.chunks(4).zip(layer.normals.chunks(4)) {
            let n = normal[0];
            let f = (0..6)
                .find(|&f| {
                    let d = DIRS[f];
                    n == [d[0] as f32, d[1] as f32, d[2] as f32]
                })
                .expect("axis-aligned normal");
            let d = f / 2;
            let lo: Vec<i32> = (0..3)
                .map(|a| q.iter().map(|p| (p[a] * 16.0) as i32).min().unwrap())
                .collect();
            let hi: Vec<i32> = (0..3)
                .map(|a| q.iter().map(|p| (p[a] * 16.0) as i32).max().unwrap())
                .collect();
            let (u, v) = ((d + 1) % 3, (d + 2) % 3);
            // The solid sub-cell is on the side the quad faces away from.
            let layer = if f % 2 == 1 { lo[d] - 1 } else { lo[d] };
            for a in lo[u]..hi[u] {
                for b in lo[v]..hi[v] {
                    let mut cell = [0i32; 3];
                    cell[d] = layer;
                    cell[u] = a;
                    cell[v] = b;
                    *drawn.entry((f, cell)).or_insert(0) += 1;
                }
            }
        }
    }
    drawn
}

fn random_scene(rng: &mut Rng) -> Chunk {
    let mut chunk = Chunk::air();
    for z in LO..LO + SPAN {
        for y in LO..LO + SPAN {
            for x in LO..LO + SPAN {
                let material = 1 + rng.below(2) as u16;
                let voxel = match rng.below(10) {
                    0..=3 => continue,
                    4..=5 => Voxel::cube(material),
                    pick => {
                        let shape = [SHAPE_SLAB, SHAPE_STAIR, SHAPE_POST][(pick as usize - 6) % 3];
                        Voxel::new(material, shape, rng.below(24) as u8)
                    }
                };
                chunk.set(x, y, z, voxel);
            }
        }
    }
    chunk
}

/// The test the occlusion rule exists to pass. Pass 2 is correct only if every
/// face a player could see is drawn, exactly once, whatever the Shapes and
/// Orientations around it - and overdraw is allowed, a hole never is.
#[test]
fn no_exposed_face_of_a_random_scene_is_missing_or_drawn_twice() {
    let mut rng = Rng(0x5eed_0005);
    for scene in 0..300 {
        let chunk = random_scene(&mut rng);
        let exposed = exposed_faces(&chunk);
        let drawn = drawn_faces(&mesh_isolated(&chunk));

        let missing: Vec<_> = exposed.iter().filter(|f| !drawn.contains_key(f)).collect();
        assert!(
            missing.is_empty(),
            "scene {scene}: {} of {} exposed faces are holes, e.g. {:?}",
            missing.len(),
            exposed.len(),
            &missing[..missing.len().min(4)]
        );
        let doubled: Vec<_> = exposed.iter().filter(|f| drawn[*f] > 1).collect();
        assert!(
            doubled.is_empty(),
            "scene {scene}: {} faces are drawn twice, e.g. {:?}",
            doubled.len(),
            &doubled[..doubled.len().min(4)]
        );
    }
}

/// Scenes that mix connected Shapes with everything else, so that arms meet
/// arms, cubes, slabs and stairs at every angle.
fn random_connected_scene(rng: &mut Rng) -> Chunk {
    let mut chunk = Chunk::air();
    for z in LO..LO + SPAN {
        for y in LO..LO + SPAN {
            for x in LO..LO + SPAN {
                let material = 1 + rng.below(2) as u16;
                let voxel = match rng.below(10) {
                    0..=2 => continue,
                    3 => Voxel::cube(material),
                    4..=5 => Voxel::new(material, FENCE, rng.below(256) as u8),
                    6 => Voxel::new(material, PIPE, 0),
                    pick => {
                        let shape = [SHAPE_SLAB, SHAPE_STAIR, SHAPE_POST][pick as usize - 7];
                        Voxel::new(material, shape, rng.below(24) as u8)
                    }
                };
                chunk.set(x, y, z, voxel);
            }
        }
    }
    chunk
}

fn assert_hole_free(chunk: &Chunk, what: &str) {
    let exposed = exposed_faces(chunk);
    let drawn = drawn_faces(&mesh_isolated(chunk));
    let missing: Vec<_> = exposed.iter().filter(|f| !drawn.contains_key(f)).collect();
    assert!(
        missing.is_empty(),
        "{what}: {} of {} exposed faces are holes, e.g. {:?}",
        missing.len(),
        exposed.len(),
        &missing[..missing.len().min(4)]
    );
    let doubled: Vec<_> = exposed.iter().filter(|f| drawn[*f] > 1).collect();
    assert!(
        doubled.is_empty(),
        "{what}: {} faces are drawn twice, e.g. {:?}",
        doubled.len(),
        &doubled[..doubled.len().min(4)]
    );
}

/// The same property, with arms in the mix. The oracle grows its arms by its
/// own copy of the rule, so a wrong face, a missing connection or an arm toward
/// everything shows as a hole or as an arm that should not be there.
#[test]
fn no_exposed_face_of_a_random_scene_with_connected_shapes_is_missing_or_drawn_twice() {
    let mut rng = Rng(0x5eed_0008);
    for scene in 0..400 {
        assert_hole_free(&random_connected_scene(&mut rng), &format!("scene {scene}"));
    }
}

/// An arm that is drawn where the rule does not put one is not a hole, so the
/// property above does not see it from the mesh side. The other direction is
/// covered by the oracle: its solid sub-cells are the only ones that count, and
/// every drawn face must have a solid sub-cell behind it.
#[test]
fn every_face_a_connected_scene_draws_has_a_solid_sub_cell_behind_it() {
    let mut rng = Rng(0x5eed_0009);
    for scene in 0..200 {
        let chunk = random_connected_scene(&mut rng);
        let solid_behind: std::collections::HashSet<_> = {
            let mut set = std::collections::HashSet::new();
            for z in LO..LO + SPAN {
                for y in LO..LO + SPAN {
                    for x in LO..LO + SPAN {
                        for b in declared(&chunk, [x as i32, y as i32, z as i32], true) {
                            for sz in b.min[2]..b.max[2] {
                                for sy in b.min[1]..b.max[1] {
                                    for sx in b.min[0]..b.max[0] {
                                        set.insert([
                                            (x * 16 + sx as usize) as i32,
                                            (y * 16 + sy as usize) as i32,
                                            (z * 16 + sz as usize) as i32,
                                        ]);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            set
        };
        for (_, cell) in drawn_faces(&mesh_isolated(&chunk)).keys() {
            assert!(
                solid_behind.contains(cell),
                "scene {scene}: a face is drawn on {cell:?}, which nothing occupies"
            );
        }
    }
}

fn place_fences(at: &[(usize, usize, usize)]) -> Chunk {
    let mut chunk = Chunk::air();
    for &(x, y, z) in at {
        chunk.set(x, y, z, Voxel::new(1, FENCE, 0));
    }
    chunk
}

#[test]
fn a_line_of_fences_draws_no_hidden_end_caps() {
    let chunk = place_fences(&[(10, 10, 10), (11, 10, 10), (12, 10, 10), (13, 10, 10)]);
    let exposed = exposed_faces(&chunk);
    let drawn = drawn_faces(&mesh_isolated(&chunk));
    let hidden: Vec<_> = drawn.keys().filter(|f| !exposed.contains(f)).collect();
    assert!(hidden.is_empty(), "{} hidden faces drawn", hidden.len());
    assert!(exposed.iter().all(|f| drawn.contains_key(f)));
    // And the arms are there: four posts alone would be 4 * 6 quads.
    assert!(mesh_isolated(&chunk).quad_count() > 4 * 6);
}

#[test]
fn a_fence_grows_an_arm_towards_a_cube_and_the_cube_hides_its_end() {
    let mut chunk = place_fences(&[(10, 10, 10)]);
    let alone = mesh_isolated(&chunk).quad_count();
    chunk.set(11, 10, 10, Voxel::cube(1));
    assert_hole_free(&chunk, "fence beside cube");
    // The cube's own face under the bars is drawn, because pass 1 culls only
    // against a neighbour that seals and a fence never does: hidden overdraw,
    // not a hole. The arm's end cap, on the same plane, is not drawn.
    let exposed = exposed_faces(&chunk);
    let hidden: Vec<_> = drawn_faces(&mesh_isolated(&chunk))
        .into_keys()
        .filter(|f| !exposed.contains(f))
        .collect();
    let cube_side = 11 * 16;
    assert!(
        hidden
            .iter()
            .all(|&(face, cell)| face == NEG_X && cell[0] == cube_side),
        "{hidden:?}"
    );
    assert_eq!(hidden.len(), 12, "one bar of 2 x 6 sub-cells");
    // The cube adds its own five faces, so the arm shows as the quads beyond it.
    let with = mesh_isolated(&chunk).quad_count();
    assert!(with > alone + 5, "{with} against {alone}");
}

#[test]
fn a_fence_beside_a_stair_and_a_slab_is_hole_free_whichever_way_they_face() {
    for turn in 0..4 {
        for up in [POS_Y, NEG_Y] {
            let mut chunk = place_fences(&[(10, 10, 10)]);
            chunk.set(11, 10, 10, shaped(SHAPE_STAIR, up, turn));
            chunk.set(9, 10, 10, shaped(SHAPE_SLAB, up, turn));
            assert_hole_free(&chunk, &format!("turn {turn} up {up}"));
        }
    }
}

/// The shell is where an arm toward the next Chunk is read, and the neighbour's
/// coverage is what hides its end.
#[test]
fn an_arm_across_a_chunk_boundary_is_read_from_the_shell() {
    let last = CHUNK_SIZE - 1;
    let mut a = Chunk::air();
    a.set(last, 10, 10, Voxel::new(1, FENCE, 0));
    let mesh = |b: &Chunk| {
        let neighbors = [None, Some(unlit(b)), None, None, None, None];
        let m = mesh_chunk(unlit(&a), neighbors, tables());
        let cap_on_the_boundary = m
            .opaque
            .positions
            .chunks(4)
            .zip(m.opaque.normals.chunks(4))
            .any(|(q, n)| n[0] == [1.0, 0.0, 0.0] && q.iter().all(|p| p[0] == CHUNK_SIZE as f32));
        (m.quad_count(), cap_on_the_boundary)
    };

    let (bare, bare_cap) = mesh(&Chunk::air());
    assert!(!bare_cap);

    let fence = {
        let mut b = Chunk::air();
        b.set(0, 10, 10, Voxel::new(1, FENCE, 0));
        b
    };
    let (joined, cap) = mesh(&fence);
    assert!(joined > bare, "{joined} against {bare}: no arm was grown");
    assert!(
        !cap,
        "the arm's end is drawn against the fence that meets it"
    );

    let pipe = {
        let mut b = Chunk::air();
        b.set(0, 10, 10, Voxel::new(1, PIPE, 0));
        b
    };
    assert_eq!(mesh(&pipe).0, bare, "a pipe is another group: no arm");

    let wall = {
        let mut b = Chunk::air();
        b.set(0, 10, 10, Voxel::cube(1));
        b
    };
    let (walled, wall_cap) = mesh(&wall);
    assert!(walled > bare && !wall_cap);
}

/// Culling by coverage rather than by seal is what keeps a floor of slabs from
/// carrying four hidden quads on every slab.
#[test]
fn the_interior_of_a_slab_floor_draws_no_hidden_faces() {
    let mut chunk = Chunk::air();
    for z in LO..LO + SPAN {
        for x in LO..LO + SPAN {
            chunk.set(x, LO, z, shaped(SHAPE_SLAB, POS_Y, 0));
        }
    }
    let exposed = exposed_faces(&chunk);
    let drawn = drawn_faces(&mesh_isolated(&chunk));
    let hidden: Vec<_> = drawn.keys().filter(|f| !exposed.contains(f)).collect();
    assert!(hidden.is_empty(), "{} hidden faces drawn", hidden.len());
    assert!(exposed.iter().all(|f| drawn.contains_key(f)));
}

/// A stair's tread is inside its own cell and looks into the stair's own air,
/// which the stair holds light for; its sealed back looks out at the Voxel
/// behind it. Same rule as pass 1, "the light of the Voxel in front".
#[test]
fn a_stairs_tread_is_lit_by_its_own_voxel_and_its_back_by_the_neighbour() {
    let mut c = Chunk::air();
    c.set(10, 10, 10, shaped(SHAPE_STAIR, POS_Y, 0));
    let mut light = ChunkLight::dark();
    light.set(10, 10, 10, Light::new(7, 3));
    light.set(10, 10, 11, Light::new(2, 1));
    light.set(10, 10, 9, Light::new(4, 0));
    light.set(10, 11, 10, Light::new(9, 9));
    let mesh = greedy_mesh(&Neighborhood::isolated(LitChunk::new(&c, &light)), tables());

    let lit_at = |normal: [f32; 3], axis: usize, at: f32| -> Vec<u32> {
        (0..mesh.opaque.positions.len())
            .filter(|&i| mesh.opaque.normals[i] == normal && mesh.opaque.positions[i][axis] == at)
            .map(|i| mesh.opaque.light[i])
            .collect()
    };
    let packed = |sky, block| Light::new(sky, block).packed() as u32;

    // Inside the cell: the tread at half height, and the riser half way back.
    assert_eq!(lit_at([0.0, 1.0, 0.0], 1, 10.5), vec![packed(7, 3); 4]);
    assert_eq!(lit_at([0.0, 0.0, -1.0], 2, 10.5), vec![packed(7, 3); 4]);
    // On the boundary: the front looks at -Z, the back at +Z, the top upwards.
    assert_eq!(lit_at([0.0, 0.0, -1.0], 2, 10.0), vec![packed(4, 0); 4]);
    assert_eq!(lit_at([0.0, 0.0, 1.0], 2, 11.0), vec![packed(2, 1); 4]);
    assert_eq!(lit_at([0.0, 1.0, 0.0], 1, 11.0), vec![packed(9, 9); 4]);
}

/// One of every built-in Shape in every Orientation, far enough apart that none
/// touches another.
fn catalogue() -> Chunk {
    let mut c = Chunk::air();
    let mut i = 0;
    for shape in [SHAPE_SLAB, SHAPE_STAIR, SHAPE_POST] {
        for o in Orientation::all() {
            c.set(
                1 + 2 * (i % 12),
                4,
                1 + 2 * (i / 12),
                Voxel::shaped(1, shape, o),
            );
            i += 1;
        }
    }
    c
}

#[test]
fn pass_2_geometry_is_wound_to_its_normal_planar_and_in_lockstep() {
    let mesh = mesh_isolated(&catalogue());
    assert_eq!(mesh.shaped_voxels, 72);
    assert!(mesh.quad_count() > 72);

    for tri in mesh.opaque.indices.chunks(3) {
        let p: Vec<[f32; 3]> = tri
            .iter()
            .map(|&i| mesh.opaque.positions[i as usize])
            .collect();
        let e1 = [p[1][0] - p[0][0], p[1][1] - p[0][1], p[1][2] - p[0][2]];
        let e2 = [p[2][0] - p[0][0], p[2][1] - p[0][1], p[2][2] - p[0][2]];
        let cross = [
            e1[1] * e2[2] - e1[2] * e2[1],
            e1[2] * e2[0] - e1[0] * e2[2],
            e1[0] * e2[1] - e1[1] * e2[0],
        ];
        let n = mesh.opaque.normals[tri[0] as usize];
        let dot = cross[0] * n[0] + cross[1] * n[1] + cross[2] * n[2];
        assert!(dot > 0.0, "winding flipped: {cross:?} vs normal {n:?}");
    }
    for quad in mesh.opaque.positions.chunks(4) {
        let axes = (0..3)
            .filter(|&ax| quad.iter().all(|p| p[ax] == quad[0][ax]))
            .count();
        assert_eq!(axes, 1, "quad {quad:?} is not on one axis-aligned plane");
    }
    let n = mesh.opaque.positions.len();
    assert_eq!(mesh.opaque.normals.len(), n);
    assert_eq!(mesh.opaque.uvs.len(), n);
    assert_eq!(mesh.opaque.materials.len(), n);
    assert_eq!(mesh.opaque.light.len(), n);
    assert!(mesh.opaque.indices.iter().all(|&i| (i as usize) < n));
}

/// A half-height slab side shows half the texture, not a squashed whole one.
#[test]
fn uvs_are_the_position_inside_the_voxel() {
    let mut c = Chunk::air();
    c.set(10, 10, 10, shaped(SHAPE_SLAB, POS_Y, 0));
    let mesh = mesh_isolated(&c);
    for i in 0..mesh.opaque.positions.len() {
        if mesh.opaque.normals[i] == [1.0, 0.0, 0.0] {
            // A +X face is on axes (y, z), so (u, v) = (y, z) inside the cell.
            let p = mesh.opaque.positions[i];
            assert_eq!(mesh.opaque.uvs[i], [p[1] - 10.0, p[2] - 10.0]);
            assert!(
                mesh.opaque.uvs[i][0] <= 0.5,
                "the slab's side is half height"
            );
        }
    }
}

// Translucent Materials: what a face is hidden by is the Material across it.

use tesserae_core::material::{GLASS, LEAVES, STONE, WATER};
use tesserae_core::{MaterialDef, Occlusion, RenderClass};

fn put(chunk: &mut Chunk, at: [usize; 3], voxel: Voxel) {
    chunk.set(at[0], at[1], at[2], voxel);
}

/// The normals of every quad of a layer, one per quad.
fn quad_normals(layer: &tesserae_core::MeshLayer) -> Vec<[f32; 3]> {
    layer.normals.chunks(4).map(|n| n[0]).collect()
}

/// A second, differently coloured glass: same properties, another Material.
fn with_tinted_glass() -> (Tables, u16) {
    let mut t = Tables::builtin();
    let id = t
        .materials
        .register(MaterialDef {
            emission: 0,
            dimming: 0,
            occlusion: Occlusion::SameMaterial,
            render: RenderClass::Blend,
            solid: true,
        })
        .unwrap();
    (t, id)
}

#[test]
fn glass_beside_glass_is_a_skin_and_not_a_honeycomb() {
    let mut c = Chunk::air();
    put(&mut c, [10, 10, 10], Voxel::cube(GLASS));
    put(&mut c, [11, 10, 10], Voxel::cube(GLASS));
    let mesh = mesh_isolated(&c);
    // One box: six faces, the shared one gone and the rest merged across both.
    assert_eq!(mesh.blended.quad_count(), 6);
    assert!(mesh.opaque.is_empty());
    let on_seam = mesh
        .blended
        .positions
        .chunks(4)
        .filter(|quad| quad.iter().all(|p| p[0] == 11.0))
        .count();
    assert_eq!(on_seam, 0);
}

#[test]
fn two_different_glasses_keep_the_face_between_them() {
    let (tables, tinted) = with_tinted_glass();
    let mut c = Chunk::air();
    put(&mut c, [10, 10, 10], Voxel::cube(GLASS));
    put(&mut c, [11, 10, 10], Voxel::cube(tinted));
    let mesh = greedy_mesh(&Neighborhood::isolated(unlit(&c)), &tables);
    // Each keeps all six faces, and the two coplanar outer faces cannot merge.
    assert_eq!(mesh.blended.quad_count(), 12);
}

#[test]
fn leaves_hide_nothing_not_even_each_other() {
    let mut c = Chunk::air();
    put(&mut c, [10, 10, 10], Voxel::cube(LEAVES));
    put(&mut c, [11, 10, 10], Voxel::cube(LEAVES));
    let mesh = mesh_isolated(&c);
    // The two shared faces are drawn, one facing each way, on the plane x=11.
    let facing_seam: Vec<_> = mesh
        .cutout
        .positions
        .chunks(4)
        .zip(quad_normals(&mesh.cutout))
        .filter(|(quad, _)| quad.iter().all(|p| p[0] == 11.0))
        .map(|(_, n)| n)
        .collect();
    assert_eq!(facing_seam.len(), 2);
    assert!(facing_seam.contains(&[1.0, 0.0, 0.0]) && facing_seam.contains(&[-1.0, 0.0, 0.0]));
    // Leaves have a pass of their own, which may discard.
    assert!(mesh.opaque.is_empty() && mesh.blended.is_empty());
}

#[test]
fn stone_beside_glass_keeps_its_face_and_glass_beside_stone_loses_its_own() {
    let mut c = Chunk::air();
    put(&mut c, [10, 10, 10], Voxel::cube(STONE));
    put(&mut c, [11, 10, 10], Voxel::cube(GLASS));
    let mesh = mesh_isolated(&c);
    assert_eq!(mesh.opaque.quad_count(), 6, "stone is seen through glass");
    assert_eq!(
        mesh.blended.quad_count(),
        5,
        "glass against stone is hidden"
    );
    assert!(
        !quad_normals(&mesh.blended).contains(&[-1.0, 0.0, 0.0]),
        "the glass face towards the stone"
    );
}

#[test]
fn every_quad_lands_in_the_layer_of_its_materials_render_class() {
    let mut c = Chunk::air();
    for (i, m) in [STONE, GLASS, LEAVES, WATER].into_iter().enumerate() {
        put(&mut c, [10 + 2 * i, 10, 10], Voxel::cube(m));
        put(
            &mut c,
            [10 + 2 * i, 14, 10],
            Voxel::shaped(m, tesserae_core::SHAPE_SLAB, Orientation::IDENTITY),
        );
    }
    let mesh = mesh_isolated(&c);
    let class = |m: u32| tables().materials.def(m as u16).render;
    assert!(
        mesh.opaque
            .materials
            .iter()
            .all(|&m| class(m) == RenderClass::Opaque)
    );
    assert!(
        mesh.blended
            .materials
            .iter()
            .all(|&m| class(m) == RenderClass::Blend)
    );
    assert!(
        mesh.cutout
            .materials
            .iter()
            .all(|&m| class(m) == RenderClass::Cutout)
    );
    for (layer, expected) in [
        (&mesh.opaque, vec![STONE]),
        (&mesh.blended, vec![GLASS, WATER]),
        (&mesh.cutout, vec![LEAVES]),
    ] {
        for m in expected {
            assert!(layer.materials.contains(&(m as u32)), "material {m}");
        }
    }
    for layer in [&mesh.opaque, &mesh.blended, &mesh.cutout] {
        assert_eq!(layer.positions.len(), layer.normals.len());
        assert_eq!(layer.positions.len(), layer.uvs.len());
        assert_eq!(layer.positions.len(), layer.materials.len());
        assert_eq!(layer.positions.len(), layer.light.len());
    }
}

#[test]
fn a_shaped_voxel_is_hidden_only_by_a_neighbour_that_hides_it() {
    // A slab of stone beside a cube of stone loses its sealed side; beside a cube
    // of glass it keeps it, because glass proves nothing about stone.
    let slab = Voxel::shaped(STONE, tesserae_core::SHAPE_SLAB, Orientation::IDENTITY);
    let faces_towards_neighbour = |beside: Voxel| {
        let mut c = Chunk::air();
        put(&mut c, [10, 10, 10], slab);
        put(&mut c, [11, 10, 10], beside);
        let mesh = mesh_isolated(&c);
        // Quads lying on the shared plane x=11 and facing +X are the slab's.
        mesh.opaque
            .positions
            .chunks(4)
            .zip(quad_normals(&mesh.opaque))
            .filter(|(quad, n)| *n == [1.0, 0.0, 0.0] && quad.iter().all(|p| p[0] == 11.0))
            .count()
    };
    assert_eq!(faces_towards_neighbour(Voxel::cube(STONE)), 0);
    assert_eq!(faces_towards_neighbour(Voxel::cube(GLASS)), 1);
    assert_eq!(faces_towards_neighbour(Voxel::cube(LEAVES)), 1);
}

#[test]
fn terrain_meshes_to_one_layer_exactly_as_it_did() {
    let mesh = mesh_isolated(&testgen::terrain_chunk(0, 0, 0));
    assert!(mesh.blended.is_empty());
    assert!(!mesh.opaque.is_empty());
}

#[test]
fn a_uniform_chunk_declines_only_if_the_material_hides_itself_and_what_is_beside_it() {
    let uniform = |m| Chunk::filled(Voxel::cube(m));
    let declines = |centre: &Chunk, around: &Chunk| {
        mesh_chunk(unlit(centre), enclosed_by(around), tables()).is_empty()
    };
    assert!(declines(&uniform(STONE), &uniform(STONE)));
    assert!(
        declines(&uniform(GLASS), &uniform(GLASS)),
        "a block of glass is a skin"
    );
    assert!(
        declines(&uniform(GLASS), &uniform(STONE)),
        "stone hides glass"
    );
    assert!(declines(&uniform(WATER), &uniform(WATER)));
    // Stone's faces are seen through the glass beside it.
    assert!(!declines(&uniform(STONE), &uniform(GLASS)));
    // Leaves draw every face, inside the Chunk as well as on its boundary.
    assert!(!declines(&uniform(LEAVES), &uniform(LEAVES)));
    assert!(!declines(&uniform(LEAVES), &uniform(STONE)));
    // Water beside glass: neither hides the other.
    assert!(!declines(&uniform(WATER), &uniform(GLASS)));
}

/// The occlusion rule, restated from the spec and not from the mesher.
fn hides(across: u16, here: u16) -> bool {
    match across {
        GLASS | WATER => across == here,
        LEAVES => false,
        _ => true,
    }
}

fn random_translucent_scene(rng: &mut Rng) -> Chunk {
    let mut chunk = Chunk::air();
    for z in LO..LO + SPAN {
        for y in LO..LO + SPAN {
            for x in LO..LO + SPAN {
                let material = [STONE, GLASS, LEAVES, WATER][rng.below(4) as usize];
                let voxel = match rng.below(10) {
                    0..=3 => continue,
                    4..=7 => Voxel::cube(material),
                    pick => {
                        let shape = [
                            tesserae_core::SHAPE_SLAB,
                            tesserae_core::SHAPE_STAIR,
                            tesserae_core::SHAPE_POST,
                        ][(pick as usize - 8 + rng.below(2) as usize) % 3];
                        Voxel::new(material, shape, rng.below(24) as u8)
                    }
                };
                chunk.set(x, y, z, voxel);
            }
        }
    }
    chunk
}

/// The step-5 test again with Materials that hide differently. No face that the
/// stated rule says is visible may be missing from either layer, and none may be
/// drawn twice; drawing a hidden one is allowed, because the failure mode of an
/// unproven face is overdraw.
#[test]
fn no_exposed_face_of_a_random_translucent_scene_is_missing_or_drawn_twice() {
    let mut rng = Rng(0x5eed_0008);
    let mut seen_through = 0;
    for scene in 0..200 {
        let chunk = random_translucent_scene(&mut rng);
        let exposed = exposed_faces_unless(&chunk, hides);
        let all = exposed_faces(&chunk);
        seen_through += exposed.len() - all.len();
        let drawn = drawn_faces(&mesh_isolated(&chunk));

        let missing: Vec<_> = exposed.iter().filter(|f| !drawn.contains_key(f)).collect();
        assert!(
            missing.is_empty(),
            "scene {scene}: {} of {} exposed faces are holes, e.g. {:?}",
            missing.len(),
            exposed.len(),
            &missing[..missing.len().min(4)]
        );
        let doubled: Vec<_> = exposed.iter().filter(|f| drawn[*f] > 1).collect();
        assert!(
            doubled.is_empty(),
            "scene {scene}: {} faces are drawn twice, e.g. {:?}",
            doubled.len(),
            &doubled[..doubled.len().min(4)]
        );
    }
    // The scenes must exercise the rule, or this passes for the wrong reason:
    // faces an opaque-only mesher would have dropped and this one must keep.
    assert!(
        seen_through > 1000,
        "only {seen_through} faces seen through"
    );
}

/// The other direction: the mesher culls what it can prove, so a scene of glass
/// has fewer drawn faces than the exposed-by-geometry count, not more.
#[test]
fn a_block_of_glass_draws_only_its_skin() {
    let mut c = Chunk::air();
    for z in LO..LO + SPAN {
        for y in LO..LO + SPAN {
            for x in LO..LO + SPAN {
                c.set(x, y, z, Voxel::cube(GLASS));
            }
        }
    }
    let mesh = mesh_isolated(&c);
    assert_eq!(mesh.blended.quad_count(), 6);
    let mut stone = Chunk::air();
    put(&mut stone, [20, 20, 20], Voxel::cube(STONE));
    assert!(mesh_isolated(&stone).blended.is_empty());
}

// Generated terrain. Slope smoothing puts stairs and slabs on real ground, so
// the checks above are repeated on the input the mesher actually meets: a Chunk
// with real neighbours, not a hand-built scene in an isolated shell.

fn world() -> Worldgen {
    Worldgen::new(0x7e55_e2ae, 1.0, WorldgenParams::default())
}

/// Chunk coordinates near the origin, at the height the surface crosses it,
/// whose Chunk holds at least `min` shaped Voxels. In a fixed scan order.
fn ramp_chunks(g: &Worldgen, min: usize) -> Vec<[i32; 3]> {
    let cy = (g.surface_height_m(0.0, 0.0) / CHUNK_SIZE as f32).floor() as i32;
    let mut found = vec![];
    for cz in -3..3 {
        for cx in -3..3 {
            if g.chunk(cx, cy, cz).shaped_count() >= min {
                found.push([cx, cy, cz]);
            }
        }
    }
    found
}

/// A Chunk and the 26 around it, indexed `(dz + 1) * 9 + (dy + 1) * 3 + dx + 1`.
fn block_around(g: &Worldgen, c: [i32; 3]) -> Vec<Chunk> {
    (-1..=1)
        .flat_map(|dz| (-1..=1).flat_map(move |dy| (-1..=1).map(move |dx| [dx, dy, dz])))
        .map(|d| g.chunk(c[0] + d[0], c[1] + d[1], c[2] + d[2]))
        .collect()
}

/// The Voxel at `at` Voxels from the middle Chunk's corner, which may be up to a
/// Chunk outside it on any axis.
fn voxel_in(block: &[Chunk], at: [i32; 3]) -> Voxel {
    let size = CHUNK_SIZE as i32;
    let d = at.map(|v| v.div_euclid(size) + 1);
    let l = at.map(|v| v.rem_euclid(size) as usize);
    block[(d[2] * 9 + d[1] * 3 + d[0]) as usize].get(l[0], l[1], l[2])
}

fn solid_sub_cell(block: &[Chunk], sub: [i32; 3]) -> bool {
    let v = voxel_in(block, sub.map(|s| s.div_euclid(16)));
    if v.is_air() {
        return false;
    }
    let l = sub.map(|s| s.rem_euclid(16) as u8);
    shapes()
        .selection(v, Connections::NONE)
        .iter()
        .any(|b| (0..3).all(|a| b.min[a] <= l[a] && l[a] < b.max[a]))
}

/// The exposed faces of the Voxels in a cube of `radius` round `centre`, clipped
/// to the middle Chunk, judged against the real World across its edges.
fn exposed_near(
    block: &[Chunk],
    centre: [i32; 3],
    radius: i32,
) -> std::collections::BTreeSet<SubFace> {
    let size = CHUNK_SIZE as i32;
    let mut faces = std::collections::BTreeSet::new();
    let range = |c: i32| (c - radius).max(0)..=(c + radius).min(size - 1);
    for z in range(centre[2]) {
        for y in range(centre[1]) {
            for x in range(centre[0]) {
                let v = voxel_in(block, [x, y, z]);
                if v.is_air() {
                    continue;
                }
                let cube = v.is_full_cube();
                for sz in 0..16 {
                    for sy in 0..16 {
                        for sx in 0..16 {
                            // The interior of a cube is not exposed to anything,
                            // and is most of the sub-cells.
                            if cube && ![sx, sy, sz].iter().any(|&s| s == 0 || s == 15) {
                                continue;
                            }
                            let s = [x * 16 + sx, y * 16 + sy, z * 16 + sz];
                            if !solid_sub_cell(block, s) {
                                continue;
                            }
                            for (f, d) in DIRS.iter().enumerate() {
                                let n = [s[0] + d[0], s[1] + d[1], s[2] + d[2]];
                                if !solid_sub_cell(block, n) {
                                    faces.insert((f, s));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    faces
}

fn mesh_with_neighbours(block: &[Chunk]) -> tesserae_core::ChunkMesh {
    let at = |dx: usize, dy: usize, dz: usize| &block[dz * 9 + dy * 3 + dx];
    let refs = [
        at(0, 1, 1),
        at(2, 1, 1),
        at(1, 0, 1),
        at(1, 2, 1),
        at(1, 1, 0),
        at(1, 1, 2),
    ]
    .map(|c| Some(unlit(c)));
    mesh_chunk(unlit(at(1, 1, 1)), refs, tables())
}

/// Around the first shaped Voxel of a Chunk in the interior, one on each kind of
/// Chunk edge, and one against the floor and ceiling: every face a player could
/// see is drawn, once, with the neighbour Chunks' real Voxels across the seam.
#[test]
fn no_exposed_face_of_generated_ramps_is_missing_or_drawn_twice() {
    let g = world();
    let chunks = ramp_chunks(&g, 20);
    assert!(!chunks.is_empty(), "no ramp near the origin");
    let size = CHUNK_SIZE as i32;
    let (mut checked, mut seams) = (0, 0);
    for coord in chunks.into_iter().take(3) {
        let block = block_around(&g, coord);
        let middle = &block[13];
        let shaped: Vec<[i32; 3]> = (0..size)
            .flat_map(|z| (0..size).flat_map(move |y| (0..size).map(move |x| [x, y, z])))
            .filter(|p| {
                middle
                    .get(p[0] as usize, p[1] as usize, p[2] as usize)
                    .is_shaped()
            })
            .collect();
        let on_edge = |p: &&[i32; 3]| [p[0], p[2]].iter().any(|&c| c == 0 || c == size - 1);
        let picks = [
            shaped.first(),
            shaped.iter().find(on_edge),
            shaped.iter().find(|p| !on_edge(p)),
        ];
        let drawn = drawn_faces(&mesh_with_neighbours(&block));
        for centre in picks.into_iter().flatten() {
            let exposed = exposed_near(&block, *centre, 2);
            let missing: Vec<_> = exposed.iter().filter(|f| !drawn.contains_key(f)).collect();
            assert!(
                missing.is_empty(),
                "{coord:?} near {centre:?}: {} of {} exposed faces are holes, e.g. {:?}",
                missing.len(),
                exposed.len(),
                &missing[..missing.len().min(4)]
            );
            let doubled: Vec<_> = exposed.iter().filter(|f| drawn[*f] > 1).collect();
            assert!(
                doubled.is_empty(),
                "{coord:?} near {centre:?}: {} faces drawn twice, e.g. {:?}",
                doubled.len(),
                &doubled[..doubled.len().min(4)]
            );
            checked += exposed.len();
            seams += usize::from(on_edge(&centre));
        }
    }
    assert!(checked > 1000, "only {checked} faces checked");
    assert!(seams > 0, "no ramp on a Chunk seam was checked");
}

#[test]
fn declining_never_changes_the_mesh_of_a_ramp_chunk() {
    let g = world();
    let coord = *ramp_chunks(&g, 20).first().expect("a ramp near the origin");
    let block = block_around(&g, coord);
    assert!(block[13].may_contain_shaped());
    let fast = mesh_with_neighbours(&block);
    let refs = [12, 14, 10, 16, 4, 22].map(|i| Some(unlit(&block[i])));
    let slow = greedy_mesh(&Neighborhood::new(unlit(&block[13]), refs), tables());
    assert_eq!(fast.opaque.positions, slow.opaque.positions);
    assert_eq!(fast.opaque.indices, slow.opaque.indices);
    assert_eq!(fast.opaque.light, slow.opaque.light);
    assert!(fast.shaped_voxels > 0);
}

fn slabs(from: [usize; 3], to: [usize; 3], material: u16) -> Chunk {
    let mut chunk = Chunk::air();
    fill_slabs(&mut chunk, from, to, material);
    chunk
}

fn fill_slabs(chunk: &mut Chunk, from: [usize; 3], to: [usize; 3], material: u16) {
    for z in from[2]..to[2] {
        for y in from[1]..to[1] {
            for x in from[0]..to[0] {
                chunk.set(
                    x,
                    y,
                    z,
                    Voxel::shaped(material, SHAPE_SLAB, Orientation::IDENTITY),
                );
            }
        }
    }
}

#[test]
fn a_floor_of_slabs_is_one_top_one_bottom_and_four_sides() {
    let floor = slabs([4, 8, 4], [12, 9, 12], 1);
    assert_eq!(mesh_isolated(&floor).quad_count(), 6);
    assert_hole_free(&floor, "slab floor");
    let big = slabs([0, 8, 0], [32, 9, 32], 1);
    assert_eq!(mesh_isolated(&big).quad_count(), 6);
}

#[test]
fn a_line_of_fences_costs_the_same_for_each_fence_added_along_either_axis() {
    // Post: a top, a bottom, two plain sides and two sides cut around the bar
    // (4 each), 12; the bar between two posts is 4 quads in all, not 8. Adding
    // a fence to a line of 4 or more adds one post and one link: 16.
    for axis in [0, 2] {
        let line = |n: usize| {
            let mut chunk = Chunk::air();
            for i in 0..n {
                let mut at = [8, 8, 8];
                at[axis] += i;
                chunk.set(at[0], at[1], at[2], Voxel::new(1, FENCE, 0));
            }
            let count = mesh_isolated(&chunk).quad_count();
            assert_hole_free(&chunk, &format!("line of {n} along {axis}"));
            count
        };
        for n in 4..8 {
            assert_eq!(line(n + 1) - line(n), 16, "axis {axis}, {n} to {}", n + 1);
        }
    }
}

#[test]
fn nothing_merges_across_a_step_in_light() {
    let floor = slabs([2, 8, 4], [10, 9, 12], 1);
    let light = |step: bool| {
        let mut l = ChunkLight::dark();
        for z in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let level = if step && x >= 5 { 8 } else { MAX_LIGHT };
                    l.set(x, y, z, Light::new(level, 0));
                }
            }
        }
        l
    };
    let count = |l: &ChunkLight| {
        greedy_mesh(&Neighborhood::isolated(LitChunk::new(&floor, l)), tables()).quad_count()
    };
    assert_eq!(count(&light(false)), 6);
    // The top and bottom and the two long sides each split in two; the ends
    // look out at one level each.
    assert_eq!(count(&light(true)), 10);
}

#[test]
fn nothing_merges_across_a_change_of_material() {
    let mut floor = slabs([2, 8, 4], [6, 9, 12], 1);
    fill_slabs(&mut floor, [6, 8, 4], [10, 9, 12], 2);
    assert_eq!(mesh_isolated(&floor).quad_count(), 10);
    assert_hole_free(&floor, "two materials");
}

#[test]
fn a_gap_is_not_merged_across() {
    let mut floor = slabs([2, 8, 4], [5, 9, 12], 1);
    fill_slabs(&mut floor, [6, 8, 4], [9, 9, 12], 1);
    assert_eq!(mesh_isolated(&floor).quad_count(), 12);
}

#[test]
fn a_merged_quad_keeps_the_textures_per_voxel_repeat_and_a_slabs_half_height() {
    let floor = slabs([2, 8, 4], [10, 9, 12], 1);
    let layer = mesh_isolated(&floor).opaque;
    for (i, (q, uv)) in layer
        .positions
        .chunks(4)
        .zip(layer.uvs.chunks(4))
        .enumerate()
    {
        let n = layer.normals[i * 4];
        let d = (0..3).find(|&a| n[a] != 0.0).unwrap();
        for (k, a) in [(d + 1) % 3, (d + 2) % 3].into_iter().enumerate() {
            // The UV is the position less a whole number of Voxels, the same
            // for all four corners, so a tiling sampler still repeats once per
            // Voxel as it did before merging.
            let shift = q[0][a] - uv[0][k];
            assert!((shift - shift.round()).abs() < 1e-5, "{uv:?} against {q:?}");
            for c in 1..4 {
                assert!((q[c][a] - uv[c][k] - shift).abs() < 1e-5, "{uv:?} {q:?}");
            }
            // A slab's side is half a texture tall, not a squashed whole one.
            if a == 1 {
                let tall = uv.iter().map(|c| c[k]).fold(0.0f32, f32::max)
                    - uv.iter().map(|c| c[k]).fold(f32::MAX, f32::min);
                assert!((tall - 0.5).abs() < 1e-5, "{tall}");
            }
        }
    }
}

#[test]
fn a_row_of_slabs_never_leaves_its_chunk() {
    let row = slabs([0, 8, 8], [32, 9, 9], 1);
    let mesh = mesh_isolated(&row);
    assert!(
        mesh.opaque
            .positions
            .iter()
            .all(|p| (0.0..=32.0).contains(&p[0]))
    );
}

fn flowing(surface: u8) -> tesserae_core::Overlay {
    tesserae_core::Overlay {
        fluid: Some(tesserae_core::Fluid {
            source: false,
            surface,
            reach: 1,
            ..tesserae_core::Fluid::source(tesserae_core::material::WATER)
        }),
        burning: None,
    }
}

/// Every vertex height of the blended layer's quads facing `normal`.
fn blended_heights(mesh: &tesserae_core::ChunkMesh, normal: [f32; 3]) -> Vec<f32> {
    let layer = &mesh.blended;
    let mut ys: Vec<f32> = (0..layer.positions.len())
        .filter(|&i| layer.normals[i] == normal)
        .map(|i| layer.positions[i][1])
        .collect();
    ys.sort_by(f32::total_cmp);
    ys.dedup();
    ys
}

/// Flowing water in the overlay has geometry though its packed Voxel is air:
/// a box up to its surface, top face at the surface, on the blended pass.
#[test]
fn flowing_water_meshes_to_its_surface() {
    let chunk = Chunk::air();
    let mut overlay = tesserae_core::ChunkOverlay::none();
    overlay.set(4, 4, 4, flowing(8));
    let lit = unlit(&chunk).with_overlay(&overlay);
    let mesh = mesh_chunk(lit, [None; 6], tables());
    assert_eq!(mesh.blended.quad_count(), 6, "a box of six faces");
    assert!(mesh.opaque.is_empty());
    assert_eq!(blended_heights(&mesh, [0.0, 1.0, 0.0]), vec![4.5]);
    assert_eq!(blended_heights(&mesh, [1.0, 0.0, 0.0]), vec![4.0, 4.5]);

    // Full, it is a water cube and meshes as one.
    overlay.set(4, 4, 4, flowing(16));
    let lit = unlit(&chunk).with_overlay(&overlay);
    let mesh = mesh_chunk(lit, [None; 6], tables());
    assert_eq!(blended_heights(&mesh, [0.0, 1.0, 0.0]), vec![5.0]);
}

/// Between two flowing Voxels only the step between their surfaces shows, and a
/// solid floor hides the bottom.
#[test]
fn neighbouring_flow_shows_only_the_step_between_surfaces() {
    let mut chunk = Chunk::air();
    let mut overlay = tesserae_core::ChunkOverlay::none();
    for x in 4..=5 {
        chunk.set(x, 3, 4, Voxel::cube(1));
    }
    overlay.set(4, 4, 4, flowing(12));
    overlay.set(5, 4, 4, flowing(8));
    let lit = unlit(&chunk).with_overlay(&overlay);
    let mesh = mesh_chunk(lit, [None; 6], tables());
    // Facing +x out of the higher one: just 8/16 to 12/16 of the shared face,
    // then the lower one's full side further out.
    let layer = &mesh.blended;
    let mut plus_x: Vec<(f32, f32)> = (0..layer.positions.len())
        .filter(|&i| layer.normals[i] == [1.0, 0.0, 0.0])
        .map(|i| (layer.positions[i][0], layer.positions[i][1]))
        .collect();
    plus_x.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    plus_x.dedup();
    assert_eq!(
        plus_x,
        vec![(5.0, 4.5), (5.0, 4.75), (6.0, 4.0), (6.0, 4.5)]
    );
    assert!(
        blended_heights(&mesh, [0.0, -1.0, 0.0]).is_empty(),
        "the stone floor hides the bottom"
    );
}

/// Full water under a lower surface of the same water is one body: the face
/// between them is inside it and not drawn, from either side, and the surface
/// on top is.
#[test]
fn full_water_under_a_lower_surface_draws_no_face_between_them() {
    let chunk = Chunk::air();
    let mut overlay = tesserae_core::ChunkOverlay::none();
    overlay.set(4, 4, 4, flowing(16));
    overlay.set(4, 5, 4, flowing(6));
    let lit = unlit(&chunk).with_overlay(&overlay);
    let mesh = mesh_chunk(lit, [None; 6], tables());
    assert_eq!(blended_heights(&mesh, [0.0, 1.0, 0.0]), vec![5.375]);
    assert_eq!(blended_heights(&mesh, [0.0, -1.0, 0.0]), vec![4.0]);

    // Against a different fluid the face stays.
    let mut glass = Chunk::air();
    glass.set(4, 4, 4, Voxel::cube(tesserae_core::material::GLASS));
    let lit = unlit(&glass).with_overlay(&overlay);
    let mesh = mesh_chunk(lit, [None; 6], tables());
    assert!(
        mesh.blended
            .positions
            .chunks(4)
            .zip(mesh.blended.normals.chunks(4))
            .any(|(q, n)| n[0] == [0.0, 1.0, 0.0] && q[0][1] == 5.0),
        "glass under water keeps its top"
    );
}

/// A fence beside full flowing water grows no arm towards it: the water's
/// packed Voxel is air, which is what collision and the simulation read its
/// arms from, though the water is drawn as a cube. A packed source is a cube
/// to all three, and the arm grows.
#[test]
fn a_fence_grows_no_arm_towards_flowing_water() {
    let opaque = |chunk: &Chunk, overlay: &tesserae_core::ChunkOverlay| {
        mesh_chunk(unlit(chunk).with_overlay(overlay), [None; 6], tables())
            .opaque
            .positions
    };
    let fence = place_fences(&[(10, 10, 10)]);
    let bare = opaque(&fence, &tesserae_core::ChunkOverlay::none());
    let mut overlay = tesserae_core::ChunkOverlay::none();
    overlay.set(11, 10, 10, flowing(16));
    assert_eq!(opaque(&fence, &overlay), bare, "an arm towards flow");

    let mut sourced = fence.clone();
    sourced.set(11, 10, 10, Voxel::cube(tesserae_core::material::WATER));
    assert_ne!(
        opaque(&sourced, &tesserae_core::ChunkOverlay::none()),
        bare,
        "no arm towards a source"
    );
}

/// A dry overlay changes nothing about a mesh.
#[test]
fn a_dry_overlay_meshes_exactly_as_none() {
    let chunk = testgen::terrain_chunk(1, 0, 0);
    let overlay = tesserae_core::ChunkOverlay::none();
    let a = mesh_chunk(unlit(&chunk), [None; 6], tables());
    let b = mesh_chunk(unlit(&chunk).with_overlay(&overlay), [None; 6], tables());
    assert_eq!(a.blended.positions, b.blended.positions);
    assert_eq!(a.opaque.positions, b.opaque.positions);
    assert_eq!(a.quad_count(), b.quad_count());
}

fn soaked(fluid: tesserae_core::Fluid) -> tesserae_core::Overlay {
    tesserae_core::Overlay {
        fluid: Some(fluid),
        burning: None,
    }
}

fn blended_positions(mesh: &tesserae_core::ChunkMesh) -> Vec<[f32; 3]> {
    mesh.blended.positions.clone()
}

/// Water in a stair is drawn in the stair's notch and nowhere else, and the
/// stair itself meshes exactly as it does dry.
#[test]
fn a_waterlogged_stair_draws_its_water_in_the_notch_alone() {
    let stair = Voxel::shaped(1, SHAPE_STAIR, Orientation::new(POS_Y, 0));
    let mut chunk = Chunk::air();
    chunk.set(4, 4, 4, stair);
    let mut overlay = tesserae_core::ChunkOverlay::none();
    overlay.set(
        4,
        4,
        4,
        soaked(tesserae_core::Fluid::source(tesserae_core::material::WATER)),
    );
    let dry = mesh_chunk(unlit(&chunk), [None; 6], tables());
    let wet = mesh_chunk(unlit(&chunk).with_overlay(&overlay), [None; 6], tables());
    assert_eq!(wet.opaque.positions, dry.opaque.positions);
    assert_eq!(wet.shaped_voxels, dry.shaped_voxels);
    // The notch is the upper front quarter: top, front and its two sides.
    assert_eq!(wet.blended.quad_count(), 4);
    for p in blended_positions(&wet) {
        assert!(
            (4.5..=5.0).contains(&p[1]) && (4.0..=4.5).contains(&p[2]),
            "{p:?}"
        );
    }
}

/// Fluid below a full surface in a slab's void stands from the slab's top to
/// its own surface.
#[test]
fn a_waterlogged_slab_draws_water_from_its_top_to_the_surface() {
    let slab = Voxel::shaped(1, SHAPE_SLAB, Orientation::new(POS_Y, 0));
    let mut chunk = Chunk::air();
    chunk.set(4, 4, 4, slab);
    let mut overlay = tesserae_core::ChunkOverlay::none();
    let level = tesserae_core::Level {
        surface: 12,
        reach: 2,
        descent: 0,
    };
    let none = tesserae_core::Level::DRY;
    let fluid =
        tesserae_core::Fluid::flowing(tesserae_core::material::WATER, [level, none, none, none])
            .unwrap();
    overlay.set(4, 4, 4, soaked(fluid));
    let mesh = mesh_chunk(unlit(&chunk).with_overlay(&overlay), [None; 6], tables());
    assert_eq!(blended_heights(&mesh, [0.0, 1.0, 0.0]), vec![4.75]);
    assert_eq!(blended_heights(&mesh, [1.0, 0.0, 0.0]), vec![4.5, 4.75]);
    assert!(blended_heights(&mesh, [0.0, -1.0, 0.0]).is_empty());
}

/// A waterlogged stair inside a lake shows no water face between the two:
/// the lake's faces against it are covered by its solid and its full void,
/// and its void's faces by the lake. Only the lake's outside is drawn.
#[test]
fn a_waterlogged_stair_in_a_lake_draws_no_faces_inside_the_water() {
    let water = tesserae_core::material::WATER;
    let stair = Voxel::shaped(1, SHAPE_STAIR, Orientation::new(POS_Y, 0));
    let mut chunk = Chunk::air();
    for x in 3..6 {
        for y in 3..6 {
            for z in 3..6 {
                chunk.set(x, y, z, Voxel::cube(water));
            }
        }
    }
    chunk.set(4, 4, 4, stair);
    let mut overlay = tesserae_core::ChunkOverlay::none();
    overlay.set(4, 4, 4, soaked(tesserae_core::Fluid::source(water)));
    let mesh = mesh_chunk(unlit(&chunk).with_overlay(&overlay), [None; 6], tables());
    for p in blended_positions(&mesh) {
        assert!(p.iter().any(|&c| c == 3.0 || c == 6.0), "{p:?}");
    }
    assert_eq!(mesh.blended.quad_count(), 6, "the lake's six sides");
}

/// A burning cube draws a flame over each face open to air, a sixteenth
/// outside it, in the cutout layer with the flame index; a face against a
/// stone neighbour gets none. Its own geometry is the unburnt cube's, so
/// collision and selection, which read the Voxel, are untouched.
#[test]
fn a_burning_solid_draws_flames_over_its_open_faces_and_keeps_its_geometry() {
    use tesserae_core::mesh::FLAME;
    use tesserae_core::{Burning, ChunkOverlay, Overlay};
    let mut chunk = Chunk::air();
    chunk.set(16, 16, 16, Voxel::cube(1));
    chunk.set(17, 16, 16, Voxel::cube(1));
    let mut overlay = ChunkOverlay::none();
    overlay.set(
        16,
        16,
        16,
        Overlay {
            fluid: None,
            burning: Some(Burning { fuel: 9 }),
        },
    );
    let dry = mesh_isolated(&chunk);
    let lit = unlit(&chunk).with_overlay(&overlay);
    let burning = greedy_mesh(&Neighborhood::isolated(lit), tables());
    assert_eq!(burning.opaque.positions, dry.opaque.positions);
    assert_eq!(burning.opaque.materials, dry.opaque.materials);
    assert!(dry.cutout.is_empty());
    assert_eq!(
        burning.cutout.quad_count(),
        5,
        "every face but the shared one"
    );
    assert!(burning.cutout.materials.iter().all(|&m| m == FLAME));
    for p in &burning.cutout.positions {
        let outside = (0..3).any(|a| p[a] < 16.0 || p[a] > 17.0);
        assert!(outside, "a flame vertex inside its Voxel: {p:?}");
    }
    assert!(
        !burning.cutout.normals.contains(&[1.0, 0.0, 0.0]),
        "a flame on the face against stone"
    );
}
