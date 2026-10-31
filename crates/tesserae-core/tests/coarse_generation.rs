//! Coarse generation is derived data: a change that makes it cheaper must make
//! the same Chunks. These digests were recorded before any such change, from
//! the Server's sources for levels 1 to 3 (`LodGenerators::new`), over a box
//! that crosses rock, caves, the surface, the lake and the sky.

use sha2::{Digest, Sha256};
use tesserae_core::testgen::fixtures;
use tesserae_core::worldgen::WaterParams;
use tesserae_core::{CHUNK_SIZE, Chunk, Voxel, Worldgen, WorldgenParams, downsample};

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

fn put(digest: &mut Sha256, v: Voxel) {
    digest.update(v.material().to_le_bytes());
    digest.update(v.shape().to_le_bytes());
    digest.update([v.orientation()]);
}

fn hash(digest: &mut Sha256, chunk: &Chunk) {
    match chunk.uniform() {
        Some(v) => {
            digest.update([1]);
            put(digest, v);
        }
        None => {
            digest.update([0]);
            chunk.kinds().iter().for_each(|&v| put(digest, v));
            for z in 0..CHUNK_SIZE {
                for y in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        put(digest, chunk.get(x, y, z));
                    }
                }
            }
        }
    }
}

/// Every child and every coarse Chunk of a level over the box, in order.
fn level_digest(seed: u64, level: u32, caves: bool) -> String {
    let source = source(seed, level, caves);
    let shapes = &fixtures().tables.shapes;
    let cell_m = CHUNK_SIZE as f32 * 2f32.powi(level as i32);
    let (lowest, highest) = (-24.0f32, 48.0f32);
    let rows = (lowest / cell_m).floor() as i32..=(highest / cell_m).floor() as i32;
    let mut digest = Sha256::new();
    for cy in rows {
        for cz in -1..=1 {
            for cx in -1..=1 {
                let children: [Chunk; 8] = std::array::from_fn(|i| {
                    source.chunk(
                        2 * cx + (i & 1) as i32,
                        2 * cy + (i >> 1 & 1) as i32,
                        2 * cz + (i >> 2) as i32,
                    )
                });
                children.iter().for_each(|c| hash(&mut digest, c));
                hash(
                    &mut digest,
                    &downsample(std::array::from_fn(|i| &children[i]), shapes),
                );
            }
        }
    }
    format!("{:x}", digest.finalize())
}

#[test]
fn coarse_generation_makes_the_recorded_chunks() {
    let digests: Vec<String> = SEEDS
        .iter()
        .flat_map(|&seed| (1..=3).map(move |level| level_digest(seed, level, true)))
        .collect();
    assert_eq!(digests, RECORDED);
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
    "98c2b7a7339a8a9a27026b11f10c7b7d7a6f3ff6df44677a62995e626a1653ba",
    "6c85e4de887343eb19cb93a9ed4f9dde6425f5b230edfbc1ff36f92d7db3894e",
    "b72260c8135e0d9fbfc2dc7d35c70276fc591030ef6113d012def23c4ab7e11e",
    "77ec008a50ece99498a033c18593d7dc55ff33cb17ffe706398d1487d08d1e06",
    "b0406c89466629eb046c391631deab65c647cbbf31a98bbaf900cce7aa338c57",
    "77b46bc33e7dbd3e066ae5b922e3cc230351900ef98ead899ca8b4020b2b6db4",
    "ea1a2afd94daf783aa71c80d9ab670e95a3754a3ba379269cf420b151393e558",
    "bc2806f705d1f542fac132f127584979c0e9ec23124ef92856ecabb2ea639ac8",
    "0cc8e5edcf2eeb58024dff406a1b65ad8c92af1fe38e723e365c5e5e4d278fff",
];
