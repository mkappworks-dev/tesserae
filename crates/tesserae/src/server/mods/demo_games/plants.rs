//! Terra's farm: what a crop, a bush and an apple tree yield and how they reset, and the food
//! Recipes that use the produce.

use super::{loaded, material};

fn yields_of(loaded: &super::LoadedMods, name: &str) -> Vec<(String, u32)> {
    let id = material(loaded, name);
    loaded
        .tables
        .plants
        .yields(id)
        .iter()
        .map(|y| (y.item.clone(), y.count.min))
        .collect()
}

#[test]
fn ripe_wheat_gives_wheat_and_seeds_and_a_berry_bush_fruits_again() {
    let (_install, loaded) = loaded("terra", 0.5);
    let plants = &loaded.tables.plants;
    let ripe = material(&loaded, "terra:wheat_ripe");
    assert_eq!(
        yields_of(&loaded, "terra:wheat_ripe"),
        [("terra:wheat".into(), 1), ("terra:wheat_seeds".into(), 1)]
    );
    assert_eq!(
        plants.harvest(ripe).map(|h| h.ripe),
        None,
        "wheat is cut, not picked"
    );

    let berries = plants
        .harvest(material(&loaded, "terra:berry_ripe"))
        .unwrap();
    assert!(berries.ripe);
    assert_eq!(berries.to, material(&loaded, "terra:berry_bush"));
    assert_eq!(berries.yields[0].item, "terra:berries");
}

#[test]
fn an_apple_tree_bears_apples_that_use_picks_and_that_grow_back() {
    let (_install, loaded) = loaded("terra", 0.5);
    let plants = &loaded.tables.plants;
    let ripe = plants
        .harvest(material(&loaded, "terra:apple_ripe"))
        .unwrap();
    assert!(ripe.ripe);
    assert_eq!(ripe.to, material(&loaded, "terra:apple_unripe"));
    assert_eq!(ripe.yields[0].item, "terra:apple");
    let apple = loaded
        .items
        .iter()
        .find(|i| i.name == "terra:apple")
        .unwrap();
    assert!(apple.food.is_some_and(|f| f > 0.0));
    assert_eq!(
        plants.canopy_of_fruit(material(&loaded, "terra:apple_ripe")),
        Some(material(&loaded, "terra:apple_leaves"))
    );
}

#[test]
fn bread_and_pie_are_made_from_the_produce() {
    let (_install, loaded) = loaded("terra", 0.5);
    let items = loaded.every_item(0.5);
    for food in ["terra:bread", "terra:berry_pie"] {
        items.named(food).expect(food);
        assert!(
            loaded.recipes.iter().any(|r| r.output.item == food),
            "{food} has a Recipe"
        );
        let item = loaded.items.iter().find(|i| i.name == food).unwrap();
        assert!(item.food.is_some_and(|f| f > 0.0));
    }
}

/// Meadows and forests in a new World: the temperate Biome near the spawn shows its oaks, apples
/// and wheat, though other Biomes may hold the spawn itself.
#[test]
fn a_new_terra_world_grows_oaks_apples_and_wheat_within_a_short_walk() {
    use super::food::{SHORT_WALK_M, distances_m};
    let (_install, loaded) = loaded("terra", 0.5);
    let world = loaded.world.unwrap();
    let generator = tesserae_core::Worldgen::new(7, 0.5, world);
    for names in [
        &["terra:oak_leaves"][..],
        &["terra:apple_unripe", "terra:apple_ripe"],
        &[
            "terra:wheat_sprout",
            "terra:wheat_young",
            "terra:wheat_green",
            "terra:wheat_ripe",
        ],
    ] {
        let found: usize = names
            .iter()
            .map(|n| distances_m(&generator, material(&loaded, n), SHORT_WALK_M).len())
            .sum();
        assert!(found > 0, "{names:?} within {SHORT_WALK_M} m");
    }
}
