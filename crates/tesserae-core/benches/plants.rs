//! Plants in the mesher and the generator, timed as interleaved CPU time: each
//! round runs every variant once, and the median over rounds is reported, so
//! host load and thermals fall on all variants alike (wall-clock criterion
//! runs one variant after another and drifts with load).
//!
//! `cargo bench -p tesserae-core --bench plants`

use std::hint::black_box;
use std::sync::OnceLock;
use tesserae_core::light::{Light, MAX_LIGHT};
use tesserae_core::plant::{Branching, CountRange};
use tesserae_core::testgen::{self, fixtures};
use tesserae_core::{
    AIR, Biome, BiomeName, BiomeTable, CHUNK_SIZE, Chunk, ChunkLight, ClimateParams, Clump, Form,
    LitChunk, MAX_SHAPED_PER_CHUNK, MAX_SURFACE_STAGES, MaterialId, Neighborhood, NodeBox,
    Orientation, SHAPE_STAIR, ShapeDef, SurfaceFeature, SurfaceFeatures, SurfacePlant,
    SurfacePlantKind, SurfacePlantKind as Kind, SurfaceTree, Tables, Voxel, Worldgen,
    WorldgenParams, greedy_mesh,
};

/// Thread CPU time (user + system) in nanoseconds.
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
    static START: OnceLock<std::time::Instant> = OnceLock::new();
    START
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_nanos() as u64
}

fn load() -> String {
    let out = std::process::Command::new("uptime").output().ok();
    out.map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

/// Runs every variant `reps` times per round for `rounds` rounds, interleaved,
/// and prints each one's median CPU time per call.
fn compare(
    title: &str,
    rounds: usize,
    reps: usize,
    variants: &mut [(&str, Box<dyn FnMut() + '_>)],
) {
    let mut samples: Vec<Vec<f64>> = vec![Vec::new(); variants.len()];
    for (_, f) in variants.iter_mut() {
        for _ in 0..reps {
            f();
        }
    }
    for _ in 0..rounds {
        for (i, (_, f)) in variants.iter_mut().enumerate() {
            let t = cpu_ns();
            for _ in 0..reps {
                f();
            }
            samples[i].push((cpu_ns() - t) as f64 / reps as f64);
        }
    }
    println!("{title}  [{rounds} rounds x {reps}]");
    let mut base = None;
    for ((name, _), mut s) in variants.iter().zip(samples) {
        s.sort_by(f64::total_cmp);
        let median = s[s.len() / 2];
        let first = *base.get_or_insert(median);
        println!(
            "  {name:<28} {:>10.1} us   {:>6.1}% of first",
            median / 1e3,
            median / first * 100.0
        );
    }
}

fn tables() -> &'static Tables {
    static TABLES: OnceLock<Tables> = OnceLock::new();
    TABLES.get_or_init(|| {
        let mut t = fixtures().tables.clone();
        t.shapes
            .register(ShapeDef {
                form: Form::Cross,
                boxes: vec![NodeBox::new([4, 0, 4], [12, 12, 12])],
                ..ShapeDef::default()
            })
            .unwrap();
        t
    })
}

fn cross() -> u8 {
    static_cross()
}

fn static_cross() -> u8 {
    static ID: OnceLock<u8> = OnceLock::new();
    *ID.get_or_init(|| {
        let mut t = fixtures().tables.clone();
        t.shapes
            .register(ShapeDef {
                form: Form::Cross,
                boxes: vec![NodeBox::new([4, 0, 4], [12, 12, 12])],
                ..ShapeDef::default()
            })
            .unwrap()
    })
}

fn surface(chunk: &Chunk, x: usize, z: usize) -> Option<usize> {
    (0..CHUNK_SIZE)
        .rev()
        .find(|&y| !chunk.get(x, y, z).is_air())
}

/// Terrain with `put` on top of the first `n` columns.
fn terrain_with(n: usize, put: impl Fn(usize, usize) -> Voxel) -> Chunk {
    let mut chunk = testgen::terrain_chunk(0, 0, 0, fixtures().ground);
    let mut placed = 0;
    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            if placed == n {
                return chunk;
            }
            let Some(y) = surface(&chunk, x, z) else {
                continue;
            };
            if y + 1 < CHUNK_SIZE {
                // The stair replaces the surface Voxel; a cross stands on it.
                chunk.set(x, y + 1, z, put(x, z));
                placed += 1;
            }
        }
    }
    chunk
}

