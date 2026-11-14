//! Terra's farm: what a crop, a bush and an apple tree yield and how they reset, and the food
//! Recipes that use the produce.

use super::{loaded, material, shape};
use tesserae_core::plant::Kind;

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
    let ripe = material(&loaded, "crops:wheat_ripe");
    assert_eq!(
        yields_of(&loaded, "crops:wheat_ripe"),
        [("crops:wheat".into(), 1), ("crops:wheat_seeds".into(), 1)]
    );
    assert_eq!(
        plants.harvest(ripe).map(|h| h.ripe),
        None,
        "wheat is cut, not picked"
    );

    let berries = plants
        .harvest(material(&loaded, "crops:berry_ripe"))
        .unwrap();
    assert!(berries.ripe);
    assert_eq!(berries.to, material(&loaded, "crops:berry_bush"));
    assert_eq!(berries.yields[0].item, "crops:berries");
}

#[test]
fn an_apple_tree_bears_apples_that_breaking_picks_and_that_the_tree_grows_again() {
    let (_install, loaded) = loaded("terra", 0.5);
    let plants = &loaded.tables.plants;
    let ripe = material(&loaded, "flora:apple_ripe");
    assert!(plants.harvest(ripe).is_none(), "fruit is broken, not reset");
    assert_eq!(plants.yields(ripe)[0].item, "flora:apple");
    assert!(
        plants
            .yields(material(&loaded, "flora:apple_unripe"))
            .is_empty()
    );
    let apple = loaded
        .items
        .iter()
        .find(|i| i.name == "flora:apple")
        .unwrap();
    assert!(apple.food.is_some_and(|f| f > 0.0));
    assert_eq!(
        plants.canopy_of_fruit(material(&loaded, "flora:apple_ripe")),
        Some(material(&loaded, "flora:apple_leaves"))
    );
}

#[test]
fn bread_and_pie_are_made_from_the_produce() {
    let (_install, loaded) = loaded("terra", 0.5);
    let items = loaded.every_item(0.5);
    for food in ["crops:bread", "crops:berry_pie"] {
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
        &["flora:oak_leaves"][..],
        &["flora:apple_unripe", "flora:apple_ripe"],
        &[
            "crops:wheat_sprout",
            "crops:wheat_young",
            "crops:wheat_green",
            "crops:wheat_ripe",
        ],
    ] {
        let found: usize = names
            .iter()
            .map(|n| distances_m(&generator, material(&loaded, n), SHORT_WALK_M).len())
            .sum();
        assert!(found > 0, "{names:?} within {SHORT_WALK_M} m");
    }
}

#[test]
fn terras_plants_name_their_shapes_and_a_generated_apple_is_the_box_apple() {
    use super::food::SHORT_WALK_M;
    use tesserae_core::CHUNK_SIZE;
    let (_install, loaded) = loaded("terra", 0.5);
    let plants = &loaded.tables.plants;
    for (name, want) in [
        ("crops:wheat_sprout", "flora:cross"),
        ("crops:berry_sprout", "crops:berry_sprout"),
        ("flora:oak_sapling", "flora:cross"),
        ("flora:cane", "flora:cross"),
        ("aquatic:kelp", "flora:cross"),
        ("flora:mushroom_red", "flora:mushroom"),
        ("flora:vine", "flora:plane"),
    ] {
        assert_eq!(
            plants.fixed_shape(material(&loaded, name)),
            Some(shape(&loaded, want)),
            "{name}"
        );
    }
    let apple = shape(&loaded, "flora:apple");
    let Kind::Tree(tree) = &plants
        .plants()
        .iter()
        .find(|p| p.name == "flora:apple")
        .unwrap()
        .kind
    else {
        panic!("a tree")
    };
    assert_eq!(tree.fruit.as_ref().unwrap().shape, Some(apple));
    for face in 0..6 {
        let voxel = tesserae_core::Voxel::new(material(&loaded, "flora:apple_ripe"), apple, 0);
        assert!(!loaded.tables.shapes.seals(voxel, face), "face {face}");
    }

    let generator = tesserae_core::Worldgen::new(7, 0.5, loaded.world.unwrap());
    let small = shape(&loaded, "flora:apple_unripe");
    let drawn = [
        (material(&loaded, "flora:apple_unripe"), small),
        (material(&loaded, "flora:apple_ripe"), apple),
    ];
    let chunk_m = CHUNK_SIZE as f32 * 0.5;
    let (lowest, highest) = generator.surface_bounds_m();
    let reach = (SHORT_WALK_M / chunk_m).ceil() as i32;
    let mut found = 0;
    let mut seen = [false; 2];
    for cz in -reach..reach {
        for cx in -reach..reach {
            for cy in (lowest / chunk_m).floor() as i32..=(highest / chunk_m).ceil() as i32 {
                let chunk = generator.chunk(cx, cy, cz);
                if chunk.uniform().is_some() {
                    continue;
                }
                for i in 0..CHUNK_SIZE.pow(3) {
                    let at = (
                        i % CHUNK_SIZE,
                        i / CHUNK_SIZE % CHUNK_SIZE,
                        i / CHUNK_SIZE.pow(2),
                    );
                    let voxel = chunk.get(at.0, at.1, at.2);
                    if let Some(&(_, want)) = drawn.iter().find(|d| d.0 == voxel.material()) {
                        assert_eq!(
                            voxel.shape(),
                            want,
                            "a generated apple takes its Stage's Shape"
                        );
                        found += 1;
                        seen[usize::from(want == apple)] = true;
                    }
                }
            }
        }
    }
    assert!(found > 0, "no apple generated near the spawn");
    assert!(seen[1], "a ripe apple is generated near the spawn");
}

/// The berry bush is a stem, a young mound and a mound with berries, each its own Shape that seals
/// no face and collides with nothing, and a ripe break resets it to the young mound.
#[test]
fn each_berry_bush_stage_is_its_own_box_shape_that_seals_no_face() {
    let (_install, loaded) = loaded("terra", 0.5);
    let plants = &loaded.tables.plants;
    let shapes = &loaded.tables.shapes;
    let stages = [
        ("crops:berry_sprout", "crops:berry_sprout"),
        ("crops:berry_bush", "crops:berry_young"),
        ("crops:berry_ripe", "crops:berry_ripe"),
    ];
    let mut volumes = Vec::new();
    for (stage, want) in stages {
        let id = material(&loaded, stage);
        let drawn = plants.stage_shape(id).unwrap();
        assert_eq!(drawn, shape(&loaded, want), "{stage}");
        let voxel = tesserae_core::Voxel::new(id, drawn, 0);
        assert!(
            (0..6).all(|face| !shapes.seals(voxel, face)),
            "{stage} seals a face"
        );
        assert!(
            shapes.collision(voxel, Default::default()).is_empty(),
            "{stage} is walked through"
        );
        assert!(
            !shapes.selection(voxel, Default::default()).is_empty(),
            "{stage} is selectable"
        );
        volumes.push(shapes.volume(voxel));
    }
    assert!(
        volumes.windows(2).all(|w| w[0] < w[1]),
        "each Stage is fuller: {volumes:?}"
    );
    let seed = material(&loaded, "crops:berry_sprout");
    assert_eq!(
        plants.fixed_shape(seed),
        Some(shape(&loaded, "crops:berry_sprout"))
    );
    let ripe = material(&loaded, "crops:berry_ripe");
    let to = plants.harvest(ripe).unwrap().to;
    assert_eq!(
        plants.stage_shape(to),
        Some(shape(&loaded, "crops:berry_young"))
    );
}
