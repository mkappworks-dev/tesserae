//! What the plant tick costs, as interleaved thread-CPU time (`--ignored`,
//! release): each round rebuilds and runs every variant once, so load falls on
//! all of them alike, and the median over rounds is printed.
//!
//! `cargo test --release -p tesserae --bin tesserae plants::tests::bench -- --ignored --nocapture --test-threads=1`

use super::tree::{BASE, Orchard, orchard_of, planted};
use super::*;
use tesserae_core::plant::Branching;
use tesserae_core::plant::CountRange;
use tesserae_core::plant::tree::tree_voxels;

#[cfg(target_os = "macos")]
fn cpu_ns() -> u64 {
    unsafe extern "C" {
        fn clock_gettime_nsec_np(clock: u32) -> u64;
    }
    const CLOCK_THREAD_CPUTIME_ID: u32 = 16;
    unsafe { clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID) }
}

#[cfg(not(target_os = "macos"))]
fn cpu_ns() -> u64 {
    0
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

fn uptime() -> String {
    let out = std::process::Command::new("uptime").output().unwrap();
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// Runs `f` on the parts of the World the tick reads.
fn with_world<R>(
    farm: &mut Farm,
    f: impl FnOnce(
        &mut FluidSim,
        &mut ChunkStore,
        &mut MeshState,
        &Tables,
        &plants::Env,
        &HeatScale,
    ) -> R,
) -> R {
    let tables = Arc::clone(&farm.app.world().resource::<ContentTables>().0);
    let size = farm.voxel_m;
    farm.app
        .world_mut()
        .resource_scope(|world, mut sim: Mut<FluidSim>| {
            world.resource_scope(|world, mut store: Mut<ChunkStore>| {
                let mut meshes = world.resource_mut::<MeshState>();
                let env = plants::Env {
                    seed: SEED,
                    voxel_size: size,
                    generator: None,
                };
                let scale = Arc::clone(sim.scale.as_ref().expect("simulate ran"));
                f(
                    &mut sim,
                    &mut store,
                    &mut meshes,
                    &tables,
                    &env,
                    &scale.heat,
                )
            })
        })
}

/// One Logical step's plant tick, as `simulate` runs it, and its CPU time.
fn tick(farm: &mut Farm) -> f64 {
    with_world(farm, |sim, store, meshes, tables, env, heat| {
        let t = cpu_ns();
        sample_plants(sim, store, meshes, tables, heat, env);
        let dt = cpu_ns() - t;
        sim.next_step();
        dt as f64
    })
}

fn admit(farm: &mut Farm, chunks: usize) {
    farm.run(4);
    let held = farm.sim().simulated.len();
    assert_eq!(held, chunks, "chunks admitted to the simulation");
}

fn meadow(mean: f32, chunks: i32) -> Farm {
    let Content { tables, stages } = content_every(mean, |_| {});
    let mut chunk = Chunk::air();
    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            chunk.set(x, SOIL_Y, z, Voxel::cube(soil()));
            chunk.set(x, CROP_Y as usize, z, Voxel::cube(stages[0]));
        }
    }
    many(tables, stages, chunk, chunks)
}

fn many(tables: Tables, stages: [MaterialId; 3], chunk: Chunk, chunks: i32) -> Farm {
    let mut farm = Farm::open(1.0, tables, stages, chunk.clone());
    for i in 1..chunks {
        farm.hold(ChunkCoord::new(i % 8, 0, i / 8), chunk.clone());
    }
    admit(&mut farm, chunks as usize);
    farm
}

const CHUNKS: i32 = 64;

fn ground() -> Chunk {
    let mut c = Chunk::air();
    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            c.set(x, SOIL_Y, z, Voxel::cube(soil()));
        }
    }
    c
}

struct Variant {
    name: &'static str,
    build: Box<dyn Fn() -> Farm>,
    steps: usize,
}