fn meshing() {
    let sky = ChunkLight::filled(Light::new(MAX_LIGHT, 0));
    let leaves = fixtures().leaves;
    let rock = fixtures().ground.rock;
    let cross = Voxel::new(leaves, cross(), 0);
    let bare = testgen::terrain_chunk(0, 0, 0, fixtures().ground);
    let crosses = terrain_with(1024, |_, _| cross);
    let half = terrain_with(512, |_, _| cross);
    let stairs = {
        let mut c = testgen::terrain_chunk(0, 0, 0, fixtures().ground);
        let mut placed = 0;
        'all: for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                if placed == MAX_SHAPED_PER_CHUNK {
                    break 'all;
                }
                if let Some(y) = surface(&c, x, z) {
                    let o = Orientation::from_bits(((x * 7 + z * 3) % 24) as u8).unwrap();
                    c.set(x, y, z, Voxel::shaped(rock, SHAPE_STAIR, o));
                    placed += 1;
                }
            }
        }
        c
    };
    let nb = |c| Neighborhood::isolated(LitChunk::new(c, &sky));
    let (bare_nb, cross_nb, half_nb, stair_nb) = (nb(&bare), nb(&crosses), nb(&half), nb(&stairs));
    // A tall meadow: four stacked crosses over every column, which only a
    // Player's tall cane or reeds can make.
    let tall = {
        let mut c = testgen::terrain_chunk(0, 0, 0, fixtures().ground);
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                if let Some(y) = surface(&c, x, z) {
                    for dy in 1..=4.min(CHUNK_SIZE - 1 - y) {
                        c.set(x, y + dy, z, cross);
                    }
                }
            }
        }
        c
    };
    let tall_nb = nb(&tall);
    let t = tables();
    compare(
        "greedy_mesh over the same terrain",
        41,
        20,
        &mut [
            (
                "terrain, bare",
                Box::new(|| drop(black_box(greedy_mesh(&bare_nb, t)))),
            ),
            (
                "terrain + 1024 cross",
                Box::new(|| drop(black_box(greedy_mesh(&cross_nb, t)))),
            ),
            (
                "terrain + 512 cross",
                Box::new(|| drop(black_box(greedy_mesh(&half_nb, t)))),
            ),
            (
                "terrain + ~4096 cross (4 high)",
                Box::new(|| drop(black_box(greedy_mesh(&tall_nb, t)))),
            ),
            (
                "terrain + 1024 stairs",
                Box::new(|| drop(black_box(greedy_mesh(&stair_nb, t)))),
            ),
        ],
    );
}

const TRUNK: MaterialId = 210;
const CANOPY: MaterialId = 211;

fn stages(first: MaterialId, len: u8) -> SurfacePlantKind {
    let mut materials = [AIR; MAX_SURFACE_STAGES];
    for i in 0..len {
        materials[i as usize] = first + i as MaterialId;
    }
    Kind::Stages { materials, len }
}

