//! The Tesserae engine's Bevy application.

mod client;
mod content;
#[cfg(feature = "dev")]
mod cost;
mod render;
mod server;
mod world;

use bevy::app::TaskPoolPlugin;
use bevy::prelude::*;

fn main() -> std::process::ExitCode {
    let settings = world::WorldSettings::default();
    let loaded = match server::mods::load_from_env(&settings) {
        Ok(loaded) => loaded,
        Err(error) => {
            eprintln!("Cannot open World: {error}");
            return std::process::ExitCode::FAILURE;
        }
    };
    App::new()
        .insert_resource(settings)
        .insert_non_send(loaded)
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
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
#[cfg(feature = "dev")]
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

#[cfg(not(feature = "dev"))]
fn task_pools() -> TaskPoolPlugin {
    TaskPoolPlugin::default()
}

#[cfg(feature = "dev")]
fn dev_plugins(app: &mut App) {
    app.add_plugins(cost::plugin);
}

#[cfg(not(feature = "dev"))]
fn dev_plugins(_: &mut App) {}
