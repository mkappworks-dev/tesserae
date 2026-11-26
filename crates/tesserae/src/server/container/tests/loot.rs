//! Chests worldgen stands hold loot drawn for their place: Terra loaded through the Mod loader, its
//! World generated, and every chest reached through Use, Edits and the Container path.

use std::collections::BTreeMap;

use tesserae_core::loot::LootTable;
use tesserae_core::template::{Cell, Turn};
use tesserae_core::{CHUNK_SIZE, Chunk, Orientation, SHAPE_FULL_CUBE, Worldgen};
use tesserae_mods::Kind;

use super::*;
use crate::server::WorldGenerator;
use crate::server::edit::StructureIntent;
use crate::server::mods::LoadedMods;
use crate::server::mods::demo_games::structures::{Generated, SEED, standing, terra_town_m};
use crate::server::mods::demo_games::{Install, loaded};
use crate::world::WorldSettings;

struct Game {
    _install: Install,
    loaded: LoadedMods,
    generator: Worldgen,
    chest: MaterialId,
}

/// `name` loaded as a World of `size` opens it.
fn game(name: &'static str, size: f32) -> Game {
    let (install, loaded) = loaded(name, size);
    game_of(size, (install, loaded))
}

fn game_of(size: f32, (install, loaded): (Install, LoadedMods)) -> Game {
    let generator = Worldgen::new(SEED, size, loaded.world.unwrap())
        .with_templates(&loaded.tables.templates)
        .with_settlements(&loaded.tables.settlements)
        .with_loot(&loaded.biome_loot);
    let chest = loaded
        .registry
        .id_of(Kind::Material, "workshop:chest")
        .unwrap() as MaterialId;
    Game {
        _install: install,
        loaded,
        generator,
        chest,
    }
}

impl Game {
    fn size(&self) -> f32 {
        self.generator.voxel_size()
    }

    /// A Server holding `chunks` in that order, the Game's Items and generator, and a Player
    /// holding 4 chests.
    fn server(&self, chunks: &[(ChunkCoord, Chunk)]) -> App {
        let mut app = app_with_tables(chunks, self.loaded.tables.clone());
        let items = self.loaded.every_item(self.size());
        let chest_item = "workshop:chest".to_owned();
        app.insert_resource(WorldSettings {
            voxel_size: self.size(),
            seed: SEED,
        })
        .insert_resource(WorldGenerator(self.generator.clone()))
        .insert_resource(PlacingItems::new(
            [(self.chest, &*chest_item, true)],
            &items,
        ))
        .insert_resource(items)
        .add_plugins((tool::plugin, inventory::plugin, plugin));
        let mut held = tesserae_core::inventory::Inventory::default();
        held.take(&chest_item, 4, Destination::Slot(Slot::Hotbar(0)), |_| 64)
            .unwrap();
        let mut players = app.world_mut().query::<&mut Inventory>();
        players.single_mut(app.world_mut()).unwrap().0 = held;
        app
    }

    /// The Chunks round `at`, three a side.
    fn chunks_round(&self, at: [i32; 3]) -> Vec<(ChunkCoord, Chunk)> {
        let home = at.map(|g| g.div_euclid(CHUNK_SIZE as i32));
        let mut chunks = Vec::new();
        for dy in -1..=1 {
            for dz in -1..=1 {
                for dx in -1..=1 {
                    let [x, y, z] = [home[0] + dx, home[1] + dy, home[2] + dz];
                    chunks.push((ChunkCoord::new(x, y, z), self.generator.chunk(x, y, z)));
                }
            }
        }
        chunks
    }

    /// Every chest of a Template standing whole within `half_m` of `centre_m`, with air above it,
    /// and the Template's name.
    fn chests(&self, centre_m: [f32; 2], half_m: f32) -> (Generated, Vec<([i32; 3], String)>) {
        self.chests_in(Generated::around(&self.generator, centre_m, half_m))
    }

    /// [`Self::chests`] of the Templates standing whole in `world`.
    fn chests_in(&self, world: Generated) -> (Generated, Vec<([i32; 3], String)>) {
        let templates = &self.loaded.tables.templates;
        let mut chests = Vec::new();
        for f in standing(&world, templates, self.size()) {
            let sampled = templates[f.template].sample(self.size(), f.turn);
            let [sx, sy, sz] = sampled.size();
            for y in 0..sy {
                for z in 0..sz {
                    for x in 0..sx {
                        let Cell::Voxel(v) = sampled.cell([x, y, z]) else {
                            continue;
                        };
                        let at = [f.corner[0] + x, f.corner[1] + y, f.corner[2] + z];
                        let above = world.get([at[0], at[1] + 1, at[2]]);
                        if v.material() == self.chest && above.is_some_and(|v| v.is_air()) {
                            chests.push((at, templates[f.template].name.clone()));
                        }
                    }
                }
            }
        }
        (world, chests)
    }

