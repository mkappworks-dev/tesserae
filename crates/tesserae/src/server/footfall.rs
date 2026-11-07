//! The Sounds of a Player's feet: a `step` per stride walked and a `land`
//! after a fall, both from the solid underfoot, measured from the body the
//! Server advances.

use bevy::prelude::*;
use tesserae_core::MaterialId;
use tesserae_core::audio::Cause;

use super::ContentTables;
use super::player::Player;
use super::sound::{SoundEvent, SoundTable};
use super::world_state::ChunkStore;
use crate::session::SessionApp;
use crate::world::WorldSettings;

/// Metres of horizontal travel between two `step`s.
pub const STRIDE_M: f32 = 1.5;
/// The least fall in metres that makes a `land`.
pub const LAND_FALL_M: f32 = 1.0;

pub(super) fn plugin(app: &mut App) {
    app.init_session_resource::<Gait>()
        .add_session_message::<SoundEvent>()
        .add_session_systems(FixedUpdate, tread.after(super::player::simulate));
}

/// The metres walked towards the next `step`.
#[derive(Resource, Default)]
struct Gait {
    stride_m: f32,
}

fn tread(
    mut player: Single<&mut Player>,
    (store, tables, world): (Res<ChunkStore>, Res<ContentTables>, Res<WorldSettings>),
    (sounds, time): (Res<SoundTable>, Res<Time<Fixed>>),
    mut gait: ResMut<Gait>,
    mut heard: MessageWriter<SoundEvent>,
) {
    if player.take_landing_m() >= LAND_FALL_M
        && let Some((at, material)) = underfoot(&player, &store, &tables, world.voxel_size)
    {
        heard.write_batch(sounds.voxel(Cause::Land, material, at, world.voxel_size, player.eye()));
    }
    let body = player.body_state();
    if !body.grounded || body.crouching || body.swimming {
        return;
    }
    gait.stride_m += body.horizontal_mps * time.delta_secs();
    if gait.stride_m < STRIDE_M {
        return;
    }
    gait.stride_m -= STRIDE_M;
    if let Some((at, material)) = underfoot(&player, &store, &tables, world.voxel_size) {
        heard.write_batch(sounds.voxel(Cause::Step, material, at, world.voxel_size, player.eye()));
    }
}

