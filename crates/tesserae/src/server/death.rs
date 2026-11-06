//! A Player's death: what killed them, where their feet were and what they
//! dropped, kept by the Server until they respawn; the Client is told
//! (`docs/adr/0002-client-server-from-day-one.md`).

use bevy::ecs::message::Messages;
use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use tesserae_core::entity::{Entities, EntityId, STEP_S};
use tesserae_core::inventory::Stack;
use tesserae_core::save::{DeathCause, DeathV1};

use super::ContentTables;
use super::edit::PendingEdits;
use super::entities::EntityStore;
use super::inventory::{Inventory, drop_around};
use super::items::Items;
use super::persistence::extensions::PlayerState;
use super::player::{Player, PlayerInput, Respawned};
use super::survival::Survival;
use super::undo::Journal;
use super::vitals::{DamageCause, LastDamage, Vitals};
use super::world_state::{ChunkStore, SpawnPoint};
use crate::session::SessionApp;
use crate::world::WorldSettings;

#[derive(Component, Clone, PartialEq, Debug)]
pub struct Death {
    pub cause: Option<DeathCause>,
    /// Where the feet were.
    pub at_m: Vec3,
    /// Dropped where they fell, as Dropped items; empty when the Game keeps
    /// the Inventory.
    pub dropped: Vec<Stack>,
    /// The Dropped items `dropped` became, so their despawn can be told.
    pub drops: Vec<EntityId>,
    pub kept_inventory: bool,
}

/// What the death screen shows.
#[derive(Clone, PartialEq, Debug)]
pub struct DeathReport {
    pub death: Death,
    pub from_spawn_m: f32,
    /// The Game's safe fall height, which a fall's cause names.
    pub safe_fall_m: f32,
    /// Whole seconds, rounded up, until the first of the death's Dropped items
    /// still lying there despawns; `None` when none are.
    pub despawns_in_s: Option<u32>,
}

/// The time left on the death's drops, from their own despawn timers: they
/// age only while their Chunk is held, so no deadline taken at death holds.
pub(crate) fn despawns_in_s(death: &Death, entities: &Entities) -> Option<u32> {
    let steps = death
        .drops
        .iter()
        .filter_map(|&id| entities.get(id))
        .map(|e| e.as_dropped().steps_left)
        .min()?;
    Some((steps as f32 * STEP_S).ceil() as u32)
}

/// The Player's death, sent when it happens or a reopened World holds one, and
/// again each time its despawn countdown passes a whole second; `None` once
/// they respawn.
#[derive(Message, Clone, PartialEq, Debug)]
pub struct DeathChanged(pub Option<DeathReport>);

/// How a Player's intent is registered, so it is ignored while they are dead.
pub(super) trait PlayerIntents {
    fn add_player_intent<M: Message>(&mut self) -> &mut Self;
}

impl PlayerIntents for App {
    fn add_player_intent<M: Message>(&mut self) -> &mut Self {
        self.add_session_message::<M>().add_session_systems(
            RunFixedMainLoop,
            discard::<M>
                .run_if(dead)
                .before(PlayerInput)
                .in_set(RunFixedMainLoopSystems::BeforeFixedMainLoop),
        )
    }
}

pub(crate) fn plugin(app: &mut App) {
    app.add_session_message::<DeathChanged>()
        .add_session_systems(
            RunFixedMainLoop,
            revive
                .after(PlayerInput)
                .in_set(RunFixedMainLoopSystems::BeforeFixedMainLoop),
        )
        .add_session_systems(
            FixedUpdate,
            (die.run_if(resource_exists::<Survival>), announce)
                .chain()
                .after(super::vitals::tick),
        );
}

pub(super) fn dead(players: Query<(), (With<Player>, With<Death>)>) -> bool {
    !players.is_empty()
}

pub(super) fn alive(players: Query<(), (With<Player>, With<Death>)>) -> bool {
    players.is_empty()
}

