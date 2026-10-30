//! How this process was started and what it hands over to the next one. Each
//! process opens at most one World; switching re-executes the binary, so no
//! job or answer from one World can land in another.

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use bevy::ecs::system::NonSendMarker;
use bevy::prelude::*;
use bevy::window::{MonitorSelection, PrimaryWindow, WindowMode, WindowPosition, WindowResolution};
use tesserae_core::save::library::GameRef;

const WORLD_ARG: &str = "--world";
const WINDOW_ARG: &str = "--window";

/// Variables that open a World directly, for fixtures and scripted captures.
const DIRECT: [&str; 4] = [
    "TESSERAE_WORLD_DIR",
    "TESSERAE_EPHEMERAL",
    "TESSERAE_PROFILE_FIXTURE",
    "TESSERAE_SCREENSHOT",
];

#[derive(Clone, Debug, PartialEq)]
pub enum Launch {
    /// Opened from the environment; a failure prints and exits.
    Direct,
    List,
    World(PathBuf),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Args {
    pub launch: Launch,
    pub window: Option<Geometry>,
    /// Everything else, forwarded unchanged to Mod loading and to the next process.
    pub user: Vec<OsString>,
}

pub fn parse(
    args: impl IntoIterator<Item = OsString>,
    set: impl Fn(&str) -> bool,
) -> Result<Args, String> {
    let mut world = None;
    let mut window = None;
    let mut user = Vec::new();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if arg == WORLD_ARG {
            world = Some(PathBuf::from(
                args.next().ok_or("--world needs a World folder")?,
            ));
        } else if arg == WINDOW_ARG {
            let value = args.next().ok_or("--window needs a geometry")?;
            window = Geometry::parse(&value.to_string_lossy());
        } else {
            user.push(arg);
        }
    }
    let launch = match world {
        Some(path) => Launch::World(path),
        None if DIRECT.iter().any(|name| set(name)) => Launch::Direct,
        None => Launch::List,
    };
    Ok(Args {
        launch,
        window,
        user,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Windowed,
    Borderless,
    Fullscreen,
}

/// Logical size and physical position; `None` centres the next window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Geometry {
    pub position: Option<IVec2>,
    pub width: u32,
    pub height: u32,
    pub mode: Mode,
}

impl Geometry {
    pub fn arg(&self) -> String {
        let mode = match self.mode {
            Mode::Windowed => "windowed",
            Mode::Borderless => "borderless",
            Mode::Fullscreen => "fullscreen",
        };
        let at = self
            .position
            .map_or_else(|| "auto,auto".into(), |p| format!("{},{}", p.x, p.y));
        format!("{at},{},{},{mode}", self.width, self.height)
    }

    /// `None` for anything malformed: a bad geometry falls back to the default window.
    pub fn parse(text: &str) -> Option<Self> {
        let mut parts = text.split(',');
        let mut next = || parts.next();
        let position = match (next()?, next()?) {
            ("auto", "auto") => None,
            (x, y) => Some(IVec2::new(x.parse().ok()?, y.parse().ok()?)),
        };
        let geometry = Self {
            position,
            width: next()?.parse().ok().filter(|&w| w > 0)?,
            height: next()?.parse().ok().filter(|&h| h > 0)?,
            mode: match next()? {
                "windowed" => Mode::Windowed,
                "borderless" => Mode::Borderless,
                "fullscreen" => Mode::Fullscreen,
                _ => return None,
            },
        };
        next().is_none().then_some(geometry)
    }

    pub fn of(window: &Window) -> Self {
        Self {
            position: match window.position {
                WindowPosition::At(at) => Some(at),
                _ => None,
            },
            width: window.resolution.width().round() as u32,
            height: window.resolution.height().round() as u32,
            mode: match window.mode {
                WindowMode::Windowed => Mode::Windowed,
                WindowMode::BorderlessFullscreen(_) => Mode::Borderless,
                WindowMode::Fullscreen(..) => Mode::Fullscreen,
            },
        }
    }

