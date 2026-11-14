//! Every demo Game's food: declared, lying on the surface of a new World near the spawn, and
//! placed, broken, picked up and eaten through the Server's own systems.

use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use tesserae_core::{CHUNK_SIZE, Chunk, MaterialId, Orientation, SHAPE_FULL_CUBE, Voxel, Worldgen};
use tesserae_mods::Kind;

use super::{loaded, material};
use crate::server::edit::EditAction;
use crate::server::edit::tests::{
    app_with_tables, down, held, place_held, send, set_eye, survival, tick, voxel_at,
};
use crate::server::entities::{EntityStore, Pickup, Refused, pick_up, step_entities};
use crate::server::inventory::Inventory;
use crate::server::items::PlacingItems;
use crate::server::survival::Survival;
use crate::server::tool::UseOutcome;
use crate::server::vitals::{EatIntent, Vitals, eat};
use crate::world::ChunkCoord;

struct Food {
    game: &'static str,
    item: &'static str,
    /// The Biomes it grows in, in declaration order.
    biomes: &'static [&'static str],
}

const FOODS: [Food; 5] = [
    Food {
        game: "terra",
        item: "terra:melon",
        biomes: &["terra:temperate"],
    },
    Food {
        game: "meadow",
        item: "meadow:pumpkin",
        biomes: &["meadow:meadow", "meadow:heath"],
    },
    Food {
        game: "ember",
        item: "ember:pepper",
        biomes: &["ember:ash_flats", "ember:highland"],
    },
    Food {
        game: "tidewater",
        item: "tidewater:coconut",
        biomes: &["tidewater:lagoon", "tidewater:salt_flats"],
    },
    Food {
        game: "lumen",
        item: "lumen:glowberry",
        biomes: &["lumen:garden", "lumen:frost_court"],
    },
];

#[test]
fn every_demo_game_declares_a_food_of_its_own_that_grows_on_its_biomes() {
    for food in &FOODS {
        let (_install, loaded) = loaded(food.game, 0.5);
        let item = loaded
            .items
            .iter()
            .find(|i| i.name == food.item)
            .unwrap_or_else(|| panic!("{} declares {}", food.game, food.item));
        let value = item.food.expect("a food value");
        assert!((4.0..=8.0).contains(&value), "{}: {value}", food.item);
        let id = material(&loaded, food.item);
        assert_eq!(
            item.material,
            Some(id),
            "{} places its own Material",
            food.item
        );
        assert!(loaded.tables.materials.solid(Voxel::cube(id)));
        assert!(loaded.appearance.iter().any(|(m, _)| *m == id));
        assert!(loaded.textures.iter().any(|(m, _)| *m == id));

        let world = loaded.world.unwrap();
        let growing: Vec<&str> = world
            .biomes
            .as_slice()
            .iter()
            .filter(|b| {
                b.surface_features
                    .as_slice()
                    .iter()
                    .any(|f| f.material == id && (12.0..=24.0).contains(&f.spacing_m))
            })
            .map(|b| b.name.as_str())
            .collect();
        assert_eq!(growing, food.biomes, "{}", food.item);
    }
}

/// Horizontal metres from the spawn, over the origin, to the nearest Voxel of `food`.
fn nearest_m(generator: &Worldgen, food: MaterialId, within_m: f32) -> Option<f32> {
    distances_m(generator, food, within_m)
        .into_iter()
        .reduce(f32::min)
}

/// Horizontal metres from the spawn to every Voxel of `material` within `within_m`.
fn distances_m(generator: &Worldgen, material: MaterialId, within_m: f32) -> Vec<f32> {
    let size = generator.voxel_size();
    let chunk_m = CHUNK_SIZE as f32 * size;
    let (lowest, highest) = generator.surface_bounds_m();
    let rows = (lowest / chunk_m).floor() as i32..=(highest / chunk_m).ceil() as i32;
    let reach = (within_m / chunk_m).ceil() as i32;
    let mut found = Vec::new();
    for cz in -reach..reach {
        for cx in -reach..reach {
            for cy in rows.clone() {
                let chunk = generator.chunk(cx, cy, cz);
                if chunk.uniform().is_some() {
                    continue;
                }
                for z in 0..CHUNK_SIZE {
                    for y in 0..CHUNK_SIZE {
                        for x in 0..CHUNK_SIZE {
                            if chunk.get(x, y, z).material() != material {
                                continue;
                            }
                            let at = |c: i32, n: usize| (c * CHUNK_SIZE as i32 + n as i32) as f32;
                            let d = (at(cx, x) * size).hypot(at(cz, z) * size);
                            if d <= within_m {
                                found.push(d);
                            }
                        }
                    }
                }
            }
        }
    }
    found
}

