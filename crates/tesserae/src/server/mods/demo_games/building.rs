//! The shared building Materials and Stations Terra's buildings hold: textured, in the Catalogue,
//! and Obtainable from what Terra's worldgen lays, through Stations that are.

use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use tesserae_core::Voxel;
use tesserae_core::craft::{Count, Station};

use super::{loaded, material};
use crate::server::craft::{self, Craft, CraftRefusal, CraftRefused};
use crate::server::inventory::Inventory;
use crate::server::mods::placeable;
use crate::server::obtainable::obtainable;
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

#[test]
fn every_building_material_and_station_is_obtainable_from_what_terras_worldgen_lays() {
    let (_install, loaded) = loaded("terra", 0.5);
    let got = obtainable(&loaded, &loaded.every_item(0.5));
    for name in BUILDING.into_iter().chain(STATIONS.map(|(name, _)| name)) {
        assert!(
            got.materials.contains(&material(&loaded, name)),
            "{name} is Obtainable"
        );
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
