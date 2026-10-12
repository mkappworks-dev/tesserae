//! Writes a PNG of the running app and exits. `dev` only, and inert unless
//! `TESSERAE_SCREENSHOT` names a path. `TESSERAE_SCREENSHOT_OFFSCREEN` renders
//! into an image instead of the window, which is what to use when nobody is
//! looking at the screen.
//!
//! Through Bevy's own screenshot path rather than an OS screen grab, because a
//! screen grab needs macOS Screen Recording permission that a build machine, or
//! an agent, does not have — and because flying the camera for a fixed distance
//! first makes the result the same picture every time.

use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::window::PresentMode;

use crate::cost::{FrameCost, MainThreadCost};

use crate::client::camera::FlyCamera;
use crate::server::streaming::stats::StreamingStats;

/// Seconds of simulated flight before the shutter, so the picture shows Chunks
/// that streamed in around a camera that moved rather than the initial fill.
/// The flight starts once the initial fill has finished, so the two are
/// measured apart.
const FLY_FOR_S: f32 = 6.0;
/// How long the initial fill is given before the flight starts regardless, so a
/// fill that never settles fails a run instead of hanging it.
const FILL_TIMEOUT_S: f32 = 30.0;
/// And then long enough for the Chunks ahead to arrive and mesh.
const SETTLE_FOR_S: f32 = 4.0;

#[derive(Resource)]
struct Capture {
    path: String,
    fly_for_s: f32,
    settle_for_s: f32,
    /// Metres per second, when the player's own speed is not the one wanted: a
    /// sprint asks streaming for several times what a walk does.
    speed_m_s: Option<f32>,
    /// Earliest time the flight may start, so two builds whose fills end at
    /// different times fly at the same time since launch.
    fly_after_s: f32,
    /// Render to an image instead of the window; see [`render_offscreen`].
    offscreen: bool,
    taken: bool,
    /// Whether the screenshot entity has actually been observed in the World.
    pending_seen: bool,
    frames: FrameLog,
}

/// Every frame after `SETTLES_AFTER_S`, sorted into the two situations the
/// streaming budget is judged by. The initial fill and flight want different
/// answers: one is a burst of work with nothing yet to draw, the other is a
/// steady trickle behind a moving camera.
#[derive(Default)]
struct FrameLog {
    /// When the initial fill ended and the flight began.
    fill_ended_s: Option<f32>,
    /// Whether it ended because the Server settled and not on the timeout.
    settled: bool,
    /// When the Server first reported nothing left to do, which the flight may
    /// start later than.
    settled_s: Option<f32>,
    /// When every Chunk in the mesh extent had first been sent a mesh.
    shown_s: Option<f32>,
    /// Worst frame before that.
    worst_fill_s: f32,
    /// Every frame of the flight, in seconds.
    flight: Vec<f32>,
    /// The longest single tick-system run, in seconds.
    worst_tick_s: f32,
    /// The most the main thread spent on streaming in one frame (ticks plus
    /// mesh uploads), before the fill ended and after.
    worst_cost_fill_s: f32,
    worst_cost_flight_s: f32,
    uploads: u32,
    upload_total_s: f32,
}

impl FrameLog {
    fn record(&mut self, elapsed_s: f32, delta_s: f32, settled: bool, fly_for_s: f32) {
        // Classified before the fill is allowed to end, so the frame that
        // finished it still counts as part of it.
        if elapsed_s >= SETTLES_AFTER_S {
            match self.fill_ended_s {
                None => self.worst_fill_s = self.worst_fill_s.max(delta_s),
                Some(end) if elapsed_s <= end + fly_for_s => self.flight.push(delta_s),
                Some(_) => {}
            }
        }
        if self.fill_ended_s.is_none() && (settled || elapsed_s >= FILL_TIMEOUT_S) {
            self.fill_ended_s = Some(elapsed_s);
            self.settled = settled;
        }
    }

