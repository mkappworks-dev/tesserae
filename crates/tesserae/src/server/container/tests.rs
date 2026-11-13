use bevy::ecs::system::RunSystemOnce;
use tesserae_core::inventory::{Destination, Refusal, Slot};
use tesserae_core::{MaterialDef, Tables, VoxelUse};
use tesserae_mods::Intent;

use super::super::edit::tests::{
    app_with_tables, ground, place_held, rejections, replay, send, set_eye, tick, voxel_at,
};
use super::super::edit::{self, EditAction, EditIntent, RejectReason};
use super::super::inventory::{
    self, Inventory, InventoryChanged, InventoryIntent, InventoryRefused, Reason, SlotAddress,
};
use super::super::items::{Items, PlacingItems};
use super::super::streaming::MeshState;
use super::super::tool::{self, UseIntent, UseOutcome, UseResult, answer_use};
use super::super::undo::HistoryAction::{Redo, Undo};
use super::*;
use crate::world::ChunkCoord;

const EYE: Vec3 = Vec3::new(5.5, 12.0, 5.5);
const CHEST: [i32; 3] = [5, 8, 5];

/// The ground with a 4-slot Chest at (5, 8, 5), and a Player looking down at
/// it holding 10 rock, 5 dust and 3 chests.
fn app() -> (App, MaterialId) {
    let mut tables = crate::server::fixture_tables();
    let chest = chest_material(&mut tables);
    let mut chunk = ground();
    chunk.set(5, 8, 5, Voxel::cube(chest));
    let mut app = app_with_tables(&[(ChunkCoord::new(0, 0, 0), chunk)], tables);
    let item = |name: &str, material| tesserae_mods::ItemDef {
        name: name.into(),
        size_m: 0.25,
        color: [0; 4],
        material,
        max_stack: 64,
        food: None,
    };
    let items = Items(vec![
        item("t:chest", Some(chest)),
        item("t:rock", None),
        item("t:dust", None),
    ]);
    app.insert_resource(PlacingItems::new([(chest, "t:chest", true)], &items))
        .insert_resource(items)
        .add_plugins((tool::plugin, inventory::plugin, plugin));
    let mut held = tesserae_core::inventory::Inventory::default();
    for (at, name, count) in [(0, "t:chest", 3), (1, "t:rock", 10), (2, "t:dust", 5)] {
        let slot = Destination::Slot(Slot::Hotbar(at));
        held.take(name, count, slot, |_| 64).unwrap();
    }
    let mut players = app.world_mut().query::<&mut Inventory>();
    players.single_mut(app.world_mut()).unwrap().0 = held;
    (app, chest)
}

fn chest_material(tables: &mut Tables) -> MaterialId {
    let chest = tables.materials.register(MaterialDef::OPAQUE).unwrap();
    tables
        .register_use(chest, VoxelUse::Container { slots: 4 })
        .unwrap();
    // Fuel that never burns out, so a fire on it stays until put out.
    tables
        .register_combustion(
            chest,
            tesserae_core::CombustionDef {
                threshold: 6,
                ignition_delay_s: 2.0,
                emission: 0,
                heat: None,
                burn: tesserae_core::Burn::Permanent,
            },
        )
        .unwrap();
    chest
}

fn use_down(app: &mut App) -> Vec<UseOutcome> {
    set_eye(app, EYE);
    app.world_mut().write_message(UseIntent {
        tool: None,
        eat: true,
        origin: EYE,
        direction: Vec3::NEG_Y,
        expected: None,
    });
    app.world_mut().run_system_once(answer_use).unwrap();
    app.world_mut()
        .resource_mut::<Messages<UseIntent>>()
        .clear();
    app.world_mut()
        .resource_mut::<Messages<UseOutcome>>()
        .drain()
        .collect()
}

/// Runs the Container systems once, as a tick does after the World changed.
fn settle(app: &mut App) {
    app.world_mut().run_system_once(watch).unwrap();
    app.world_mut().run_system_once(announce).unwrap();
}

fn shown(app: &mut App) -> Vec<Vec<Option<Stack>>> {
    app.world_mut()
        .resource_mut::<Messages<ContainerShown>>()
        .drain()
        .map(|m| m.slots)
        .collect()
}

fn closed(app: &mut App) -> Vec<Closed> {
    app.world_mut()
        .resource_mut::<Messages<ContainerClosed>>()
        .drain()
        .map(|m| m.why)
        .collect()
}

fn ask(app: &mut App, intent: InventoryIntent) -> Vec<Reason> {
    app.world_mut().write_message(intent);
    app.world_mut().run_system_once(inventory::arrange).unwrap();
    app.world_mut()
        .resource_mut::<Messages<InventoryIntent>>()
        .clear();
    settle(app);
    app.world_mut()
        .resource_mut::<Messages<InventoryChanged>>()
        .clear();
    app.world_mut()
        .resource_mut::<Messages<InventoryRefused>>()
        .drain()
        .map(|m| m.0)
        .collect()
}

