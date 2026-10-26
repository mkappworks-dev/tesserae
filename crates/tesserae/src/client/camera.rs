//! A fly camera: WASD to move, mouse to look, shift to sprint, `-` and `=` to
//! change speed, R to return to the spawn point, Esc to release the cursor.
//!
//! Hand-rolled rather than a dependency so the Client's one piece of input
//! handling stays small enough to read, and so it is already expressed in
//! metres per second when the player controller replaces it.

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use std::f32::consts::FRAC_PI_2;

use crate::server::player::{FLY_SPEED_CEILING_MPS, RespawnIntent};

/// Just inside ±90°, where the yaw/pitch basis degenerates.
const PITCH_LIMIT: f32 = FRAC_PI_2 - 1e-3;

/// Metres per second. The lower bound is a slow walk for inspecting a face; the
/// upper is a Cell every half second, past which Chunks cannot be meshed fast
/// enough to keep up once streaming exists.
const SPEED_RANGE: std::ops::RangeInclusive<f32> = 0.5..=64.0;

/// Multiplier per press, so the range above crosses in a dozen presses
/// regardless of where in it you start.
const SPEED_PER_PRESS: f32 = 1.5;

/// The Player's camera. `With<Camera3d>` also matches the coarse-level cameras,
/// so anything that means "the camera" filters by this.
#[derive(Component)]
pub struct MainCamera;

#[derive(Component)]
pub struct FlyCamera {
    /// Metres per second, not Voxels per second. A speed in Voxels would be
    /// secretly calibrated to one Voxel size.
    /// See `docs/adr/0003-voxel-size-is-per-world.md`.
    pub speed: f32,
    pub sprint_multiplier: f32,
    /// Radians of rotation per pixel of mouse travel.
    pub sensitivity: f32,
    pub yaw: f32,
    pub pitch: f32,
}

impl Default for FlyCamera {
    fn default() -> Self {
        FlyCamera {
            speed: 12.0,
            sprint_multiplier: 5.0,
            sensitivity: 0.003,
            yaw: 0.0,
            pitch: 0.0,
        }
    }
}

impl FlyCamera {
    /// Derives yaw and pitch from a transform so the camera can be spawned with
    /// `looking_at` and not snap on the first frame.
    pub fn looking_along(direction: Vec3) -> Self {
        let d = direction.normalize_or_zero();
        FlyCamera {
            yaw: (-d.x).atan2(-d.z),
            pitch: d.y.asin().clamp(-PITCH_LIMIT, PITCH_LIMIT),
            ..default()
        }
    }
}

/// How [`reset`] turns the camera back. Only the orientation: where the camera
/// goes is the Server's spawn eye, which it sends back as a correction.
#[derive(Component, Clone, Copy)]
pub struct CameraHome(pub Transform);

/// Shown whenever the cursor is free. Mouse look is gated on a click and nothing
/// else on screen says so, which reads as the mouse not working at all.
#[derive(Component)]
struct CursorHint;

pub(super) fn plugin(app: &mut App) {
    let input = (grab_cursor, show_hint, reset, look, adjust_speed, fly).chain();
    #[cfg(feature = "dev")]
    let input = input.run_if(|| std::env::var_os("TESSERAE_SCREENSHOT").is_none());
    app.add_systems(Startup, spawn_hint)
        .add_systems(Update, input);
}

fn spawn_hint(mut commands: Commands) {
    commands.spawn((
        CursorHint,
        // ASCII only: Bevy's embedded default font is a FiraMono subset and
        // renders anything outside it, a middle dot included, as tofu.
        Text::new("Click to look around  -  Esc to release the cursor"),
        TextFont {
            font_size: bevy::text::FontSize::Px(14.0),
            ..default()
        },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.7)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(16.0),
            bottom: Val::Px(16.0),
            ..default()
        },
    ));
}

