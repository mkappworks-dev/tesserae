//! Writes a PNG of the running app and exits. Available with `dev` or
//! `profiling`, and inert unless
//! `TESSERAE_SCREENSHOT` names a path. `TESSERAE_SCREENSHOT_OFFSCREEN` renders
//! into an image instead of the window, which is what to use when nobody is
//! looking at the screen. `TESSERAE_SCREENSHOT_MENU` opens the picker menu, so
//! the picture shows it.
//!
//! Through Bevy's own screenshot path rather than an OS screen grab, because a
//! screen grab needs macOS Screen Recording permission that a build machine, or
//! an agent, does not have — and because flying the camera for a fixed distance
//! first makes the result the same picture every time.

use bevy::camera::RenderTarget;
use bevy::mesh::MeshTag;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::tasks::AsyncComputeTaskPool;
use bevy::window::PresentMode;

use crate::cost::{FrameCost, MainThreadCost};
use crate::world::WorldSettings;

use crate::client::camera::{FlyCamera, MainCamera};
use crate::server::player::FLY_SPEED_CEILING_MPS;
use crate::server::streaming::stats::{LevelStats, StreamingStats};

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
    origin: Option<std::time::Instant>,
    path: String,
    fly_for_s: f32,
    settle_for_s: f32,
    /// Metres per second, when the player's own speed is not the one wanted: a
    /// sprint asks streaming for several times what a walk does.
    speed_m_s: Option<f32>,
    /// Degrees above the horizon, set on the first frame. The default view looks
    /// down at the ground and shows nothing of the distance.
    pitch_deg: Option<f32>,
    /// Degrees of heading, set on the first frame. The default flies along an
    /// axis from x = z = 0, which stays on a lattice line of every level; any
    /// other heading leaves it.
    yaw_deg: Option<f32>,
    lift_m: f32,
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
    /// Per level, when it first held the Chunks it holds once filled, and when
    /// every Chunk of its mesh shell had first been sent a mesh.
    level_filled_s: Vec<Option<f32>>,
    level_shown_s: Vec<Option<f32>>,
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
    /// The longest a frame spent re-sorting blended meshes, and how many it did.
    worst_sort_s: f32,
    sorts: u32,
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

    fn note_levels(&mut self, elapsed_s: f32, levels: &[LevelStats]) {
        let latch = |times: &mut Vec<Option<f32>>, level: usize, reached: bool| {
            times.resize(times.len().max(level + 1), None);
            if reached && times[level].is_none() {
                times[level] = Some(elapsed_s);
            }
        };
        for (level, s) in levels.iter().enumerate() {
            latch(
                &mut self.level_filled_s,
                level,
                s.steady > 0 && s.held >= s.steady,
            );
            latch(
                &mut self.level_shown_s,
                level,
                s.shell > 0 && s.meshed >= s.shell,
            );
        }
    }

    fn note_cost(&mut self, elapsed_s: f32, cost: FrameCost, fly_for_s: f32) {
        self.worst_tick_s = self.worst_tick_s.max(cost.longest_tick_s);
        self.uploads += cost.uploads;
        self.upload_total_s += cost.upload_s;
        self.worst_sort_s = self.worst_sort_s.max(cost.sort_s);
        self.sorts += cost.sorts;
        let spent = cost.tick_s + cost.upload_s + cost.sort_s;
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
    /// Whether the shutter is at most `lead_s` away.
    fn before_shutter(&self, elapsed_s: f32, lead_s: f32) -> bool {
        self.frames
            .fill_ended_s
            .is_some_and(|start| elapsed_s >= start + self.fly_for_s + self.settle_for_s - lead_s)
    }
    fn elapsed_s(&self, time: &Time<Real>) -> f32 {
        self.origin.map_or_else(
            || time.elapsed_secs(),
            |origin| origin.elapsed().as_secs_f32(),
        )
    }
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
    // Not the sky's grey, so a crack showing the clear colour can be counted.
    if std::env::var_os("TESSERAE_SCREENSHOT_MAGENTA_SKY").is_some() {
        app.insert_resource(ClearColor(Color::srgb(1.0, 0.0, 1.0)));
    }
    #[cfg(feature = "profiling")]
    let origin = std::env::var_os("TESSERAE_PROFILE_FIXTURE").and_then(|_| {
        app.world()
            .get_resource::<crate::profiling::ProcessStart>()
            .map(|start| start.0)
    });
    #[cfg(not(feature = "profiling"))]
    let origin = None;
    app.insert_resource(Capture {
        origin,
        path,
        fly_for_s: seconds("TESSERAE_SCREENSHOT_FLY_S", FLY_FOR_S),
        settle_for_s: seconds("TESSERAE_SCREENSHOT_SETTLE_S", SETTLE_FOR_S),
        fly_after_s: seconds("TESSERAE_SCREENSHOT_FLY_AFTER_S", 0.0),
        speed_m_s: std::env::var("TESSERAE_SCREENSHOT_SPEED_M_S")
            .ok()
            .and_then(|v| v.parse().ok()),
        pitch_deg: std::env::var("TESSERAE_SCREENSHOT_PITCH_DEG")
            .ok()
            .and_then(|v| v.parse().ok()),
        yaw_deg: std::env::var("TESSERAE_SCREENSHOT_YAW_DEG")
            .ok()
            .and_then(|v| v.parse().ok()),
        lift_m: seconds("TESSERAE_SCREENSHOT_LIFT_M", 0.0),
        offscreen: std::env::var_os("TESSERAE_SCREENSHOT_OFFSCREEN").is_some(),
        taken: false,
        pending_seen: false,
        frames: FrameLog::default(),
    });
    if std::env::var_os("TESSERAE_SCREENSHOT_SPRINT").is_some() {
        app.insert_resource(super::camera::Sprint(true));
    }
    if std::env::var_os("TESSERAE_SCREENSHOT_MENU").is_some() {
        app.insert_resource(super::menu::Menu::opened());
    }
    app.insert_resource(super::help::Panels {
        debug: std::env::var_os("TESSERAE_SCREENSHOT_DEBUG").is_some(),
    });
    app.add_systems(
        Update,
        super::removal::capture_progress
            .after(super::removal::hold)
            .before(super::removal::bar),
    );
    app.add_systems(PreUpdate, save_preview);
    app.add_systems(Startup, (uncap_frame_rate, view_fixture))
        .add_systems(
            Update,
            (
                render_offscreen.after(super::preview::camera).run_if(
                    |capture: Res<Capture>, target: Option<Res<Offscreen>>| {
                        capture.offscreen && target.is_none()
                    },
                ),
                capture_holding,
                menu_view
                    .after(super::menu::rebuild_bar)
                    .before(super::menu::drag_preview),
                menu_drag.before(super::menu::drag_preview),
                watch_frames,
                aim,
                fly_forward,
                report,
                shutter,
                exit_when_written,
            )
                .chain(),
        );
}