fn holding(app: &mut App) -> tesserae_core::inventory::Inventory {
    let mut players = app.world_mut().query::<&Inventory>();
    players.single(app.world()).unwrap().0.clone()
}

fn stack(item: &str, count: u32) -> Option<Stack> {
    Some(Stack {
        item: item.into(),
        count,
    })
}

fn mine(slot: Slot) -> SlotAddress {
    SlotAddress::Inventory(slot)
}

/// Opens the Chest and puts 10 rock in its slot 0.
fn stocked() -> App {
    let (mut app, _) = app();
    assert_eq!(
        use_down(&mut app),
        [UseOutcome(Ok(UseResult::Container(CHEST)))]
    );
    settle(&mut app);
    assert_eq!(shown(&mut app), [vec![None; 4]], "opened, empty");
    let moved = InventoryIntent::Move {
        from: mine(Slot::Hotbar(1)),
        to: SlotAddress::Container(0),
    };
    assert!(ask(&mut app, moved).is_empty());
    app
}

fn dropped(app: &App) -> Vec<(u32, u32)> {
    app.world()
        .resource::<EntityStore>()
        .iter()
        .map(|e| (e.as_dropped().item, e.as_dropped().count))
        .collect()
}

#[test]
fn use_on_a_chest_opens_it_and_stacks_move_both_ways() {
    let mut app = stocked();
    assert_eq!(
        shown(&mut app),
        [vec![stack("t:rock", 10), None, None, None]]
    );
    assert_eq!(holding(&mut app).get(Slot::Hotbar(1)), None);
    assert!(ask(&mut app, InventoryIntent::Split(SlotAddress::Container(0))).is_empty());
    assert_eq!(
        shown(&mut app),
        [vec![stack("t:rock", 5), stack("t:rock", 5), None, None]]
    );
    let merge = InventoryIntent::Move {
        from: SlotAddress::Container(1),
        to: SlotAddress::Container(0),
    };
    assert!(ask(&mut app, merge).is_empty());
    let swap = InventoryIntent::Move {
        from: mine(Slot::Hotbar(2)),
        to: SlotAddress::Container(0),
    };
    assert!(ask(&mut app, swap).is_empty());
    assert_eq!(
        holding(&mut app).get(Slot::Hotbar(2)).cloned(),
        stack("t:rock", 10)
    );
    assert_eq!(shown(&mut app).pop().unwrap()[0], stack("t:dust", 5));
    assert!(
        ask(
            &mut app,
            InventoryIntent::QuickTransfer(mine(Slot::Hotbar(2)))
        )
        .is_empty(),
        "an open Chest takes a quick transfer from the Inventory"
    );
    assert_eq!(
        shown(&mut app).pop().unwrap(),
        [stack("t:dust", 5), stack("t:rock", 10), None, None]
    );
    assert!(ask(&mut app, InventoryIntent::SortContainer).is_empty());
    assert_eq!(
        shown(&mut app).pop().unwrap(),
        [stack("t:dust", 5), stack("t:rock", 10), None, None],
        "sorted by Item name"
    );
    assert!(ask(&mut app, InventoryIntent::TakeAll).is_empty());
    assert_eq!(shown(&mut app).pop().unwrap(), [None, None, None, None]);
    let held = holding(&mut app);
    assert_eq!((held.count("t:rock"), held.count("t:dust")), (10, 5));
    assert!(app.world().resource::<edit::PendingEdits>().is_empty());
    assert!(
        app.world()
            .resource::<super::super::undo::Journal>()
            .is_empty(),
        "moves never enter the journal"
    );
}

#[test]
fn a_container_slot_with_none_open_or_past_its_end_is_refused() {
    let (mut app, _) = app();
    let into = |to| InventoryIntent::Move {
        from: mine(Slot::Hotbar(1)),
        to,
    };
    assert_eq!(
        ask(&mut app, into(SlotAddress::Container(0))),
        [Reason::NoContainer]
    );
    use_down(&mut app);
    settle(&mut app);
    assert_eq!(
        ask(&mut app, into(SlotAddress::Container(4))),
        [Reason::Rule(Refusal::NoSuchSlot)]
    );
    assert_eq!(
        ask(&mut app, InventoryIntent::TakeAll),
        [Reason::Rule(Refusal::Empty)]
    );
    ask(&mut app, InventoryIntent::CloseContainer);
    assert_eq!(
        ask(&mut app, into(SlotAddress::Container(0))),
        [Reason::NoContainer]
    );
    assert!(
        closed(&mut app).is_empty(),
        "the Player's own close is not answered"
    );
}

