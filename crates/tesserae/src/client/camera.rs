//! A fly camera: WASD to move, mouse to look, shift to sprint, scroll to change
//! speed, R to return to the spawn point, Esc to release the cursor.
//!
//! Hand-rolled rather than a dependency so the Client's one piece of input
//! handling stays small enough to read, and so it is already expressed in
//! metres per second when the player controller replaces it.

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use std::f32::consts::FRAC_PI_2;

/// Just inside ±90°, where the yaw/pitch basis degenerates.
const PITCH_LIMIT: f32 = FRAC_PI_2 - 1e-3;

/// Metres per second. The lower bound is a slow walk for inspecting a face; the
/// upper is a Cell every half second, past which Chunks cannot be meshed fast
/// enough to keep up once streaming exists.
const SPEED_RANGE: std::ops::RangeInclusive<f32> = 0.5..=64.0;

/// Multiplier per scroll line, so the range above crosses in about fifty
/// notches regardless of where in it you start.
const SPEED_PER_NOTCH: f32 = 1.1;

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

/// Where [`reset`] returns to. Set by whatever spawns the camera, so the
/// Server's spawn point can own it once step 3 exists.
#[derive(Component, Clone, Copy)]
pub struct CameraHome(pub Transform);

/// Shown whenever the cursor is free. Mouse look is gated on a click and nothing
/// else on screen says so, which reads as the mouse not working at all.
#[derive(Component)]
struct CursorHint;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Startup, spawn_hint).add_systems(
        Update,
        (grab_cursor, show_hint, reset, look, adjust_speed, fly).chain(),
    );
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
) {
    if !keys.just_pressed(KeyCode::KeyR) {
        return;
    }
    let (mut transform, mut fly, home) = camera.into_inner();
    *transform = home.0;
    let restored = FlyCamera::looking_along(*home.0.forward());
    fly.yaw = restored.yaw;
    fly.pitch = restored.pitch;
}

/// Scroll changes how fast you fly rather than the field of view: in a fly
/// camera the distance to what you are looking at is the thing you actually
/// want to control, and W/S already do that. Multiplicative so a notch means
/// the same thing when crawling along a face and when crossing the grid.
fn adjust_speed(
    scroll: Res<AccumulatedMouseScroll>,
    cursor: Single<&CursorOptions, With<PrimaryWindow>>,
    mut fly: Single<&mut FlyCamera>,
) {
    if cursor.grab_mode == CursorGrabMode::None || scroll.delta.y == 0.0 {
        return;
    }
    // A trackpad reports pixels and a wheel reports lines; without this a
    // trackpad flick would cross the whole range in one gesture.
    let notches = match scroll.unit {
        MouseScrollUnit::Line => scroll.delta.y,
        MouseScrollUnit::Pixel => scroll.delta.y / 16.0,
    };
    fly.speed =
        (fly.speed * SPEED_PER_NOTCH.powf(notches)).clamp(*SPEED_RANGE.start(), *SPEED_RANGE.end());
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
    let speed = if sprinting {
        fly.speed * fly.sprint_multiplier
    } else {
        fly.speed
    };
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
    fn pitch_stays_inside_the_basis() {
        let straight_up = FlyCamera::looking_along(Vec3::Y);
        assert!(straight_up.pitch.abs() <= PITCH_LIMIT);
    }
}
