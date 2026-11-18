//! Coarse generation is derived data: a change that makes it cheaper must make
//! the same Chunks. These digests were recorded before any such change, from
//! the Server's sources for levels 1 to 3 (`LodGenerators::new`), over a box
//! that crosses rock, caves, the surface, any water and the sky.

use sha2::{Digest, Sha256};
use tesserae_core::testgen::fixtures;
use tesserae_core::worldgen::WaterParams;
use tesserae_core::{CHUNK_SIZE, Chunk, Voxel, Worldgen, WorldgenParams, downsample};

const SEEDS: [u64; 3] = [0x7e55_e2ae_0000_0001, 1, 0xdead_beef_cafe_f00d];

fn params(generator: u32, caves: bool) -> WorldgenParams {
    let defaults = match generator {
        2 => tesserae_core::testgen::world_params(),
        _ => tesserae_core::testgen::world_params_v3(),
    };
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

fn source(seed: u64, generator: u32, level: u32, caves: bool) -> Worldgen {
    let params = if level == 1 {
        params(generator, caves)
    } else {
        WorldgenParams {
            smoothing: None,
            ..params(generator, caves)
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

/// The box's middle column along x and its floor and ceiling, in metres. Version 2's land lies
/// inside a fixed band at the origin; version 3's anywhere in its own, and under the sea a
/// tunnel never opens, so its box is hung from the first ground east of the origin standing
/// well above Sea level.
fn box_m(seed: u64, generator: u32) -> (f32, f32, f32) {
    if generator == 2 {
        return (0.0, -24.0, 48.0);
    }
    let g = source(seed, generator, 1, true);
    let x_m = (0..)
        .map(|i| i as f32 * 128.0)
        .find(|&x| g.surface_height_m(x, 0.0) > g.params().sea_level_m + 10.0)
        .unwrap();
    let ground_m = g.surface_height_m(x_m, 0.0);
    (x_m, ground_m - 48.0, ground_m + 24.0)
}

/// Every child and every coarse Chunk of a level over the box, in order.
fn level_digest(seed: u64, generator: u32, level: u32, caves: bool) -> String {
    let source = source(seed, generator, level, caves);
    let shapes = &fixtures().tables.shapes;
    let cell_m = CHUNK_SIZE as f32 * 2f32.powi(level as i32);
    let (x_m, lowest, highest) = box_m(seed, generator);
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
                children.iter().for_each(|c| hash(&mut digest, c));
                hash(
                    &mut digest,
                    &downsample(std::array::from_fn(|i| &children[i]), shapes),
                );
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
        .flat_map(|&seed| (1..=3).map(move |level| level_digest(seed, generator, level, true)))
        .collect()
}

#[test]
fn coarse_generation_makes_the_recorded_chunks() {
    assert_eq!(digests(3), RECORDED_V3);
}

#[test]
fn coarse_generation_makes_the_recorded_version_2_chunks() {
    assert_eq!(digests(2), RECORDED);
}

/// Without this the digests could be of a box no tunnel reaches.
#[test]
fn the_box_holds_caves_at_every_level_and_seed() {
    for generator in [2, 3] {
        for seed in SEEDS {
            for level in 1..=3 {
                assert_ne!(
                    level_digest(seed, generator, level, true),
                    level_digest(seed, generator, level, false),
                    "version {generator} seed {seed:#x} level {level}"
                );
            }
        }
    }
}

/// Seed by seed, levels 1 to 3.
const RECORDED_V3: [&str; 9] = [
    "0dac0ab4e3f47e573f129ee13edaa7e9b32b6085ee904fcd8941924640a768eb",
    "4143c6812bd9c83c95b95936e5b40ddddbecd16baecd0441ad8e838728f3de9b",
    "828c72d635866191045ea71815be1f73360b9d818defedfd30d1401b267c5077",
    "e721fa23701ccd30231d31b2aef6710d494b348df1451ff02ab18e57478285f5",
    "275dbb72503ece87f6dbca9379a0845e84fbdf2ff06c309753dc6ac972fc78c1",
    "231fbb8e7781ae9f47cc2fe1cd03a1a06da60158ef9daf1523559211ad66dc05",
    "4e9208c14abe8f725e335fed32544932fb5bc109efacae51a13eed3d5afe1081",
    "8b0871bd21530d3c966a53a5d87c5c60de3d3ba3002587a21a38a712ee2ae50a",
    "fc5d6ec5b8ec83281061b05989ffbe98d2abc007617196d64a2ee4a3c58dbf07",
];

/// Version 2's, seed by seed, levels 1 to 3.
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