#[test]
#[ignore = "bench"]
fn sample_tick_cost_per_logical_step() {
    let forest = || {
        let o = orchard_of(None, (4.0, 6.0), 2.5, None);
        let mut c = ground();
        for gz in 0..4 {
            for gx in 0..4 {
                let base = [4 + gx * 8, CROP_Y, 4 + gz * 8];
                for v in tree_voxels(&o.tree, SEED, base, 1.0) {
                    c.set(
                        v.at[0] as usize,
                        v.at[1] as usize,
                        v.at[2] as usize,
                        Voxel::cube(v.material),
                    );
                }
            }
        }
        many(o.tables.clone(), [o.sapling; 3], c, CHUNKS)
    };
    // Every canopy Voxel of these is a re-bearing site: `bare` has none of its fruit, so each
    // passing roll lands; the other starts at generation's density.
    let fruit_forest = |bare: bool| {
        move || {
            let o = orchard_of(Some(0.12), (4.0, 6.0), 2.5, None);
            let mut c = ground();
            for gz in 0..4 {
                for gx in 0..4 {
                    let base = [4 + gx * 8, CROP_Y, 4 + gz * 8];
                    for v in tree_voxels(&o.tree, SEED, base, 1.0) {
                        let material = if bare && v.part == tesserae_core::plant::tree::Part::Fruit
                        {
                            o.tree.canopy.first()
                        } else {
                            v.material
                        };
                        c.set(
                            v.at[0] as usize,
                            v.at[1] as usize,
                            v.at[2] as usize,
                            Voxel::cube(material),
                        );
                    }
                }
            }
            many(o.tables.clone(), [o.sapling; 3], c, CHUNKS)
        }
    };
    let variants = [
        Variant {
            name: "no plant table",
            build: Box::new(|| many(crate::server::fixture_tables(), [0; 3], ground(), CHUNKS)),
            steps: 200,
        },
        Variant {
            name: "table, chunks hold none",
            build: Box::new(|| {
                let Content { tables, stages } = content(|_| {});
                many(tables, stages, ground(), CHUNKS)
            }),
            steps: 200,
        },
        Variant {
            name: "meadow 300 s mean",
            build: Box::new(|| meadow(300.0, CHUNKS)),
            steps: 200,
        },
        Variant {
            name: "meadow 16 s (saturated)",
            build: Box::new(|| meadow(tesserae_core::plant::SAMPLE_INTERVAL_S, CHUNKS)),
            steps: 40,
        },
        Variant {
            name: "forest (16 grown trees)",
            build: Box::new(forest),
            steps: 200,
        },
        Variant {
            name: "fruit forest (16 trees, 0.12)",
            build: Box::new(fruit_forest(false)),
            steps: 200,
        },
        Variant {
            name: "fruit forest, no fruit (bare)",
            build: Box::new(fruit_forest(true)),
            steps: 200,
        },
        Variant {
            name: "every Voxel a crop, no soil",
            build: Box::new(|| {
                let Content { tables, stages } = content(|_| {});
                many(
                    tables,
                    stages,
                    Chunk::filled(Voxel::cube(stages[0])),
                    CHUNKS,
                )
            }),
            steps: 100,
        },
    ];
    println!("{}", uptime());
    let rounds = 15;
    let mut per_step = vec![Vec::new(); variants.len()];
    let mut landed = vec![Vec::new(); variants.len()];
    for _ in 0..rounds {
        for (i, v) in variants.iter().enumerate() {
            let mut farm = (v.build)();
            let before = farm.sim().plants.landed;
            let total: f64 = (0..v.steps).map(|_| tick(&mut farm)).sum();
            per_step[i].push(total / v.steps as f64);
            landed[i].push((farm.sim().plants.landed - before) as f64 / v.steps as f64);
        }
    }
    println!(
        "Sample tick per Logical step over {CHUNKS} held Chunks ({rounds} rounds, median CPU):"
    );
    for (i, v) in variants.iter().enumerate() {
        let us = median(per_step[i].clone()) / 1e3;
        println!(
            "  {:<30} {:>9.2} us/step  {:>7.3} us/chunk  {:>6.1} landed/step",
            v.name,
            us,
            us / CHUNKS as f64,
            median(landed[i].clone())
        );
    }
    println!("{}", uptime());
}

