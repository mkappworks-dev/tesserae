//! Every demo Game's food: declared, lying on the surface of a new World near the spawn, and
//! placed, broken, picked up and eaten through the Server's own systems.

use std::ops::ControlFlow;

use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use tesserae_core::{
    CHUNK_SIZE, Chunk, MaterialId, Orientation, SHAPE_FULL_CUBE, Tables, Voxel, Worldgen,
    WorldgenParams,
};
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

/// A new World of `loaded` at `seed`, with the Settlements a new Player is spawned in.
pub(super) fn new_world(loaded: &super::LoadedMods, seed: u64, size: f32) -> Worldgen {
    new_world_of(loaded.world.unwrap(), &loaded.tables, seed, size)
}

fn new_world_of(world: WorldgenParams, tables: &Tables, seed: u64, size: f32) -> Worldgen {
    Worldgen::new(seed, size, world)
        .with_templates(&tables.templates)
        .with_settlements(&tables.settlements)
}

/// Where a search looks: horizontal metres from `centre`, between the heights `rows_m`.
#[derive(Clone, Copy)]
pub(super) struct Around {
    pub(super) centre: [f32; 2],
    pub(super) rows_m: (f32, f32),
}

/// The spawn's horizontal place, and the rows within 48 m of its height, which a short walk keeps
/// to.
pub(super) fn around_spawn(generator: &Worldgen) -> Around {
    let eye = crate::server::spawn_eye(generator);
    Around {
        centre: [eye.x, eye.z],
        rows_m: (eye.y - 48.0, eye.y + 48.0),
    }
}

/// Horizontal metres from `centre` to every Voxel of each of `materials` within `within_m`, between
/// the heights `rows_m`, by index in `materials`.
fn distances_around_m(
    generator: &Worldgen,
    materials: &[MaterialId],
    within_m: f32,
    around: Around,
) -> Vec<Vec<f32>> {
    let mut found = vec![Vec::new(); materials.len()];
    search_around_m(generator, materials, within_m, around, |i, d| {
        found[i].push(d);
        ControlFlow::Continue(())
    });
    found
}

/// Whether a Voxel of any of `materials` stands within `within_m` of `centre`, between the heights
/// `rows_m`.
fn any_around_m(
    generator: &Worldgen,
    materials: &[MaterialId],
    within_m: f32,
    around: Around,
) -> bool {
    let mut any = false;
    search_around_m(generator, materials, within_m, around, |_, _| {
        any = true;
        ControlFlow::Break(())
    });
    any
}

/// Hands `found` each Voxel of `materials` within `within_m` of `centre`, between the heights
/// `rows_m`: its index in `materials` and its horizontal metres from `centre`, until `found`
/// breaks. Chunk columns are searched nearest first, so a search for any one stops early.
pub(super) fn search_around_m(
    generator: &Worldgen,
    materials: &[MaterialId],
    within_m: f32,
    Around {
        centre,
        rows_m: (lowest, highest),
    }: Around,
    mut found: impl FnMut(usize, f32) -> ControlFlow<()>,
) {
    let size = generator.voxel_size();
    let chunk_m = CHUNK_SIZE as f32 * size;
    let rows = (lowest / chunk_m).floor() as i32..=(highest / chunk_m).ceil() as i32;
    let reach = (within_m / chunk_m).ceil() as i32;
    let [ox, oz] = centre.map(|m| (m / chunk_m).floor() as i32);
    let mut columns: Vec<[i32; 2]> = (oz - reach..oz + reach)
        .flat_map(|cz| (ox - reach..ox + reach).map(move |cx| [cx, cz]))
        .collect();
    let gap_m = |c: i32, from: f32| {
        let low = c as f32 * chunk_m;
        (low - from).max(from - low - chunk_m).max(0.0)
    };
    columns.sort_by(|&[ax, az], &[bx, bz]| {
        let near = |cx, cz| gap_m(cx, centre[0]).hypot(gap_m(cz, centre[1]));
        near(ax, az).total_cmp(&near(bx, bz))
    });
    for [cx, cz] in columns {
        for cy in rows.clone() {
            let chunk = generator.chunk(cx, cy, cz);
            if chunk.uniform().is_some() {
                continue;
            }
            for z in 0..CHUNK_SIZE {
                for y in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        let material = chunk.get(x, y, z).material();
                        let Some(i) = materials.iter().position(|&m| m == material) else {
                            continue;
                        };
                        let at = |c: i32, n: usize| (c * CHUNK_SIZE as i32 + n as i32) as f32;
                        let d = (at(cx, x) * size - centre[0]).hypot(at(cz, z) * size - centre[1]);
                        if d <= within_m && found(i, d).is_break() {
                            return;
                        }
                    }
                }
            }
        }
    }
}

/// Half a minute's walk. A spawn in a Biome without the food, such as Terra's snow, is the far case
/// at about 80 m.
pub(super) const SHORT_WALK_M: f32 = 96.0;

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
    crate::server::in_parallel(&FOODS, |food| {
        let (_install, loaded) = loaded(food.game, 0.5);
        let (world, tables) = (loaded.world.unwrap(), &loaded.tables);
        let id = material(&loaded, food.item);
        crate::server::in_parallel(&seeds, |&seed| {
            let generator = new_world_of(world, tables, seed, 0.5);
            assert!(
                any_around_m(&generator, &[id], SHORT_WALK_M, around_spawn(&generator)),
                "{} seed {seed}: no {} within {SHORT_WALK_M} m",
                food.game,
                food.item
            );
        });
    });
}