/// `TESSERAE_SCREENSHOT_VIEW=third` and `TESSERAE_SCREENSHOT_ZOOM_M` choose the
/// view for this run only: nothing is written to the user profile.
fn view_fixture(mut preferences: ResMut<super::profile::Preferences>) {
    use super::view::{View, ZOOM_M};
    preferences.view = match std::env::var("TESSERAE_SCREENSHOT_VIEW").as_deref() {
        Ok("third") => View::ThirdPerson,
        Ok("first") | Err(_) => View::FirstPerson,
        Ok(other) => panic!("Unknown fixture view: {other}"),
    };
    if let Some(zoom) = std::env::var("TESSERAE_SCREENSHOT_ZOOM_M")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
    {
        preferences.zoom_m = zoom.clamp(*ZOOM_M.start(), *ZOOM_M.end());
    }
}

fn save_preview(
    capture: Res<Capture>,
    mut status: ResMut<crate::server::persistence::SaveStatus>,
    mut shown: Local<bool>,
) {
    if *shown || capture.frames.fill_ended_s.is_none() {
        return;
    }
    let Ok(state) = std::env::var("TESSERAE_SCREENSHOT_SAVE_STATE") else {
        return;
    };
    match state.as_str() {
        "saving" => {
            status.quitting = true;
            status.saving = true;
        }
        "error" | "confirm" => {
            status.quitting = true;
            status.saving = false;
            status.error = Some("Disk full (visual fixture)".into());
            status.confirm_unsafe = state == "confirm";
        }
        _ => return,
    }
    *shown = true;
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

/// Renders the cameras into an image instead of the window, for a capture.
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
    cameras: Query<Entity, (With<Camera3d>, Without<super::tile_preview::AtlasCamera>)>,
    main: Single<Entity, With<MainCamera>>,
    menu: Res<super::menu::Menu>,
) {
    let dimension = |name, fallback| {
        std::env::var(name)
            .ok()
            .and_then(|n| n.parse::<u32>().ok())
            .filter(|&n| n > 0)
            .unwrap_or(fallback)
    };
    let image = Image::new_target_texture(
        dimension("TESSERAE_SCREENSHOT_WIDTH", 1920),
        dimension("TESSERAE_SCREENSHOT_HEIGHT", 1080),
        TextureFormat::Rgba8UnormSrgb,
        None,
    );
    let handle = images.add(image);
    // Every window camera must draw into the capture; the icon atlas stays separate.
    for camera in &cameras {
        commands
            .entity(camera)
            .insert(RenderTarget::Image(handle.clone().into()));
    }
    // The UI follows a window camera by default, and none is left.
    if menu.open {
        commands.entity(*main).insert(IsDefaultUiCamera);
    }
    commands.insert_resource(Offscreen(handle));
}

