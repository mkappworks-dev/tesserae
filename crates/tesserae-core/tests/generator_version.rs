//! A World keeps its terrain for as long as it keeps its Generator version.
//! These digests were recorded from the generator before it knew about
//! versions, over a box that crosses rock, caves, the surface, smoothed slopes,
//! the lake and the sky, at two Voxel sizes.

use sha2::{Digest, Sha256};
use tesserae_core::testgen::fixtures;
use tesserae_core::worldgen::WaterParams;
use tesserae_core::{CHUNK_SIZE, Chunk, Voxel, Worldgen, WorldgenParams};

const SEEDS: [u64; 3] = [0x7e55_e2ae_0000_0001, 1, 0xdead_beef_cafe_f00d];
const VOXEL_SIZES: [f32; 2] = [1.0, 0.5];

fn params() -> WorldgenParams {
    let defaults = tesserae_core::testgen::world_params();
    WorldgenParams {
        sea_level_m: 24.0,
        relief_m: 24.0,
        water: Some(WaterParams {
            level_m: 34.0,
            material: fixtures().water,
        }),
        ..defaults
    }
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

fn digest(generator: &Worldgen) -> String {
    let chunk_m = CHUNK_SIZE as f32 * generator.voxel_size();
    let rows = (-8.0 / chunk_m).floor() as i32..=(56.0 / chunk_m).floor() as i32;
    let reach = (64.0 / chunk_m).ceil() as i32;
    let mut digest = Sha256::new();
    for cy in rows {
        for cz in -reach..=reach {
            for cx in -reach..=reach {
                hash(&mut digest, &generator.chunk(cx, cy, cz));
            }
        }
    }
    digest
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn recorded_digests() -> Vec<String> {
    SEEDS
        .iter()
        .flat_map(|&seed| {
            VOXEL_SIZES
                .iter()
                .map(move |&size| digest(&Worldgen::new(seed, size, params())))
        })
        .collect()
}

#[test]
fn the_generator_makes_the_recorded_chunks() {
    assert_eq!(recorded_digests(), RECORDED);
}

/// Without this the digests could be of a box no tunnel or lake reaches.
#[test]
fn the_box_holds_caves_and_water() {
    for seed in SEEDS {
        let full = digest(&Worldgen::new(seed, 1.0, params()));
        let no_caves = WorldgenParams {
            caves: None,
            ..params()
        };
        let no_water = WorldgenParams {
            water: None,
            ..params()
        };
        assert_ne!(full, digest(&Worldgen::new(seed, 1.0, no_caves)));
        assert_ne!(full, digest(&Worldgen::new(seed, 1.0, no_water)));
    }
}

/// Seed by seed, Voxel sizes 1.0 then 0.5.
const RECORDED: [&str; 6] = [
    "d8a4bca29887a6f53033169f34a01430031890064be74a63a1aa53a90be0965a",
    "bd049c82f8b2e0cd6ea4466fdfb778f84eede09865515673bad07843a7941255",
    "94bf81fdade7d9907d1b6a2753d901cddb4c02412a5d1a93a848e6b61c62b9e1",
    "82ed0811e445278ff1ea82e6a574ec68647529b9f6965e386218ad96389d942e",
    "1a7dc5fbe16dc2596c1f4bb11c0929a5caa749c1e986d57a2b9bb2ae590e1cfa",
    "4966249f62949791068dfbb898b399bf2cf762878738b282812369358e8d93aa",
];
