//! Planted fields in the demo Games: where wheat fields stand and what they are made of.

use std::collections::BTreeMap;

use tesserae_core::{Biome, BiomeTable, CHUNK_SIZE, SurfaceFeatures, Worldgen};

use super::loaded;

/// The names of the materials `game`'s planted fields change within `half_m` of the origin, counted
/// in Voxels, at Voxels of 1 m.
fn field_cells(game: &str, seed: u64, half_m: f32) -> BTreeMap<String, u64> {
    let (_install, loaded) = loaded(game, 1.0);
    let params = loaded.world.unwrap();
    let bare: Vec<Biome> = params
        .biomes
        .as_slice()
        .iter()
        .map(|b| Biome {
            surface_features: SurfaceFeatures::new(b.surface_features.as_slice()).unwrap(),
            ..*b
        })
        .collect();
    let without = tesserae_core::WorldgenParams {
        biomes: BiomeTable::new(&bare).unwrap(),
        ..params
    };
    let (with, without) = (
        Worldgen::new(seed, 1.0, params),
        Worldgen::new(seed, 1.0, without),
    );
    let (low, high) = with.surface_bounds_m();
    let size = CHUNK_SIZE as f32;
    let reach = (half_m / size).ceil() as i32;
    let mut names: BTreeMap<u16, u64> = BTreeMap::new();
    for cy in (low / size).floor() as i32 - 1..=(high / size).ceil() as i32 + 1 {
        for cz in -reach..reach {
            for cx in -reach..reach {
                let (a, b) = (with.chunk(cx, cy, cz), without.chunk(cx, cy, cz));
                for i in 0..CHUNK_SIZE.pow(3) {
                    let at = (
                        i % CHUNK_SIZE,
                        i / CHUNK_SIZE % CHUNK_SIZE,
                        i / CHUNK_SIZE.pow(2),
                    );
                    let voxel = a.get(at.0, at.1, at.2);
                    if voxel != b.get(at.0, at.1, at.2) {
                        *names.entry(voxel.material()).or_default() += 1;
                    }
                }
            }
        }
    }
    names
        .into_iter()
        .map(|(id, n)| {
            let name = loaded
                .registry
                .names(tesserae_mods::Kind::Material)
                .find(|&(at, _)| at == usize::from(id))
                .map_or_else(|| id.to_string(), |(_, n)| n.to_owned());
            (name, n)
        })
        .collect()
}

/// Voxels of the crop `name` in Stages, across every Stage Material.
fn crop_voxels(cells: &BTreeMap<String, u64>, name: &str) -> u64 {
    cells
        .iter()
        .filter(|(m, _)| m.starts_with(&format!("crops:{name}_")))
        .map(|(_, n)| n)
        .sum()
}

#[test]
fn every_game_with_planted_fields_places_wheat_most_often_beside_paths() {
    // The window each Game needs for a field to stand in it: Terra's and tidewater's ground is
    // rough, so a level place is rarer.
    for (game, half_m, path, ditch) in [
        ("terra", 512.0, "terra:sand", Some("terra:water")),
        ("meadow", 256.0, "strata:sand", None),
        ("tidewater", 512.0, "strata:sand", Some("waters:water")),
        ("lumen", 256.0, "strata:sand", None),
    ] {
        let cells = field_cells(game, 7, half_m);
        let [wheat, carrot, potato] = ["wheat", "carrot", "potato"].map(|c| crop_voxels(&cells, c));
        assert!(
            wheat > carrot && wheat > potato && carrot > 0 && potato > 0,
            "{game}: wheat {wheat}, carrot {carrot}, potato {potato}"
        );
        assert!(cells.get(path).is_some_and(|&n| n > 0), "{game}: {cells:?}");
        match ditch {
            Some(fluid) => assert!(
                cells.get(fluid).is_some_and(|&n| n > 0),
                "{game} has no ditch"
            ),
            None => assert!(
                !cells.contains_key("waters:water") && !cells.contains_key("terra:water"),
                "{game} has no lake, so no ditch"
            ),
        }
    }
}

/// The size of each field group a Biome declares, largest first: its flowers and its vegetables.
#[test]
fn ground_cover_competes_by_region_in_the_biomes_that_have_flowers_and_vegetables() {
    for (game, biome, groups) in [
        ("terra", "terra:temperate", vec![3, 2]),
        ("terra", "terra:savanna", vec![2]),
        ("meadow", "meadow:meadow", vec![3, 3]),
        ("meadow", "meadow:prairie", vec![2]),
        ("lumen", "lumen:garden", vec![3, 2]),
    ] {
        let (_install, loaded) = loaded(game, 1.0);
        let world = loaded.world.unwrap();
        let features = super::biome(&world, biome).surface_features;
        let mut sizes: BTreeMap<u8, usize> = BTreeMap::new();
        for field in features.as_slice().iter().filter_map(|f| f.field) {
            *sizes.entry(field.group).or_default() += 1;
        }
        let mut sizes: Vec<usize> = sizes.into_values().collect();
        sizes.sort_by(|a, b| b.cmp(a));
        assert_eq!(sizes, groups, "{game} {biome}");
    }
}
