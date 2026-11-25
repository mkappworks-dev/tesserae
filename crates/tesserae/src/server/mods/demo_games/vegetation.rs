//! What Terra's Biomes grow: a sample of generated Chunks counted by Plant and Material name, so a
//! Biome that loses a species, goes bare by accident or sprouts standalone Wood fails.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use tesserae_core::plant::Kind as PlantKind;
use tesserae_core::{CHUNK_SIZE, SurfacePlantKind, Worldgen};
use tesserae_mods::Kind;

use super::{loaded, material, shape};

const TREES: [&str; 10] = [
    "oak", "birch", "pine", "palm", "acacia", "jungle", "swamp", "apple", "orange", "cherry",
];
/// The Biomes of a Game whose trees hang vines.
const VINED: [&str; 2] = ["flora:jungle", "flora:swamp"];

struct Game {
    name: &'static str,
    /// Each Biome's declared species, in declaration order of the Biome table.
    biomes: &'static [(&'static str, &'static [&'static str])],
    /// Biomes declared bare on purpose.
    bare: &'static [&'static str],
}

const TERRA: Game = Game {
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
        // The Biomes of the land's shape, which only version 3 chooses: the sea's plants, and
        // bare sand and rock.
        (
            "ocean",
            &["aquatic:kelp", "aquatic:seagrass", "aquatic:red_weed"],
        ),
        ("beach", &[]),
        ("alpine", &[]),
        ("river", &["flora:cane"]),
    ],
    bare: &["beach", "alpine"],
};

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
/// Chunk columns sampled per Biome where water covers its ground, besides [`SAMPLES`]: a
/// Biome holds water only in its few lakes, which the first [`SAMPLES`] may all miss.
const WET_SAMPLES: usize = 8;
/// The census's name for a tree trunk's lowest Voxel: a log on the ground under [`TREE_CLEAR_M`] of
/// logs, which a neighbour's limb resting on a slope never is.
const TRUNK_FOOT: &str = "trunk foot";

/// The highest row of each Chunk column a Plant can reach: none stands higher than the ground of
/// its own Chunk column or a neighbour's, past which no crown reaches, raised by the overhangs and
/// the tallest Plant. The rows above are sky, the costliest Chunks generation makes.
struct PlantCeiling<'a> {
    generator: &'a Worldgen,
    lift_m: f32,
    highest_m: HashMap<[i32; 2], f32>,
}

impl<'a> PlantCeiling<'a> {
    fn new(generator: &'a Worldgen) -> Self {
        // Each Plant's rounding to Voxels, with room over the generator's own three Voxels.
        let slack_m = 8.0 * generator.voxel_size();
        let params = generator.params();
        let plants = || {
            params
                .biomes
                .as_slice()
                .iter()
                .flat_map(|b| b.surface_features.as_slice())
                .filter_map(|f| f.plant)
        };
        let reach_m = plants()
            .filter_map(|p| match p.kind {
                SurfacePlantKind::Tree(t) => Some(t.reach_m()),
                SurfacePlantKind::Vine(v) => Some(v.reach_m),
                _ => None,
            })
            .fold(0.0, f32::max)
            + slack_m;
        assert!(
            reach_m <= CHUNK_SIZE as f32 * generator.voxel_size(),
            "a crown reaches {reach_m} m, past the next Chunk column"
        );
        let rise_m = plants().map(|p| p.rise_m()).fold(0.0, f32::max);
        Self {
            generator,
            lift_m: params.landforms.overhang_m + rise_m + slack_m,
            highest_m: HashMap::new(),
        }
    }

    fn row(&mut self, [cx, cz]: [i32; 2]) -> i32 {
        let generator = self.generator;
        let size = generator.voxel_size();
        let mut top_m = f32::MIN;
        for dz in -1..=1 {
            for dx in -1..=1 {
                let [cx, cz] = [cx + dx, cz + dz];
                let highest_m = self.highest_m.entry([cx, cz]).or_insert_with(|| {
                    let at = |c: i32, n: usize| (c * CHUNK_SIZE as i32 + n as i32) as f32 * size;
                    (0..CHUNK_SIZE * CHUNK_SIZE)
                        .map(|i| {
                            generator
                                .surface_height_m(at(cx, i % CHUNK_SIZE), at(cz, i / CHUNK_SIZE))
                        })
                        .fold(f32::MIN, f32::max)
                });
                top_m = top_m.max(*highest_m);
            }
        }
        ((top_m + self.lift_m) / (CHUNK_SIZE as f32 * size)).floor() as i32
    }
}

