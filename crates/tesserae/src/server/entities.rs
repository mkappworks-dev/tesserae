//! Dropped items: the Server's entity simulation.
//!
//! A deliberate Remove of a Voxel whose Material an Item names spawns a drop,
//! which then falls and rests on the logical step. The motion is
//! `tesserae_core::entity`; this plugin only feeds it the World.

use std::collections::HashMap;

use bevy::prelude::*;
use tesserae_core::entity::{DropSpec, Entities, EntityId, WorldEntity};

use super::ContentTables;
use super::edit;
use super::inventory::Inventory;
use super::items::Items;
use super::player::Player;
use super::world_state::ChunkStore;
use crate::session::SessionApp;
use crate::world::WorldSettings;

/// Metres from a drop's centre to the Player's body box within which it is
/// picked up.
pub const PICKUP_RADIUS_M: f32 = 1.5;

#[derive(Resource, Default, Deref, DerefMut)]
pub struct EntityStore(pub Entities);

/// What the Client is told about one entity.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct EntityView {
    pub id: EntityId,
    pub item: u32,
    pub count: u32,
    pub size_m: f32,
    pub pos_m: [f32; 3],
    pub resting: bool,
}

impl From<&WorldEntity> for EntityView {
    fn from(e: &WorldEntity) -> Self {
        let d = e.as_dropped();
        EntityView {
            id: e.id,
            item: d.item,
            count: d.count,
            size_m: d.size_m,
            pos_m: d.pos_m,
            resting: d.resting,
        }
    }
}

/// A change to what the Client draws. Plain data, the shape step 22 designs
/// transport against.
#[derive(Message, Clone, Copy, PartialEq, Debug)]
pub enum EntityDelta {
    Spawned(EntityView),
    Moved(EntityView),
    Removed(EntityId),
}

/// One drop's Pickup: what the Inventory took and what stayed on the ground.
#[derive(Message, Clone, PartialEq, Debug)]
pub struct Pickup {
    pub item: String,
    pub taken: u32,
    pub left: u32,
}

/// Drops the Client was told the Inventory had no room for, with the count
/// left then. Cleared when the Inventory changes, so standing by a drop tells
/// once and making room tells again.
#[derive(Resource, Default)]
pub(super) struct Refused(HashMap<EntityId, u32>);

/// What the Client was last told, so each tick sends only the difference.
#[derive(Resource, Default)]
struct Announced(HashMap<EntityId, EntityView>);

pub(super) fn plugin(app: &mut App) {
    app.init_session_resource::<EntityStore>()
        .init_session_resource::<Items>()
        .init_session_resource::<Announced>()
        .init_session_resource::<Refused>()
        .add_session_message::<EntityDelta>()
        .add_session_message::<Pickup>()
        .add_session_systems(
            FixedPreUpdate,
            (
                (step_entities, pick_up.run_if(super::death::alive))
                    .run_if(super::persistence::playing),
                announce_entities,
            )
                .chain()
                .after(edit::apply_pending),
        );
}

/// Spawns one of the Item at `item` popping out of the Voxel `at`.
pub(super) fn drop_one(
    entities: &mut Entities,
    store: &ChunkStore,
    tables: &tesserae_core::Tables,
    voxel_size: f32,
    items: &Items,
    at: [i32; 3],
    item: usize,
) -> EntityId {
    entities.spawn_drop(
        store,
        tables,
        voxel_size,
        at,
        DropSpec {
            item: item as u32,
            count: 1,
            size_m: items.0[item].size_m,
        },
    )
}

fn step_entities(
    mut entities: ResMut<EntityStore>,
    (store, tables, world): (Res<ChunkStore>, Res<ContentTables>, Res<WorldSettings>),
) {
    if entities.is_empty() {
        return;
    }
    entities.step(&*store, &tables.0, world.voxel_size);
}

fn distance_to_box(point: [f32; 3], body: &tesserae_core::collide::Aabb) -> f32 {
    (0..3)
        .map(|a| {
            (body.min[a] - point[a])
                .max(point[a] - body.max[a])
                .max(0.0)
                .powi(2)
        })
        .sum::<f32>()
        .sqrt()
}

