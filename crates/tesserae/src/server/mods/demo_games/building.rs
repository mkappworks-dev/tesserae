//! The shared building Materials and Stations Terra's buildings hold: textured, in the Catalogue,
//! and Obtainable from what Terra's worldgen lays, through Stations that are.

use std::collections::HashSet;

use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use tesserae_core::craft::{Count, Station};
use tesserae_core::{GroundParams, MaterialId, SurfacePlantKind, Voxel};

use super::{loaded, material};
use crate::server::craft::{self, Craft, CraftRefusal, CraftRefused};
use crate::server::inventory::Inventory;
use crate::server::mods::{LoadedMods, placeable};
use crate::server::player::Player;
use crate::server::station::AtStation;

const STATIONS: [(&str, &str); 3] = [
    ("workshop:anvil", "anvil"),
    ("workshop:forge", "forge"),
    ("workshop:oven", "oven"),
];

/// The Materials Terra's Templates are built of, from the shared building Mods.
const BUILDING: [&str; 22] = [
    "timber:planks",
    "timber:pine_planks",
    "timber:dark_planks",
    "timber:red_planks",
    "timber:frame",
    "timber:dark_frame",
    "timber:shingles",
    "timber:thatch",
    "timber:awning",
    "timber:door",
    "brickwork:cobble",
    "brickwork:ashlar",
    "brickwork:slate",
    "brickwork:brick",
    "brickwork:tile",
    "brickwork:red_tile",
    "brickwork:whitewash",
    "glassworks:glass",
    "glassworks:lantern",
    "workshop:workbench",
    "workshop:chest",
    "workshop:oven",
];

#[test]
fn terras_building_materials_are_textured_and_in_the_catalogue() {
    let (_install, loaded) = loaded("terra", 0.5);
    for name in BUILDING {
        let id = material(&loaded, name);
        assert!(
            loaded.textures.iter().any(|t| t.material == id),
            "{name} has a texture"
        );
        assert!(placeable(&loaded.tables, id), "{name} is in the Catalogue");
    }
}

/// The Materials a World of `loaded` lays: its ground, every Biome's ground, ores and surface
/// features, and every Material of the Plants those features stand.
fn laid(loaded: &LoadedMods) -> HashSet<MaterialId> {
    let world = loaded.world.as_ref().expect("a Game sets its World");
    let mut laid = HashSet::new();
    let mut ground = |g: &GroundParams| laid.extend([g.rock, g.soil, g.surface]);
    world.ground.iter().for_each(&mut ground);
    for biome in world.biomes.as_slice() {
        biome.ground.iter().for_each(&mut ground);
    }
    laid.extend(world.ores.as_slice().iter().map(|o| o.material));
    for biome in world.biomes.as_slice() {
        for feature in biome.surface_features.as_slice() {
            laid.insert(feature.material);
            let Some(plant) = feature.plant else { continue };
            match plant.kind {
                SurfacePlantKind::Stages { materials, len, .. } => {
                    laid.extend(&materials[..usize::from(len)])
                }
                SurfacePlantKind::Column { material, .. } => {
                    laid.insert(material);
                }
                SurfacePlantKind::Tree(tree) => {
                    laid.insert(tree.trunk);
                    laid.extend(tree.canopies(world.biomes.variety()).as_slice());
                }
                SurfacePlantKind::Vine(vine) => {
                    laid.insert(vine.material);
                    laid.extend(&vine.fruit[..usize::from(vine.len)]);
                }
            }
        }
    }
    laid
}

