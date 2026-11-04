//! Dropped items: the Server's entity simulation.
//!
//! A deliberate Remove of a Voxel whose Material an Item names spawns a drop,
//! which then falls and rests on the logical step. The motion is
//! `tesserae_core::entity`; this plugin only feeds it the World.

use std::collections::{BTreeMap, HashMap};

use bevy::prelude::*;
use tesserae_core::entity::{DropSpec, Entities, EntityId, WorldEntity};

use super::ContentTables;
use super::edit::{self, VoxelRemoved};
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

/// What a Player holds after Pickup, by Item name. The smallest sink that makes
/// Pickup observable, not an inventory: no slots, no limits, no way to put an
/// item down. Step 15 replaces it.
#[derive(Component, Default, Clone, PartialEq, Debug)]
pub struct Carried(pub BTreeMap<String, u32>);

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

/// The Player's whole Carried count, sent whenever it changes.
#[derive(Message, Clone, PartialEq, Debug)]
pub struct CarriedChanged(pub BTreeMap<String, u32>);

/// What the Client was last told, so each tick sends only the difference.
#[derive(Resource, Default)]
struct Announced(HashMap<EntityId, EntityView>);

pub(super) fn plugin(app: &mut App) {
    app.init_session_resource::<EntityStore>()
        .init_session_resource::<Items>()
        .init_session_resource::<Announced>()
        .add_session_message::<EntityDelta>()
        .add_session_message::<CarriedChanged>()
        .add_session_systems(Update, restore_carried)
        .add_session_systems(
            FixedPreUpdate,
            (
                (spawn_drops, step_entities, pick_up).run_if(super::persistence::playing),
                announce_entities,
                announce_carried,
            )
                .chain()
                .after(edit::apply_pending),
        );
}

/// A reopened World's Carried count, once its Player exists. Checks through
/// the shared reference first so the resource is not marked changed every frame.
fn restore_carried(
    mut persistence: Option<ResMut<super::persistence::Persistence>>,
    mut carried: Single<&mut Carried>,
) {
    if !persistence
        .as_deref()
        .is_some_and(|p| p.has_carried_to_restore())
    {
        return;
    }
    if let Some(held) = persistence.as_mut().and_then(|p| p.take_carried()) {
        carried.0 = held;
    }
}

