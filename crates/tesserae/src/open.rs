//! Opening a World inside the running app: the request, the load that runs off
//! the main thread with its real stages, and the hand-over into the session.

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, futures::check_ready};

use crate::client::worlds::{LastFailure, OpenFailure};
use crate::server;
use crate::session::AppState;
use crate::world::WorldSettings;

/// Arguments forwarded unchanged to Mod loading.
#[derive(Resource)]
pub struct UserArgs(pub Vec<OsString>);

#[derive(Clone, Debug, PartialEq)]
pub enum What {
    /// The environment names the World; a failure prints and exits.
    Direct,
    Folder(PathBuf),
}

/// Inserted by whoever wants a World opened, together with a move to
/// `AppState::Loading`.
#[derive(Resource, Clone, Debug)]
pub struct OpenRequest {
    pub what: What,
    pub name: Option<String>,
    pub from_list: bool,
}

/// What the loading screen and the in-World menu need to know about how this
/// World was opened.
#[derive(Resource, Clone, Debug)]
pub struct Opened {
    pub from_list: bool,
    pub name: Option<String>,
}

/// The stages the loading bar follows, in order. The last is the Client's
/// first drawn terrain, which only it can tell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Stage {
    ReadingSave,
    LoadingGame,
    PreparingWorld,
    FirstTerrain,
}

impl Stage {
    pub fn label(self) -> &'static str {
        match self {
            Stage::ReadingSave => "Reading the save",
            Stage::LoadingGame => "Loading the Game",
            Stage::PreparingWorld => "Preparing the World",
            Stage::FirstTerrain => "Building the first terrain",
        }
    }

    /// Where the bar stands on entering the stage, and where the next begins.
    pub fn span(self) -> (f32, f32) {
        match self {
            Stage::ReadingSave => (0.0, 0.15),
            Stage::LoadingGame => (0.15, 0.55),
            Stage::PreparingWorld => (0.55, 0.75),
            Stage::FirstTerrain => (0.75, 1.0),
        }
    }

    fn from_u8(n: u8) -> Self {
        match n {
            0 => Stage::ReadingSave,
            1 => Stage::LoadingGame,
            2 => Stage::PreparingWorld,
            _ => Stage::FirstTerrain,
        }
    }
}

struct Parts {
    settings: WorldSettings,
    loaded: server::mods::LoadedMods,
    persistence: server::persistence::Persistence,
    name: Option<String>,
}

#[derive(Resource)]
pub struct Load {
    stage: Arc<AtomicU8>,
    task: Option<Task<Result<Parts, String>>>,
    direct: bool,
    folder: Option<PathBuf>,
}

