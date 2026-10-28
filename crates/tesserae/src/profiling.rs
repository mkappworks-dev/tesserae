//! Bounded, opt-in measurements. Frame intervals end at the next First;
//! main schedule spans end at Last and exclude extraction and render work.
use std::io::{BufWriter, Write};
use std::time::Instant;

use crate::cost::{FrameCost, MainThreadCost};
use crate::server::{
    ProfileFixtureStatus, SimulationProfile,
    streaming::{StreamingSettings, stats::StreamingStats},
};
use crate::world::WorldSettings;
use bevy::camera::RenderTarget;
use bevy::prelude::*;

#[derive(Resource, Clone, Copy)]
pub struct ProcessStart(pub Instant);

const MAX_FRAMES: usize = 250_000;

#[derive(Resource)]
struct Profile {
    origin: Instant,
    start: Instant,
    cpu: Option<f64>,
    path: std::path::PathBuf,
    rows: Vec<Row>,
    pending: Option<Row>,
    saved: bool,
    render: RenderSamples,
    surfaces: SurfaceConfigurations,
    presentation: String,
}

struct Row {
    id: usize,
    start_s: f64,
    interval_s: f64,
    wall_s: f64,
    cpu_s: Option<f64>,
    cost: FrameCost,
    sim: SimulationProfile,
    save: crate::server::persistence::SaveProfile,
    fixture: ProfileFixtureStatus,
    shown: bool,
    settled: bool,
    focused: bool,
}

pub fn plugin(app: &mut App) {
    if std::env::var_os("TESSERAE_PROFILE_CONTINUOUS").is_some() {
        app.insert_resource(bevy::winit::WinitSettings::continuous());
    }
    let Some(path) = std::env::var_os("TESSERAE_PROFILE_CSV") else {
        return;
    };
    let now = Instant::now();
    let render = RenderSamples::default();
    if let Some(render_app) = app.get_sub_app_mut(bevy::render::RenderApp) {
        use bevy::render::{ExtractSchedule, Render, RenderSystems};
        render_app
            .insert_resource(render.clone())
            .init_resource::<RenderSpan>()
            .add_systems(ExtractSchedule, extract_frame)
            .add_systems(Render, render_begin.before(RenderSystems::ExtractCommands))
            .add_systems(Render, render_end.after(RenderSystems::PostCleanup));
    }
    app.init_resource::<ProfileFixtureStatus>();
    app.insert_non_send(std::rc::Rc::new(()));
    let surfaces = app
        .world()
        .get_resource::<SurfaceConfigurations>()
        .cloned()
        .unwrap_or_default();
    app.insert_resource(Profile {
        origin: app
            .world()
            .get_resource::<ProcessStart>()
            .map_or(now, |start| start.0),
        start: now,
        cpu: None,
        path: path.into(),
        rows: Vec::with_capacity(MAX_FRAMES),
        pending: None,
        saved: false,
        render,
        surfaces,
        presentation: String::new(),
    })
    .insert_resource(bevy::winit::WinitSettings::continuous())
    .add_systems(First, begin.after(crate::client::presentation::FramePacing))
    .add_systems(Last, end);
}

fn begin(mut profile: ResMut<Profile>, _thread: NonSend<std::rc::Rc<()>>) {
    if profile.saved {
        return;
    }
    let now = Instant::now();
    if let Some(mut row) = profile.pending.take() {
        row.interval_s = now.duration_since(profile.start).as_secs_f64();
        profile.rows.push(row);
    }
    profile.start = now;
    profile.cpu = thread_cpu();
}

