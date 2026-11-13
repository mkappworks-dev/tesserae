//! Use on a Usable Voxel: what the aim offers before the press, and what the
//! press does. Each kind runs here in native code; no Mod is called
//! (`docs/adr/0026-usable-voxels-are-declared-kinds-the-server-runs.md`).

use bevy::prelude::*;
use tesserae_core::collide::{Aabb, VoxelVolume, overlaps, raycast};
use tesserae_core::{Occupants, Tables, Voxel, VoxelUse};
use tesserae_mods::{EditEvent, EditKind};

use super::edit::{self, Batch, PlacedOnly, RejectReason};
use super::mods::EditHistory;
use super::streaming::MeshState;
use super::tool::{Usable, UseAim, UseRefusal, UseResult, UseTarget};
use super::world_state::{ChunkStore, VoxelCoord};
use crate::world::WorldSettings;

/// How far a ray still names a Usable Voxel past Edit reach, in metres, so
/// the prompt can say it is out of reach before the press.
const SIGHT_M: f32 = 2.0 * edit::EDIT_REACH_M;

/// The most Voxels of one Door a Use toggles: a run taller than this is two
/// Doors to the Player and one to the Server, which toggles the four nearest
/// its foot.
const DOOR_RUN_MAX: i32 = 4;

/// What Use with no Tool would do with this ray: the Usable Voxel it strikes in
/// reach, `OutOfReach` for one past it, or `None` when nothing usable is
/// aimed at and Use eats.
pub(super) fn aim(
    store: &ChunkStore,
    tables: &Tables,
    world: &WorldSettings,
    ray: (Vec3, Vec3),
    eye: Vec3,
) -> Option<UseAim> {
    match edit::solid_hit(store, tables, world, ray, eye) {
        Ok((hit, struck)) => {
            let voxel = store.voxel(hit.at)?;
            Some(Ok(UseTarget {
                at: hit.at,
                voxel,
                corner: struck,
                usable: Some(usable(tables, voxel)?),
            }))
        }
        Err(RejectReason::OutOfReach) => {
            let (origin, direction) = ray;
            let hit = raycast(
                store,
                tables,
                world.voxel_size,
                origin.to_array(),
                direction.to_array(),
                SIGHT_M,
            )?;
            usable(tables, store.voxel(hit.at)?)?;
            Some(Err(UseRefusal::OutOfReach))
        }
        Err(_) => None,
    }
}

fn usable(tables: &Tables, voxel: Voxel) -> Option<Usable> {
    match tables.uses.of(voxel.material())? {
        (_, &VoxelUse::Door { open }) => Some(Usable::Door {
            open: voxel.material() == open,
        }),
    }
}

