//! A World keeps its terrain for as long as it keeps its Generator version.
//! These digests are version 3's, over a box that crosses rock, caves, the
//! surface, smoothed slopes, the sea and the sky, at two Voxel sizes; and over
//! boxes on each kind of content it places, with the spawn point.

#![allow(
    clippy::disallowed_methods,
    reason = "a test's own maths is not generation"
)]

mod common;
mod digest;

use common::{CASES, chunk_span, water_near};
use sha2::{Digest, Sha256};
use tesserae_core::material::MaterialDef;
use tesserae_core::plant::{Canopies, Storey, Storeys};
use tesserae_core::save::Mapping;
use tesserae_core::testgen::{
    Fixtures, content, fixtures, register_content, register_glass, register_ground,
    register_lantern, register_leaves, register_water, world_params,
};
use tesserae_core::worldgen::WaterParams;
use tesserae_core::{
    Biome, BiomeName, BiomeTable, CHUNK_SIZE, Chunk, ChunkOverlay, Fluid, GroundParams, MaterialId,
    PlantTable, SHAPE_FULL_CUBE, SurfaceFeature, SurfaceFeatures, SurfacePlant, SurfacePlantKind,
    SurfaceTree, Tables, TreeVariety, Worldgen, WorldgenParams,
};

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
    let mut names = fixtures().mapping();
    names.materials.insert(WOOD, "t:wood".to_owned());
    names
        .materials
        .insert(DARK_LEAVES, "t:dark_leaves".to_owned());
    names.materials.insert(PALE_WOOD, "t:pale_wood".to_owned());
    names
}

/// The fixtures again, behind a Material they do not use and each in another order, as another
/// set of Mods would load them.
fn reordered() -> Fixtures {
    let mut tables = Tables::builtin();
    tables.materials.register(MaterialDef::OPAQUE).unwrap();
    let content = register_content(&mut tables);
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
        content,
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
                digest::chunk(&mut digest, names, &generator.chunk(cx, cy, cz));
            }
        }
    }
    digest::hex(digest)
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
                digest::chunk(&mut digest, names, &generator.chunk(cx + dx, cy, cz + dz));
            }
        }
    }
    digest::hex(digest)
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
    "1c036f51e6ca33fe9932daebf73c2c51003ed48eb6018dbe3c50b92db4fdb576",
    "df143051397417a0e8916a02eb4030b8947b8a31717c0fb72487e867c0e4581b",
    "f0270d679918566778d4f0004799f0f1af5960570f9c335b53d4fc26053bee3b",
    "44e0402afe6124bb3f702ff90d37becd443552003432be6098546cab03d05d3a",
];

/// The forest's trunks. Unregistered, since generation never asks the tables; [`names`] names it.
const WOOD: MaterialId = 200;

/// A second canopy variant, unregistered like [`WOOD`].
const DARK_LEAVES: MaterialId = 201;

/// A second species' trunks, unregistered like [`WOOD`].
const PALE_WOOD: MaterialId = 202;

fn forest_tree() -> SurfaceTree {
    SurfaceTree {
        trunk: WOOD,
        storeys_index: 0,
        canopies_index: 0,
        canopy: fixtures().leaves,
        trunk_height_m: (3.0, 5.0),
        canopy_radius_m: 2.5,
        fruit: None,
        vines: None,
        branching: None,
        crowding_m: None,
    }
}

/// The fixture ground and sea under one Biome of trees close enough to crowd, and no Settlement,
/// so a Player spawns on the dry land nearest the origin.
fn forest() -> WorldgenParams {
    forest_of(forest_tree(), &TreeVariety::NONE)
}

