//! `cross` and `plane` Shapes: how they load, mesh, light and collide.

use tesserae_core::shape::{NodeBox, ShapeError, examples};
use tesserae_core::testgen::fixtures;
use tesserae_core::{
    CHUNK_SIZE, Chunk, ChunkLight, Connections, Form, LitChunk, MaterialDef, Neighborhood,
    Occlusion, Orientation, RenderClass, ShapeDef, ShapeId, Tables, Voxel, greedy_mesh,
};

static DARK: ChunkLight = ChunkLight::dark();

struct Flats {
    tables: Tables,
    cross: ShapeId,
    plane: ShapeId,
    grass: u16,
}

fn flats() -> Flats {
    let mut tables = fixtures().tables.clone();
    let grass = tables
        .materials
        .register(MaterialDef {
            emission: 0,
            dimming: 0,
            occlusion: Occlusion::Never,
            render: RenderClass::Cutout,
            solid: true,
        })
        .unwrap();
    let mut flat = |form, boxes| {
        tables
            .shapes
            .register(ShapeDef {
                form,
                connect: None,
                boxes,
                collision: None,
            })
            .unwrap()
    };
    let cross = flat(Form::Cross, vec![NodeBox::new([4, 0, 4], [12, 16, 12])]);
    let plane = flat(Form::Plane, vec![NodeBox::new([1, 1, 0], [15, 15, 2])]);
    Flats {
        tables,
        cross,
        plane,
        grass,
    }
}

fn mesh(f: &Flats, chunk: &Chunk) -> tesserae_core::ChunkMesh {
    greedy_mesh(
        &Neighborhood::isolated(LitChunk::new(chunk, &DARK)),
        &f.tables,
    )
}

fn put(chunk: &mut Chunk, at: [usize; 3], voxel: Voxel) {
    chunk.set(at[0], at[1], at[2], voxel);
}

#[test]
fn a_flat_shape_that_seals_a_face_or_connects_is_a_load_error() {
    let mut shapes = fixtures().tables.shapes.clone();
    let sealing = |form| ShapeDef {
        form,
        connect: None,
        boxes: vec![NodeBox::new([0; 3], [16; 3])],
        collision: None,
    };
    for form in [Form::Cross, Form::Plane] {
        assert_eq!(shapes.register(sealing(form)), Err(ShapeError::FormSeals));
    }
    let connected = ShapeDef {
        form: Form::Cross,
        ..examples::fence()
    };
    assert_eq!(shapes.register(connected), Err(ShapeError::FormConnected));
}

#[test]
fn a_flat_voxel_is_walked_through_but_selectable_and_costs_nothing_against_the_cap() {
    let f = flats();
    let shapes = &f.tables.shapes;
    for shape in [f.cross, f.plane] {
        let v = Voxel::shaped(f.grass, shape, Orientation::default());
        assert!(shapes.collision(v, Connections::NONE).is_empty());
        assert!(!shapes.selection(v, Connections::NONE).is_empty());
        assert_eq!(shapes.cap_units(v), 0);
    }
    let slab = Voxel::shaped(f.grass, tesserae_core::SHAPE_SLAB, Orientation::default());
    assert_eq!(shapes.cap_units(slab), 1);
}

#[test]
fn a_cross_draws_two_double_sided_quads_and_hides_no_neighbour_face() {
    let f = flats();
    let rock = fixtures().ground.rock;
    let cross = Voxel::shaped(f.grass, f.cross, Orientation::default());
    let mut alone = Chunk::air();
    put(&mut alone, [10, 10, 10], cross);
    let drawn = mesh(&f, &alone).quad_count();
    assert_eq!(drawn, 4);

    let mut beside = Chunk::air();
    put(&mut beside, [10, 10, 10], cross);
    put(&mut beside, [11, 10, 10], Voxel::cube(rock));
    assert_eq!(mesh(&f, &beside).quad_count(), drawn + 6);

    let mut pair = Chunk::air();
    put(&mut pair, [10, 10, 10], cross);
    put(&mut pair, [11, 10, 10], cross);
    assert_eq!(mesh(&f, &pair).quad_count(), 2 * drawn, "nothing merges");
}

