//! What each demo Game's Biomes grow: a sample of generated Chunks counted by Plant and Material
//! name, so a Biome that loses a species, goes bare by accident or sprouts standalone Wood fails.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use tesserae_core::plant::Kind as PlantKind;
use tesserae_core::{CHUNK_SIZE, Worldgen};
use tesserae_mods::Kind;

use super::{loaded, material, shape};

const TREES: [&str; 10] = [
    "oak", "birch", "pine", "palm", "acacia", "jungle", "swamp", "apple", "orange", "cherry",
];
/// Plants that stand in water: they generate only where the Game's lake covers the column.
const WATER_PLANTS: [&str; 3] = ["aquatic:kelp", "aquatic:seagrass", "aquatic:red_weed"];
/// The Biomes of a Game whose trees hang vines.
const VINED: [&str; 2] = ["flora:jungle", "flora:swamp"];

struct Game {
    name: &'static str,
    /// Each Biome's declared species, in declaration order of the Biome table.
    biomes: &'static [(&'static str, &'static [&'static str])],
    /// Biomes declared bare on purpose.
    bare: &'static [&'static str],
    lake: bool,
}

const GAMES: [Game; 5] = [
    Game {
        name: "terra",
        biomes: &[
            (
                "temperate",
                &[
                    "flora:cherry",
                    "flora:apple",
                    "flora:birch",
                    "flora:oak",
                    "crops:potato",
                    "crops:carrot",
                    "crops:wheat",
                    "crops:berry",
                    "crops:watermelon",
                    "crops:pumpkin",
                    "flora:mushroom_red",
                    "flora:daisy",
                    "flora:poppy",
                    "flora:tuft",
                ],
            ),
            (
                "dry",
                &[
                    "flora:orange",
                    "flora:palm",
                    "flora:cactus",
                    "crops:berry",
                    "flora:cane",
                    "flora:dandelion",
                    "flora:dry_grass",
                ],
            ),
            (
                "cold",
                &[
                    "flora:birch",
                    "flora:pine",
                    "crops:berry",
                    "flora:mushroom_brown",
                    "flora:daisy",
                    "flora:fern",
                    "flora:tuft",
                ],
            ),
            (
                "jungle",
                &[
                    "flora:jungle",
                    "flora:palm",
                    "flora:orchid",
                    "crops:berry",
                    "flora:cane",
                    "flora:mushroom_red",
                    "flora:fern",
                    "flora:tall_grass",
                ],
            ),
            (
                "swamp",
                &[
                    "flora:swamp",
                    "flora:orchid",
                    "crops:berry",
                    "flora:mushroom_brown",
                    "flora:cane",
                    "aquatic:kelp",
                    "aquatic:seagrass",
                    "aquatic:red_weed",
                    "flora:fern",
                    "flora:tuft",
                ],
            ),
            (
                "savanna",
                &[
                    "flora:acacia",
                    "flora:cactus",
                    "crops:wheat",
                    "crops:watermelon",
                    "flora:poppy",
                    "flora:dandelion",
                    "flora:tuft",
                    "flora:dry_grass",
                ],
            ),
        ],
        bare: &[],
        lake: true,
    },
    Game {
        name: "meadow",
        biomes: &[
            (
                "meadow",
                &[
                    "flora:cherry",
                    "flora:apple",
                    "flora:oak",
                    "crops:potato",
                    "crops:watermelon",
                    "crops:pumpkin",
                    "crops:carrot",
                    "crops:wheat",
                    "flora:dandelion",
                    "flora:daisy",
                    "flora:poppy",
                    "flora:tuft",
                ],
            ),
            (
                "heath",
                &[
                    "flora:swamp",
                    "flora:birch",
                    "crops:berry",
                    "crops:pumpkin",
                    "flora:mushroom_brown",
                    "flora:fern",
                    "flora:tall_grass",
                ],
            ),
            (
                "dunes",
                &[
                    "flora:palm",
                    "flora:cactus",
                    "flora:cane",
                    "flora:dry_grass",
                ],
            ),
            (
                "thicket",
                &[
                    "flora:jungle",
                    "flora:orchid",
                    "crops:berry",
                    "flora:mushroom_red",
                    "flora:fern",
                    "flora:tuft",
                ],
            ),
            (
                "fen",
                &[
                    "flora:swamp",
                    "flora:cane",
                    "flora:mushroom_brown",
                    "flora:fern",
                    "flora:tuft",
                ],
            ),
            (
                "prairie",
                &[
                    "flora:acacia",
                    "crops:wheat",
                    "crops:pumpkin",
                    "flora:poppy",
                    "flora:dandelion",
                    "flora:tuft",
                    "flora:dry_grass",
                ],
            ),
        ],
        bare: &[],
        lake: false,
    },
    Game {
        name: "ember",
        biomes: &[
            (
                "ash_flats",
                &[
                    "flora:birch",
                    "crops:berry",
                    "crops:pumpkin",
                    "flora:poppy",
                    "flora:dry_grass",
                ],
            ),
            ("scorched", &[]),
            (
                "highland",
                &[
                    "flora:pine",
                    "flora:birch",
                    "flora:mushroom_brown",
                    "flora:fern",
                    "flora:tuft",
                ],
            ),
            (
                "cinder_bog",
                &[
                    "flora:swamp",
                    "flora:cane",
                    "flora:mushroom_red",
                    "flora:fern",
                    "flora:tall_grass",
                ],
            ),
        ],
        bare: &["scorched"],
        lake: true,
    },
    Game {
        name: "tidewater",
        biomes: &[
            (
                "lagoon",
                &[
                    "flora:jungle",
                    "flora:palm",
                    "crops:berry",
                    "crops:watermelon",
                    "flora:cane",
                    "aquatic:seagrass",
                    "aquatic:kelp",
                    "flora:fern",
                ],
            ),
            (
                "salt_flats",
                &[
                    "flora:palm",
                    "flora:cactus",
                    "flora:cane",
                    "flora:dry_grass",
                ],
            ),
            (
                "cold_shallows",
                &["flora:pine", "crops:berry", "aquatic:kelp", "flora:tuft"],
            ),
            (
                "rainforest",
                &[
                    "flora:jungle",
                    "flora:palm",
                    "flora:orchid",
                    "flora:mushroom_red",
                    "flora:cane",
                    "aquatic:kelp",
                    "flora:fern",
                ],
            ),
            (
                "tidal_marsh",
                &[
                    "flora:swamp",
                    "flora:cane",
                    "flora:mushroom_brown",
                    "aquatic:red_weed",
                    "aquatic:kelp",
                    "flora:fern",
                ],
            ),
        ],
        bare: &[],
        lake: true,
    },
    Game {
        name: "lumen",
        biomes: &[
            (
                "garden",
                &[
                    "flora:cherry",
                    "flora:apple",
                    "crops:carrot",
                    "crops:wheat",
                    "crops:pumpkin",
                    "crops:watermelon",
                    "flora:dandelion",
                    "flora:daisy",
                    "flora:poppy",
                    "flora:fern",
                    "flora:tuft",
                ],
            ),
            (
                "arid_court",
                &[
                    "flora:palm",
                    "flora:orange",
                    "flora:cactus",
                    "flora:dry_grass",
                ],
            ),
            (
                "frost_court",
                &[
                    "flora:pine",
                    "flora:birch",
                    "crops:berry",
                    "flora:mushroom_brown",
                    "flora:tall_grass",
                ],
            ),
            (
                "conservatory",
                &[
                    "flora:jungle",
                    "flora:cherry",
                    "flora:orange",
                    "flora:palm",
                    "flora:orchid",
                    "flora:fern",
                ],
            ),
            (
                "lily_pond",
                &[
                    "flora:swamp",
                    "flora:cherry",
                    "flora:cane",
                    "flora:orchid",
                    "flora:fern",
                    "flora:tuft",
                ],
            ),
        ],
        bare: &[],
        lake: false,
    },
];