    fn note_settled(&mut self, elapsed_s: f32, settled: bool) {
        if self.settled_s.is_none() && settled {
            self.settled_s = Some(elapsed_s);
        }
    }

    fn note_shown(&mut self, elapsed_s: f32, shown: bool) {
        if self.shown_s.is_none() && shown {
            self.shown_s = Some(elapsed_s);
        }
    }

    fn note_cost(&mut self, elapsed_s: f32, cost: FrameCost, fly_for_s: f32) {
        self.worst_tick_s = self.worst_tick_s.max(cost.tick_s);
        self.uploads += cost.uploads;
        self.upload_total_s += cost.upload_s;
        let spent = cost.tick_s + cost.upload_s;
        match self.fill_ended_s {
            None => self.worst_cost_fill_s = self.worst_cost_fill_s.max(spent),
            Some(end) if elapsed_s <= end + fly_for_s => {
                self.worst_cost_flight_s = self.worst_cost_flight_s.max(spent);
            }
            Some(_) => {}
        }
    }

    /// Whether the camera should be moving at this time.
    fn flying(&self, elapsed_s: f32, fly_for_s: f32) -> bool {
        self.fill_ended_s
            .is_some_and(|end| elapsed_s > end && elapsed_s <= end + fly_for_s)
    }

    fn flight_summary(&self) -> Option<FlightSummary> {
        let mut sorted = self.flight.clone();
        sorted.sort_unstable_by(f32::total_cmp);
        Some(FlightSummary {
            frames: sorted.len(),
            median_s: percentile(&sorted, 0.5)?,
            p99_s: percentile(&sorted, 0.99)?,
            worst_s: *sorted.last()?,
        })
    }
}

struct FlightSummary {
    frames: usize,
    median_s: f32,
    p99_s: f32,
    worst_s: f32,
}

/// Nearest-rank percentile of an ascending slice; `None` when it is empty.
fn percentile(sorted: &[f32], p: f32) -> Option<f32> {
    let rank = (p * sorted.len() as f32).ceil() as usize;
    sorted.get(rank.max(1) - 1).copied()
}

impl Capture {
    /// Time for the picture: after the flight, and long enough past it for the
    /// Chunks ahead to arrive.
    fn due(&self, elapsed_s: f32) -> bool {
        !self.taken
            && self
                .frames
                .fill_ended_s
                .is_some_and(|end| elapsed_s >= end + self.fly_for_s + self.settle_for_s)
    }
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
    app.insert_resource(Capture {
        path,
        fly_for_s: seconds("TESSERAE_SCREENSHOT_FLY_S", FLY_FOR_S),
        settle_for_s: seconds("TESSERAE_SCREENSHOT_SETTLE_S", SETTLE_FOR_S),
        fly_after_s: seconds("TESSERAE_SCREENSHOT_FLY_AFTER_S", 0.0),
        speed_m_s: std::env::var("TESSERAE_SCREENSHOT_SPEED_M_S")
            .ok()
            .and_then(|v| v.parse().ok()),
        offscreen: std::env::var_os("TESSERAE_SCREENSHOT_OFFSCREEN").is_some(),
        taken: false,
        pending_seen: false,
        frames: FrameLog::default(),
    })
    .add_systems(Startup, uncap_frame_rate)
    .add_systems(
        Update,
        (
            render_offscreen.run_if(|capture: Res<Capture>, target: Option<Res<Offscreen>>| {
                capture.offscreen && target.is_none()
            }),
            watch_frames,
            fly_forward,
            report,
            shutter,
            exit_when_written,
        )
            .chain(),
    );
}

/// Presents without waiting for the display, unless `TESSERAE_SCREENSHOT_VSYNC`
/// is set. Under vsync a frame is as long as the refresh interval, which the
/// display changes from run to run (60, 120, 144 Hz on the same commit), so the
/// worst frame would measure the monitor. Unthrottled, a frame costs what its
/// work costs, and a frame that runs a streaming tick shows it.
fn uncap_frame_rate(mut windows: Query<&mut Window>) {
    if std::env::var_os("TESSERAE_SCREENSHOT_VSYNC").is_some() {
        return;
    }
    for mut window in &mut windows {
        window.present_mode = PresentMode::AutoNoVsync;
    }
}