/// Sixteen features as a Terra-like Biome lists them: three kinds of tree, a
/// run of crops and flowers, two columns and two spreaders.
///
/// With `branched` the three trees take the shared oak's, jungle's and birch's branching.
fn sixteen(cross: u8, branched: bool) -> Vec<SurfaceFeature> {
    let limbs = |min, max| CountRange { min, max };
    let shapes = [
        Branching {
            limbs: limbs(3, 4),
            start: 0.25,
            reach_m: 3.5,
            rise: 0.3,
            fork: Some(0.3),
            lean_m: 0.0,
            tiers: false,
            lobe_radius_m: 1.5,
        },
        Branching {
            limbs: limbs(3, 4),
            start: 0.55,
            reach_m: 4.5,
            rise: 0.2,
            fork: None,
            lean_m: 0.0,
            tiers: false,
            lobe_radius_m: 1.8,
        },
        Branching {
            limbs: limbs(2, 3),
            start: 0.5,
            reach_m: 1.5,
            rise: 0.6,
            fork: None,
            lean_m: 1.0,
            tiers: false,
            lobe_radius_m: 0.8,
        },
    ];
    let tree = |trunk_m: (f32, f32), radius, shape: usize| SurfaceTree {
        trunk: TRUNK,
        canopy: CANOPY,
        trunk_height_m: trunk_m,
        canopy_radius_m: radius,
        fruit: None,
        vines: None,
        branching: branched.then_some(shapes[shape]),
    };
    let plant = |first, spacing_m, kind| SurfaceFeature {
        material: first,
        spacing_m,
        plant: Some(SurfacePlant { shape: cross, kind }),
        clump: None,
    };
    let mut f = vec![
        plant(TRUNK, 7.0, Kind::Tree(tree((3.0, 5.0), 2.5, 0))),
        plant(TRUNK, 11.0, Kind::Tree(tree((5.0, 8.0), 3.5, 1))),
        plant(TRUNK, 13.0, Kind::Tree(tree((2.0, 3.0), 1.5, 2))),
    ];
    for i in 0..4 {
        f.push(plant(220 + i * 3, 4.0, stages(220 + i * 3, 3)));
    }
    for i in 0..5 {
        f.push(plant(240 + i, 4.0, stages(240 + i, 1)));
    }
    for i in 0..2 {
        f.push(plant(
            250 + i,
            6.0,
            Kind::Column {
                material: 250 + i,
                max_height_m: 3.0,
                stands_in: None,
            },
        ));
    }
    f.push(plant(260, 9.0, stages(260, 1)));
    f.push(plant(261, 9.0, stages(261, 1)));
    assert_eq!(f.len(), 16);
    f
}

fn params(features: &[SurfaceFeature]) -> WorldgenParams {
    let biome = Biome {
        name: BiomeName::new("t:plain").unwrap(),
        temperature: 0.5,
        humidity: 0.5,
        ground: None,
        sky_fog: None,
        cave_scale: None,
        surface_features: SurfaceFeatures::new(features).unwrap(),
    };
    WorldgenParams {
        caves: None,
        climate: ClimateParams {
            temperature_size_m: 64.0,
            humidity_size_m: 64.0,
        },
        biomes: BiomeTable::new(&[biome]).unwrap(),
        ..testgen::world_params()
    }
}

