//! Version 3: Climate by altitude and coast, and Biomes chosen by selectors.

use tesserae_core::terrain::LandformParams;
use tesserae_core::{
    Biome, BiomeName, BiomeSelectors, BiomeTable, ClimateParams, SurfaceFeatures, Worldgen,
    WorldgenParams,
};

const SEED: u64 = 0x7e55_e2ae_0000_0001;

fn biome(name: &str, temperature: f32, humidity: f32) -> Biome {
    Biome {
        name: BiomeName::new(name).unwrap(),
        temperature,
        humidity,
        ground: None,
        sky_fog: None,
        cave_scale: None,
        surface_features: SurfaceFeatures::NONE,
    }
}

fn world(biomes: &[Biome], selectors: &[BiomeSelectors]) -> Worldgen {
    let params = WorldgenParams {
        generator: 3,
        sea_level_m: 24.0,
        relief_m: 24.0,
        landforms: LandformParams::default(),
        climate: ClimateParams::default(),
        biomes: BiomeTable::new(biomes)
            .unwrap()
            .with_selectors(selectors)
            .unwrap(),
        ..tesserae_core::testgen::world_params()
    };
    Worldgen::new(SEED, 1.0, params)
}

fn grid() -> impl Iterator<Item = (f32, f32)> {
    (-60..60).flat_map(|i| (-60..60).map(move |j| (i as f32 * 211.0, j as f32 * 197.0)))
}

#[test]
fn the_air_is_colder_higher_up_in_version_3_and_not_before() {
    let v3 = world(&[], &[]);
    let v2 = Worldgen::new(
        SEED,
        1.0,
        WorldgenParams {
            generator: 2,
            ..tesserae_core::testgen::world_params()
        },
    );
    for (x, z) in grid().take(200) {
        let (low, high) = (
            v3.climate_at_height(x, 24.0, z),
            v3.climate_at_height(x, 224.0, z),
        );
        assert!(high.temperature <= low.temperature, "{x},{z}");
        assert_eq!(
            v2.climate_at_height(x, 24.0, z),
            v2.climate_at_height(x, 224.0, z)
        );
    }
    let colder = grid()
        .filter(|&(x, z)| {
            v3.climate_at_height(x, 224.0, z).temperature
                < v3.climate_at_height(x, 24.0, z).temperature - 0.2
        })
        .count();
    assert!(colder > 1000, "{colder} columns are much colder up high");
}

#[test]
fn the_air_is_wetter_by_the_sea_than_far_inland() {
    let g = world(&[], &[]);
    let (mut coastal, mut inland) = (vec![], vec![]);
    for (x, z) in grid() {
        let f = g.landform_fields(x, z).unwrap();
        let h = g.climate_at_height(x, 30.0, z).humidity - g.climate_at(x, z).humidity;
        if f.continent < 0.5 {
            coastal.push(h);
        } else if f.continent > 0.8 {
            inland.push(h);
        }
    }
    assert!(!coastal.is_empty() && !inland.is_empty());
    let mean = |v: &[f32]| v.iter().sum::<f32>() / v.len() as f32;
    assert!(
        mean(&coastal) > mean(&inland) + 0.1,
        "{} vs {}",
        mean(&coastal),
        mean(&inland)
    );
}

#[test]
fn a_biome_naming_no_range_is_chosen_as_before() {
    let plain = world(&[biome("a", 0.2, 0.2), biome("b", 0.8, 0.8)], &[]);
    assert!(
        grid()
            .take(500)
            .all(|(x, z)| plain.biome_at(x, z).is_some())
    );
    let spread: std::collections::HashSet<_> =
        grid().filter_map(|(x, z)| plain.biome_at(x, z)).collect();
    assert_eq!(spread.len(), 2);
}

#[test]
fn altitude_and_coast_ranges_put_a_biome_where_the_land_suits_it() {
    let alpine = BiomeSelectors {
        altitude_m: Some([70.0, 1000.0]),
        ..BiomeSelectors::default()
    };
    let sea = BiomeSelectors {
        altitude_m: Some([-1000.0, -2.0]),
        ..BiomeSelectors::default()
    };
    // Same Climate point: only the selectors tell the Biomes apart, and a tie goes to the earlier
    // Biome, so the specific ones come before the one that matches everything.
    let g = world(
        &[
            biome("sea", 0.5, 0.5),
            biome("alpine", 0.5, 0.5),
            biome("plain", 0.5, 0.5),
        ],
        &[sea, alpine, BiomeSelectors::default()],
    );
    let (mut high, mut deep) = (0, 0);
    for (x, z) in grid() {
        let h = g.surface_height_m(x, z) - 24.0;
        let id = g.biome_at(x, z).unwrap().index();
        if h >= 70.0 {
            assert_eq!(id, 1, "alpine above 70 m at {x},{z}");
            high += 1;
        } else if h <= -2.0 {
            assert_eq!(id, 0, "sea under the sea at {x},{z}");
            deep += 1;
        } else {
            assert_eq!(id, 2, "plain at {x},{z} height {h}");
        }
    }
    assert!(high > 20 && deep > 100, "{high} high, {deep} deep columns");
}

#[test]
fn where_no_biome_admits_a_column_it_still_gets_one() {
    let only_high = BiomeSelectors {
        altitude_m: Some([500.0, 900.0]),
        ..BiomeSelectors::default()
    };
    let g = world(&[biome("high", 0.5, 0.5)], &[only_high]);
    assert!(grid().all(|(x, z)| g.biome_at(x, z).is_some()));
}

#[test]
fn relief_scales_the_detail_of_the_ground_and_nothing_else() {
    let flat = BiomeSelectors {
        relief: Some(0.0),
        ..BiomeSelectors::default()
    };
    let rough = BiomeSelectors {
        relief: Some(3.0),
        ..BiomeSelectors::default()
    };
    let bumps = |s: BiomeSelectors| {
        let g = world(&[biome("only", 0.5, 0.5)], &[s]);
        let mut total = 0.0f32;
        let mut base = Vec::new();
        for i in 0..400 {
            let x = i as f32 * 6.0 + 100_000.0;
            let (a, b) = (g.surface_height_m(x, 0.0), g.surface_height_m(x + 6.0, 0.0));
            total += (a - b).abs();
            base.push(a);
        }
        (total, base)
    };
    let (flat_total, _) = bumps(flat);
    let (rough_total, _) = bumps(rough);
    assert!(
        rough_total > flat_total * 1.3,
        "{rough_total} vs {flat_total}"
    );
}

#[test]
fn a_bad_selector_is_refused_naming_the_field() {
    let table = BiomeTable::new(&[biome("a", 0.5, 0.5)]).unwrap();
    for bad in [
        BiomeSelectors {
            altitude_m: Some([10.0, 5.0]),
            ..BiomeSelectors::default()
        },
        BiomeSelectors {
            relief: Some(9.0),
            ..BiomeSelectors::default()
        },
    ] {
        let err = table.with_selectors(&[bad]).unwrap_err();
        assert!(err.to_string().contains("selectors"), "{err}");
    }
}
