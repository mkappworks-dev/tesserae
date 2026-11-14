//! Columns, spreaders and climbers on the Farm rig.

use super::*;
use tesserae_core::plant::Direction;
use tesserae_core::shape::{Form, NodeBox, Orientation, ShapeDef};
use tesserae_core::{ShapeId, mesh};

const WALL_X: i32 = 20;

struct Wild {
    tables: Tables,
    plant: MaterialId,
    shape: ShapeId,
}

impl Wild {
    fn voxel(&self) -> Voxel {
        Voxel::new(self.plant, self.shape, 0)
    }
}

/// One Plant Material of `kind`, drawn with a flat Shape that leaves voids.
fn wild(form: Form, kind: Kind, needs: impl FnOnce(&mut Needs)) -> Wild {
    let mut tables = crate::server::fixture_tables();
    let plant = tables
        .materials
        .register(MaterialDef {
            emission: 0,
            dimming: 0,
            occlusion: Occlusion::Never,
            render: RenderClass::Cutout,
            solid: true,
        })
        .unwrap();
    let boxes = match form {
        Form::Plane => vec![NodeBox::new([1, 1, 0], [15, 15, 2])],
        _ => vec![NodeBox::new([6, 0, 6], [10, 16, 10])],
    };
    let shape = tables
        .shapes
        .register(ShapeDef {
            form,
            boxes,
            ..ShapeDef::default()
        })
        .unwrap();
    let mut n = Needs {
        soil: vec![soil()],
        light_max: 15,
        ..Needs::default()
    };
    needs(&mut n);
    tables
        .plants
        .push(Plant {
            name: "wild".into(),
            seed: "t:seed".into(),
            first: plant,
            shape: None,
            needs: n,
            kind,
            sapling_yields: Vec::new(),
        })
        .unwrap();
    Wild {
        tables,
        plant,
        shape,
    }
}

fn column(direction: Direction, stands_in: Option<MaterialId>, height_m: f32) -> Kind {
    Kind::Column {
        max_height_m: height_m,
        direction,
        mean_seconds: tesserae_core::plant::SAMPLE_INTERVAL_S,
        stands_in: stands_in.into_iter().collect(),
        yields: Vec::new(),
    }
}

fn spreader(reach_m: f32, density: u32) -> Kind {
    Kind::Spreader {
        reach_m,
        mean_seconds: tesserae_core::plant::SAMPLE_INTERVAL_S,
        density,
        yields: Vec::new(),
    }
}

fn ground() -> Chunk {
    let mut chunk = Chunk::air();
    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            chunk.set(x, SOIL_Y, z, Voxel::cube(soil()));
        }
    }
    chunk
}

fn farm(size: f32, wild: &Wild, chunk: Chunk) -> Farm {
    Farm::open(size, wild.tables.clone(), [wild.plant; 3], chunk)
}

fn material_at(farm: &Farm, at: [i32; 3]) -> MaterialId {
    let v = VoxelCoord::of(at);
    farm.store()
        .get(v.chunk)
        .unwrap()
        .get(v.x, v.y, v.z)
        .material()
}

/// Voxels of `plant` stacked above `base`.
fn run_up(farm: &Farm, base: [i32; 3], plant: MaterialId) -> i32 {
    (0..CHUNK_SIZE as i32)
        .take_while(|dy| material_at(farm, [base[0], base[1] + dy, base[2]]) == plant)
        .count() as i32
}

fn run_until(farm: &mut Farm, limit: u64, done: impl Fn(&Farm) -> bool) {
    let start = farm.sim().now();
    while farm.sim().now() < start + limit && !done(farm) {
        farm.run(100);
    }
}