/// The forest's trees in two storeys, crowding less than their crowns so the storeys contest
/// Voxels, under two canopy variants.
fn storeyed_forest() -> WorldgenParams {
    let storey = |weight, trunk_height_m, canopy_radius_m| Storey {
        weight,
        trunk_height_m,
        canopy_radius_m,
    };
    let mut variety = TreeVariety::NONE;
    let tree = variety
        .freeze(
            SurfaceTree {
                crowding_m: Some(2.0),
                ..forest_tree()
            },
            &Canopies::new(&[fixtures().leaves, DARK_LEAVES]).unwrap(),
            &Storeys::new(&[storey(0.3, (6.0, 8.0), 3.0), storey(0.7, (3.0, 5.0), 2.0)]).unwrap(),
        )
        .unwrap();
    forest_of(tree, &variety)
}

/// The forest's trees and a taller second species without storeys in one Biome, crowding less
/// than their crowns so the species contest Voxels.
fn mixed_forest() -> WorldgenParams {
    let feature = |tree: SurfaceTree| SurfaceFeature {
        material: tree.trunk,
        spacing_m: 5.0,
        plant: Some(SurfacePlant {
            shape: SHAPE_FULL_CUBE,
            kind: SurfacePlantKind::Tree(tree),
        }),
        clump: None,
        field: None,
    };
    let short = SurfaceTree {
        crowding_m: Some(2.0),
        ..forest_tree()
    };
    let tall = SurfaceTree {
        trunk: PALE_WOOD,
        canopy: DARK_LEAVES,
        trunk_height_m: (6.0, 8.0),
        canopy_radius_m: 3.0,
        ..short
    };
    WorldgenParams {
        biomes: BiomeTable::new(&[Biome {
            name: BiomeName::new("t:mixed_forest").unwrap(),
            temperature: 0.5,
            humidity: 0.5,
            ground: None,
            sky_fog: None,
            cave_scale: None,
            cloud_cover: None,
            surface_features: SurfaceFeatures::new(&[feature(short), feature(tall)]).unwrap(),
        }])
        .unwrap(),
        ..world_params()
    }
}

fn forest_of(tree: SurfaceTree, variety: &TreeVariety) -> WorldgenParams {
    let tree = SurfaceFeature {
        material: WOOD,
        spacing_m: 6.0,
        plant: Some(SurfacePlant {
            shape: SHAPE_FULL_CUBE,
            kind: SurfacePlantKind::Tree(tree),
        }),
        clump: None,
        field: None,
    };
    WorldgenParams {
        biomes: BiomeTable::new_with_variety(
            &[Biome {
                name: BiomeName::new("t:forest").unwrap(),
                temperature: 0.5,
                humidity: 0.5,
                ground: None,
                sky_fog: None,
                cave_scale: None,
                cloud_cover: None,
                surface_features: SurfaceFeatures::new(&[tree]).unwrap(),
            }],
            variety,
        )
        .unwrap(),
        ..world_params()
    }
}

/// A content box: Chunks with the Overlay the Plant table derives for each.
type Boxed = Vec<(Chunk, ChunkOverlay)>;

fn overlay_of(g: &Worldgen, plants: &PlantTable, chunk: &Chunk) -> ChunkOverlay {
    plants.generated_overlay(chunk, g.params().water.map(|w| w.material))
}

/// The Chunks within `half_m` of `centre_m` across, from 16 m under its ground to 16 m over it.
fn boxed(g: &Worldgen, plants: &PlantTable, centre_m: [f32; 2], half_m: f32) -> Boxed {
    let chunk_m = CHUNK_SIZE as f32 * g.voxel_size();
    let ground_m = g.surface_height_m(centre_m[0], centre_m[1]);
    let mut chunks = Vec::new();
    for cy in chunk_span(ground_m, 16.0, chunk_m) {
        for cz in chunk_span(centre_m[1], half_m, chunk_m) {
            for cx in chunk_span(centre_m[0], half_m, chunk_m) {
                let chunk = g.chunk(cx, cy, cz);
                let overlay = overlay_of(g, plants, &chunk);
                chunks.push((chunk, overlay));
            }
        }
    }
    chunks
}

fn holds(boxed: &Boxed, material: MaterialId) -> bool {
    boxed
        .iter()
        .any(|(c, _)| c.kinds().iter().any(|v| v.material() == material))
}