/// The solid the feet stand on, and its Voxel.
fn underfoot(
    player: &Player,
    store: &ChunkStore,
    tables: &ContentTables,
    size: f32,
) -> Option<([i32; 3], MaterialId)> {
    // A body resting on a top face has its feet on the boundary.
    const PROBE_M: f32 = 0.01;
    let feet = player.eye().with_y(player.body().min[1] - PROBE_M);
    let at = (feet / size).floor().as_ivec3().to_array();
    let solid = store.occupants(at, &tables.fluids)?.solid;
    (!solid.is_air()).then(|| (at, solid.material()))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use bevy::time::TimeUpdateStrategy;
    use tesserae_core::audio::{Cause, Falloff, SoundCategory, SoundDef, SoundId, SoundSet, Span};
    use tesserae_core::{Chunk, MaterialId, Voxel};

    use super::*;
    use crate::server::player::{MovementIntent, Player};
    use crate::server::sound::SoundTable;
    use crate::server::world_state::{ChunkStore, SpawnPoint};
    use crate::server::{ContentTables, fixture_tables};
    use crate::world::{ChunkCoord, WorldSettings};

    const STEP: SoundId = SoundId(0);
    const LAND: SoundId = SoundId(1);
    const SOIL_LAND: SoundId = SoundId(2);
    const FLOOR_M: f32 = 2.0;

    fn rock() -> MaterialId {
        tesserae_core::testgen::fixtures().ground.rock
    }

    fn soil() -> MaterialId {
        tesserae_core::testgen::fixtures().ground.soil
    }

    fn sound() -> tesserae_mods::Sound {
        tesserae_mods::Sound {
            name: "t:thud".into(),
            def: SoundDef {
                weights: vec![1],
                volume: Span::at(1.0),
                pitch: Span::at(1.0),
                category: SoundCategory::Effects,
                falloff: Some(Falloff {
                    reference_m: 1.0,
                    max_m: 32.0,
                }),
            },
            recordings: vec![Vec::new()],
        }
    }

    /// `spawned` over a floor of rock [`FLOOR_M`] thick whose steps and
    /// landings are voiced, under water up to `surface_m`.
    fn app(size: f32, spawned: Player, surface_m: f32) -> App {
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
            .insert_resource(ContentTables(Arc::new(fixture_tables())))
            .insert_resource(SoundTable::new(&tesserae_mods::Sounds {
                sounds: vec![sound(); 3],
                material_sounds: vec![
                    (
                        rock(),
                        SoundSet {
                            step: Some(STEP),
                            land: Some(LAND),
                            ..SoundSet::default()
                        },
                    ),
                    (
                        soil(),
                        SoundSet {
                            land: Some(SOIL_LAND),
                            ..SoundSet::default()
                        },
                    ),
                ],
            }))
            .add_plugins((crate::server::player::plugin, plugin));
        app.update();
        let mut chunk = Chunk::air();
        let cells = |m: f32| (m / size).round() as usize;
        for y in 0..cells(surface_m.max(FLOOR_M)) {
            let voxel = if y < cells(FLOOR_M) {
                Voxel::cube(rock())
            } else {
                Voxel::cube(tesserae_core::testgen::fixtures().water)
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
        *app.world_mut()
            .query::<&mut Player>()
            .single_mut(app.world_mut())
            .unwrap() = spawned;
        app
    }

    /// Where the Eye is when the feet stand on the floor at `x`, `z` metres.
    fn on_floor(x: f32, z: f32) -> Vec3 {
        Vec3::new(x, FLOOR_M + crate::server::player::EYE_HEIGHT, z)
    }

    fn standing(size: f32, x: f32, z: f32) -> App {
        app(size, Player::physics_at(on_floor(x, z)), 0.0)
    }

    fn body(app: &mut App) -> crate::server::player::BodyState {
        app.world_mut()
            .query::<&Player>()
            .single(app.world())
            .unwrap()
            .body_state()
    }

    fn eye(app: &mut App) -> Vec3 {
        app.world_mut()
            .query::<&Player>()
            .single(app.world())
            .unwrap()
            .eye()
    }

    fn heard(app: &mut App) -> Vec<SoundEvent> {
        app.world_mut()
            .resource_mut::<Messages<SoundEvent>>()
            .drain()
            .collect()
    }

    /// Moves as `intent` asks until the Eye has gone `metres` across, then
    /// stops; the Sound events on the way.
    fn walk(app: &mut App, intent: MovementIntent, metres: f32) -> Vec<SoundEvent> {
        let from = eye(app);
        app.world_mut().write_message(intent);
        let mut events = Vec::new();
        for _ in 0..64 * 10 {
            app.update();
            events.extend(heard(app));
            if (eye(app) - from).with_y(0.0).length() >= metres {
                break;
            }
        }
        app.world_mut().write_message(MovementIntent::default());
        app.update();
        events.extend(heard(app));
        assert!(
            (eye(app) - from).with_y(0.0).length() >= metres,
            "went only {} m",
            (eye(app) - from).length()
        );
        events
    }

    fn east() -> MovementIntent {
        MovementIntent {
            direction: Vec3::X,
            ..default()
        }
    }

    fn causes(events: &[SoundEvent]) -> Vec<(SoundId, Cause)> {
        events.iter().map(|e| (e.sound, e.cause)).collect()
    }

    #[test]
    fn walking_six_metres_on_rock_makes_four_steps_from_the_rock_underfoot() {
        let mut app = standing(1.0, 4.5, 4.5);
        let events = walk(&mut app, east(), 6.01);
        assert_eq!(causes(&events), [(STEP, Cause::Step); 4]);
        // Each from a Voxel of the floor's top layer, centred in metres.
        for e in &events {
            assert_eq!(e.pos_m[1], 1.5, "{e:?}");
            assert_eq!(e.pos_m[2], 4.5, "{e:?}");
        }
        // A sprint keeps the stride: its steps only come faster.
        let sprint = MovementIntent {
            speed: crate::server::player::SpeedLevel::One,
            ..east()
        };
        assert_eq!(
            causes(&walk(&mut app, sprint, 6.01)),
            [(STEP, Cause::Step); 4]
        );
    }

    #[test]
    fn crouching_flying_swimming_and_standing_still_make_no_steps() {
        let mut crouched = standing(1.0, 4.5, 4.5);
        let crouch = MovementIntent {
            crouch: true,
            ..east()
        };
        assert!(walk(&mut crouched, crouch, 6.01).is_empty());

        let mut flying = app(1.0, Player::at(on_floor(4.5, 4.5)), 0.0);
        let fly = MovementIntent {
            speed_mps: crate::server::player::WALK_MPS,
            ..east()
        };
        assert!(walk(&mut flying, fly, 6.01).is_empty());

        // A metre of water wets more than half the body, which swims on the
        // bed.
        let mut swimmer = app(1.0, Player::physics_at(on_floor(4.5, 4.5)), FLOOR_M + 1.0);
        swimmer.update();
        let state = body(&mut swimmer);
        assert!(state.swimming && state.grounded, "{state:?}");
        assert!(walk(&mut swimmer, east(), 6.01).is_empty());

        let mut still = standing(1.0, 4.5, 4.5);
        for _ in 0..64 * 4 {
            still.update();
        }
        assert!(heard(&mut still).is_empty());
    }

    #[test]
    fn steps_follow_the_stride_in_metres_at_every_voxel_size() {
        // The top of the floor is one Voxel thick at each size.
        for (size, top_y) in [(0.5, 1.75), (2.0, 1.0)] {
            let mut app = standing(size, 4.1, 4.1);
            let events = walk(&mut app, east(), 6.01);
            assert_eq!(causes(&events), [(STEP, Cause::Step); 4], "at {size} m");
            assert!(events.iter().all(|e| e.pos_m[1] == top_y), "{events:?}");
        }
    }

    #[test]
    fn a_solid_with_no_sound_set_is_silent_and_the_stride_still_advances() {
        // Glass for the first five metres east, then rock.
        let mut app = standing(1.0, 3.6, 4.5);
        let mut chunk = app
            .world()
            .resource::<ChunkStore>()
            .get(ChunkCoord::new(0, 0, 0))
            .unwrap()
            .clone();
        for z in 0..32 {
            for x in 0..5 {
                chunk.set(
                    x,
                    1,
                    z,
                    Voxel::cube(tesserae_core::testgen::fixtures().glass),
                );
            }
        }
        app.world_mut()
            .resource_mut::<ChunkStore>()
            .insert(ChunkCoord::new(0, 0, 0), chunk);
        let events = walk(&mut app, east(), 1.55);
        assert_eq!(causes(&events), [(STEP, Cause::Step)]);
        assert_eq!(events[0].pos_m[0], 5.5);
    }

    fn seconds(app: &mut App, s: f32) -> Vec<SoundEvent> {
        (0..(s * 64.0).round() as usize)
            .flat_map(|_| {
                app.update();
                heard(app)
            })
            .collect()
    }

    fn lands(events: &[SoundEvent]) -> usize {
        events.iter().filter(|e| e.cause == Cause::Land).count()
    }

    #[test]
    fn a_drop_of_a_metre_lands_and_one_of_nine_tenths_does_not() {
        for (drop_m, count) in [(0.9, 0), (1.0, 1)] {
            let eye = on_floor(4.5, 4.5) + Vec3::Y * drop_m;
            let mut app = app(1.0, Player::physics_at(eye), 0.0);
            let events = seconds(&mut app, 2.0);
            assert!(body(&mut app).grounded);
            assert_eq!(lands(&events), count, "from {drop_m} m: {events:?}");
        }
    }

    #[test]
    fn a_landing_sounds_the_solid_landed_on() {
        let mut app = app(
            1.0,
            Player::physics_at(on_floor(4.5, 4.5) + Vec3::Y * 3.0),
            0.0,
        );
        let mut chunk = app
            .world()
            .resource::<ChunkStore>()
            .get(ChunkCoord::new(0, 0, 0))
            .unwrap()
            .clone();
        chunk.set(4, 1, 4, Voxel::cube(soil()));
        app.world_mut()
            .resource_mut::<ChunkStore>()
            .insert(ChunkCoord::new(0, 0, 0), chunk);
        assert_eq!(
            seconds(&mut app, 2.0),
            [SoundEvent {
                sound: SOIL_LAND,
                pos_m: [4.5, 1.5, 4.5],
                cause: Cause::Land,
            }]
        );
    }

    fn switch(app: &mut App) {
        app.world_mut().write_message(MovementIntent {
            switch_mode: true,
            ..default()
        });
    }

    #[test]
    fn a_fall_caught_by_flight_does_not_land_from_its_old_height() {
        // Falling from 6 m, the Player takes flight, flies down and drops the
        // last half metre.
        let start = on_floor(4.5, 4.5) + Vec3::Y * 6.0;
        let mut app = app(1.0, Player::physics_at(start), 0.0);
        app.update();
        assert!(eye(&mut app).y < start.y);
        switch(&mut app);
        let down = MovementIntent {
            direction: Vec3::NEG_Y,
            speed_mps: crate::server::player::WALK_MPS,
            ..default()
        };
        app.world_mut().write_message(down);
        while eye(&mut app).y > on_floor(4.5, 4.5).y + 0.5 {
            app.update();
        }
        switch(&mut app);
        let events = seconds(&mut app, 2.0);
        assert!(body(&mut app).grounded);
        assert_eq!(lands(&events), 0, "{events:?}");
    }
}