#[test]
fn a_column_reaches_its_height_in_metres_at_every_voxel_size_and_stops() {
    for size in SIZES {
        let wild = wild(Form::Cross, column(Direction::Up, None, 2.0), |_| {});
        let mut chunk = ground();
        let bases: Vec<[i32; 3]> = (0..8).map(|i| [2 + 3 * i, CROP_Y, 2]).collect();
        for b in &bases {
            chunk.set(b[0] as usize, b[1] as usize, b[2] as usize, wild.voxel());
        }
        let mut farm = farm(size, &wild, chunk);
        let cap = (2.0 / size) as i32;
        let tallest = |f: &Farm| {
            bases
                .iter()
                .map(|&b| run_up(f, b, wild.plant))
                .max()
                .unwrap()
        };
        run_until(&mut farm, 60_000, |f| tallest(f) >= cap);
        assert_eq!(tallest(&farm), cap, "{size}: not 2 m tall");
        farm.run(4000);
        for &b in &bases {
            let run = run_up(&farm, b, wild.plant);
            assert!(run <= cap, "{size}: a column of {run} passed its cap {cap}");
        }
        assert_eq!(tallest(&farm), cap, "{size}: a capped column died");
    }
}

#[test]
fn cane_stops_at_a_roof_without_dying_and_grows_in_air_not_water() {
    let size = 1.0;
    let wild = wild(Form::Cross, column(Direction::Up, None, 8.0), |_| {});
    let mut chunk = ground();
    let water = tesserae_core::testgen::fixtures().water;
    for i in 0..8 {
        let x = 2 + 3 * i;
        chunk.set(x, CROP_Y as usize, 2, wild.voxel());
        chunk.set(x, CROP_Y as usize + 2, 2, Voxel::cube(rock()));
        chunk.set(x, CROP_Y as usize, 10, wild.voxel());
    }
    let mut farm = farm(size, &wild, chunk);
    for i in 0..8 {
        let at = [2 + 3 * i, CROP_Y + 1, 10];
        farm.set(
            at,
            Occupants {
                solid: Voxel::AIR,
                fluid: Some(Fluid::source(water)),
                burning: None,
            },
        );
    }
    farm.run(30_000);
    let grew = (0..8)
        .filter(|i| run_up(&farm, [2 + 3 * i, CROP_Y, 2], wild.plant) == 2)
        .count();
    assert!(grew > 0, "no cane grew up to its roof");
    for i in 0..8 {
        let x = 2 + 3 * i;
        assert_eq!(material_at(&farm, [x, CROP_Y + 2, 2]), rock(), "roof kept");
        assert_eq!(material_at(&farm, [x, CROP_Y, 2]), wild.plant, "base kept");
        assert_eq!(
            run_up(&farm, [x, CROP_Y, 10], wild.plant),
            1,
            "cane grew into water"
        );
    }
}

/// A walled pool of `depth` source cells over the floor, with the kelp's base
/// waterlogged in it.
fn pool(wild: &Wild, depth: i32, bases: &[[i32; 3]]) -> Farm {
    let mut chunk = ground();
    let water = tesserae_core::testgen::fixtures().water;
    for y in 0..14usize {
        for i in 7..=18usize {
            for edge in [7usize, 18] {
                chunk.set(edge, CROP_Y as usize + y, i, Voxel::cube(rock()));
                chunk.set(i, CROP_Y as usize + y, edge, Voxel::cube(rock()));
            }
        }
    }
    let mut farm = farm_in(wild, chunk);
    for y in 0..depth {
        for z in 8..18 {
            for x in 8..18 {
                farm.set(
                    [x, CROP_Y + y, z],
                    Occupants {
                        solid: Voxel::AIR,
                        fluid: Some(Fluid::source(water)),
                        burning: None,
                    },
                );
            }
        }
    }
    for &b in bases {
        farm.set(
            b,
            Occupants {
                solid: wild.voxel(),
                fluid: Some(Fluid::source(water)),
                burning: None,
            },
        );
    }
    farm
}

fn farm_in(wild: &Wild, chunk: Chunk) -> Farm {
    farm(1.0, wild, chunk)
}