fn end(
    mut profile: ResMut<Profile>,
    _thread: NonSend<std::rc::Rc<()>>,
    mut cost: ResMut<MainThreadCost>,
    (sim, streaming, fixture, save_profile): (
        Res<SimulationProfile>,
        Res<StreamingStats>,
        Res<ProfileFixtureStatus>,
        Res<crate::server::persistence::SaveProfile>,
    ),
    (settings, view, preferences): (
        Res<WorldSettings>,
        Res<StreamingSettings>,
        Option<Res<crate::client::profile::Preferences>>,
    ),
    (windows, cameras): (Query<&Window>, Query<(&Camera, &RenderTarget)>),
    mut exit: MessageReader<AppExit>,
) {
    if profile.saved {
        return;
    }
    if let Some(preferences) = preferences {
        profile.presentation = format!(
            "selected_display_sync={} selected_fps_cap={:?} capture_override={}",
            preferences.display_sync,
            preferences.fps_cap,
            std::env::var_os("TESSERAE_SCREENSHOT").is_some()
        );
    }
    let wall_s = profile.start.elapsed().as_secs_f64();
    let cpu_s = profile.cpu.zip(thread_cpu()).map(|(a, b)| b - a);
    profile.pending = Some(Row {
        id: profile.rows.len(),
        start_s: profile.start.duration_since(profile.origin).as_secs_f64(),
        interval_s: 0.0,
        wall_s,
        cpu_s,
        cost: cost.take_frame(),
        sim: *sim,
        save: *save_profile,
        fixture: *fixture,
        shown: streaming.shown,
        settled: streaming.settled,
        focused: windows.iter().any(|w| w.focused),
    });
    if profile.rows.len() >= MAX_FRAMES || exit.read().next().is_some() {
        // The incomplete final interval is excluded, never replaced with virtual time.
        profile.saved = true;
        if let Err(error) = save(&profile, &settings, &view, &windows, &cameras) {
            panic!("Cannot save frame profile: {error}");
        }
    }
}

fn save(
    profile: &Profile,
    settings: &WorldSettings,
    view: &StreamingSettings,
    windows: &Query<&Window>,
    cameras: &Query<(&Camera, &RenderTarget)>,
) -> std::io::Result<()> {
    let mut out = BufWriter::new(
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&profile.path)?,
    );
    writeln!(
        out,
        "frame_id,start_s,end_s,phase,frame_ms,main_wall_ms,main_cpu_ms,server_wall_ms,longest_tick_ms,upload_wall_ms,uploads,sort_wall_ms,sorts,streaming_shown,streaming_settled,simulation_step,simulation_idle,simulation_running,awaiting,suspended,timers,landed,deferred,focused,fixture_ready,fixture_ignitions,fixture_fire_burning,save_snapshots,save_snapshot_ms,save_worker_ms,save_chunks"
    )?;
    for r in &profile.rows {
        let phase = phase(r.start_s);
        writeln!(
            out,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            r.id,
            r.start_s,
            r.start_s + r.interval_s,
            phase,
            r.interval_s * 1e3,
            r.wall_s * 1e3,
            r.cpu_s.map(|s| (s * 1e3).to_string()).unwrap_or_default(),
            r.cost.tick_s * 1e3,
            r.cost.longest_tick_s * 1e3,
            r.cost.upload_s * 1e3,
            r.cost.uploads,
            r.cost.sort_s * 1e3,
            r.cost.sorts,
            r.shown,
            r.settled,
            r.sim.step,
            r.sim.idle,
            r.sim.running,
            r.sim.awaiting,
            r.sim.suspended,
            r.sim.timers,
            r.sim.landed,
            r.sim.deferred,
            r.focused,
            r.fixture.ready,
            r.fixture.ignitions,
            r.fixture.fire_burning,
            r.save.snapshots,
            r.save.snapshot_ms,
            r.save.worker_ms,
            r.save.chunks
        )?;
    }
    out.flush()?;
    let mut meta = BufWriter::new(
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(profile.path.with_extension("metadata.txt"))?,
    );
    writeln!(
        meta,
        "schema=1\nseed={}\nvoxel_size_m={}\nnear_m={}\nlod_m={}\nprofiling=true\ndev={}\nclock=Instant monotonic; main CPU=thread clock\nmain_span=First to Last; excludes extract/render\nextract=unavailable (no whole-extraction boundary)\nrender_thread_cpu=render.csv thread clock\ngpu_elapsed=unavailable\npresent_elapsed=unavailable\nbackend_resolved_present_mode=see observed backend_surface_configuration records; unavailable when no surface is configured or logging is filtered\nmax_frames={}\nlimit_reached={}\nincomplete_final_interval=excluded\nstartup_before_first_First_s={}\nmovement=fixture capture free flight; virtual movement semantics\nview=first person\nquality=unchanged; no profiling quality switches",
        settings.seed,
        settings.voxel_size,
        view.view_distance_m,
        view.lod_distance_m,
        cfg!(feature = "dev"),
        MAX_FRAMES,
        profile.rows.len() >= MAX_FRAMES,
        profile.rows.first().map_or(0.0, |r| r.start_s)
    )?;
    writeln!(meta, "{}", profile.presentation)?;
    for configuration in profile.surfaces.0.lock().unwrap().iter() {
        writeln!(meta, "backend_surface_configuration={configuration}")?;
    }
    for w in windows.iter() {
        writeln!(
            meta,
            "window={}x{} scale={} requested_present={:?} focused={}",
            w.physical_width(),
            w.physical_height(),
            w.scale_factor(),
            w.present_mode,
            w.focused
        )?;
    }
    for (camera, target) in cameras.iter() {
        writeln!(
            meta,
            "camera active={} physical_viewport={:?} target={target:?}",
            camera.is_active,
            camera.physical_viewport_size()
        )?;
    }
    meta.flush()?;
    let mut render = BufWriter::new(
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(profile.path.with_extension("render.csv"))?,
    );
    writeln!(
        render,
        "frame_id,render_schedule_wall_ms,render_thread_cpu_ms"
    )?;
    for (id, wall, cpu) in profile.render.0.lock().unwrap().iter() {
        writeln!(
            render,
            "{id},{},{}",
            wall * 1e3,
            cpu.map(|s| (s * 1e3).to_string()).unwrap_or_default()
        )?;
    }
    render.flush()
}

