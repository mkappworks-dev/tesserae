use tesserae_core::inventory::Stack;
use tesserae_core::{MaterialDef, MaterialId, VoxelUse};

use super::*;
use crate::server::undo::HistoryAction::{Redo, Undo};

#[test]
fn one_undo_takes_the_whole_structure_back_and_one_redo_restores_it() {
    let mut app = hut_app();
    send(&mut app, intent("t:hut"));
    settle(&mut app);
    let whole = built(&app);
    assert_eq!(whole, 6 * 6 + 4 * 5);
    replay(&mut app, Undo);
    assert!(rejections(&mut app).is_empty());
    assert_eq!(built(&app), 0);
    replay(&mut app, Redo);
    assert!(rejections(&mut app).is_empty());
    assert_eq!(built(&app), whole);
    assert_eq!(app.world().resource::<Journal>().groups(), 1);
}

/// A chest beside a stone, so a placement holds a Container.
fn chest_app() -> (App, MaterialId) {
    let mut tables = tables_with(Vec::new());
    let chest = tables.materials.register(MaterialDef::OPAQUE).unwrap();
    tables
        .register_use(chest, VoxelUse::Container { slots: 4 }, None)
        .unwrap();
    let row = vec![
        vec![vec![Cell::Keep, Cell::Keep]],
        vec![vec![Cell::Voxel(Voxel::cube(chest)), Cell::Voxel(stone())]],
    ];
    tables.templates = vec![Template::new("t:store".into(), 1.0, row, 0, None, 1.0).unwrap()];
    let mut app = app_with_tables(&[(origin(), ground())], tables);
    app.init_resource::<Containers>();
    (app, chest)
}

/// Where `chest_app`'s chest lands: one west of the anchor at (16, 7, 16), a layer up.
const CHEST: [i32; 3] = [15, 8, 16];

fn set_chest(app: &mut App, stack: Option<Stack>) {
    let voxel = voxel_at(app, CHEST);
    let mut containers = app.world_mut().resource_mut::<Containers>();
    containers.open(Entity::PLACEHOLDER, CHEST, voxel, 4);
    containers.viewing_mut(Entity::PLACEHOLDER).unwrap()[0] = stack;
    containers.close(Entity::PLACEHOLDER);
}

#[test]
fn undo_is_refused_while_a_container_the_structure_placed_holds_stacks() {
    let (mut app, chest) = chest_app();
    send(&mut app, intent("t:store"));
    settle(&mut app);
    assert!(rejections(&mut app).is_empty());
    assert_eq!(voxel_at(&app, CHEST), Voxel::cube(chest));
    set_chest(
        &mut app,
        Some(Stack {
            item: "t:rock".into(),
            count: 1,
        }),
    );
    replay(&mut app, Undo);
    assert_eq!(
        rejections(&mut app),
        [RejectReason::ContainerNotEmpty(chest)]
    );
    assert_eq!(voxel_at(&app, CHEST), Voxel::cube(chest));
    assert_eq!(voxel_at(&app, [16, 8, 16]), stone(), "nothing was undone");
    set_chest(&mut app, None);
    replay(&mut app, Undo);
    assert!(rejections(&mut app).is_empty(), "an empty one undoes");
    assert!(voxel_at(&app, CHEST).is_air());
    assert!(voxel_at(&app, [16, 8, 16]).is_air());
}

#[test]
fn a_placement_past_the_undo_budget_is_refused_before_any_cell_lands() {
    // 48 m a side at 0.25 m: the floor replaces stone with stone, and the roof and walls change
    // 159,488 cells, past the 131,072 the journal holds.
    let tables = tables_with(vec![hut("t:hall", 48)]);
    let chunks: Vec<_> = (-3..=3)
        .flat_map(|x| (-3..=3).map(move |z| (ChunkCoord::new(x, 0, z), ground())))
        .collect();
    let mut app = app_with_tables(&chunks, tables);
    app.insert_resource(WorldSettings {
        voxel_size: 0.25,
        ..default()
    });
    send(
        &mut app,
        StructureIntent {
            origin: Vec3::new(4.125, 3.0, 4.125),
            ..intent("t:hall")
        },
    );
    for _ in 0..1000 {
        if app.world().resource::<PendingEdits>().is_empty() {
            break;
        }
        tick(&mut app);
        assert_eq!(built(&app), 0, "nothing lands");
    }
    assert_eq!(rejections(&mut app), [RejectReason::TooBigToUndo]);
    assert!(app.world().resource::<Journal>().is_empty());
}