#[test]
fn kelp_grows_through_its_water_waterlogged_and_stops_at_the_surface() {
    let water = tesserae_core::testgen::fixtures().water;
    let wild = wild(Form::Cross, column(Direction::Up, Some(water), 30.0), |n| {
        n.light_max = 15
    });
    let bases: Vec<[i32; 3]> = (0..6).map(|i| [9 + i, CROP_Y, 12]).collect();
    let depth = 4;
    let mut farm = pool(&wild, depth, &bases);
    run_until(&mut farm, 40_000, |f| {
        bases.iter().any(|&b| run_up(f, b, wild.plant) == depth)
    });
    let tallest = bases
        .iter()
        .map(|&b| run_up(&farm, b, wild.plant))
        .max()
        .unwrap();
    assert_eq!(tallest, depth, "kelp did not climb the whole pool");
    farm.run(3000);
    for &b in &bases {
        let run = run_up(&farm, b, wild.plant);
        assert!(run <= depth, "kelp left the water: {run}");
        for dy in 0..run {
            let at = [b[0], b[1] + dy, b[2]];
            let here = farm
                .store()
                .occupants(at, &farm.app.world().resource::<ContentTables>().0.fluids)
                .unwrap();
            assert_eq!(here.solid.shape(), wild.shape);
            assert_eq!(
                here.fluid.map(|f| f.material),
                Some(water),
                "kelp at {at:?} is not waterlogged"
            );
        }
        assert_eq!(material_at(&farm, [b[0], b[1] + depth, b[2]]), 0);
    }
}

#[test]
fn kelp_in_air_does_not_grow() {
    let water = tesserae_core::testgen::fixtures().water;
    let wild = wild(
        Form::Cross,
        column(Direction::Up, Some(water), 30.0),
        |_| {},
    );
    let mut chunk = ground();
    chunk.set(9, CROP_Y as usize, 9, wild.voxel());
    let mut farm = farm_in(&wild, chunk);
    farm.run(30_000);
    assert_eq!(run_up(&farm, [9, CROP_Y, 9], wild.plant), 1);
}

#[test]
fn cutting_a_columns_middle_breaks_the_part_above_and_the_base_regrows() {
    let wild = wild(Form::Cross, column(Direction::Up, None, 8.0), |_| {});
    let mut chunk = ground();
    for dy in 0..5 {
        chunk.set(5, CROP_Y as usize + dy, 5, wild.voxel());
    }
    let mut farm = farm(1.0, &wild, chunk);
    farm.solid([5, CROP_Y + 2, 5], Voxel::AIR);
    farm.run(40);
    let broken = farm.sim_mut().take_broken();
    let mut cells: Vec<_> = broken.iter().map(|b| b.at).collect();
    cells.sort();
    assert_eq!(cells, vec![[5, CROP_Y + 3, 5], [5, CROP_Y + 4, 5]]);
    assert!(broken.iter().all(|b| b.material == wild.plant));
    assert_eq!(run_up(&farm, [5, CROP_Y, 5], wild.plant), 2, "base kept");
    run_until(&mut farm, 60_000, |f| {
        run_up(f, [5, CROP_Y, 5], wild.plant) >= 4
    });
    assert!(
        run_up(&farm, [5, CROP_Y, 5], wild.plant) >= 3,
        "the base did not regrow"
    );
}

#[test]
fn a_downward_column_grows_down_from_its_anchor() {
    let wild = wild(Form::Cross, column(Direction::Down, None, 3.0), |n| {
        n.soil = vec![rock()];
    });
    let mut chunk = ground();
    for x in 0..8usize {
        chunk.set(x * 3, 12, 5, Voxel::cube(rock()));
        chunk.set(x * 3, 11, 5, wild.voxel());
    }
    let mut farm = farm(1.0, &wild, chunk);
    run_until(&mut farm, 60_000, |f| {
        (0..8).any(|x| material_at(f, [x * 3, 9, 5]) == wild.plant)
    });
    let hanging = (0..8)
        .filter(|x| material_at(&farm, [x * 3, 9, 5]) == wild.plant)
        .count();
    assert!(hanging > 0, "no column hung 3 m");
    for x in 0..8 {
        assert_ne!(
            material_at(&farm, [x * 3, 8, 5]),
            wild.plant,
            "past its cap"
        );
    }
}

fn tufts(farm: &Farm, plant: MaterialId, y: i32) -> Vec<[i32; 3]> {
    let mut found = Vec::new();
    for z in 0..CHUNK_SIZE as i32 {
        for x in -(CHUNK_SIZE as i32)..2 * CHUNK_SIZE as i32 {
            if farm
                .store()
                .voxel([x, y, z])
                .is_some_and(|v| v.material() == plant)
            {
                found.push([x, y, z]);
            }
        }
    }
    found
}

