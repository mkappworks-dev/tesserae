//! A fly camera: WASD to move, mouse to look, shift to sprint, Esc to release
//! the cursor.
//!
//! Hand-rolled rather than a dependency so the Client's one piece of input
//! handling stays small enough to read, and so it is already expressed in
//! metres per second when the player controller replaces it.

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use std::f32::consts::FRAC_PI_2;

/// Just inside ±90°, where the yaw/pitch basis degenerates.
const PITCH_LIMIT: f32 = FRAC_PI_2 - 1e-3;

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

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Update, (grab_cursor, look, fly).chain());
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
