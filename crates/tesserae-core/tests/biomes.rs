//! Climate and Biome selection through the generator's public query. Nothing
//! here looks at a Chunk: Biomes change no generated Voxel yet.

use tesserae_core::climate::BORDER_WARP;
use tesserae_core::{
    Biome, BiomeId, BiomeName, BiomeTable, CHUNK_SIZE, ClimateParams, MAX_BIOMES, Worldgen,
    WorldgenParams,
};

const SEED: u64 = 0x7e55_e2ae_0000_0001;

fn generator(seed: u64, voxel_size: f32) -> Worldgen {
    Worldgen::new(seed, voxel_size, WorldgenParams::default())
}

/// A coarse grid of metre positions, spaced past one Climate feature size so
/// it crosses several regions.
fn grid() -> impl Iterator<Item = (f32, f32)> {
    (-20..20).flat_map(|i| (-20..20).map(move |j| (i as f32 * 53.0, j as f32 * 53.0)))
}

#[test]
fn climate_stays_inside_the_unit_square() {
    let g = generator(SEED, 1.0);
    for (x, z) in grid() {
        let c = g.climate_at(x, z);
        assert!(
            (0.0..1.0).contains(&c.temperature),
            "temperature at {x},{z}"
        );
        assert!((0.0..1.0).contains(&c.humidity), "humidity at {x},{z}");
    }
}

#[test]
fn climate_varies_across_the_world_and_between_its_two_axes() {
    let g = generator(SEED, 1.0);
    let at: Vec<_> = grid().map(|(x, z)| g.climate_at(x, z)).collect();
    assert!(at.windows(2).any(|w| w[0].temperature != w[1].temperature));
    assert!(at.windows(2).any(|w| w[0].humidity != w[1].humidity));
    assert!(at.iter().any(|c| c.temperature != c.humidity));
}

#[test]
fn climate_is_a_pure_function_of_the_seed() {
    let (a, b) = (generator(SEED, 1.0), generator(SEED, 1.0));
    assert!(grid().all(|(x, z)| a.climate_at(x, z) == b.climate_at(x, z)));
    let other = generator(SEED + 1, 1.0);
    assert!(grid().any(|(x, z)| a.climate_at(x, z) != other.climate_at(x, z)));
}

#[test]
fn climate_is_the_same_at_every_voxel_size_and_lod_level() {
    let base = generator(SEED, 1.0);
    // `Worldgen::new(seed, voxel_size * 2^j, params)` is the level-j generator.
    for voxel_size in [0.25, 0.5, 3.0, 2.0, 4.0, 8.0, 16.0] {
        let other = generator(SEED, voxel_size);
        assert!(
            grid().all(|(x, z)| base.climate_at(x, z) == other.climate_at(x, z)),
            "differs at voxel size {voxel_size}"
        );
    }
}

#[test]
fn climate_regions_are_sized_in_metres() {
    let sized = |m: f32| {
        Worldgen::new(
            SEED,
            1.0,
            WorldgenParams {
                climate: ClimateParams {
                    temperature_size_m: m,
                    humidity_size_m: m * 4.0,
                },
                ..WorldgenParams::default()
            },
        )
    };
    // Doubling both sizes and both distances from the origin is the same field.
    let (small, large) = (sized(100.0), sized(200.0));
    for (x, z) in grid() {
        assert_eq!(
            small.climate_at(x, z),
            large.climate_at(x * 2.0, z * 2.0),
            "at {x},{z}"
        );
    }
}

#[test]
fn the_two_axes_are_sized_independently() {
    let with = |temperature_size_m, humidity_size_m| {
        Worldgen::new(
            SEED,
            1.0,
            WorldgenParams {
                climate: ClimateParams {
                    temperature_size_m,
                    humidity_size_m,
                },
                ..WorldgenParams::default()
            },
        )
    };
    let (a, b) = (with(100.0, 300.0), with(100.0, 700.0));
    assert!(grid().all(|(x, z)| a.climate_at(x, z).temperature == b.climate_at(x, z).temperature));
    assert!(grid().any(|(x, z)| a.climate_at(x, z).humidity != b.climate_at(x, z).humidity));
}

fn biome(name: &str, temperature: f32, humidity: f32) -> Biome {
    Biome {
        name: BiomeName::new(name).unwrap(),
        temperature,
        humidity,
        ground: None,
        sky_fog: None,
        cave_scale: None,
    }
}