#[test]
fn a_tuft_spreads_onto_soil_within_its_reach_and_never_elsewhere() {
    for size in SIZES {
        let reach = 2.0;
        let wild = wild(Form::Cross, spreader(reach, 1000), |n| n.light_min = 0);
        let mut chunk = ground();
        let rock_from = 16usize;
        for z in 0..CHUNK_SIZE {
            for x in rock_from..CHUNK_SIZE {
                chunk.set(x, SOIL_Y, z, Voxel::cube(rock()));
            }
        }
        chunk.set(12, CROP_Y as usize, 16, Voxel::cube(rock()));
        let start = [13, CROP_Y, 16];
        chunk.set(13, CROP_Y as usize, 16, wild.voxel());
        let mut farm = farm(size, &wild, chunk);
        let mut known = vec![start];
        for _ in 0..40_000u64 / 20 {
            farm.run(20);
            for t in tufts(&farm, wild.plant, CROP_Y) {
                if known.contains(&t) {
                    continue;
                }
                let near = known
                    .iter()
                    .map(|k| {
                        let d: [f32; 3] = std::array::from_fn(|i| (k[i] - t[i]) as f32 * size);
                        (d[0] * d[0] + d[2] * d[2]).sqrt()
                    })
                    .fold(f32::MAX, f32::min);
                assert!(near <= reach + 1e-3, "{size}: {t:?} spread {near} m");
                known.push(t);
            }
            if known.len() >= 6 {
                break;
            }
        }
        assert!(known.len() >= 3, "{size}: a tuft never spread");
        let tables = Arc::clone(&farm.app.world().resource::<ContentTables>().0);
        for t in &known[1..] {
            assert!(t[0] < rock_from as i32, "{size}: on rock at {t:?}");
            assert_ne!(*t, [12, CROP_Y, 16], "{size}: over a solid");
            let below = farm
                .store()
                .occupants([t[0], t[1] - 1, t[2]], &tables.fluids);
            assert_eq!(below.unwrap().solid.material(), soil());
        }
        assert_eq!(material_at(&farm, [12, CROP_Y, 16]), rock());
    }
}

#[test]
fn a_tuft_never_spreads_into_water_or_under_it() {
    let water = tesserae_core::testgen::fixtures().water;
    let wild = wild(Form::Cross, spreader(3.0, 1000), |n| n.light_min = 0);
    let mut chunk = ground();
    chunk.set(8, CROP_Y as usize, 8, wild.voxel());
    for i in 3..=13usize {
        for edge in [9usize, 14] {
            chunk.set(edge, CROP_Y as usize, i, Voxel::cube(rock()));
        }
        if (9..=14).contains(&i) {
            for edge in [3usize, 13] {
                chunk.set(i, CROP_Y as usize, edge, Voxel::cube(rock()));
            }
        }
    }
    let mut farm = farm(1.0, &wild, chunk);
    for z in 4..13 {
        for x in 10..14 {
            farm.set(
                [x, CROP_Y, z],
                Occupants {
                    solid: Voxel::AIR,
                    fluid: Some(Fluid::source(water)),
                    burning: None,
                },
            );
            farm.set(
                [x, CROP_Y - 1, z],
                Occupants {
                    solid: Voxel::cube(soil()),
                    fluid: None,
                    burning: None,
                },
            );
        }
    }
    farm.run(30_000);
    let tables = Arc::clone(&farm.app.world().resource::<ContentTables>().0);
    for y in [CROP_Y, CROP_Y + 1] {
        for t in tufts(&farm, wild.plant, y) {
            assert!(
                !(10..14).contains(&t[0]) || !(4..13).contains(&t[2]),
                "{t:?} is in the pool"
            );
            assert!(
                farm.store()
                    .occupants([t[0], t[1] + 1, t[2]], &tables.fluids)
                    .is_none_or(|o| o.fluid.is_none()),
                "{t:?} is under a fluid"
            );
            let here = farm.store().occupants(t, &tables.fluids).unwrap();
            assert!(here.fluid.is_none());
        }
    }
    assert!(tufts(&farm, wild.plant, CROP_Y).len() > 1, "never spread");
}