/// `(Biome index, Material name)` to Voxels found.
type Counts = BTreeMap<(usize, String), u64>;

/// What stands on the surface of a sample of every Biome's ground: a cave under a Biome holds its
/// cave Biome's Plants, not the Biome's.
fn census(game: &Game) -> (Counts, Vec<String>) {
    surface_census(game, |loaded| loaded)
}

/// [`census`] of the Game as `edit` leaves it.
fn surface_census(
    game: &Game,
    edit: fn(super::LoadedMods) -> super::LoadedMods,
) -> (Counts, Vec<String>) {
    let (_install, loaded) = loaded(game.name, 1.0);
    let loaded = edit(loaded);
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
            tracked && !name.contains("_leaves")
        })
        .map(|(id, name)| (id as u16, name.to_owned()))
        .collect();

    let logs: BTreeSet<u16> = named
        .iter()
        .filter(|(_, name)| name.ends_with("_log"))
        .map(|(&id, _)| id)
        .collect();
    let plant_made: BTreeSet<u16> = loaded
        .registry
        .names(Kind::Material)
        .filter(|(_, name)| {
            ["flora:", "crops:", "aquatic:"]
                .iter()
                .any(|owner| name.starts_with(owner))
        })
        .map(|(id, _)| id as u16)
        .collect();

    let generator = Worldgen::new(7, 1.0, world);
    let mut ceiling = PlantCeiling::new(&generator);
    let water = generator.water().map(|w| w.material);
    let chunk_m = CHUNK_SIZE as f32;
    let (low, high) = generator.surface_bounds_m();
    let rows = (low / chunk_m).floor() as i32..=(high / chunk_m).ceil() as i32;
    let centre = |cx: i32, cz: i32| ((cx as f32 + 0.5) * chunk_m, (cz as f32 + 0.5) * chunk_m);
    let biome_of = |(x, z): (f32, f32)| generator.biome_at(x, z).unwrap().index();
    let mut taken = vec![0usize; biomes.len()];
    let mut columns = Vec::new();
    for cz in (-120..120).step_by(3) {
        for cx in (-120..120).step_by(3) {
            let biome = biome_of(centre(cx, cz));
            if taken[biome] < SAMPLES {
                taken[biome] += 1;
                columns.push((cx, cz));
            }
        }
    }
    let mut wet = vec![0usize; biomes.len()];
    for cz in -120..120 {
        for cx in -120..120 {
            let (x, z) = centre(cx, cz);
            let under_water = generator
                .water_surface_m(x, z)
                .is_some_and(|w| w > generator.surface_height_m(x, z));
            let biome = biome_of((x, z));
            if under_water && wet[biome] < WET_SAMPLES && !columns.contains(&(cx, cz)) {
                wet[biome] += 1;
                columns.push((cx, cz));
            }
        }
    }
    let at = |c: i32, n: usize| (c * CHUNK_SIZE as i32 + n as i32) as f32;
    let mut counts = Counts::new();
    let rise = (TREE_CLEAR_M as usize).min(CHUNK_SIZE);
    for (cx, cz) in columns {
        let ground_m = |x: usize, z: usize| generator.surface_height_m(at(cx, x), at(cz, z));
        let surface: Vec<f32> = (0..CHUNK_SIZE * CHUNK_SIZE)
            .map(|i| ground_m(i % CHUNK_SIZE, i / CHUNK_SIZE))
            .collect();
        let lowest = surface.iter().copied().fold(f32::MAX, f32::min);
        for cy in *rows.start()..=ceiling.row([cx, cz]).min(*rows.end()) {
            if at(cy, CHUNK_SIZE - 1) < lowest - 1.0 {
                continue;
            }
            let chunk = generator.chunk(cx, cy, cz);
            if chunk.uniform().is_some() {
                continue;
            }
            let mut above = None;
            for z in 0..CHUNK_SIZE {
                for y in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        let material = chunk.get(x, y, z).material();
                        let Some(name) = named.get(&material) else {
                            continue;
                        };
                        // A Plant stands on the column's top Voxel, so its lowest Voxel is the one at
                        // the surface.
                        if at(cy, y) < surface[z * CHUNK_SIZE + x] - 1.0 {
                            continue;
                        }
                        let here = biome_of((at(cx, x), at(cz, z)));
                        *counts.entry((here, name.clone())).or_default() += 1;
                        let below = (y > 0).then(|| chunk.get(x, y - 1, z));
                        let on_ground = below.is_some_and(|b| {
                            !b.is_air()
                                && !plant_made.contains(&b.material())
                                && Some(b.material()) != water
                        });
                        let mut trunk = || {
                            (y..y + rise).all(|up| {
                                let voxel = if up < CHUNK_SIZE {
                                    chunk.get(x, up, z)
                                } else {
                                    let next = above
                                        .get_or_insert_with(|| generator.chunk(cx, cy + 1, cz));
                                    next.get(x, up - CHUNK_SIZE, z)
                                };
                                logs.contains(&voxel.material())
                            })
                        };
                        if logs.contains(&material) && on_ground && trunk() {
                            *counts.entry((here, TRUNK_FOOT.to_owned())).or_default() += 1;
                        }
                    }
                }
            }
        }
    }
    let selectors = generator.params().biomes;
    assert!(
        taken
            .iter()
            .enumerate()
            // A Biome of the land's shape may find no column it admits near here.
            .all(|(i, &n)| n > 0 || selectors.selectors()[i].is_ranged()),
        "{}: a Biome has no ground within 3.8 km: {taken:?}",
        game.name
    );
    (counts, biomes)
}