/// Half a minute's walk. A spawn in a Biome without the food, such as Terra's snow, is the far case
/// at about 80 m.
const SHORT_WALK_M: f32 = 96.0;

#[test]
fn a_new_world_of_every_demo_game_has_its_food_within_a_short_walk_of_the_spawn() {
    let seeds = [
        crate::world::WorldSettings::default().seed,
        1,
        3,
        4,
        7,
        2024,
    ];
    for food in &FOODS {
        let (_install, loaded) = loaded(food.game, 0.5);
        let world = loaded.world.unwrap();
        let id = material(&loaded, food.item);
        for seed in seeds {
            let generator = Worldgen::new(seed, 0.5, world);
            assert!(
                nearest_m(&generator, id, SHORT_WALK_M).is_some(),
                "{} seed {seed}: no {} within {SHORT_WALK_M} m",
                food.game,
                food.item
            );
        }
    }
}

/// A Workbench, then a Chest and a Door: 4 + 8 + 3.
const WOOD_FOR_A_HOME: usize = 15;

#[test]
fn a_survival_player_finds_wood_for_a_workbench_a_chest_and_a_door_near_the_spawn() {
    let (_install, loaded) = loaded("terra", 0.5);
    let world = loaded.world.unwrap();
    let wood = material(&loaded, "terra:wood");
    for seed in [
        crate::world::WorldSettings::default().seed,
        1,
        3,
        4,
        7,
        2024,
    ] {
        let generator = Worldgen::new(seed, 0.5, world);
        let found = distances_m(&generator, wood, SHORT_WALK_M).len();
        assert!(
            found >= WOOD_FOR_A_HOME,
            "seed {seed}: {found} Wood within {SHORT_WALK_M} m"
        );
    }
}

#[test]
fn terra_wood_breaks_into_one_wood_item() {
    let (_install, mut app, loaded) = kitchen("terra");
    let cell = [5, 8, 5];
    give(&mut app, "terra:wood");
    send(&mut app, place_held(SHAPE_FULL_CUBE, Orientation::IDENTITY));
    tick(&mut app);
    assert_eq!(
        voxel_at(&app, cell),
        Voxel::cube(material(&loaded, "terra:wood"))
    );

    send(&mut app, down(EditAction::Remove));
    tick(&mut app);
    assert!(voxel_at(&app, cell).is_air());
    for _ in 0..40 {
        app.world_mut().run_system_once(step_entities).unwrap();
    }
    set_eye(&mut app, Vec3::new(5.5, 9.0, 5.5));
    app.world_mut().run_system_once(pick_up).unwrap();
    assert_eq!(held(&mut app, "terra:wood"), 1);
}

/// Stone below 8 m, as the edit tests' ground, with the Game's own Items and placing rules.
fn kitchen(game: &str) -> (super::Install, App, super::LoadedMods) {
    kitchen_in(game, true)
}

fn kitchen_in(game: &str, survival_mode: bool) -> (super::Install, App, super::LoadedMods) {
    let (install, loaded) = loaded(game, 1.0);
    let rock = Voxel::cube(loaded.world.unwrap().ground.unwrap().rock);
    let mut ground = Chunk::air();
    for y in 0..8 {
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                ground.set(x, y, z, rock);
            }
        }
    }
    let mut app = app_with_tables(&[(ChunkCoord::new(0, 0, 0), ground)], loaded.tables.clone());
    if survival_mode {
        survival(&mut app);
    }
    let items = loaded.every_item(1.0);
    let solid = |id| loaded.tables.materials.solid(Voxel::cube(id));
    app.insert_resource(PlacingItems::new(
        loaded
            .registry
            .names(Kind::Material)
            .map(|(id, name)| (id as MaterialId, name, solid(id as MaterialId))),
        &items,
    ))
    .insert_resource(items)
    .insert_resource(Survival::default())
    .init_resource::<EntityStore>()
    .init_resource::<Refused>()
    .init_resource::<crate::server::vitals::Eating>()
    .init_resource::<Time>()
    .add_message::<Pickup>()
    .add_message::<EatIntent>()
    .add_message::<UseOutcome>();
    (install, app, loaded)
}

fn give(app: &mut App, item: &str) {
    let mut inventory = app.world_mut().query::<&mut Inventory>();
    let left = inventory
        .single_mut(app.world_mut())
        .unwrap()
        .receive(item, 1, 64);
    assert_eq!(left, 0);
}

fn hunger(app: &mut App) -> f32 {
    let mut vitals = app.world_mut().query::<&Vitals>();
    vitals.single(app.world()).unwrap().hunger
}

