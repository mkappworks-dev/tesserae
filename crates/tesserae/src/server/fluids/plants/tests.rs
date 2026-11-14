use super::super::*;
use bevy::ecs::system::RunSystemOnce;
use bevy::tasks::{AsyncComputeTaskPool, TaskPoolBuilder};
use tesserae_core::light::{MAX_LIGHT, light_cells};
use tesserae_core::mesh::POS_Y;
use tesserae_core::plant::{Kind, Needs, Plant, Stage, Water};
use tesserae_core::{
    CHUNK_SIZE, Chunk, ChunkLight, Climate, ClimateParams, Fluid, Light, LitChunk, MaterialDef,
    MaterialId, Occlusion, Occupants, RenderClass, Voxel, Worldgen, WorldgenParams,
};

use super::super::super::WorldGenerator;
use super::super::super::edit::set_occupants;

const SEED: u64 = 0x7e55_e2ae_0000_0007;
const SOIL_Y: usize = 3;
const CROP_Y: i32 = SOIL_Y as i32 + 1;
const FIELD: i32 = 16;
const BOUND_FREE: usize = usize::MAX;

fn soil() -> MaterialId {
    tesserae_core::testgen::fixtures().ground.soil
}

fn rock() -> MaterialId {
    tesserae_core::testgen::fixtures().ground.rock
}

struct Content {
    tables: Tables,
    /// The crop's Stages, seed first.
    stages: [MaterialId; 3],
}

/// A crop that rolls every Sample (`mean-seconds` at the interval), so a Stage
/// is a matter of which Voxels the draw lands on and the needs alone.
fn content(needs: impl FnOnce(&mut Needs)) -> Content {
    let mut tables = crate::server::fixture_tables();
    let def = MaterialDef {
        emission: 0,
        dimming: 0,
        occlusion: Occlusion::Never,
        render: RenderClass::Cutout,
        solid: true,
    };
    let stages = [(); 3].map(|_| tables.materials.register(def).unwrap());
    let mut n = Needs {
        soil: vec![soil()],
        light_min: 8,
        light_max: 15,
        ..Needs::default()
    };
    needs(&mut n);
    let stage = |m, mean| Stage {
        material: m,
        mean_seconds: mean,
        yields: Vec::new(),
    };
    let every = Some(tesserae_core::plant::SAMPLE_INTERVAL_S);
    tables
        .plants
        .push(Plant {
            name: "wheat".into(),
            seed: "t:seed".into(),
            first: stages[0],
            shape: None,
            needs: n,
            kind: Kind::Crop {
                stages: vec![
                    stage(stages[0], every),
                    stage(stages[1], every),
                    stage(stages[2], None),
                ],
                harvest_to: None,
            },
            sapling_yields: Vec::new(),
        })
        .unwrap();
    Content { tables, stages }
}

struct Farm {
    app: App,
    voxel_m: f32,
    stages: [MaterialId; 3],
}