fn found(counts: &Counts, biome: usize, name: &str) -> u64 {
    counts.get(&(biome, name.to_owned())).copied().unwrap_or(0)
}

/// The names of every tree's fruit Materials, in every Stage.
fn fruit_of(loaded: &super::LoadedMods) -> BTreeSet<String> {
    loaded
        .registry
        .names(Kind::Material)
        .filter(|&(id, _)| loaded.tables.plants.canopy_of_fruit(id as u16).is_some())
        .map(|(_, name)| name.to_owned())
        .collect()
}

/// What Biome `index` grows that a bare Biome must not. A neighbour's crowns reach over the border
/// with their limbs, vines and fruit, however long the border; a tree standing here has a trunk
/// foot.
fn strays<'a>(counts: &'a Counts, index: usize, fruit: &BTreeSet<String>) -> Vec<(&'a str, u64)> {
    counts
        .iter()
        .filter(|((b, name), _)| {
            let overhang = name == "flora:vine" || name.ends_with("_log") || fruit.contains(name);
            *b == index && !overhang
        })
        .map(|((_, name), n)| (name.as_str(), *n))
        .collect()
}

/// Terra with its beach growing the temperate Biome's ground cover and trees.
fn beach_grown_by_accident(mut loaded: super::LoadedMods) -> super::LoadedMods {
    let world = loaded.world.as_mut().unwrap();
    let mut biomes = world.biomes.as_slice().to_vec();
    let temperate = super::biome(world, "terra:temperate").surface_features;
    let beach = biomes.iter_mut().find(|b| b.name.as_str() == "terra:beach");
    beach.unwrap().surface_features = temperate;
    world.biomes = tesserae_core::BiomeTable::new(&biomes)
        .and_then(|t| t.with_selectors(world.biomes.selectors()))
        .and_then(|t| t.with_strata(world.biomes.strata()))
        .unwrap();
    loaded
}

#[test]
fn a_bare_biome_that_grows_on_its_own_surface_is_caught() {
    let terra = &TERRA;
    let (counts, biomes) = surface_census(terra, beach_grown_by_accident);
    let beach = biomes.iter().position(|b| b == "terra:beach").unwrap();
    let (_install, loaded) = loaded(terra.name, 1.0);
    let plants = strays(&counts, beach, &fruit_of(&loaded));
    assert!(
        plants.iter().any(|&(name, _)| name == "flora:tuft")
            && plants.iter().any(|&(name, _)| name == TRUNK_FOOT),
        "{plants:?}"
    );
}

