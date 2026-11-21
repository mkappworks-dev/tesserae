//! Bounded, opt-in measurements. Frame intervals end at the next First;
//! main schedule spans end at Last and exclude extraction and render work.
mod gpu;
mod monitor;
pub(crate) use monitor::window as monitor_window;
mod fill;
mod waits;

use crate::session::SessionApp;

use std::io::{BufWriter, Write};
use std::time::Instant;

use crate::client::holes::{Holes, LEVELS};
use crate::client::lag::Lag;
use crate::client::sight::Sight;
use crate::cost::{FrameCost, MainThreadCost, thread_cpu};
use crate::render::chunk_material::ChunkMaterial;
use crate::server::{
    ProfileFixtureStatus, SimulationProfile,
    streaming::{
        StreamingSettings,
        causes::{Cause, LodCause, SolveCause},
        stats::{Entered, Flow, Stages, StreamingStats},
    },
};
use crate::world::WorldSettings;
use bevy::camera::RenderTarget;
use bevy::prelude::*;

#[derive(Resource, Clone, Copy)]
pub struct ProcessStart(pub Instant);

const MAX_FRAMES: usize = 250_000;
/// One per level per fixed tick: about 25 minutes at 64 Hz with six levels.
const MAX_FLOW_ROWS: usize = 600_000;
/// Each of entries into the mesh extent and drawn changes: a 64 m/s flight
/// makes a few hundred a second.
const MAX_FILL_EVENTS: usize = 1_000_000;

#[derive(Resource)]
struct Profile {
    origin: Instant,
    start: Instant,
    cpu: Option<f64>,
    path: std::path::PathBuf,
    rows: Vec<Row>,
    pending: Option<Row>,
    flow: Vec<FlowRow>,
    /// `TESSERAE_PROFILE_LAG`: entries and drawn changes are kept.
    lag: bool,
    entered: Vec<Entered>,
    drawn: Vec<fill::Drawn>,
    /// Some entries or drawn changes were dropped at [`MAX_FILL_EVENTS`].
    fill_limited: bool,
    saved: bool,
    render: RenderSamples,
    gpu: gpu::GpuSamples,
    surfaces: SurfaceConfigurations,
    presentation: String,
}

/// Streaming work as the Server published it on one fixed tick, for one level.
struct FlowRow {
    frame_id: usize,
    t_s: f64,
    level: usize,
    held: usize,
    meshed: usize,
    flow: Flow,
    fluid_run: (u64, f64),
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
    holes: Holes,
    visible: Visible,
    sight: Sight,
    mesh_reach: Option<[(u64, f64); 2]>,
    /// The Client's `Lag::missing_m`, infinite when nothing is missing; `None`
    /// when not measured.
    missing_m: Option<f32>,
}

/// Each stage's cause columns: the index into its counts, and the column suffix.
fn cause_columns() -> Stages<Vec<(usize, &'static str)>> {
    let named = |names: &[&'static str]| names.iter().copied().enumerate().collect::<Vec<_>>();
    let mut generate = named(&LodCause::NAMES);
    // Generation is never for a move.
    generate.retain(|&(i, _)| i != LodCause::Moved.index());
    Stages {
        generate,
        relight: named(&SolveCause::NAMES),
        mesh: named(&LodCause::NAMES),
    }
}

/// Camera orders recorded: the Player's and one per coarse level.
const CAMERAS: usize = 8;

const CASCADES: usize = crate::client::chunk_view::SUN_CASCADE_BOUNDS.len();

/// What each view draws, counted from the visibility lists the render world
/// extracts.
#[derive(Resource, Default, Clone, Copy)]
struct Visible {
    /// Mesh entities each camera's visibility pass kept, by camera order.
    meshes: [u32; CAMERAS],
    /// Indirect draws each camera's opaque, cutout and blended passes issue,
    /// by camera order.
    draws: [u32; CAMERAS],
    /// Mesh entities each sun cascade of the Player camera holds.
    casters: [u32; CASCADES],
    /// Indirect draws each of those cascades issues.
    cascade_draws: [u32; CASCADES],
}

/// Client observations for the frame and export metadata.
#[derive(bevy::ecs::system::SystemParam)]
struct Drawn<'w, 's> {
    holes: Option<Res<'w, Holes>>,
    lag: Option<ResMut<'w, Lag>>,
    sight: Option<Res<'w, Sight>>,
    visible: Res<'w, Visible>,
    exit: MessageReader<'w, 's, AppExit>,
    monitors: Query<'w, 's, &'static bevy::window::Monitor>,
    on_monitor: Query<'w, 's, &'static bevy::window::OnMonitor, With<Window>>,
}

type DrawnBy<'a> = (
    &'a Mesh3d,
    Option<&'a MeshMaterial3d<ChunkMaterial>>,
    Option<&'a MeshMaterial3d<StandardMaterial>>,
);