    fn items(&self, table: &LootTable) -> Vec<String> {
        table.entries.iter().map(|e| e.item.clone()).collect()
    }
}

/// Every Chunk of `world`, as a Server holds them.
fn all(world: &Generated) -> Vec<(ChunkCoord, Chunk)> {
    world
        .chunks
        .iter()
        .map(|(&[x, y, z], c)| (ChunkCoord::new(x, y, z), c.clone()))
        .collect()
}

/// The eye a Voxel's height over `at`, looking down at it.
fn over(app: &App, at: [i32; 3]) -> Vec3 {
    let size = app.world().resource::<WorldSettings>().voxel_size;
    Vec3::new(at[0] as f32 + 0.5, at[1] as f32 + 1.5, at[2] as f32 + 0.5) * size
}

/// Uses the Voxel at `at` from just above it, and what the Container there holds after.
fn open(app: &mut App, at: [i32; 3]) -> Vec<Option<Stack>> {
    let eye = over(app, at);
    set_eye(app, eye);
    app.world_mut().write_message(UseIntent {
        with: UseWith::Hand { eat: false },
        origin: eye,
        direction: Vec3::NEG_Y,
        expected: None,
    });
    app.world_mut().run_system_once(answer_use).unwrap();
    app.world_mut()
        .resource_mut::<Messages<UseIntent>>()
        .clear();
    let outcomes: Vec<_> = app
        .world_mut()
        .resource_mut::<Messages<UseOutcome>>()
        .drain()
        .collect();
    assert_eq!(outcomes, [UseOutcome(Ok(UseResult::Container(at)))]);
    settle(app);
    app.world()
        .resource::<Containers>()
        .held
        .get(&at)
        .cloned()
        .unwrap_or_default()
}

fn edit_down(app: &mut App, at: [i32; 3], action: EditAction) {
    let origin = over(app, at);
    send(
        app,
        EditIntent {
            action,
            origin,
            direction: Vec3::NEG_Y,
        },
    );
    tick(app);
    settle(app);
    assert!(rejections(app).is_empty());
}

fn remove(app: &mut App, at: [i32; 3]) {
    edit_down(app, at, EditAction::Remove);
    assert!(voxel_at(app, at).is_air());
}

/// Places a held chest on what stands under `at`.
fn place_chest(app: &mut App, at: [i32; 3]) {
    edit_down(
        app,
        at,
        EditAction::PlaceHeld {
            shape: SHAPE_FULL_CUBE,
            orientation: Orientation::IDENTITY,
        },
    );
}

/// The Stacks, by Item, with their counts added.
fn counted<'a>(stacks: impl IntoIterator<Item = (&'a str, u32)>) -> BTreeMap<String, u32> {
    let mut counts = BTreeMap::new();
    for (item, count) in stacks {
        *counts.entry(item.to_owned()).or_default() += count;
    }
    counts
}

fn held_counts(slots: &[Option<Stack>]) -> BTreeMap<String, u32> {
    counted(slots.iter().flatten().map(|s| (&*s.item, s.count)))
}

fn dropped_counts(app: &App, except: &str) -> BTreeMap<String, u32> {
    let items = app.world().resource::<Items>();
    let names: Vec<_> = app
        .world()
        .resource::<EntityStore>()
        .iter()
        .map(|e| {
            let d = e.as_dropped();
            (items.0[d.item as usize].name.clone(), d.count)
        })
        .filter(|(name, _)| name != except)
        .collect();
    counted(names.iter().map(|(n, c)| (&**n, *c)))
}

/// Inside the village a new Player enters, a point its chests lie within 128 m of.
fn village_m(terra: &Game) -> [f32; 2] {
    terra
        .generator
        .settlement_spawn_m(4096.0)
        .expect("a village near spawn")
}

/// Which of Terra's Settlements laid out the well at `well`: only a town lays cobblestone roads.
fn settlement_at(terra: &Game, world: &Generated, well: [i32; 3]) -> &'static str {
    let cobble = terra
        .loaded
        .registry
        .id_of(Kind::Material, "brickwork:cobble")
        .unwrap() as MaterialId;
    let cobbled = (-30..30)
        .flat_map(|dz| (-30..30).map(move |dx| [well[0] + dx, well[2] + dz]))
        .filter(|&at| world.top(at).is_some_and(|v| v.material() == cobble))
        .count();
    if cobbled > 100 {
        "terra:town"
    } else {
        "terra:village"
    }
}

