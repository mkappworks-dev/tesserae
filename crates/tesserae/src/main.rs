//! The Tesserae engine's Bevy application.

mod client;
mod content;
#[cfg(any(feature = "dev", feature = "profiling"))]
mod cost;
#[cfg(feature = "profiling")]
mod profiling;
mod render;
mod server;
mod world;

use bevy::app::TaskPoolPlugin;
use bevy::prelude::*;

fn main() -> std::process::ExitCode {
    #[cfg(feature = "profiling")]
    let process_start = profiling::ProcessStart(std::time::Instant::now());
    #[cfg(any(feature = "dev", feature = "profiling"))]
    let mut settings = world::WorldSettings::from_env(|name| std::env::var(name).ok());
    #[cfg(not(any(feature = "dev", feature = "profiling")))]
    let mut settings = world::WorldSettings::default();
    let opening = match server::persistence::Opening::from_env() {
        Ok(opening) => opening,
        Err(error) => {
            eprintln!("Cannot open World: {error}");
            return std::process::ExitCode::FAILURE;
        }
    };
    if let Some(recovered) = &opening.recovered {
        settings.seed = recovered.commit.seed;
        settings.voxel_size = recovered.commit.voxel_size_m;
    }
    let mut loaded = match server::mods::load_from_env(&settings, &opening) {
        Ok(loaded) => loaded,
        Err(error) => {
            eprintln!("Cannot open World: {error}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let persistence =
        match server::persistence::Persistence::prepare(opening, &mut loaded, settings) {
            Ok(persistence) => persistence,
            Err(error) => {
                eprintln!("Cannot open World: {error}");
                return std::process::ExitCode::FAILURE;
            }
        };
    let mut app = App::new();
    app.insert_resource(persistence);
    #[cfg(feature = "profiling")]
    app.insert_resource(process_start);
    app.insert_resource(settings)
        .insert_non_send(loaded)
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    close_when_requested: false,
                    primary_window: Some(Window {
                        title: "Tesserae".into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(task_pools()),
        )
        .add_plugins((render::plugin, server::plugin, client::plugin))
        .add_plugins(dev_plugins)
        .run();
    std::process::ExitCode::SUCCESS
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