#[test]
fn a_refused_move_neither_resends_nor_saves_the_chest() {
    let mut app = stocked();
    shown(&mut app);
    let revision = app.world().resource::<Containers>().revision();
    let past_its_end = InventoryIntent::Move {
        from: mine(Slot::Hotbar(2)),
        to: SlotAddress::Container(4),
    };
    assert_eq!(
        ask(&mut app, past_its_end),
        [Reason::Rule(Refusal::NoSuchSlot)]
    );
    assert!(shown(&mut app).is_empty());
    assert_eq!(app.world().resource::<Containers>().revision(), revision);
}

#[test]
fn two_sessions_on_one_chest_share_its_stacks() {
    let mut app = stocked();
    shown(&mut app);
    let other = app.world_mut().spawn_empty().id();
    let chest = voxel_at(&app, CHEST);
    app.world_mut()
        .resource_mut::<Containers>()
        .open(other, CHEST, chest, 4);
    settle(&mut app);
    assert_eq!(
        shown(&mut app),
        [vec![stack("t:rock", 10), None, None, None]],
        "the second viewer opens on what the first put in"
    );
    assert!(ask(&mut app, InventoryIntent::TakeAll).is_empty());
    assert_eq!(
        shown(&mut app),
        [vec![None; 4]],
        "one change, sent whole to the Chest's viewers"
    );
}

fn remove_down(app: &mut App) {
    send(
        app,
        EditIntent {
            action: EditAction::Remove,
            origin: EYE,
            direction: Vec3::NEG_Y,
        },
    );
    tick(app);
}

#[test]
fn removing_a_chest_that_holds_stacks_is_refused_and_an_empty_one_breaks() {
    let mut app = stocked();
    remove_down(&mut app);
    let chest = voxel_at(&app, CHEST).material();
    assert_eq!(
        rejections(&mut app),
        [RejectReason::ContainerNotEmpty(chest)]
    );
    assert!(!voxel_at(&app, CHEST).is_air());
    ask(&mut app, InventoryIntent::TakeAll);
    remove_down(&mut app);
    assert!(rejections(&mut app).is_empty());
    assert!(voxel_at(&app, CHEST).is_air());
    settle(&mut app);
    assert_eq!(closed(&mut app), [Closed::Changed]);
}

#[test]
fn a_burning_full_chest_is_put_out_rather_than_refused() {
    let mut app = stocked();
    let tables = app.world().resource::<ContentTables>().0.clone();
    let mut store = app.world_mut().resource_mut::<ChunkStore>();
    let here = store.occupants(CHEST, &tables.fluids).unwrap();
    let lit = tesserae_core::Occupants {
        burning: Some(tesserae_core::Burning { fuel: 64 }),
        ..here
    };
    edit::set_occupants(&mut store, &mut MeshState::default(), &tables, CHEST, lit).unwrap();
    remove_down(&mut app);
    assert!(rejections(&mut app).is_empty());
    let store = app.world().resource::<ChunkStore>();
    let now = store.occupants(CHEST, &tables.fluids).unwrap();
    assert_eq!(now.burning, None);
    assert!(!now.solid.is_air(), "only the fire went");
    assert!(app.world().resource::<Containers>().holds_stacks(CHEST));
}

#[test]
fn undo_and_redo_that_would_remove_a_full_chest_are_refused() {
    let (mut app, chest) = app();
    let place_at = |app: &mut App| {
        set_eye(app, Vec3::new(7.5, 12.0, 5.5));
        send(app, {
            let mut intent = place_held(
                tesserae_core::SHAPE_FULL_CUBE,
                tesserae_core::Orientation::IDENTITY,
            );
            intent.origin = Vec3::new(7.5, 12.0, 5.5);
            intent
        });
        tick(app);
    };
    place_at(&mut app);
    assert!(rejections(&mut app).is_empty());
    let placed = [7, 8, 5];
    assert_eq!(voxel_at(&app, placed).material(), chest);
    let fill = |app: &mut App| {
        app.world_mut().resource_mut::<Containers>().open(
            Entity::PLACEHOLDER,
            placed,
            Voxel::cube(chest),
            4,
        );
        app.world_mut()
            .resource_mut::<Containers>()
            .viewing_mut(Entity::PLACEHOLDER)
            .unwrap()[0] = stack("t:rock", 1);
    };
    fill(&mut app);
    replay(&mut app, Undo);
    assert_eq!(
        rejections(&mut app),
        [RejectReason::ContainerNotEmpty(chest)]
    );
    assert_eq!(voxel_at(&app, placed).material(), chest);
    app.world_mut()
        .resource_mut::<Containers>()
        .viewing_mut(Entity::PLACEHOLDER)
        .unwrap()[0] = None;
    replay(&mut app, Undo);
    assert!(rejections(&mut app).is_empty(), "an empty one undoes");
    assert!(voxel_at(&app, placed).is_air());
    replay(&mut app, Redo);
    assert!(rejections(&mut app).is_empty());
    // A Remove recorded while empty, redone after it filled.
    app.world_mut()
        .resource_mut::<Containers>()
        .close(Entity::PLACEHOLDER);
    settle(&mut app);
    send(
        &mut app,
        EditIntent {
            action: EditAction::Remove,
            origin: Vec3::new(7.5, 12.0, 5.5),
            direction: Vec3::NEG_Y,
        },
    );
    tick(&mut app);
    assert!(rejections(&mut app).is_empty());
    replay(&mut app, Undo);
    assert!(rejections(&mut app).is_empty());
    fill(&mut app);
    replay(&mut app, Redo);
    assert_eq!(
        rejections(&mut app),
        [RejectReason::ContainerNotEmpty(chest)]
    );
    assert_eq!(voxel_at(&app, placed).material(), chest);
}

