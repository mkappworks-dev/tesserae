use super::*;
use bevy::ecs::message::Messages;
use bevy::ecs::system::RunSystemOnce;
use tesserae_core::SHAPE_FULL_CUBE;
use tesserae_core::inventory::{Destination, Slot};
use tesserae_core::layout::{Dim, LayoutAxis};
use tesserae_core::plant::tree::{Part, TreeVoxel, tree_voxels};
use tesserae_core::plant::{Branching, CountRange, Fruit, Kind, Needs, Plant, Stage, Tree, Yield};
use tesserae_core::{MaterialDef, MaterialId, Occlusion, Orientation, RenderClass};
use tesserae_mods::{Intent, ItemDef};

use super::super::edit::tests::{
    app_with_tables, ground, rejections, replay, send, set_eye, survival, tick, voxel_at,
};
use super::super::edit::{
    AimReport, EditAction, EditIntent, LayoutIntent, LayoutParams, RejectReason, send_layout,
};
use super::super::inventory::Inventory as InventoryComponent;
use super::super::items::PlacingItems;
use super::super::tool::{self, UseIntent, UseOutcome, UseWith, answer_use};
use super::super::undo::HistoryAction::{Redo, Undo};
use super::super::vitals::EatIntent;
use crate::world::ChunkCoord;

const EYE: Vec3 = Vec3::new(5.5, 12.0, 5.5);
const CROP: [i32; 3] = [5, 8, 5];

struct Content {
    tables: Tables,
    wheat: [MaterialId; 3],
    bush: [MaterialId; 3],
    sapling: MaterialId,
    trunk: MaterialId,
    canopy: MaterialId,
    fruit: [MaterialId; 2],
    cane: MaterialId,
    placed: MaterialId,
    cross: tesserae_core::ShapeId,
    /// The Shapes the berry bush's three Stages and the unripe fruit take when `staged`.
    stage_shapes: [tesserae_core::ShapeId; 4],
}

fn soil() -> MaterialId {
    tesserae_core::testgen::fixtures().ground.soil
}

fn rock() -> MaterialId {
    tesserae_core::testgen::fixtures().ground.rock
}

fn yields(rows: &[(&str, u32, u32)]) -> Vec<Yield> {
    rows.iter()
        .map(|&(item, min, max)| Yield {
            item: item.into(),
            count: CountRange { min, max },
        })
        .collect()
}

fn stage(material: MaterialId, mean: Option<f32>, rows: &[(&str, u32, u32)]) -> Stage {
    Stage {
        material,
        mean_seconds: mean,
        yields: yields(rows),
        shape: None,
    }
}

/// `shaped` gives the wheat Plant the `cross` Shape the Content registers.
fn content_with(shaped: bool) -> Content {
    content_of(shaped, None)
}

/// As [`content_with`], the apple tree branching as given and tall enough to show it.
fn content_of(shaped: bool, branching: Option<Branching>) -> Content {
    content_full(shaped, branching, false)
}

/// As [`content_of`]; `staged` gives each berry bush Stage and the unripe fruit a Shape of its own.
fn content_full(shaped: bool, branching: Option<Branching>, staged: bool) -> Content {
    let mut tables = crate::server::fixture_tables();
    let cross = tables
        .shapes
        .register(tesserae_core::ShapeDef {
            form: tesserae_core::Form::Cross,
            boxes: vec![tesserae_core::NodeBox::new([4, 0, 4], [12, 12, 12])],
            ..tesserae_core::ShapeDef::default()
        })
        .unwrap();
    let mut boxed = |top: u8| {
        tables
            .shapes
            .register(tesserae_core::ShapeDef {
                boxes: vec![tesserae_core::NodeBox::new([4, 0, 4], [12, top, 12])],
                collision: Some(Vec::new()),
                ..tesserae_core::ShapeDef::default()
            })
            .unwrap()
    };
    let stage_shapes = [boxed(3), boxed(8), boxed(12), boxed(6)];
    let own = |mut stage: Stage, n: usize| {
        stage.shape = staged.then_some(stage_shapes[n]);
        stage
    };
    let def = MaterialDef {
        emission: 0,
        dimming: 0,
        occlusion: Occlusion::Never,
        render: RenderClass::Cutout,
        solid: true,
    };
    let mut material = || tables.materials.register(def).unwrap();
    let wheat = [(); 3].map(|_| material());
    let bush = [(); 3].map(|_| material());
    let [sapling, trunk, canopy] = [(); 3].map(|_| material());
    let fruit = [(); 2].map(|_| material());
    let cane = material();
    let placed = material();
    let needs = || Needs {
        soil: vec![soil()],
        light_max: 15,
        ..Needs::default()
    };
    let grow = Some(60.0);
    let plant = |name: &str, seed: &str, first, kind| Plant {
        name: name.into(),
        seed: seed.into(),
        first,
        shape: None,
        needs: needs(),
        kind,
        sapling_yields: Vec::new(),
    };
    for p in [
        plant(
            "t:wheat",
            "t:seed",
            wheat[0],
            Kind::Crop {
                stages: vec![
                    stage(wheat[0], grow, &[]),
                    stage(wheat[1], grow, &[("t:seed", 0, 1)]),
                    stage(wheat[2], None, &[("t:wheat", 2, 4), ("t:seed", 1, 3)]),
                ],
                harvest_to: None,
                fruit_vine: None,
            },
        ),
        plant(
            "t:bush",
            "t:bush_seed",
            bush[0],
            Kind::Crop {
                stages: vec![
                    own(stage(bush[0], grow, &[]), 0),
                    own(stage(bush[1], grow, &[]), 1),
                    own(stage(bush[2], None, &[("t:berry", 2, 2)]), 2),
                ],
                harvest_to: Some(1),
                fruit_vine: None,
            },
        ),
        plant(
            "t:cane",
            "t:cane_seed",
            cane,
            Kind::Column {
                max_height_m: 4.0,
                direction: tesserae_core::plant::Direction::Up,
                mean_seconds: 60.0,
                stands_in: Vec::new(),
                yields: yields(&[("t:cane", 1, 2)]),
            },
        ),
        plant(
            "t:apple",
            "t:sapling",
            sapling,
            Kind::Tree(Tree {
                vines: None,
                branching,
                crowding_m: None,
                trunk,
                canopy,
                trunk_height_m: if branching.is_some() {
                    (4.0, 5.0)
                } else {
                    (2.0, 3.0)
                },
                canopy_radius_m: 2.0,
                mean_seconds: 60.0,
                trunk_yield: yields(&[("t:log", 1, 1)]),
                canopy_yield: yields(&[("t:leaf", 1, 1)]),
                sapling_yield: CountRange { min: 1, max: 2 },
                fruit: Some(Fruit {
                    stages: vec![
                        own(stage(fruit[0], grow, &[]), 3),
                        stage(fruit[1], None, &[("t:apple", 1, 1)]),
                    ],
                    density: 0.1,
                    regrow_seconds: 120.0,
                    shape: staged.then_some(stage_shapes[2]),
                }),
            }),
        ),
    ] {
        let mut p = p;
        if shaped && p.name == "t:wheat" {
            p.shape = Some(cross);
        }
        tables.plants.push(p).unwrap();
    }
    Content {
        tables,
        wheat,
        bush,
        sapling,
        trunk,
        canopy,
        fruit,
        cane,
        placed,
        cross,
        stage_shapes,
    }
}

