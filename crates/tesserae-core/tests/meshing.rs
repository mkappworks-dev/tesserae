use std::sync::OnceLock;
use tesserae_core::light::{Channel, Light, MAX_LIGHT, transmitted};
use tesserae_core::mesh::{NEG_X, NEG_Y, NEG_Z, POS_X, POS_Y, POS_Z};
use tesserae_core::shape::{NodeBox, examples};
use tesserae_core::{
    CHUNK_SIZE, Chunk, ChunkLight, ConnectDef, Connections, LitChunk, Neighborhood, Voxel,
    greedy_mesh, mesh_chunk, testgen,
};
use tesserae_core::{
    Orientation, SHAPE_POST, SHAPE_SLAB, SHAPE_STAIR, ShapeTable, Worldgen, WorldgenParams,
};

const FENCE: u8 = 4;
const PIPE: u8 = 5;

/// The builtin Shapes, then a fence and a six-way pipe registered as a Mod would.
fn shapes() -> &'static ShapeTable {
    static TABLE: OnceLock<ShapeTable> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut t = ShapeTable::builtin();
        assert_eq!(t.register(examples::fence()), Ok(FENCE));
        assert_eq!(t.register(examples::pipe()), Ok(PIPE));
        t
    })
}

/// The light the tests that are about *geometry* use. Lent from a `static`
/// because `LitChunk` borrows it; the light-specific tests below build their
/// own.
static DARK: ChunkLight = ChunkLight::dark();

fn unlit(chunk: &Chunk) -> LitChunk<'_> {
    LitChunk::new(chunk, &DARK)
}