/// The Materials that show a Plant stands: a tree's trunk, a crop's every Stage, else its own.
fn materials_of(plant: &str) -> Vec<String> {
    let (owner, name) = plant.split_once(':').unwrap();
    if owner == "flora" && TREES.contains(&name) {
        return vec![format!("flora:{name}_log")];
    }
    match plant {
        "crops:berry" => ["sprout", "bush", "ripe"]
            .map(|stage| format!("crops:berry_{stage}"))
            .into(),
        _ if owner == "crops" => ["sprout", "young", "green", "ripe"]
            .map(|stage| format!("{plant}_{stage}"))
            .into(),
        _ => vec![plant.to_owned()],
    }
}

fn is_standalone_wood(name: &str) -> bool {
    name.rsplit(':')
        .next()
        .is_some_and(|n| n == "wood" || n.ends_with("_wood"))
}

/// Chunk columns sampled per Biome, each 32 m square at a 1 m Voxel.
const SAMPLES: usize = 32;

/// `(Biome index, Material name)` to Voxels found, over a sample of every Biome's ground.
fn census(game: &Game) -> (BTreeMap<(usize, String), u64>, Vec<String>) {
    let (_install, loaded) = loaded(game.name, 1.0);
    let world = loaded.world.unwrap();
    let biomes: Vec<String> = world
        .biomes
        .as_slice()
        .iter()
        .map(|b| b.name.as_str().to_owned())
        .collect();
    let named: HashMap<u16, String> = loaded
        .registry
        .names(Kind::Material)
        .filter(|(_, name)| {
            let tracked = name.starts_with("flora:")
                || name.starts_with("crops:")
                || name.starts_with("aquatic:")
                || is_standalone_wood(name);
            tracked && !name.ends_with("_leaves")
        })
        .map(|(id, name)| (id as u16, name.to_owned()))
        .collect();

    let generator = Worldgen::new(7, 1.0, world);
    let chunk_m = CHUNK_SIZE as f32;
    let (low, high) = generator.surface_bounds_m();
    let rows = (low / chunk_m).floor() as i32..=(high / chunk_m).ceil() as i32;
    let mut taken = vec![0usize; biomes.len()];
    let mut counts = BTreeMap::new();
    for cz in (-120..120).step_by(3) {
        for cx in (-120..120).step_by(3) {
            let centre = ((cx as f32 + 0.5) * chunk_m, (cz as f32 + 0.5) * chunk_m);
            let biome = generator.biome_at(centre.0, centre.1).unwrap().index();
            if taken[biome] >= SAMPLES {
                continue;
            }
            taken[biome] += 1;
            for cy in rows.clone() {
                let chunk = generator.chunk(cx, cy, cz);
                if chunk.uniform().is_some() {
                    continue;
                }
                for z in 0..CHUNK_SIZE {
                    for y in 0..CHUNK_SIZE {
                        for x in 0..CHUNK_SIZE {
                            let Some(name) = named.get(&chunk.get(x, y, z).material()) else {
                                continue;
                            };
                            let at = |c: i32, n: usize| (c * CHUNK_SIZE as i32 + n as i32) as f32;
                            let here = generator.biome_at(at(cx, x), at(cz, z));
                            let here = here.unwrap().index();
                            *counts.entry((here, name.clone())).or_default() += 1;
                        }
                    }
                }
            }
        }
    }
    assert!(
        taken.iter().all(|&n| n > 0),
        "{}: a Biome has no ground within 3.8 km: {taken:?}",
        game.name
    );
    (counts, biomes)
}

