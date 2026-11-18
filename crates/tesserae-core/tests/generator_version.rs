//! A World keeps its terrain for as long as it keeps its Generator version.
//! These digests are version 3's, over a box that crosses rock, caves, the
//! surface, smoothed slopes, the sea and the sky, at two Voxel sizes.

use sha2::{Digest, Sha256};
use tesserae_core::material::MaterialDef;
use tesserae_core::save::Mapping;
use tesserae_core::testgen::{
    Fixtures, fixtures, register_glass, register_ground, register_lantern, register_leaves,
    register_water,
};
use tesserae_core::worldgen::WaterParams;
use tesserae_core::{CHUNK_SIZE, Chunk, GroundParams, Tables, Worldgen, WorldgenParams};

const SEEDS: [u64; 3] = [0x7e55_e2ae_0000_0001, 1, 0xdead_beef_cafe_f00d];
const VOXEL_SIZES: [f32; 2] = [1.0, 0.5];

/// The fixture ground alone, with none of the fixture Biomes these were recorded without.
fn params() -> WorldgenParams {
    params_of(fixtures())
}

fn params_of(f: &Fixtures) -> WorldgenParams {
    WorldgenParams {
        sea_level_m: 24.0,
        relief_m: 24.0,
        water: Some(WaterParams {
            level_m: 24.0,
            material: f.water,
        }),
        ground: Some(f.ground),
        ..WorldgenParams::default()
    }
}

fn names() -> Mapping {
    fixtures().mapping()
}

/// The fixtures again, behind a Material they do not use and each in another order, as another
/// set of Mods would load them.
fn reordered() -> Fixtures {
    let mut tables = Tables::builtin();
    tables.materials.register(MaterialDef::OPAQUE).unwrap();
    let water = register_water(&mut tables);
    let leaves = register_leaves(&mut tables);
    let glass = register_glass(&mut tables);
    let lantern = register_lantern(&mut tables);
    let ground = register_ground(&mut tables);
    Fixtures {
        tables,
        ground: GroundParams {
            rock: ground.surface,
            soil: ground.rock,
            surface: ground.soil,
        },
        lantern,
        glass,
        leaves,
        water,
    }
}

/// Every Voxel in box order as its Material name, Shape name and Orientation, so content
/// registered in another order makes the same digest.
fn hash(digest: &mut Sha256, names: &Mapping, chunk: &Chunk) {
    for z in 0..CHUNK_SIZE {
        for y in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let v = chunk.get(x, y, z);
                let material = names
                    .materials
                    .get(&v.material())
                    .expect("a named Material");
                let shape = names.shapes.get(&v.shape()).expect("a named Shape");
                for name in [material, shape] {
                    digest.update((name.len() as u32).to_le_bytes());
                    digest.update(name.as_bytes());
                }
                digest.update([v.orientation()]);
            }
        }
    }
}

fn digest(generator: &Worldgen, names: &Mapping) -> String {
    let chunk_m = CHUNK_SIZE as f32 * generator.voxel_size();
    let rows = (-8.0 / chunk_m).floor() as i32..=(56.0 / chunk_m).floor() as i32;
    let reach = (64.0 / chunk_m).ceil() as i32;
    let mut digest = Sha256::new();
    for cy in rows {
        for cz in -reach..=reach {
            for cx in -reach..=reach {
                hash(&mut digest, names, &generator.chunk(cx, cy, cz));
            }
        }
    }
    digest
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn digests() -> Vec<String> {
    SEEDS
        .iter()
        .flat_map(|&seed| {
            VOXEL_SIZES
                .iter()
                .map(move |&size| digest(&Worldgen::new(seed, size, params()), &names()))
        })
        .collect()
}

#[test]
fn a_new_generator_defaults_to_the_current_version() {
    assert_eq!(WorldgenParams::default().generator, 3);
}

/// Without this the digests could be of boxes no tunnel or sea reaches. The origin's sits at or
/// under Sea level, where no tunnel opens, so the high ground's holds the caves.
#[test]
fn the_boxes_hold_caves_and_water() {
    for seed in SEEDS {
        let g = |params| Worldgen::new(seed, 1.0, params);
        let no_caves = WorldgenParams {
            caves: None,
            ..params()
        };
        let no_water = WorldgenParams {
            water: None,
            ..params()
        };
        let names = names();
        assert_ne!(
            mountain_digest(&g(params()), &names),
            mountain_digest(&g(no_caves), &names)
        );
        assert_ne!(digest(&g(params()), &names), digest(&g(no_water), &names));
    }
}

/// A box over version 3's high ground, found on a coarse walk, where its mountains and crags are:
/// the terrain whose arithmetic must round alike on every platform.
fn mountain_digest(generator: &Worldgen, names: &Mapping) -> String {
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
                hash(&mut digest, names, &generator.chunk(cx + dx, cy, cz + dz));
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
fn content_registered_in_another_order_makes_the_recorded_chunks() {
    let f = &reordered();
    assert_ne!(f.water, fixtures().water);
    let names = &f.mapping();
    let made: Vec<[String; 2]> = SEEDS
        .iter()
        .flat_map(|&seed| {
            VOXEL_SIZES.iter().map(move |&size| {
                let g = Worldgen::new(seed, size, params_of(f));
                [digest(&g, names), mountain_digest(&g, names)]
            })
        })
        .collect();
    let recorded: Vec<[String; 2]> = RECORDED
        .iter()
        .zip(RECORDED_HIGH_GROUND)
        .map(|(a, b)| [a.to_string(), b.to_string()])
        .collect();
    assert_eq!(made, recorded);
}

#[test]
fn version_3_makes_the_recorded_chunks() {
    assert_eq!(digests(), RECORDED);
}

#[test]
fn version_3_high_ground_makes_the_recorded_chunks() {
    let made: Vec<String> = SEEDS
        .iter()
        .flat_map(|&seed| {
            VOXEL_SIZES
                .iter()
                .map(move |&size| mountain_digest(&Worldgen::new(seed, size, params()), &names()))
        })
        .collect();
    assert_eq!(made, RECORDED_HIGH_GROUND);
}

/// Seed by seed, Voxel sizes 1.0 then 0.5, recorded by name on macOS arm64.
const RECORDED: [&str; 6] = [
    "a4fd4e45002e6d42f0ae07f8621954dd30c57bad3b9d3e7584a330d758228206",
    "ebf20050e277ede4cefbea7b74827e4ab7e48325c8c2b71cc4b80a66cfb5b499",
    "593c469c29549daace8b653525371b75d8908e28c1bef334e52192adf8b832a3",
    "f1cd3f1aa34a942e1d9957e652976f6d4d7ee7fbf52bd0a747cd1e3e0da872af",
    "a9c87fa3863e38373f9130af4b73433c28a029132a904b07eec63875b494738b",
    "84f84bcc01850e9d34a116049bba82ae18d59fbeb1f2f10ed13d8de388d0123d",
];

const RECORDED_HIGH_GROUND: [&str; 6] = [
    "7a80e94176e2e757e7e2bbbcb177bd1486b213d92f4b52d6741b867b8a1a65a6",
    "befbf69f9d6199a7c80d8987ca55d247e30926e811b78b2f2d8cf404ddbceaf8",
    "a294e853ab69022edcb21a4acc16e61c8f66cab913008c5c8916ae19052c8472",
    "df143051397417a0e8916a02eb4030b8947b8a31717c0fb72487e867c0e4581b",
    "95d2a641d8ef992f71229012b2c5b2639cb304ae2ecfb77445b4938a70a9f936",
    "44e0402afe6124bb3f702ff90d37becd443552003432be6098546cab03d05d3a",
];
