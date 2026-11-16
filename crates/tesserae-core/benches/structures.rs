//! Structures and Settlements in the generator, timed as interleaved CPU time: each round runs
//! every variant once, and the median over rounds is reported, so host load and thermals fall on
//! all variants alike.
//!
//! `cargo bench -p tesserae-core --bench structures`; with `TESSERAE_BENCH_BARE=1` only the bare
//! World runs, for an A/B against another build of the same rounds.

use std::hint::black_box;
use tesserae_core::settlement::{Building, PlotRule, Settlement};
use tesserae_core::template::{Cell, Template};
use tesserae_core::testgen;
use tesserae_core::{
    Biome, BiomeName, BiomeSettlement, BiomeStructure, BiomeTable, CHUNK_SIZE, ClimateParams,
    MaterialId, SurfaceFeatures, Voxel, WaterParams, Worldgen, WorldgenParams,
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
    static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
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

type Variant<'a> = (&'a str, Box<dyn FnMut() + 'a>);

/// A generator at a Voxel size.
type Make = Box<dyn Fn(f32) -> Worldgen>;

/// Runs every variant `reps` times per round for `rounds` rounds, interleaved, and prints each
/// one's median CPU time per call divided by `per`, and the median over rounds of its ratio to the
/// last variant named bare before it, which drifts less with load than a ratio of medians.
fn compare(title: &str, rounds: usize, reps: usize, per: f64, variants: &mut [Variant]) {
    let mut samples: Vec<Vec<f64>> = vec![Vec::new(); variants.len()];
    for (_, f) in variants.iter_mut() {
        f();
    }
    for _ in 0..rounds {
        for (i, (_, f)) in variants.iter_mut().enumerate() {
            let t = cpu_ns();
            for _ in 0..reps {
                f();
            }
            samples[i].push((cpu_ns() - t) as f64 / (reps as f64 * per));
        }
    }
    let median = |mut s: Vec<f64>| {
        s.sort_by(f64::total_cmp);
        s[s.len() / 2]
    };
    println!("{title}  [{rounds} rounds x {reps}]");
    let mut base = 0;
    for (i, (name, _)) in variants.iter().enumerate() {
        if name.contains("bare") {
            base = i;
        }
        let ratios = samples[i]
            .iter()
            .zip(&samples[base])
            .map(|(s, b)| s / b)
            .collect();
        println!(
            "  {name:<40} {:>10.2} us   {:>6.1}% of bare",
            median(samples[i].clone()) / 1e3,
            median(ratios) * 100.0
        );
    }
}

const SEED: u64 = 0x7e55_e2ae;
const FLOOR: MaterialId = 180;
const WALL: MaterialId = 181;
const DOOR: MaterialId = 182;
const ROOF: MaterialId = 183;
const PATH: MaterialId = 184;
const FOUNDATION: MaterialId = 185;

/// A box `w` by `d` metres and `h` high on a 1 m grid: a floor on the ground, walls with a door
/// on the -Z edge, a roof, and air inside.
fn hut(name: &str, [w, h, d]: [usize; 3], tolerance_m: f32) -> Template {
    let layers = (0..h)
        .map(|y| {
            (0..d)
                .map(|z| {
                    (0..w)
                        .map(|x| {
                            let edge = x == 0 || z == 0 || x == w - 1 || z == d - 1;
                            let cube = |m| Cell::Voxel(Voxel::cube(m));
                            match y {
                                0 => cube(FLOOR),
                                _ if y == h - 1 => cube(ROOF),
                                1 | 2 if z == 0 && x == w / 2 => cube(DOOR),
                                _ if edge => cube(WALL),
                                _ => Cell::Air,
                            }
                        })
                        .collect()
                })
                .collect()
        })
        .collect();
    Template::new(name.into(), 1.0, layers, 0, Some(FOUNDATION), tolerance_m).unwrap()
}

/// At both caps: 64 m on its long sides and 65,536 cells, every one written: a hall of four
/// floors.
fn hall() -> Template {
    let layers = (0..16)
        .map(|y| {
            (0..64)
                .map(|z| {
                    (0..64)
                        .map(|x| {
                            let edge = x == 0 || z == 0 || x == 63 || z == 63;
                            if y % 5 == 0 || edge {
                                Cell::Voxel(Voxel::cube(if y == 15 { ROOF } else { WALL }))
                            } else {
                                Cell::Air
                            }
                        })
                        .collect()
                })
                .collect()
        })
        .collect();
    Template::new("t:hall".into(), 1.0, layers, 0, Some(FOUNDATION), 16.0).unwrap()
}

/// A 15 m by 5 m bridge: a deck on the Ground layer, piers under its ends, parapets.
fn bridge() -> Template {
    let layers = (0..3)
        .map(|y| {
            (0..5)
                .map(|z| {
                    (0..15)
                        .map(|x| {
                            let end = x == 0 || x == 14;
                            let edge = z == 0 || z == 4;
                            match y {
                                0 if end => Cell::Voxel(Voxel::cube(WALL)),
                                1 => Cell::Voxel(Voxel::cube(FLOOR)),
                                2 if edge => Cell::Voxel(Voxel::cube(WALL)),
                                _ => Cell::Air,
                            }
                        })
                        .collect()
                })
                .collect()
        })
        .collect();
    Template::new("t:bridge".into(), 1.0, layers, 1, Some(FOUNDATION), 2.0)
        .unwrap()
        .spanning()
        .unwrap()
}

/// Indices into it: a house, a shed, a well, the hall at the cap, then a bridge.
fn templates() -> Vec<Template> {
    vec![
        hut("t:house", [7, 6, 7], 2.0),
        hut("t:shed", [5, 4, 5], 2.0),
        hut("t:well", [3, 3, 3], 2.0),
        hall(),
        bridge(),
    ]
}

fn building(templates: &[Template], index: u16) -> Building {
    Building {
        template: index,
        door: templates[usize::from(index)].door_cell(|m| m == DOOR),
    }
}

fn settlement(name: &str, radius_m: f32, plots: u8) -> Settlement {
    let t = templates();
    let rule = |index, weight, min, max| PlotRule {
        building: building(&t, index),
        weight,
        min,
        max,
        loot: None,
    };
    Settlement {
        name: name.into(),
        radius_m,
        radius_min_m: radius_m,
        margin_m: 16.0,
        centre: Some(building(&t, 2)),
        plots,
        plots_min: plots,
        rules: vec![rule(0, 2.0, 1, plots), rule(1, 1.0, 1, plots / 2)],
        minimum: 2,
        path: Voxel::cube(PATH),
        loot: None,
    }
}

/// Gentle hills with caves and smoothing: ground a house stands on.
fn params(features: SurfaceFeatures) -> WorldgenParams {
    WorldgenParams {
        relief_m: 12.0,
        climate: ClimateParams {
            temperature_size_m: 64.0,
            humidity_size_m: 64.0,
        },
        biomes: BiomeTable::new(&[Biome {
            name: BiomeName::new("t:plain").unwrap(),
            temperature: 0.5,
            humidity: 0.5,
            ground: None,
            sky_fog: None,
            cave_scale: None,
            surface_features: features,
        }])
        .unwrap(),
        ..testgen::world_params()
    }
}

fn bare(size: f32) -> Worldgen {
    Worldgen::new(SEED, size, params(SurfaceFeatures::NONE))
}

/// Steep hills with lakes in their valleys: ground a bridge finds gaps in.
fn valleys(features: SurfaceFeatures) -> WorldgenParams {
    let hills = WorldgenParams {
        relief_m: 32.0,
        feature_size_m: 24.0,
        ..params(features)
    };
    WorldgenParams {
        water: Some(WaterParams {
            level_m: hills.sea_level_m + hills.relief_m * 5.0 / 12.0,
            material: testgen::fixtures().water,
        }),
        ..hills
    }
}

fn bare_valleys(size: f32) -> Worldgen {
    Worldgen::new(SEED, size, valleys(SurfaceFeatures::NONE))
}

fn with_bridge(size: f32) -> Worldgen {
    let row = BiomeStructure {
        template: 4,
        spacing_m: 96.0,
        chance: 1.0,
    };
    let features = SurfaceFeatures::NONE.with_structures(&[row]).unwrap();
    Worldgen::new(SEED, size, valleys(features)).with_templates(&templates())
}

fn with_structure(size: f32, template: u16, spacing_m: f32) -> Worldgen {
    let row = BiomeStructure {
        template,
        spacing_m,
        chance: 1.0,
    };
    let features = SurfaceFeatures::NONE.with_structures(&[row]).unwrap();
    Worldgen::new(SEED, size, params(features)).with_templates(&templates())
}

fn with_settlement(size: f32, settlement: Settlement) -> Worldgen {
    let row = BiomeSettlement {
        settlement: 0,
        spacing_m: 2.0 * (settlement.radius_m + settlement.margin_m),
        chance: 1.0,
        near_spawn: false,
    };
    let features = SurfaceFeatures::NONE.with_settlements(&[row]).unwrap();
    Worldgen::new(SEED, size, params(features))
        .with_templates(&templates())
        .with_settlements(&[settlement])
}

/// The metre column the 9 Chunks centre on: amid what `with` stands nearest the origin, found at
/// 1 m, so every Voxel size times the same ground. Also the number of Voxels it changed there.
/// How many Chunks either way of the origin [`centre_m`] looks: far enough for the cap town,
/// whose disc and margin keep it over 224 m out.
const SCAN_CHUNKS: i32 = 12;

fn centre_m(with: &Worldgen, bare: &Worldgen) -> ([f32; 2], usize) {
    let chunk_m = CHUNK_SIZE as f32;
    let (low, high) = with.surface_bounds_m();
    let rows = (low / chunk_m).floor() as i32..=(high / chunk_m).ceil() as i32;
    let mut changed = Vec::new();
    for cz in -SCAN_CHUNKS..SCAN_CHUNKS {
        for cx in -SCAN_CHUNKS..SCAN_CHUNKS {
            for cy in rows.clone() {
                let (w, b) = (with.chunk(cx, cy, cz), bare.chunk(cx, cy, cz));
                if w.same_packing(&b) {
                    continue;
                }
                for z in 0..CHUNK_SIZE {
                    for y in 0..CHUNK_SIZE {
                        for x in 0..CHUNK_SIZE {
                            if w.get(x, y, z) != b.get(x, y, z) {
                                let at =
                                    |c: i32, l: usize| (c * CHUNK_SIZE as i32) as f32 + l as f32;
                                changed.push([at(cx, x), at(cz, z)]);
                            }
                        }
                    }
                }
            }
        }
    }
    let distance = |p: &[f32; 2]| p[0] * p[0] + p[1] * p[1];
    let nearest = *changed
        .iter()
        .min_by(|a, b| distance(a).total_cmp(&distance(b)))
        .expect("nothing stood in the scan");
    let near: Vec<_> = changed
        .iter()
        .filter(|p| (p[0] - nearest[0]).abs() < 32.0 && (p[1] - nearest[1]).abs() < 32.0)
        .collect();
    let mean = [0, 1].map(|a| near.iter().map(|p| p[a]).sum::<f32>() / near.len() as f32);
    (mean.map(f32::floor), changed.len())
}

/// The 9 Chunks around the surface Chunk over `centre_m`.
fn ring(with: &Worldgen, centre_m: [f32; 2]) -> Vec<[i32; 3]> {
    let cell = CHUNK_SIZE as f32 * with.voxel_size();
    let y = (with.surface_height_m(centre_m[0], centre_m[1]) / cell).floor() as i32;
    let [x, z] = centre_m.map(|m| (m / cell).floor() as i32);
    (-1..=1)
        .flat_map(|dz| (-1..=1).map(move |dx| [x + dx, y, z + dz]))
        .collect()
}

fn timed<'a>(name: &'a str, generator: &'a Worldgen, ring: &'a [[i32; 3]]) -> Variant<'a> {
    (
        name,
        Box::new(move || {
            for &[x, y, z] in ring {
                black_box(generator.chunk(x, y, z));
            }
        }),
    )
}

