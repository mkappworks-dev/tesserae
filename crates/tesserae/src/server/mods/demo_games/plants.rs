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
fn watermelons_and_pumpkins_bear_grown_fruit_that_breaking_drops_as_food() {
    let (_install, loaded) = loaded("terra", 0.5);
    let plants = &loaded.tables.plants;
    for (crop, ripe_yield, seeds) in [
        (
            "watermelon",
            "crops:watermelon_slice",
            "crops:watermelon_seeds",
        ),
        ("pumpkin", "crops:pumpkin", "crops:pumpkin_seeds"),
    ] {
        let plant = plants
            .plants()
            .iter()
            .find(|p| p.name == format!("crops:{crop}"))
            .unwrap();
        let Kind::Crop {
            fruit_vine: Some(fruit),
            harvest_to: None,
            ..
        } = &plant.kind
        else {
            panic!("{crop} is a vine crop");
        };
        assert_eq!((fruit.reach_m, fruit.gap_m, fruit.max_fruit), (1.5, 0.5, 5));
        let ripe = material(&loaded, &format!("crops:{crop}_ripe"));
        let unripe = material(&loaded, &format!("crops:{crop}_unripe"));
        assert!(plants.is_grown(ripe) && plants.is_grown(unripe), "{crop}");
        assert!(plants.yields(unripe).is_empty());
        assert_eq!(plants.yields(ripe)[0].item, ripe_yield);
        assert_eq!(plants.canopy_of_fruit(ripe), None, "{crop} leaves air");
        let items = loaded.every_item(0.5);
        assert!(items.named(seeds).is_some(), "{seeds}");
        let food = loaded.items.iter().find(|i| i.name == ripe_yield).unwrap();
        assert!(food.food.is_some_and(|f| f > 0.0), "{ripe_yield} is food");
        assert!(
            items.dropped_by(ripe).is_none() && items.dropped_by(unripe).is_none(),
            "{crop} fruit has no Item that places it"
        );
    }
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
    use super::food::{SHORT_WALK_M, around_spawn, new_world, search_around_m};
    let (_install, loaded) = loaded("terra", 0.5);
    let generator = new_world(&loaded, 7, 0.5);
    let groups = [
        &["flora:oak_leaves"][..],
        &["flora:apple_unripe", "flora:apple_ripe"],
        &[
            "crops:wheat_sprout",
            "crops:wheat_young",
            "crops:wheat_green",
            "crops:wheat_ripe",
        ],
    ];
    let (materials, group): (Vec<_>, Vec<_>) = groups
        .iter()
        .enumerate()
        .flat_map(|(g, names)| names.iter().map(move |n| (n, g)))
        .map(|(n, g)| (material(&loaded, n), g))
        .unzip();
    let mut found = [false; 3];
    let around = around_spawn(&generator);
    search_around_m(&generator, &materials, SHORT_WALK_M, around, |i, _| {
        found[group[i]] = true;
        if found.iter().all(|&f| f) {
            std::ops::ControlFlow::Break(())
        } else {
            std::ops::ControlFlow::Continue(())
        }
    });
    for (names, found) in groups.iter().zip(found) {
        assert!(found, "{names:?} within {SHORT_WALK_M} m");
    }
}