/// Whether `material` stands in a Voxel whose generated Overlay is `fluid`.
fn stands_in(chunk: &Chunk, overlay: &ChunkOverlay, material: MaterialId, fluid: Fluid) -> bool {
    voxels().any(|[x, y, z]| {
        chunk.get(x, y, z).material() == material && overlay.get(x, y, z).fluid == Some(fluid)
    })
}

fn voxels() -> impl Iterator<Item = [usize; 3]> {
    (0..CHUNK_SIZE)
        .flat_map(|z| (0..CHUNK_SIZE).flat_map(move |y| (0..CHUNK_SIZE).map(move |x| [x, y, z])))
}

/// The centre of the first Chunk, swept out from `from` ring by ring within 16 m of its ground,
/// that `found` accepts; the content's position, without API for a test.
fn sweep(
    g: &Worldgen,
    plants: &PlantTable,
    from: [f32; 2],
    found: impl Fn(&Chunk, &ChunkOverlay) -> bool,
) -> [f32; 2] {
    let chunk_m = CHUNK_SIZE as f32 * g.voxel_size();
    let [fx, fz] = from.map(|m| (m / chunk_m).floor() as i32);
    let centre = |c: i32| (c as f32 + 0.5) * chunk_m;
    (0..(256.0 / chunk_m) as i32)
        .flat_map(|r| {
            (-r..=r).flat_map(move |dz| {
                (-r..=r)
                    .filter(move |&dx| dx.abs() == r || dz.abs() == r)
                    .map(move |dx| [fx + dx, fz + dz])
            })
        })
        .find(|&[cx, cz]| {
            let ground_m = g.surface_height_m(centre(cx), centre(cz));
            chunk_span(ground_m, 16.0, chunk_m).any(|cy| {
                let chunk = g.chunk(cx, cy, cz);
                found(&chunk, &overlay_of(g, plants, &chunk))
            })
        })
        .map(|c| c.map(centre))
        .expect("the content within 256 m")
}

/// Each Voxel's generated Overlay in box order: a 0 where it holds no fluid, else a 1, the fluid's
/// name, its source flag and each void's level. Generation never sets fire.
fn hash_overlay(digest: &mut Sha256, names: &Mapping, overlay: &ChunkOverlay) {
    for [x, y, z] in voxels() {
        let Some(fluid) = overlay.get(x, y, z).fluid else {
            digest.update([0]);
            continue;
        };
        digest.update([1]);
        digest::name(
            digest,
            names.materials.get(&fluid.material).expect("a named fluid"),
        );
        digest.update([u8::from(fluid.source)]);
        for level in fluid.levels() {
            digest.update([level.surface]);
            digest.update(level.reach.to_le_bytes());
            digest.update(level.descent.to_le_bytes());
        }
    }
}

fn box_digest(boxed: &Boxed) -> String {
    let names = names();
    let mut digest = Sha256::new();
    for (chunk, overlay) in boxed {
        digest::chunk(&mut digest, &names, chunk);
        hash_overlay(&mut digest, &names, overlay);
    }
    digest::hex(digest)
}

/// Each box is checked for its content before any digest is compared, so none can pass over bare
/// ground.
fn case_digests(make: impl Fn(u64, f32) -> (Boxed, String)) -> Vec<String> {
    CASES
        .iter()
        .map(|&(seed, size)| {
            let (boxed, missing) = make(seed, size);
            assert!(missing.is_empty(), "{seed:x} at {size} m: no {missing}");
            box_digest(&boxed)
        })
        .collect()
}

fn missing(checks: &[(bool, &str)]) -> String {
    checks
        .iter()
        .filter(|(held, _)| !held)
        .map(|(_, what)| *what)
        .collect::<Vec<_>>()
        .join(", ")
}

#[test]
fn the_settlement_a_player_spawns_in_makes_the_recorded_chunks() {
    let m = &fixtures().content;
    let made = case_digests(|seed, size| {
        let g = content().worldgen(seed, size);
        let reach_m = content().settlements[0].reach_m();
        let b = boxed(&g, &content().plants, g.spawn_m(), reach_m);
        let checks = [(holds(&b, m.hall), "building"), (holds(&b, m.road), "road")];
        (b, missing(&checks))
    });
    assert_eq!(made, RECORDED_SETTLEMENT);
}

