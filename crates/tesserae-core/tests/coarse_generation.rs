//! Coarse generation is derived data: a change that makes it cheaper must make
//! the same Chunks. These digests were recorded before any such change, from
//! the Server's sources for levels 1 to 3 (`LodGenerators::new`), over a box
//! that crosses rock, caves, the surface, any water and the sky.

#![allow(
    clippy::disallowed_methods,
    reason = "a test's own maths is not generation"
)]

mod digest;

use sha2::{Digest, Sha256};
use tesserae_core::testgen::fixtures;
use tesserae_core::worldgen::WaterParams;
use tesserae_core::{CHUNK_SIZE, Chunk, Worldgen, WorldgenParams, downsample};

const SEEDS: [u64; 3] = [0x7e55_e2ae_0000_0001, 1, 0xdead_beef_cafe_f00d];

fn params(caves: bool) -> WorldgenParams {
    let defaults = tesserae_core::testgen::world_params();
    WorldgenParams {
        sea_level_m: 24.0,
        relief_m: 24.0,
        water: Some(WaterParams {
            level_m: 34.0,
            material: fixtures().water,
        }),
        caves: defaults.caves.filter(|_| caves),
        ..defaults
    }
}

fn source(seed: u64, level: u32, caves: bool) -> Worldgen {
    let params = if level == 1 {
        params(caves)
    } else {
        WorldgenParams {
            smoothing: None,
            ..params(caves)
        }
    };
    Worldgen::new(seed, 2f32.powi(level as i32 - 1), params)
}

/// The box's middle column along x and its floor and ceiling, in metres. Land lies anywhere in
/// the band, and under the sea a tunnel never opens, so the box is hung from the first ground east
/// of the origin standing well above Sea level.
fn box_m(seed: u64) -> (f32, f32, f32) {
    let g = source(seed, 1, true);
    let x_m = (0..)
        .map(|i| i as f32 * 128.0)
        .find(|&x| g.surface_height_m(x, 0.0) > g.params().sea_level_m + 10.0)
        .unwrap();
    let ground_m = g.surface_height_m(x_m, 0.0);
    (x_m, ground_m - 48.0, ground_m + 24.0)
}

/// Every child and every coarse Chunk of a level over the box, in order.
fn level_digest(seed: u64, level: u32, caves: bool) -> String {
    let source = source(seed, level, caves);
    let shapes = &fixtures().tables.shapes;
    let names = fixtures().mapping();
    let cell_m = CHUNK_SIZE as f32 * 2f32.powi(level as i32);
    let (x_m, lowest, highest) = box_m(seed);
    let middle = (x_m / cell_m).floor() as i32;
    let rows = (lowest / cell_m).floor() as i32..=(highest / cell_m).floor() as i32;
    let mut digest = Sha256::new();
    for cy in rows {
        for cz in -1..=1 {
            for cx in middle - 1..=middle + 1 {
                let children: [Chunk; 8] = std::array::from_fn(|i| {
                    source.chunk(
                        2 * cx + (i & 1) as i32,
                        2 * cy + (i >> 1 & 1) as i32,
                        2 * cz + (i >> 2) as i32,
                    )
                });
                children
                    .iter()
                    .for_each(|c| digest::chunk(&mut digest, &names, c));
                digest::chunk(
                    &mut digest,
                    &names,
                    &downsample(std::array::from_fn(|i| &children[i]), shapes),
                );
            }
        }
    }
    digest::hex(digest)
}

fn digests() -> Vec<String> {
    SEEDS
        .iter()
        .flat_map(|&seed| (1..=3).map(move |level| level_digest(seed, level, true)))
        .collect()
}

#[test]
fn coarse_generation_makes_the_recorded_chunks() {
    assert_eq!(digests(), RECORDED);
}

/// Without this the digests could be of a box no tunnel reaches.
#[test]
fn the_box_holds_caves_at_every_level_and_seed() {
    for seed in SEEDS {
        for level in 1..=3 {
            assert_ne!(
                level_digest(seed, level, true),
                level_digest(seed, level, false),
                "seed {seed:#x} level {level}"
            );
        }
    }
}

/// Seed by seed, levels 1 to 3.
const RECORDED: [&str; 9] = [
    "56fdd9256da3e9adf087ca33fc42a3aefee9e68343dc1aab9566045a640f0fd4",
    "caaa6dcf987d32def2885103f840d07b9614f29dd599b5b9c33f026a3f224f4e",
    "a396f316f40409ec8aa1240bee607b44eccd361b562d22f039904b9ddb1d4ed9",
    "49348a6d518ba777aeb27c100af00ed4f08ca0d9c7f7f4ac93735eb627a3360d",
    "25dc9999f41013cb298cd581429d9a803c9dcbc21c28ba7f297d81eb8cf71c83",
    "a756a47c6884cac57dec4c1eb22e845fc69c8d47b0319c99569a12a050e41381",
    "8e6ed67023d7d2a985557694ec1e4f6ad71cdf1eb8118fac74ab562b8ea6f0bf",
    "d09d15e4ad1f0b44b1f5f11dbcbe695bd417c0faa3cfe94f0e0fdbda45d58893",
    "871bd6b2370a005f3aed6f8d4d401023e5c929fe80a7167620ae81643ed0f8d4",
];