impl Farm {
    fn new(voxel_m: f32, content: Content) -> Self {
        let Content { tables, stages } = content;
        let mut chunk = Chunk::air();
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                chunk.set(x, SOIL_Y, z, Voxel::cube(soil()));
            }
        }
        for z in 0..FIELD as usize {
            for x in 0..FIELD as usize {
                chunk.set(x, CROP_Y as usize, z, Voxel::cube(stages[0]));
            }
        }
        Self::open(voxel_m, tables, stages, chunk)
    }

    /// A World of one lit Chunk at the origin.
    fn open(voxel_m: f32, tables: Tables, stages: [MaterialId; 3], chunk: Chunk) -> Self {
        AsyncComputeTaskPool::get_or_init(|| TaskPoolBuilder::new().num_threads(4).build());
        let mut app = App::new();
        app.insert_resource(WorldSettings {
            voxel_size: voxel_m,
            seed: SEED,
        })
        .insert_resource(ContentTables(Arc::new(tables)))
        .init_resource::<ChunkStore>()
        .init_resource::<MeshState>()
        .init_resource::<FluidSim>();
        let mut farm = Farm {
            app,
            voxel_m,
            stages,
        };
        farm.hold(ChunkCoord::new(0, 0, 0), chunk);
        farm
    }

    fn hold(&mut self, coord: ChunkCoord, chunk: Chunk) {
        let tables = Arc::clone(&self.app.world().resource::<ContentTables>().0);
        let mut store = self.app.world_mut().resource_mut::<ChunkStore>();
        store.insert(coord, chunk);
        let sky = ChunkLight::filled(Light::new(MAX_LIGHT, 0));
        let air = Chunk::air();
        let mut border = [None; 6];
        border[POS_Y] = Some(LitChunk::new(&air, &sky));
        let light = light_cells(
            store.get(coord).unwrap(),
            store.overlay(coord).unwrap(),
            border,
            &tables,
        );
        store.set_light(coord, light);
    }

    fn with_generator(mut self) -> Self {
        let generator = Worldgen::new(
            SEED,
            self.voxel_m,
            WorldgenParams {
                climate: ClimateParams::default(),
                ..WorldgenParams::default()
            },
        );
        self.app.insert_resource(WorldGenerator(generator));
        self
    }

    fn climate(&self) -> Climate {
        let g = &self.app.world().resource::<WorldGenerator>().0;
        g.climate_at(0.0, 0.0)
    }

    fn tick(&mut self) {
        self.app.world_mut().run_system_once(simulate).unwrap();
        if self.sim().job.in_flight() > 0 {
            std::thread::sleep(std::time::Duration::from_micros(50));
        }
    }

    fn run(&mut self, steps: u64) {
        let until = self.sim().now() + steps;
        while self.sim().now() < until {
            self.tick();
        }
    }

    fn sim(&self) -> &FluidSim {
        self.app.world().resource::<FluidSim>()
    }

    fn sim_mut(&mut self) -> Mut<'_, FluidSim> {
        self.app.world_mut().resource_mut::<FluidSim>()
    }

    fn store(&self) -> &ChunkStore {
        self.app.world().resource::<ChunkStore>()
    }

    fn set(&mut self, at: [i32; 3], occupants: Occupants) {
        let tables = Arc::clone(&self.app.world().resource::<ContentTables>().0);
        self.app
            .world_mut()
            .resource_scope(|world, mut store: Mut<ChunkStore>| {
                let mut meshes = world.resource_mut::<MeshState>();
                set_occupants(&mut store, &mut meshes, &tables, at, occupants).unwrap();
            });
    }

    fn solid(&mut self, at: [i32; 3], voxel: Voxel) {
        self.set(
            at,
            Occupants {
                solid: voxel,
                fluid: None,
                burning: None,
            },
        );
    }

    /// Each crop's Stage index, in x then z order.
    fn field(&self) -> Vec<usize> {
        let chunk = self.store().get(ChunkCoord::new(0, 0, 0)).unwrap();
        (0..FIELD as usize)
            .flat_map(|z| (0..FIELD as usize).map(move |x| (x, z)))
            .map(|(x, z)| {
                let m = chunk.get(x, CROP_Y as usize, z).material();
                self.stages.iter().position(|&s| s == m).expect("a Stage")
            })
            .collect()
    }

    fn advanced(&self) -> usize {
        self.field().iter().filter(|&&s| s > 0).count()
    }
}

const SIZES: [f32; 3] = [1.0, 0.5, 0.25];
const ENOUGH: u64 = 3000;

mod reach;
mod tree;

#[test]
fn a_stage_change_keeps_the_plants_shape() {
    for size in SIZES {
        let mut content = content(|_| {});
        let cross = content
            .tables
            .shapes
            .register(tesserae_core::ShapeDef {
                form: tesserae_core::Form::Cross,
                boxes: vec![tesserae_core::NodeBox::new([4, 0, 4], [12, 12, 12])],
                ..tesserae_core::ShapeDef::default()
            })
            .unwrap();
        let stages = content.stages;
        let mut farm = Farm::new(size, content);
        for z in 0..FIELD {
            for x in 0..FIELD {
                farm.solid([x, CROP_Y, z], Voxel::new(stages[0], cross, 0));
            }
        }
        farm.run(ENOUGH);
        assert!(farm.advanced() > 0, "{size}: nothing grew");
        let chunk = farm.store().get(ChunkCoord::new(0, 0, 0)).unwrap();
        for z in 0..FIELD as usize {
            for x in 0..FIELD as usize {
                assert_eq!(chunk.get(x, CROP_Y as usize, z).shape(), cross, "{size}");
            }
        }
    }
}

#[test]
fn a_crop_advances_stage_by_stage_to_ripe_and_stays_ripe() {
    for size in SIZES {
        let mut farm = Farm::new(size, content(|_| {}));
        let mut before = farm.field();
        let mut ripe = false;
        for _ in 0..ENOUGH {
            farm.run(1);
            let now = farm.field();
            for (b, a) in before.iter().zip(&now) {
                assert!(
                    *a == *b || *a == b + 1,
                    "{size}: a Stage skipped or went back"
                );
            }
            before = now;
            ripe |= before.contains(&2);
        }
        assert!(ripe, "{size}: no crop ripened in {ENOUGH} steps");
        assert!(farm.sim().plants.landed > 0);
    }
}

