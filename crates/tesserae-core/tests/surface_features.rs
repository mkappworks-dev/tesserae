//! Surface features: a Biome's Material scattered one Voxel above its ground.

#![allow(
    clippy::disallowed_methods,
    reason = "a test's own maths is not generation"
)]

use sha2::{Digest, Sha256};
use tesserae_core::save::{Generation, Mapping};
use tesserae_core::testgen::fixtures;
use tesserae_core::worldgen::WaterParams;
use tesserae_core::{
    AIR, Biome, BiomeId, BiomeName, BiomeSet, BiomeTable, CHUNK_SIZE, ClimateParams, GroundParams,
    MaterialId, Ore, OreTable, SurfaceFeature, SurfaceFeatures, Voxel, Worldgen, WorldgenParams,
};

/// Generation never asks the tables, so ids no fixture uses mark the features apart.
const MELON: MaterialId = 200;
const GOURD: MaterialId = 201;

fn feature(material: MaterialId, spacing_m: f32) -> SurfaceFeature {
    SurfaceFeature::cube(material, spacing_m)
}

fn featured(biome: Biome, features: &[SurfaceFeature]) -> Biome {
    Biome {
        surface_features: SurfaceFeatures::new(features).unwrap(),
        ..biome
    }
}

mod common;
mod digest;

const SEEDS: [u64; 2] = [0x7e55_e2ae_0000_0001, 0xdead_beef_cafe_f00d];
const VOXEL_SIZES: [f32; 2] = [1.0, 0.5];

fn biome(
    name: &str,
    temperature: f32,
    ground: Option<GroundParams>,
    cave_scale: Option<f32>,
) -> Biome {
    Biome {
        name: BiomeName::new(name).unwrap(),
        temperature,
        humidity: 0.5,
        ground,
        sky_fog: None,
        cave_scale,
        surface_features: Default::default(),
    }
}

fn desert() -> GroundParams {
    GroundParams {
        rock: fixtures().lantern,
        soil: fixtures().glass,
        surface: fixtures().leaves,
    }
}

/// Every pass the generator has: caves tuned per Biome, smoothing, water, Biome grounds and ore;
/// on low land, so the ground lies in the box the digests read.
fn biome_world() -> WorldgenParams {
    common::lowland(WorldgenParams {
        sea_level_m: 24.0,
        relief_m: 24.0,
        water: Some(WaterParams {
            level_m: 34.0,
            material: fixtures().water,
        }),
        climate: ClimateParams {
            temperature_size_m: 64.0,
            humidity_size_m: 64.0,
        },
        biomes: BiomeTable::new(&[
            biome("t:plain", 0.5, None, None),
            biome("t:desert", 0.8, Some(desert()), Some(2.0)),
            biome("t:cold", 0.2, None, Some(0.0)),
        ])
        .unwrap(),
        ores: OreTable::new(&[Ore {
            material: fixtures().glass,
            min_depth_m: 2.0,
            max_depth_m: 20.0,
            vein_size_m: 3.0,
            rarity: 0.3,
            biomes: BiomeSet::new(&[BiomeId::new(0)]).unwrap(),
        }])
        .unwrap(),
        ..tesserae_core::testgen::world_params()
    })
}

fn digest(generator: &Worldgen) -> String {
    let chunk_m = CHUNK_SIZE as f32 * generator.voxel_size();
    let rows = (8.0 / chunk_m).floor() as i32..=(56.0 / chunk_m).floor() as i32;
    let reach = (48.0 / chunk_m).ceil() as i32;
    let names = fixtures().mapping();
    let mut digest = Sha256::new();
    for cy in rows {
        for cz in -reach..=reach {
            for cx in -reach..=reach {
                digest::chunk(&mut digest, &names, &generator.chunk(cx, cy, cz));
            }
        }
    }
    digest::hex(digest)
}

fn digests(params: WorldgenParams) -> Vec<String> {
    SEEDS
        .iter()
        .flat_map(|&seed| {
            VOXEL_SIZES
                .iter()
                .map(move |&size| digest(&Worldgen::new(seed, size, params)))
        })
        .collect()
}

const RECORDED: [&str; 4] = [
    "7ba6ccc7e4808b16862f331a7ee8d2898f915cc19b5d6e4bdf0913637323122d",
    "58256182dbb92bb8723db34ea794d459a91b784313798fadcca64998d121689b",
    "80111a98cd84e8cb8d6bf9021842bab3ec76604c2000b56133a90ac25fd9d695",
    "0115fc937fa1c65c12f6d3c3ebe9ebcbb9fe5edeb409c06b6a7e566ce76cf97a",
];

#[test]
fn a_world_whose_biomes_declare_no_features_makes_the_recorded_chunks() {
    assert_eq!(digests(biome_world()), RECORDED);
}