fn show_hint(
    cursor: Single<&CursorOptions, With<PrimaryWindow>>,
    visibility: Single<&mut Visibility, With<CursorHint>>,
) {
    let wanted = if cursor.grab_mode == CursorGrabMode::None {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    // Through change detection rather than unconditionally, so the UI is not
    // re-laid out every frame.
    visibility.into_inner().set_if_neq(wanted);
}

fn reset(
    keys: Res<ButtonInput<KeyCode>>,
    camera: Single<(&mut Transform, &mut FlyCamera, &CameraHome)>,
    mut respawn: MessageWriter<RespawnIntent>,
) {
    if !keys.just_pressed(KeyCode::KeyR) {
        return;
    }
    // Asked, not done: moving the camera here would be a teleport the Server
    // clamps. Its reply puts the camera at the spawn eye.
    respawn.write(RespawnIntent);
    let (mut transform, mut fly, home) = camera.into_inner();
    transform.rotation = home.0.rotation;
    let restored = FlyCamera::looking_along(*home.0.forward());
    fly.yaw = restored.yaw;
    fly.pitch = restored.pitch;
}

/// Keys change how fast you fly rather than the field of view: in a fly camera
/// the distance to what you are looking at is the thing you actually want to
/// control, and W/S already do that. The scroll wheel picks a hotbar slot.
fn adjust_speed(
    keys: Res<ButtonInput<KeyCode>>,
    cursor: Single<&CursorOptions, With<PrimaryWindow>>,
    mut fly: Single<&mut FlyCamera>,
) {
    if cursor.grab_mode == CursorGrabMode::None {
        return;
    }
    let presses =
        keys.just_pressed(KeyCode::Equal) as i32 - keys.just_pressed(KeyCode::Minus) as i32;
    if presses != 0 {
        fly.speed = (fly.speed * SPEED_PER_PRESS.powi(presses))
            .clamp(*SPEED_RANGE.start(), *SPEED_RANGE.end());
    }
}

fn grab_cursor(
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let mut cursor = cursor.into_inner();
    if keys.just_pressed(KeyCode::Escape) {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    } else if mouse.just_pressed(MouseButton::Left) {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
}

fn look(
    motion: Res<AccumulatedMouseMotion>,
    cursor: Single<&CursorOptions, With<PrimaryWindow>>,
    camera: Single<(&mut Transform, &mut FlyCamera)>,
) {
    if cursor.grab_mode == CursorGrabMode::None || motion.delta == Vec2::ZERO {
        return;
    }

    let (mut transform, mut fly) = camera.into_inner();
    fly.yaw -= motion.delta.x * fly.sensitivity;
    fly.pitch = (fly.pitch - motion.delta.y * fly.sensitivity).clamp(-PITCH_LIMIT, PITCH_LIMIT);
    transform.rotation = Quat::from_euler(EulerRot::YXZ, fly.yaw, fly.pitch, 0.0);
}

/// Metres per second. Held to the Server's ceiling: it clamps a Player faster
/// than that, so flying past it only rubber-bands.
fn flight_speed(fly: &FlyCamera, sprinting: bool) -> f32 {
    let speed = if sprinting {
        fly.speed * fly.sprint_multiplier
    } else {
        fly.speed
    };
    speed.min(FLY_SPEED_CEILING_MPS)
}

fn fly(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    camera: Single<(&mut Transform, &FlyCamera)>,
) {
    let (mut transform, fly) = camera.into_inner();

    let mut direction = Vec3::ZERO;
    for (key, axis) in [
        (KeyCode::KeyW, *transform.forward()),
        (KeyCode::KeyS, *transform.back()),
        (KeyCode::KeyA, *transform.left()),
        (KeyCode::KeyD, *transform.right()),
        (KeyCode::Space, Vec3::Y),
        (KeyCode::ControlLeft, Vec3::NEG_Y),
    ] {
        if keys.pressed(key) {
            direction += axis;
        }
    }

    let sprinting = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let speed = flight_speed(fly, sprinting);
    transform.translation += direction.normalize_or_zero() * speed * time.delta_secs();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looking_along_round_trips_through_the_rotation_it_implies() {
        for direction in [
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(-3.0, -2.0, -4.0),
            Vec3::new(2.0, 5.0, 1.0),
        ] {
            let fly = FlyCamera::looking_along(direction);
            let rotation = Quat::from_euler(EulerRot::YXZ, fly.yaw, fly.pitch, 0.0);
            let forward = rotation * Vec3::NEG_Z;
            assert!(
                forward.abs_diff_eq(direction.normalize(), 1e-5),
                "{direction} became {forward}"
            );
        }
    }

    #[test]
    fn flight_never_outruns_the_servers_ceiling() {
        let mut fly = FlyCamera::default();
        for speed in [*SPEED_RANGE.start(), 12.0, *SPEED_RANGE.end()] {
            fly.speed = speed;
            for sprinting in [false, true] {
                assert!(flight_speed(&fly, sprinting) <= FLY_SPEED_CEILING_MPS);
            }
        }
        fly.speed = 12.0;
        assert_eq!(flight_speed(&fly, false), 12.0, "slow flight is untouched");
        assert_eq!(
            flight_speed(&fly, true),
            60.0,
            "and so is a sprint under it"
        );
        fly.speed = 20.0;
        assert_eq!(flight_speed(&fly, true), FLY_SPEED_CEILING_MPS);
    }

    #[test]
    fn the_reset_key_asks_the_server_and_turns_the_camera_but_does_not_move_it() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .add_message::<RespawnIntent>()
            .add_systems(Update, reset);
        let home = Transform::from_xyz(0.0, 50.0, 0.0).looking_to(Vec3::NEG_Z, Vec3::Y);
        let away =
            Transform::from_xyz(300.0, 9.0, -40.0).looking_to(Vec3::new(1.0, 0.0, 0.0), Vec3::Y);
        app.world_mut()
            .spawn((away, FlyCamera::default(), CameraHome(home)));

        app.update();
        assert_eq!(
            app.world().resource::<Messages<RespawnIntent>>().len(),
            0,
            "nothing pressed"
        );

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyR);
        app.update();

        assert_eq!(app.world().resource::<Messages<RespawnIntent>>().len(), 1);
        let transform = app
            .world_mut()
            .query_filtered::<&Transform, With<FlyCamera>>()
            .single(app.world())
            .unwrap();
        assert_eq!(
            transform.translation, away.translation,
            "the Server moves it"
        );
        assert!(transform.rotation.abs_diff_eq(home.rotation, 1e-6));
    }

    #[test]
    fn pitch_stays_inside_the_basis() {
        let straight_up = FlyCamera::looking_along(Vec3::Y);
        assert!(straight_up.pitch.abs() <= PITCH_LIMIT);
    }
}