impl Load {
    pub fn stage(&self) -> Stage {
        Stage::from_u8(self.stage.load(Ordering::Relaxed))
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<LastFailure>()
        .add_systems(OnEnter(AppState::List), refresh_installed)
        .add_systems(
            Update,
            (
                route.run_if(in_state(AppState::Boot)),
                begin
                    .run_if(in_state(AppState::Loading))
                    .run_if(resource_exists::<OpenRequest>),
                finish
                    .run_if(in_state(AppState::Loading))
                    .run_if(resource_exists::<Load>),
                quit_while_loading.run_if(in_state(AppState::Loading)),
            ),
        );
}

fn route(
    mut commands: Commands,
    route: Option<Res<crate::launch::Launch>>,
    mut next: ResMut<NextState<AppState>>,
) {
    match route.as_deref() {
        Some(crate::launch::Launch::Direct) => {
            commands.insert_resource(OpenRequest {
                what: What::Direct,
                name: None,
                from_list: false,
            });
            next.set(AppState::Loading);
        }
        _ => next.set(AppState::List),
    }
    commands.remove_resource::<crate::launch::Launch>();
}

fn begin(mut commands: Commands, request: Res<OpenRequest>, args: Res<UserArgs>) {
    let stage = Arc::new(AtomicU8::new(Stage::ReadingSave as u8));
    let (what, user) = (request.what.clone(), args.0.clone());
    let reported = Arc::clone(&stage);
    let task = AsyncComputeTaskPool::get().spawn(async move { load(&what, &user, &reported) });
    commands.insert_resource(Opened {
        from_list: request.from_list,
        name: request.name.clone(),
    });
    commands.insert_resource(Load {
        stage,
        task: Some(task),
        direct: request.what == What::Direct,
        folder: match &request.what {
            What::Folder(path) => Some(path.clone()),
            What::Direct => None,
        },
    });
    commands.remove_resource::<OpenRequest>();
}

fn load(what: &What, user: &[OsString], stage: &AtomicU8) -> Result<Parts, String> {
    let opening = match what {
        What::Direct => server::persistence::Opening::from_env()?,
        What::Folder(folder) => server::persistence::Opening::from_folder(folder)?,
    };
    #[cfg(any(feature = "dev", feature = "profiling"))]
    let base = WorldSettings::from_env(|name| std::env::var(name).ok());
    #[cfg(not(any(feature = "dev", feature = "profiling")))]
    let base = WorldSettings::default();
    let settings = opening.settings(base);
    let name = opening.display_name();
    let wanted = opening.pending_game().cloned();
    stage.store(Stage::LoadingGame as u8, Ordering::Relaxed);
    let mut loaded = server::mods::load_world(user, &settings, &opening)?;
    // A skipped untrusted Game would otherwise silently create a World of Air,
    // and a different installed Game would create the wrong one.
    if let Some(game) = wanted
        && !loaded.required.iter().any(|m| m.game && m.id == game.id)
    {
        return Err(format!(
            "Game `{}` could not be loaded; it may be missing, replaced or not trusted",
            game.id
        ));
    }
    stage.store(Stage::PreparingWorld as u8, Ordering::Relaxed);
    let persistence = server::persistence::Persistence::prepare(opening, &mut loaded, settings)?;
    Ok(Parts {
        settings,
        loaded,
        persistence,
        name,
    })
}

fn finish(
    mut commands: Commands,
    mut load: ResMut<Load>,
    mut opened: ResMut<Opened>,
    mut next: ResMut<NextState<AppState>>,
    mut last: ResMut<LastFailure>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(result) = load.task.as_mut().and_then(check_ready) else {
        return;
    };
    load.task = None;
    match result {
        Ok(parts) => {
            opened.name = parts.name.or(opened.name.take());
            commands.insert_resource(parts.settings);
            commands.insert_resource(parts.persistence);
            let loaded = parts.loaded;
            commands.queue(move |world: &mut World| world.insert_non_send(loaded));
            next.set(AppState::InWorld);
        }
        Err(reason) if load.direct => {
            eprintln!("Cannot open World: {reason}");
            exit.write(AppExit::error());
        }
        Err(reason) => {
            last.0 = Some(OpenFailure {
                folder: load.folder.clone().unwrap_or_default(),
                reason,
            });
            next.set(AppState::Closing);
        }
    }
    commands.remove_resource::<Load>();
}

/// Nothing is saved or locked yet, so closing the window just ends the app.
fn quit_while_loading(
    mut requests: MessageReader<bevy::window::WindowCloseRequested>,
    mut exit: MessageWriter<AppExit>,
) {
    if requests.read().count() > 0 {
        exit.write(AppExit::Success);
    }
}

/// What is installed or trusted may have changed since the list last showed.
fn refresh_installed(mut commands: Commands, args: Res<UserArgs>) {
    commands.insert_resource(server::mods::installed(&args.0));
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::state::app::StatesPlugin;
    use tesserae_core::save::WorldId;

    struct Temp(PathBuf);

    impl Temp {
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(format!("tesserae-open-{}", WorldId::new().as_str()));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn app(mods: &Temp) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin, crate::session::plugin, plugin))
            .add_message::<AppExit>()
            .add_message::<bevy::window::WindowCloseRequested>()
            .insert_resource(UserArgs(vec!["--mods".into(), mods.0.clone().into()]));
        app
    }

    fn state(app: &App) -> AppState {
        *app.world().resource::<State<AppState>>().get()
    }

    /// Runs until the app is in `want`, giving the off-thread load time to finish.
    fn until(app: &mut App, want: AppState) -> bool {
        for _ in 0..600 {
            app.update();
            if state(app) == want {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        false
    }

    fn open_failing(app: &mut App, folder: &std::path::Path) {
        app.update();
        app.insert_resource(OpenRequest {
            what: What::Folder(folder.to_owned()),
            name: Some("Broken".into()),
            from_list: true,
        });
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Loading);
    }

    fn snapshot(folder: &std::path::Path) -> Vec<(PathBuf, Vec<u8>)> {
        let mut files: Vec<_> = std::fs::read_dir(folder)
            .unwrap()
            .map(|e| e.unwrap().path())
            .map(|p| (p.clone(), std::fs::read(p).unwrap()))
            .collect();
        files.sort();
        files
    }

    #[test]
    fn a_world_that_cannot_be_opened_returns_to_the_list_with_the_reason() {
        let (mods, worlds) = (Temp::new(), Temp::new());
        let broken = worlds.0.join("Broken");
        std::fs::create_dir(&broken).unwrap();
        std::fs::write(
            broken.join("00000000000000000099.commit"),
            b"TESSAVE1 truncated",
        )
        .unwrap();
        let before = snapshot(&broken);
        let mut app = app(&mods);
        open_failing(&mut app, &broken);

        assert!(until(&mut app, AppState::List), "back on the list");
        let failure = app.world().resource::<LastFailure>().0.as_ref().unwrap();
        assert_eq!(failure.folder, broken);
        assert!(!failure.reason.is_empty());
        assert!(!app.world().contains_resource::<Load>());
        assert!(!app.world().contains_resource::<Opened>());
        assert_eq!(snapshot(&broken), before, "nothing was written");
    }

    #[test]
    fn a_folder_with_no_save_is_refused_rather_than_becoming_a_world() {
        let (mods, worlds) = (Temp::new(), Temp::new());
        let empty = worlds.0.join("Empty");
        std::fs::create_dir(&empty).unwrap();
        let mut app = app(&mods);
        open_failing(&mut app, &empty);

        assert!(until(&mut app, AppState::List));
        assert!(app.world().resource::<LastFailure>().0.is_some());
        assert!(std::fs::read_dir(&empty).unwrap().next().is_none());
    }

    #[test]
    fn a_direct_launch_that_fails_exits_instead_of_showing_the_list() {
        let mods = Temp::new();
        let mut app = app(&mods);
        app.update();
        let task = AsyncComputeTaskPool::get_or_init(Default::default)
            .spawn(async { Err::<Parts, _>("no such World".to_owned()) });
        app.insert_resource(Opened {
            from_list: false,
            name: None,
        })
        .insert_resource(Load {
            stage: Arc::default(),
            task: Some(task),
            direct: true,
            folder: None,
        });
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Loading);
        for _ in 0..200 {
            app.update();
            std::thread::sleep(std::time::Duration::from_millis(2));
            if !app.world().resource::<Messages<AppExit>>().is_empty() {
                break;
            }
        }
        assert!(!app.world().resource::<Messages<AppExit>>().is_empty());
        assert_ne!(state(&app), AppState::List);
    }

    #[test]
    fn the_loading_stages_run_in_order_and_each_has_a_label() {
        let stages = [
            Stage::ReadingSave,
            Stage::LoadingGame,
            Stage::PreparingWorld,
            Stage::FirstTerrain,
        ];
        assert!(stages.windows(2).all(|w| w[0] < w[1]));
        for stage in stages {
            assert_eq!(Stage::from_u8(stage as u8), stage);
            assert!(!stage.label().is_empty());
        }
    }
}