#[test]
fn a_mod_command_or_fire_that_replaces_a_full_chest_spills_every_stack() {
    for burnt in [false, true] {
        let mut app = stocked();
        let swap = InventoryIntent::Move {
            from: mine(Slot::Hotbar(2)),
            to: SlotAddress::Container(3),
        };
        ask(&mut app, swap);
        let tables = app.world().resource::<ContentTables>().0.clone();
        let mut meshes = MeshState::default();
        let mut store = app.world_mut().resource_mut::<ChunkStore>();
        if burnt {
            // The write a fire burning out makes: what stood there becomes air.
            let here = store.occupants(CHEST, &tables.fluids).unwrap();
            let ash = tesserae_core::Occupants {
                solid: Voxel::AIR,
                ..here
            };
            edit::set_occupants(&mut store, &mut meshes, &tables, CHEST, ash).unwrap();
        } else {
            let stone = Voxel::cube(tesserae_core::testgen::fixtures().ground.rock);
            edit::apply_intent(
                &mut store,
                &mut meshes,
                &tables,
                CHEST,
                Intent::Voxel(stone),
            )
            .unwrap();
        }
        settle(&mut app);
        assert_eq!(closed(&mut app), [Closed::Changed]);
        let mut spilled = dropped(&app);
        spilled.sort();
        assert_eq!(spilled, [(1, 10), (2, 5)], "burnt {burnt}");
        assert!(!app.world().resource::<Containers>().holds_stacks(CHEST));
    }
}

#[test]
fn walking_out_of_reach_closes_the_session() {
    let mut app = stocked();
    set_eye(&mut app, Vec3::new(5.5, 9.0 + EDIT_REACH_M + 1.0, 5.5));
    settle(&mut app);
    assert_eq!(closed(&mut app), [Closed::OutOfReach]);
    assert_eq!(
        ask(&mut app, InventoryIntent::TakeAll),
        [Reason::NoContainer]
    );
    assert!(
        app.world().resource::<Containers>().holds_stacks(CHEST),
        "the Stacks stay"
    );
}

#[test]
fn a_chest_sounds_at_itself_when_opened_and_closed_but_not_when_broken() {
    use super::super::sound::heard;
    use tesserae_core::audio::Cause::{ContainerClose, ContainerOpen};
    let centre = [5.5, 8.5, 5.5];
    let (mut app, _) = app();
    heard::voice(&mut app, &[ContainerOpen, ContainerClose]);
    ask(&mut app, InventoryIntent::CloseContainer);
    assert_eq!(heard::causes(&mut app), [], "nothing was open");
    use_down(&mut app);
    assert_eq!(heard::causes(&mut app), [(ContainerOpen, centre)]);
    ask(&mut app, InventoryIntent::CloseContainer);
    assert_eq!(heard::causes(&mut app), [(ContainerClose, centre)]);
    use_down(&mut app);
    set_eye(&mut app, Vec3::new(5.5, 9.0 + EDIT_REACH_M + 1.0, 5.5));
    settle(&mut app);
    assert_eq!(
        heard::causes(&mut app),
        [(ContainerOpen, centre), (ContainerClose, centre)],
        "walking away closes it"
    );
    assert_eq!(closed(&mut app), [Closed::OutOfReach]);
    set_eye(&mut app, EYE);
    use_down(&mut app);
    heard::causes(&mut app);
    remove_down(&mut app);
    settle(&mut app);
    assert_eq!(closed(&mut app), [Closed::Changed]);
    assert!(
        !heard::causes(&mut app)
            .iter()
            .any(|&(cause, _)| cause == ContainerClose),
        "a broken Chest is heard breaking, not closing"
    );
}
