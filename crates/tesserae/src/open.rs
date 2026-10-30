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

/// Where the app goes after its first frame.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Route {
    List,
    Direct,
}

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

fn route(mut commands: Commands, route: Option<Res<Route>>, mut next: ResMut<NextState<AppState>>) {
    match route.as_deref() {
        Some(Route::Direct) => {
            commands.insert_resource(OpenRequest {
                what: What::Direct,
                name: None,
                from_list: false,
            });
            next.set(AppState::Loading);
        }
        _ => next.set(AppState::List),
    }
    commands.remove_resource::<Route>();
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