#[test]
fn the_structure_nearest_spawn_makes_the_recorded_chunks() {
    let stone = fixtures().content.stone;
    let made = case_digests(|seed, size| {
        let g = content().worldgen(seed, size);
        let plants = &content().plants;
        let at = sweep(&g, plants, g.spawn_m(), |c, _| {
            c.kinds().iter().any(|v| v.material() == stone)
        });
        let b = boxed(&g, plants, at, 8.0);
        let checks = [(holds(&b, stone), "Structure")];
        (b, missing(&checks))
    });
    assert_eq!(made, RECORDED_STRUCTURE);
}

#[test]
fn the_column_plant_in_water_makes_the_recorded_chunks_and_overlay() {
    let reed = fixtures().content.reed;
    let wet = Fluid::source(fixtures().water);
    let made = case_digests(|seed, size| {
        let g = content().worldgen(seed, size);
        let plants = &content().plants;
        let at = sweep(&g, plants, water_near(&g, g.spawn_m()), |c, o| {
            stands_in(c, o, reed, wet)
        });
        let b = boxed(&g, plants, at, 8.0);
        let checks = [(
            b.iter().any(|(c, o)| stands_in(c, o, reed, wet)),
            "Plant in water",
        )];
        (b, missing(&checks))
    });
    assert_eq!(made, RECORDED_PLANT);
}

#[test]
fn the_forest_a_player_spawns_in_makes_the_recorded_chunks() {
    let made = case_digests(|seed, size| {
        let g = Worldgen::new(seed, size, forest());
        let b = boxed(&g, &PlantTable::default(), g.spawn_m(), 16.0);
        let checks = [
            (holds(&b, WOOD), "trunk"),
            (holds(&b, fixtures().leaves), "canopy"),
        ];
        (b, missing(&checks))
    });
    assert_eq!(made, RECORDED_FOREST);
}

#[test]
fn the_storeyed_forest_makes_the_recorded_chunks() {
    let made = case_digests(|seed, size| {
        let g = Worldgen::new(seed, size, storeyed_forest());
        let b = boxed(&g, &PlantTable::default(), g.spawn_m(), 16.0);
        let checks = [
            (holds(&b, WOOD), "trunk"),
            (holds(&b, fixtures().leaves), "canopy"),
            (holds(&b, DARK_LEAVES), "canopy variant"),
        ];
        (b, missing(&checks))
    });
    assert_eq!(made, RECORDED_STOREYED_FOREST);
}

/// Only storeyed rows stand the taller first where crowns meet: two species without storeys keep
/// the order they generated in before storeys, so these are 0b03c594's chunks.
#[test]
fn two_species_without_storeys_make_the_chunks_they_made_before_storeys() {
    let made = case_digests(|seed, size| {
        let g = Worldgen::new(seed, size, mixed_forest());
        let b = boxed(&g, &PlantTable::default(), g.spawn_m(), 16.0);
        let checks = [
            (holds(&b, WOOD), "first trunk"),
            (holds(&b, PALE_WOOD), "second trunk"),
            (holds(&b, fixtures().leaves), "first canopy"),
            (holds(&b, DARK_LEAVES), "second canopy"),
        ];
        (b, missing(&checks))
    });
    assert_eq!(made, RECORDED_MIXED_FOREST);
}

/// In the content's hamlet, then on the forest's dry land, which a Player looks for only because
/// the origin is sea.
#[test]
fn a_player_spawns_where_recorded() {
    let made: Vec<[[u32; 2]; 2]> = CASES
        .iter()
        .map(|&(seed, size)| {
            let forest = Worldgen::new(seed, size, forest());
            assert!(forest.water_surface_m(0.0, 0.0).is_some(), "{seed:x}");
            [content().worldgen(seed, size).spawn_m(), forest.spawn_m()]
                .map(|p| p.map(f32::to_bits))
        })
        .collect();
    let recorded: Vec<[[u32; 2]; 2]> = RECORDED_SPAWN
        .iter()
        .map(|c| c.map(|p| p.map(f32::to_bits)))
        .collect();
    assert_eq!(made, recorded);
}

