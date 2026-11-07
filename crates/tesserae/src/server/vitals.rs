//! A Player's health, hunger and air, kept by the Server on the fixed tick in
//! metres and seconds; the Client is told
//! (`docs/adr/0002-client-server-from-day-one.md`).

use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use tesserae_core::collide::{Aabb, Terrain};
use tesserae_core::flow::CellView;
use tesserae_core::save::VitalsV1;
use tesserae_core::{Fluid, MaterialId, Occupants, SurvivalRules};

use super::ContentTables;
use super::death::{Death, PlayerIntents};
use super::game_mode::PlayerMode;
use super::inventory::Inventory;
use super::items::Items;
use super::persistence::extensions::PlayerState;
use super::player::Player;
use super::survival::Survival;
use super::tool::{UseOutcome, UseRefusal};
use super::world_state::ChunkStore;
use crate::session::SessionApp;
use crate::world::WorldSettings;

/// Health and hunger in points, air in seconds.
#[derive(Component, Clone, Copy, PartialEq, Debug)]
pub struct Vitals {
    pub health: f32,
    pub hunger: f32,
    pub air_s: f32,
}

impl Vitals {
    pub fn full(rules: &SurvivalRules) -> Self {
        Self {
            health: rules.max_health,
            hunger: rules.max_hunger,
            air_s: rules.max_air_s,
        }
    }
}

impl Default for Vitals {
    fn default() -> Self {
        Self::full(&SurvivalRules::default())
    }
}

/// Points as the HUD draws them, a half tile each: a part point shows as the
/// next whole one, so any loss shows.
pub fn drawn_points(points: f32) -> u32 {
    points.max(0.0).ceil() as u32
}

/// What the HUD draws of the Vitals, so a message is sent only when it moves.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Drawn {
    health: u32,
    hunger: u32,
    air: u32,
    air_full: bool,
    dead: bool,
}