fn table(biomes: &[Biome]) -> BiomeTable {
    BiomeTable::new(biomes).unwrap()
}

fn with_biomes(biomes: &[Biome], generator: u32, voxel_size: f32) -> Worldgen {
    Worldgen::new(
        SEED,
        voxel_size,
        WorldgenParams {
            biomes: table(biomes),
            generator,
            ..WorldgenParams::default()
        },
    )
}

/// A fine grid across several regions, so a border is crossed many times.
fn dense() -> impl Iterator<Item = (f32, f32)> {
    (-100..100).flat_map(|i| (-100..100).map(move |j| (i as f32 * 11.0, j as f32 * 11.0)))
}

const COLD: BiomeId = BiomeId::new(0);
const WARM: BiomeId = BiomeId::new(1);

/// Two Biomes that differ in temperature alone, so the border between them is
/// where temperature crosses 0.5 whatever the humidity.
fn cold_and_warm(generator: u32, voxel_size: f32) -> Worldgen {
    with_biomes(
        &[biome("cold", 0.2, 0.5), biome("warm", 0.8, 0.5)],
        generator,
        voxel_size,
    )
}

#[test]
fn a_world_without_biomes_has_none() {
    let g = generator(SEED, 1.0);
    assert!(grid().all(|(x, z)| g.biome_at(x, z).is_none()));
}

#[test]
fn generator_version_1_ignores_its_biomes() {
    let g = cold_and_warm(1, 1.0);
    assert!(grid().all(|(x, z)| g.biome_at(x, z).is_none()));
    assert!(cold_and_warm(2, 1.0).biome_at(0.0, 0.0).is_some());
}

#[test]
fn a_lone_biome_covers_the_world() {
    let g = with_biomes(&[biome("only", 0.9, 0.1)], 2, 1.0);
    assert!(grid().all(|(x, z)| g.biome_at(x, z) == Some(BiomeId::new(0))));
}

#[test]
fn the_nearest_point_wins_away_from_the_border() {
    let g = cold_and_warm(2, 1.0);
    let (mut cold, mut warm) = (0, 0);
    for (x, z) in dense() {
        let t = g.climate_at(x, z).temperature;
        if (t - 0.5).abs() <= BORDER_WARP {
            continue;
        }
        let want = if t < 0.5 { COLD } else { WARM };
        assert_eq!(g.biome_at(x, z), Some(want), "t {t} at {x},{z}");
        if want == COLD { cold += 1 } else { warm += 1 }
    }
    // Without both, the loop above checked nothing.
    assert!(cold > 100 && warm > 100, "cold {cold}, warm {warm}");
}

#[test]
fn the_border_is_warped_and_never_strays_beyond_the_warp() {
    let g = cold_and_warm(2, 1.0);
    let mut moved = 0;
    for (x, z) in dense() {
        let t = g.climate_at(x, z).temperature;
        let straight = if t < 0.5 { COLD } else { WARM };
        if g.biome_at(x, z) != Some(straight) {
            moved += 1;
            assert!((t - 0.5).abs() <= BORDER_WARP, "t {t} at {x},{z}");
        }
    }
    assert!(
        moved > 50,
        "only {moved} columns are on the other side of a straight border"
    );
}

#[test]
fn a_tie_goes_to_the_earlier_biome() {
    let g = with_biomes(&[biome("a", 0.4, 0.4), biome("b", 0.4, 0.4)], 2, 1.0);
    assert!(grid().all(|(x, z)| g.biome_at(x, z) == Some(COLD)));
}

#[test]
fn the_biome_layout_is_the_same_at_every_voxel_size_and_lod_level() {
    let base = cold_and_warm(2, 1.0);
    for voxel_size in [0.25, 0.5, 2.0, 4.0, 8.0, 16.0] {
        let other = cold_and_warm(2, voxel_size);
        assert!(
            dense().all(|(x, z)| base.biome_at(x, z) == other.biome_at(x, z)),
            "differs at voxel size {voxel_size}"
        );
    }
}

#[test]
fn a_seed_makes_its_own_layout() {
    let a = cold_and_warm(2, 1.0);
    let b = Worldgen::new(SEED + 1, 1.0, a.params());
    assert!(dense().any(|(x, z)| a.biome_at(x, z) != b.biome_at(x, z)));
}