/// Runs the same field with a need unmet and met, which must differ only by it.
fn holds_then_grows(
    size: f32,
    fail: impl FnOnce(&mut Needs),
    fix: impl FnOnce(&mut Farm),
    farm: impl Fn(Farm) -> Farm,
) {
    let mut held = farm(Farm::new(size, content(fail)));
    held.run(ENOUGH);
    assert_eq!(held.advanced(), 0, "{size}: grew with a need unmet");
    assert_eq!(held.sim().plants.landed, 0);
    fix(&mut held);
    held.run(ENOUGH);
    assert!(
        held.advanced() > 0,
        "{size}: did not resume once the need held"
    );
}

#[test]
fn a_crop_on_the_wrong_soil_holds_until_the_soil_is_right() {
    for size in SIZES {
        holds_then_grows(
            size,
            |n| n.soil = vec![rock()],
            |f| {
                for z in 0..FIELD {
                    for x in 0..FIELD {
                        f.solid([x, SOIL_Y as i32, z], Voxel::cube(rock()));
                    }
                }
            },
            |f| f,
        );
    }
}

#[test]
fn a_crop_in_the_dark_holds_until_light_reaches_it() {
    for size in SIZES {
        let roof = |f: &mut Farm| {
            for z in 0..CHUNK_SIZE as i32 {
                for x in 0..CHUNK_SIZE as i32 {
                    f.solid([x, CROP_Y + 3, z], Voxel::cube(rock()));
                }
            }
        };
        let mut farm = Farm::new(size, content(|_| {}));
        roof(&mut farm);
        farm.run(ENOUGH);
        assert_eq!(farm.advanced(), 0, "{size}: grew in the dark");
        for z in 0..CHUNK_SIZE as i32 {
            for x in 0..CHUNK_SIZE as i32 {
                farm.solid([x, CROP_Y + 3, z], Voxel::AIR);
            }
        }
        farm.run(ENOUGH);
        assert!(farm.advanced() > 0, "{size}: did not resume in the light");
    }
}

#[test]
fn a_crop_outside_its_climate_holds_and_inside_it_grows() {
    for size in SIZES {
        let probe = Farm::new(size, content(|_| {})).with_generator();
        let here = probe.climate().temperature;
        let (low, high) = if here < 0.6 {
            (here + 0.3, here + 0.4)
        } else {
            (here - 0.4, here - 0.3)
        };
        let mut farm =
            Farm::new(size, content(|n| n.temperature = Some((low, high)))).with_generator();
        farm.run(ENOUGH);
        assert_eq!(farm.advanced(), 0, "{size}: grew outside its Climate");
        let mut farm = Farm::new(
            size,
            content(|n| n.temperature = Some((here - 0.1, here + 0.1))),
        )
        .with_generator();
        farm.run(ENOUGH);
        assert!(farm.advanced() > 0, "{size}: held inside its Climate");
    }
}

#[test]
fn a_crop_that_needs_water_grows_only_within_its_reach_in_metres() {
    let fluid = tesserae_core::testgen::fixtures().water;
    for size in SIZES {
        let needs = |n: &mut Needs| {
            n.water = Some(Water {
                fluids: vec![fluid],
                reach_m: 2.0,
            });
        };
        let source = |f: &mut Farm, at: [i32; 3]| {
            f.set(
                at,
                Occupants {
                    solid: Voxel::AIR,
                    fluid: Some(Fluid::source(fluid)),
                    burning: None,
                },
            );
        };
        let mut dry = Farm::new(size, content(needs));
        dry.run(ENOUGH);
        assert_eq!(dry.advanced(), 0, "{size}: grew with no water");

        let mut far = Farm::new(size, content(needs));
        source(&mut far, [31, CROP_Y, 31]);
        far.run(ENOUGH);
        assert_eq!(far.advanced(), 0, "{size}: grew with water out of reach");

        let mut near = Farm::new(size, content(needs));
        source(&mut near, [FIELD / 2, CROP_Y, FIELD]);
        near.run(ENOUGH);
        assert!(
            near.advanced() > 0,
            "{size}: water at hand did not let crops grow"
        );
    }
}

#[test]
fn growth_past_the_landing_bound_lands_on_later_steps() {
    let mut farm = Farm::new(1.0, content(|_| {}));
    farm.sim_mut().plants.set_bound(1);
    let mut late = false;
    for _ in 0..ENOUGH {
        let before = farm.sim().plants.landed;
        farm.run(1);
        let landed = farm.sim().plants.landed - before;
        assert!(
            landed <= 1,
            "{landed} landed in one step against a bound of 1"
        );
        late |= farm.sim().plants.late > 0;
    }
    assert!(late, "a bound of one never deferred a change");
    assert!(farm.advanced() > 0);
}