#[test]
fn every_demo_games_food_places_breaks_picks_up_and_eats() {
    let cell = [5, 8, 5];
    for food in &FOODS {
        let (_install, mut app, loaded) = kitchen(food.game);
        let voxel = Voxel::cube(material(&loaded, food.item));

        give(&mut app, food.item);
        send(&mut app, place_held(SHAPE_FULL_CUBE, Orientation::IDENTITY));
        tick(&mut app);
        assert_eq!(voxel_at(&app, cell), voxel, "{} places", food.item);
        assert_eq!(held(&mut app, food.item), 0);

        send(&mut app, down(EditAction::Remove));
        tick(&mut app);
        assert!(voxel_at(&app, cell).is_air(), "{} breaks", food.item);
        assert_eq!(app.world().resource::<EntityStore>().len(), 1);

        for _ in 0..40 {
            app.world_mut().run_system_once(step_entities).unwrap();
        }
        set_eye(&mut app, Vec3::new(5.5, 9.0, 5.5));
        app.world_mut().run_system_once(pick_up).unwrap();
        assert_eq!(held(&mut app, food.item), 1, "{} is picked up", food.item);

        let mut vitals = app.world_mut().query::<&mut Vitals>();
        vitals.single_mut(app.world_mut()).unwrap().hunger = 10.0;
        app.world_mut().write_message(EatIntent);
        // A meal takes a second, the first bite spending the Item.
        for _ in 0..90 {
            app.world_mut().run_system_once(eat).unwrap();
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_millis(16));
        }
        let value = loaded
            .items
            .iter()
            .find(|i| i.name == food.item)
            .unwrap()
            .food;
        // Three bites each refill a third, so the sum may be a rounding short.
        let eaten = hunger(&mut app) - 10.0;
        assert!(
            value.is_some_and(|value| (eaten - value).abs() < 1e-4),
            "{} is eaten: {eaten} of {value:?}",
            food.item
        );
        assert_eq!(held(&mut app, food.item), 0);
    }
}

/// Creative shares Survival's economy: the melon, like each Game's food, is spent by placing and
/// drops on the ground when broken, to be picked up.
#[test]
fn in_creative_every_demo_games_food_is_spent_by_placing_and_drops_when_broken() {
    let cell = [5, 8, 5];
    for food in &FOODS {
        let (_install, mut app, loaded) = kitchen_in(food.game, false);
        let voxel = Voxel::cube(material(&loaded, food.item));

        give(&mut app, food.item);
        send(&mut app, place_held(SHAPE_FULL_CUBE, Orientation::IDENTITY));
        tick(&mut app);
        assert_eq!(voxel_at(&app, cell), voxel, "{} places", food.item);
        assert_eq!(held(&mut app, food.item), 0, "{} is spent", food.item);

        send(&mut app, down(EditAction::Remove));
        tick(&mut app);
        assert!(voxel_at(&app, cell).is_air(), "{} breaks", food.item);
        assert_eq!(app.world().resource::<EntityStore>().len(), 1);

        for _ in 0..40 {
            app.world_mut().run_system_once(step_entities).unwrap();
        }
        set_eye(&mut app, Vec3::new(5.5, 9.0, 5.5));
        app.world_mut().run_system_once(pick_up).unwrap();
        assert_eq!(held(&mut app, food.item), 1, "{} is picked up", food.item);
    }
}

/// FNV-1a, enough to tell whether any texture byte moved.
fn fingerprint(textures: &[(String, Vec<u8>)]) -> u64 {
    textures
        .iter()
        .flat_map(|(name, pixels)| name.bytes().chain(pixels.iter().copied()))
        .fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
        })
}

/// Pins every texture the demo Games make, so code they share cannot change what they draw.
#[test]
fn the_demo_games_textures_are_unchanged() {
    let pinned = [
        ("terra", 0x4ac0_7d21_685a_633c),
        ("meadow", 0x5a2b_6c07_7a9e_ec3d),
        ("ember", 0xf36d_dd1c_3496_4938),
        ("tidewater", 0xb185_0251_6959_e86a),
        ("lumen", 0xc983_25a8_ef29_a087),
    ];
    for (game, expected) in pinned {
        let (_install, loaded) = loaded(game, 1.0);
        let mut textures: Vec<_> = loaded
            .textures
            .iter()
            .map(|(id, pixels)| {
                let name = loaded
                    .registry
                    .names(Kind::Material)
                    .find(|&(at, _)| at == usize::from(*id))
                    .map(|(_, name)| name.to_owned())
                    .unwrap();
                (name, pixels.clone())
            })
            .collect();
        textures.sort();
        assert_eq!(fingerprint(&textures), expected, "{game}");
    }
}