#[test]
fn the_density_cap_keeps_a_field_from_filling_solid() {
    let reach = 2.0;
    let count = |cap| {
        let wild = wild(Form::Cross, spreader(reach, cap), |n| n.light_min = 0);
        let mut chunk = ground();
        chunk.set(16, CROP_Y as usize, 16, wild.voxel());
        let mut farm = farm(1.0, &wild, chunk);
        farm.run(60_000);
        tufts(&farm, wild.plant, CROP_Y).len()
    };
    let (capped, free) = (count(2), count(1000));
    assert!(capped >= 2);
    assert!(capped * 2 < free, "{capped} capped against {free} free");
}

/// A roof of rock over the whole floor, a metre above it.
fn roofed(chunk: &mut Chunk) {
    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            chunk.set(x, CROP_Y as usize + 3, z, Voxel::cube(rock()));
        }
    }
}

#[test]
fn mushrooms_spread_in_the_dark_and_not_in_light_and_grass_the_reverse() {
    let kinds = [("mushroom", 0u8, 4u8), ("grass", 8, 15)];
    for (name, min, max) in kinds {
        for dark in [true, false] {
            let wild = wild(Form::Cross, spreader(2.0, 1000), |n| {
                n.light_min = min;
                n.light_max = max;
            });
            let mut chunk = ground();
            if dark {
                roofed(&mut chunk);
            }
            chunk.set(16, CROP_Y as usize, 16, wild.voxel());
            let mut farm = farm(1.0, &wild, chunk);
            farm.run(20_000);
            let spread = tufts(&farm, wild.plant, CROP_Y).len() > 1;
            let wants_dark = name == "mushroom";
            assert_eq!(spread, dark == wants_dark, "{name} dark={dark}");
        }
    }
}

#[test]
fn a_bare_chunk_beside_a_meadow_grows_across_the_boundary_and_never_into_an_unheld_chunk() {
    let wild = wild(Form::Cross, spreader(3.0, 1000), |n| n.light_min = 0);
    let mut meadow = ground();
    for z in 0..CHUNK_SIZE {
        for x in 0..4 {
            meadow.set(x, CROP_Y as usize, z, wild.voxel());
        }
        for x in CHUNK_SIZE - 4..CHUNK_SIZE {
            meadow.set(x, CROP_Y as usize, z, wild.voxel());
        }
    }
    let mut farm = farm(1.0, &wild, meadow);
    farm.hold(ChunkCoord::new(1, 0, 0), ground());
    farm.run(20_000);
    let east = farm.store().get(ChunkCoord::new(1, 0, 0)).unwrap();
    let across = (0..CHUNK_SIZE)
        .flat_map(|z| (0..CHUNK_SIZE).map(move |x| (x, z)))
        .filter(|&(x, z)| east.get(x, CROP_Y as usize, z).material() == wild.plant)
        .count();
    assert!(across > 0, "nothing crossed into the held Chunk");
    assert!(
        farm.store().get(ChunkCoord::new(-1, 0, 0)).is_none(),
        "an unheld Chunk appeared"
    );
    assert_eq!(farm.sim().plants.stale, 0, "a spread tried an unheld Chunk");
    let west = (0..CHUNK_SIZE)
        .filter(|&z| material_at(&farm, [4, CROP_Y, z as i32]) == wild.plant)
        .count();
    assert!(west > 0, "the meadow did not grow inside its own Chunk");
}

fn vines(length_m: f32) -> Wild {
    wild(
        Form::Plane,
        Kind::Climber {
            mean_seconds: tesserae_core::plant::SAMPLE_INTERVAL_S,
            length_m,
            yields: Vec::new(),
        },
        |n| n.light_min = 0,
    )
}

fn vine_cells(farm: &Farm, plant: MaterialId) -> Vec<[i32; 3]> {
    let mut found = Vec::new();
    for y in 0..CHUNK_SIZE as i32 {
        for z in 0..CHUNK_SIZE as i32 {
            for x in 0..CHUNK_SIZE as i32 {
                if material_at(farm, [x, y, z]) == plant {
                    found.push([x, y, z]);
                }
            }
        }
    }
    found
}