    pub fn apply(&self, window: &mut Window) {
        if let Some(at) = self.position {
            window.position = WindowPosition::At(at);
        }
        // Without a scale factor yet, `new` sizes the window in logical pixels.
        window.resolution = WindowResolution::new(self.width, self.height);
        window.mode = match self.mode {
            Mode::Windowed => WindowMode::Windowed,
            Mode::Borderless => WindowMode::BorderlessFullscreen(MonitorSelection::Current),
            Mode::Fullscreen => WindowMode::Fullscreen(
                MonitorSelection::Current,
                bevy::window::VideoModeSelection::Current,
            ),
        };
    }
}

/// What the window system knows that Bevy's `Window` does not: winit sends no
/// move event for a window that never moved, and the macOS green button
/// changes fullscreen without touching `WindowMode`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Native {
    pub position: Option<IVec2>,
    pub fullscreen: bool,
}

impl Native {
    fn of(entity: Entity) -> Self {
        bevy::winit::WINIT_WINDOWS.with_borrow(|windows| {
            let Some(window) = windows.get_window(entity) else {
                return Self::default();
            };
            Self {
                position: window
                    .outer_position()
                    .ok()
                    .map(|at| IVec2::new(at.x, at.y)),
                fullscreen: window.fullscreen().is_some(),
            }
        })
    }
}

/// While fullscreen the window's own frame is the monitor's, so the last
/// windowed one carries instead.
pub fn observe(window: &Window, native: Native, previous: Option<Geometry>) -> Geometry {
    let seen = Geometry::of(window);
    let mode = match seen.mode {
        Mode::Windowed if native.fullscreen => Mode::Borderless,
        mode => mode,
    };
    if mode == Mode::Windowed {
        return Geometry {
            position: seen.position.or(native.position),
            ..seen
        };
    }
    match previous {
        Some(windowed) => Geometry { mode, ..windowed },
        None => Geometry {
            position: None,
            mode,
            ..seen
        },
    }
}

/// A display in physical pixels, as `Monitor` entities report it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Monitor {
    pub origin: IVec2,
    pub size: UVec2,
    pub scale: f32,
}

impl Monitor {
    fn contains(&self, point: IVec2) -> bool {
        let end = self.origin + self.size.as_ivec2();
        point.cmpge(self.origin).all() && point.cmplt(end).all()
    }
}