fn item(name: &str, material: Option<MaterialId>, food: Option<f32>) -> ItemDef {
    ItemDef {
        name: name.into(),
        size_m: 0.25,
        color: [0; 4],
        material,
        max_stack: 64,
        food,
        texture: None,
    }
}

/// Soil at (5, 7, 5) and (6, 7, 5) in rock, with the Player holding three each of
/// the Seeds in hotbar slots 0 to 2 and the wheat Seed selected.
fn farm() -> (App, Content) {
    farm_with(false)
}

fn farm_with(shaped: bool) -> (App, Content) {
    farm_of(content_with(shaped))
}

fn farm_of(c: Content) -> (App, Content) {
    bevy::tasks::AsyncComputeTaskPool::get_or_init(|| {
        bevy::tasks::TaskPoolBuilder::new().num_threads(2).build()
    });
    let mut chunk = ground();
    for x in [5, 6] {
        chunk.set(x, 7, 5, Voxel::cube(soil()));
    }
    let mut app = app_with_tables(&[(ChunkCoord::new(0, 0, 0), chunk)], c.tables.clone());
    let items = Items::with_implicit(
        vec![
            item("t:seed", Some(c.wheat[0]), None),
            item("t:bush_seed", Some(c.bush[0]), None),
            item("t:sapling", Some(c.sapling), None),
            item("t:wheat", None, None),
            item("t:berry", None, Some(2.0)),
            item("t:apple", None, Some(2.0)),
            item("t:cane", None, None),
            item("t:log", None, None),
            item("t:leaf", None, None),
            item("t:junk", None, None),
        ],
        [(rock(), "t:stone", [0; 4]), (soil(), "t:soil", [0; 4])],
        1.0,
    );
    let placeable = |id, name, yes| (id, name, yes);
    app.insert_resource(PlacingItems::new(
        [
            placeable(rock(), "t:stone", true),
            placeable(soil(), "t:soil", true),
            placeable(c.wheat[0], "t:wheat_0", true),
            placeable(c.wheat[1], "t:wheat_1", false),
            placeable(c.wheat[2], "t:wheat_2", false),
            placeable(c.bush[0], "t:bush_0", true),
            placeable(c.sapling, "t:sapling_m", true),
            placeable(c.fruit[0], "t:fruit_0", false),
        ],
        &items,
    ))
    .insert_resource(items)
    .init_resource::<FluidSim>()
    .add_plugins(tool::plugin)
    .add_plugins(plugin);
    let mut inventory = tesserae_core::inventory::Inventory::default();
    for (at, name) in ["t:seed", "t:bush_seed", "t:sapling"]
        .into_iter()
        .enumerate()
    {
        inventory
            .take(name, 3, Destination::Slot(Slot::Hotbar(at)), |_| 64)
            .unwrap();
    }
    inventory.select(0).unwrap();
    app.world_mut()
        .query::<&mut InventoryComponent>()
        .single_mut(app.world_mut())
        .unwrap()
        .0 = inventory;
    (app, c)
}

fn inventory<R>(app: &mut App, f: impl FnOnce(&mut tesserae_core::inventory::Inventory) -> R) -> R {
    let mut q = app.world_mut().query::<&mut InventoryComponent>();
    f(&mut q.single_mut(app.world_mut()).unwrap().0)
}

fn count(app: &mut App, item: &str) -> u32 {
    inventory(app, |i| i.count(item))
}

fn set(app: &mut App, at: [i32; 3], voxel: Voxel) {
    app.world_mut()
        .run_system_once(
            move |mut store: ResMut<ChunkStore>,
                  mut meshes: ResMut<MeshState>,
                  tables: Res<ContentTables>| {
                edit::set_voxel(&mut store, &mut meshes, &tables, at, voxel).unwrap();
            },
        )
        .unwrap();
}

/// A Mod's Command, through the one World change.
fn command(app: &mut App, at: [i32; 3], voxel: Voxel) {
    app.world_mut()
        .run_system_once(
            move |mut store: ResMut<ChunkStore>,
                  mut meshes: ResMut<MeshState>,
                  tables: Res<ContentTables>| {
                edit::apply_intent(&mut store, &mut meshes, &tables, at, Intent::Voxel(voxel))
                    .unwrap();
            },
        )
        .unwrap();
}

/// One Logical step's look at what changed, and the Seeds that dropped.
fn settle(app: &mut App) {
    app.world_mut()
        .run_system_once(crate::server::fluids::simulate)
        .unwrap();
    app.world_mut().run_system_once(drop_seeds).unwrap();
    app.world_mut().run_system_once(drop_felled).unwrap();
}

/// The Dropped items lying in the World, by Item name, sorted.
fn lying(app: &mut App) -> Vec<(String, u32)> {
    let items = app.world().resource::<Items>().0.clone();
    let mut all: Vec<_> = app
        .world()
        .resource::<EntityStore>()
        .iter()
        .map(|e| {
            let d = e.as_dropped();
            (items[d.item as usize].name.clone(), d.count)
        })
        .collect();
    all.sort();
    all
}

fn plant_at(x: i32, z: i32) -> EditIntent {
    EditIntent {
        action: EditAction::PlaceHeld {
            shape: SHAPE_FULL_CUBE,
            orientation: Orientation::IDENTITY,
        },
        origin: Vec3::new(x as f32 + 0.5, 12.0, z as f32 + 0.5),
        direction: Vec3::NEG_Y,
    }
}

fn remove_crop() -> EditIntent {
    EditIntent {
        action: EditAction::Remove,
        origin: EYE,
        direction: Vec3::NEG_Y,
    }
}