/// Also the vines of jungle and swamp trees, and Wood only in trees: the census is nearly all of
/// each one's time, so the three share it.
#[test]
fn every_terra_biome_grows_its_declared_species_and_none_is_bare_by_accident() {
    let game = &TERRA;
    let (counts, biomes) = census(game);
    let (_install, loaded) = loaded(game.name, 1.0);
    let fruit = fruit_of(&loaded);
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
        // A Biome of the land's shape the census may never choose.
        let chosen = counts.keys().any(|(b, _)| *b == index);
        for plant in *species {
            if !chosen {
                continue;
            }
            let total: u64 = materials_of(plant)
                .iter()
                .map(|material| found(&counts, index, material))
                .sum();
            assert!(total > 0, "{full} grows no {plant}");
        }
        if species.is_empty() {
            let plants = strays(&counts, index, &fruit);
            assert!(plants.is_empty(), "{full} is bare on purpose: {plants:?}");
        }
    }
    jungle_and_swamp_trees_hang_vines(game, &counts);
    generated_wood_is_only_ever_the_trunk_of_a_grown_tree(game, &counts);
}

fn jungle_and_swamp_trees_hang_vines(game: &Game, counts: &Counts) {
    for (index, (name, species)) in game.biomes.iter().enumerate() {
        if species.iter().any(|p| VINED.contains(p)) {
            assert!(
                found(counts, index, "flora:vine") > 0,
                "{}:{name} has no vines",
                game.name
            );
        }
    }
}