fn mapping() -> Mapping {
    let f = fixtures();
    let named = [
        (0, "core:air"),
        (f.ground.rock, "t:rock"),
        (f.ground.soil, "t:soil"),
        (f.ground.surface, "t:surface"),
        (f.lantern, "t:lantern"),
        (f.glass, "t:glass"),
        (f.leaves, "t:leaves"),
        (f.water, "t:water"),
    ];
    Mapping {
        materials: named.map(|(id, n)| (id, n.to_owned())).into(),
        shapes: [(0, "core:cube".to_owned())].into(),
    }
}

/// A snapshot without features is written exactly as before them, so a World saved then reopens
/// to the terrain it had.
#[test]
fn a_snapshot_whose_biomes_have_no_features_restores_to_the_recorded_chunks() {
    let saved = Generation::capture(biome_world(), &[], &[], &mapping()).unwrap();
    let json = serde_json::to_value(&saved).unwrap();
    let biomes = json["biomes"].as_array().unwrap();
    assert_eq!(biomes.len(), 3);
    assert!(biomes.iter().all(|b| b.get("surface_features").is_none()));
    let reopened: Generation = serde_json::from_value(json).unwrap();
    let params = reopened.restore(&mapping(), 3).unwrap();
    assert_eq!(digests(params), RECORDED);
}

#[test]
fn a_biome_holds_up_to_sixteen_features_spaced_4_to_1024_m_apart() {
    let at = |spacing_m| SurfaceFeatures::new(&[feature(MELON, spacing_m)]);
    assert!(at(4.0).is_ok());
    assert!(at(1024.0).is_ok());
    for spacing_m in [0.9, 1024.5, 0.0, -8.0, f32::NAN, f32::INFINITY] {
        let err = at(spacing_m).unwrap_err();
        assert_eq!(
            err.field, "biomes.surface_features.spacing_m",
            "{spacing_m}"
        );
    }
    assert_eq!(
        SurfaceFeatures::new(&[feature(AIR, 16.0)])
            .unwrap_err()
            .field,
        "biomes.surface_features.material"
    );
    let sixteen = [feature(MELON, 16.0); 16];
    assert_eq!(SurfaceFeatures::new(&sixteen).unwrap().as_slice(), sixteen);
    assert_eq!(
        SurfaceFeatures::new(&[feature(MELON, 16.0); 17])
            .unwrap_err()
            .field,
        "biomes.surface_features"
    );
}

/// The metre height of the lowest Voxel [`columns`] returns.
fn base_m(g: &Worldgen) -> f32 {
    let chunk_m = CHUNK_SIZE as f32 * g.voxel_size();
    ((g.surface_bounds_m().0 - 8.0) / chunk_m).floor() * chunk_m
}

/// Each column's Voxels from [`base_m`] to above the surface band.
fn columns(g: &Worldgen, half_m: f32) -> Vec<((f32, f32), Vec<Voxel>)> {
    let size = CHUNK_SIZE as i32;
    let voxel = g.voxel_size();
    let chunk_m = CHUNK_SIZE as f32 * voxel;
    let first = (base_m(g) / chunk_m).round() as i32;
    let rows = first..=(g.surface_bounds_m().1 / chunk_m).ceil() as i32;
    let reach = (half_m / chunk_m).ceil() as i32;
    let mut out = Vec::new();
    for cz in -reach..reach {
        for cx in -reach..reach {
            let chunks: Vec<_> = rows.clone().map(|cy| g.chunk(cx, cy, cz)).collect();
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let at = (
                        (cx * size + x as i32) as f32 * voxel,
                        (cz * size + z as i32) as f32 * voxel,
                    );
                    let voxels = chunks
                        .iter()
                        .flat_map(|c| (0..CHUNK_SIZE).map(move |y| c.get(x, y, z)))
                        .collect();
                    out.push((at, voxels));
                }
            }
        }
    }
    out
}

fn count(g: &Worldgen, half_m: f32, material: MaterialId) -> usize {
    columns(g, half_m)
        .iter()
        .flat_map(|(_, voxels)| voxels)
        .filter(|v| v.material() == material)
        .count()
}

fn plain(features: &[SurfaceFeature]) -> WorldgenParams {
    WorldgenParams {
        caves: None,
        smoothing: None,
        biomes: BiomeTable::new(&[featured(biome("t:plain", 0.5, None, None), features)]).unwrap(),
        water: None,
        ..tesserae_core::testgen::world_params()
    }
}

/// One feature for every `spacing_m` squared of ground, whatever a Voxel is worth.
#[test]
fn the_declared_spacing_holds_at_any_voxel_size() {
    let params = plain(&[feature(MELON, 8.0), feature(GOURD, 16.0)]);
    let half_m = 96.0;
    let area_m2 = (2.0 * half_m) * (2.0 * half_m);
    for size in [1.0, 0.5, 0.25] {
        let g = Worldgen::new(SEEDS[0], size, params);
        for (material, spacing_m) in [(MELON, 8.0f32), (GOURD, 16.0)] {
            let expected = area_m2 / spacing_m.powi(2);
            let found = count(&g, half_m, material) as f32;
            // Four standard deviations of a Poisson count.
            assert!(
                (found - expected).abs() < 4.0 * expected.sqrt(),
                "{size} m Voxels: {found} of {material}, expected about {expected}"
            );
        }
    }
}