impl Drawn {
    fn of(vitals: &Vitals, full: &Vitals, dead: bool) -> Self {
        Self {
            health: drawn_points(vitals.health),
            hunger: drawn_points(vitals.hunger),
            air: drawn_points(2.0 * vitals.air_s),
            air_full: vitals.air_s >= full.air_s,
            dead,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DamageCause {
    Fall { height_m: f32 },
    Drowning,
    Contact(MaterialId),
    Starvation,
}

#[derive(Component, Clone, Copy, Default, PartialEq, Debug)]
pub struct LastDamage(pub Option<DamageCause>);

/// The Player's Vitals, sent when they arrive, when their mode, death or last
/// damage changes, and when what the HUD draws of them does.
#[derive(Message, Clone, Copy, PartialEq, Debug)]
pub struct VitalsChanged {
    pub vitals: Vitals,
    pub full: Vitals,
    pub cause: Option<DamageCause>,
}

/// Use with no tool in hand: eat one of the Item in the selected hotbar slot.
/// Answered only when refused; the Vitals and the Inventory show success.
#[derive(Message, Clone, Copy, Debug)]
pub struct EatIntent;

pub(crate) fn plugin(app: &mut App) {
    app.add_session_message::<VitalsChanged>()
        .add_player_intent::<EatIntent>()
        .add_session_message::<UseOutcome>()
        .add_session_systems(
            FixedPreUpdate,
            eat.run_if(super::persistence::playing)
                .run_if(resource_exists::<Survival>)
                .before(super::inventory::announce_inventory),
        )
        .add_session_systems(
            FixedUpdate,
            (tick.run_if(super::death::alive), announce)
                .chain()
                .after(super::player::simulate)
                .run_if(resource_exists::<Survival>),
        );
}

impl PlayerState for VitalsV1 {
    fn capture(player: EntityRef) -> Option<Self> {
        player.get::<Vitals>().map(|v| VitalsV1 {
            health: v.health,
            hunger: v.hunger,
            air_s: v.air_s,
        })
    }

    fn restore(self, player: &mut EntityCommands) {
        player.insert(Vitals {
            health: self.health,
            hunger: self.hunger,
            air_s: self.air_s,
        });
    }
}

type Announced<'a> = (
    Ref<'a, Vitals>,
    Ref<'a, LastDamage>,
    Ref<'a, PlayerMode>,
    Has<Death>,
);

fn announce(
    survival: Res<Survival>,
    player: Single<Announced>,
    mut told: Local<Option<Drawn>>,
    mut changed: MessageWriter<VitalsChanged>,
) {
    let (vitals, last, mode, dead) = player.into_inner();
    let full = Vitals::full(&survival.rules);
    let drawn = Drawn::of(&vitals, &full, dead);
    if vitals.is_added()
        || last.is_changed()
        || mode.is_changed()
        || survival.is_changed()
        || *told != Some(drawn)
    {
        *told = Some(drawn);
        changed.write(VitalsChanged {
            vitals: *vitals,
            full,
            cause: last.0,
        });
    }
}

pub(super) fn eat(
    mut intents: MessageReader<EatIntent>,
    survival: Res<Survival>,
    items: Option<Res<Items>>,
    player: Single<(&mut Inventory, &mut Vitals, &PlayerMode)>,
    mut outcomes: MessageWriter<UseOutcome>,
) {
    let (mut inventory, mut vitals, mode) = player.into_inner();
    let max = survival.rules.max_hunger;
    for _ in intents.read() {
        let food = inventory
            .held()
            .and_then(|stack| Some((stack.item.clone(), items.as_ref()?.food(&stack.item)?)));
        let Some((item, food)) = food else {
            outcomes.write(UseOutcome(Err(UseRefusal::NotFood)));
            continue;
        };
        // The HUD rounds hunger up, so a Player who sees it full has less than max.
        if !mode.survival() || drawn_points(vitals.hunger) >= drawn_points(max) {
            outcomes.write(UseOutcome(Err(UseRefusal::NotHungry)));
            continue;
        }
        if inventory.spend(&item, 1).is_ok() {
            vitals.hunger = (vitals.hunger + food).min(max);
        }
    }
}

pub(super) fn tick(
    survival: Res<Survival>,
    player: Single<(&mut Player, &PlayerMode, &mut Vitals, &mut LastDamage)>,
    (store, tables, world): (Res<ChunkStore>, Res<ContentTables>, Res<WorldSettings>),
    time: Res<Time<Fixed>>,
    status: Option<Res<super::persistence::SaveStatus>>,
) {
    let (mut player, mode, mut vitals, mut last) = player.into_inner();
    let rules = &survival.rules;
    let dt = time.delta_secs();
    // Taken in Creative too, so a landing there never hurts later.
    let fall_m = player.take_fall_m();
    if !mode.survival() || status.is_some_and(|s| s.modal()) {
        return;
    }
    let mut now = *vitals;
    let mut hurt = |now: &mut Vitals, damage: f32, cause: DamageCause| {
        if damage > 0.0 {
            now.health -= damage;
            last.set_if_neq(LastDamage(Some(cause)));
        }
    };
    hurt(
        &mut now,
        (fall_m - rules.safe_fall_m).max(0.0) * rules.fall_damage_per_m,
        DamageCause::Fall { height_m: fall_m },
    );
    if player.eye_submerged() {
        let breathless_s = (dt - now.air_s).max(0.0);
        now.air_s = (now.air_s - dt).max(0.0);
        hurt(
            &mut now,
            breathless_s * rules.drowning_damage_per_s,
            DamageCause::Drowning,
        );
    } else {
        now.air_s = rules.max_air_s;
    }
    if let Some((material, per_s)) = contact(
        &tables.terrain(&store, world.voxel_size),
        player.body(),
        &survival,
    ) {
        hurt(&mut now, per_s * dt, DamageCause::Contact(material));
    }
    let hunger_per_min = if player.sprinting() {
        rules.sprinting_hunger_per_min
    } else {
        rules.hunger_per_min
    };
    now.hunger = (now.hunger - hunger_per_min / 60.0 * dt).max(0.0);
    if now.hunger <= 0.0 {
        hurt(
            &mut now,
            rules.starvation_damage_per_s * dt,
            DamageCause::Starvation,
        );
    } else if now.hunger >= rules.regeneration_hunger {
        now.health += rules.regeneration_per_s * dt;
    }
    now.health = now.health.clamp(0.0, rules.max_health);
    now.hunger = now.hunger.clamp(0.0, rules.max_hunger);
    now.air_s = now.air_s.min(rules.max_air_s);
    vitals.set_if_neq(now);
}

/// The most harmful Material the body overlaps, by Voxel for a solid and up to
/// its surface for a fluid.
fn contact(
    terrain: &Terrain<impl CellView>,
    body: Aabb,
    survival: &Survival,
) -> Option<(MaterialId, f32)> {
    if !survival.any_contact_damage() {
        return None;
    }
    let Terrain {
        voxels: store,
        tables,
        voxel_size: size,
    } = *terrain;
    // Shrunk so a body resting on a Voxel, or flush against one, does not
    // touch it.
    const EPS_M: f32 = 1e-3;
    let voxels = |a: usize| {
        ((body.min[a] + EPS_M) / size).floor() as i32
            ..=((body.max[a] - EPS_M) / size).floor() as i32
    };
    let mut worst: Option<(MaterialId, f32)> = None;
    for z in voxels(2) {
        for y in voxels(1) {
            for x in voxels(0) {
                let Some((voxel, overlay)) = store.stored([x, y, z]) else {
                    continue;
                };
                let here = Occupants::of(voxel, overlay, &tables.fluids);
                let solid = (!here.solid.is_air()).then(|| here.solid.material());
                let fluid = here.fluid.and_then(|fluid| {
                    let wettest = fluid
                        .others
                        .iter()
                        .fold(fluid.surface, |m, l| m.max(l.surface));
                    let top_m =
                        (y as f32 + f32::from(wettest) / f32::from(Fluid::FULL_SURFACE)) * size;
                    (body.min[1] + EPS_M < top_m).then_some(fluid.material)
                });
                for material in solid.into_iter().chain(fluid) {
                    let per_s = survival.contact_damage_per_s(material);
                    if per_s > worst.map_or(0.0, |(_, w)| w) {
                        worst = Some((material, per_s));
                    }
                }
            }
        }
    }
    worst
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use bevy::ecs::message::Messages;
    use bevy::time::TimeUpdateStrategy;
    use tesserae_core::{Chunk, Voxel};

    use super::*;
    use crate::server::ContentTables;
    use crate::server::game_mode::GameMode;
    use crate::server::world_state::{ChunkStore, SpawnPoint};
    use crate::world::{ChunkCoord, WorldSettings};

    fn rock() -> Voxel {
        Voxel::cube(tesserae_core::testgen::fixtures().ground.rock)
    }

    /// Between starving and healing, so a damage test sees damage alone.
    const HELD_HUNGER: f32 = 10.0;

    fn hold_hunger(mut app: App) -> App {
        let mut vitals = app.world_mut().query::<&mut Vitals>();
        vitals.single_mut(app.world_mut()).unwrap().hunger = HELD_HUNGER;
        app
    }

    /// A Server ticking once per update, holding one Chunk filled with rock
    /// below `floor_m`, and a walking Player in `mode` with their feet at
    /// `feet`.
    fn app(size: f32, floor_m: f32, feet: Vec3, mode: GameMode) -> App {
        let eye = feet + Vec3::Y * super::super::player::EYE_HEIGHT;
        hold_hunger(app_with(
            size,
            (floor_m, floor_m),
            Player::physics_at(eye),
            mode,
            Survival::default(),
        ))
    }

    fn water() -> tesserae_core::MaterialId {
        tesserae_core::testgen::fixtures().water
    }

    /// Rock below `floor_m` and a still lake from there up to `surface_m`, with
    /// a flying Player whose Eye is at `eye`: noclip holds it where a test
    /// puts it.
    fn lake(size: f32, (floor_m, surface_m): (f32, f32), eye: Vec3, mode: GameMode) -> App {
        lake_with(size, (floor_m, surface_m), eye, mode, Survival::default())
    }

    fn lake_with(
        size: f32,
        levels: (f32, f32),
        eye: Vec3,
        mode: GameMode,
        survival: Survival,
    ) -> App {
        hold_hunger(app_with(size, levels, Player::at(eye), mode, survival))
    }

    fn put_eye(app: &mut App, eye: Vec3) {
        app.world_mut()
            .query::<&mut Player>()
            .single_mut(app.world_mut())
            .unwrap()
            .set_eye(eye);
    }

    fn app_with(
        size: f32,
        (floor_m, surface_m): (f32, f32),
        spawned: Player,
        mode: GameMode,
        survival: Survival,
    ) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_nanos(
                15_625_000,
            )))
            .insert_resource(WorldSettings {
                voxel_size: size,
                ..default()
            })
            .insert_resource(SpawnPoint::for_tests(Vec3::ZERO))
            .init_resource::<ChunkStore>()
            .insert_resource(ContentTables(Arc::new(crate::server::fixture_tables())))
            .insert_resource(survival)
            .add_plugins((super::super::player::plugin, plugin));
        app.update();
        let cells = |m: f32| (m / size).round() as usize;
        let mut chunk = Chunk::air();
        for y in 0..cells(surface_m) {
            let voxel = if y < cells(floor_m) {
                rock()
            } else {
                Voxel::cube(water())
            };
            for z in 0..32 {
                for x in 0..32 {
                    chunk.set(x, y, z, voxel);
                }
            }
        }
        app.world_mut()
            .resource_mut::<ChunkStore>()
            .insert(ChunkCoord::new(0, 0, 0), chunk);
        let (mut player, mut player_mode) = app
            .world_mut()
            .query::<(&mut Player, &mut PlayerMode)>()
            .single_mut(app.world_mut())
            .unwrap();
        *player = spawned;
        player_mode.0 = mode;
        app
    }

    fn last_damage(app: &mut App) -> Option<DamageCause> {
        app.world_mut()
            .query::<&LastDamage>()
            .single(app.world())
            .unwrap()
            .0
    }

    fn seconds(app: &mut App, s: f32) {
        for _ in 0..(s * 64.0).round() as usize {
            app.update();
        }
    }

    fn vitals(app: &mut App) -> Vitals {
        *app.world_mut()
            .query::<&Vitals>()
            .single(app.world())
            .unwrap()
    }

    fn fall_damage(size: f32, height_m: f32, mode: GameMode) -> f32 {
        let floor_m = 2.0;
        let mut app = app(size, floor_m, Vec3::new(4.0, floor_m + height_m, 4.0), mode);
        let before = vitals(&mut app).health;
        seconds(&mut app, 3.0);
        before - vitals(&mut app).health
    }

    #[test]
    fn the_same_fall_in_metres_deals_the_same_damage_at_two_voxel_sizes() {
        let coarse = fall_damage(1.0, 10.0, GameMode::Survival);
        let fine = fall_damage(0.5, 10.0, GameMode::Survival);
        // 1 per metre beyond 3 m; the fixed step lands a little slower than
        // the exact 10 m.
        assert!((coarse - 7.0).abs() < 0.4, "{coarse}");
        assert!((coarse - fine).abs() < 0.01, "{coarse} vs {fine}");
        assert_eq!(
            fall_damage(1.0, 2.5, GameMode::Survival),
            0.0,
            "a safe fall"
        );
        assert_eq!(fall_damage(1.0, 10.0, GameMode::Creative), 0.0);
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.05
    }

    #[test]
    fn air_drains_only_with_the_eye_under_a_fluid_and_drowning_starts_at_zero() {
        for size in [1.0, 0.5] {
            let under = Vec3::new(4.0, 4.0, 4.0);
            let mut app = lake(size, (2.0, 6.0), under, GameMode::Survival);
            seconds(&mut app, 4.0);
            let now = vitals(&mut app);
            assert!(close(now.air_s, 6.0), "{size}: {now:?}");
            assert_eq!(now.health, 20.0, "{size}");

            seconds(&mut app, 8.0);
            let now = vitals(&mut app);
            assert_eq!(now.air_s, 0.0, "{size}");
            assert!(close(now.health, 16.0), "2 s at 2 per second: {now:?}");
            assert_eq!(last_damage(&mut app), Some(DamageCause::Drowning));

            put_eye(&mut app, Vec3::new(4.0, 6.5, 4.0));
            seconds(&mut app, 1.0);
            let now = vitals(&mut app);
            assert_eq!(now.air_s, 10.0, "refilled on surfacing: {size}");
            assert!(close(now.health, 16.0), "{size}: {now:?}");
        }
    }

    #[test]
    fn a_body_in_water_with_its_eye_above_keeps_its_air() {
        let mut app = lake(
            1.0,
            (2.0, 6.0),
            Vec3::new(4.0, 6.2, 4.0),
            GameMode::Survival,
        );
        seconds(&mut app, 3.0);
        let now = vitals(&mut app);
        assert_eq!((now.health, now.air_s), (20.0, 10.0));
    }

    #[test]
    fn contact_damage_applies_per_second_of_overlap_and_stops_on_leaving() {
        for size in [1.0, 0.5] {
            let harmful = Survival::new(default(), vec![(water(), 4.0)]);
            // Feet on the bed of a 1 m pool, Eye in the air above it.
            let wading = Vec3::new(4.0, 2.0 + super::super::player::EYE_HEIGHT, 4.0);
            let mut app = lake_with(size, (2.0, 3.0), wading, GameMode::Survival, harmful);
            seconds(&mut app, 1.5);
            let now = vitals(&mut app);
            assert!(close(now.health, 14.0), "{size}: {now:?}");
            assert_eq!(last_damage(&mut app), Some(DamageCause::Contact(water())));

            put_eye(&mut app, wading + Vec3::Y * 1.01);
            seconds(&mut app, 1.0);
            assert!(
                close(vitals(&mut app).health, 14.0),
                "{size}: left the pool"
            );
        }
        let harmful = Survival::new(default(), vec![(water(), 4.0)]);
        let wading = Vec3::new(4.0, 3.62, 4.0);
        let mut app = lake_with(1.0, (2.0, 3.0), wading, GameMode::Creative, harmful);
        seconds(&mut app, 1.0);
        assert_eq!(vitals(&mut app).health, 20.0, "Creative");
    }

    #[test]
    fn the_vitals_are_sent_on_arrival_and_on_change_with_the_last_cause() {
        let mut app = lake_with(
            1.0,
            (2.0, 6.0),
            Vec3::new(4.0, 7.0, 4.0),
            GameMode::Survival,
            Survival::new(no_drain(), Vec::new()),
        );
        let sent = |app: &mut App| {
            app.world_mut()
                .resource_mut::<Messages<VitalsChanged>>()
                .drain()
                .collect::<Vec<_>>()
        };
        app.update();
        let full = Vitals::default();
        assert_eq!(
            sent(&mut app).last(),
            Some(&VitalsChanged {
                vitals: Vitals {
                    hunger: HELD_HUNGER,
                    ..full
                },
                full,
                cause: None
            })
        );
        seconds(&mut app, 1.0);
        assert!(sent(&mut app).is_empty(), "nothing changed");
        put_eye(&mut app, Vec3::new(4.0, 4.0, 4.0));
        seconds(&mut app, 11.0);
        let last = *sent(&mut app).last().unwrap();
        assert_eq!(last.cause, Some(DamageCause::Drowning));
        assert_eq!(last.vitals, vitals(&mut app));
        assert_eq!(last.full, full);
    }

    /// Every Vitals message the Server sent, kept across updates.
    #[derive(Resource, Default)]
    struct Heard(Vec<VitalsChanged>);

    fn listen(mut heard: ResMut<Heard>, mut sent: ResMut<Messages<VitalsChanged>>) {
        heard.0.extend(sent.drain());
    }

    #[test]
    fn draining_hunger_is_sent_once_per_point_the_hud_draws_not_every_tick() {
        let rules = SurvivalRules {
            hunger_per_min: 60.0,
            ..default()
        };
        let mut app = field(rules, GameMode::Survival);
        app.init_resource::<Heard>().add_systems(Last, listen);
        let heard = |app: &mut App| std::mem::take(&mut app.world_mut().resource_mut::<Heard>().0);
        app.update();
        assert!(!heard(&mut app).is_empty(), "on arrival");
        seconds(&mut app, 3.0);
        let drained = heard(&mut app);
        assert!(
            (2..=4).contains(&drained.len()),
            "a point a second for 3 s: {}",
            drained.len()
        );
        let now = vitals(&mut app);
        let last = drained.last().unwrap().vitals;
        assert_eq!(last.hunger.ceil(), now.hunger.ceil(), "{last:?} {now:?}");

        let mut mode = app.world_mut().query::<&mut PlayerMode>();
        mode.single_mut(app.world_mut()).unwrap().0 = GameMode::Creative;
        app.update();
        assert_eq!(heard(&mut app).len(), 1, "on a mode change");
    }

    #[test]
    fn a_new_player_starts_full_by_the_games_rules() {
        let rules = SurvivalRules {
            max_health: 30.0,
            max_hunger: 12.0,
            max_air_s: 15.0,
            ..no_drain()
        };
        let mut app = app_with(
            1.0,
            (2.0, 2.0),
            Player::at(Vec3::new(4.0, 6.0, 4.0)),
            GameMode::Survival,
            Survival::new(rules, Vec::new()),
        );
        seconds(&mut app, 0.5);
        assert_eq!(vitals(&mut app), Vitals::full(&rules));
    }

    fn no_drain() -> SurvivalRules {
        SurvivalRules {
            hunger_per_min: 0.0,
            sprinting_hunger_per_min: 0.0,
            ..default()
        }
    }

    /// A walking Player standing on a rock floor, under `rules`.
    fn field(rules: SurvivalRules, mode: GameMode) -> App {
        let feet = Vec3::new(4.0, 2.0, 4.0);
        app_with(
            1.0,
            (2.0, 2.0),
            Player::physics_at(feet + Vec3::Y * super::super::player::EYE_HEIGHT),
            mode,
            Survival::new(rules, Vec::new()),
        )
    }

    fn set_vitals(app: &mut App, health: f32, hunger: f32) {
        let mut vitals = app.world_mut().query::<&mut Vitals>();
        let mut vitals = vitals.single_mut(app.world_mut()).unwrap();
        vitals.health = health;
        vitals.hunger = hunger;
    }

    fn walk(app: &mut App, direction: Vec3, sprint: bool) {
        app.world_mut()
            .write_message(super::super::player::MovementIntent {
                direction,
                sprint,
                ..default()
            });
    }

    #[test]
    fn hunger_drains_at_the_declared_rate_and_faster_only_while_sprinting() {
        let rules = SurvivalRules {
            hunger_per_min: 60.0,
            sprinting_hunger_per_min: 120.0,
            ..default()
        };
        let mut app = field(rules, GameMode::Survival);
        seconds(&mut app, 3.0);
        let now = vitals(&mut app).hunger;
        assert!(close(now, 17.0), "idle: {now}");

        walk(&mut app, Vec3::ZERO, true);
        seconds(&mut app, 1.0);
        let now = vitals(&mut app).hunger;
        assert!(close(now, 16.0), "sprint held while standing: {now}");

        walk(&mut app, Vec3::X, true);
        seconds(&mut app, 2.0);
        let now = vitals(&mut app).hunger;
        assert!(close(now, 12.0), "sprinting: {now}");

        walk(&mut app, Vec3::X, false);
        seconds(&mut app, 2.0);
        let now = vitals(&mut app).hunger;
        assert!(close(now, 10.0), "walking: {now}");
    }

    #[test]
    fn health_regenerates_at_the_threshold_and_starvation_hurts_at_zero() {
        let mut app = field(no_drain(), GameMode::Survival);
        set_vitals(&mut app, 10.0, 18.0);
        seconds(&mut app, 4.0);
        let now = vitals(&mut app).health;
        assert!(close(now, 11.0), "at the threshold, 1 per 4 s: {now}");

        set_vitals(&mut app, 10.0, 17.9);
        seconds(&mut app, 4.0);
        assert_eq!(vitals(&mut app).health, 10.0, "below it");

        set_vitals(&mut app, 10.0, 0.0);
        seconds(&mut app, 4.0);
        let now = vitals(&mut app).health;
        assert!(close(now, 9.0), "starving, 1 per 4 s: {now}");
        assert_eq!(last_damage(&mut app), Some(DamageCause::Starvation));
    }

    #[test]
    fn health_regenerates_at_the_games_declared_rate() {
        let rules = SurvivalRules {
            regeneration_per_s: 1.5,
            ..no_drain()
        };
        let mut app = field(rules, GameMode::Survival);
        set_vitals(&mut app, 10.0, 18.0);
        seconds(&mut app, 2.0);
        let now = vitals(&mut app).health;
        assert!(close(now, 13.0), "1.5 per second: {now}");
    }

    #[test]
    fn creative_players_neither_hunger_nor_starve() {
        let rules = SurvivalRules {
            hunger_per_min: 60.0,
            sprinting_hunger_per_min: 120.0,
            ..default()
        };
        let mut app = field(rules, GameMode::Creative);
        walk(&mut app, Vec3::X, true);
        seconds(&mut app, 2.0);
        assert_eq!(vitals(&mut app), Vitals::default());
        set_vitals(&mut app, 10.0, 0.0);
        seconds(&mut app, 2.0);
        assert_eq!(vitals(&mut app).health, 10.0);
    }

    fn pantry(app: &mut App, stacks: &[(&str, u32)]) {
        let def = |name: &str, food| tesserae_mods::ItemDef {
            name: name.into(),
            size_m: 0.25,
            color: [0, 0, 0, 255],
            material: None,
            max_stack: 64,
            food,
        };
        app.insert_resource(super::super::items::Items(vec![
            def("m:bread", Some(6.0)),
            def("m:pebble", None),
        ]));
        let mut inventory = app
            .world_mut()
            .query::<&mut super::super::inventory::Inventory>();
        let mut inventory = inventory.single_mut(app.world_mut()).unwrap();
        for &(item, count) in stacks {
            inventory.receive(item, count, 64);
        }
    }

    fn eat(app: &mut App) -> Vec<super::super::tool::UseOutcome> {
        app.world_mut().write_message(EatIntent);
        app.update();
        app.world_mut()
            .resource_mut::<Messages<super::super::tool::UseOutcome>>()
            .drain()
            .collect()
    }

    fn held(app: &mut App) -> Option<(String, u32)> {
        let mut inventory = app
            .world_mut()
            .query::<&super::super::inventory::Inventory>();
        inventory
            .single(app.world())
            .unwrap()
            .held()
            .map(|s| (s.item.clone(), s.count))
    }

    #[test]
    fn eating_spends_one_held_food_and_refills_hunger_up_to_the_maximum() {
        use super::super::tool::{UseOutcome, UseRefusal};
        let mut app = field(no_drain(), GameMode::Survival);
        pantry(&mut app, &[("m:bread", 2)]);
        set_vitals(&mut app, 20.0, 10.0);
        assert!(eat(&mut app).is_empty(), "no message for a routine success");
        assert_eq!(vitals(&mut app).hunger, 16.0);
        assert_eq!(held(&mut app), Some(("m:bread".into(), 1)));

        eat(&mut app);
        assert_eq!(vitals(&mut app).hunger, 20.0, "capped at the maximum");
        assert_eq!(held(&mut app), None);

        pantry(&mut app, &[("m:bread", 1)]);
        assert_eq!(
            eat(&mut app),
            [UseOutcome(Err(UseRefusal::NotHungry))],
            "full"
        );
        assert_eq!(held(&mut app), Some(("m:bread".into(), 1)));
    }

    #[test]
    fn eating_at_full_hunger_spends_nothing_in_either_mode() {
        use super::super::tool::{UseOutcome, UseRefusal};
        for (mode, hunger) in [
            (GameMode::Survival, 20.0),
            // Drawn as full: the HUD rounds up.
            (GameMode::Survival, 19.9),
            (GameMode::Creative, 20.0),
            (GameMode::Creative, 5.0),
        ] {
            let mut app = field(no_drain(), mode);
            pantry(&mut app, &[("m:bread", 3)]);
            set_vitals(&mut app, 20.0, hunger);
            assert_eq!(
                eat(&mut app),
                [UseOutcome(Err(UseRefusal::NotHungry))],
                "{mode:?} at {hunger}"
            );
            assert_eq!(held(&mut app), Some(("m:bread".into(), 3)));
            assert_eq!(vitals(&mut app).hunger, hunger);
        }
    }

    #[test]
    fn eating_what_is_not_food_or_nothing_is_refused() {
        use super::super::tool::{UseOutcome, UseRefusal};
        let mut app = field(no_drain(), GameMode::Survival);
        set_vitals(&mut app, 20.0, 10.0);
        assert_eq!(eat(&mut app), [UseOutcome(Err(UseRefusal::NotFood))]);
        pantry(&mut app, &[("m:pebble", 3)]);
        assert_eq!(eat(&mut app), [UseOutcome(Err(UseRefusal::NotFood))]);
        assert_eq!(held(&mut app), Some(("m:pebble".into(), 3)));
        assert_eq!(vitals(&mut app).hunger, 10.0);
    }

    #[test]
    fn creative_players_take_no_damage_and_keep_their_air() {
        let mut app = lake(
            1.0,
            (2.0, 6.0),
            Vec3::new(4.0, 4.0, 4.0),
            GameMode::Creative,
        );
        seconds(&mut app, 12.0);
        assert_eq!(
            vitals(&mut app),
            Vitals {
                hunger: HELD_HUNGER,
                ..default()
            }
        );
        assert_eq!(last_damage(&mut app), None);
    }
}