/// Cleared from the buffer rather than left unread, so an intent sent while
/// dead cannot be read once the Player respawns.
fn discard<M: Message>(messages: Option<ResMut<Messages<M>>>) {
    if let Some(mut messages) = messages {
        messages.clear();
    }
}

type Dying<'a> = (
    Entity,
    &'a mut Player,
    &'a Vitals,
    &'a LastDamage,
    &'a mut Inventory,
);

fn die(
    mut commands: Commands,
    survival: Res<Survival>,
    player: Single<Dying, Without<Death>>,
    mut entities: ResMut<EntityStore>,
    (store, tables, world, items): (
        Res<ChunkStore>,
        Res<ContentTables>,
        Res<WorldSettings>,
        Res<Items>,
    ),
    (mut journal, mut pending): (ResMut<Journal>, ResMut<PendingEdits>),
) {
    let (id, mut player, vitals, last, mut inventory) = player.into_inner();
    if vitals.health > 0.0 {
        return;
    }
    let body = player.body();
    let kept_inventory = survival.rules.keep_inventory;
    let held = if kept_inventory {
        Vec::new()
    } else {
        inventory.take_all()
    };
    let (dropped, drops) = drop_around(
        &mut entities,
        &tables.terrain(&store, world.voxel_size),
        &items,
        &body,
        held,
    )
    .into_iter()
    .unzip();
    journal.clear();
    pending.clear();
    player.stop();
    commands.entity(id).insert(Death {
        cause: last.0.map(|cause| saved_cause(cause, &survival)),
        at_m: Vec3::from_array(body.feet()),
        dropped,
        drops,
        kept_inventory,
    });
}

fn saved_cause(cause: DamageCause, survival: &Survival) -> DeathCause {
    match cause {
        DamageCause::Fall { height_m } => DeathCause::Fall { height_m },
        DamageCause::Drowning => DeathCause::Drowning,
        DamageCause::Contact(material) => DeathCause::Contact {
            material: survival
                .material_name(material)
                .unwrap_or_default()
                .to_owned(),
        },
        DamageCause::Starvation => DeathCause::Starvation,
    }
}

/// Reads every respawn, so one made while alive is not taken for a later
/// death's.
fn revive(
    mut respawned: MessageReader<Respawned>,
    mut commands: Commands,
    survival: Option<Res<Survival>>,
    mut players: Query<(Entity, &mut Vitals, &mut LastDamage), With<Death>>,
) {
    if respawned.read().count() == 0 {
        return;
    }
    let rules = survival.map(|s| s.rules).unwrap_or_default();
    for (id, mut vitals, mut last) in &mut players {
        *vitals = Vitals::full(&rules);
        *last = LastDamage::default();
        commands.entity(id).remove::<Death>();
    }
}

/// What the Client was last told: nothing, or a death with its countdown.
#[derive(Default, PartialEq)]
enum Told {
    #[default]
    Alive,
    Dead(Option<u32>),
}

fn announce(
    player: Single<Option<Ref<Death>>, With<Player>>,
    spawn: Res<SpawnPoint>,
    entities: Res<EntityStore>,
    survival: Option<Res<Survival>>,
    mut told: Local<Told>,
    mut changed: MessageWriter<DeathChanged>,
) {
    match player.into_inner() {
        Some(death) => {
            let left = despawns_in_s(&death, &entities.0);
            if death.is_changed() || *told != Told::Dead(left) {
                *told = Told::Dead(left);
                changed.write(DeathChanged(Some(DeathReport {
                    from_spawn_m: death.at_m.distance(spawn.metres()),
                    safe_fall_m: survival.as_ref().map(|s| s.rules).unwrap_or_default().safe_fall_m,
                    death: Death::clone(&death),
                    despawns_in_s: left,
                })));
            }
        }
        None if *told != Told::Alive => {
            *told = Told::Alive;
            changed.write(DeathChanged(None));
        }
        None => {}
    }
}

