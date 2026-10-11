//! Writes a PNG of the running app and exits. `dev` only, and inert unless
//! `TESSERAE_SCREENSHOT` names a path.
//!
//! Through Bevy's own screenshot path rather than an OS screen grab, because a
//! screen grab needs macOS Screen Recording permission that a build machine, or
//! an agent, does not have — and because flying the camera for a fixed distance
//! first makes the result the same picture every time.

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};

use crate::client::camera::FlyCamera;

/// Seconds of simulated flight before the shutter, so the picture shows Chunks
/// that streamed in around a camera that moved rather than the initial fill.
const FLY_FOR_S: f32 = 6.0;
/// And then long enough for the Chunks ahead to arrive and mesh.
const SETTLE_FOR_S: f32 = 4.0;

#[derive(Resource)]
struct Capture {
    path: String,
    fly_for_s: f32,
    at_s: f32,
    taken: bool,
    /// Whether the screenshot entity has actually been observed in the World.
    pending_seen: bool,
    /// Worst frame since `SETTLES_AFTER_S`, for sizing the streaming budget.
    worst_frame_s: f32,
}

fn seconds(var: &str, fallback: f32) -> f32 {
    std::env::var(var)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(fallback)
}

pub(super) fn plugin(app: &mut App) {
    let Ok(path) = std::env::var("TESSERAE_SCREENSHOT") else {
        return;
    };
    let fly_for_s = seconds("TESSERAE_SCREENSHOT_FLY_S", FLY_FOR_S);
    app.insert_resource(Capture {
        path,
        fly_for_s,
        at_s: fly_for_s + seconds("TESSERAE_SCREENSHOT_SETTLE_S", SETTLE_FOR_S),
        taken: false,
        pending_seen: false,
        worst_frame_s: 0.0,
    })
    .add_systems(
        Update,
        (watch_frames, fly_forward, shutter, exit_when_written).chain(),
    );
}

/// Ignores the first second and a half: window creation, pipeline
/// specialisation and the first shader compile all land there and say nothing
/// about what streaming costs.
const SETTLES_AFTER_S: f32 = 1.5;

fn watch_frames(time: Res<Time>, mut capture: ResMut<Capture>) {
    if time.elapsed_secs() < SETTLES_AFTER_S {
        return;
    }
    capture.worst_frame_s = capture.worst_frame_s.max(time.delta_secs());
}

/// Stands in for the player holding W, so the screenshot exercises streaming
/// rather than only the Startup fill.
///
/// Horizontally, not along `forward`: the camera looks down, so following its
/// own forward vector for a few seconds flies it into the ground and the
/// picture comes out black.
fn fly_forward(
    time: Res<Time>,
    capture: Res<Capture>,
    camera: Single<(&mut Transform, &FlyCamera)>,
) {
    if time.elapsed_secs() > capture.fly_for_s {
        return;
    }
    let (mut transform, fly) = camera.into_inner();
    let heading = transform.forward().with_y(0.0).normalize_or_zero();
    transform.translation += heading * fly.speed * time.delta_secs();
}

fn shutter(
    mut commands: Commands,
    time: Res<Time>,
    mut capture: ResMut<Capture>,
    stats: Res<crate::server::streaming::stats::StreamingStats>,
    generator: Res<crate::server::WorldGenerator>,
    diagnostics: Res<bevy::diagnostic::DiagnosticsStore>,
    camera: Single<&Transform, With<Camera3d>>,
    drawn: Query<(), With<Mesh3d>>,
) {
    if capture.taken || time.elapsed_secs() < capture.at_s {
        return;
    }
    capture.taken = true;
    let worst_frame_ms = diagnostics
        .get(&bevy::diagnostic::FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .map(|d| d.values().copied().fold(0.0f64, f64::max))
        .unwrap_or(0.0);
    info!(
        "shutter at {:.1}s: camera {:?} surface {:.1}m chunk {} loaded {} \
         delivered {} entities {} worst frame {worst_frame_ms:.1}ms \
         (since {SETTLES_AFTER_S}s: {:.1}ms)",
        time.elapsed_secs(),
        camera.translation,
        generator.surface_height_m(camera.translation.x, camera.translation.z),
        stats.anchor,
        stats.loaded,
        stats.meshed,
        drawn.iter().count(),
        capture.worst_frame_s * 1000.0,
    );
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(capture.path.clone()));
}

/// Quits once the screenshot entity has been captured and despawned, which is
/// how `bevy_render` signals the file is written.
///
/// The `pending_seen` latch is the whole point: `shutter` spawns through
/// `Commands`, so on that frame the entity does not exist yet and an
/// `is_empty()` test alone reads as "already finished". Exiting there writes
/// `AppExit` before the frame is presented, and the capture comes out pure
/// black — a picture of an unwritten drawable, with no sky and no UI in it.
fn exit_when_written(
    mut capture: ResMut<Capture>,
    pending: Query<(), With<Screenshot>>,
    mut exit: MessageWriter<AppExit>,
) {
    if !capture.taken {
        return;
    }
    if pending.is_empty() {
        if capture.pending_seen {
            exit.write(AppExit::Success);
        }
    } else {
        capture.pending_seen = true;
    }
}