/// What an offscreen capture renders into.
#[derive(Resource)]
struct Offscreen(Handle<Image>);

/// Renders the camera into an image instead of the window, for a capture.
/// `TESSERAE_SCREENSHOT_OFFSCREEN` turns it on.
///
/// A window that is not being presented is never drawn, and a capture of one is
/// pure black: a locked screen does it, and so does a window on another Space or
/// under a full-screen app. An image target does not depend on the display at
/// all, so the picture is the same whoever is at the machine. Off by default
/// because it leaves the window blank, and the frame figures want the real
/// presentation path.
fn render_offscreen(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    camera: Single<Entity, With<Camera3d>>,
) {
    let image = Image::new_target_texture(1920, 1080, TextureFormat::Rgba8UnormSrgb, None);
    let handle = images.add(image);
    commands
        .entity(*camera)
        .insert(RenderTarget::Image(handle.clone().into()));
    commands.insert_resource(Offscreen(handle));
}

/// Ignores the first second and a half: window creation, pipeline
/// specialisation and the first shader compile all land there and say nothing
/// about what streaming costs.
const SETTLES_AFTER_S: f32 = 1.5;

fn watch_frames(
    time: Res<Time>,
    stats: Res<StreamingStats>,
    mut capture: ResMut<Capture>,
    mut cost: ResMut<MainThreadCost>,
) {
    let fly_for_s = capture.fly_for_s;
    let frame_cost = cost.take_frame();
    capture
        .frames
        .note_cost(time.elapsed_secs(), frame_cost, fly_for_s);
    capture.frames.note_shown(time.elapsed_secs(), stats.shown);
    capture
        .frames
        .note_settled(time.elapsed_secs(), stats.settled);
    let may_fly = time.elapsed_secs() >= capture.fly_after_s;
    capture.frames.record(
        time.elapsed_secs(),
        time.delta_secs(),
        stats.settled && may_fly,
        fly_for_s,
    );
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
    if !capture
        .frames
        .flying(time.elapsed_secs(), capture.fly_for_s)
    {
        return;
    }
    let (mut transform, fly) = camera.into_inner();
    let heading = transform.forward().with_y(0.0).normalize_or_zero();
    let speed = capture.speed_m_s.unwrap_or(fly.speed);
    transform.translation += heading * speed * time.delta_secs();
}

/// Logs what the picture is of, and the frame times behind it, as one line of
/// `key=value` pairs so `scripts/frame-runs` can aggregate several runs.
/// Runs before `shutter` rather than inside it, so neither function carries the
/// other's system parameters.
fn report(
    time: Res<Time>,
    capture: Res<Capture>,
    stats: Res<StreamingStats>,
    camera: Single<&Transform, With<Camera3d>>,
    drawn: Query<(), With<Mesh3d>>,
) {
    if !capture.due(time.elapsed_secs()) {
        return;
    }
    let ms = |s: f32| format!("{:.1}", s * 1000.0);
    let fill = capture
        .frames
        .settled_s
        .map_or("never".to_string(), |s| format!("{s:.2}"));
    let shown = capture
        .frames
        .shown_s
        .map_or("never".to_string(), |s| format!("{s:.2}"));
    let flight = capture.frames.flight_summary();
    let or_na =
        |f: fn(&FlightSummary) -> f32| flight.as_ref().map_or("na".to_string(), |s| ms(f(s)));
    let f = &capture.frames;
    info!(
        "capture_cost worst_tick_ms={} worst_cost_fill_ms={} worst_cost_flight_ms={} \
         mean_upload_us={:.0} uploads={}",
        ms(f.worst_tick_s),
        ms(f.worst_cost_fill_s),
        ms(f.worst_cost_flight_s),
        f.upload_total_s * 1e6 / f.uploads.max(1) as f32,
        f.uploads,
    );
    info!(
        "capture shown_s={shown} fill_s={fill} worst_fill_ms={} worst_flight_ms={} \
         median_flight_ms={} p99_flight_ms={} flight_frames={} \
         loaded={} meshed={} drawn={} relight_landed={} relight_stale={} mesh_landed={} \
         mesh_stale={} chunk={} camera={:?}",
        ms(capture.frames.worst_fill_s),
        or_na(|s| s.worst_s),
        or_na(|s| s.median_s),
        or_na(|s| s.p99_s),
        flight.as_ref().map_or(0, |s| s.frames),
        stats.loaded,
        stats.meshed,
        drawn.iter().count(),
        stats.landed[1],
        stats.stale[1],
        stats.landed[2],
        stats.stale[2],
        stats.anchor,
        camera.translation,
    );
}