#[test]
fn terras_plants_name_their_shapes_and_a_generated_apple_is_the_box_apple() {
    use super::food::{SHORT_WALK_M, new_world};
    use super::structures::settled_rows;
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

    let generator = new_world(&loaded, 7, 0.5);
    let small = shape(&loaded, "flora:apple_unripe");
    let drawn = [
        (material(&loaded, "flora:apple_unripe"), small),
        (material(&loaded, "flora:apple_ripe"), apple),
    ];
    let chunk_m = CHUNK_SIZE as f32 * 0.5;
    let reach = (SHORT_WALK_M / chunk_m).ceil() as i32;
    let eye = crate::server::spawn_eye(&generator);
    let [ox, oz] = [eye.x, eye.z].map(|m| (m / chunk_m).floor() as i32);
    let mut found = 0;
    let mut seen = [false; 2];
    for cz in oz - reach..oz + reach {
        for cx in ox - reach..ox + reach {
            // An apple hangs on a tree standing on the ground.
            for cy in settled_rows(&generator, [cx, cz]) {
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

/// A flat floor of solid ground under the fruit that `fruit` names, for walking a body over it.
struct Patch<'a> {
    ground: tesserae_core::Voxel,
    fruit: &'a std::collections::HashMap<[i32; 3], tesserae_core::Voxel>,
}

impl tesserae_core::collide::VoxelVolume for Patch<'_> {
    fn voxel(&self, at: [i32; 3]) -> Option<tesserae_core::Voxel> {
        Some(match at[1] {
            ..0 => self.ground,
            0 => self
                .fruit
                .get(&at)
                .copied()
                .unwrap_or(tesserae_core::Voxel::AIR),
            _ => tesserae_core::Voxel::AIR,
        })
    }
}

/// The Player's box, 0.4 m wide and 1.8 m tall, standing on the floor at `(x, z)`.
fn player_at(x: f32, z: f32) -> tesserae_core::collide::Aabb {
    tesserae_core::collide::Aabb::new([x - 0.2, 0.0, z - 0.2], [x + 0.2, 1.8, z + 0.2])
}

#[test]
fn watermelons_and_pumpkins_block_a_walking_body_but_berries_do_not() {
    use tesserae_core::collide::overlaps;
    for size in [1.0, 0.5, 0.25] {
        let (_install, loaded) = loaded("terra", size);
        let fruit_voxel = |name: &str, shape_name: &str| {
            tesserae_core::Voxel::new(material(&loaded, name), shape(&loaded, shape_name), 0)
        };
        let ground = tesserae_core::Voxel::cube(material(&loaded, "strata:stone"));
        let cases = [
            ("crops:watermelon_ripe", "crops:watermelon", true),
            ("crops:watermelon_unripe", "crops:watermelon_unripe", true),
            ("crops:pumpkin_ripe", "crops:pumpkin", true),
            ("crops:pumpkin_unripe", "crops:pumpkin_unripe", true),
            ("crops:berry_ripe", "crops:berry_ripe", false),
        ];
        for (name, shape_name, blocks) in cases {
            let cells = [([0, 0, 0], fruit_voxel(name, shape_name))].into();
            let volume = Patch {
                ground,
                fruit: &cells,
            };
            let centre = size / 2.0;
            let body = tesserae_core::collide::Aabb::new(
                [centre - 0.05, 0.0, centre - 0.05],
                [centre + 0.05, 1.0, centre + 0.05],
            );
            assert_eq!(
                overlaps(&volume, &loaded.tables, size, body),
                blocks,
                "{name} at {size} m"
            );
        }
    }
}

/// A vine's fruit stands a `gap` of bare ground apart: a body fits between two at the default gap
/// and, at the narrowest gap a Voxel allows, still walks round a whole row of them.
#[test]
fn a_body_can_pass_between_fruit_and_walk_round_a_row_of_it_at_every_size() {
    use std::collections::{HashMap, HashSet, VecDeque};
    use tesserae_core::collide::overlaps;
    use tesserae_core::plant::Kind;
    const UNIT_M: f32 = 0.05;
    for size in [1.0, 0.5, 0.25] {
        let (_install, loaded) = loaded("terra", size);
        let ground = tesserae_core::Voxel::cube(material(&loaded, "strata:stone"));
        let melon = tesserae_core::Voxel::new(
            material(&loaded, "crops:watermelon_ripe"),
            shape(&loaded, "crops:watermelon"),
            0,
        );
        let gap_m = loaded
            .tables
            .plants
            .plants()
            .iter()
            .find_map(|p| match &p.kind {
                Kind::Crop {
                    fruit_vine: Some(f),
                    ..
                } if p.name == "crops:watermelon" => Some(f.gap_m),
                _ => None,
            })
            .unwrap();
        let row = |gap_voxels: i32| -> (HashMap<[i32; 3], tesserae_core::Voxel>, i32) {
            let step = gap_voxels + 1;
            ((0..5).map(|i| ([i * step, 0, 0], melon)).collect(), step)
        };

        let default_gap = ((gap_m / size).ceil() as i32).max(1);
        let (cells, step) = row(default_gap);
        let volume = Patch {
            ground,
            fruit: &cells,
        };
        let between = player_at((step as f32 + 1.0) / 2.0 * size, 0.5 * size);
        assert!(
            !overlaps(&volume, &loaded.tables, size, between),
            "a body fits between two fruit {default_gap} Voxel(s) apart at {size} m"
        );

        let (cells, step) = row(1);
        let volume = Patch {
            ground,
            fruit: &cells,
        };
        let end_m = (4 * step) as f32 * size;
        let (low, high) = (-2.0_f32, end_m + 2.0);
        let side = ((high - low) / UNIT_M) as i32;
        let index = |m: f32| ((m - low) / UNIT_M).round() as i32;
        let at = |i: i32, j: i32| player_at(low + i as f32 * UNIT_M, low + j as f32 * UNIT_M);
        let (start, goal) = (
            (index(-1.5), index(0.5 * size)),
            (index(end_m + 1.5), index(0.5 * size)),
        );
        let mut seen = HashSet::from([start]);
        let mut queue = VecDeque::from([start]);
        let mut crossed = false;
        while let Some((i, j)) = queue.pop_front() {
            crossed |= (i, j) == goal;
            for (di, dj) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let next = (i + di, j + dj);
                if (0..side).contains(&next.0)
                    && (0..side).contains(&next.1)
                    && !overlaps(&volume, &loaded.tables, size, at(next.0, next.1))
                    && seen.insert(next)
                {
                    queue.push_back(next);
                }
            }
        }
        assert!(
            crossed,
            "a body walks round a row of fruit one Voxel apart at {size} m"
        );
    }
}
