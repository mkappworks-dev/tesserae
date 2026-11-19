//! Planted fields in the demo Games: where wheat fields stand and what they are made of.

use std::collections::BTreeMap;

use tesserae_core::{Biome, BiomeTable, CHUNK_SIZE, Worldgen};

use super::loaded;
use super::structures::settled_rows;

/// The names of the materials `game`'s planted fields change within `half_m` of the spawn, counted
/// in Voxels, at Voxels of 1 m, against the same World with only its crop fields dropped. A field
/// and its ditch lie on the ground, so only its [`settled_rows`] are compared.
fn field_cells(game: &str, seed: u64, half_m: f32) -> BTreeMap<String, u64> {
    let (_install, loaded) = loaded(game, 1.0);
    let params = loaded.world.unwrap();
    let fieldless: Vec<Biome> = params
        .biomes
        .as_slice()
        .iter()
        .map(|b| Biome {
            surface_features: b.surface_features.with_crop_fields(&[]).unwrap(),
            ..*b
        })
        .collect();
    let biomes = BiomeTable::new(&fieldless)
        .and_then(|t| t.with_selectors(params.biomes.selectors()))
        .and_then(|t| t.with_strata(params.biomes.strata()))
        .unwrap();
    let without = tesserae_core::WorldgenParams { biomes, ..params };
    let (with, without) = (
        Worldgen::new(seed, 1.0, params),
        Worldgen::new(seed, 1.0, without),
    );
    let size = CHUNK_SIZE as f32;
    let reach = (half_m / size).ceil() as i32;
    let eye = crate::server::spawn_eye(&with);
    let [ox, oz] = [eye.x, eye.z].map(|m| (m / size).floor() as i32);
    let mut names: BTreeMap<u16, u64> = BTreeMap::new();
    for cz in oz - reach..oz + reach {
        for cx in ox - reach..ox + reach {
            for cy in settled_rows(&with, [cx, cz]) {
                let (a, b) = (with.chunk(cx, cy, cz), without.chunk(cx, cy, cz));
                if a.uniform().is_some() && a.uniform() == b.uniform() {
                    continue;
                }
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

/// Each Game's half window for a field to stand in it, its path and its ditch's fluid: Terra's and
/// tidewater's ground is rough, so a level place is rarer.
const FIELD_GAMES: [(&str, f32, &str, Option<&str>); 5] = [
    ("terra", 512.0, "terra:sand", Some("terra:water")),
    ("meadow", 256.0, "strata:sand", None),
    ("tidewater", 512.0, "strata:sand", Some("waters:water")),
    ("lumen", 256.0, "strata:sand", None),
    ("ember", 512.0, "strata:basalt", Some("waters:water")),
];

fn every_game_with_planted_fields_places_its_lead_crop_most_often_beside_paths(game: &str) {
    let &(game, half_m, path, ditch) = FIELD_GAMES.iter().find(|g| g.0 == game).unwrap();
    let cells = field_cells(game, 7, half_m);
    let [wheat, carrot, potato] = ["wheat", "carrot", "potato"].map(|c| crop_voxels(&cells, c));
    // Ember's ash grows hardy roots, so its potatoes lead.
    let leads = if game == "ember" {
        potato > wheat && potato > carrot
    } else {
        wheat > carrot && wheat > potato
    };
    assert!(
        leads && wheat > 0 && carrot > 0 && potato > 0,
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

each_case!(every_game_with_planted_fields_places_its_lead_crop_most_often_beside_paths:
    terra, meadow, tidewater, lumen, ember);

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
