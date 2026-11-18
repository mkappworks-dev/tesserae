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

fn at_version(generator: u32) -> WorldgenParams {
    WorldgenParams {
        generator,
        ..params()
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

fn digests(generator: u32) -> Vec<String> {
    SEEDS
        .iter()
        .flat_map(|&seed| {
            VOXEL_SIZES
                .iter()
                .map(move |&size| digest(&Worldgen::new(seed, size, at_version(generator))))
        })
        .collect()
}

#[test]
fn version_1_makes_the_recorded_chunks() {
    assert_eq!(digests(1), RECORDED);
}

#[test]
fn version_2_with_no_biomes_equals_version_1() {
    assert_eq!(digests(2), RECORDED);
}

/// A version 2 World keeps its own Chunks now that a new World is version 3.
#[test]
fn version_3_is_not_version_2() {
    assert_ne!(digests(3), RECORDED);
}

#[test]
fn a_new_generator_defaults_to_the_current_version() {
    assert_eq!(WorldgenParams::default().generator, 3);
}

/// Without this the digests could be of a box no tunnel or lake reaches.
#[test]
fn the_box_holds_caves_and_water() {
    for seed in SEEDS {
        let full = digest(&Worldgen::new(seed, 1.0, at_version(2)));
        let no_caves = WorldgenParams {
            caves: None,
            ..at_version(2)
        };
        let no_water = WorldgenParams {
            water: None,
            ..at_version(2)
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

/// A box over version 3's high ground, found on a coarse walk, where its mountains and crags are:
/// the terrain whose arithmetic must round alike on every platform.
fn mountain_digest(generator: &Worldgen) -> String {
    let chunk_m = CHUNK_SIZE as f32 * generator.voxel_size();
    let (x, z) = (-40..40)
        .flat_map(|i| (-40..40).map(move |j| (i as f32 * 97.0, j as f32 * 89.0)))
        .find(|&(x, z)| generator.surface_height_m(x, z) > 90.0)
        .expect("a mountain");
    let h = generator.surface_height_m(x, z);
    let [cx, cz] = [x, z].map(|m| (m / chunk_m).floor() as i32);
    let rows = ((h - 40.0) / chunk_m).floor() as i32..=((h + 8.0) / chunk_m).floor() as i32;
    let reach = (24.0 / chunk_m).ceil() as i32;
    let mut digest = Sha256::new();
    for cy in rows {
        for dz in -reach..=reach {
            for dx in -reach..=reach {
                hash(&mut digest, &generator.chunk(cx + dx, cy, cz + dz));
            }
        }
    }
    digest
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[test]
fn version_3_makes_the_recorded_chunks() {
    assert_eq!(digests(3), RECORDED_V3);
}

#[test]
fn version_3_high_ground_makes_the_recorded_chunks() {
    let made: Vec<String> = SEEDS
        .iter()
        .flat_map(|&seed| {
            VOXEL_SIZES
                .iter()
                .map(move |&size| mountain_digest(&Worldgen::new(seed, size, at_version(3))))
        })
        .collect();
    assert_eq!(made, RECORDED_V3_HIGH_GROUND);
}

/// Seed by seed, Voxel sizes 1.0 then 0.5, recorded when version 3 became the default.
const RECORDED_V3: [&str; 6] = [
    "d32a1f39d58ffc2d24aed96a17be69a8b733d125e9d71f6286e0d4d95723a18f",
    "ea9cfe26fa6fb73bc97dd6ee376e86aa1e23308e1a7bd218b10b02149ef5319e",
    "41f49b58fa581d660964fe68cddded7111c1b502e1a7edeaeb1eb7350d25e544",
    "660d62813c2afda2bfb4c29ee7ad09869ff0ed3a4ff7a380cdcb1015c14c0cc0",
    "2133246b0ad95656e25f9ad142c4660fe3b87fc7d2bc47b1378653f07376b785",
    "2a3f04ee201eff445cdc79ebf440d27b94b843b1ac79644a82c92d386c9b84a4",
];

const RECORDED_V3_HIGH_GROUND: [&str; 6] = [
    "d63e72fa06046418db49e380f08c47ebd92923242dd6eac213dfb1baf7d0121f",
    "0e08a079a56249e4fadfba5139800dc5e3fedd610269a4d6f610463053e2e47d",
    "cc1c576b00b9dd62dd692d99ca3805702a2f93ab1ca5726e2610bbe618450df8",
    "065a43afdfab4c37686f04a47b49c2fc96563cfe4f9a7002e34d796ebc75afb4",
    "953d165751df3f52f0011f3d7c550f7ae7267dd0a24446ea6e984d580ef799cf",
    "9cfe5be297cf04ee308ae2ff0f5d1d2dddd3794d8ee67af6632a14185601718f",
];