pub(super) fn pick_up(
    mut entities: ResMut<EntityStore>,
    items: Res<Items>,
    mut player: Single<(&Player, &mut Inventory)>,
    mut refused: ResMut<Refused>,
    mut told: MessageWriter<Pickup>,
) {
    if entities.is_empty() {
        return;
    }
    let (player, inventory) = &mut *player;
    if inventory.is_changed() {
        refused.0.clear();
    }
    let body = player.body();
    let near: Vec<_> = entities
        .iter()
        .filter(|e| {
            let d = e.as_dropped();
            d.pickup_in == 0 && distance_to_box(d.pos_m, &body) <= PICKUP_RADIUS_M
        })
        .map(|e| (e.id, e.as_dropped().item, e.as_dropped().count))
        .collect();
    for (id, item, count) in near {
        let Some(item) = items.0.get(item as usize) else {
            continue;
        };
        // Bypassed so a Pickup with no room leaves the Inventory unchanged,
        // which is what keeps a refusal from being told every tick.
        let left = inventory
            .bypass_change_detection()
            .receive(&item.name, count, item.max_stack);
        let taken = count - left;
        if taken > 0 {
            inventory.set_changed();
            entities.take(id, taken);
            refused.0.clear();
        } else if refused.0.get(&id) == Some(&left) {
            continue;
        }
        if left > 0 {
            refused.0.insert(id, left);
        }
        told.write(Pickup {
            item: item.name.clone(),
            taken,
            left,
        });
    }
    refused.0.retain(|id, _| entities.get(*id).is_some());
}