fn use_down(app: &mut App) -> Vec<UseOutcome> {
    set_eye(app, EYE);
    app.world_mut().write_message(UseIntent {
        with: UseWith::Hand { eat: true },
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

fn eaten(app: &mut App) -> usize {
    app.world_mut()
        .resource_mut::<Messages<EatIntent>>()
        .drain()
        .count()
}

fn journal_is_empty(app: &App) -> bool {
    app.world()
        .resource::<super::super::undo::Journal>()
        .is_empty()
}

fn needs_text(app: &mut App) -> Vec<String> {
    app.world_mut()
        .resource_mut::<Messages<PlantNeedsSoil>>()
        .drain()
        .map(|m| m.text)
        .collect()
}

#[test]
fn a_seed_places_its_first_stage_and_is_spent_in_both_modes() {
    for survival_mode in [false, true] {
        let (mut app, c) = farm();
        if survival_mode {
            survival(&mut app);
        }
        send(&mut app, plant_at(5, 5));
        tick(&mut app);
        assert!(rejections(&mut app).is_empty());
        assert_eq!(voxel_at(&app, CROP), Voxel::cube(c.wheat[0]));
        assert_eq!(count(&mut app, "t:seed"), 2, "survival: {survival_mode}");
    }
}

#[test]
fn a_later_stage_is_no_item_and_not_in_the_catalogue() {
    let (app, c) = farm();
    let placing = app.world().resource::<PlacingItems>();
    for later in [c.wheat[1], c.wheat[2], c.fruit[0], c.trunk, c.canopy] {
        assert_eq!(placing.item(later), None);
    }
    assert_eq!(placing.item(c.wheat[0]), Some("t:seed"));
    assert_eq!(placing.material("t:wheat"), None);
}

#[test]
fn removing_each_stage_drops_its_yield_the_same_in_both_modes() {
    let mut by_mode = Vec::new();
    for survival_mode in [false, true] {
        let mut per_stage = Vec::new();
        for n in 0..3 {
            let (mut app, c) = farm();
            if survival_mode {
                survival(&mut app);
            }
            set(&mut app, CROP, Voxel::cube(c.wheat[n]));
            send(&mut app, remove_crop());
            tick(&mut app);
            assert!(rejections(&mut app).is_empty());
            assert!(voxel_at(&app, CROP).is_air());
            per_stage.push(lying(&mut app));
        }
        by_mode.push(per_stage);
    }
    assert_eq!(by_mode[0], by_mode[1], "Creative drops what Survival does");
    let [young, mid, ripe] = &by_mode[0][..] else {
        unreachable!()
    };
    assert_eq!(young, &[("t:seed".to_owned(), 1)], "no yield: its Seed");
    assert!(mid.iter().all(|(n, c)| n == "t:seed" && *c <= 1));
    let of = |name: &str| ripe.iter().find(|(n, _)| n == name).map(|(_, c)| *c);
    assert!((2..=4).contains(&of("t:wheat").unwrap()));
    assert!((1..=3).contains(&of("t:seed").unwrap()));
}

#[test]
fn cutting_a_column_voxel_drops_its_yield_and_nothing_else() {
    let (mut app, c) = farm();
    set(&mut app, CROP, Voxel::cube(c.cane));
    send(&mut app, remove_crop());
    tick(&mut app);
    assert!(voxel_at(&app, CROP).is_air());
    let lying = lying(&mut app);
    assert!(
        matches!(&lying[..], [(name, n)] if name == "t:cane" && (1..=2).contains(n)),
        "{lying:?}"
    );
}

#[test]
fn undo_of_a_seed_placement_and_redo_hold_after_the_crop_grew() {
    let (mut app, c) = farm();
    survival(&mut app);
    send(&mut app, plant_at(5, 5));
    tick(&mut app);
    set(&mut app, CROP, Voxel::cube(c.wheat[1]));
    replay(&mut app, Undo);
    assert!(rejections(&mut app).is_empty());
    assert!(voxel_at(&app, CROP).is_air());
    assert_eq!(count(&mut app, "t:seed"), 3, "refunded");
    replay(&mut app, Redo);
    assert!(rejections(&mut app).is_empty());
    assert_eq!(voxel_at(&app, CROP), Voxel::cube(c.wheat[0]));
    assert_eq!(count(&mut app, "t:seed"), 2, "spent again");
}

#[test]
fn undo_and_redo_of_a_removal_take_the_drops_back_and_drop_them_again() {
    let (mut app, c) = farm();
    set(&mut app, CROP, Voxel::cube(c.wheat[1]));
    send(&mut app, remove_crop());
    tick(&mut app);
    let first = lying(&mut app);
    replay(&mut app, Undo);
    assert!(rejections(&mut app).is_empty());
    assert_eq!(voxel_at(&app, CROP), Voxel::cube(c.wheat[1]));
    assert!(lying(&mut app).is_empty(), "taken back off the ground");
    set(&mut app, CROP, Voxel::cube(c.wheat[2]));
    replay(&mut app, Redo);
    assert!(rejections(&mut app).is_empty(), "the crop advanced a Stage");
    assert!(voxel_at(&app, CROP).is_air());
    assert_eq!(lying(&mut app), first, "dropped again");
}

#[test]
fn undoing_a_removal_whose_drops_were_picked_up_spends_them_or_is_refused() {
    let (mut app, c) = farm();
    set(&mut app, CROP, Voxel::cube(c.wheat[2]));
    send(&mut app, remove_crop());
    tick(&mut app);
    let dropped = lying(&mut app);
    let ids: Vec<_> = app
        .world()
        .resource::<EntityStore>()
        .iter()
        .map(|e| (e.id, e.as_dropped().count))
        .collect();
    app.world_mut()
        .resource_scope(|_, mut entities: Mut<EntityStore>| {
            for (id, n) in ids {
                entities.take(id, n);
            }
        });
    replay(&mut app, Undo);
    assert_eq!(rejections(&mut app), [RejectReason::ItemNeeded]);
    assert!(voxel_at(&app, CROP).is_air(), "refused whole");
    for (name, n) in &dropped {
        inventory(&mut app, |i| i.receive(name, *n, 64));
    }
    replay(&mut app, Undo);
    assert!(rejections(&mut app).is_empty());
    assert_eq!(voxel_at(&app, CROP), Voxel::cube(c.wheat[2]));
    for (name, _) in &dropped {
        assert_eq!(count(&mut app, name), if name == "t:seed" { 3 } else { 0 });
    }
}

#[test]
fn use_no_longer_harvests_a_ripe_crop_or_fruit_it_eats() {
    let (mut app, c) = farm();
    set(&mut app, CROP, Voxel::cube(c.bush[2]));
    assert!(use_down(&mut app).is_empty());
    assert_eq!(eaten(&mut app), 1);
    assert_eq!(voxel_at(&app, CROP), Voxel::cube(c.bush[2]));
    assert_eq!(count(&mut app, "t:berry"), 0);
    set(&mut app, CROP, Voxel::cube(c.fruit[1]));
    assert!(use_down(&mut app).is_empty());
    assert_eq!(eaten(&mut app), 1);
    assert_eq!(voxel_at(&app, CROP), Voxel::cube(c.fruit[1]));
    assert_eq!(count(&mut app, "t:apple"), 0);
    assert!(journal_is_empty(&app));
}

#[test]
fn the_aim_offers_no_use_on_a_plant_ripe_or_not() {
    let (mut app, c) = farm();
    let report = AimReport {
        seq: 1,
        origin: EYE,
        direction: Vec3::NEG_Y,
        held: None,
        tool: None,
        remove_held: false,
    };
    for material in [c.bush[1], c.bush[2], c.fruit[0], c.fruit[1]] {
        set(&mut app, CROP, Voxel::cube(material));
        let aimed = super::super::edit::tests::aim(&mut app, report).use_aim;
        assert_eq!(aimed, None, "{material}");
    }
}

#[test]
fn use_on_a_plant_with_nothing_to_harvest_eats() {
    let (mut app, c) = farm();
    set(&mut app, CROP, Voxel::cube(c.wheat[2]));
    assert!(use_down(&mut app).is_empty());
    assert_eq!(eaten(&mut app), 1);
    assert_eq!(voxel_at(&app, CROP), Voxel::cube(c.wheat[2]));
}

#[test]
fn a_harvest_that_does_not_fit_the_inventory_drops_at_the_player() {
    let (mut app, c) = farm();
    inventory(&mut app, |i| i.receive("t:junk", 36 * 64, 64));
    set(&mut app, CROP, Voxel::cube(c.bush[2]));
    send(&mut app, remove_crop());
    tick(&mut app);
    assert_eq!(count(&mut app, "t:berry"), 0);
    assert_eq!(lying(&mut app), [("t:berry".to_owned(), 2)]);
    assert_eq!(voxel_at(&app, CROP), Voxel::cube(c.bush[1]));
}

fn pickups(app: &mut App) -> Vec<Pickup> {
    app.world_mut()
        .resource_mut::<Messages<Pickup>>()
        .drain()
        .collect()
}

#[test]
fn a_harvest_tells_each_yield_as_a_pickup_so_the_feed_and_the_sound_answer_it() {
    let (mut app, c) = farm();
    set(&mut app, CROP, Voxel::cube(c.bush[2]));
    send(&mut app, remove_crop());
    tick(&mut app);
    assert_eq!(
        pickups(&mut app),
        [Pickup {
            item: "t:berry".into(),
            taken: 2,
            left: 0
        }]
    );

    let (mut app, c) = farm();
    inventory(&mut app, |i| i.receive("t:junk", 36 * 64, 64));
    set(&mut app, CROP, Voxel::cube(c.bush[2]));
    send(&mut app, remove_crop());
    tick(&mut app);
    assert!(
        pickups(&mut app).is_empty(),
        "nothing taken, so nothing told; the drop tells when it is collected"
    );
}

fn tree_with_fruit(app: &mut App, c: &Content, fruit: MaterialId) {
    set(app, [6, 7, 5], Voxel::cube(c.trunk));
    set(app, [6, 8, 5], Voxel::cube(c.canopy));
    set(app, CROP, Voxel::cube(fruit));
}

#[test]
fn breaking_ripe_fruit_drops_its_yield_leaves_canopy_and_the_tree_stands() {
    for survival_mode in [false, true] {
        let (mut app, c) = farm();
        if survival_mode {
            survival(&mut app);
        }
        tree_with_fruit(&mut app, &c, c.fruit[1]);
        send(&mut app, remove_crop());
        tick(&mut app);
        assert!(rejections(&mut app).is_empty());
        assert_eq!(voxel_at(&app, CROP), Voxel::cube(c.canopy));
        assert_eq!(lying(&mut app), [("t:apple".to_owned(), 1)]);
        assert_eq!(count(&mut app, "t:apple"), 0, "an ordinary removal drop");
        assert_eq!(voxel_at(&app, [6, 8, 5]), Voxel::cube(c.canopy));
        assert_eq!(voxel_at(&app, [6, 7, 5]), Voxel::cube(c.trunk));
        assert!(!journal_is_empty(&app));
    }
}

#[test]
fn breaking_unripe_fruit_drops_nothing_and_leaves_canopy() {
    let (mut app, c) = farm();
    tree_with_fruit(&mut app, &c, c.fruit[0]);
    send(&mut app, remove_crop());
    tick(&mut app);
    assert!(rejections(&mut app).is_empty());
    assert_eq!(voxel_at(&app, CROP), Voxel::cube(c.canopy));
    assert!(lying(&mut app).is_empty());
}

#[test]
fn undo_and_redo_of_a_fruit_break_restore_the_fruit_and_take_the_drops_back() {
    let (mut app, c) = farm();
    tree_with_fruit(&mut app, &c, c.fruit[1]);
    send(&mut app, remove_crop());
    tick(&mut app);
    let first = lying(&mut app);
    assert_eq!(first, [("t:apple".to_owned(), 1)]);
    replay(&mut app, Undo);
    assert!(rejections(&mut app).is_empty());
    assert_eq!(voxel_at(&app, CROP), Voxel::cube(c.fruit[1]));
    assert!(lying(&mut app).is_empty(), "taken back off the ground");
    replay(&mut app, Redo);
    assert!(rejections(&mut app).is_empty());
    assert_eq!(voxel_at(&app, CROP), Voxel::cube(c.canopy));
    assert_eq!(lying(&mut app), first, "dropped again");
}

#[test]
fn removing_a_ripe_berry_bush_harvests_it_and_the_unripe_bush_breaks() {
    for survival_mode in [false, true] {
        let (mut app, c) = farm();
        if survival_mode {
            survival(&mut app);
        }
        set(&mut app, CROP, Voxel::cube(c.bush[2]));
        send(&mut app, remove_crop());
        tick(&mut app);
        assert_eq!(voxel_at(&app, CROP), Voxel::cube(c.bush[1]));
        assert_eq!(count(&mut app, "t:berry"), 2);
        assert!(journal_is_empty(&app));
        send(&mut app, remove_crop());
        tick(&mut app);
        assert!(voxel_at(&app, CROP).is_air(), "the reset Stage is broken");
    }
}

#[test]
fn a_seed_on_the_wrong_ground_is_refused_naming_the_need_in_both_modes() {
    for survival_mode in [false, true] {
        let (mut app, _) = farm();
        if survival_mode {
            survival(&mut app);
        }
        send(&mut app, plant_at(9, 5));
        tick(&mut app);
        assert_eq!(rejections(&mut app), [RejectReason::NeedsSoil]);
        assert_eq!(needs_text(&mut app), ["Wheat needs soil."]);
        assert!(voxel_at(&app, [9, 8, 5]).is_air());
        assert_eq!(count(&mut app, "t:seed"), 3, "nothing spent");
    }
}

#[test]
fn a_seed_in_the_dark_is_accepted_and_waits() {
    let (mut app, c) = farm();
    // No light was ever solved for this Chunk: light is never a placement rule.
    send(&mut app, plant_at(5, 5));
    tick(&mut app);
    assert!(rejections(&mut app).is_empty());
    assert_eq!(voxel_at(&app, CROP), Voxel::cube(c.wheat[0]));
}

fn chosen() -> (tesserae_core::ShapeId, Orientation) {
    (tesserae_core::SHAPE_STAIR, Orientation::IDENTITY.turned())
}

#[test]
fn a_seed_is_the_plants_shape_whatever_shape_and_orientation_the_player_chose() {
    for survival_mode in [false, true] {
        let (mut app, c) = farm_with(true);
        if survival_mode {
            survival(&mut app);
        }
        let (shape, orientation) = chosen();
        let mut intent = plant_at(5, 5);
        intent.action = EditAction::PlaceHeld { shape, orientation };
        send(&mut app, intent);
        tick(&mut app);
        assert!(rejections(&mut app).is_empty());
        assert_eq!(voxel_at(&app, CROP), Voxel::new(c.wheat[0], c.cross, 0));
        assert_eq!(count(&mut app, "t:seed"), 2, "survival: {survival_mode}");
    }
}

#[test]
fn a_plant_that_names_no_shape_keeps_the_players_choice() {
    let (mut app, c) = farm();
    let (shape, orientation) = chosen();
    let mut intent = plant_at(5, 5);
    intent.action = EditAction::PlaceHeld { shape, orientation };
    send(&mut app, intent);
    tick(&mut app);
    assert!(rejections(&mut app).is_empty());
    assert_eq!(
        voxel_at(&app, CROP),
        Voxel::shaped(c.wheat[0], shape, orientation)
    );
}

#[test]
fn a_layout_of_seeds_is_the_plants_shape_in_every_cell() {
    for survival_mode in [false, true] {
        let (mut app, c) = farm_with(true);
        if survival_mode {
            survival(&mut app);
        }
        set(&mut app, [7, 7, 5], Voxel::cube(soil()));
        let (shape, orientation) = chosen();
        let mut intent = row(3);
        intent.voxel = Voxel::shaped(rock(), shape, orientation);
        send_layout(&mut app, intent);
        tick(&mut app);
        assert!(rejections(&mut app).is_empty());
        for x in 5..8 {
            assert_eq!(
                voxel_at(&app, [x, 8, 5]),
                Voxel::new(c.wheat[0], c.cross, 0),
                "x {x}, survival: {survival_mode}"
            );
        }
    }
}

#[test]
fn undo_and_redo_of_a_seed_keep_the_plants_shape() {
    let (mut app, c) = farm_with(true);
    send(&mut app, plant_at(5, 5));
    tick(&mut app);
    replay(&mut app, Undo);
    assert!(voxel_at(&app, CROP).is_air());
    replay(&mut app, Redo);
    assert!(rejections(&mut app).is_empty());
    assert_eq!(voxel_at(&app, CROP), Voxel::new(c.wheat[0], c.cross, 0));
}

#[test]
fn the_catalogue_reports_the_plants_shape_as_the_materials_fixed_shape() {
    let c = content_with(true);
    assert_eq!(c.tables.plants.fixed_shape(c.wheat[0]), Some(c.cross));
    assert_eq!(
        c.tables.plants.fixed_shape(c.wheat[1]),
        None,
        "a later Stage is never placed"
    );
    assert_eq!(
        content_with(false).tables.plants.fixed_shape(c.wheat[0]),
        None
    );
}

fn row(count: u32) -> LayoutIntent {
    LayoutIntent {
        origin: EYE,
        direction: Vec3::NEG_Y,
        voxel: Voxel::cube(rock()),
        layout: LayoutParams {
            axes: [
                Some(LayoutAxis {
                    dim: Dim::X,
                    positive: true,
                    count,
                }),
                None,
                None,
            ],
            gap: 0,
            span: None,
        },
    }
}

#[test]
fn a_layout_of_seeds_is_refused_whole_when_one_cell_has_no_soil() {
    for survival_mode in [false, true] {
        let (mut app, c) = farm();
        if survival_mode {
            survival(&mut app);
        }
        send_layout(&mut app, row(3));
        tick(&mut app);
        assert_eq!(rejections(&mut app), [RejectReason::NeedsSoil]);
        assert_eq!(needs_text(&mut app), ["Wheat needs soil."]);
        for x in 5..8 {
            assert!(voxel_at(&app, [x, 8, 5]).is_air(), "x {x}");
        }
        assert_eq!(count(&mut app, "t:seed"), 3);

        set(&mut app, [7, 7, 5], Voxel::cube(soil()));
        send_layout(&mut app, row(3));
        tick(&mut app);
        assert!(rejections(&mut app).is_empty());
        for x in 5..8 {
            assert_eq!(voxel_at(&app, [x, 8, 5]), Voxel::cube(c.wheat[0]));
        }
        assert_eq!(count(&mut app, "t:seed"), 0);
    }
}

/// A Player's Remove from a trench beside the soil.
fn dig_out_the_soil(app: &mut App) {
    for x in [3, 4] {
        set(app, [x, 7, 5], Voxel::AIR);
    }
    send(
        app,
        EditIntent {
            action: EditAction::Remove,
            origin: Vec3::new(4.5, 7.5, 5.5),
            direction: Vec3::X,
        },
    );
    tick(app);
    assert!(rejections(app).is_empty());
    assert!(voxel_at(app, [5, 7, 5]).is_air());
}

#[test]
fn digging_away_the_soil_breaks_the_plant_in_that_step_and_drops_its_seed() {
    let (mut app, c) = farm();
    send(&mut app, plant_at(5, 5));
    tick(&mut app);
    assert_eq!(voxel_at(&app, CROP), Voxel::cube(c.wheat[0]));
    dig_out_the_soil(&mut app);
    settle(&mut app);
    assert!(voxel_at(&app, CROP).is_air());
    let seeds = lying(&mut app).into_iter().find(|(n, _)| n == "t:seed");
    assert_eq!(seeds, Some(("t:seed".to_owned(), 1)));
}

#[test]
fn burning_the_soil_breaks_the_plant_when_the_fire_burns_out_and_drops_its_seed() {
    let mut c = content_with(false);
    c.tables
        .register_combustion(
            soil(),
            tesserae_core::CombustionDef {
                threshold: 6,
                ignition_delay_s: 1.0,
                emission: 0,
                heat: None,
                burn: tesserae_core::Burn::For {
                    seconds: 0.5,
                    residue: rock(),
                },
            },
        )
        .unwrap();
    let (mut app, c) = farm_of(c);
    send(&mut app, plant_at(5, 5));
    tick(&mut app);
    assert_eq!(voxel_at(&app, CROP), Voxel::cube(c.wheat[0]));
    app.world_mut()
        .run_system_once(
            |mut store: ResMut<ChunkStore>,
             mut meshes: ResMut<MeshState>,
             tables: Res<ContentTables>| {
                edit::apply_intent(&mut store, &mut meshes, &tables, [5, 7, 5], Intent::Ignite)
                    .unwrap();
            },
        )
        .unwrap();
    assert_eq!(
        voxel_at(&app, CROP),
        Voxel::cube(c.wheat[0]),
        "burning holds"
    );
    for _ in 0..400 {
        settle(&mut app);
        std::thread::sleep(std::time::Duration::from_micros(200));
        if voxel_at(&app, CROP).is_air() {
            break;
        }
    }
    assert_eq!(voxel_at(&app, [5, 7, 5]), Voxel::cube(rock()));
    assert!(voxel_at(&app, CROP).is_air());
    let seeds = lying(&mut app).into_iter().find(|(n, _)| n == "t:seed");
    assert_eq!(seeds, Some(("t:seed".to_owned(), 1)));
}

#[test]
fn a_mod_command_replacing_the_soil_breaks_the_plant_and_new_soil_holds_one_up() {
    let (mut app, c) = farm();
    set(&mut app, CROP, Voxel::cube(c.wheat[1]));
    command(&mut app, [5, 7, 5], Voxel::cube(rock()));
    settle(&mut app);
    assert!(voxel_at(&app, CROP).is_air());
    assert!(lying(&mut app).iter().any(|(n, _)| n == "t:seed"));

    let (mut app, c) = farm();
    command(&mut app, [9, 8, 9], Voxel::cube(soil()));
    command(&mut app, [9, 9, 9], Voxel::cube(c.wheat[0]));
    settle(&mut app);
    assert_eq!(voxel_at(&app, [9, 9, 9]), Voxel::cube(c.wheat[0]));
    assert!(lying(&mut app).is_empty());
}

#[test]
fn nothing_is_polled_an_unsupported_plant_nobody_touched_stays_until_a_neighbour_changes() {
    let (mut app, c) = farm();
    let at = [11, 8, 5];
    app.world_mut()
        .resource_mut::<ChunkStore>()
        .set_voxel(VoxelCoord::of(at), Voxel::cube(c.wheat[0]));
    for _ in 0..6 {
        settle(&mut app);
    }
    assert_eq!(voxel_at(&app, at), Voxel::cube(c.wheat[0]), "never scanned");
    set(&mut app, [12, 8, 5], Voxel::cube(rock()));
    settle(&mut app);
    assert!(voxel_at(&app, at).is_air(), "woken by the change beside it");
}

#[test]
fn a_broken_plant_unsupports_what_rests_on_it_in_the_same_step() {
    let (mut app, c) = farm();
    // A column of rock under one spot stands in for a chain: break the soil and
    // the crop goes, then a crop-on-crop (itself unsoiled) goes with it.
    set(&mut app, CROP, Voxel::cube(c.wheat[0]));
    set(&mut app, [5, 9, 5], Voxel::cube(c.wheat[0]));
    command(&mut app, [5, 7, 5], Voxel::cube(rock()));
    settle(&mut app);
    assert!(voxel_at(&app, CROP).is_air());
    assert!(voxel_at(&app, [5, 9, 5]).is_air());
}

#[test]
fn a_run_of_stages_is_the_same_voxel_to_the_journal_but_a_trunk_is_not_a_sapling() {
    let (app, c) = farm();
    let tables = &app.world().resource::<ContentTables>().0;
    assert_eq!(tables.plants.canonical(c.wheat[2]), c.wheat[0]);
    assert_eq!(tables.plants.canonical(c.fruit[1]), c.fruit[0]);
    assert_ne!(tables.plants.canonical(c.trunk), c.sapling);
    assert_eq!(tables.plants.canonical(rock()), rock());
}

#[test]
fn undoing_a_sapling_placement_after_it_grew_is_refused() {
    let (mut app, c) = farm();
    inventory(&mut app, |i| i.select(2).unwrap());
    send(&mut app, plant_at(5, 5));
    tick(&mut app);
    assert_eq!(voxel_at(&app, CROP), Voxel::cube(c.sapling));
    set(&mut app, CROP, Voxel::cube(c.trunk));
    replay(&mut app, Undo);
    assert_eq!(rejections(&mut app), [RejectReason::HistoryChanged]);
    assert_eq!(voxel_at(&app, CROP), Voxel::cube(c.trunk));
}

const BASE: [i32; 3] = [6, 8, 5];

fn the_tree(app: &App) -> Tree {
    let tables = &app.world().resource::<ContentTables>().0;
    match &tables
        .plants
        .plants()
        .iter()
        .find(|p| p.name == "t:apple")
        .unwrap()
        .kind
    {
        Kind::Tree(tree) => tree.clone(),
        _ => unreachable!(),
    }
}

/// Grows the apple tree whose sapling would stand at `base`, as worldgen or a
/// Sample would, and returns its Voxels.
fn grow_tree(app: &mut App, base: [i32; 3]) -> Vec<TreeVoxel> {
    let (tree, seed) = (the_tree(app), app.world().resource::<WorldSettings>().seed);
    let voxels = tree_voxels(&tree, seed, base, 1.0);
    for v in &voxels {
        set(app, v.at, Voxel::cube(v.material));
    }
    voxels
}

fn grown_left(app: &App, voxels: &[TreeVoxel]) -> usize {
    voxels
        .iter()
        .filter(|v| !voxel_at(app, v.at).is_air())
        .count()
}

/// Settles until the Felling queue is empty, returning how many tree Voxels
/// broke in each Logical step that broke any.
fn settle_felling(app: &mut App, voxels: &[TreeVoxel]) -> Vec<Vec<[i32; 3]>> {
    let mut steps = Vec::new();
    let mut standing: Vec<[i32; 3]> = voxels
        .iter()
        .map(|v| v.at)
        .filter(|&c| !voxel_at(app, c).is_air())
        .collect();
    for _ in 0..400 {
        settle(app);
        std::thread::sleep(std::time::Duration::from_micros(50));
        let now: Vec<[i32; 3]> = standing
            .iter()
            .copied()
            .filter(|&c| !voxel_at(app, c).is_air())
            .collect();
        if now.len() != standing.len() {
            steps.push(
                standing
                    .iter()
                    .copied()
                    .filter(|c| !now.contains(c))
                    .collect(),
            );
            standing = now;
        }
        if app.world().resource::<FluidSim>().felling_left() == 0 && !steps.is_empty() {
            break;
        }
    }
    steps
}

fn lying_of(app: &mut App, item: &str) -> (usize, u32) {
    let all = lying(app);
    let of: Vec<_> = all.iter().filter(|(n, _)| n == item).collect();
    (of.len(), of.iter().map(|(_, c)| c).sum())
}

fn cut_the_foot(app: &mut App) {
    send(
        app,
        EditIntent {
            action: EditAction::Remove,
            origin: Vec3::new(4.5, 8.5, 5.5),
            direction: Vec3::X,
        },
    );
    tick(app);
    assert!(rejections(app).is_empty());
    assert!(voxel_at(app, BASE).is_air());
}

#[test]
fn cutting_the_trunk_breaks_the_part_above_top_down_under_its_bound_and_drops_items() {
    let (mut app, c) = farm();
    let voxels = grow_tree(&mut app, BASE);
    let ripe = voxels
        .iter()
        .find(|v| v.part == Part::Fruit)
        .map(|v| v.at)
        .expect("the apple tree bears fruit");
    set(&mut app, ripe, Voxel::cube(c.fruit[1]));
    app.world_mut()
        .resource_mut::<FluidSim>()
        .set_felled_bound(6);
    let trunk = voxels.iter().filter(|v| v.part == Part::Trunk).count() as u32;
    let leaves = voxels.iter().filter(|v| v.part == Part::Canopy).count() as u32;
    assert!(voxels.len() > 12, "enough Voxels to take several steps");

    cut_the_foot(&mut app);
    let steps = settle_felling(&mut app, &voxels);
    assert!(steps.len() >= 3, "{} steps", steps.len());
    assert!(
        steps.iter().all(|s| s.len() <= 6),
        "a step broke past its bound"
    );
    for pair in steps.windows(2) {
        let lowest = pair[0].iter().map(|c| c[1]).min().unwrap();
        let highest = pair[1].iter().map(|c| c[1]).max().unwrap();
        assert!(lowest >= highest, "top-down: {lowest} then {highest}");
    }
    assert_eq!(grown_left(&app, &voxels), 0);
    assert_eq!(lying_of(&mut app, "t:log"), (trunk as usize, trunk));
    let fallen = leaves as usize;
    assert_eq!(lying_of(&mut app, "t:leaf").1, fallen as u32);
    assert_eq!(
        lying_of(&mut app, "t:apple"),
        (1, 1),
        "only the ripe fruit gives"
    );
    let (stacks, saplings) = lying_of(&mut app, "t:sapling");
    assert_eq!(stacks, 1, "one drop for the whole tree");
    assert!((1..=2).contains(&saplings));
}

#[test]
fn undo_after_a_felling_restores_only_the_cut_voxel_and_the_journal_holds_one_group() {
    let (mut app, c) = farm();
    let voxels = grow_tree(&mut app, BASE);
    cut_the_foot(&mut app);
    settle_felling(&mut app, &voxels);
    assert_eq!(grown_left(&app, &voxels), 0);
    assert_eq!(
        app.world()
            .resource::<super::super::undo::Journal>()
            .groups(),
        1
    );
    replay(&mut app, Undo);
    assert!(rejections(&mut app).is_empty());
    assert_eq!(voxel_at(&app, BASE), Voxel::cube(c.trunk));
    assert_eq!(grown_left(&app, &voxels), 1, "the cut Voxel and no more");
    settle(&mut app);
    assert_eq!(
        voxel_at(&app, BASE),
        Voxel::cube(c.trunk),
        "a stump is grounded"
    );
}

#[test]
fn a_cut_that_leaves_the_tree_grounded_breaks_nothing_and_a_leaf_drops_only_its_yield() {
    let (mut app, _) = farm();
    let voxels = grow_tree(&mut app, BASE);
    let top = (0..20)
        .rev()
        .map(|y| [6, y, 5])
        .find(|&c| !voxel_at(&app, c).is_air())
        .unwrap();
    let leaf = voxel_at(&app, top).material();
    send(
        &mut app,
        EditIntent {
            action: EditAction::Remove,
            origin: Vec3::new(6.5, 18.5, 5.5),
            direction: Vec3::NEG_Y,
        },
    );
    tick(&mut app);
    assert!(rejections(&mut app).is_empty());
    assert!(voxel_at(&app, top).is_air());
    let before = grown_left(&app, &voxels);
    for _ in 0..30 {
        settle(&mut app);
    }
    assert_eq!(grown_left(&app, &voxels), before, "nothing else broke");
    assert_eq!(lying_of(&mut app, "t:sapling"), (0, 0));
    let tables = app.world().resource::<ContentTables>().0.clone();
    let expected: u32 = tables.plants.yields(leaf).iter().map(|y| y.count.min).sum();
    let dropped: u32 = lying(&mut app).iter().map(|(_, c)| c).sum();
    assert_eq!(dropped, expected, "only the leaf's own declared yield");
}

#[test]
fn placed_voxels_of_any_look_are_never_felled_a_cabin_stays_and_a_touching_one_stays() {
    let (mut app, c) = farm();
    let voxels = grow_tree(&mut app, BASE);
    let beside = [BASE[0] + 1, BASE[1], BASE[2]];
    assert!(voxel_at(&app, beside).is_air());
    set(&mut app, beside, Voxel::cube(c.placed));
    let cabin: Vec<[i32; 3]> = (0..3)
        .flat_map(|x| (0..3).flat_map(move |y| (0..3).map(move |z| [14 + x, 8 + y, 14 + z])))
        .collect();
    for &cell in &cabin {
        set(&mut app, cell, Voxel::cube(c.placed));
    }
    cut_the_foot(&mut app);
    settle_felling(&mut app, &voxels);
    assert_eq!(grown_left(&app, &voxels), 0, "the tree fell");
    assert_eq!(voxel_at(&app, beside), Voxel::cube(c.placed));
    assert!(
        cabin
            .iter()
            .all(|&cell| voxel_at(&app, cell) == Voxel::cube(c.placed))
    );

    command(&mut app, [14, 9, 14], Voxel::AIR);
    for _ in 0..30 {
        settle(&mut app);
    }
    assert!(
        cabin
            .iter()
            .filter(|&&cell| cell != [14, 9, 14])
            .all(|&cell| { voxel_at(&app, cell) == Voxel::cube(c.placed) })
    );
    assert_eq!(
        lying_of(&mut app, "t:sapling").0,
        1,
        "the cabin dropped none"
    );
}

#[test]
fn a_mod_command_and_a_burn_out_fell_by_the_same_rule() {
    let (mut app, _) = farm();
    let voxels = grow_tree(&mut app, BASE);
    command(&mut app, [6, 9, 5], Voxel::AIR);
    let steps = settle_felling(&mut app, &voxels);
    assert!(!steps.is_empty());
    assert_eq!(
        grown_left(&app, &voxels),
        1,
        "the foot below the cut stands"
    );
    assert_eq!(lying_of(&mut app, "t:sapling").0, 1);

    let (mut app, _) = farm();
    let voxels = grow_tree(&mut app, BASE);
    app.world_mut()
        .run_system_once(
            |mut store: ResMut<ChunkStore>,
             mut meshes: ResMut<MeshState>,
             tables: Res<ContentTables>| {
                let at = [6, 9, 5];
                let here = store.occupants(at, &tables.fluids).unwrap();
                let gone = Occupants {
                    solid: Voxel::AIR,
                    ..here
                };
                edit::set_occupants(&mut store, &mut meshes, &tables, at, gone).unwrap();
            },
        )
        .unwrap();
    settle_felling(&mut app, &voxels);
    assert_eq!(grown_left(&app, &voxels), 1);
}

#[test]
fn a_trunk_standing_on_a_crop_is_grounded_by_it() {
    let (mut app, c) = farm();
    let foot = [BASE[0], BASE[1] + 1, BASE[2]];
    set(&mut app, BASE, Voxel::cube(c.wheat[0]));
    let voxels = grow_tree(&mut app, foot);
    command(&mut app, [foot[0], foot[1] + 1, foot[2]], Voxel::AIR);
    let steps = settle_felling(&mut app, &voxels);
    assert!(!steps.is_empty());
    assert_eq!(grown_left(&app, &voxels), 1, "the foot stays on the crop");
    assert_eq!(voxel_at(&app, BASE), Voxel::cube(c.wheat[0]));
}

#[test]
fn a_region_over_the_plants_size_in_metres_does_not_break_and_a_small_one_does() {
    let (mut app, c) = farm();
    let big: Vec<[i32; 3]> = (0..14).map(|x| [2 + x, 16, 20]).collect();
    for &cell in &big {
        set(&mut app, cell, Voxel::cube(c.canopy));
    }
    set(&mut app, [1, 16, 20], Voxel::cube(c.placed));
    command(&mut app, [1, 16, 20], Voxel::AIR);
    for _ in 0..30 {
        settle(&mut app);
        std::thread::sleep(std::time::Duration::from_micros(500));
    }
    assert!(big.iter().all(|&cell| !voxel_at(&app, cell).is_air()));

    let small: Vec<[i32; 3]> = (0..3).map(|x| [2 + x, 16, 26]).collect();
    for &cell in &small {
        set(&mut app, cell, Voxel::cube(c.canopy));
    }
    set(&mut app, [1, 16, 26], Voxel::cube(c.placed));
    command(&mut app, [1, 16, 26], Voxel::AIR);
    // Felling waits on the async light solve, so step until it drains.
    for _ in 0..400 {
        settle(&mut app);
        std::thread::sleep(std::time::Duration::from_micros(500));
        if small.iter().all(|&cell| voxel_at(&app, cell).is_air()) {
            break;
        }
    }
    assert!(
        small.iter().all(|&cell| voxel_at(&app, cell).is_air()),
        "the small region was not felled within 400 steps"
    );
}

#[test]
fn a_felled_trees_sapling_is_the_plants_seed_item_and_plants_to_grow_another() {
    let (mut app, c) = farm();
    let voxels = grow_tree(&mut app, BASE);
    command(&mut app, [6, 9, 5], Voxel::AIR);
    settle_felling(&mut app, &voxels);
    let items = app.world().resource::<Items>().0.clone();
    let sapling = app
        .world()
        .resource::<EntityStore>()
        .iter()
        .map(|e| e.as_dropped())
        .find(|d| items[d.item as usize].name == "t:sapling")
        .expect("a sapling lies where it was cut");
    assert_eq!(items[sapling.item as usize].material, Some(c.sapling));
}

#[test]
fn removing_a_sapling_drops_its_item_and_a_grown_trunk_drops_the_log_never_a_grown_material() {
    let (mut app, c) = farm();
    set(&mut app, CROP, Voxel::cube(c.sapling));
    send(&mut app, remove_crop());
    tick(&mut app);
    assert_eq!(lying(&mut app), [("t:sapling".to_owned(), 1)]);

    let (mut app, c) = farm();
    set(&mut app, [6, 8, 5], Voxel::cube(c.trunk));
    send(
        &mut app,
        EditIntent {
            action: EditAction::Remove,
            origin: Vec3::new(4.5, 8.5, 5.5),
            direction: Vec3::X,
        },
    );
    tick(&mut app);
    assert_eq!(lying(&mut app), [("t:log".to_owned(), 1)]);
}

fn branching() -> Branching {
    Branching {
        limbs: CountRange { min: 2, max: 3 },
        start: 0.2,
        reach_m: 3.0,
        rise: 0.0,
        fork: None,
        lean_m: 0.0,
        tiers: false,
        lobe_radius_m: 0.6,
    }
}

#[test]
fn cutting_a_branched_trunk_fells_its_limbs_and_lobes_and_drops_one_sapling() {
    let (mut app, _) = farm_of(content_of(false, Some(branching())));
    let voxels = grow_tree(&mut app, BASE);
    let wood = voxels.iter().filter(|v| v.part == Part::Trunk).count();
    let off_axis = voxels
        .iter()
        .filter(|v| v.part == Part::Trunk && (v.at[0] != BASE[0] || v.at[2] != BASE[2]))
        .count();
    assert!(off_axis > 3, "limbs and leaders stand off the trunk");
    cut_the_foot(&mut app);
    settle_felling(&mut app, &voxels);
    assert_eq!(grown_left(&app, &voxels), 0, "the limbs and lobes fell too");
    assert_eq!(lying_of(&mut app, "t:log"), (wood, wood as u32));
    let (stacks, saplings) = lying_of(&mut app, "t:sapling");
    assert_eq!(stacks, 1, "one drop for the whole tree");
    assert!((1..=2).contains(&saplings));
}

#[test]
fn cutting_a_limb_drops_its_end_and_what_hung_on_it_and_leaves_the_trunk_standing() {
    let (mut app, _) = farm_of(content_of(false, Some(branching())));
    let voxels = grow_tree(&mut app, BASE);
    let across = |at: [i32; 3]| (at[0] - BASE[0]).abs().max((at[2] - BASE[2]).abs());
    let limbs: Vec<_> = voxels
        .iter()
        .filter(|v| v.part == Part::Trunk && across(v.at) >= 1)
        .collect();
    // The lowest limb hangs clear of the crown, so nothing else holds its end up.
    let low = limbs.iter().map(|v| v.at[1]).min().unwrap();
    let lowest: Vec<_> = limbs.iter().filter(|v| v.at[1] == low).collect();
    let cut = lowest.iter().find(|v| across(v.at) >= 2).unwrap().at;
    let tip = lowest.iter().max_by_key(|v| across(v.at)).unwrap().at;
    let lobe = voxels
        .iter()
        .find(|v| v.part != Part::Trunk && v.at[1] <= low + 1 && across(v.at) >= 3)
        .expect("a lobe at the limb's tip")
        .at;
    command(&mut app, cut, Voxel::AIR);
    settle_felling(&mut app, &voxels);
    assert!(!voxel_at(&app, BASE).is_air(), "the foot stands");
    let axis: Vec<_> = voxels
        .iter()
        .filter(|v| v.part == Part::Trunk && across(v.at) == 0)
        .collect();
    assert!(
        axis.iter().all(|v| !voxel_at(&app, v.at).is_air()),
        "the trunk stands"
    );
    assert!(voxel_at(&app, tip).is_air(), "the end of the limb fell");
    assert!(voxel_at(&app, lobe).is_air(), "what hung on it fell");
    assert!(
        voxels
            .iter()
            .filter(|v| v.part == Part::Canopy)
            .any(|v| !voxel_at(&app, v.at).is_air()),
        "the crown stays"
    );
}

fn staged_farm() -> (App, Content) {
    farm_of(content_full(false, None, true))
}

#[test]
fn a_berry_seed_places_stage_zeros_shape_and_a_ripe_break_resets_to_the_young_shape() {
    let (mut app, c) = staged_farm();
    let [sprout, young, ripe, _] = c.stage_shapes;
    assert_eq!(c.tables.plants.fixed_shape(c.bush[0]), Some(sprout));
    assert_eq!(c.tables.plants.fixed_shape(c.bush[1]), None);
    inventory(&mut app, |i| i.select(1).unwrap());
    send(&mut app, plant_at(5, 5));
    tick(&mut app);
    assert_eq!(voxel_at(&app, CROP), Voxel::new(c.bush[0], sprout, 0));
    set(&mut app, CROP, Voxel::new(c.bush[2], ripe, 0));
    send(&mut app, remove_crop());
    tick(&mut app);
    assert_eq!(voxel_at(&app, CROP), Voxel::new(c.bush[1], young, 0));
    assert_eq!(count(&mut app, "t:berry"), 2);
}

#[test]
fn undo_and_redo_of_a_seed_hold_after_its_stages_changed_shape() {
    let (mut app, c) = staged_farm();
    let [sprout, young, ..] = c.stage_shapes;
    survival(&mut app);
    inventory(&mut app, |i| i.select(1).unwrap());
    send(&mut app, plant_at(5, 5));
    tick(&mut app);
    set(&mut app, CROP, Voxel::new(c.bush[1], young, 0));
    replay(&mut app, Undo);
    assert!(rejections(&mut app).is_empty());
    assert!(voxel_at(&app, CROP).is_air());
    replay(&mut app, Redo);
    assert!(rejections(&mut app).is_empty());
    assert_eq!(voxel_at(&app, CROP), Voxel::new(c.bush[0], sprout, 0));
}