#[test]
fn a_settlement_chest_holds_only_its_buildings_table_or_else_its_settlements() {
    let (install, loaded) = loaded("terra", 1.0);
    let town_m = terra_town_m(&loaded);
    let terra = game_of(1.0, (install, loaded));
    let mut seen = BTreeMap::<(&str, bool), usize>::new();
    for (centre_m, half_m) in [(village_m(&terra), 128.0), (town_m, 128.0)] {
        let (world, chests) =
            terra.chests_in(Generated::around_ground(&terra.generator, centre_m, half_m));
        let wells: Vec<_> = standing(&world, &terra.loaded.tables.templates, 1.0)
            .into_iter()
            .filter(|f| terra.loaded.tables.templates[f.template].name == "terra:well")
            .map(|f| f.corner)
            .collect();
        let mut app = terra.server(&all(&world));
        for (at, template) in chests {
            let well = *wells
                .iter()
                .min_by_key(|w| (w[0] - at[0]).pow(2) + (w[2] - at[2]).pow(2))
                .unwrap();
            let name = settlement_at(&terra, &world, well);
            let settlement = terra
                .loaded
                .tables
                .settlements
                .iter()
                .find(|s| s.name == name)
                .unwrap();
            let rule = settlement
                .rules
                .iter()
                .find(|r| {
                    terra.loaded.tables.templates[usize::from(r.building.template)].name == template
                })
                .unwrap_or_else(|| panic!("{template} is no building of {name}"));
            let table = rule.loot.as_ref().or(settlement.loot.as_ref()).unwrap();
            *seen.entry((name, rule.loot.is_some())).or_default() += 1;
            let slots = open(&mut app, at);
            assert!(
                slots.iter().flatten().count() >= usize::from(table.rolls_min),
                "{name} {template} at {at:?} holds its draw"
            );
            let allowed = terra.items(table);
            for stack in slots.iter().flatten() {
                assert!(
                    allowed.contains(&stack.item),
                    "{name} {template} at {at:?} holds {}",
                    stack.item
                );
            }
        }
    }
    for kind in [
        ("terra:village", false),
        ("terra:town", false),
        ("terra:town", true),
    ] {
        assert!(seen.contains_key(&kind), "{kind:?} chests opened: {seen:?}");
    }
}

/// A chest of Terra's village at 1.0 m: where it stands, and its Template's name.
fn village_chest(terra: &Game) -> ([i32; 3], String) {
    let (_, chests) = terra.chests(village_m(terra), 128.0);
    chests
        .into_iter()
        .find(|(_, t)| t == "terra:cottage")
        .expect("a cottage's chest")
}

/// The first Voxel, lowest on every axis, of the chest that fills the metre cell at `metre`, with
/// the chest Voxels of that cell above it removed so it can be aimed at.
fn first_voxel(g: &Game, app: &mut App, metre: [i32; 3]) -> [i32; 3] {
    let per = (1.0 / g.size()).round() as i32;
    let store = app.world().resource::<ChunkStore>();
    let mut cell = Vec::new();
    for y in (metre[1] - 2) * per..(metre[1] + 3) * per {
        for z in metre[2] * per..(metre[2] + 1) * per {
            for x in metre[0] * per..(metre[0] + 1) * per {
                if store
                    .voxel([x, y, z])
                    .is_some_and(|v| v.material() == g.chest)
                {
                    cell.push([x, y, z]);
                }
            }
        }
    }
    assert_eq!(
        cell.len(),
        (per * per * per) as usize,
        "one chest, sampled whole"
    );
    let first = [0, 1, 2].map(|a| cell.iter().map(|at| at[a]).min().unwrap());
    let mut above: Vec<_> = cell.into_iter().filter(|at| at[1] > first[1]).collect();
    above.sort_by_key(|at| std::cmp::Reverse(at[1]));
    for at in above {
        remove(app, at);
        assert!(
            dropped_counts(app, "workshop:chest").is_empty(),
            "only the first Voxel draws"
        );
    }
    first
}