fn spawn_drops(
    mut removed: MessageReader<VoxelRemoved>,
    mut entities: ResMut<EntityStore>,
    (store, tables, world, items): (
        Res<ChunkStore>,
        Res<ContentTables>,
        Res<WorldSettings>,
        Res<Items>,
    ),
) {
    for gone in removed.read() {
        let Some(item) = items.dropped_by(gone.material) else {
            continue;
        };
        entities.spawn_drop(
            &*store,
            &tables.0,
            world.voxel_size,
            gone.at,
            DropSpec {
                item: item as u32,
                count: 1,
                size_m: items.0[item].size_m,
            },
        );
    }
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

fn pick_up(
    mut entities: ResMut<EntityStore>,
    items: Res<Items>,
    mut player: Single<(&Player, &mut Carried)>,
) {
    if entities.is_empty() {
        return;
    }
    let (player, carried) = &mut *player;
    let body = player.body();
    let taken: Vec<_> = entities
        .iter()
        .filter(|e| {
            let d = e.as_dropped();
            d.pickup_in == 0 && distance_to_box(d.pos_m, &body) <= PICKUP_RADIUS_M
        })
        .map(|e| e.id)
        .collect();
    for id in taken {
        let Some(gone) = entities.remove(id) else {
            continue;
        };
        let d = gone.as_dropped();
        let Some(item) = items.0.get(d.item as usize) else {
            continue;
        };
        let held = carried.0.entry(item.name.clone()).or_insert(0);
        *held = held.saturating_add(d.count);
    }
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

fn announce_carried(carried: Single<Ref<Carried>>, mut changed: MessageWriter<CarriedChanged>) {
    if carried.is_changed() {
        changed.write(CarriedChanged(carried.0.clone()));
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;
    use tesserae_mods::ItemDef;

    use super::super::edit::tests::{app_with, down, ground, send, stone, tick, voxel_at};
    use super::super::edit::{EditAction, apply_intent};
    use super::*;

    fn dropping_stone() -> Items {
        Items(vec![ItemDef {
            name: "t:rock".into(),
            size_m: 0.25,
            color: [1, 2, 3, 255],
            material: Some(stone().material()),
        }])
    }

    fn game() -> App {
        let mut app = app_with(&[(Default::default(), ground())]);
        app.insert_resource(dropping_stone());
        app.init_resource::<EntityStore>();
        app
    }

    fn remove_the_ground(app: &mut App) {
        send(app, down(EditAction::Remove));
        tick(app);
        app.world_mut().run_system_once(spawn_drops).unwrap();
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
        world.run_system_once(spawn_drops).unwrap();
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

    fn carried(app: &mut App) -> BTreeMap<String, u32> {
        let mut q = app.world_mut().query::<&Carried>();
        q.single(app.world()).unwrap().0.clone()
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

    fn game_with_messages() -> App {
        let mut app = game();
        app.init_resource::<Announced>()
            .add_message::<EntityDelta>()
            .add_message::<CarriedChanged>();
        app
    }

    #[test]
    fn a_drop_near_the_player_is_picked_up_into_carried_after_its_delay() {
        let mut app = game();
        remove_the_ground(&mut app);
        near_the_drop(&mut app);
        app.world_mut().run_system_once(pick_up).unwrap();
        assert!(carried(&mut app).is_empty(), "before the delay");
        assert_eq!(drops(&app), 1);
        after_the_pickup_delay(&mut app);
        app.world_mut().run_system_once(pick_up).unwrap();
        assert_eq!(
            carried(&mut app),
            BTreeMap::from([("t:rock".to_owned(), 1)])
        );
        assert_eq!(drops(&app), 0);
    }

    #[test]
    fn a_drop_beyond_the_radius_stays() {
        let mut app = game();
        remove_the_ground(&mut app);
        after_the_pickup_delay(&mut app);
        super::super::edit::tests::set_eye(&mut app, Vec3::new(40.5, 12.0, 5.5));
        app.world_mut().run_system_once(pick_up).unwrap();
        assert!(carried(&mut app).is_empty());
        assert_eq!(drops(&app), 1);
    }

    #[test]
    fn a_whole_stack_is_taken_and_counts_saturate() {
        let mut app = game();
        remove_the_ground(&mut app);
        after_the_pickup_delay(&mut app);
        near_the_drop(&mut app);
        let mut q = app.world_mut().query::<&mut Carried>();
        q.single_mut(app.world_mut())
            .unwrap()
            .0
            .insert("t:rock".into(), u32::MAX);
        app.world_mut().run_system_once(pick_up).unwrap();
        assert_eq!(carried(&mut app)["t:rock"], u32::MAX);
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
    fn carried_is_sent_when_it_changes_and_not_otherwise() {
        let mut app = game_with_messages();
        // A schedule keeps the system's last-run tick; `run_system_once` builds
        // a fresh one every call, which would see everything as changed.
        let mut schedule = Schedule::default();
        schedule.add_systems(announce_carried);
        let mut sent = |app: &mut App| {
            schedule.run(app.world_mut());
            app.world_mut()
                .resource_mut::<Messages<CarriedChanged>>()
                .drain()
                .count()
        };
        assert_eq!(sent(&mut app), 1, "the first tick tells the Client");
        assert_eq!(sent(&mut app), 0);
        let mut q = app.world_mut().query::<&mut Carried>();
        q.single_mut(app.world_mut())
            .unwrap()
            .0
            .insert("t:rock".into(), 1);
        assert_eq!(sent(&mut app), 1);
        assert_eq!(sent(&mut app), 0);
    }
}