fn announce_entities(
    entities: Res<EntityStore>,
    mut announced: ResMut<Announced>,
    mut deltas: MessageWriter<EntityDelta>,
) {
    if entities.is_empty() && announced.0.is_empty() {
        return;
    }
    for entity in entities.iter() {
        let view = EntityView::from(entity);
        match announced.0.insert(entity.id, view) {
            None => {
                deltas.write(EntityDelta::Spawned(view));
            }
            Some(before) if before != view => {
                deltas.write(EntityDelta::Moved(view));
            }
            Some(_) => {}
        }
    }
    announced.0.retain(|id, _| {
        let alive = entities.get(*id).is_some();
        if !alive {
            deltas.write(EntityDelta::Removed(*id));
        }
        alive
    });
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;
    use tesserae_core::inventory::Stack;

    use super::super::inventory::{InventoryChanged, announce_inventory};
    use tesserae_mods::ItemDef;

    use super::super::edit::tests::{
        app_with, down, ground, send, stone, survival, tick, voxel_at,
    };
    use super::super::edit::{EditAction, apply_intent};
    use super::*;

    fn dropping_stone() -> Items {
        Items(vec![ItemDef {
            name: "t:rock".into(),
            size_m: 0.25,
            color: [1, 2, 3, 255],
            material: Some(stone().material()),
            max_stack: 64,
            food: None,
        }])
    }

    /// In Survival, where a removal drops.
    fn game() -> App {
        let mut app = app_with(&[(Default::default(), ground())]);
        survival(&mut app);
        app.insert_resource(dropping_stone());
        app.init_resource::<EntityStore>()
            .init_resource::<Refused>()
            .add_message::<Pickup>();
        app
    }

    fn remove_the_ground(app: &mut App) {
        send(app, down(EditAction::Remove));
        tick(app);
    }

    fn drops(app: &App) -> usize {
        app.world().resource::<EntityStore>().len()
    }

    #[test]
    fn a_deliberate_remove_of_a_mapped_voxel_spawns_exactly_one_drop() {
        let mut app = game();
        remove_the_ground(&mut app);
        assert_eq!(drops(&app), 1);
        assert!(voxel_at(&app, [5, 7, 5]).is_air());
    }

    /// World generation stands a feature like any other Voxel, so removing one drops its Item.
    #[test]
    fn removing_a_generated_surface_feature_drops_its_item() {
        use tesserae_core::{
            Biome, BiomeName, BiomeTable, CHUNK_SIZE, SurfaceFeature, SurfaceFeatures, Worldgen,
            WorldgenParams,
        };
        let lantern = tesserae_core::testgen::fixtures().lantern;
        let field = Biome {
            name: BiomeName::new("t:field").unwrap(),
            temperature: 0.5,
            humidity: 0.5,
            ground: None,
            sky_fog: None,
            cave_scale: None,
            surface_features: SurfaceFeatures::new(&[SurfaceFeature {
                material: lantern,
                spacing_m: 4.0,
            }])
            .unwrap(),
        };
        let params = WorldgenParams {
            caves: None,
            smoothing: None,
            biomes: BiomeTable::new(&[field]).unwrap(),
            ..tesserae_core::testgen::world_params()
        };
        let generator = Worldgen::new(7, 1.0, params);
        let size = CHUNK_SIZE as i32;
        let (coord, chunk, at) = (0..4)
            .find_map(|cy| {
                let chunk = generator.chunk(0, cy, 0);
                let local = (0..size - 4).find_map(|y| {
                    (2..size - 2).find_map(|z| {
                        (2..size - 2)
                            .find(|&x| {
                                chunk.get(x as usize, y as usize, z as usize).material() == lantern
                            })
                            .map(|x| [x, y, z])
                    })
                })?;
                Some((crate::world::ChunkCoord::new(0, cy, 0), chunk, local))
            })
            .expect("a feature near the origin");
        let at = [at[0], coord.y * size + at[1], at[2]];
        let mut app = app_with(&[(coord, chunk)]);
        survival(&mut app);
        app.insert_resource(Items(vec![ItemDef {
            name: "t:lamp".into(),
            size_m: 0.25,
            color: [1, 2, 3, 255],
            material: Some(lantern),
            max_stack: 64,
            food: None,
        }]));
        app.init_resource::<EntityStore>()
            .init_resource::<Refused>()
            .add_message::<Pickup>();
        let above = Vec3::new(at[0] as f32 + 0.5, at[1] as f32 + 3.0, at[2] as f32 + 0.5);
        send(
            &mut app,
            super::super::edit::EditIntent {
                action: EditAction::Remove,
                origin: above,
                direction: Vec3::NEG_Y,
            },
        );
        tick(&mut app);
        assert!(voxel_at(&app, at).is_air());
        let store = app.world().resource::<EntityStore>();
        assert_eq!(store.len(), 1);
        assert_eq!(store.iter().next().unwrap().as_dropped().item, 0);
    }

    #[test]
    fn a_creative_removal_drops_nothing() {
        let mut app = game();
        let mut mode = app
            .world_mut()
            .query::<&mut super::super::game_mode::PlayerMode>();
        mode.single_mut(app.world_mut()).unwrap().0 = super::super::game_mode::GameMode::Creative;
        remove_the_ground(&mut app);
        assert!(voxel_at(&app, [5, 7, 5]).is_air());
        assert_eq!(drops(&app), 0);
    }

    #[test]
    fn a_voxel_no_item_names_drops_nothing() {
        let mut app = game();
        app.insert_resource(Items::default());
        remove_the_ground(&mut app);
        assert_eq!(drops(&app), 0);
    }

    #[test]
    fn a_mods_command_removes_without_a_drop() {
        let mut app = game();
        let world = app.world_mut();
        world
            .run_system_once(
                |mut store: ResMut<ChunkStore>,
                 mut meshes: ResMut<super::super::streaming::MeshState>,
                 tables: Res<ContentTables>| {
                    apply_intent(
                        &mut store,
                        &mut meshes,
                        &tables,
                        [5, 7, 5],
                        tesserae_mods::Intent::Voxel(tesserae_core::Voxel::AIR),
                    )
                    .unwrap();
                },
            )
            .unwrap();
        assert_eq!(drops(&app), 0);
    }

    #[test]
    fn the_drop_falls_out_of_the_hole_and_rests_on_the_ground() {
        let mut app = game();
        remove_the_ground(&mut app);
        for _ in 0..400 {
            app.world_mut().run_system_once(step_entities).unwrap();
        }
        let store = app.world().resource::<EntityStore>();
        let drop = store.iter().next().unwrap().as_dropped();
        assert!(drop.resting, "{drop:?}");
        // The hole is the one Voxel at y = 7, so it rests on the 7 m floor of it.
        assert!((drop.pos_m[1] - (7.0 + 0.125)).abs() < 0.2, "{drop:?}");
    }

    fn inventory(app: &mut App) -> tesserae_core::inventory::Inventory {
        let mut q = app.world_mut().query::<&Inventory>();
        q.single(app.world()).unwrap().0.clone()
    }

    fn give(app: &mut App, item: &str, count: u32) {
        let mut q = app.world_mut().query::<&mut Inventory>();
        let left = q
            .single_mut(app.world_mut())
            .unwrap()
            .receive(item, count, 64);
        assert_eq!(left, 0);
    }

    fn pickups(app: &mut App) -> Vec<Pickup> {
        app.world_mut()
            .resource_mut::<Messages<Pickup>>()
            .drain()
            .collect()
    }

    fn deltas(app: &mut App) -> Vec<EntityDelta> {
        app.world_mut()
            .resource_mut::<Messages<EntityDelta>>()
            .drain()
            .collect()
    }

    fn after_the_pickup_delay(app: &mut App) {
        for _ in 0..40 {
            app.world_mut().run_system_once(step_entities).unwrap();
        }
    }

    fn near_the_drop(app: &mut App) {
        super::super::edit::tests::set_eye(app, Vec3::new(5.5, 9.0, 5.5));
    }

    /// A resting drop of `count` rocks where the Player can reach it.
    fn rocks_at_hand(app: &mut App, count: u32) -> EntityId {
        let id = app
            .world_mut()
            .resource_scope(|world, mut entities: Mut<EntityStore>| {
                let tables = world.resource::<ContentTables>();
                entities.spawn_drop(
                    world.resource::<ChunkStore>(),
                    &tables.0,
                    1.0,
                    [5, 8, 5],
                    DropSpec {
                        item: 0,
                        count,
                        size_m: 0.25,
                    },
                )
            });
        after_the_pickup_delay(app);
        near_the_drop(app);
        id
    }

    fn game_with_messages() -> App {
        let mut app = game();
        app.init_resource::<Announced>()
            .add_message::<EntityDelta>()
            .add_message::<InventoryChanged>();
        app
    }

    fn rock(count: u32) -> Option<Stack> {
        Some(Stack {
            item: "t:rock".into(),
            count,
        })
    }

    #[test]
    fn a_drop_near_the_player_is_picked_up_into_the_inventory_after_its_delay() {
        let mut app = game();
        remove_the_ground(&mut app);
        near_the_drop(&mut app);
        app.world_mut().run_system_once(pick_up).unwrap();
        assert!(inventory(&mut app).is_empty(), "before the delay");
        assert_eq!(drops(&app), 1);
        assert!(pickups(&mut app).is_empty());
        after_the_pickup_delay(&mut app);
        app.world_mut().run_system_once(pick_up).unwrap();
        assert_eq!(inventory(&mut app).hotbar()[0], rock(1));
        assert_eq!(drops(&app), 0);
        assert_eq!(
            pickups(&mut app),
            [Pickup {
                item: "t:rock".into(),
                taken: 1,
                left: 0
            }]
        );
    }

    #[test]
    fn pickup_merges_into_a_matching_stack_before_an_empty_slot() {
        let mut app = game();
        give(&mut app, "t:dust", 64 * 2);
        give(&mut app, "t:rock", 60);
        rocks_at_hand(&mut app, 10);
        app.world_mut().run_system_once(pick_up).unwrap();
        let held = inventory(&mut app);
        assert_eq!(held.hotbar()[2], rock(64));
        assert_eq!(held.hotbar()[3], rock(6));
        assert_eq!(drops(&app), 0);
    }

    #[test]
    fn a_declared_stack_limit_of_one_keeps_every_picked_up_item_separate() {
        let mut app = game();
        app.world_mut().resource_mut::<Items>().0[0].max_stack = 1;
        rocks_at_hand(&mut app, 3);
        app.world_mut().run_system_once(pick_up).unwrap();
        let held = inventory(&mut app);
        assert_eq!(held.hotbar()[..4], [rock(1), rock(1), rock(1), None]);
        assert_eq!(drops(&app), 0);
    }

    #[test]
    fn a_drop_beyond_the_radius_stays() {
        let mut app = game();
        remove_the_ground(&mut app);
        after_the_pickup_delay(&mut app);
        super::super::edit::tests::set_eye(&mut app, Vec3::new(40.5, 12.0, 5.5));
        app.world_mut().run_system_once(pick_up).unwrap();
        assert!(inventory(&mut app).is_empty());
        assert_eq!(drops(&app), 1);
    }

    #[test]
    fn pickup_into_a_full_inventory_takes_what_fits_and_leaves_a_smaller_drop() {
        let mut app = game();
        give(&mut app, "t:dust", 64 * 35);
        give(&mut app, "t:rock", 60);
        let id = rocks_at_hand(&mut app, 10);
        app.world_mut().run_system_once(pick_up).unwrap();
        assert_eq!(inventory(&mut app).backpack()[26], rock(64));
        let left = app.world().resource::<EntityStore>().get(id).unwrap();
        assert_eq!(left.as_dropped().count, 6);
        assert_eq!(
            pickups(&mut app),
            [Pickup {
                item: "t:rock".into(),
                taken: 4,
                left: 6
            }]
        );
    }

    #[test]
    fn a_refused_pickup_is_told_once_until_the_inventory_changes() {
        let mut app = game();
        give(&mut app, "t:dust", 64 * 36);
        let id = rocks_at_hand(&mut app, 3);
        // A schedule keeps the system's last-run tick; `run_system_once` builds
        // a fresh one every call, which would see the Inventory as changed.
        let mut schedule = Schedule::default();
        schedule.add_systems(pick_up);
        let refused = Pickup {
            item: "t:rock".into(),
            taken: 0,
            left: 3,
        };
        schedule.run(app.world_mut());
        assert_eq!(pickups(&mut app), std::slice::from_ref(&refused));
        assert!(app.world().resource::<EntityStore>().get(id).is_some());
        schedule.run(app.world_mut());
        assert!(pickups(&mut app).is_empty(), "standing by it says nothing");
        let mut q = app.world_mut().query::<&mut Inventory>();
        q.single_mut(app.world_mut()).unwrap().set_changed();
        schedule.run(app.world_mut());
        assert_eq!(pickups(&mut app), [refused]);
    }

    #[test]
    fn each_tick_sends_only_what_changed() {
        let mut app = game_with_messages();
        remove_the_ground(&mut app);
        app.world_mut().run_system_once(announce_entities).unwrap();
        let sent = deltas(&mut app);
        assert!(
            matches!(sent.as_slice(), [EntityDelta::Spawned(_)]),
            "{sent:?}"
        );
        app.world_mut().run_system_once(step_entities).unwrap();
        app.world_mut().run_system_once(announce_entities).unwrap();
        assert!(matches!(
            deltas(&mut app).as_slice(),
            [EntityDelta::Moved(_)]
        ));
        for _ in 0..400 {
            app.world_mut().run_system_once(step_entities).unwrap();
        }
        app.world_mut().run_system_once(announce_entities).unwrap();
        deltas(&mut app);
        app.world_mut().run_system_once(announce_entities).unwrap();
        assert!(deltas(&mut app).is_empty(), "a resting drop sends nothing");
        let id = app
            .world()
            .resource::<EntityStore>()
            .iter()
            .next()
            .unwrap()
            .id;
        app.world_mut().resource_mut::<EntityStore>().remove(id);
        app.world_mut().run_system_once(announce_entities).unwrap();
        assert_eq!(deltas(&mut app), vec![EntityDelta::Removed(id)]);
    }

    #[test]
    fn the_inventory_is_sent_when_it_changes_and_not_otherwise() {
        let mut app = game_with_messages();
        let mut schedule = Schedule::default();
        schedule.add_systems(announce_inventory);
        let mut sent = |app: &mut App| {
            schedule.run(app.world_mut());
            app.world_mut()
                .resource_mut::<Messages<InventoryChanged>>()
                .drain()
                .collect::<Vec<_>>()
        };
        assert_eq!(sent(&mut app).len(), 1, "the first tick tells the Client");
        assert!(sent(&mut app).is_empty());
        give(&mut app, "t:rock", 2);
        assert_eq!(sent(&mut app), [InventoryChanged(inventory(&mut app))]);
        assert!(sent(&mut app).is_empty());
    }
}