impl PlayerState for DeathV1 {
    fn capture(player: EntityRef) -> Option<Self> {
        player.get::<Death>().map(|death| DeathV1 {
            cause: death.cause.clone(),
            at_m: death.at_m.to_array(),
            dropped: death.dropped.clone(),
            drops: death.drops.iter().map(|id| id.0).collect(),
            kept_inventory: death.kept_inventory,
        })
    }

    fn restore(self, player: &mut EntityCommands) {
        player.insert(Death {
            cause: self.cause,
            at_m: Vec3::from_array(self.at_m),
            dropped: self.dropped,
            drops: self.drops.into_iter().map(EntityId).collect(),
            kept_inventory: self.kept_inventory,
        });
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use bevy::time::TimeUpdateStrategy;
    use tesserae_core::collide::VoxelVolume;
    use tesserae_core::{Chunk, SurvivalRules, Voxel};

    use super::*;
    use crate::server::edit::{EditAction, EditIntent, EditRejected};
    use crate::server::entities::{EntityDelta, EntityView};
    use crate::server::game_mode::{GameMode, PlayerMode};
    use crate::server::inventory::InventoryIntent;
    use crate::server::player::MovementIntent;
    use crate::server::player::{EYE_HEIGHT, RespawnIntent};
    use crate::server::streaming::MeshState;
    use crate::server::undo::HistoryAction;
    use crate::server::vitals::EatIntent;
    use crate::world::ChunkCoord;

    const SPAWN: Vec3 = Vec3::new(20.5, 8.0, 20.5);
    /// On the ground's surface, 8 m up.
    const FEET: Vec3 = Vec3::new(5.5, 8.0, 5.5);

    /// What the Server told the Client, kept across updates.
    #[derive(Resource, Default)]
    struct Told {
        deaths: Vec<DeathChanged>,
        spawned: Vec<EntityView>,
        rejected: usize,
    }

    fn listen(
        mut told: ResMut<Told>,
        mut deaths: ResMut<Messages<DeathChanged>>,
        mut deltas: ResMut<Messages<EntityDelta>>,
        mut rejected: ResMut<Messages<EditRejected>>,
    ) {
        told.deaths.extend(deaths.drain());
        told.spawned.extend(deltas.drain().filter_map(|d| match d {
            EntityDelta::Spawned(view) => Some(view),
            _ => None,
        }));
        told.rejected += rejected.drain().count();
    }

    fn item(name: &str, food: Option<f32>) -> tesserae_mods::ItemDef {
        tesserae_mods::ItemDef {
            name: name.into(),
            size_m: 0.25,
            color: [0; 4],
            material: None,
            max_stack: 64,
            food,
        }
    }

    /// A Survival Player standing on stone, holding 70 rock and 3 bread, with
    /// the World spawn 15 m away.
    fn app(keep_inventory: bool) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_nanos(
                15_625_000,
            )))
            .insert_resource(WorldSettings::default())
            .insert_resource(SpawnPoint::for_tests(SPAWN))
            .init_resource::<ChunkStore>()
            .init_resource::<MeshState>()
            .insert_resource(ContentTables(Arc::new(crate::server::fixture_tables())))
            .insert_resource(Survival::new(
                SurvivalRules {
                    keep_inventory,
                    fall_damage_per_m: 2.0,
                    hunger_per_min: 0.0,
                    sprinting_hunger_per_min: 0.0,
                    ..default()
                },
                Vec::new(),
            ))
            .insert_resource(Items::with_implicit(
                vec![item("t:rock", None), item("t:bread", Some(6.0))],
                [(
                    tesserae_core::testgen::fixtures().water,
                    "t:water",
                    [0, 0, 255, 128],
                )],
                1.0,
            ))
            .init_resource::<Told>()
            .add_plugins((
                super::super::player::plugin,
                super::super::vitals::plugin,
                super::super::entities::plugin,
                super::super::inventory::plugin,
                super::super::edit::plugin,
                plugin,
            ))
            .add_systems(Last, listen);
        app.update();
        let stone = Voxel::cube(tesserae_core::testgen::fixtures().ground.rock);
        let mut ground = Chunk::air();
        for y in 0..8 {
            for z in 0..32 {
                for x in 0..32 {
                    ground.set(x, y, z, stone);
                }
            }
        }
        app.world_mut()
            .resource_mut::<ChunkStore>()
            .insert(ChunkCoord::new(0, 0, 0), ground);
        let (mut player, mut mode, mut inventory) = app
            .world_mut()
            .query::<(&mut Player, &mut PlayerMode, &mut Inventory)>()
            .single_mut(app.world_mut())
            .unwrap();
        *player = Player::physics_at(FEET + Vec3::Y * EYE_HEIGHT);
        mode.0 = GameMode::Survival;
        inventory.receive("t:rock", 70, 64);
        inventory.receive("t:bread", 3, 64);
        app
    }

    fn seconds(app: &mut App, s: f32) {
        for _ in 0..(s * 64.0).round() as usize {
            app.update();
        }
    }

    fn eye(app: &mut App) -> Vec3 {
        app.world_mut()
            .query::<&Player>()
            .single(app.world())
            .unwrap()
            .eye()
    }

    /// Lifts the Player 20 m over the ground, inside the one loaded Chunk, and
    /// lets them land: 17 m beyond the safe 3 m at 2 per metre outweighs 20
    /// health.
    fn fall_to_death(app: &mut App) -> DeathReport {
        let mut player = app.world_mut().query::<&mut Player>();
        let mut player = player.single_mut(app.world_mut()).unwrap();
        let eye = player.eye();
        player.set_eye(eye + Vec3::Y * 20.0);
        seconds(app, 3.0);
        let told = &app.world().resource::<Told>().deaths;
        assert!(told.iter().all(|d| d.0.is_some()), "{told:?}");
        told[0].0.clone().expect("died")
    }

    fn last_report(app: &App) -> DeathReport {
        let told = &app.world().resource::<Told>().deaths;
        told.last().and_then(|d| d.0.clone()).expect("dead")
    }

    fn stack(item: &str, count: u32) -> Stack {
        Stack {
            item: item.into(),
            count,
        }
    }

    fn inventory(app: &mut App) -> tesserae_core::inventory::Inventory {
        app.world_mut()
            .query::<&Inventory>()
            .single(app.world())
            .unwrap()
            .0
            .clone()
    }

    fn vitals(app: &mut App) -> Vitals {
        *app.world_mut()
            .query::<&Vitals>()
            .single(app.world())
            .unwrap()
    }

    #[test]
    fn death_drops_every_stack_as_dropped_items_where_the_player_fell() {
        let mut app = app(false);
        let report = fall_to_death(&mut app);
        let death = &report.death;
        assert!(
            matches!(death.cause, Some(DeathCause::Fall { height_m }) if height_m > 15.0),
            "{:?}",
            death.cause
        );
        assert!(death.at_m.distance(FEET) < 0.01, "{}", death.at_m);
        assert!((report.from_spawn_m - FEET.distance(SPAWN)).abs() < 0.01);
        assert_eq!(
            death.dropped,
            [stack("t:rock", 64), stack("t:rock", 6), stack("t:bread", 3)]
        );
        assert!(!death.kept_inventory);
        assert!(inventory(&mut app).is_empty());

        let spawned = &app.world().resource::<Told>().spawned;
        assert_eq!(
            spawned
                .iter()
                .map(|v| (v.item, v.count))
                .collect::<Vec<_>>(),
            [(0, 64), (0, 6), (1, 3)]
        );
        for view in spawned {
            let at = Vec3::from_array(view.pos_m);
            assert!(at.distance(FEET + Vec3::Y) < 1.5, "{at}");
        }
        seconds(&mut app, 2.0);
        assert!(
            inventory(&mut app).is_empty(),
            "a dead body picks nothing up"
        );
    }

    #[test]
    fn an_implicit_item_drops_on_death_and_the_death_reports_only_what_lies_there() {
        let mut app = app(false);
        let mut held = app.world_mut().query::<&mut Inventory>();
        let mut held = held.single_mut(app.world_mut()).unwrap();
        held.receive("t:water", 5, 64);
        held.receive("t:gone", 2, 64);
        let report = fall_to_death(&mut app);
        assert_eq!(
            report.death.dropped,
            [
                stack("t:rock", 64),
                stack("t:rock", 6),
                stack("t:bread", 3),
                stack("t:water", 5)
            ],
            "an Item no content names has nothing to drop as"
        );
        let spawned = &app.world().resource::<Told>().spawned;
        assert_eq!(
            spawned
                .iter()
                .map(|v| (v.item, v.count))
                .collect::<Vec<_>>(),
            [(0, 64), (0, 6), (1, 3), (2, 5)]
        );
        assert_eq!(report.death.drops.len(), report.death.dropped.len());
    }

    #[test]
    fn the_despawn_countdown_falls_as_the_server_ticks_and_ends_with_the_drops() {
        let mut app = app(false);
        let at_death = fall_to_death(&mut app);
        assert_eq!(at_death.despawns_in_s, Some(300));

        let before = last_report(&app);
        let told_before = app.world().resource::<Told>().deaths.len();
        seconds(&mut app, 10.0);
        let after = last_report(&app);
        assert_eq!(after.death, before.death);
        assert_eq!(
            before.despawns_in_s.unwrap() - after.despawns_in_s.unwrap(),
            10
        );
        let sent = app.world().resource::<Told>().deaths.len() - told_before;
        assert!(
            (9..=11).contains(&sent),
            "once a second, not a tick: {sent}"
        );

        seconds(&mut app, after.despawns_in_s.unwrap() as f32 + 1.0);
        assert!(app.world().resource::<EntityStore>().0.is_empty());
        let gone = last_report(&app);
        assert_eq!(gone.despawns_in_s, None, "the drops are gone");
        assert_eq!(gone.death.dropped, before.death.dropped);
    }

    #[test]
    fn a_game_that_keeps_the_inventory_drops_nothing_on_death() {
        let mut app = app(true);
        let before = inventory(&mut app);
        let report = fall_to_death(&mut app);
        assert!(report.death.kept_inventory);
        assert!(report.death.dropped.is_empty());
        assert_eq!(report.despawns_in_s, None);
        assert!(app.world().resource::<Told>().spawned.is_empty());
        assert_eq!(inventory(&mut app), before);
    }

    #[test]
    fn intents_sent_while_dead_are_ignored() {
        let mut app = app(true);
        fall_to_death(&mut app);
        let still = eye(&mut app);
        let below = [5, 7, 5];
        let stone = app.world().resource::<ChunkStore>().voxel(below);
        for _ in 0..32 {
            app.world_mut().write_message(MovementIntent {
                direction: Vec3::X,
                speed_mps: 4.0,
                jump: true,
                ..default()
            });
            app.world_mut().write_message(EditIntent {
                action: EditAction::Remove,
                origin: still,
                direction: Vec3::NEG_Y,
            });
            app.world_mut().write_message(InventoryIntent::Select(4));
            app.world_mut().write_message(EatIntent);
            app.update();
        }
        seconds(&mut app, 0.5);
        assert!(eye(&mut app).distance(still) < 1e-3, "{}", eye(&mut app));
        assert_eq!(
            app.world().resource::<ChunkStore>().voxel(below),
            stone,
            "no removal"
        );
        assert_eq!(app.world().resource::<Told>().rejected, 0, "nor a refusal");
        assert_eq!(inventory(&mut app).selected(), 0);
        assert_eq!(vitals(&mut app).health, 0.0);
    }

    #[derive(Message)]
    struct Poke;

    #[derive(Resource, Default)]
    struct Poked(usize);

    fn read_pokes(mut pokes: MessageReader<Poke>, mut poked: ResMut<Poked>) {
        poked.0 += pokes.read().count();
    }

    #[test]
    fn any_player_intent_is_ignored_while_dead_and_read_again_after() {
        let mut app = app(false);
        app.add_player_intent::<Poke>()
            .init_resource::<Poked>()
            .add_systems(FixedUpdate, read_pokes);
        fall_to_death(&mut app);
        for _ in 0..4 {
            app.world_mut().write_message(Poke);
            app.update();
        }
        app.world_mut().write_message(RespawnIntent);
        app.update();
        app.update();
        assert_eq!(app.world().resource::<Poked>().0, 0, "none read once alive");
        app.world_mut().write_message(Poke);
        app.update();
        assert_eq!(app.world().resource::<Poked>().0, 1);
    }

    #[test]
    fn respawn_restores_full_vitals_at_the_world_spawn_and_ends_the_death() {
        let mut app = app(false);
        fall_to_death(&mut app);
        app.world_mut().write_message(RespawnIntent);
        app.update();
        let told = &app.world().resource::<Told>().deaths;
        assert_eq!(told.last(), Some(&DeathChanged(None)));
        let rules = app.world().resource::<Survival>().rules;
        assert_eq!(vitals(&mut app), Vitals::full(&rules));
        let at = eye(&mut app);
        assert!((at.with_y(0.0) - SPAWN.with_y(0.0)).length() < 1e-3, "{at}");
        seconds(&mut app, 1.0);
        assert_eq!(vitals(&mut app), Vitals::full(&rules), "the fall is spent");

        let from = eye(&mut app);
        for _ in 0..32 {
            app.world_mut().write_message(MovementIntent {
                direction: Vec3::X,
                speed_mps: 4.0,
                ..default()
            });
            app.update();
        }
        assert!(eye(&mut app).x > from.x + 1.0, "moves again");
    }

    /// As a World reopened in Creative after a Survival death holds it.
    #[test]
    fn a_player_dead_in_creative_respawns_with_full_vitals() {
        let mut app = app(false);
        fall_to_death(&mut app);
        let mut mode = app.world_mut().query::<&mut PlayerMode>();
        mode.single_mut(app.world_mut()).unwrap().0 = GameMode::Creative;
        app.update();
        assert!(last_report(&app).death.cause.is_some(), "still dead");
        app.world_mut().write_message(RespawnIntent);
        app.update();
        let told = &app.world().resource::<Told>().deaths;
        assert_eq!(told.last(), Some(&DeathChanged(None)));
        let rules = app.world().resource::<Survival>().rules;
        assert_eq!(vitals(&mut app), Vitals::full(&rules));
        let at = eye(&mut app);
        assert!((at.with_y(0.0) - SPAWN.with_y(0.0)).length() < 1e-3, "{at}");
    }

    #[test]
    fn undo_after_a_respawn_has_nothing_from_before_the_death() {
        let mut app = app(false);
        let eye_at = eye(&mut app);
        let removed = [7, 7, 5];
        app.world_mut().write_message(EditIntent {
            action: EditAction::Remove,
            origin: eye_at,
            direction: (Vec3::new(7.5, 7.9, 5.5) - eye_at).normalize(),
        });
        seconds(&mut app, 0.25);
        let voxel = |app: &App| app.world().resource::<ChunkStore>().voxel(removed);
        assert!(voxel(&app).unwrap().is_air(), "removed");

        fall_to_death(&mut app);
        app.world_mut().write_message(RespawnIntent);
        app.update();
        app.world_mut().write_message(HistoryAction::Undo);
        seconds(&mut app, 0.25);
        assert!(voxel(&app).unwrap().is_air(), "nothing left to undo");
    }
}