fn found(counts: &BTreeMap<(usize, String), u64>, biome: usize, name: &str) -> u64 {
    counts.get(&(biome, name.to_owned())).copied().unwrap_or(0)
}

/// Trunk Voxels a bare Biome may hold from the limbs of a neighbour's trees.
const OVERHANG_LIMIT: u64 = 100;

#[test]
fn every_biome_of_every_demo_game_grows_its_declared_species_and_none_is_bare_by_accident() {
    for game in &GAMES {
        let (counts, biomes) = census(game);
        let (_install, loaded) = loaded(game.name, 1.0);
        let world = loaded.world.unwrap();
        assert_eq!(biomes.len(), game.biomes.len(), "{}", game.name);
        for (index, (name, species)) in game.biomes.iter().enumerate() {
            let full = format!("{}:{name}", game.name);
            assert_eq!(biomes[index], full, "Biome order");
            let declared = world.biomes.as_slice()[index].surface_features.as_slice();
            let food = declared.iter().filter(|f| f.plant.is_none()).count();
            assert_eq!(
                declared.len() - food,
                species.len(),
                "{full} declares exactly the table's Plants"
            );
            assert_eq!(
                declared.is_empty(),
                game.bare.contains(name),
                "{full} is bare only if declared so"
            );
            for plant in *species {
                if WATER_PLANTS.contains(plant) && !game.lake {
                    continue;
                }
                let total: u64 = materials_of(plant)
                    .iter()
                    .map(|material| found(&counts, index, material))
                    .sum();
                assert!(total > 0, "{full} grows no {plant}");
            }
            if species.is_empty() {
                // A neighbour's vines hang across the border, and its limbs reach a few Voxels
                // over it; a tree standing here would be hundreds.
                let plants: Vec<_> = counts
                    .iter()
                    .filter(|((b, name), n)| {
                        *b == index
                            && name != "flora:vine"
                            && !(name.ends_with("_log") && **n < OVERHANG_LIMIT)
                    })
                    .collect();
                assert!(plants.is_empty(), "{full} is bare on purpose: {plants:?}");
            }
        }
    }
}

#[test]
fn jungle_and_swamp_trees_hang_vines() {
    for game in &GAMES {
        let (counts, _) = census(game);
        for (index, (name, species)) in game.biomes.iter().enumerate() {
            if species.iter().any(|p| VINED.contains(p)) {
                assert!(
                    found(&counts, index, "flora:vine") > 0,
                    "{}:{name} has no vines",
                    game.name
                );
            }
        }
    }
}

#[test]
fn generated_wood_is_only_ever_the_trunk_of_a_grown_tree() {
    for game in &GAMES {
        let (counts, _) = census(game);
        let logs: u64 = counts
            .iter()
            .filter(|((_, name), _)| name.ends_with("_log"))
            .map(|(_, n)| n)
            .sum();
        assert!(logs > 0, "{}: no tree trunks generated", game.name);
        let standalone: Vec<_> = counts
            .iter()
            .filter(|((_, name), _)| is_standalone_wood(name))
            .collect();
        assert!(
            standalone.is_empty(),
            "{}: standalone Wood generated: {standalone:?}",
            game.name
        );
    }
}