fn shutter(
    mut commands: Commands,
    time: Res<Time>,
    mut capture: ResMut<Capture>,
    offscreen: Option<Res<Offscreen>>,
) {
    if !capture.due(time.elapsed_secs()) {
        return;
    }
    capture.taken = true;
    let screenshot = match offscreen {
        Some(target) => Screenshot::image(target.0.clone()),
        None => Screenshot::primary_window(),
    };
    commands
        .spawn(screenshot)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentiles_are_nearest_rank() {
        let v: Vec<f32> = (1..=100).map(|i| i as f32).collect();
        assert_eq!(percentile(&v, 0.5), Some(50.0));
        assert_eq!(percentile(&v, 0.99), Some(99.0));
        assert_eq!(percentile(&[7.0], 0.99), Some(7.0));
        assert_eq!(percentile(&[], 0.5), None);
    }

    #[test]
    fn frames_before_the_fill_and_during_flight_are_kept_apart() {
        let mut log = FrameLog::default();
        // Startup is ignored, then the fill's worst frame, then the fill ends.
        log.record(0.5, 0.200, false, 6.0);
        log.record(1.6, 0.040, false, 6.0);
        log.record(1.7, 0.010, false, 6.0);
        log.record(2.0, 0.009, true, 6.0);
        // The Server unsettling again during flight does not reopen the fill.
        log.record(3.0, 0.020, false, 6.0);
        log.record(4.0, 0.010, true, 6.0);
        // After the flight, frames are not flight.
        log.record(8.5, 0.500, true, 6.0);

        assert_eq!(log.fill_ended_s, Some(2.0));
        assert!(log.settled);
        assert_eq!(log.worst_fill_s, 0.040);
        assert_eq!(log.flight.len(), 2);
        let flight = log.flight_summary().expect("frames were recorded");
        assert_eq!(flight.worst_s, 0.020);
    }

    #[test]
    fn shown_is_latched_at_the_first_time_it_is_true() {
        let mut log = FrameLog::default();
        log.note_shown(1.0, false);
        log.note_shown(2.0, true);
        log.note_shown(3.0, true);
        assert_eq!(log.shown_s, Some(2.0));
    }

    #[test]
    fn the_camera_flies_only_between_the_fill_and_the_end_of_the_flight() {
        let mut log = FrameLog::default();
        assert!(!log.flying(1.0, 6.0));
        log.record(2.0, 0.01, true, 6.0);
        assert!(!log.flying(2.0, 6.0));
        assert!(log.flying(2.1, 6.0));
        assert!(log.flying(8.0, 6.0));
        assert!(!log.flying(8.1, 6.0));
    }

    #[test]
    fn a_fill_that_never_settles_times_out_and_is_not_reported_as_settled() {
        let mut log = FrameLog::default();
        log.record(2.0, 0.030, false, 6.0);
        assert_eq!(log.fill_ended_s, None);
        assert!(log.flight_summary().is_none());

        log.record(FILL_TIMEOUT_S, 0.030, false, 6.0);
        assert_eq!(log.fill_ended_s, Some(FILL_TIMEOUT_S));
        assert!(!log.settled);
    }
}