/// Keeps a saved window usable on today's displays: a top-left on no monitor
/// (unplugged, resolution changed) drops to centring, and a size past the
/// monitor it lands on shrinks to fit. Without any known monitor nothing is
/// known to be wrong.
pub fn fit(geometry: Geometry, monitors: &[Monitor]) -> Geometry {
    let Some(first) = monitors.first() else {
        return geometry;
    };
    let on = geometry
        .position
        .and_then(|at| monitors.iter().find(|m| m.contains(at)));
    let monitor = on.unwrap_or(first);
    let logical = |physical: u32| ((physical as f32 / monitor.scale) as u32).max(1);
    Geometry {
        position: on.and(geometry.position),
        width: geometry.width.min(logical(monitor.size.x)),
        height: geometry.height.min(logical(monitor.size.y)),
        ..geometry
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    List,
    World(PathBuf),
}

/// Set before a requested exit; `main` starts the next process from it.
#[derive(Resource, Default)]
pub struct Relaunch(pub Option<Target>);

/// How this process was started, for the screens that depend on it.
#[derive(Resource, Clone, Default)]
pub struct Launched {
    pub from_list: bool,
    pub world_name: Option<String>,
}

/// The geometry this process was started with, until it is checked against
/// the monitors, which exist only once the window system is running.
#[derive(Resource, Default)]
pub struct Saved(pub Option<Geometry>);

#[derive(Resource, Default)]
pub struct LastGeometry(pub Option<Geometry>);

/// `App::run` moves the App into the window runner, so nothing inside it is
/// readable afterwards; the next process's target leaves through this.
#[derive(Resource, Clone, Default)]
pub struct Handoff(Arc<Mutex<(Option<Target>, Option<Geometry>)>>);

impl Handoff {
    pub fn take(&self) -> (Option<Target>, Option<Geometry>) {
        std::mem::take(&mut *self.0.lock().unwrap_or_else(|e| e.into_inner()))
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Relaunch>()
        .init_resource::<LastGeometry>()
        .init_resource::<Handoff>()
        .init_resource::<Saved>()
        .add_systems(
            Update,
            restore.run_if(|saved: Res<Saved>| saved.0.is_some()),
        )
        .add_systems(Last, (track, publish).chain());
}

fn publish(relaunch: Res<Relaunch>, last: Res<LastGeometry>, handoff: Res<Handoff>) {
    if relaunch.is_changed() || last.is_changed() {
        *handoff.0.lock().unwrap_or_else(|e| e.into_inner()) = (relaunch.0.clone(), last.0);
    }
}

fn restore(
    mut saved: ResMut<Saved>,
    monitors: Query<&bevy::window::Monitor>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let (Some(geometry), Ok(mut window)) = (saved.0, windows.single_mut()) else {
        return;
    };
    let monitors: Vec<Monitor> = monitors
        .iter()
        .map(|m| Monitor {
            origin: m.physical_position,
            size: UVec2::new(m.physical_width, m.physical_height),
            scale: m.scale_factor as f32,
        })
        .collect();
    if monitors.is_empty() {
        return;
    }
    saved.0 = None;
    let fitted = fit(geometry, &monitors);
    if fitted.position != geometry.position {
        window.position = WindowPosition::Centered(MonitorSelection::Primary);
    }
    if (fitted.width, fitted.height) != (geometry.width, geometry.height) {
        window
            .resolution
            .set(fitted.width as f32, fitted.height as f32);
    }
}

fn track(
    windows: Query<(Entity, &Window), (With<PrimaryWindow>, Changed<Window>)>,
    mut last: ResMut<LastGeometry>,
    _main_thread: NonSendMarker,
) {
    for (entity, window) in &windows {
        last.0 = Some(observe(window, Native::of(entity), last.0));
    }
}

pub fn next_args(target: &Target, geometry: Option<Geometry>, user: &[OsString]) -> Vec<OsString> {
    let mut args = user.to_vec();
    if let Target::World(path) = target {
        args.push(WORLD_ARG.into());
        args.push(path.into());
    }
    if let Some(geometry) = geometry {
        args.push(WINDOW_ARG.into());
        args.push(geometry.arg().into());
    }
    args
}

/// Replaces this process on Unix, so the World's writer lock (close-on-exec)
/// is released before the next process tries to open it.
pub fn relaunch(args: Vec<OsString>) -> std::process::ExitCode {
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(e) => {
            eprintln!("Cannot restart Tesserae: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let mut command = std::process::Command::new(exe);
    command.args(args);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let error = command.exec();
        eprintln!("Cannot restart Tesserae: {error}");
        std::process::ExitCode::FAILURE
    }
    #[cfg(not(unix))]
    match command.spawn() {
        Ok(_) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Cannot restart Tesserae: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

/// A Mod that would run without a recorded Trust grant for its exact bytes.
pub struct Untrusted {
    pub prompt: String,
    pub found: tesserae_mods::Found,
    pub sha256: String,
}

/// What the World list can offer, found without running any Mod code.
#[derive(Resource)]
pub struct Installed {
    pub game: Option<GameRef>,
    pub untrusted: Vec<Untrusted>,
    /// Why the Mods folder could not be read; nothing can be created.
    pub error: Option<String>,
    /// `None` when nothing can be granted, as in tests.
    trust_file: Option<PathBuf>,
}

impl Installed {
    pub fn new(game: Option<GameRef>, untrusted: Vec<Untrusted>, trust_file: PathBuf) -> Self {
        Self {
            game,
            untrusted,
            error: None,
            trust_file: Some(trust_file),
        }
    }

    pub fn none(error: Option<String>) -> Self {
        Self {
            game: None,
            untrusted: Vec::new(),
            error,
            trust_file: None,
        }
    }

    #[cfg(test)]
    pub fn fake(game: Option<(&str, &str)>, untrusted: &[&str]) -> Self {
        Self {
            game: game.map(|(id, version)| GameRef {
                id: id.into(),
                version: version.into(),
            }),
            untrusted: untrusted
                .iter()
                .map(|id| Untrusted {
                    prompt: format!("Mod: {id}"),
                    found: tesserae_mods::Found {
                        manifest: test_manifest(id),
                        dir: PathBuf::new(),
                        manifest_bytes: Vec::new(),
                    },
                    sha256: String::new(),
                })
                .collect(),
            error: None,
            trust_file: None,
        }
    }

    /// Records Trust for the first untrusted Mod's exact bytes.
    pub fn grant_first(&mut self) -> Result<(), String> {
        if self.untrusted.is_empty() {
            return Ok(());
        }
        let first = self.untrusted.remove(0);
        let Some(trust_file) = &self.trust_file else {
            return Ok(());
        };
        let mut trust = tesserae_mods::Trust::load(trust_file);
        trust
            .grant_exact(&first.found, &first.sha256)
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
fn test_manifest(id: &str) -> tesserae_mods::Manifest {
    tesserae_mods::Manifest {
        id: id.into(),
        version: "1".into(),
        api: "0.3".into(),
        root: false,
        wasm: "m.wasm".into(),
        depends: Vec::new(),
        limits: Default::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<OsString> {
        list.iter().map(OsString::from).collect()
    }

    #[test]
    fn no_world_and_no_direct_variable_opens_the_list() {
        let parsed = parse(args(&["--mods", "/m"]), |_| false).unwrap();
        assert_eq!(parsed.launch, Launch::List);
        assert_eq!(parsed.user, args(&["--mods", "/m"]));
    }

    #[test]
    fn direct_variables_bypass_the_list() {
        for name in DIRECT {
            let parsed = parse(args(&[]), |n| n == name).unwrap();
            assert_eq!(parsed.launch, Launch::Direct, "{name}");
        }
    }

    #[test]
    fn a_world_argument_and_geometry_are_consumed_not_forwarded() {
        let parsed = parse(
            args(&[
                "--trust-all",
                "--world",
                "/w/Ridge",
                "--window",
                "10,-20,1600,1000,borderless",
            ]),
            |_| true,
        )
        .unwrap();
        assert_eq!(parsed.launch, Launch::World("/w/Ridge".into()));
        assert_eq!(
            parsed.window,
            Some(Geometry {
                position: Some(IVec2::new(10, -20)),
                width: 1600,
                height: 1000,
                mode: Mode::Borderless
            })
        );
        assert_eq!(parsed.user, args(&["--trust-all"]));
    }

    #[test]
    fn malformed_geometry_falls_back_to_the_default_window() {
        for bad in [
            "",
            "1,2,3",
            "1,2,0,4,windowed",
            "1,2,3,4,maximised",
            "1,2,3,4,windowed,9",
            "auto,2,3,4,windowed",
        ] {
            assert_eq!(Geometry::parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn the_next_process_gets_user_arguments_target_and_geometry() {
        let geometry = Geometry {
            position: Some(IVec2::new(0, 25)),
            width: 1280,
            height: 800,
            mode: Mode::Windowed,
        };
        let next = next_args(
            &Target::World("/w/A".into()),
            Some(geometry),
            &args(&["--mods", "/m"]),
        );
        let reparsed = parse(next, |_| false).unwrap();
        assert_eq!(reparsed.launch, Launch::World("/w/A".into()));
        assert_eq!(reparsed.window, Some(geometry));
        assert_eq!(reparsed.user, args(&["--mods", "/m"]));
        let unmoved = Geometry {
            position: None,
            ..geometry
        };
        let back = parse(next_args(&Target::List, Some(unmoved), &[]), |_| false).unwrap();
        assert_eq!(back.launch, Launch::List);
        assert_eq!(back.window, Some(unmoved));
    }

    fn monitor(x: i32, y: i32, width: u32, height: u32, scale: f32) -> Monitor {
        Monitor {
            origin: IVec2::new(x, y),
            size: UVec2::new(width, height),
            scale,
        }
    }

    fn windowed(x: i32, y: i32, width: u32, height: u32) -> Geometry {
        Geometry {
            position: Some(IVec2::new(x, y)),
            width,
            height,
            mode: Mode::Windowed,
        }
    }

    #[test]
    fn a_position_on_a_monitor_is_kept() {
        let monitors = [monitor(0, 0, 1920, 1080, 1.0)];
        let saved = windowed(100, 50, 1280, 800);
        assert_eq!(fit(saved, &monitors), saved);
    }

    #[test]
    fn a_position_on_a_negative_origin_monitor_is_kept() {
        let monitors = [
            monitor(0, 0, 1920, 1080, 1.0),
            monitor(-1920, -200, 1920, 1200, 1.0),
        ];
        let saved = windowed(-1500, -100, 1280, 800);
        assert_eq!(fit(saved, &monitors), saved);
    }

    #[test]
    fn a_position_on_a_removed_display_falls_back_to_centring() {
        let monitors = [monitor(0, 0, 1920, 1080, 1.0)];
        let fitted = fit(windowed(-1500, -100, 1280, 800), &monitors);
        assert_eq!(
            fitted,
            Geometry {
                position: None,
                ..windowed(0, 0, 1280, 800)
            }
        );
    }

    #[test]
    fn a_window_larger_than_its_monitor_is_clamped_in_logical_pixels() {
        let plain = [monitor(0, 0, 1920, 1080, 1.0)];
        let fitted = fit(windowed(0, 0, 3000, 2000), &plain);
        assert_eq!((fitted.width, fitted.height), (1920, 1080));
        let retina = [monitor(0, 0, 3840, 2160, 2.0)];
        let fitted = fit(windowed(0, 0, 3000, 2000), &retina);
        assert_eq!((fitted.width, fitted.height), (1920, 1080));
    }

    #[test]
    fn an_unmoved_window_is_clamped_to_the_first_monitor() {
        let monitors = [monitor(0, 0, 1280, 720, 1.0)];
        let saved = Geometry {
            position: None,
            ..windowed(0, 0, 1600, 900)
        };
        let fitted = fit(saved, &monitors);
        assert_eq!(
            (fitted.position, fitted.width, fitted.height),
            (None, 1280, 720)
        );
    }

    #[test]
    fn no_known_monitor_leaves_the_geometry_alone() {
        let saved = windowed(-1500, -100, 3000, 2000);
        assert_eq!(fit(saved, &[]), saved);
    }

    fn window(position: WindowPosition, mode: WindowMode, width: u32, height: u32) -> Window {
        Window {
            position,
            mode,
            resolution: WindowResolution::new(width, height),
            ..default()
        }
    }

    fn unmoved(width: u32, height: u32) -> Window {
        window(
            WindowPosition::Automatic,
            WindowMode::Windowed,
            width,
            height,
        )
    }

    fn native(position: Option<(i32, i32)>, fullscreen: bool) -> Native {
        Native {
            position: position.map(|(x, y)| IVec2::new(x, y)),
            fullscreen,
        }
    }

    #[test]
    fn a_window_that_never_moved_reports_the_position_the_system_gave_it() {
        let seen = observe(&unmoved(1280, 800), native(Some((220, 90)), false), None);
        assert_eq!(seen, windowed(220, 90, 1280, 800));
    }

    #[test]
    fn a_position_from_bevy_wins_over_the_native_one() {
        let moved = window(
            WindowPosition::At(IVec2::new(5, 6)),
            WindowMode::Windowed,
            1280,
            800,
        );
        let seen = observe(&moved, native(Some((220, 90)), false), None);
        assert_eq!(seen.position, Some(IVec2::new(5, 6)));
    }

    #[test]
    fn a_platform_that_reports_no_position_still_centres() {
        let seen = observe(&unmoved(1280, 800), native(None, false), None);
        assert_eq!(seen.position, None);
    }

    #[test]
    fn the_green_button_counts_as_fullscreen_and_keeps_the_windowed_frame() {
        let before = windowed(220, 90, 1280, 800);
        let full = window(WindowPosition::Automatic, WindowMode::Windowed, 2560, 1440);
        let seen = observe(&full, native(Some((0, 0)), true), Some(before));
        assert_eq!(
            seen,
            Geometry {
                mode: Mode::Borderless,
                ..before
            }
        );
        let again = observe(&full, native(Some((0, 0)), true), Some(seen));
        assert_eq!(again, seen);
    }

    #[test]
    fn fullscreen_with_no_windowed_frame_known_uses_the_current_size() {
        let full = window(WindowPosition::Automatic, WindowMode::Windowed, 2560, 1440);
        let seen = observe(&full, native(Some((0, 0)), true), None);
        assert_eq!(
            seen,
            Geometry {
                position: None,
                mode: Mode::Borderless,
                ..windowed(0, 0, 2560, 1440)
            }
        );
    }

    #[test]
    fn leaving_fullscreen_records_the_window_again() {
        let before = Geometry {
            mode: Mode::Borderless,
            ..windowed(220, 90, 1280, 800)
        };
        let seen = observe(
            &unmoved(1000, 700),
            native(Some((40, 30)), false),
            Some(before),
        );
        assert_eq!(seen, windowed(40, 30, 1000, 700));
    }

    #[test]
    fn bevy_exclusive_fullscreen_also_keeps_the_windowed_frame() {
        let before = windowed(220, 90, 1280, 800);
        let exclusive = window(
            WindowPosition::Automatic,
            WindowMode::Fullscreen(
                MonitorSelection::Current,
                bevy::window::VideoModeSelection::Current,
            ),
            2560,
            1440,
        );
        let seen = observe(&exclusive, native(Some((0, 0)), true), Some(before));
        assert_eq!(
            seen,
            Geometry {
                mode: Mode::Fullscreen,
                ..before
            }
        );
    }

    fn restoring(saved: Geometry, monitors: &[(i32, i32, u32, u32)]) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_plugins(plugin);
        app.insert_resource(Saved(Some(saved)));
        let mut window = Window::default();
        saved.apply(&mut window);
        app.world_mut().spawn((PrimaryWindow, window));
        for &(x, y, width, height) in monitors {
            app.world_mut().spawn(bevy::window::Monitor {
                name: None,
                physical_height: height,
                physical_width: width,
                physical_position: IVec2::new(x, y),
                refresh_rate_millihertz: None,
                scale_factor: 1.0,
                video_modes: Vec::new(),
            });
        }
        app
    }

    fn the_window(app: &mut App) -> &Window {
        app.world_mut()
            .query_filtered::<&Window, With<PrimaryWindow>>()
            .single(app.world())
            .unwrap()
    }

    #[test]
    fn a_window_saved_on_a_removed_display_is_centred_and_shrunk_once_monitors_are_known() {
        let mut app = restoring(windowed(-1500, -100, 3000, 2000), &[]);
        app.update();
        assert_eq!(
            the_window(&mut app).position,
            WindowPosition::At(IVec2::new(-1500, -100)),
            "monitors unknown yet"
        );
        app.world_mut().spawn(bevy::window::Monitor {
            name: None,
            physical_height: 1080,
            physical_width: 1920,
            physical_position: IVec2::ZERO,
            refresh_rate_millihertz: None,
            scale_factor: 1.0,
            video_modes: Vec::new(),
        });
        app.update();
        let window = the_window(&mut app);
        assert_eq!(
            window.position,
            WindowPosition::Centered(MonitorSelection::Primary)
        );
        assert_eq!(
            (window.resolution.width(), window.resolution.height()),
            (1920.0, 1080.0)
        );
        assert!(app.world().resource::<Saved>().0.is_none());
    }

    #[test]
    fn a_window_saved_on_a_present_display_is_left_alone() {
        let saved = windowed(100, 50, 1280, 800);
        let mut app = restoring(saved, &[(0, 0, 1920, 1080)]);
        app.update();
        let window = the_window(&mut app);
        assert_eq!(window.position, WindowPosition::At(IVec2::new(100, 50)));
        assert_eq!(window.resolution.width(), 1280.0);
    }

    #[test]
    fn the_target_outlives_the_app_that_chose_it() {
        let handoff = Handoff::default();
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_plugins(plugin);
        app.insert_resource(handoff.clone());
        app.update();
        app.world_mut().resource_mut::<Relaunch>().0 = Some(Target::List);
        app.update();
        drop(app);
        assert_eq!(handoff.take(), (Some(Target::List), None));
        assert_eq!(handoff.take(), (None, None));
    }
}