fn count_visible(
    mut visible: ResMut<Visible>,
    cameras: Query<(Entity, &Camera, &bevy::camera::visibility::VisibleEntities)>,
    suns: Query<&bevy::camera::visibility::CascadesVisibleEntities>,
    meshes: Query<DrawnBy>,
    chunk_materials: Res<Assets<ChunkMaterial>>,
    standard_materials: Res<Assets<StandardMaterial>>,
) {
    *visible = Visible::default();
    let blended = |(_, chunk, standard): DrawnBy| {
        let mode = chunk
            .and_then(|m| chunk_materials.get(&m.0).map(|m| m.base.alpha_mode))
            .or_else(|| standard.and_then(|m| standard_materials.get(&m.0).map(|m| m.alpha_mode)));
        mode.is_some_and(|mode| mode == AlphaMode::Blend)
    };
    let draws = |entities: &[Entity]| -> u32 {
        let mut bins = bevy::platform::collections::HashSet::new();
        for &entity in entities {
            let Ok(drawn @ (mesh, chunk, standard)) = meshes.get(entity) else {
                continue;
            };
            // A sorted phase draws each blended mesh alone; the binned phases
            // draw each (mesh, material) once, a bin, however many share it.
            let alone = blended(drawn).then_some(entity);
            bins.insert((
                mesh.id(),
                chunk.map(|m| m.id().untyped()),
                standard.map(|m| m.id().untyped()),
                alone,
            ));
        }
        bins.len() as u32
    };
    for (entity, camera, entities) in &cameras {
        let Some(order) = usize::try_from(camera.order).ok().filter(|&o| o < CAMERAS) else {
            continue;
        };
        let listed = entities.get(std::any::TypeId::of::<Mesh3d>());
        visible.meshes[order] = listed.len() as u32;
        visible.draws[order] = draws(listed);
        if order != 0 {
            continue;
        }
        let cascades = suns
            .iter()
            .filter_map(|sun| sun.entities.get(&entity))
            .flatten();
        for (i, cascade) in cascades.take(CASCADES).enumerate() {
            visible.casters[i] = cascade.entities.len() as u32;
            visible.cascade_draws[i] = draws(&cascade.entities);
        }
    }
}

/// A diagnostic switch, read from the environment by the profiling runner's
/// `--switches`. Each changes one cost for attribution; none is a shipping option.
pub(crate) fn switch(name: &str) -> bool {
    std::env::var_os(name).is_some_and(|v| v != "0")
}