/// A Plant is declared once, in a shared Mod, and a Game only places it: each is placed by at least
/// two Games, vines through the trees that hang them.
#[test]
fn every_shared_plant_is_placed_by_at_least_two_games() {
    let (_install, loaded) = loaded("terra", 1.0);
    let declared: BTreeSet<String> = loaded
        .tables
        .plants
        .plants()
        .iter()
        .map(|p| p.name.clone())
        .filter(|name| {
            ["flora:", "crops:", "aquatic:"]
                .iter()
                .any(|owner| name.starts_with(owner))
        })
        .filter(|name| !name.ends_with("_sapling"))
        .collect();
    assert!(declared.len() >= 26, "{declared:?}");
    for plant in &declared {
        let games = GAMES
            .iter()
            .filter(|game| {
                game.biomes.iter().any(|(_, species)| {
                    species.contains(&plant.as_str())
                        || (plant == "flora:vine" && species.iter().any(|p| VINED.contains(p)))
                })
            })
            .count();
        assert!(games >= 2, "{plant} is placed by {games} Game(s)");
    }
}

/// Every fruit tree's fruit ripens through its Stages, is broken for food and left as canopy, the
/// tree growing new fruit after `regrow-seconds`, and is drawn as its own 3D box Shape that no face
/// of seals.
#[test]
fn every_fruit_is_a_box_shape_that_breaking_picks_and_the_tree_grows_again() {
    let (_install, loaded) = loaded("terra", 0.5);
    let plants = &loaded.tables.plants;
    for (tree, fruit, item) in [
        ("apple", "apple", "flora:apple"),
        ("orange", "orange", "flora:orange"),
        ("cherry", "cherry", "flora:cherries"),
        ("palm", "coconut", "flora:coconut"),
    ] {
        let ripe = material(&loaded, &format!("flora:{fruit}_ripe"));
        assert!(
            plants.harvest(ripe).is_none(),
            "{fruit} is broken, not reset"
        );
        assert_eq!(
            plants.canopy_of_fruit(ripe),
            Some(material(&loaded, &format!("flora:{tree}_leaves"))),
            "{fruit} leaves its canopy"
        );
        assert_eq!(plants.yields(ripe)[0].item, item);
        let food = loaded.items.iter().find(|i| i.name == item).unwrap();
        assert!(food.food.is_some_and(|f| f > 0.0), "{item} feeds");

        let boxed = shape(&loaded, &format!("flora:{fruit}"));
        let PlantKind::Tree(row) = &plants
            .plants()
            .iter()
            .find(|p| p.name == format!("flora:{tree}"))
            .unwrap()
            .kind
        else {
            panic!("{tree} is a tree")
        };
        let bears = row.fruit.as_ref().unwrap();
        assert_eq!(bears.shape, Some(boxed), "{fruit}");
        assert!(bears.regrow_seconds >= 16.0, "{fruit} regrows");
        let small = shape(&loaded, &format!("flora:{fruit}_unripe"));
        let unripe = material(&loaded, &format!("flora:{fruit}_unripe"));
        assert_eq!(plants.stage_shape(unripe), Some(small), "{fruit} unripe");
        assert_eq!(plants.stage_shape(ripe), Some(boxed), "{fruit} ripe");
        let volume = |shape| {
            let voxel = tesserae_core::Voxel::new(ripe, shape, 0);
            loaded.tables.shapes.volume(voxel)
        };
        assert!(volume(small) < volume(boxed), "{fruit} unripe is smaller");
        for (stage, shape) in [(ripe, boxed), (unripe, small)] {
            for face in 0..6 {
                let voxel = tesserae_core::Voxel::new(stage, shape, 0);
                assert!(
                    !loaded.tables.shapes.seals(voxel, face),
                    "{fruit} face {face}"
                );
            }
        }
    }
}