fn walled(wild: &Wild, face: usize) -> Chunk {
    let mut chunk = ground();
    for y in CROP_Y..CROP_Y + 14 {
        for z in 0..CHUNK_SIZE {
            for x in WALL_X as usize..CHUNK_SIZE {
                chunk.set(x, y as usize, z, Voxel::cube(soil()));
            }
        }
    }
    let o = Orientation::from_bits(
        wild.tables
            .shapes
            .orientation_holding(wild.plant, wild.shape, face)
            .unwrap()
            .bits(),
    )
    .unwrap();
    chunk.set(
        WALL_X as usize - 1,
        CROP_Y as usize + 6,
        16,
        Voxel::shaped(wild.plant, wild.shape, o),
    );
    chunk
}

#[test]
fn a_vine_spreads_along_a_wall_to_its_length_in_metres_and_never_through_a_solid() {
    for size in SIZES {
        let wild = vines(2.0);
        let mut farm = farm(size, &wild, walled(&wild, mesh::POS_X));
        let cap = (2.0 / size) as usize;
        run_until(&mut farm, 80_000, |f| {
            vine_cells(f, wild.plant).len() >= cap
        });
        farm.run(3000);
        let cells = vine_cells(&farm, wild.plant);
        assert_eq!(cells.len(), cap, "{size}: vines {cells:?}");
        for c in cells {
            assert!(c[0] < WALL_X, "{size}: a vine inside the wall at {c:?}");
        }
    }
}

#[test]
fn every_vine_draws_against_the_face_it_holds_and_hangs_below_an_overhang() {
    let wild = vines(12.0);
    let mut chunk = walled(&wild, mesh::POS_X);
    for x in 12..WALL_X as usize {
        for z in 12..20 {
            chunk.set(x, CROP_Y as usize + 7, z, Voxel::cube(soil()));
        }
    }
    let mut farm = farm(1.0, &wild, chunk);
    run_until(&mut farm, 80_000, |f| vine_cells(f, wild.plant).len() >= 12);
    let cells = vine_cells(&farm, wild.plant);
    assert_eq!(cells.len(), 12);
    let tables = Arc::clone(&farm.app.world().resource::<ContentTables>().0);
    let mut faces = std::collections::BTreeSet::new();
    for c in &cells {
        let v = farm.store().voxel(*c).unwrap();
        let face = tables.shapes.plane_face(v).expect("a vine holds a face");
        faces.insert(face);
        let step = mesh::FACE_STEPS[face];
        let behind = [c[0] + step[0], c[1] + step[1], c[2] + step[2]];
        let held = farm.store().voxel(behind).unwrap().material() == soil();
        let hangs = material_at(&farm, [c[0], c[1] + 1, c[2]]) == wild.plant;
        assert!(
            held || hangs,
            "{c:?} draws against {face} with nothing there"
        );
    }
    assert!(faces.contains(&mesh::POS_X));
    assert!(
        faces.contains(&mesh::POS_Y),
        "no vine hangs under the overhang: {faces:?}"
    );
}

#[test]
fn removing_the_wall_breaks_the_vines_that_held_it() {
    let wild = vines(3.0);
    let mut farm = farm(1.0, &wild, walled(&wild, mesh::POS_X));
    run_until(&mut farm, 80_000, |f| vine_cells(f, wild.plant).len() >= 3);
    let before = vine_cells(&farm, wild.plant).len();
    assert!(before >= 3);
    farm.sim_mut().take_broken();
    for y in CROP_Y..CROP_Y + 14 {
        for z in 0..CHUNK_SIZE as i32 {
            for x in WALL_X..CHUNK_SIZE as i32 {
                farm.solid([x, y, z], Voxel::AIR);
            }
        }
    }
    farm.run(200);
    assert!(vine_cells(&farm, wild.plant).is_empty(), "a vine floats");
    assert_eq!(farm.sim_mut().take_broken().len(), before);
}

#[test]
fn a_body_walks_through_a_vine_and_a_ray_stops_on_it() {
    let wild = vines(3.0);
    let shapes = &wild.tables.shapes;
    for o in Orientation::all() {
        let vine = Voxel::shaped(wild.plant, wild.shape, o);
        let none = tesserae_core::shape::Connections::NONE;
        assert!(
            shapes.collision(vine, none).is_empty(),
            "{o:?} blocks a body"
        );
        assert!(
            !shapes.selection(vine, none).is_empty(),
            "{o:?} cannot be aimed at"
        );
    }
}