#[test]
fn generation_is_deterministic_and_follows_the_seed() {
    let params = plain(&[feature(MELON, 8.0)]);
    let places = |seed| {
        let g = Worldgen::new(seed, 1.0, params);
        columns(&g, 32.0)
            .into_iter()
            .filter(|(_, voxels)| voxels.iter().any(|v| v.material() == MELON))
            .map(|(at, _)| at)
            .collect::<Vec<_>>()
    };
    assert!(!places(SEEDS[0]).is_empty());
    assert_eq!(places(SEEDS[0]), places(SEEDS[0]));
    assert_ne!(places(SEEDS[0]), places(SEEDS[1]));
}

/// A 4 m spacing at 4 m Voxels is every column, so each column that may hold a feature must, and
/// the generator's every other pass is in play: caves, smoothing, a lake and Biome grounds. Lifted,
/// the band crosses a Chunk's top, so some features stand in the Chunk above their ground; dry,
/// since the Chunk above asks whether a tunnel opened the ground without knowing that version 3
/// keeps tunnels shut beside the sea.
#[test]
fn a_feature_stands_on_every_full_surface_voxel_of_its_biome_and_nowhere_else() {
    let size = 4.0;
    let mut params = biome_world();
    let table: Vec<Biome> = params.biomes.as_slice().to_vec();
    params.biomes = BiomeTable::new(&[
        featured(table[0], &[feature(MELON, 4.0)]),
        featured(table[1], &[feature(GOURD, 4.0)]),
        table[2],
    ])
    .unwrap();
    let lifted = WorldgenParams {
        sea_level_m: params.sea_level_m + 100.0,
        water: None,
        ..params
    };
    for (seed, params) in [(SEEDS[0], params), (SEEDS[1], params), (SEEDS[0], lifted)] {
        let with = Worldgen::new(seed, size, params);
        let bare = Worldgen::new(seed, size, params.without_surface_features());
        let (base, highest) = (base_m(&bare), bare.surface_bounds_m().1);
        let (mut placed, mut refused, mut above) = (0, 0, 0);
        for (((x_m, z_m), bare_column), (_, column)) in
            columns(&bare, 192.0).into_iter().zip(columns(&with, 192.0))
        {
            let biome = params.biomes.as_slice()[bare.biome_at(x_m, z_m).unwrap().index()];
            let surface = biome.ground.or(params.ground).unwrap().surface;
            // A column carved to the bottom of the box has no top.
            let top = bare_column.iter().rposition(|v| !v.is_air());
            let ground_top = ((bare.surface_height_m(x_m, z_m) - base) / size).floor() as usize;
            let eligible = top == Some(ground_top)
                && bare_column[ground_top] == Voxel::cube(surface)
                && base + (ground_top + 1) as f32 * size <= highest;
            let expected = biome
                .surface_features
                .as_slice()
                .first()
                .filter(|_| eligible);
            for (y, (was, now)) in bare_column.iter().zip(&column).enumerate() {
                match expected {
                    Some(f) if y == ground_top + 1 => {
                        assert_eq!(*now, Voxel::cube(f.material), "({x_m}, {z_m})")
                    }
                    _ => assert_eq!(was, now, "({x_m}, {z_m}) y {y}"),
                }
            }
            match expected {
                Some(_) => {
                    placed += 1;
                    above += usize::from(ground_top % CHUNK_SIZE == CHUNK_SIZE - 1);
                }
                None => refused += usize::from(!biome.surface_features.is_empty()),
            }
        }
        assert!(
            placed > 100 && refused > 100,
            "{placed} placed, {refused} refused"
        );
        if params == lifted {
            assert!(above > 10, "{above} in the Chunk above");
        }
    }
}

#[test]
fn a_feature_must_be_a_registered_solid() {
    let f = fixtures();
    let with = |material| plain(&[feature(material, 16.0)]).validate_ground(&f.tables);
    assert_eq!(with(f.lantern), Ok(()));
    for material in [f.water, MELON] {
        assert_eq!(
            with(material).unwrap_err().field,
            "biomes.surface_features.material",
            "{material}"
        );
    }
}

#[test]
fn the_coarse_levels_parameters_have_bare_ground() {
    let params = plain(&[feature(MELON, 4.0)]);
    let bare = params.without_surface_features();
    assert!(
        bare.biomes
            .as_slice()
            .iter()
            .all(|b| b.surface_features.is_empty())
    );
    assert_eq!(bare.without_surface_features(), bare);
    assert_eq!(count(&Worldgen::new(SEEDS[0], 2.0, bare), 32.0, MELON), 0);
    assert_eq!(
        bare.biomes.as_slice()[0].name,
        params.biomes.as_slice()[0].name
    );
}