pub fn plugin(app: &mut App) {
    monitor::plugin(app);
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
    if std::env::var_os("TESSERAE_PROFILE_WAITS").is_some() {
        waits::plugin(app);
    }
    let gpu = gpu::GpuSamples::default();
    // Its marker passes cost more than the overhead bound allows a timing run.
    if std::env::var_os("TESSERAE_PROFILE_GPU").is_some() {
        gpu::plugin(app, gpu.clone());
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
        flow: Vec::new(),
        lag: crate::cost::measuring_lag(),
        entered: Vec::new(),
        drawn: Vec::new(),
        fill_limited: false,
        saved: false,
        render,
        gpu,
        surfaces,
        presentation: String::new(),
    })
    .insert_resource(bevy::winit::WinitSettings::continuous())
    .init_resource::<Visible>();
    // Frames are rows from the World session on; startup before it is in the
    // metadata.
    app.add_session_systems(First, begin.after(crate::client::presentation::FramePacing))
        .add_session_systems(Last, (count_visible, end).chain())
        .add_session_systems(
            FixedPostUpdate,
            sample_flow.after(crate::server::streaming::stats::Published),
        )
        .add_session_systems(Startup, open_session);
}

/// `begin` runs in `First`, before the state moves into the World, so a
/// reopened session's first frame would otherwise start at the previous
/// session's last and hide the reopen inside one frame.
fn open_session(mut profile: ResMut<Profile>) {
    if !profile.rows.is_empty() {
        profile.start = Instant::now();
        profile.cpu = thread_cpu();
    }
}