fn phase(seconds: f64) -> &'static str {
    match seconds {
        s if s < 16.0 => "startup",
        s if s < 20.0 => "stationary",
        s if s < 22.0 => "transition",
        s if s < 32.0 => "fixture",
        _ => "settle",
    }
}

#[cfg(unix)]
fn thread_cpu() -> Option<f64> {
    let mut time = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // The thread clock excludes waits and worker CPU; failure remains unavailable.
    let result = unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut time) };
    (result == 0).then_some(time.tv_sec as f64 + time.tv_nsec as f64 * 1e-9)
}
#[cfg(not(unix))]
fn thread_cpu() -> Option<f64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn surface_metadata_observes_backend_configuration_events() {
        use bevy::log::tracing_subscriber::prelude::*;
        let configurations = SurfaceConfigurations::default();
        let subscriber =
            bevy::log::tracing_subscriber::registry().with(SurfaceLayer(configurations.clone()));
        bevy::log::tracing::subscriber::with_default(subscriber, || {
            bevy::log::tracing::debug!(target: "wgpu_core::device::resource", "configuring surface with SurfaceConfiguration {{ width: 1920, height: 1080, present_mode: Immediate }}");
            bevy::log::tracing::debug!(target: "other", "configuring surface with ignored");
        });
        let records = configurations.0.lock().unwrap();
        assert_eq!(records.len(), 1);
        assert!(records[0].contains("present_mode: Immediate"));
    }

    #[test]
    fn completed_intervals_retain_stalls_and_frame_identity() {
        let mut app = App::new();
        let now = Instant::now();
        app.insert_non_send(std::rc::Rc::new(()));
        app.insert_resource(Profile {
            origin: now - std::time::Duration::from_secs(2),
            start: now - std::time::Duration::from_millis(750),
            cpu: None,
            path: std::path::PathBuf::new(),
            rows: Vec::new(),
            saved: false,
            render: RenderSamples::default(),
            surfaces: SurfaceConfigurations::default(),
            presentation: String::new(),
            pending: Some(Row {
                id: 0,
                start_s: 1.25,
                interval_s: 0.0,
                wall_s: 0.5,
                cpu_s: Some(0.01),
                cost: FrameCost::default(),
                sim: SimulationProfile::default(),
                save: crate::server::persistence::SaveProfile::default(),
                fixture: ProfileFixtureStatus::default(),
                shown: false,
                settled: false,
                focused: false,
            }),
        });
        app.add_systems(First, begin.after(crate::client::presentation::FramePacing));
        app.update();
        let profile = app.world().resource::<Profile>();
        assert_eq!(profile.rows.len(), 1);
        assert_eq!(profile.rows[0].id, 0);
        assert!(profile.rows[0].interval_s >= 0.75);
        assert_eq!(profile.rows[0].cpu_s, Some(0.01));
        assert_eq!(profile.rows[0].start_s, 1.25);
    }

    #[test]
    fn disabled_profiler_installs_no_buffer() {
        const CHILD: &str = "TESSERAE_PROFILE_DISABLED_TEST";
        if std::env::var_os(CHILD).is_some() {
            let mut app = App::new();
            app.add_plugins(plugin);
            assert!(!app.world().contains_resource::<Profile>());
            assert!(!app.world().contains_resource::<RenderSamples>());
            app.update();
            return;
        }
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "profiling::tests::disabled_profiler_installs_no_buffer",
            ])
            .env_remove("TESSERAE_PROFILE_CSV")
            .env_remove("TESSERAE_PROFILE_CONTINUOUS")
            .env(CHILD, "1")
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }

    #[test]
    fn phase_boundaries_are_explicit_and_unclamped() {
        assert_eq!(phase(0.7), "startup");
        assert_eq!(phase(16.0), "stationary");
        assert_eq!(phase(20.0), "transition");
        assert_eq!(phase(22.0), "fixture");
        assert_eq!(phase(32.0), "settle");
    }
}