#[test]
fn a_deferred_change_is_dropped_when_its_voxel_changed() {
    let mut farm = Farm::new(1.0, content(|_| {}));
    farm.sim_mut().plants.set_bound(1);
    let mut steps = 0;
    while farm.sim().plants.waiting() == 0 {
        farm.run(1);
        steps += 1;
        assert!(steps < 20_000, "nothing was ever deferred");
    }
    let waiting: Vec<_> = farm.sim().plants.deferred.keys().copied().collect();
    for at in &waiting {
        farm.solid(*at, Voxel::AIR);
    }
    farm.sim_mut().plants.set_bound(BOUND_FREE);
    farm.run(1);
    for at in waiting {
        let v = VoxelCoord::of(at);
        let held = farm.store().get(v.chunk).unwrap().get(v.x, v.y, v.z);
        assert!(held.is_air(), "a dropped change came back at {at:?}");
    }
}

#[test]
fn a_crop_beside_unheld_terrain_holds_rather_than_reading_air() {
    let mut farm = Farm::new(1.0, content(|_| {}));
    let far = ChunkCoord::new(7, 0, 7);
    let mut above = Chunk::air();
    above.set(0, 0, 0, Voxel::cube(farm.stages[0]));
    farm.hold(far, above);
    farm.run(ENOUGH);
    let crop = |farm: &Farm| farm.store().get(far).unwrap().get(0, 0, 0).material();
    assert_eq!(
        crop(&farm),
        farm.stages[0],
        "grew over a Chunk that is not held"
    );
    let mut below = Chunk::air();
    below.set(0, CHUNK_SIZE - 1, 0, Voxel::cube(soil()));
    farm.hold(ChunkCoord::new(7, -1, 7), below);
    farm.run(ENOUGH);
    assert_ne!(
        crop(&farm),
        farm.stages[0],
        "did not grow once its soil was held"
    );
}

#[test]
fn a_chunk_whose_light_solve_is_pending_holds_and_resumes_with_no_credit() {
    let mut farm = Farm::new(1.0, content(|_| {}));
    let home = ChunkCoord::new(0, 0, 0);
    farm.run(200);
    let landed = farm.sim().plants.landed;
    farm.app
        .world_mut()
        .resource_mut::<MeshState>()
        .mark_unlit(home);
    farm.run(ENOUGH);
    assert_eq!(
        farm.sim().plants.landed,
        landed,
        "grew while its light was pending"
    );
    farm.app
        .world_mut()
        .resource_mut::<MeshState>()
        .mark_lit(home);
    farm.run(ENOUGH);
    assert!(farm.advanced() > 0);
}

#[test]
fn an_evicted_chunk_grows_nothing_while_gone_and_nothing_is_credited_on_return() {
    let mut farm = Farm::new(1.0, content(|_| {}));
    let home = ChunkCoord::new(0, 0, 0);
    farm.run(500);
    let held = farm.store().get(home).unwrap().clone();
    farm.app
        .world_mut()
        .resource_mut::<ChunkStore>()
        .remove(home);
    let landed = farm.sim().plants.landed;
    farm.run(ENOUGH);
    assert_eq!(farm.sim().plants.landed, landed);
    farm.hold(home, held.clone());
    let back = farm.field();
    farm.run(1);
    farm.run(1);
    let after: usize = farm
        .field()
        .iter()
        .zip(&back)
        .filter(|(a, b)| a != b)
        .count();
    assert!(
        after <= 2,
        "{after} crops moved at once on return, as if credited for time away"
    );
}

#[test]
fn equal_seed_state_and_events_give_equal_stages_at_every_worker_division() {
    let run = |evaluations, jobs| {
        let mut farm = Farm::new(1.0, content(|_| {}));
        farm.sim_mut().set_division(evaluations, jobs);
        let water = tesserae_core::testgen::fixtures().water;
        farm.set(
            [24, CROP_Y, 24],
            Occupants {
                solid: Voxel::AIR,
                fluid: Some(Fluid::source(water)),
                burning: None,
            },
        );
        while farm.sim().now() < 800 {
            farm.tick();
        }
        let chunk = farm.store().get(ChunkCoord::new(0, 0, 0)).unwrap();
        let mut state = Vec::new();
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    state.push(chunk.get(x, y, z));
                }
            }
        }
        (state, farm.advanced())
    };
    let (a, grown) = run(1024, 4);
    assert!(grown > 0);
    assert!(a == run(1, 1).0, "one-evaluation slices changed what grew");
    assert!(a == run(64, 2).0, "two workers changed what grew");
}