fn sample_flow(mut profile: ResMut<Profile>, streaming: Res<StreamingStats>) {
    if profile.saved {
        return;
    }
    if profile.lag {
        if profile.entered.len() + streaming.entered.len() <= MAX_FILL_EVENTS {
            profile.entered.extend_from_slice(&streaming.entered);
        } else {
            profile.fill_limited = true;
        }
    }
    if profile.flow.len() + streaming.levels.len() > MAX_FLOW_ROWS {
        return;
    }
    let frame_id = profile.rows.len();
    let t_s = profile.origin.elapsed().as_secs_f64();
    for (level, stats) in streaming.levels.iter().enumerate() {
        profile.flow.push(FlowRow {
            frame_id,
            t_s,
            level,
            held: stats.held,
            meshed: stats.meshed,
            flow: stats.flow,
            fluid_run: streaming.fluid_run,
        });
    }
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
    mut drawn: Drawn,
) {
    if profile.saved {
        return;
    }
    profile.presentation.clear();
    if let Some(preferences) = preferences {
        profile.presentation = format!(
            "selected_display_sync={} selected_fps_cap={:?} anti_aliasing={:?} render_scale={} sharpen={} capture_override={}",
            preferences.display_sync,
            preferences.fps_cap,
            crate::client::anti_alias::chosen(&preferences),
            crate::client::render_scale::percent(&preferences),
            crate::client::render_scale::sharpen(),
            std::env::var_os("TESSERAE_SCREENSHOT").is_some()
        );
    }
    for on in &drawn.on_monitor {
        if let Ok(monitor) = drawn.monitors.get(on.0) {
            profile.presentation.push_str(&format!(
                "\nmonitor name={:?} physical={}x{} reported_refresh_millihertz={:?}",
                monitor.name,
                monitor.physical_width,
                monitor.physical_height,
                monitor.refresh_rate_millihertz
            ));
        }
    }
    let wall_s = profile.start.elapsed().as_secs_f64();
    let cpu_s = profile.cpu.zip(thread_cpu()).map(|(a, b)| b - a);
    let missing_m = drawn.lag.as_mut().map(|lag| {
        let changed = std::mem::take(&mut lag.changed);
        if profile.drawn.len() + changed.len() <= MAX_FILL_EVENTS {
            profile.drawn.extend(changed);
        } else {
            profile.fill_limited = true;
        }
        lag.missing_m.unwrap_or(f32::INFINITY)
    });
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
        holes: drawn.holes.as_deref().copied().unwrap_or_default(),
        visible: *drawn.visible,
        sight: drawn.sight.as_deref().copied().unwrap_or_default(),
        mesh_reach: streaming.mesh_reach,
        missing_m,
    });
    if profile.rows.len() >= MAX_FRAMES || drawn.exit.read().next().is_some() {
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
        "frame_id,start_s,end_s,phase,frame_ms,main_wall_ms,main_cpu_ms,server_wall_ms,longest_tick_ms,upload_wall_ms,uploads,sort_wall_ms,sorts,streaming_shown,streaming_settled,simulation_step,simulation_idle,simulation_running,awaiting,suspended,timers,landed,deferred,focused,fixture_ready,fixture_ignitions,fixture_fire_burning,fixture_moves,save_snapshots,save_snapshot_ms,save_worker_ms,save_chunks,rule_holes,fill_holes{}",
        (0..LEVELS)
            .map(|l| format!(",rule_holes_l{l},fill_holes_l{l}"))
            .chain(std::iter::once(",chunk_entities".to_string()))
            .chain((0..CAMERAS).map(|c| format!(",visible_meshes_camera{c}")))
            .chain(std::iter::once(",rescore_ms".to_string()))
            .chain(std::iter::once(
                ",sight_drawn0,sight_culled0,sight_retained_hidden0,mesh_reach_run,mesh_reach_cpu_ms,mesh_unreach_run,mesh_unreach_cpu_ms,level0_missing_m".to_string()
            ))
            .chain((0..CAMERAS).map(|c| format!(",draws_camera{c}")))
            .chain((0..CASCADES).map(|i| format!(",shadow_casters_cascade{i},shadow_draws_cascade{i}")))
            .collect::<String>()
    )?;
    for r in &profile.rows {
        let phase = phase(r.start_s);
        write!(
            out,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
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
            r.fixture.moves,
            r.save.snapshots,
            r.save.snapshot_ms,
            r.save.worker_ms,
            r.save.chunks
        )?;
        if r.holes.counted {
            write!(
                out,
                ",{},{}",
                r.holes.rule.iter().sum::<u32>(),
                r.holes.fill.iter().sum::<u32>()
            )?;
            for l in 0..LEVELS {
                write!(out, ",{},{}", r.holes.rule[l], r.holes.fill[l])?;
            }
        } else {
            write!(out, "{}", ",".repeat(2 + 2 * LEVELS))?;
        }
        write!(out, ",{}", r.holes.chunk_entities)?;
        for count in r.visible.meshes {
            write!(out, ",{count}")?;
        }
        write!(out, ",{}", r.cost.rescore_s * 1e3)?;
        if r.sight.counted {
            let s = r.sight;
            write!(out, ",{},{},{}", s.drawn, s.culled, s.retained_hidden)?;
        } else {
            write!(out, ",,,")?;
        }
        match r.mesh_reach {
            Some([(a, b), (c, d)]) => write!(out, ",{a},{b},{c},{d}")?,
            None => write!(out, ",,,,")?,
        }
        match r.missing_m {
            Some(m) => write!(out, ",{m}")?,
            None => write!(out, ",")?,
        }
        for count in r.visible.draws {
            write!(out, ",{count}")?;
        }
        for (casters, draws) in r.visible.casters.iter().zip(r.visible.cascade_draws) {
            write!(out, ",{casters},{draws}")?;
        }
        writeln!(out)?;
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
        "schema=1\nseed={}\nvoxel_size_m={}\nnear_m={}\nlod_m={}\nprofiling=true\ndev={}\nclock=Instant monotonic; main CPU=thread clock\nmain_span=First to Last; excludes extract/render\nextract=unavailable (no whole-extraction boundary)\nrender_thread_cpu=render.csv thread clock\ngpu_elapsed={}\npresent_elapsed=unavailable\nbackend_resolved_present_mode=see observed backend_surface_configuration records; unavailable when no surface is configured or logging is filtered\nmax_frames={}\nlimit_reached={}\nincomplete_final_interval=excluded\nstartup_before_first_First_s={}\nmovement=Server fixed-step intent fixture; actual mode in Player spawned log\nview=first person\nquality=unchanged; no profiling quality switches",
        settings.seed,
        settings.voxel_size,
        view.view_distance_m,
        view.lod_distance_m,
        cfg!(feature = "dev"),
        if std::env::var_os("TESSERAE_PROFILE_GPU").is_none() {
            "off (TESSERAE_PROFILE_GPU unset)"
        } else if profile.gpu.1.load(std::sync::atomic::Ordering::Relaxed) {
            "gpu.csv: timestamps at empty compute pass boundaries between Bevy's passes"
        } else {
            "unavailable (no TIMESTAMP_QUERY)"
        },
        MAX_FRAMES,
        profile.rows.len() >= MAX_FRAMES,
        profile.rows.first().map_or(0.0, |r| r.start_s)
    )?;
    writeln!(
        meta,
        "movement_requested={}",
        std::env::var("TESSERAE_FIXTURE_MOVEMENT").unwrap_or_else(|_| "flight".into())
    )?;
    writeln!(meta, "{}", profile.presentation)?;
    writeln!(
        meta,
        "wait_intervals={}; enclosing CPU wall intervals, not GPU elapsed or display latency",
        std::env::var_os("TESSERAE_PROFILE_WAITS").is_some()
    )?;
    for configuration in profile.surfaces.0.lock().unwrap().iter() {
        writeln!(meta, "backend_surface_configuration={configuration}")?;
    }
    for w in windows.iter() {
        writeln!(
            meta,
            "window={}x{} scale={} requested_present={:?} focused={} mode={:?}",
            w.physical_width(),
            w.physical_height(),
            w.scale_factor(),
            w.present_mode,
            w.focused,
            w.mode
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
        "frame_id,render_schedule_wall_ms,render_thread_cpu_ms,window_prepare_interval_ms,render_graph_interval_ms,render_finalize_interval_ms"
    )?;
    for (id, wall, cpu, waits) in profile.render.0.lock().unwrap().iter() {
        write!(
            render,
            "{id},{},{}",
            wall * 1e3,
            cpu.map(|s| (s * 1e3).to_string()).unwrap_or_default()
        )?;
        for value in waits {
            write!(
                render,
                ",{}",
                value.map(|s| (s * 1e3).to_string()).unwrap_or_default()
            )?;
        }
        writeln!(render)?;
    }
    render.flush()?;
    let mut flow = BufWriter::new(
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(profile.path.with_extension("flow.csv"))?,
    );
    write!(flow, "frame_id,t_s,level,held,meshed")?;
    let columns = cause_columns();
    for (stage, _) in columns.named() {
        write!(
            flow,
            ",{stage}_waiting,{stage}_in_flight,{stage}_run,{stage}_cpu_ms"
        )?;
    }
    write!(flow, ",mesh_empty,fluid_run,fluid_cpu_ms")?;
    for (stage, names) in columns.named() {
        for (_, name) in names {
            write!(flow, ",{stage}_{name}")?;
        }
    }
    writeln!(
        flow,
        ",children_generated,children_in_load,jobs_in_load,limit_reached"
    )?;
    let limited = profile.flow.len() + 32 > MAX_FLOW_ROWS;
    for r in &profile.flow {
        write!(
            flow,
            "{},{},{},{},{}",
            r.frame_id, r.t_s, r.level, r.held, r.meshed
        )?;
        for (_, s) in r.flow.stages.named() {
            write!(
                flow,
                ",{},{},{},{}",
                s.waiting, s.in_flight, s.run, s.cpu_ms
            )?;
        }
        write!(
            flow,
            ",{},{},{}",
            r.flow.empty_meshes, r.fluid_run.0, r.fluid_run.1
        )?;
        for ((_, s), (_, names)) in r.flow.stages.named().into_iter().zip(columns.named()) {
            for &(i, _) in names {
                write!(flow, ",{}", s.causes[i])?;
            }
        }
        let c = r.flow.children;
        writeln!(
            flow,
            ",{},{},{},{limited}",
            c.generated, c.in_load, c.jobs_in_load
        )?;
    }
    flow.flush()?;
    if profile.lag {
        save_fill(profile)?;
    }
    if std::env::var_os("TESSERAE_PROFILE_GPU").is_none() {
        return Ok(());
    }
    let mut gpu = BufWriter::new(
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(profile.path.with_extension("gpu.csv"))?,
    );
    writeln!(gpu, "frame_id,marker,since_frame_start_ms")?;
    for (frame, marker, ms) in profile.gpu.0.lock().unwrap().iter() {
        let ms = ms.map(|ms| ms.to_string()).unwrap_or_default();
        writeln!(gpu, "{frame},{marker},{ms}")?;
    }
    gpu.flush()
}