#[derive(Resource, Default, Clone)]
struct RenderSamples(std::sync::Arc<std::sync::Mutex<Vec<RenderRow>>>);
type RenderRow = (usize, f64, Option<f64>);
#[derive(Resource, Default)]
struct RenderSpan {
    id: Option<usize>,
    start: Option<Instant>,
    cpu: Option<f64>,
}
fn extract_frame(mut span: ResMut<RenderSpan>, profile: bevy::render::Extract<Res<Profile>>) {
    span.id = profile.pending.as_ref().map(|row| row.id);
}
fn render_begin(world: &mut World) {
    let mut span = world.resource_mut::<RenderSpan>();
    span.start = Some(Instant::now());
    span.cpu = thread_cpu();
}
fn render_end(world: &mut World) {
    let span = world.resource::<RenderSpan>();
    if let (Some(id), Some(start)) = (span.id, span.start) {
        let wall = start.elapsed().as_secs_f64();
        let cpu = span.cpu.zip(thread_cpu()).map(|(a, b)| b - a);
        let samples = world.resource::<RenderSamples>();
        let mut rows = samples.0.lock().unwrap();
        if rows.len() < MAX_FRAMES {
            rows.push((id, wall, cpu));
        }
    }
}

#[derive(Resource, Clone)]
struct SurfaceConfigurations(std::sync::Arc<std::sync::Mutex<Vec<String>>>, Instant);
impl Default for SurfaceConfigurations {
    fn default() -> Self {
        Self(default(), Instant::now())
    }
}
struct SurfaceLayer(SurfaceConfigurations);
impl<S: bevy::log::tracing::Subscriber> bevy::log::tracing_subscriber::Layer<S> for SurfaceLayer {
    fn on_event(
        &self,
        event: &bevy::log::tracing::Event<'_>,
        _context: bevy::log::tracing_subscriber::layer::Context<'_, S>,
    ) {
        let target = event.metadata().target();
        if target != "wgpu_core::device::resource" && target != "log" {
            return;
        }
        struct Message {
            text: String,
            target: bool,
        }
        impl bevy::log::tracing::field::Visit for Message {
            fn record_str(&mut self, field: &bevy::log::tracing::field::Field, value: &str) {
                if field.name() == "log.target" {
                    self.target = value == "wgpu_core::device::resource";
                }
                if field.name() == "message" {
                    self.text = value.to_owned();
                }
            }
            fn record_debug(
                &mut self,
                field: &bevy::log::tracing::field::Field,
                value: &dyn std::fmt::Debug,
            ) {
                if field.name() == "message" {
                    self.text = format!("{value:?}");
                }
            }
        }
        let mut message = Message {
            text: String::new(),
            target: target == "wgpu_core::device::resource",
        };
        event.record(&mut message);
        if message.target && message.text.starts_with("configuring surface with ") {
            let mut configurations = self.0.0.lock().unwrap();
            if configurations.len() < 64 {
                configurations.push(format!(
                    "observer_elapsed_s={} {}",
                    self.0.1.elapsed().as_secs_f64(),
                    message.text
                ));
            }
        }
    }
}
pub(crate) fn surface_log_layer(app: &mut App) -> Option<bevy::log::BoxedLayer> {
    std::env::var_os("TESSERAE_PROFILE_CSV")?;
    let configurations = SurfaceConfigurations::default();
    app.insert_resource(configurations.clone());
    Some(Box::new(SurfaceLayer(configurations)))
}