fn generation() {
    let features = sixteen(cross(), false);
    let branched = sixteen(cross(), true);
    let grouped: Vec<SurfaceFeature> = features
        .iter()
        .map(|f| SurfaceFeature {
            clump: Some(Clump {
                scale_m: 32.0,
                strength: 0.7,
            }),
            ..*f
        })
        .collect();
    for size in [1.0f32, 0.5, 0.25] {
        let with = Worldgen::new(0x7e55_e2ae, size, params(&features));
        let bare = Worldgen::new(
            0x7e55_e2ae,
            size,
            params(&features).without_surface_features(),
        );
        let clumped = Worldgen::new(0x7e55_e2ae, size, params(&grouped));
        let trees = Worldgen::new(0x7e55_e2ae, size, params(&features[..3]));
        let flora = Worldgen::new(0x7e55_e2ae, size, params(&features[3..]));
        let branching = Worldgen::new(0x7e55_e2ae, size, params(&branched[..3]));
        let all_branching = Worldgen::new(0x7e55_e2ae, size, params(&branched));
        let cell = CHUNK_SIZE as f32 * size;
        let y = (with.surface_height_m(0.0, 0.0) / cell).floor() as i32;
        // The nine Chunks around the surface Chunk: trees come in from neighbours.
        let ring: Vec<(i32, i32, i32)> = (-1..=1)
            .flat_map(|z| (-1..=1).map(move |x| (x, y, z)))
            .collect();
        compare(
            &format!("worldgen of 9 surface Chunks (divide by 9), {size} m"),
            41,
            3,
            &mut [
                (
                    "16 features off (bare)",
                    Box::new(|| {
                        for &(x, y, z) in &ring {
                            black_box(bare.chunk(x, y, z));
                        }
                    }),
                ),
                (
                    "3 trees only",
                    Box::new(|| {
                        for &(x, y, z) in &ring {
                            black_box(trees.chunk(x, y, z));
                        }
                    }),
                ),
                (
                    "3 branching trees only",
                    Box::new(|| {
                        for &(x, y, z) in &ring {
                            black_box(branching.chunk(x, y, z));
                        }
                    }),
                ),
                (
                    "13 plants, no trees",
                    Box::new(|| {
                        for &(x, y, z) in &ring {
                            black_box(flora.chunk(x, y, z));
                        }
                    }),
                ),
                (
                    "16 features (trees+plants)",
                    Box::new(|| {
                        for &(x, y, z) in &ring {
                            black_box(with.chunk(x, y, z));
                        }
                    }),
                ),
                (
                    "16 features, every one clumped",
                    Box::new(|| {
                        for &(x, y, z) in &ring {
                            black_box(clumped.chunk(x, y, z));
                        }
                    }),
                ),
                (
                    "16 features, trees branching",
                    Box::new(|| {
                        for &(x, y, z) in &ring {
                            black_box(all_branching.chunk(x, y, z));
                        }
                    }),
                ),
            ],
        );
    }
}

/// What a fruit tree's first re-bearing Sample pays: the expected canopy size, built once per
/// Plant and Voxel size from four reference trees and cached after.
fn rebearing() {
    use tesserae_core::plant::tree::expected_canopy_cells;
    use tesserae_core::plant::{Fruit, Stage, Tree};
    let oak = |branching| Tree {
        trunk: 1,
        canopy: 2,
        trunk_height_m: (4.0, 6.0),
        canopy_radius_m: 2.5,
        mean_seconds: 60.0,
        trunk_yield: Vec::new(),
        canopy_yield: Vec::new(),
        sapling_yield: CountRange { min: 1, max: 1 },
        fruit: Some(Fruit {
            stages: vec![
                Stage {
                    material: 3,
                    mean_seconds: Some(40.0),
                    yields: Vec::new(),
                },
                Stage {
                    material: 4,
                    mean_seconds: None,
                    yields: Vec::new(),
                },
            ],
            density: 0.12,
            regrow_seconds: 120.0,
            shape: None,
        }),
        vines: None,
        branching,
    };
    let branching = Branching {
        limbs: CountRange { min: 3, max: 4 },
        start: 0.25,
        reach_m: 3.5,
        rise: 0.3,
        fork: Some(0.3),
        lean_m: 0.0,
        tiers: false,
        lobe_radius_m: 1.5,
    };
    for size in [1.0, 0.5, 0.25] {
        let (plain, branched) = (oak(None), oak(Some(branching)));
        println!(
            "{size} m: {} / {} expected canopy cells (plain / branched)",
            expected_canopy_cells(&plain, size),
            expected_canopy_cells(&branched, size)
        );
        compare(
            &format!("expected canopy cells at {size} m"),
            9,
            3,
            &mut [
                (
                    "plain oak",
                    Box::new(|| {
                        black_box(expected_canopy_cells(&plain, size));
                    }),
                ),
                (
                    "branching oak",
                    Box::new(|| {
                        black_box(expected_canopy_cells(&branched, size));
                    }),
                ),
            ],
        );
    }
}

fn main() {
    println!("{}", load());
    meshing();
    generation();
    rebearing();
    println!("{}", load());
}