#[test]
fn a_cross_spans_exactly_one_voxel_whatever_the_chunk_holds() {
    let f = flats();
    let mut c = Chunk::air();
    put(
        &mut c,
        [3, 4, 5],
        Voxel::shaped(f.grass, f.cross, Orientation::default()),
    );
    let cutout = mesh(&f, &c).cutout.soup();
    let lo = cutout.positions.iter().fold([f32::MAX; 3], |a, p| {
        [a[0].min(p[0]), a[1].min(p[1]), a[2].min(p[2])]
    });
    let hi = cutout.positions.iter().fold([f32::MIN; 3], |a, p| {
        [a[0].max(p[0]), a[1].max(p[1]), a[2].max(p[2])]
    });
    assert_eq!(lo, [3.0, 4.0, 5.0]);
    assert_eq!(hi, [4.0, 5.0, 6.0]);
}

#[test]
fn a_plane_draws_one_double_sided_quad_in_every_orientation() {
    let f = flats();
    let shapes = &f.tables.shapes;
    let mut faces = std::collections::HashSet::new();
    for orientation in Orientation::all() {
        let v = Voxel::shaped(f.grass, f.plane, orientation);
        let mut c = Chunk::air();
        put(&mut c, [CHUNK_SIZE / 2; 3], v);
        assert_eq!(mesh(&f, &c).quad_count(), 2, "{orientation:?}");
        faces.insert(shapes.plane_face(v).expect("the plane lies against a face"));
    }
    assert_eq!(faces.len(), 6, "every face is reachable");
}

fn assert_upright(m: &tesserae_core::ChunkMesh, y0: f32, y1: f32) {
    let l = &m.cutout.soup();
    for (i, p) in l.positions.iter().enumerate() {
        assert_eq!(
            l.uvs[i][1],
            if p[1] == y0 { 0.0 } else { 1.0 },
            "v is 0 at the foot"
        );
        assert!(p[1] == y0 || p[1] == y1);
    }
}

#[test]
fn a_cross_faces_sideways_with_its_sprite_upright() {
    let f = flats();
    let mut c = Chunk::air();
    put(
        &mut c,
        [3, 4, 5],
        Voxel::shaped(f.grass, f.cross, Orientation::default()),
    );
    let m = mesh(&f, &c);
    assert_upright(&m, 4.0, 5.0);
    let n = &m.cutout.soup().normals;
    assert_eq!(n.len(), 16);
    assert!(
        n.iter()
            .all(|n| n[1] == 0.0 && n.iter().map(|c| c * c).sum::<f32>() == 1.0)
    );
    assert!(n.contains(&[0.0, 0.0, 1.0]) && n.contains(&[0.0, 0.0, -1.0]));
}

#[test]
fn a_plane_faces_along_its_thin_axis_with_its_sprite_upright() {
    let f = flats();
    for orientation in Orientation::all() {
        let v = Voxel::shaped(f.grass, f.plane, orientation);
        let mut c = Chunk::air();
        put(&mut c, [3, 4, 5], v);
        let l = mesh(&f, &c).cutout.soup();
        let axis = l.normals[0].iter().position(|&c| c != 0.0).unwrap();
        for (i, n) in l.normals.iter().enumerate() {
            assert_eq!(
                n.iter().filter(|&&c| c != 0.0).count(),
                1,
                "{orientation:?}"
            );
            assert!(n[axis].abs() == 1.0);
            // Only wall faces have a vertical axis to check; a ceiling or floor Plane lies flat.
            if axis != 1 {
                let p = l.positions[i];
                assert_eq!(
                    l.uvs[i][usize::from(axis == 2)] == 0.0,
                    p[1] < 4.5,
                    "{orientation:?}"
                );
            }
        }
    }
}