/// The 9 Chunks from a new generator, which lays out every lattice cell they reach afresh.
fn cold<'a>(name: &'a str, make: impl Fn() -> Worldgen + 'a, ring: &'a [[i32; 3]]) -> Variant<'a> {
    (
        name,
        Box::new(move || {
            let generator = make();
            for &[x, y, z] in ring {
                black_box(generator.chunk(x, y, z));
            }
        }),
    )
}

const SIZES: [f32; 3] = [1.0, 0.5, 0.25];
const ROUNDS: usize = 101;

fn surface() {
    // Each kind with the bare World it adds to.
    let kinds: [(&str, Make, Make); 7] = [
        (
            "house, sparse (64 m)",
            Box::new(|s| with_structure(s, 0, 64.0)),
            Box::new(bare),
        ),
        (
            "house, dense (14 m)",
            Box::new(|s| with_structure(s, 0, 14.0)),
            Box::new(bare),
        ),
        (
            "hall at the cap (128 m)",
            Box::new(|s| with_structure(s, 3, 128.0)),
            Box::new(bare),
        ),
        (
            "in a village (32 m, 12 Plots)",
            Box::new(|s| with_settlement(s, settlement("t:village", 32.0, 12))),
            Box::new(bare),
        ),
        (
            "in a town (64 m, 24 Plots)",
            Box::new(|s| with_settlement(s, settlement("t:town", 64.0, 24))),
            Box::new(bare),
        ),
        (
            "in a town at the cap (160 m, 96 Plots)",
            Box::new(|s| with_settlement(s, settlement("t:city", 160.0, 96))),
            Box::new(bare),
        ),
        (
            "bridge over a valley (96 m)",
            Box::new(with_bridge),
            Box::new(bare_valleys),
        ),
    ];
    let centres: Vec<[f32; 2]> = kinds
        .iter()
        .map(|(name, make, plain)| {
            let (centre, changed) = centre_m(&make(1.0), &plain(1.0));
            println!("{name}: {changed} Voxels changed at 1 m within 384 m, centred on {centre:?}");
            centre
        })
        .collect();
    for size in SIZES {
        let generators: Vec<Worldgen> = kinds.iter().map(|(_, make, _)| make(size)).collect();
        let plains: Vec<Worldgen> = kinds.iter().map(|(_, _, plain)| plain(size)).collect();
        let rings: Vec<Vec<[i32; 3]>> = centres
            .iter()
            .zip(&plains)
            .map(|(&c, plain)| ring(plain, c))
            .collect();
        let cold_names: Vec<String> = kinds.iter().map(|(n, ..)| format!("{n}, cold")).collect();
        let mut variants = Vec::new();
        for (i, ((name, make, plain), with)) in kinds.iter().zip(&generators).enumerate() {
            variants.push(timed("bare", &plains[i], &rings[i]));
            variants.push(timed(name, with, &rings[i]));
            variants.push(cold("bare, cold", move || plain(size), &rings[i]));
            variants.push(cold(&cold_names[i], move || make(size), &rings[i]));
        }
        compare(
            &format!("worldgen of 9 surface Chunks, per Chunk, {size} m"),
            ROUNDS,
            1,
            9.0,
            &mut variants,
        );
    }
}