#[test]
fn biomes_do_not_change_a_generated_chunk() {
    let plain = tesserae_core::testgen::world_params();
    let with = WorldgenParams {
        biomes: table(&[biome("cold", 0.2, 0.5), biome("warm", 0.8, 0.5)]),
        ..plain
    };
    let (a, b) = (
        Worldgen::new(SEED, 1.0, plain),
        Worldgen::new(SEED, 1.0, with),
    );
    let voxels = |g: &Worldgen, (cx, cy, cz)| {
        let chunk = g.chunk(cx, cy, cz);
        let n = CHUNK_SIZE;
        (0..n * n * n)
            .map(|i| chunk.get(i % n, i / n % n, i / (n * n)))
            .collect::<Vec<_>>()
    };
    for at in [(0, 0, 0), (3, 1, -2), (-5, 0, 4), (0, -1, 0)] {
        assert_eq!(voxels(&a, at), voxels(&b, at), "{at:?}");
    }
}

#[test]
fn the_generator_is_send_sync_and_copyable_params_stay_copy() {
    fn send_sync<T: Send + Sync>() {}
    fn copy<T: Copy>() {}
    send_sync::<Worldgen>();
    copy::<WorldgenParams>();
}

#[test]
fn a_table_holds_up_to_the_capacity_with_distinct_names() {
    let many: Vec<Biome> = (0..MAX_BIOMES)
        .map(|i| biome(&format!("b{i}"), i as f32 / 16.0, 0.5))
        .collect();
    let full = table(&many);
    assert_eq!(full.len(), MAX_BIOMES);
    assert_eq!(full.as_slice(), many.as_slice());
    assert_eq!(full.get(BiomeId::new(3)).unwrap().name.as_str(), "b3");
    assert!(full.get(BiomeId::new(MAX_BIOMES as u8)).is_none());

    let mut too_many = many.clone();
    too_many.push(biome("extra", 0.5, 0.5));
    assert!(BiomeTable::new(&too_many).is_err());
    assert!(BiomeTable::new(&[biome("a", 0.1, 0.1), biome("a", 0.9, 0.9)]).is_err());
}

#[test]
fn a_point_outside_the_unit_square_is_refused() {
    for (t, h) in [
        (-0.01, 0.5),
        (1.01, 0.5),
        (0.5, -0.01),
        (0.5, 1.01),
        (f32::NAN, 0.5),
    ] {
        assert!(BiomeTable::new(&[biome("x", t, h)]).is_err(), "{t},{h}");
    }
    assert!(BiomeTable::new(&[biome("corner", 0.0, 1.0)]).is_ok());
}

#[test]
fn a_name_is_one_to_thirty_two_bytes() {
    assert!(BiomeName::new("").is_err());
    assert!(BiomeName::new(&"a".repeat(32)).is_ok());
    assert!(BiomeName::new(&"a".repeat(33)).is_err());
    assert_eq!(BiomeName::new("tundra").unwrap().as_str(), "tundra");
}

#[test]
fn climate_sizes_must_be_positive_and_finite() {
    for bad in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        for params in [
            ClimateParams {
                temperature_size_m: bad,
                ..ClimateParams::default()
            },
            ClimateParams {
                humidity_size_m: bad,
                ..ClimateParams::default()
            },
        ] {
            let p = WorldgenParams {
                climate: params,
                ..WorldgenParams::default()
            };
            assert!(p.validate().is_err(), "{bad}");
        }
    }
    assert!(WorldgenParams::default().validate().is_ok());
}

#[test]
fn climate_params_round_trip_and_refuse_unknown_fields() {
    let p = ClimateParams {
        temperature_size_m: 300.0,
        humidity_size_m: 700.0,
    };
    let again: ClimateParams = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
    assert_eq!(again, p);
    assert!(
        serde_json::from_str::<ClimateParams>(
            r#"{"temperature_size_m":1.0,"humidity_size_m":1.0,"octaves":3}"#
        )
        .is_err()
    );
}

#[test]
fn an_empty_table_is_the_default() {
    assert!(WorldgenParams::default().biomes.is_empty());
    assert_eq!(WorldgenParams::default().biomes, BiomeTable::default());
}