/// The content digests, case by case, recorded on macOS arm64.
const RECORDED_SETTLEMENT: [&str; 4] = [
    "4c3bed3bdf258509af3cdaf5718d89c701a9b67a4d12cbde577750b6712cb141",
    "d2f0122fcd542eef39517a5e69358abee348aa9b9a9d2bc5cd5fe45dd115f7c1",
    "014596455645e5e30040affcaef21807a8cce2b1ef5a1e2c9ff03f357ba3b34a",
    "1423b450b3c40e46172ce4b30d2add6c85786e669eca6388d9fcbbb1b3d2966b",
];

const RECORDED_STRUCTURE: [&str; 4] = [
    "a67fdb05d35498ea46216ec509e7fa11a673da838a810968c59c01f529dd7ce6",
    "2556c024eb01fed68ecf917c11b553dfcee2ace83508d31c56b534cbecbb4016",
    "0c5a7a58831979cb7efd5357c51cad66112b44002b9404c16faaf53b682c1821",
    "1f0a38614024300a6cb3aa27bc91299f11b1ed55b0be668a842efe1671347cdc",
];

const RECORDED_PLANT: [&str; 4] = [
    "debc2adff742e1944d542cc18ceb2538b5574058adf7b700ef00c8543ed34d8a",
    "8755dfbabe3ddf2d09f88ebcf55a17c2578e78282932852be580b1968450490d",
    "4315b5755ae17b1b18e17eacd411d91ac561930c33d672c64e914339ed9023b3",
    "370ab713501b0510f5b54bdae8ed94e4f30dbcc678a66d29bdf716b67455843c",
];

const RECORDED_FOREST: [&str; 4] = [
    "208e96faaaba0382cf0f8c97f854af1eef84fc08009456eea5f1b552837aa09a",
    "2cfb0213c80b50a85fe81127378521c803038e63dee8600e378ae2aca62ee34b",
    "5ced9a6a5d38f78a54d558498ff69c728c24b879450ffedae7ceaf47118649c0",
    "dbae335dfab55261370553635cfe9f03e6ec78aad8cfb980b612c480a0a10daa",
];

/// Recorded with storeys and canopy variants, on macOS arm64.
const RECORDED_STOREYED_FOREST: [&str; 4] = [
    "dc60515327b244703bf9b85064e50f1211a19114b4573a27181774c23cada099",
    "bce614fe180c9cfbd29d3e0f1d7d781f09ea8697a9952a82458972d1d771b4af",
    "d8e048ea8aee25d59e9f5b91f1a2fd98388db23413ad031aea2c3d37bbbd843b",
    "e5d39098a34865ce7aa27c25ba4a555d98ac6db1222a7497436a69c47ae18e5c",
];

/// Recorded on 0b03c594, before storeys, on macOS arm64.
const RECORDED_MIXED_FOREST: [&str; 4] = [
    "e84dcd6e9606a26104c32bffa3c3561152911c5b6191472324e1f093168e90b9",
    "bff2e32d9bbcd52f4bac5b29478037707ea336c3efd3136e951804ab918d0843",
    "ed6d4b962fe696dd022c548b4e05a1272fd8d37be7590505e08efbec29371a81",
    "889496d6ef6ba10661d721cd5b3277d327cf0b7f8111623000d60463ad1c0bc1",
];

/// The content's then the forest's, compared bit for bit.
const RECORDED_SPAWN: [[[f32; 2]; 2]; 4] = [
    [[-1031.0, 825.8], [-460.59995, 787.1415]],
    [[-1031.0, 825.8], [-460.59995, 787.1415]],
    [[-54.0, 1513.8], [-1802.0979, -938.86487]],
    [[-54.0, 1513.8], [-1802.0979, -938.86487]],
];