fn generated_wood_is_only_ever_the_trunk_of_a_grown_tree(game: &Game, counts: &Counts) {
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
            // Flora declares `flora:grass`, but it is ground.
            *name != "flora:grass"
                && ["flora:", "crops:", "aquatic:"]
                    .iter()
                    .any(|owner| name.starts_with(owner))
        })
        .map(|(id, _)| id as u16)
        .collect();
    let generator = Worldgen::new(7, 1.0, world);
    let mut ceiling = PlantCeiling::new(&generator);
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
            // A tree stands on the ground, so no trunk foot is in a row wholly below it.
            let lowest = (0..CHUNK_SIZE * CHUNK_SIZE)
                .map(|i| generator.surface_height_m(at(cx, i % CHUNK_SIZE), at(cz, i / CHUNK_SIZE)))
                .fold(f32::MAX, f32::min);
            for cy in *rows.start()..=ceiling.row([cx, cz]).min(*rows.end()) {
                if at(cy, CHUNK_SIZE - 1) < lowest - 1.0 {
                    continue;
                }
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

/// The approved mockup's density classes, by Terra's Biome name: a jungle or swamp packs trees
/// 4 to 6 m apart (the colour spec widened them from 4 m so a forest reads as separate trees;
/// the swamp's clumping measures its 5 m at 6.4), a temperate or cold forest 6 to 10 m, a savanna
/// or desert 12 m or more.
const DENSE: [&str; 2] = ["jungle", "swamp"];
const MODERATE: [&str; 2] = ["temperate", "cold"];
const SPARSE: [&str; 4] = [
    "savanna", "dry", // The sea grows weed, not trees, and a river's banks cane.
    "ocean", "river",
];

/// Also that jungle and swamp are denser than savanna and desert: counting the trees is nearly all
/// of either one's time, so the two share it.
#[test]
fn each_terra_biome_scatters_its_trees_at_its_density_class() {
    let game = &TERRA;
    let density = trees_per_hectare(game);
    for (name, per_ha) in density.iter().cloned() {
        let spacing = mean_spacing_m(per_ha);
        let full = format!("{}:{name}", game.name);
        if DENSE.contains(&name.as_str()) {
            assert!(
                (2.5..=6.5).contains(&spacing),
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
            assert!(game.bare.contains(&name.as_str()), "{full}: unclassed");
        }
    }
    assert!(
        jungle_and_swamp_are_denser_than_savanna_and_desert(game, &density),
        "{}: no dense and sparse Biomes to compare",
        game.name
    );
}

/// Whether `game` has both classes to compare.
fn jungle_and_swamp_are_denser_than_savanna_and_desert(
    game: &Game,
    density: &[(String, f64)],
) -> bool {
    let of = |class: &[&str]| -> Vec<(String, f64)> {
        density
            .iter()
            .filter(|(n, _)| class.contains(&n.as_str()))
            .cloned()
            .collect()
    };
    let (dense, sparse) = (of(&DENSE), of(&SPARSE));
    if dense.is_empty() || sparse.is_empty() {
        return false;
    }
    let thinnest = dense.iter().map(|d| d.1).fold(f64::MAX, f64::min);
    let thickest = sparse.iter().map(|d| d.1).fold(0.0, f64::max);
    assert!(
        thinnest > 2.0 * thickest,
        "{}: dense {dense:?} is not clearly past sparse {sparse:?}",
        game.name
    );
    true
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
        for z in (-4000..4000).step_by(16) {
            for x in (-4000..4000).step_by(16) {
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
    let mut ceiling = PlantCeiling::new(&generator);
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
                if cy > ceiling.row([cx + dx, cz + dz]) {
                    continue;
                }
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
const GRASSY: [&str; 2] = ["temperate", "cold"];
const ARID: [&str; 2] = ["dry", "savanna"];

/// A Biome's ground cover is patches with bare ground between, never an even carpet, and a flower
/// is rarer than the grass it grows among.
#[test]
fn ground_cover_grows_in_patches_at_each_terra_biomes_class_and_flowers_stay_rare() {
    let game = &TERRA;
    let grounds = ground_of(game, 7);
    for (index, (name, _)) in game.biomes.iter().enumerate() {
        let full = format!("{}:{name}", game.name);
        let name = *name;
        if grounds[index].is_empty() {
            // A Biome of the land's shape, which only version 3 chooses.
            continue;
        }
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
        // The sea's floor and a river's bed lie under water, where no ground cover grows.
        if game.bare.contains(&name) || ["ocean", "river"].contains(&name) {
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

/// Headroom a Player walks under: nothing of a tree but its trunk hangs lower.
const TREE_CLEAR_M: f32 = 4.0;

/// An understory, the lowest storey of a row with more than one, stands below the canopy by
/// design, so its crown is exempt.
#[test]
fn under_every_tree_above_the_understory_nothing_but_its_trunk_is_lower_than_four_metres() {
    use tesserae_core::plant::tree::{band_of, tree_voxels, vine_voxels};
    for size in [1.0, 0.5, 0.25] {
        let (_install, loaded) = loaded("terra", size);
        let plants = &loaded.tables.plants;
        let mut low = Vec::new();
        for plant in plants.plants() {
            let PlantKind::Tree(row) = &plant.kind else {
                continue;
            };
            let vines = row
                .vines
                .as_ref()
                .map(|v| plants.vines(&loaded.tables.shapes, v).unwrap());
            let storeys = row.storeys.as_slice();
            let understory = (storeys.len() > 1).then(|| {
                storeys
                    .iter()
                    .map(|s| s.trunk_height_m.1)
                    .fold(f32::MAX, f32::min)
            });
            let mut lowest_m = f32::MAX;
            for i in 0..200 {
                let base = [i % 20 * 37, 0, i / 20 * 41];
                let grown = tree_voxels(row, 7, base, size);
                let band = band_of(row, 7, base, size);
                // The trunk stands in its own column, leaning by at most `lean-m` as it rises.
                let lean = band.branching.as_ref().map_or(0.0, |b| b.lean_m);
                let trunk = |at: [i32; 3]| {
                    let off = ((at[0] - base[0]) as f32).hypot((at[2] - base[2]) as f32) * size;
                    off <= lean * (at[1] as f32 * size) / band.trunk_height_m.0 + size
                };
                let hung: Vec<[i32; 3]> = vines
                    .as_ref()
                    .map(|v| {
                        vine_voxels(&grown, v, 7, size)
                            .iter()
                            .flat_map(|v| v.cells())
                            .collect()
                    })
                    .unwrap_or_default();
                let wood_or_leaf = grown.iter().filter(|v| {
                    !(v.part == tesserae_core::plant::tree::Part::Trunk && trunk(v.at))
                });
                let low_m = wood_or_leaf
                    .map(|v| v.at)
                    .chain(hung)
                    .map(|at| at[1] as f32 * size)
                    .fold(f32::MAX, f32::min);
                if understory != Some(band.trunk_height_m.1) {
                    lowest_m = lowest_m.min(low_m);
                }
                let top_m = grown.iter().map(|v| v.at[1] + 1).max().unwrap() as f32 * size;
                assert!(
                    top_m <= tesserae_core::plant::MAX_PLANT_HEIGHT_M,
                    "{} at {size} m: {top_m} m tall",
                    plant.name
                );
            }
            low.push((plant.name.clone(), lowest_m));
        }
        for (name, lowest_m) in low {
            assert!(
                lowest_m >= TREE_CLEAR_M,
                "{name} at {size} m: leaves, limbs or vines at {lowest_m} m"
            );
        }
    }
}