fn menu_view(
    mut commands: Commands,
    catalogue: Res<super::edit::Catalogue>,
    entries: Query<(Entity, &super::menu::Entry)>,
    mut positions: Query<&mut ScrollPosition, With<bevy::ui_widgets::ScrollArea>>,
    mut hovered: Local<bool>,
) {
    if let Some(scroll) = std::env::var("TESSERAE_SCREENSHOT_MENU_SCROLL_PX")
        .ok()
        .and_then(|n| n.parse::<f32>().ok())
    {
        for mut position in &mut positions {
            position.y = scroll;
        }
    }
    if *hovered {
        return;
    }
    let Ok(name) = std::env::var("TESSERAE_SCREENSHOT_MENU_HOVER") else {
        return;
    };
    let Some(material) = catalogue.0.materials.iter().find(|m| m.name == name) else {
        return;
    };
    let Some((entity, _)) = entries
        .iter()
        .find(|(_, e)| **e == super::menu::Entry::Material(material.id))
    else {
        return;
    };
    use bevy::picking::{
        backend::HitData,
        events::{Pointer, PointerOver},
        pointer::{Location, PointerId},
    };
    commands.trigger(PointerOver {
        entity,
        pointer: Pointer::new(
            PointerId::Mouse,
            Location {
                target: bevy::camera::NormalizedRenderTarget::None {
                    width: 1280,
                    height: 600,
                },
                position: Vec2::ZERO,
            },
        ),
        hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
    });
    *hovered = true;
}

#[derive(Default)]
struct MenuDrag {
    source: Option<Entity>,
    dropped: bool,
}