/// Each level-0 Chunk's entry into the mesh extent, and how long after it the
/// Client drew it, empty if it never did.
fn save_fill(profile: &Profile) -> std::io::Result<()> {
    let mut fill = BufWriter::new(
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(profile.path.with_extension("fill.csv"))?,
    );
    writeln!(fill, "t_s,x,y,z,distance_m,drawn_after_s,limit_reached")?;
    let limited = profile.fill_limited;
    let after = fill::drawn_after(&profile.entered, &profile.drawn);
    for (entry, after) in profile.entered.iter().zip(after) {
        let t_s = entry
            .at
            .saturating_duration_since(profile.origin)
            .as_secs_f64();
        let c = entry.coord;
        let after = after.map(|d| d.as_secs_f64().to_string());
        writeln!(
            fill,
            "{t_s},{},{},{},{},{},{limited}",
            c.x,
            c.y,
            c.z,
            entry.distance_m,
            after.unwrap_or_default()
        )?;
    }
    fill.flush()
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
            flow: Vec::new(),
            lag: false,
            entered: Vec::new(),
            drawn: Vec::new(),
            fill_limited: false,
            saved: false,
            render: RenderSamples::default(),
            gpu: gpu::GpuSamples::default(),
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
                holes: Holes::default(),
                visible: Visible::default(),
                sight: Sight::default(),
                mesh_reach: None,
                missing_m: None,
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
    fn a_reopened_session_starts_its_first_frame_when_it_opens() {
        let mut app = App::new();
        let long_ago = Instant::now() - std::time::Duration::from_secs(5);
        let row = Row {
            id: 0,
            start_s: 0.0,
            interval_s: 0.01,
            wall_s: 0.01,
            cpu_s: None,
            cost: FrameCost::default(),
            sim: SimulationProfile::default(),
            save: crate::server::persistence::SaveProfile::default(),
            fixture: ProfileFixtureStatus::default(),
            shown: false,
            settled: false,
            focused: false,
            holes: Holes::default(),
            visible: Visible::default(),
            sight: Sight::default(),
            mesh_reach: None,
            missing_m: None,
        };
        app.insert_resource(Profile {
            origin: long_ago,
            start: long_ago,
            cpu: None,
            path: std::path::PathBuf::new(),
            rows: Vec::new(),
            flow: Vec::new(),
            lag: false,
            entered: Vec::new(),
            drawn: Vec::new(),
            fill_limited: false,
            saved: false,
            render: RenderSamples::default(),
            gpu: gpu::GpuSamples::default(),
            surfaces: SurfaceConfigurations::default(),
            presentation: String::new(),
            pending: None,
        });
        app.add_systems(Update, open_session);
        app.update();
        // The first session's first frame keeps the startup stall.
        assert_eq!(app.world().resource::<Profile>().start, long_ago);
        app.world_mut().resource_mut::<Profile>().rows.push(row);
        app.update();
        assert!(
            app.world()
                .resource::<Profile>()
                .start
                .elapsed()
                .as_secs_f64()
                < 1.0
        );
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
type RenderRow = (usize, f64, Option<f64>, [Option<f64>; 3]);
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
    if let Some(mut intervals) = world.get_resource_mut::<waits::Intervals>() {
        intervals.reset();
    }
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
            let waits = world
                .get_resource::<waits::Intervals>()
                .map_or([None; 3], |intervals| intervals.seconds());
            rows.push((id, wall, cpu, waits));
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