/// Toggles the Door at `at` with the vertical run of its Voxels it belongs to,
/// keeping each Voxel's Shape, Orientation, fluid and fire. Mods hear it as
/// the Player's Edits; the undo journal never records it (ADR-0023).
pub(super) fn toggle_door(
    (store, meshes): (&mut ChunkStore, &mut MeshState),
    tables: &Tables,
    world: &WorldSettings,
    history: &mut EditHistory,
    body: Option<Aabb>,
    at: [i32; 3],
) -> Result<UseResult, UseRefusal> {
    let aimed = store.voxel(at).ok_or(UseRefusal::NotLoaded)?;
    let door = tables
        .uses
        .door(aimed.material())
        .ok_or(UseRefusal::TargetChanged)?;
    let of_door = |p: [i32; 3]| {
        store
            .voxel(p)
            .filter(|v| tables.uses.door(v.material()) == Some(door))
    };
    let mut foot = at;
    while at[1] - foot[1] < DOOR_RUN_MAX - 1 && of_door([foot[0], foot[1] - 1, foot[2]]).is_some() {
        foot[1] -= 1;
    }
    let to = door.toggled(aimed.material());
    let run: Vec<([i32; 3], Occupants)> = (0..DOOR_RUN_MAX)
        .map(|dy| [foot[0], foot[1] + dy, foot[2]])
        .map_while(|p| {
            let v = of_door(p)?;
            let here = store.occupants(p, &tables.fluids)?;
            let solid = Voxel::new(to, v.shape(), v.orientation());
            Some((p, Occupants { solid, ..here }))
        })
        .collect();
    if run
        .iter()
        .any(|&(p, _)| meshes.is_unlit(VoxelCoord::of(p).chunk))
    {
        return Err(UseRefusal::Busy);
    }
    if !door.is_open(to)
        && let Some(body) = body
        && run.iter().any(|&(p, next)| {
            let placed = PlacedOnly {
                at: p,
                voxel: next.solid,
            };
            overlaps(&placed, tables, world.voxel_size, body)
        })
    {
        return Err(UseRefusal::InsidePlayer);
    }
    // Vetted whole first, so a refusal leaves none of the run changed.
    for &(_, next) in &run {
        tables
            .vet_occupants(next)
            .map_err(|fault| UseRefusal::from(RejectReason::from(fault)))?;
    }
    let mut batch = Batch::default();
    for &(p, next) in &run {
        let before = store.voxel(p).expect("the run is held");
        let was = store.occupants(p, &tables.fluids).expect("the run is held");
        let written = batch.set_occupants(store, meshes, tables, p, next);
        written.expect("vetted, and the same Shape takes the same room");
        history.record(EditEvent {
            at: p.map(|i| i as f32 * world.voxel_size),
            kind: EditKind::Solid {
                before: was.solid,
                after: next.solid,
            },
            before,
            after: store.voxel(p).expect("the run is held"),
        });
    }
    batch.finish(store, meshes, tables);
    Ok(UseResult::Door {
        opened: door.is_open(to),
    })
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;
    use tesserae_core::{MaterialDef, MaterialId, Orientation, SHAPE_STAIR, VoxelUse};

    use super::super::edit::tests::{
        app_with_tables, ground, held, place_held, rejections, replay, send, tick, voxel_at,
    };
    use super::super::edit::{EditAction, EditIntent};
    use super::super::inventory::Inventory;
    use super::super::items::{Items, PlacingItems};
    use super::super::tool::{self, ToolId, UseIntent, UseOutcome, answer_use};
    use super::super::undo::HistoryAction::{Redo, Undo};
    use super::super::vitals::EatIntent;
    use super::*;
    use crate::world::ChunkCoord;
    use tesserae_core::SHAPE_FULL_CUBE;
    use tesserae_core::inventory::{Destination, Slot};

    const EYE: Vec3 = Vec3::new(5.5, 12.0, 5.5);

    struct Doors {
        closed: MaterialId,
        open: MaterialId,
    }

    fn doors(tables: &mut Tables) -> Doors {
        let [closed, open] =
            [(); 2].map(|_| tables.materials.register(MaterialDef::OPAQUE).unwrap());
        tables
            .register_use(closed, VoxelUse::Door { open })
            .unwrap();
        Doors { closed, open }
    }

    /// The ground with a Door of `height` Voxels standing at (5, 8, 5), and a
    /// Player looking down at it, holding `t:door` and `t:bread`.
    fn app(height: i32, open: bool) -> (App, Doors) {
        let mut tables = crate::server::fixture_tables();
        let doors = doors(&mut tables);
        let mut chunk = ground();
        let material = if open { doors.open } else { doors.closed };
        for y in 8..8 + height as usize {
            chunk.set(5, y, 5, Voxel::cube(material));
        }
        let mut app = app_with_tables(&[(ChunkCoord::new(0, 0, 0), chunk)], tables);
        let item = |name: &str, material, food| tesserae_mods::ItemDef {
            name: name.into(),
            size_m: 0.25,
            color: [0; 4],
            material,
            max_stack: 64,
            food,
        };
        let items = Items(vec![
            item("t:door", Some(doors.closed), None),
            item("t:bread", None, Some(4.0)),
        ]);
        app.insert_resource(PlacingItems::new([(doors.closed, "t:door", true)], &items))
            .insert_resource(items)
            .add_plugins(tool::plugin);
        let mut inventory = tesserae_core::inventory::Inventory::default();
        for (at, name) in [(0, "t:door"), (1, "t:bread")] {
            let slot = Destination::Slot(Slot::Hotbar(at));
            inventory.take(name, 3, slot, |_| 64).unwrap();
        }
        let mut held = app.world_mut().query::<&mut Inventory>();
        held.single_mut(app.world_mut()).unwrap().0 = inventory;
        (app, doors)
    }

    fn press(app: &mut App, eye: Vec3, direction: Vec3, tool: Option<ToolId>) -> Vec<UseOutcome> {
        super::super::edit::tests::set_eye(app, eye);
        app.world_mut().write_message(UseIntent {
            tool,
            eat: true,
            origin: eye,
            direction,
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

    fn use_down(app: &mut App) -> Vec<UseOutcome> {
        press(app, EYE, Vec3::NEG_Y, None)
    }

    fn eaten(app: &mut App) -> usize {
        app.world_mut()
            .resource_mut::<Messages<EatIntent>>()
            .drain()
            .count()
    }

    fn materials(app: &App, height: i32) -> Vec<MaterialId> {
        (8..8 + height)
            .map(|y| voxel_at(app, [5, y, 5]).material())
            .collect()
    }

    fn journal_is_empty(app: &App) -> bool {
        app.world()
            .resource::<super::super::undo::Journal>()
            .is_empty()
    }

    #[test]
    fn use_opens_and_closes_a_door_and_never_enters_the_journal() {
        let (mut app, d) = app(1, false);
        assert_eq!(
            use_down(&mut app),
            [UseOutcome(Ok(UseResult::Door { opened: true }))]
        );
        assert_eq!(materials(&app, 1), [d.open]);
        assert_eq!(
            use_down(&mut app),
            [UseOutcome(Ok(UseResult::Door { opened: false }))]
        );
        assert_eq!(materials(&app, 1), [d.closed]);
        assert!(journal_is_empty(&app));
        assert_eq!(eaten(&mut app), 0, "using the Door ate nothing");
    }

    #[test]
    fn a_door_of_two_opens_as_one_and_a_taller_run_toggles_four() {
        let (mut two, d) = app(2, false);
        use_down(&mut two);
        assert_eq!(materials(&two, 2), [d.open; 2]);
        let (mut tall, d) = app(5, false);
        use_down(&mut tall);
        assert_eq!(
            materials(&tall, 5),
            [d.closed, d.open, d.open, d.open, d.open],
            "the four nearest the aimed top"
        );
    }

    #[test]
    fn a_toggle_keeps_shape_and_orientation() {
        let (mut app, d) = app(0, false);
        let turned = Orientation::IDENTITY.turned();
        let stair = Voxel::shaped(d.closed, SHAPE_STAIR, turned);
        let tables = app
            .world()
            .resource::<super::super::ContentTables>()
            .0
            .clone();
        let mut meshes = MeshState::default();
        let mut store = app.world_mut().resource_mut::<ChunkStore>();
        edit::set_voxel(&mut store, &mut meshes, &tables, [5, 8, 5], stair).unwrap();
        use_down(&mut app);
        assert_eq!(
            voxel_at(&app, [5, 8, 5]),
            Voxel::shaped(d.open, SHAPE_STAIR, turned)
        );
    }

    #[test]
    fn closing_a_door_on_the_player_is_refused_as_place_is() {
        let (mut app, d) = app(2, true);
        // Feet on the ground beside the Door, the body's edge inside it.
        let eye = Vec3::new(4.9, 8.0 + crate::server::player::EYE_HEIGHT, 5.5);
        let mut players = app
            .world_mut()
            .query::<&mut crate::server::player::Player>();
        *players.single_mut(app.world_mut()).unwrap() =
            crate::server::player::Player::physics_at(eye);
        let refused = press(&mut app, eye, Vec3::X, None);
        assert_eq!(refused, [UseOutcome(Err(UseRefusal::InsidePlayer))]);
        assert_eq!(
            UseRefusal::InsidePlayer.message(),
            RejectReason::InsidePlayer.message()
        );
        assert_eq!(materials(&app, 2), [d.open; 2]);
    }

    #[test]
    fn a_door_out_of_reach_is_named_before_the_press_and_refused_after_it() {
        let (mut app, _) = app(1, false);
        let far = Vec3::new(5.5, 9.0 + 12.0, 5.5);
        let aimed = |app: &App, eye| {
            aim(
                app.world().resource::<ChunkStore>(),
                &app.world().resource::<super::super::ContentTables>().0,
                app.world().resource::<WorldSettings>(),
                (eye, Vec3::NEG_Y),
                eye,
            )
        };
        assert_eq!(aimed(&app, far), Some(Err(UseRefusal::OutOfReach)));
        let near = aimed(&app, EYE).unwrap().unwrap();
        assert_eq!(near.usable, Some(Usable::Door { open: false }));
        assert_eq!(near.usable.map(Usable::verb), Some("Open"));
        assert_eq!(near.at, [5, 8, 5]);
        assert_eq!(
            press(&mut app, far, Vec3::NEG_Y, None),
            [UseOutcome(Err(UseRefusal::OutOfReach))]
        );
        assert_eq!(eaten(&mut app), 0, "a Door in sight is not a meal");
    }

    #[test]
    fn use_runs_a_held_tool_first_then_a_usable_voxel_then_eats() {
        let (mut app, d) = app(1, false);
        app.insert_resource(tool::Tools(vec![tesserae_mods::ToolDef {
            name: "t:measure".into(),
            kind: tool::ToolKind::Measure,
        }]));
        let measured = press(&mut app, EYE, Vec3::NEG_Y, Some(ToolId(0)));
        assert!(matches!(
            measured[..],
            [UseOutcome(Ok(UseResult::Measured(_)))]
        ));
        assert_eq!(materials(&app, 1), [d.closed]);
        // Aimed at the ground beside the Door: nothing usable, so it eats.
        let ground = press(&mut app, Vec3::new(7.5, 12.0, 5.5), Vec3::NEG_Y, None);
        assert!(ground.is_empty(), "eating answers for itself");
        assert_eq!(eaten(&mut app), 1);
        assert!(press(&mut app, EYE, Vec3::Y, None).is_empty());
        assert_eq!(eaten(&mut app), 1, "aimed at the sky, it eats too");
    }

    #[test]
    fn sample_of_an_open_door_takes_the_closed_one() {
        let (app, d) = app(1, true);
        let picked = edit::pick_item(
            app.world().resource::<ChunkStore>(),
            &app.world().resource::<super::super::ContentTables>().0,
            app.world().resource::<WorldSettings>(),
            edit::PickIntent {
                seq: 0,
                origin: EYE,
                direction: Vec3::NEG_Y,
            },
            EYE,
        );
        assert_eq!(picked, Some(edit::AimItem::Solid(Voxel::cube(d.closed))));
    }

    #[test]
    fn breaking_an_open_door_drops_the_closed_ones_item() {
        let (mut app, _) = app(1, true);
        send(
            &mut app,
            EditIntent {
                action: EditAction::Remove,
                origin: EYE,
                direction: Vec3::NEG_Y,
            },
        );
        tick(&mut app);
        assert!(rejections(&mut app).is_empty());
        assert!(voxel_at(&app, [5, 8, 5]).is_air());
        let door_item = app.world().resource::<Items>().named("t:door").unwrap() as u32;
        let dropped: Vec<_> = app
            .world()
            .resource::<super::super::entities::EntityStore>()
            .iter()
            .map(|e| e.as_dropped().item)
            .collect();
        assert_eq!(dropped, [door_item]);
    }

    #[test]
    fn undo_and_redo_of_a_doors_placement_work_open_or_closed() {
        for opened_after in [false, true] {
            let (mut app, d) = app(0, false);
            send(&mut app, place_held(SHAPE_FULL_CUBE, Orientation::IDENTITY));
            tick(&mut app);
            assert!(rejections(&mut app).is_empty());
            assert_eq!(materials(&app, 1), [d.closed]);
            if opened_after {
                use_down(&mut app);
                assert_eq!(materials(&app, 1), [d.open]);
            }
            replay(&mut app, Undo);
            assert!(rejections(&mut app).is_empty(), "opened {opened_after}");
            assert!(voxel_at(&app, [5, 8, 5]).is_air());
            assert_eq!(held(&mut app, "t:door"), 3, "refunded");
            replay(&mut app, Redo);
            assert!(rejections(&mut app).is_empty());
            assert_eq!(materials(&app, 1), [d.closed]);
        }
    }

    #[test]
    fn undo_of_a_removal_puts_the_door_back_as_it_was() {
        let (mut app, d) = app(1, true);
        send(
            &mut app,
            EditIntent {
                action: EditAction::Remove,
                origin: EYE,
                direction: Vec3::NEG_Y,
            },
        );
        tick(&mut app);
        replay(&mut app, Undo);
        assert!(rejections(&mut app).is_empty());
        assert_eq!(materials(&app, 1), [d.open]);
    }
}