fn menu_drag(
    mut commands: Commands,
    catalogue: Res<super::edit::Catalogue>,
    time: Res<Time>,
    entries: Query<(
        Entity,
        &super::menu::Entry,
        &UiGlobalTransform,
        &ComputedNode,
    )>,
    mut drag: Local<MenuDrag>,
) {
    use super::menu::Entry;
    use bevy::picking::{
        backend::HitData,
        events::{Pointer, PointerDrag, PointerDragDrop, PointerDragStart, PointerOver},
        pointer::{Location, PointerButton, PointerId},
    };

    let Ok(name) = std::env::var("TESSERAE_SCREENSHOT_MENU_DRAG") else {
        return;
    };
    if drag.dropped {
        return;
    }
    let wanted = if name == "fire" {
        Entry::Fire
    } else if let Some(material) = catalogue.0.materials.iter().find(|m| m.name == name) {
        if material.fluid.is_some() {
            Entry::Fluid(material.id)
        } else {
            Entry::Material(material.id)
        }
    } else {
        return;
    };
    let slot = std::env::var("TESSERAE_SCREENSHOT_MENU_DRAG_SLOT")
        .ok()
        .and_then(|n| n.parse::<usize>().ok())
        .unwrap_or(5)
        .clamp(1, 9)
        - 1;
    let Some((target, _, transform, node)) = entries
        .iter()
        .find(|(_, e, _, node)| **e == Entry::Slot(slot) && node.size().min_element() > 0.0)
    else {
        return;
    };
    let affine: bevy::math::Affine2 = transform.into();
    let position = affine.translation * node.inverse_scale_factor();
    let pointer = Pointer::new(
        PointerId::Mouse,
        Location {
            target: bevy::camera::NormalizedRenderTarget::None {
                width: 1920,
                height: 1080,
            },
            position,
        },
    );
    let hit = HitData::new(Entity::PLACEHOLDER, 0.0, None, None);
    let button = PointerButton::Primary;
    let Some((source, _, _, _)) = entries.iter().find(|(_, e, _, _)| **e == wanted) else {
        return;
    };
    if drag.source.is_none() {
        commands.trigger(PointerDragStart {
            entity: source,
            pointer: pointer.clone(),
            button,
            hit: hit.clone(),
        });
        drag.source = Some(source);
    }
    commands.trigger(PointerDrag {
        entity: source,
        pointer: pointer.clone(),
        button,
        distance: Vec2::ZERO,
        delta: Vec2::ZERO,
    });
    commands.trigger(PointerOver {
        entity: target,
        pointer: pointer.clone(),
        hit: hit.clone(),
    });
    if std::env::var_os("TESSERAE_SCREENSHOT_MENU_DROP").is_some() && time.elapsed_secs() > 2.0 {
        commands.trigger(PointerDragDrop {
            entity: target,
            dropped: drag.source.unwrap(),
            pointer,
            button,
            hit,
        });
        drag.dropped = true;
    }
}

/// Ignores the first second and a half: window creation, pipeline
/// specialisation and the first shader compile all land there and say nothing
/// about what streaming costs.
const SETTLES_AFTER_S: f32 = 1.5;

