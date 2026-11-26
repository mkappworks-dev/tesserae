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
    "fc57beb73c7d2e07773c3808af538cbfc06779a8c5c39d6dc0ec986e2678c225",
    "a3b5250f729a3c9bae3fce6fc4af87132002c11a02a444b550243e4ebe846dc5",
    "48f8181467907e50ad3b90b3d1cb64df3a0b93b4c318f7bb38a5d9fb5085a11a",
    "a1cd2f541c39a52374580d47555e45b9c6b327e61cfd9101a46e8d3623ca1e1a",
    "1e4e8e3212ff88d1702293fbba19aeb690c8b7d65a57db832e2bfea6f0c95db9",
    "7902857de7af0e641dc21fbac66f3caf9e43962eb3b4221514f63bbd7d6c510c",
    "6c076586879cc4bf7ad6f0b03fdd646eed30195ec4628bc68c8cddff79102c9d",
    "2bb21242a0c11ea921cf2989480da6d92af2f945385800f6f6109fde4f97d61d",
    "a85cacc1c46240e7b9c73887891b837f2a1cd18504ed5167ddb8f4dff5657cef",
];