#[test]
fn a_chest_draws_alike_opened_or_broken_first_in_any_chunk_order_and_at_every_voxel_size() {
    let terra = game("terra", 1.0);
    let (at, _) = village_chest(&terra);

    let mut opened = terra.server(&terra.chunks_round(at));
    let slots = open(&mut opened, at);
    assert!(slots.iter().any(Option::is_some));
    let later = open(&mut opened, at);
    assert_eq!(later, slots, "drawn once");

    let mut chunks = terra.chunks_round(at);
    chunks.reverse();
    let mut broken = terra.server(&chunks);
    for _ in 0..50 {
        tick(&mut broken);
        settle(&mut broken);
    }
    remove(&mut broken, at);
    assert_eq!(
        dropped_counts(&broken, "workshop:chest"),
        held_counts(&slots),
        "broken first, the same Items spill"
    );

    for size in [0.5, 0.25] {
        let finer = game("terra", size);
        let per = (1.0 / size).round() as i32;
        let mut app = finer.server(&finer.chunks_round(at.map(|g| g * per)));
        let first = first_voxel(&finer, &mut app, at);
        assert_eq!(open(&mut app, first), slots, "at {size} m");
    }
}

#[test]
fn a_chest_put_back_by_a_player_and_one_placed_anywhere_start_empty() {
    let terra = game("terra", 1.0);
    let (at, _) = village_chest(&terra);
    let mut app = terra.server(&terra.chunks_round(at));
    remove(&mut app, at);
    assert!(
        !dropped_counts(&app, "workshop:chest").is_empty(),
        "its loot spilt"
    );
    place_chest(&mut app, at);
    assert_eq!(voxel_at(&app, at).material(), terra.chest);
    assert!(open(&mut app, at).iter().all(Option::is_none), "put back");

    let on_top = [at[0], at[1] + 1, at[2]];
    place_chest(&mut app, on_top);
    assert!(open(&mut app, on_top).iter().all(Option::is_none), "placed");
}

#[test]
fn a_place_tool_structures_chests_start_empty() {
    let terra = game("terra", 1.0);
    let ground = terra.generator.surface_height_m(0.5, 0.5).floor() as i32;
    let mut app = terra.server(&terra.chunks_round([0, ground, 0]));
    let eye = Vec3::new(0.5, ground as f32 + 4.0, 0.5);
    set_eye(&mut app, eye);
    edit::send_structure(
        &mut app,
        StructureIntent {
            origin: eye,
            direction: Vec3::NEG_Y,
            template: "terra:cottage".into(),
            turn: Turn::NONE,
            pinned: None,
        },
    );
    for _ in 0..200 {
        app.world_mut()
            .run_system_once(edit::drain_structures)
            .unwrap();
    }
    assert!(rejections(&mut app).is_empty());
    let store = app.world().resource::<ChunkStore>();
    let placed: Vec<_> = (-12..12)
        .flat_map(|z| (-12..12).flat_map(move |x| (ground - 2..ground + 8).map(move |y| [x, y, z])))
        .filter(|&at| store.voxel(at).is_some_and(|v| v.material() == terra.chest))
        .filter(|&[x, y, z]| store.voxel([x, y + 1, z]).is_some_and(|v| v.is_air()))
        .collect();
    assert!(!placed.is_empty(), "the cottage stands with its chest");
    for at in placed {
        assert!(open(&mut app, at).iter().all(Option::is_none), "{at:?}");
    }
}

#[test]
fn a_drawn_chest_keeps_what_is_left_in_it_through_save_and_reopen() {
    let terra = game("terra", 1.0);
    let (at, _) = village_chest(&terra);
    let mut app = terra.server(&terra.chunks_round(at));
    let drawn = open(&mut app, at);
    let first = drawn.iter().position(Option::is_some).unwrap();
    let take = InventoryIntent::Move {
        from: SlotAddress::Container(first),
        to: mine(Slot::Hotbar(5)),
    };
    assert!(ask(&mut app, take).is_empty());
    let mut left = drawn.clone();
    left[first] = None;

    let saved = |app: &App| {
        let containers = app.world().resource::<Containers>();
        let commit = (containers.capture(), containers.looted());
        serde_json::from_str(&serde_json::to_string(&commit).unwrap()).unwrap()
    };
    let (held, looted) = saved(&app);
    let mut reopened = terra.server(&terra.chunks_round(at));
    reopened.insert_resource(Containers::restore(held, looted));
    assert_eq!(open(&mut reopened, at), left);

    ask(&mut reopened, InventoryIntent::TakeAll);
    let (held, looted) = saved(&reopened);
    let mut emptied = terra.server(&terra.chunks_round(at));
    emptied.insert_resource(Containers::restore(held, looted));
    assert!(
        open(&mut emptied, at).iter().all(Option::is_none),
        "an emptied chest never draws again"
    );
}