fn watch_frames(
    time: Res<Time<Real>>,
    stats: Res<StreamingStats>,
    mut capture: ResMut<Capture>,
    mut cost: ResMut<MainThreadCost>,
) {
    let elapsed_s = capture.elapsed_s(&time);
    let fly_for_s = capture.fly_for_s;
    let frame_cost =
        if cfg!(feature = "profiling") && std::env::var_os("TESSERAE_PROFILE_CSV").is_some() {
            cost.frame()
        } else {
            cost.take_frame()
        };
    capture.frames.note_cost(elapsed_s, frame_cost, fly_for_s);
    capture.frames.note_shown(elapsed_s, stats.shown);
    capture.frames.note_levels(elapsed_s, &stats.levels);
    capture.frames.note_settled(elapsed_s, stats.settled);
    let may_fly = elapsed_s >= capture.fly_after_s;
    capture.frames.record(
        elapsed_s,
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
    time: Res<Time<Real>>,
    (mut switched, mut jumped): (Local<bool>, Local<bool>),
    mut intents: MessageWriter<crate::server::player::MovementIntent>,
    capture: Res<Capture>,
    camera: Single<(&Transform, &FlyCamera), With<MainCamera>>,
) {
    let elapsed_s = capture.elapsed_s(&time);
    let crouch = std::env::var_os("TESSERAE_SCREENSHOT_CROUCH").is_some();
    let switch_mode = !*switched
        && std::env::var_os("TESSERAE_SCREENSHOT_SWITCH_MODE").is_some()
        && capture.before_shutter(elapsed_s, 0.7);
    *switched |= switch_mode;
    // Timed so the shutter falls near the top of the jump.
    let jump = !*jumped
        && std::env::var_os("TESSERAE_SCREENSHOT_JUMP").is_some()
        && capture.before_shutter(elapsed_s, 0.3);
    *jumped |= jump;
    let sprint = std::env::var_os("TESSERAE_SCREENSHOT_SPRINT").is_some();
    // A walking or sprinting pose needs the body still moving at the shutter.
    let keep_moving = std::env::var_os("TESSERAE_SCREENSHOT_KEEP_MOVING").is_some()
        && capture
            .frames
            .fill_ended_s
            .is_some_and(|start| elapsed_s >= start);
    if !capture.frames.flying(elapsed_s, capture.fly_for_s) && !keep_moving {
        intents.write(crate::server::player::MovementIntent {
            crouch,
            switch_mode,
            jump,
            yaw: camera.1.yaw,
            pitch: camera.1.pitch,
            ..default()
        });
        return;
    }
    let (transform, fly) = camera.into_inner();
    let heading = transform.forward().with_y(0.0).normalize_or_zero();
    let speed = capture.speed_m_s.unwrap_or(fly.speed);
    let climb = climb_m_s(capture.lift_m, capture.fly_for_s, speed);
    let velocity = heading * speed + Vec3::Y * climb;
    intents.write(crate::server::player::MovementIntent {
        crouch,
        switch_mode,
        jump,
        sprint,
        direction: velocity,
        speed_mps: velocity.length(),
        yaw: fly.yaw,
        pitch: fly.pitch,
        ..default()
    });
}

/// The climb that adds `lift_m` over the flight, held to what the Server's speed
/// ceiling leaves after the horizontal speed: faster and the Server clamps the
/// Player back, so the picture is not from where it is said to be.
fn climb_m_s(lift_m: f32, fly_for_s: f32, horizontal_m_s: f32) -> f32 {
    let wanted = lift_m / fly_for_s.max(f32::EPSILON);
    let room = (FLY_SPEED_CEILING_MPS.powi(2) - horizontal_m_s.powi(2))
        .max(0.0)
        .sqrt();
    wanted.clamp(-room, room)
}

fn capture_holding(
    catalogue: Res<super::edit::Catalogue>,
    mut hotbar: ResMut<super::edit::Hotbar>,
    mut configured: Local<bool>,
) {
    if *configured {
        return;
    }
    let Ok(name) = std::env::var("TESSERAE_SCREENSHOT_MATERIAL") else {
        return;
    };
    if name == "empty" {
        hotbar.outside = Some(super::edit::Holding::Empty);
        *configured = true;
        return;
    }
    let Some(material) = catalogue
        .0
        .materials
        .iter()
        .find(|material| material.name == name)
    else {
        return;
    };
    let shape_name =
        std::env::var("TESSERAE_SCREENSHOT_SHAPE").unwrap_or_else(|_| "core:cube".into());
    let Some(shape) = catalogue
        .0
        .shapes
        .iter()
        .find(|shape| shape.name == shape_name)
    else {
        return;
    };
    let orientation = std::env::var("TESSERAE_SCREENSHOT_ORIENTATION")
        .ok()
        .and_then(|value| value.parse().ok())
        .and_then(tesserae_core::Orientation::from_bits)
        .unwrap_or(tesserae_core::Orientation::IDENTITY);
    hotbar.outside = Some(super::edit::Holding::Solid {
        material: material.id,
        shape: shape.id,
        orientation,
    });
    *configured = true;
}

fn aim(
    capture: Res<Capture>,
    camera: Single<(&mut Transform, &mut FlyCamera), With<MainCamera>>,
    mut aimed: Local<bool>,
) {
    if capture.pitch_deg.is_none() && capture.yaw_deg.is_none() {
        return;
    }
    if std::mem::replace(&mut *aimed, true) {
        return;
    }
    let (mut transform, mut fly) = camera.into_inner();
    if let Some(pitch_deg) = capture.pitch_deg {
        fly.pitch = pitch_deg.to_radians();
    }
    if let Some(yaw_deg) = capture.yaw_deg {
        fly.yaw = yaw_deg.to_radians();
    }
    transform.rotation = Quat::from_euler(EulerRot::YXZ, fly.yaw, fly.pitch, 0.0);
}

/// Logs what the picture is of, and the frame times behind it, as one line of
/// `key=value` pairs so `scripts/frame-runs` can aggregate several runs.
/// Runs before `shutter` rather than inside it, so neither function carries the
/// other's system parameters.
fn report(
    (time, capture): (Res<Time<Real>>, Res<Capture>),
    stats: Res<StreamingStats>,
    camera: Single<(&Transform, &FlyCamera), With<MainCamera>>,
    preferences: Res<super::profile::Preferences>,
    drawn: Query<(&MeshTag, &Transform, &Mesh3d)>,
    meshes: Res<Assets<Mesh>>,
    world: Res<WorldSettings>,
) {
    let (camera, movement) = camera.into_inner();
    let elapsed_s = capture.elapsed_s(&time);
    if !capture.due(elapsed_s) {
        return;
    }
    info!(
        "capture movement_mode={:?}, authority=Server fixed64, view={:?}, zoom_m={}, \
         eye={:?}, camera={:?}, quality=unchanged",
        movement.movement_mode,
        preferences.view,
        preferences.zoom_m,
        movement.eye,
        camera.translation,
    );
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
         mean_upload_us={:.0} uploads={} worst_sort_us={:.0} sorts={}",
        ms(f.worst_tick_s),
        ms(f.worst_cost_fill_s),
        ms(f.worst_cost_flight_s),
        f.upload_total_s * 1e6 / f.uploads.max(1) as f32,
        f.uploads,
        f.worst_sort_s * 1e6,
        f.sorts,
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
    // Measured from what is drawn, not from what the rings say.
    let eye = camera.translation;
    let mut reach_m: Vec<(f32, f32)> = Vec::new();
    let mut held_bytes: Vec<usize> = Vec::new();
    for (tag, at, mesh) in &drawn {
        let level = tag.value as usize;
        held_bytes.resize(held_bytes.len().max(level + 1), 0);
        held_bytes[level] += meshes.get(&mesh.0).map_or(0, mesh_bytes);
        let cell = world.cell_size_at(tag.value);
        let corners = [(0.0, 0.0), (cell, 0.0), (0.0, cell), (cell, cell)].map(|(dx, dz)| {
            Vec2::new(at.translation.x + dx - eye.x, at.translation.z + dz - eye.z).length()
        });
        reach_m.resize(reach_m.len().max(level + 1), (0.0, 0.0));
        let (near, far) = &mut reach_m[level];
        *near = near.max(corners.into_iter().fold(f32::MAX, f32::min));
        *far = far.max(corners.into_iter().fold(0.0, f32::max));
    }
    let reach: Vec<String> = reach_m
        .iter()
        .enumerate()
        .map(|(level, (near, far))| format!("{level}:{near:.0}-{far:.0}"))
        .collect();
    info!("capture_reach_m {}", reach.join(","));
    // A line of its own, so `scripts/frame-runs` keeps aggregating the one above.
    let per_level: Vec<String> = stats
        .levels
        .iter()
        .enumerate()
        .map(|(level, s)| format!("{level}:{}/{}", s.held, s.meshed))
        .collect();
    info!("capture_lod held/meshed={}", per_level.join(","));
    log_levels(&capture.frames, &stats.levels, &held_bytes);
}

fn mesh_bytes(mesh: &Mesh) -> usize {
    mesh.get_vertex_buffer_size() + mesh.get_index_buffer_bytes().map_or(0, <[u8]>::len)
}

/// One line per level and kind, so `scripts/frame-runs` still sees only the lines
/// it reads.
///
/// `held_bytes` is what the Client's own meshes add up to, level 0 included; the
/// per-level `bytes` is what the Server delivered, for levels above 0.
fn log_levels(frames: &FrameLog, levels: &[LevelStats], held_bytes: &[usize]) {
    let at = |times: &[Option<f32>], level: usize| {
        times
            .get(level)
            .copied()
            .flatten()
            .map_or("never".to_string(), |s| format!("{s:.2}"))
    };
    for (level, s) in levels.iter().enumerate() {
        info!(
            "capture_lod_fill level={level} held={} steady={} filled_s={} meshed={} shell={} \
             shown_s={}",
            s.held,
            s.steady,
            at(&frames.level_filled_s, level),
            s.meshed,
            s.shell,
            at(&frames.level_shown_s, level),
        );
        if level > 0 {
            info!(
                "capture_lod_cost level={level} non_uniform={} meshes={} non_empty={} quads={} \
                 bytes={} gen_us={:.0} gen_max_us={:.0} gen_jobs={} mesh_us={:.0} \
                 mesh_max_us={:.0} mesh_jobs={}",
                s.non_uniform,
                s.mesh.meshes,
                s.mesh.non_empty,
                s.mesh.quads,
                s.mesh.bytes,
                s.generate.mean_us,
                s.generate.longest_us,
                s.generate.jobs,
                s.mesh_job.mean_us,
                s.mesh_job.longest_us,
                s.mesh_job.jobs,
            );
        }
    }
    info!(
        "capture_lod_pool async_threads={}",
        AsyncComputeTaskPool::get().thread_num()
    );
    let per_level: Vec<String> = held_bytes
        .iter()
        .enumerate()
        .map(|(level, bytes)| format!("{level}:{bytes}"))
        .collect();
    info!(
        "capture_lod_memory total_bytes={} levels={}",
        held_bytes.iter().sum::<usize>(),
        per_level.join(",")
    );
}

fn shutter(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut capture: ResMut<Capture>,
    offscreen: Option<Res<Offscreen>>,
) {
    let elapsed_s = capture.elapsed_s(&time);
    if !capture.due(elapsed_s) {
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
    fn a_climb_is_held_to_what_the_speed_ceiling_leaves_after_the_horizontal() {
        assert_eq!(climb_m_s(60.0, 6.0, 12.0), 10.0);
        assert_eq!(climb_m_s(-60.0, 6.0, 12.0), -10.0);
        let climb = climb_m_s(6000.0, 6.0, 12.0);
        assert!(12.0f32.hypot(climb) <= FLY_SPEED_CEILING_MPS + 1e-3);
        assert_eq!(climb_m_s(100.0, 6.0, FLY_SPEED_CEILING_MPS), 0.0);
        assert_eq!(climb_m_s(0.0, 6.0, 12.0), 0.0);
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
    fn a_level_is_filled_and_shown_at_the_first_time_it_holds_what_it_will_keep() {
        let level = |held, meshed| LevelStats {
            held,
            meshed,
            steady: 10,
            shell: 4,
            ..default()
        };
        let mut log = FrameLog::default();
        log.note_levels(1.0, &[level(9, 3), level(0, 0)]);
        log.note_levels(2.0, &[level(10, 4), level(3, 0)]);
        // A level that drops below its count again keeps the time it first got there.
        log.note_levels(3.0, &[level(7, 2), level(10, 4)]);
        assert_eq!(log.level_filled_s, [Some(2.0), Some(3.0)]);
        assert_eq!(log.level_shown_s, [Some(2.0), Some(3.0)]);
    }

    #[test]
    fn a_level_with_nothing_to_hold_is_never_filled() {
        let mut log = FrameLog::default();
        log.note_levels(1.0, &[LevelStats::default()]);
        assert_eq!(log.level_filled_s, [None]);
        assert_eq!(log.level_shown_s, [None]);
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
