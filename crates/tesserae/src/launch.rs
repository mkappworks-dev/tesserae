//! How this process was started and what it hands over to the next one. Each
//! process opens at most one World; switching re-executes the binary, so no
//! job or answer from one World can land in another.

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use bevy::window::{MonitorSelection, PrimaryWindow, WindowMode, WindowPosition, WindowResolution};

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

/// Logical size and, once the window has moved, physical position: winit sends
/// no position for a window that never moved, so the next one is centred.
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
        .add_systems(Last, (track, publish).chain());
}

fn publish(relaunch: Res<Relaunch>, last: Res<LastGeometry>, handoff: Res<Handoff>) {
    if relaunch.is_changed() || last.is_changed() {
        *handoff.0.lock().unwrap_or_else(|e| e.into_inner()) = (relaunch.0.clone(), last.0);
    }
}

fn track(
    windows: Query<&Window, (With<PrimaryWindow>, Changed<Window>)>,
    mut last: ResMut<LastGeometry>,
) {
    for window in &windows {
        last.0 = Some(Geometry::of(window));
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