/// Trees per hectare in each Biome, by trunk feet found in a sample of Chunks and the ground those
/// Chunks' columns of the Biome cover.
fn trees_per_hectare(game: &Game) -> Vec<(String, f64)> {
    let (_install, loaded) = loaded(game.name, 1.0);
    let world = loaded.world.unwrap();
    let biomes: Vec<String> = world
        .biomes
        .as_slice()
        .iter()
        .map(|b| b.name.as_str().rsplit(':').next().unwrap().to_owned())
        .collect();
    let logs: BTreeSet<u16> = loaded
        .registry
        .names(Kind::Material)
        .filter(|(_, name)| name.starts_with("flora:") && name.ends_with("_log"))
        .map(|(id, _)| id as u16)
        .collect();
    let plant_made: BTreeSet<u16> = loaded
        .registry
        .names(Kind::Material)
        .filter(|(_, name)| {
            // Ember and the other Games with their own ground stand on `flora:grass`.
            *name != "flora:grass"
                && ["flora:", "crops:", "aquatic:"]
                    .iter()
                    .any(|owner| name.starts_with(owner))
        })
        .map(|(id, _)| id as u16)
        .collect();
    let generator = Worldgen::new(7, 1.0, world);
    let chunk_m = CHUNK_SIZE as f32;
    let (low, high) = generator.surface_bounds_m();
    let rows = (low / chunk_m).floor() as i32..=(high / chunk_m).ceil() as i32;
    let mut taken = vec![0usize; biomes.len()];
    let mut area_m2 = vec![0.0f64; biomes.len()];
    let mut feet = vec![0u64; biomes.len()];
    let at = |c: i32, n: usize| (c * CHUNK_SIZE as i32 + n as i32) as f32;
    for cz in (-120..120).step_by(3) {
        for cx in (-120..120).step_by(3) {
            let centre = ((cx as f32 + 0.5) * chunk_m, (cz as f32 + 0.5) * chunk_m);
            let biome = generator.biome_at(centre.0, centre.1).unwrap().index();
            if taken[biome] >= SAMPLES {
                continue;
            }
            taken[biome] += 1;
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let here = generator.biome_at(at(cx, x), at(cz, z)).unwrap().index();
                    area_m2[here] += 1.0;
                }
            }
            for cy in rows.clone() {
                let chunk = generator.chunk(cx, cy, cz);
                if chunk.uniform().is_some() {
                    continue;
                }
                for z in 0..CHUNK_SIZE {
                    for y in 1..CHUNK_SIZE {
                        for x in 0..CHUNK_SIZE {
                            let voxel = chunk.get(x, y, z);
                            let below = chunk.get(x, y - 1, z);
                            if logs.contains(&voxel.material())
                                && !plant_made.contains(&below.material())
                                && !below.is_air()
                            {
                                let here = generator.biome_at(at(cx, x), at(cz, z)).unwrap();
                                feet[here.index()] += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    biomes
        .into_iter()
        .zip(feet.into_iter().zip(area_m2))
        .map(|(name, (feet, area))| (name, feet as f64 / (area / 10_000.0).max(1e-9)))
        .collect()
}

fn mean_spacing_m(per_hectare: f64) -> f64 {
    100.0 / per_hectare.max(1e-9).sqrt()
}

/// The approved mockup's density classes, by Biome name across the Games: a jungle or swamp packs
/// trees about 4 m apart, a temperate or cold forest 6 to 10 m, a savanna, prairie, desert or
/// frost Biome 12 m or more.
const DENSE: [&str; 9] = [
    "jungle",
    "swamp",
    "thicket",
    "fen",
    "cinder_bog",
    "rainforest",
    "tidal_marsh",
    "conservatory",
    "lily_pond",
];
const MODERATE: [&str; 4] = ["temperate", "cold", "heath", "cold_shallows"];
const SPARSE: [&str; 12] = [
    "savanna",
    "dry",
    "prairie",
    "dunes",
    "salt_flats",
    "arid_court",
    "meadow",
    "garden",
    "highland",
    "frost_court",
    "ash_flats",
    "lagoon",
];

#[test]
fn each_biome_scatters_its_trees_at_its_density_class() {
    for game in &GAMES {
        for (name, per_ha) in trees_per_hectare(game) {
            let spacing = mean_spacing_m(per_ha);
            let full = format!("{}:{name}", game.name);
            if DENSE.contains(&name.as_str()) {
                assert!(
                    (2.5..=6.0).contains(&spacing),
                    "{full}: dense, {spacing:.1} m"
                );
            } else if MODERATE.contains(&name.as_str()) {
                assert!(
                    (5.5..=11.0).contains(&spacing),
                    "{full}: moderate, {spacing:.1} m"
                );
            } else if SPARSE.contains(&name.as_str()) {
                assert!(spacing >= 12.0, "{full}: sparse, {spacing:.1} m");
            } else {
                assert!(
                    game.bare.contains(&name.as_str()) || name == "scorched",
                    "{full}: unclassed"
                );
            }
        }
    }
}

#[test]
fn jungle_and_swamp_are_denser_than_savanna_prairie_and_desert_in_every_game_that_has_both() {
    let mut compared = 0;
    for game in &GAMES {
        let density = trees_per_hectare(game);
        let of = |class: &[&str]| -> Vec<(String, f64)> {
            density
                .iter()
                .filter(|(n, _)| class.contains(&n.as_str()))
                .cloned()
                .collect()
        };
        let (dense, sparse) = (of(&DENSE), of(&SPARSE));
        if dense.is_empty() || sparse.is_empty() {
            continue;
        }
        compared += 1;
        let thinnest = dense.iter().map(|d| d.1).fold(f64::MAX, f64::min);
        let thickest = sparse.iter().map(|d| d.1).fold(0.0, f64::max);
        assert!(
            thinnest > 2.0 * thickest,
            "{}: dense {dense:?} is not clearly past sparse {sparse:?}",
            game.name
        );
    }
    assert_eq!(compared, 5);
}

/// Climate is a continuous field and a Biome is the nearest point, so a tree's band must hold
/// across the Biome's region and not only at its point.
#[test]
fn terra_trees_find_their_climate_across_their_whole_biome() {
    const NATIVE: [(&str, &str); 11] = [
        ("oak", "temperate"),
        ("birch", "temperate"),
        ("apple", "temperate"),
        ("cherry", "temperate"),
        ("pine", "cold"),
        ("palm", "dry"),
        ("orange", "dry"),
        ("jungle", "jungle"),
        ("palm", "jungle"),
        ("swamp", "swamp"),
        ("acacia", "savanna"),
    ];
    let (_install, loaded) = loaded("terra", 1.0);
    let world = loaded.world.unwrap();
    let biomes: Vec<String> = world
        .biomes
        .as_slice()
        .iter()
        .map(|b| b.name.as_str().to_owned())
        .collect();
    let generator = Worldgen::new(7, 1.0, world);
    for (tree, biome) in NATIVE {
        let sapling = material(&loaded, &format!("flora:{tree}_sapling"));
        let needs = &loaded
            .tables
            .plants
            .plant(loaded.tables.plants.of(sapling).unwrap().plant)
            .unwrap()
            .needs;
        let index = biomes
            .iter()
            .position(|b| *b == format!("terra:{biome}"))
            .unwrap();
        let (mut inside, mut fit) = (0, 0);
        for z in (-4000..4000).step_by(32) {
            for x in (-4000..4000).step_by(32) {
                let (x, z) = (x as f32, z as f32);
                if generator.biome_at(x, z).map(|b| b.index()) != Some(index) {
                    continue;
                }
                let climate = generator.climate_at(x, z);
                let within = |range: Option<(f32, f32)>, v: f32| {
                    range.is_none_or(|(low, high)| (low..=high).contains(&v))
                };
                inside += 1;
                fit += (within(needs.temperature, climate.temperature)
                    && within(needs.humidity, climate.humidity)) as u32;
            }
        }
        assert!(inside > 500, "{tree}: {biome} is sampled");
        assert!(
            fit as f32 >= 0.95 * inside as f32,
            "{tree} fits only {fit} of {inside} columns of {biome}"
        );
    }
}

/// How many of a Biome's dry ground columns have a top ground Voxel that is soil to `plant`,
/// out of the columns counted. Plants and trees standing on the ground are looked through; a
/// lake is not ground.
fn soil_share(game: &str, biome: &str, plant: &str) -> (u32, u32) {
    let (_install, loaded) = loaded(game, 1.0);
    let world = loaded.world.unwrap();
    let index = world
        .biomes
        .as_slice()
        .iter()
        .position(|b| b.name.as_str() == format!("{game}:{biome}"))
        .unwrap();
    let sapling = material(&loaded, &format!("flora:{plant}_sapling"));
    let soil = loaded
        .tables
        .plants
        .plant(loaded.tables.plants.of(sapling).unwrap().plant)
        .unwrap()
        .needs
        .soil
        .clone();
    let ground: HashMap<u16, String> = loaded
        .registry
        .names(Kind::Material)
        .filter(|(_, name)| name.starts_with(&format!("{game}:")))
        .map(|(id, name)| (id as u16, name.to_owned()))
        .collect();
    let generator = Worldgen::new(7, 1.0, world);
    let n = CHUNK_SIZE as i32;
    let mut chunks = HashMap::new();
    let (mut columns, mut soiled) = (0, 0);
    for z in (-4000..4000_i32).step_by(64) {
        for x in (-4000..4000_i32).step_by(64) {
            if generator.biome_at(x as f32, z as f32).map(|b| b.index()) != Some(index) {
                continue;
            }
            let top = generator.surface_height_m(x as f32, z as f32).ceil() as i32 + 14;
            let found = (top - 20..=top).rev().find_map(|y| {
                let key = (x.div_euclid(n), y.div_euclid(n), z.div_euclid(n));
                let chunk = chunks
                    .entry(key)
                    .or_insert_with(|| generator.chunk(key.0, key.1, key.2));
                let v = chunk.get(
                    x.rem_euclid(n) as usize,
                    y.rem_euclid(n) as usize,
                    z.rem_euclid(n) as usize,
                );
                ground.contains_key(&v.material()).then(|| v.material())
            });
            let Some(found) = found else { continue };
            if ground[&found].ends_with(":water") {
                continue;
            }
            columns += 1;
            soiled += soil.contains(&found) as u32;
        }
    }
    (soiled, columns)
}

#[test]
fn terra_pines_find_soil_on_the_cold_biomes_snow() {
    let (soiled, columns) = soil_share("terra", "cold", "pine");
    println!("pine on cold: {soiled}/{columns}");
    assert!(columns > 500);
    assert!(
        soiled as f32 >= 0.95 * columns as f32,
        "only {soiled} of {columns} cold columns are soil to a pine"
    );
}

/// Plants that cover the ground, as opposed to trees, cacti and water plants.
const GROUND_COVER: [&str; 15] = [
    "flora:tuft",
    "flora:tall_grass",
    "flora:dry_grass",
    "flora:poppy",
    "flora:daisy",
    "flora:dandelion",
    "flora:fern",
    "flora:orchid",
    "flora:mushroom_red",
    "flora:mushroom_brown",
    "flora:cane",
    "crops:potato",
    "crops:carrot",
    "crops:wheat",
    "crops:berry",
];
const FLOWERS: [&str; 4] = [
    "flora:poppy",
    "flora:daisy",
    "flora:dandelion",
    "flora:orchid",
];
const GRASSES: [&str; 4] = [
    "flora:tuft",
    "flora:tall_grass",
    "flora:dry_grass",
    "flora:fern",
];

/// Side of a measured block, in columns at a 1 m Voxel: two Chunks.
const BLOCK: usize = 2 * CHUNK_SIZE;
/// Blocks measured per Biome.
const BLOCKS: usize = 12;

/// Blocks of two by two Chunks lying wholly in one Biome, by Biome index: each block's columns, the
/// [`GROUND_COVER`] Plant standing in each if any.
fn ground_of(game: &Game, seed: u64) -> Vec<Vec<Vec<Option<u8>>>> {
    let (_install, loaded) = loaded(game.name, 1.0);
    let world = loaded.world.unwrap();
    let cover: HashMap<u16, u8> = GROUND_COVER
        .iter()
        .enumerate()
        .flat_map(|(i, plant)| {
            materials_of(plant)
                .into_iter()
                .filter_map(|m| loaded.registry.id_of(Kind::Material, &m))
                .map(move |id| (id as u16, i as u8))
        })
        .collect();
    let mut out = vec![Vec::new(); world.biomes.as_slice().len()];
    let generator = Worldgen::new(seed, 1.0, world);
    let chunk_m = CHUNK_SIZE as f32;
    let (low, high) = generator.surface_bounds_m();
    let rows = (low / chunk_m).floor() as i32..=(high / chunk_m).ceil() as i32;
    let biome_of = |c: i32, d: i32| {
        let centre = |c: i32| (c as f32 + 0.5) * chunk_m;
        generator.biome_at(centre(c), centre(d)).unwrap().index()
    };
    for cz in (-120..120).step_by(4) {
        for cx in (-120..120).step_by(4) {
            let biome = biome_of(cx, cz);
            let whole = [(0, 1), (1, 0), (1, 1)]
                .iter()
                .all(|&(dx, dz)| biome_of(cx + dx, cz + dz) == biome);
            if out[biome].len() >= BLOCKS || !whole {
                continue;
            }
            let mut columns = vec![None; BLOCK * BLOCK];
            for (dx, dz, cy) in block_chunks(rows.clone()) {
                let chunk = generator.chunk(cx + dx, cy, cz + dz);
                if chunk.uniform().is_some() {
                    continue;
                }
                for z in 0..CHUNK_SIZE {
                    for y in 0..CHUNK_SIZE {
                        for x in 0..CHUNK_SIZE {
                            if let Some(&plant) = cover.get(&chunk.get(x, y, z).material()) {
                                let column = (dz as usize * CHUNK_SIZE + z) * BLOCK
                                    + dx as usize * CHUNK_SIZE
                                    + x;
                                columns[column] = Some(plant);
                            }
                        }
                    }
                }
            }
            out[biome].push(columns);
        }
    }
    out
}

/// Every `(dx, dz, cy)` of a block's four Chunk columns over the surface rows.
fn block_chunks(rows: std::ops::RangeInclusive<i32>) -> Vec<(i32, i32, i32)> {
    rows.flat_map(|cy| [(0, 0), (1, 0), (0, 1), (1, 1)].map(|(dx, dz)| (dx, dz, cy)))
        .collect()
}

/// What a Biome's ground cover looks like over its measured blocks.
struct Cover {
    /// Share of columns with any ground cover.
    total: f64,
    /// Share of columns by [`GROUND_COVER`] index.
    by_plant: Vec<f64>,
    /// Share of all columns with no ground cover within 2 m.
    bare: f64,
    /// Share of ground-cover columns in a patch of at least 12, cover within 2 m joined.
    clustered: f64,
}

fn measure(blocks: &[Vec<Option<u8>>]) -> Cover {
    let n = BLOCK as i32;
    let (mut columns, mut covered, mut bare, mut clustered) = (0usize, 0usize, 0usize, 0usize);
    let mut by_plant = vec![0usize; GROUND_COVER.len()];
    for block in blocks {
        let at = |x: i32, z: i32| {
            (0..n).contains(&x) && (0..n).contains(&z) && block[(z * n + x) as usize].is_some()
        };
        let near = |x: i32, z: i32| {
            (-2..=2).any(|dz| (-2..=2).any(|dx| dx * dx + dz * dz <= 4 && at(x + dx, z + dz)))
        };
        let mut seen = vec![false; BLOCK * BLOCK];
        for z in 0..n {
            for x in 0..n {
                columns += 1;
                if let Some(plant) = block[(z * n + x) as usize] {
                    covered += 1;
                    by_plant[plant as usize] += 1;
                } else {
                    bare += !near(x, z) as usize;
                }
                if !at(x, z) || seen[(z * n + x) as usize] {
                    continue;
                }
                let mut stack = vec![(x, z)];
                seen[(z * n + x) as usize] = true;
                let mut patch = 0;
                while let Some((px, pz)) = stack.pop() {
                    patch += 1;
                    for dz in -2..=2 {
                        for dx in -2..=2 {
                            let (qx, qz) = (px + dx, pz + dz);
                            if dx * dx + dz * dz <= 4 && at(qx, qz) && !seen[(qz * n + qx) as usize]
                            {
                                seen[(qz * n + qx) as usize] = true;
                                stack.push((qx, qz));
                            }
                        }
                    }
                }
                if patch >= 12 {
                    clustered += patch;
                }
            }
        }
    }
    let share = |count: usize| count as f64 / columns as f64;
    Cover {
        total: share(covered),
        by_plant: by_plant.into_iter().map(share).collect(),
        bare: share(bare),
        clustered: clustered as f64 / covered.max(1) as f64,
    }
}

/// Ground cover by Biome class, as a share of the surface: understory a dense Biome keeps, cover a
/// grassy or moderate one grows in patches, and what a sparse or arid Biome allows.
const COVER_DENSE: std::ops::RangeInclusive<f64> = 0.20..=0.55;
const COVER_GRASSY: std::ops::RangeInclusive<f64> = 0.09..=0.23;
const COVER_SPARSE: std::ops::RangeInclusive<f64> = 0.015..=0.10;
const GRASSY: [&str; 8] = [
    "temperate",
    "cold",
    "meadow",
    "garden",
    "heath",
    "highland",
    "cold_shallows",
    "prairie",
];
const ARID: [&str; 8] = [
    "dry",
    "savanna",
    "dunes",
    "salt_flats",
    "arid_court",
    "frost_court",
    "ash_flats",
    "lagoon",
];

/// A Biome's ground cover is patches with bare ground between, never an even carpet, and a flower
/// is rarer than the grass it grows among.
#[test]
fn ground_cover_grows_in_patches_at_each_biomes_class_and_flowers_stay_rare() {
    for game in &GAMES {
        let grounds = ground_of(game, 7);
        for (index, (name, _)) in game.biomes.iter().enumerate() {
            let full = format!("{}:{name}", game.name);
            let name = *name;
            let cover = measure(&grounds[index]);
            println!(
                "{full}: cover {:.1}% bare {:.1}% clustered {:.0}%",
                cover.total * 100.0,
                cover.bare * 100.0,
                cover.clustered * 100.0
            );
            let share = |list: &[&str]| -> f64 {
                list.iter()
                    .map(|p| cover.by_plant[GROUND_COVER.iter().position(|c| c == p).unwrap()])
                    .sum()
            };
            if game.bare.contains(&name) {
                assert!(
                    cover.total < 0.02,
                    "{full}: bare on purpose, {:.3}",
                    cover.total
                );
                continue;
            }
            let (range, bare_floor, clustered_floor) = if DENSE.contains(&name) {
                (COVER_DENSE, 0.08, 0.75)
            } else if GRASSY.contains(&name) {
                (COVER_GRASSY, 0.28, 0.55)
            } else {
                assert!(ARID.contains(&name), "{full}: unclassed");
                (COVER_SPARSE, 0.45, 0.0)
            };
            assert!(
                range.contains(&cover.total),
                "{full}: {:.1}% covered, wanted {range:?}",
                cover.total * 100.0
            );
            assert!(
                cover.bare >= bare_floor,
                "{full}: only {:.1}% of the ground is more than 2 m from cover",
                cover.bare * 100.0
            );
            assert!(
                cover.clustered >= clustered_floor,
                "{full}: {:.0}% of the cover is in patches",
                cover.clustered * 100.0
            );
            if !DENSE.contains(&name) {
                for flower in FLOWERS {
                    assert!(
                        share(&[flower]) <= 0.03,
                        "{full}: {flower} covers {:.1}%",
                        share(&[flower]) * 100.0
                    );
                }
                assert!(
                    share(&FLOWERS) <= 0.5 * share(&GRASSES),
                    "{full}: flowers {:.1}% against grass {:.1}%",
                    share(&FLOWERS) * 100.0,
                    share(&GRASSES) * 100.0
                );
            }
        }
    }
}
