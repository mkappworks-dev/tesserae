//! The Tesserae engine's Bevy application.

mod client;
mod content;
#[cfg(any(feature = "dev", feature = "profiling"))]
mod cost;
mod launch;
#[cfg(feature = "profiling")]
mod profiling;
mod render;
mod server;
mod session;
mod world;

use bevy::app::TaskPoolPlugin;
use bevy::prelude::*;

fn main() -> std::process::ExitCode {
    #[cfg(feature = "profiling")]
    let process_start = profiling::ProcessStart(std::time::Instant::now());
    let args = match launch::parse(std::env::args_os().skip(1), |name| {
        std::env::var_os(name).is_some()
    }) {
        Ok(args) => args,
        Err(error) => {
            eprintln!("{error}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let mut app = match &args.launch {
        launch::Launch::List => list_app(&args, None),
        launch::Launch::Direct => {
            match server::persistence::Opening::from_env().and_then(|o| world_parts(o, &args)) {
                Ok(parts) => world_app(parts, &args, false),
                Err(error) => {
                    eprintln!("Cannot open World: {error}");
                    return std::process::ExitCode::FAILURE;
                }
            }
        }
        launch::Launch::World(folder) => {
            match server::persistence::Opening::from_folder(folder)
                .and_then(|o| world_parts(o, &args))
            {
                Ok(parts) => world_app(parts, &args, true),
                Err(reason) => {
                    eprintln!("Cannot open World: {reason}");
                    list_app(
                        &args,
                        Some(client::worlds::OpenFailure {
                            folder: folder.clone(),
                            reason,
                        }),
                    )
                }
            }
        }
    };
    #[cfg(feature = "profiling")]
    app.insert_resource(process_start);
    let handoff = launch::Handoff::default();
    app.insert_resource(handoff.clone());
    app.run();
    let (target, geometry) = handoff.take();
    match target {
        Some(target) => launch::relaunch(launch::next_args(&target, geometry, &args.user)),
        None => std::process::ExitCode::SUCCESS,
    }
}

struct WorldParts {
    settings: world::WorldSettings,
    loaded: server::mods::LoadedMods,
    persistence: server::persistence::Persistence,
    name: Option<String>,
}

fn world_parts(
    opening: server::persistence::Opening,
    args: &launch::Args,
) -> Result<WorldParts, String> {
    #[cfg(any(feature = "dev", feature = "profiling"))]
    let base = world::WorldSettings::from_env(|name| std::env::var(name).ok());
    #[cfg(not(any(feature = "dev", feature = "profiling")))]
    let base = world::WorldSettings::default();
    let settings = opening.settings(base);
    let name = opening.display_name();
    let wanted = opening.pending_game().cloned();
    let mut loaded = server::mods::load_world(&args.user, &settings, &opening)?;
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
    let persistence = server::persistence::Persistence::prepare(opening, &mut loaded, settings)?;
    Ok(WorldParts {
        settings,
        loaded,
        persistence,
        name,
    })
}

fn world_app(parts: WorldParts, args: &launch::Args, from_list: bool) -> App {
    let mut app = App::new();
    app.insert_resource(parts.persistence)
        .insert_resource(launch::Saved(args.window))
        .insert_resource(launch::Launched {
            from_list,
            world_name: parts.name,
        })
        .insert_resource(parts.settings)
        .insert_non_send(parts.loaded)
        .add_plugins(
            DefaultPlugins
                .set(window_plugin(args.window))
                .set(task_pools())
                .set(log_settings()),
        )
        .add_plugins(launch::plugin)
        .add_plugins((render::plugin, server::plugin, client::plugin))
        .add_plugins(dev_plugins);
    app
}

fn list_app(args: &launch::Args, failure: Option<client::worlds::OpenFailure>) -> App {
    let mut app = App::new();
    let worlds = server::persistence::worlds_dir().unwrap_or_else(|| "worlds".into());
    app.insert_resource(launch::Saved(args.window))
        .insert_resource(launch::Launched {
            from_list: false,
            world_name: None,
        })
        .insert_resource(server::mods::installed(&args.user))
        .insert_resource(client::worlds::WorldsDir(worlds))
        .add_plugins(
            DefaultPlugins
                .set(window_plugin(args.window))
                .set(log_settings()),
        )
        .add_plugins((launch::plugin, client::worlds::plugin));
    if let Some(failure) = failure {
        app.insert_resource(failure);
    }
    app
}

fn window_plugin(geometry: Option<launch::Geometry>) -> WindowPlugin {
    let mut window = Window {
        title: "Tesserae".into(),
        ..default()
    };
    if let Some(geometry) = geometry {
        geometry.apply(&mut window);
    }
    WindowPlugin {
        close_when_requested: false,
        primary_window: Some(window),
        ..default()
    }
}

fn log_settings() -> bevy::log::LogPlugin {
    #[cfg(feature = "profiling")]
    if std::env::var_os("TESSERAE_PROFILE_CSV").is_some() {
        return bevy::log::LogPlugin {
            filter: "info,wgpu_core::device::resource=debug".into(),
            custom_layer: profiling::surface_log_layer,
            ..default()
        };
    }
    bevy::log::LogPlugin::default()
}

/// Bevy's default pool sizes, except that a `dev` run can pin the async compute
/// pool, which streaming jobs run on, to measure what its size is worth.
#[cfg(any(feature = "dev", feature = "profiling"))]
fn task_pools() -> TaskPoolPlugin {
    use bevy::app::{TaskPoolOptions, TaskPoolThreadAssignmentPolicy};

    let mut options = TaskPoolOptions::default();
    if let Some(n) = std::env::var("TESSERAE_ASYNC_THREADS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
    {
        options.async_compute = TaskPoolThreadAssignmentPolicy {
            min_threads: n,
            max_threads: n,
            percent: 1.0,
            on_thread_spawn: None,
            on_thread_destroy: None,
        };
    }
    TaskPoolPlugin {
        task_pool_options: options,
    }
}

#[cfg(not(any(feature = "dev", feature = "profiling")))]
fn task_pools() -> TaskPoolPlugin {
    TaskPoolPlugin::default()
}

#[cfg(any(feature = "dev", feature = "profiling"))]
fn dev_plugins(app: &mut App) {
    app.add_plugins(cost::plugin);
    #[cfg(feature = "profiling")]
    app.add_plugins(profiling::plugin);
}

#[cfg(not(any(feature = "dev", feature = "profiling")))]
fn dev_plugins(_: &mut App) {}
