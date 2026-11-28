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
    "5874ccd471c21198dbc29caacb6556b43fabf3320ac2e79a382edb835717500e",
    "40c775097b2cdac00ee34380eaf5cc4343d10367249f607e4ee97c47b0550bf3",
    "34320f7639c78c1aefdd4c5a6bd3d744019e861cecb7d0a3f62bf9616aa9f2ef",
    "66ce2016d15dc7a51d903b3a99bc0c1f52d40dec4bbbf8e27eb256cc19be494e",
    "e574a92f7e2e757ca4292117f21e20398c6bb27bc4774a0c650c547f4ab56a2f",
    "d4094a048af36fb74425d6ae835caa8d4e61f39a80453474831e0f034d63254d",
    "e5763eb27c86ff741e0fa32c14d9766ebd28078a45b213119e702fca1c095119",
    "e0026fd30850f50644a3f1217d2337737b5dca782b7e701d4c4e9c5a29bc6cc4",
    "8f769b94087ec854c345211fe90bbdb1391ac98066e14726c329be401438611a",
];