/// Sky above everything a Structure reaches and rock below it, where the generator answers
/// before sampling the ground.
fn early_outs() {
    for size in SIZES {
        let plain = bare(size);
        let town = with_settlement(size, settlement("t:town", 64.0, 24));
        let cap = with_structure(size, 3, 128.0);
        let cell = CHUNK_SIZE as f32 * size;
        let (low, high) = cap.surface_bounds_m();
        let (sky, deep) = (
            (high / cell).floor() as i32 + 1,
            ((low - plain.params().soil_depth_m) / cell).floor() as i32 - 2,
        );
        let sky_ring: Vec<[i32; 3]> = (0..9).map(|i| [i % 3, sky, i / 3]).collect();
        let deep_ring: Vec<[i32; 3]> = (0..9).map(|i| [i % 3, deep, i / 3]).collect();
        compare(
            &format!("sky and deep-rock Chunks, per Chunk, {size} m"),
            ROUNDS,
            20,
            9.0,
            &mut [
                timed("sky, bare", &plain, &sky_ring),
                timed("sky, hall at the cap", &cap, &sky_ring),
                timed("sky, town at the cap", &town, &sky_ring),
                timed("deep rock, bare", &plain, &deep_ring),
                timed("deep rock, hall at the cap", &cap, &deep_ring),
                timed("deep rock, town at the cap", &town, &deep_ring),
            ],
        );
    }
}

/// The bare World alone, at the origin, as a build without Structures generates it.
fn bare_only() {
    for size in SIZES {
        let plain = bare(size);
        let rows = ring(&plain, [0.0, 0.0]);
        compare(
            &format!("bare worldgen of 9 surface Chunks, per Chunk, {size} m"),
            ROUNDS,
            3,
            9.0,
            &mut [timed("bare", &plain, &rows)],
        );
    }
}

fn main() {
    println!("{}", load());
    if std::env::var_os("TESSERAE_BENCH_BARE").is_some() {
        bare_only();
    } else {
        surface();
        early_outs();
    }
    println!("{}", load());
}