/// A Workbench, then a Chest and a Door: 4 + 8 + 3.
const WOOD_FOR_A_HOME: u32 = 15;

/// Wood comes only from oaks: none lies about on its own, and the trunks within a short walk of
/// the spawn drop enough of the placed Wood Item for a Workbench, a Chest and a Door.
fn a_survival_player_finds_wood_for_a_workbench_a_chest_and_a_door_in_the_oaks_near_the_spawn(
    seed: u64,
) {
    let (_install, loaded) = loaded("terra", 0.5);
    let wood = material(&loaded, "flora:wood");
    let trunk = material(&loaded, "flora:oak_log");
    let yields = loaded.tables.plants.yields(trunk);
    assert!(
        matches!(yields, [y] if y.item == "flora:log" && y.count.min >= 1),
        "{yields:?}"
    );
    let per_trunk = yields[0].count.min;
    let generator = new_world(&loaded, seed, 0.5);
    let [wood, trunks] = <[_; 2]>::try_from(distances_around_m(
        &generator,
        &[wood, trunk],
        SHORT_WALK_M,
        around_spawn(&generator),
    ))
    .unwrap();
    assert!(wood.is_empty(), "seed {seed}: Wood lies about on its own");
    let found = trunks.len() as u32 * per_trunk;
    assert!(
        found >= WOOD_FOR_A_HOME,
        "seed {seed}: {found} Wood within {SHORT_WALK_M} m"
    );
}

each_case!(a_survival_player_finds_wood_for_a_workbench_a_chest_and_a_door_in_the_oaks_near_the_spawn:
    default_seed = crate::world::WorldSettings::default().seed,
    seed_1 = 1,
    seed_3 = 3,
    seed_4 = 4,
    seed_7 = 7,
    seed_2024 = 2024,
);

/// What a grown oak gives is the placed Wood Item, and nothing grown can be placed.
#[test]
fn a_grown_oak_drops_placed_items_and_its_grown_materials_have_none() {
    let (_install, loaded) = loaded("terra", 0.5);
    let items = loaded.every_item(0.5);
    for name in [
        "oak_log",
        "oak_leaves",
        "apple_log",
        "apple_leaves",
        "apple_ripe",
    ] {
        let id = material(&loaded, &format!("flora:{name}"));
        assert!(loaded.tables.plants.is_grown(id), "{name}");
        assert!(
            !loaded.items.iter().any(|i| i.material == Some(id)),
            "{name} has an Item"
        );
    }
    for item in ["flora:log", "flora:leaves", "flora:oak_sapling"] {
        assert!(items.named(item).is_some(), "{item}");
    }
    let sapling = loaded
        .tables
        .plants
        .plants()
        .iter()
        .find(|p| p.name == "flora:oak")
        .expect("flora declares an oak");
    let tesserae_core::plant::Kind::Tree(tree) = &sapling.kind else {
        panic!("the oak is a tree");
    };
    assert!(tree.sapling_yield.min >= 1);
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
        ("terra", 0x6fe5049b5af5e13d),
        ("meadow", 0xa7da360e5c42e803),
        ("ember", 0x324f6f9f16d933f5),
        ("tidewater", 0x05a5e08b5a1e9e58),
        ("lumen", 0x9de6015676a2512f),
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

/// A new Player enters a village, so its food, in a patch by it, and the oaks for a first
/// Workbench, Chest and Door are a short walk from there.
#[test]
fn a_new_world_has_food_and_wood_within_a_short_walk_of_the_spawn() {
    crate::server::in_parallel(&FOODS, |food| {
        let (_install, loaded) = loaded(food.game, 1.0);
        let id = material(&loaded, food.item);
        let trunk = material(&loaded, "flora:oak_log");
        let per_trunk = loaded.tables.plants.yields(trunk)[0].count.min;
        for seed in [crate::world::WorldSettings::default().seed, 1, 3, 7] {
            let generator = new_world(&loaded, seed, 1.0);
            let around = around_spawn(&generator);
            let needed = if food.game == "terra" {
                WOOD_FOR_A_HOME
            } else {
                0
            };
            let (mut fed, mut wood) = (false, 0);
            search_around_m(&generator, &[id, trunk], SHORT_WALK_M, around, |i, _| {
                if i == 0 {
                    fed = true;
                } else {
                    wood += per_trunk;
                }
                if fed && wood >= needed {
                    ControlFlow::Break(())
                } else {
                    ControlFlow::Continue(())
                }
            });
            assert!(
                fed,
                "{} seed {seed}: no {} within {SHORT_WALK_M} m of the spawn",
                food.game, food.item
            );
            assert!(
                wood >= needed,
                "seed {seed}: {wood} Wood within {SHORT_WALK_M} m of the spawn"
            );
        }
    });
}