/// The Items a Player can come by in a World of `loaded`: what breaking a laid Voxel yields or
/// drops, then every Recipe output whose inputs are already in hand at a Station whose Item is,
/// until nothing changes.
fn obtainable_items(loaded: &LoadedMods) -> HashSet<String> {
    let items = loaded.every_item(0.5);
    let mut held: HashSet<String> = HashSet::new();
    for material in laid(loaded) {
        let yields = loaded.tables.plants.yields(material);
        if yields.is_empty() {
            held.extend(
                items
                    .dropped_by(material)
                    .map(|at| items.0[at].name.clone()),
            );
        } else {
            held.extend(yields.iter().map(|y| y.item.clone()));
        }
    }
    loop {
        let before = held.len();
        let stations: HashSet<&str> = held
            .iter()
            .filter_map(|name| items.named(name))
            .filter_map(|at| items.0[at].material)
            .filter_map(|m| loaded.tables.uses.station(m))
            .collect();
        let at_station = |station: &Station| match station {
            Station::Hand => true,
            Station::Tag(tag) => stations.contains(tag.as_str()),
        };
        let made: Vec<String> = loaded
            .recipes
            .iter()
            .filter(|r| at_station(&r.station) && r.inputs.iter().all(|i| held.contains(&i.item)))
            .map(|r| r.output.item.clone())
            .collect();
        held.extend(made);
        if held.len() == before {
            return held;
        }
    }
}

#[test]
fn every_building_material_and_station_is_obtainable_from_what_terras_worldgen_lays() {
    let (_install, loaded) = loaded("terra", 0.5);
    let held = obtainable_items(&loaded);
    for name in BUILDING.into_iter().chain(STATIONS.map(|(name, _)| name)) {
        assert!(held.contains(name), "{name} is Obtainable; held {held:?}");
    }
}

/// As the Workbench is: a Recipe that needs the Station is refused away from it and crafts beside
/// it, for every Recipe each Station holds.
#[test]
fn the_anvil_forge_and_oven_are_stations_whose_recipes_craft_only_beside_them() {
    let game = "terra";
    let (_install, loaded) = loaded(game, 0.5);
    let stations = STATIONS.map(|(name, tag)| {
        let id = material(&loaded, name);
        assert_eq!(loaded.tables.uses.station(id), Some(tag), "{name}");
        id
    });
    let mut app = App::new();
    app.insert_non_send(loaded)
        .add_plugins((super::super::plugin, craft::plugin));
    let book = app
        .world_mut()
        .resource_mut::<Messages<crate::content::RecipeBook>>()
        .drain()
        .next()
        .unwrap()
        .0;
    for ((_, name), id) in STATIONS.into_iter().zip(stations) {
        let here: Vec<usize> = (0..book.len())
            .filter(|&at| book[at].station == Station::Tag(name.into()))
            .collect();
        assert!(!here.is_empty(), "{game}: the {name} holds a Recipe");
        for at in here {
            let mut inventory = Inventory::default();
            for input in &book[at].inputs {
                inventory.receive(&input.item, input.count, 64);
            }
            let player = app
                .world_mut()
                .spawn((Player::at(Vec3::ZERO), inventory))
                .id();
            let output = &book[at].output.item;
            assert_eq!(
                ask(&mut app, at),
                [CraftRefusal::NeedsStation],
                "{game}: {output} away from the {name}"
            );
            app.world_mut().entity_mut(player).insert(AtStation {
                at: [0; 3],
                voxel: Voxel::cube(id),
                tag: name.into(),
            });
            assert_eq!(ask(&mut app, at), [], "{game}: {output} at the {name}");
            let held = app.world().get::<Inventory>(player).unwrap();
            assert_eq!(
                held.count(output),
                book[at].output.count,
                "{game}: {output}"
            );
            app.world_mut().despawn(player);
        }
    }
}

fn ask(app: &mut App, recipe: usize) -> Vec<CraftRefusal> {
    app.world_mut().write_message(Craft {
        recipe,
        count: Count::One,
    });
    app.world_mut().run_system_once(craft::craft).unwrap();
    app.world_mut().resource_mut::<Messages<Craft>>().clear();
    app.world_mut()
        .resource_mut::<Messages<CraftRefused>>()
        .drain()
        .map(|m| m.0)
        .collect()
}