/// The shared oak's branching, in metres.
const OAK_BRANCHING: Branching = Branching {
    limbs: CountRange { min: 3, max: 4 },
    start: 0.25,
    reach_m: 3.5,
    rise: 0.3,
    fork: Some(0.3),
    lean_m: 0.0,
    tiers: false,
    lobe_radius_m: 1.5,
};

/// A sapling at the middle of the ground Chunk, and what a tree of 7 m across needs held around
/// it: the nine columns of Chunks two high when `wide`, else one air Chunk above.
fn grove(size: f32, o: &Orchard, wide: bool) -> Farm {
    let mut farm = planted(size, o, &[BASE]);
    let mut chunks = 1;
    for y in 0..2 {
        for z in -1..=1 {
            for x in -1..=1 {
                let at = ChunkCoord::new(x, y, z);
                let ring = wide || (x, z) == (0, 0);
                if ring && (x, y, z) != (0, 0, 0) {
                    farm.hold(at, if y == 0 { ground() } else { Chunk::air() });
                    chunks += 1;
                }
            }
        }
    }
    admit(&mut farm, chunks);
    farm
}

/// A sapling grown to the declared tree at one Voxel size, and then felled.
#[test]
#[ignore = "bench"]
fn tree_growth_and_felling_cost() {
    println!("{}", uptime());
    for size in SIZES {
        for (label, branching) in [("oak", None), ("branching oak", Some(OAK_BRANCHING))] {
            let (mut landing, mut felling) = (Vec::new(), Vec::new());
            let mut voxels = 0;
            let mut per_bound: Vec<(usize, Vec<f64>, Vec<f64>)> = [4, 8, 32, 128, 512]
                .into_iter()
                .map(|b| (b, vec![], vec![]))
                .collect();
            for _ in 0..9 {
                let wide = branching.is_some();
                let o = orchard_of(None, (4.0, 6.0), 2.5, branching);
                let mut farm = grove(size, &o, wide);
                let mut grown = None;
                for _ in 0..200_000 {
                    let before = farm.sim().plants.landed;
                    let dt = tick(&mut farm);
                    if farm.sim().plants.landed > before {
                        grown = Some(dt);
                        break;
                    }
                }
                landing.push(grown.expect("the sapling grew") / 1e3);
                voxels = tree_voxels(&o.tree, SEED, BASE, size).len();
                for (bound, step_us, voxel_us) in &mut per_bound {
                    // Fell the tree one trunk Voxel above its foot, at each bound.
                    let mut farm2 = grove(size, &o, wide);
                    for v in tree_voxels(&o.tree, SEED, BASE, size) {
                        farm2.solid(v.at, Voxel::cube(v.material));
                    }
                    let cut = BASE;
                    with_world(&mut farm2, |sim, store, meshes, tables, env, _| {
                        sim.plants.set_felled_bound(*bound);
                        let mut batch = edit::Batch::default();
                        let here = store.occupants(cut, &tables.fluids).unwrap();
                        let air = Occupants {
                            solid: Voxel::AIR,
                            ..here
                        };
                        batch
                            .set_occupants(store, meshes, tables, cut, air)
                            .unwrap();
                        batch.finish(store, meshes, tables);
                        plants::mark_felled(sim, store, tables, env, &[cut]);
                    });
                    let queued = farm2.sim().plants.felling_left();
                    assert!(queued > 0, "{size}: nothing queued");
                    let mut steps = Vec::new();
                    while farm2.sim().plants.felling_left() > 0 {
                        steps.push(tick(&mut farm2));
                    }
                    let total: f64 = steps.iter().sum();
                    step_us.push(median(steps) / 1e3);
                    voxel_us.push(total / queued as f64 / 1e3);
                    if *bound == 32 {
                        felling.push(queued);
                    }
                }
            }
            println!(
                "{size} m {label}: {voxels} Voxels land in {:.1} us (median CPU, whole group + relight)",
                median(landing)
            );
            println!("  felling {} Voxels queued", felling[0]);
            for (bound, step_us, voxel_us) in per_bound {
                println!(
                    "  FELLED_PER_STEP {bound:>3}: median step {:>8.1} us, {:>6.2} us per Voxel (whole region)",
                    median(step_us),
                    median(voxel_us)
                );
            }
        }
    }
    println!("{}", uptime());
}
