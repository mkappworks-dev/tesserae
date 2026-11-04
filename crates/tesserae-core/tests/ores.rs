//! The ore record a World declares: validated plain data, empty by default,
//! carried in `WorldgenParams`. Where it generates is `ore_veins.rs`.

use tesserae_core::{
    AIR, Biome, BiomeId, BiomeName, BiomeSet, BiomeTable, MAX_BIOMES, MAX_ORES, Ore, OreTable,
    WorldgenParams,
};

fn ore() -> Ore {
    Ore {
        material: 9,
        min_depth_m: 4.0,
        max_depth_m: 40.0,
        vein_size_m: 3.0,
        rarity: 0.02,
        biomes: BiomeSet::default(),
    }
}

fn biome(name: &str) -> Biome {
    Biome {
        name: BiomeName::new(name).unwrap(),
        temperature: 0.5,
        humidity: 0.5,
        ground: None,
        sky_fog: None,
        cave_scale: None,
    }
}

#[test]
fn a_table_is_empty_by_default_and_keeps_declaration_order() {
    assert!(OreTable::default().is_empty());
    assert!(WorldgenParams::default().ores.is_empty());
    let (a, b) = (
        ore(),
        Ore {
            material: 10,
            ..ore()
        },
    );
    let table = OreTable::new(&[a, b]).unwrap();
    assert_eq!(table.as_slice(), [a, b]);
    assert_eq!(table.len(), 2);
}

#[test]
fn an_ore_with_a_bad_number_is_refused() {
    let bad = [
        Ore {
            material: AIR,
            ..ore()
        },
        Ore {
            min_depth_m: -1.0,
            ..ore()
        },
        Ore {
            min_depth_m: 50.0,
            ..ore()
        },
        Ore {
            max_depth_m: f32::NAN,
            ..ore()
        },
        Ore {
            max_depth_m: f32::INFINITY,
            ..ore()
        },
        Ore {
            vein_size_m: 0.0,
            ..ore()
        },
        Ore {
            vein_size_m: f32::NAN,
            ..ore()
        },
        Ore {
            rarity: 0.0,
            ..ore()
        },
        Ore {
            rarity: 1.5,
            ..ore()
        },
        Ore {
            rarity: f32::NAN,
            ..ore()
        },
    ];
    for o in bad {
        assert!(OreTable::new(&[o]).is_err(), "{o:?}");
    }
}

#[test]
fn a_band_may_be_a_single_depth_and_rarity_may_be_one() {
    let o = Ore {
        min_depth_m: 6.0,
        max_depth_m: 6.0,
        rarity: 1.0,
        ..ore()
    };
    assert!(OreTable::new(&[o]).is_ok());
}

#[test]
fn a_table_holds_at_most_the_cap() {
    assert!(OreTable::new(&vec![ore(); MAX_ORES]).is_ok());
    assert!(OreTable::new(&vec![ore(); MAX_ORES + 1]).is_err());
}

#[test]
fn no_biomes_named_means_every_biome() {
    let any = BiomeSet::default();
    assert!(any.is_unrestricted());
    assert!((0..MAX_BIOMES as u8).all(|i| any.contains(BiomeId::new(i))));
}

#[test]
fn a_set_holds_the_biomes_it_was_given_once_each() {
    let set = BiomeSet::new(&[BiomeId::new(3), BiomeId::new(1), BiomeId::new(3)]).unwrap();
    assert!(!set.is_unrestricted());
    assert!(set.contains(BiomeId::new(1)) && set.contains(BiomeId::new(3)));
    assert!(!set.contains(BiomeId::new(2)));
    let ids: Vec<_> = set.ids().map(BiomeId::index).collect();
    assert_eq!(ids, [1, 3]);
}

#[test]
fn a_set_cannot_name_a_biome_the_table_cannot_hold() {
    assert!(BiomeSet::new(&[BiomeId::new(MAX_BIOMES as u8 - 1)]).is_ok());
    assert!(BiomeSet::new(&[BiomeId::new(MAX_BIOMES as u8)]).is_err());
}

#[test]
fn params_refuse_an_ore_that_names_a_biome_the_world_lacks() {
    let ores = |biomes| OreTable::new(&[Ore { biomes, ..ore() }]).unwrap();
    let biomes = BiomeTable::new(&[biome("dry"), biome("wet")]).unwrap();
    let in_range = ores(BiomeSet::new(&[BiomeId::new(1)]).unwrap());
    let beyond = ores(BiomeSet::new(&[BiomeId::new(2)]).unwrap());
    let with = |biomes, ores| WorldgenParams {
        biomes,
        ores,
        ..WorldgenParams::default()
    };
    assert!(with(biomes, in_range).validate().is_ok());
    assert!(with(biomes, beyond).validate().is_err());
    assert!(with(BiomeTable::default(), in_range).validate().is_err());
    assert!(
        with(BiomeTable::default(), ores(BiomeSet::default()))
            .validate()
            .is_ok()
    );
}