fn mesh_isolated(chunk: &Chunk) -> tesserae_core::ChunkMesh {
    greedy_mesh(&Neighborhood::isolated(unlit(chunk)), shapes())
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
    let open = greedy_mesh(&Neighborhood::isolated(unlit(&solid)), shapes());
    let closed = greedy_mesh(
        &Neighborhood::new(
            unlit(&solid),
            [None, Some(unlit(&solid)), None, None, None, None],
        ),
        shapes(),
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

    let open = greedy_mesh(&Neighborhood::isolated(unlit(&center)), shapes());
    let closed = greedy_mesh(
        &Neighborhood::new(
            unlit(&center),
            [Some(unlit(&left)), None, None, None, None, None],
        ),
        shapes(),
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
    assert!(mesh_chunk(unlit(&air), enclosed_by(&solid), shapes()).is_empty());
    assert!(mesh_chunk(unlit(&air), [None; 6], shapes()).is_empty());
}

#[test]
fn a_fully_enclosed_solid_chunk_declines() {
    let solid = Chunk::filled(Voxel::cube(1));
    assert!(mesh_chunk(unlit(&solid), enclosed_by(&solid), shapes()).is_empty());
}

#[test]
fn an_unknown_neighbour_is_not_an_occluder() {
    let solid = Chunk::filled(Voxel::cube(1));
    let mut neighbors = enclosed_by(&solid);
    neighbors[0] = None;
    // Unknown, not air: the face has to be emitted or the Chunk is open to a
    // hole once that neighbour streams in.
    assert_eq!(
        mesh_chunk(unlit(&solid), neighbors, shapes()).quad_count(),
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
        mesh_chunk(unlit(&solid), neighbors, shapes()).quad_count(),
        1
    );
}

#[test]
fn a_shaped_uniform_chunk_is_not_an_occluder() {
    // A stair does not seal every face, so a Chunk of them neither satisfies the
    // enclosed case nor is hidden by it: each tread is inside its own cell.
    let stairs = Chunk::filled(shaped(SHAPE_STAIR, POS_Y, 0));
    let solid = Chunk::filled(Voxel::cube(1));
    assert!(mesh_chunk(unlit(&stairs), enclosed_by(&solid), shapes()).quad_count() > 0);
    assert!(mesh_chunk(unlit(&solid), enclosed_by(&stairs), shapes()).quad_count() > 0);
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

    let fast = mesh_chunk(unlit(&center), refs, shapes());
    let slow = greedy_mesh(&Neighborhood::new(unlit(&center), refs), shapes());
    assert_eq!(fast.positions, slow.positions);
    assert_eq!(fast.normals, slow.normals);
    assert_eq!(fast.uvs, slow.uvs);
    assert_eq!(fast.materials, slow.materials);
    assert_eq!(fast.light, slow.light);
    assert_eq!(fast.indices, slow.indices);
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
        shapes(),
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
        shapes(),
    );
    let shredded = greedy_mesh(
        &Neighborhood::isolated(LitChunk::new(&chunk, &split)),
        shapes(),
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
        shapes(),
    );
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
    let mesh = greedy_mesh(
        &Neighborhood::new(LitChunk::new(&center, &center_light), neighbors),
        shapes(),
    );

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
    assert!(mesh_chunk(LitChunk::new(&air, &lumpy), neighbors, shapes()).is_empty());
    assert!(mesh_chunk(LitChunk::new(&solid, &lumpy), neighbors, shapes()).is_empty());
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
        shapes(),
    );
    let seen: std::collections::BTreeSet<u8> = (0..mesh.positions.len())
        .filter(|&i| mesh.normals[i] == [0.0, 1.0, 0.0])
        .map(|i| (mesh.light[i] & 0x0F) as u8)
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
    let mut solid = std::collections::HashSet::new();
    for z in LO..LO + SPAN {
        for y in LO..LO + SPAN {
            for x in LO..LO + SPAN {
                for b in declared(chunk, [x as i32, y as i32, z as i32], true) {
                    for sz in b.min[2]..b.max[2] {
                        for sy in b.min[1]..b.max[1] {
                            for sx in b.min[0]..b.max[0] {
                                solid.insert([
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
    let mut faces = std::collections::BTreeSet::new();
    for s in &solid {
        for (f, d) in DIRS.iter().enumerate() {
            let n = [s[0] + d[0], s[1] + d[1], s[2] + d[2]];
            if !solid.contains(&n) {
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
    for (q, normal) in mesh.positions.chunks(4).zip(mesh.normals.chunks(4)) {
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
    // And the rails are there: four posts alone would be 4 * 6 quads.
    assert!(mesh_isolated(&chunk).quad_count() > 4 * 6);
}

#[test]
fn a_fence_grows_an_arm_towards_a_cube_and_the_cube_hides_its_end() {
    let mut chunk = place_fences(&[(10, 10, 10)]);
    let alone = mesh_isolated(&chunk).quad_count();
    chunk.set(11, 10, 10, Voxel::cube(1));
    assert_hole_free(&chunk, "fence beside cube");
    // The cube's own face under the rails is drawn, because pass 1 culls only
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
    assert_eq!(hidden.len(), 12, "two rails of 2 x 3 sub-cells");
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
        let m = mesh_chunk(unlit(&a), neighbors, shapes());
        let cap_on_the_boundary = m
            .positions
            .chunks(4)
            .zip(m.normals.chunks(4))
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
    let mesh = greedy_mesh(&Neighborhood::isolated(LitChunk::new(&c, &light)), shapes());

    let lit_at = |normal: [f32; 3], axis: usize, at: f32| -> Vec<u32> {
        (0..mesh.positions.len())
            .filter(|&i| mesh.normals[i] == normal && mesh.positions[i][axis] == at)
            .map(|i| mesh.light[i])
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
        assert!(dot > 0.0, "winding flipped: {cross:?} vs normal {n:?}");
    }
    for quad in mesh.positions.chunks(4) {
        let axes = (0..3)
            .filter(|&ax| quad.iter().all(|p| p[ax] == quad[0][ax]))
            .count();
        assert_eq!(axes, 1, "quad {quad:?} is not on one axis-aligned plane");
    }
    let n = mesh.positions.len();
    assert_eq!(mesh.normals.len(), n);
    assert_eq!(mesh.uvs.len(), n);
    assert_eq!(mesh.materials.len(), n);
    assert_eq!(mesh.light.len(), n);
    assert!(mesh.indices.iter().all(|&i| (i as usize) < n));
}

/// A half-height slab side shows half the texture, not a squashed whole one.
#[test]
fn uvs_are_the_position_inside_the_voxel() {
    let mut c = Chunk::air();
    c.set(10, 10, 10, shaped(SHAPE_SLAB, POS_Y, 0));
    let mesh = mesh_isolated(&c);
    for i in 0..mesh.positions.len() {
        if mesh.normals[i] == [1.0, 0.0, 0.0] {
            // A +X face is on axes (y, z), so (u, v) = (y, z) inside the cell.
            let p = mesh.positions[i];
            assert_eq!(mesh.uvs[i], [p[1] - 10.0, p[2] - 10.0]);
            assert!(mesh.uvs[i][0] <= 0.5, "the slab's side is half height");
        }
    }
}
