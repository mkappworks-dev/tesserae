//! The Tesserae engine's Bevy application.

mod client;
mod content;
#[cfg(any(feature = "dev", feature = "profiling"))]
mod cost;
mod launch;
mod open;
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
    let args = launch::parse(std::env::args_os().skip(1), |name| {
        std::env::var_os(name).is_some()
    });
    let worlds = server::persistence::worlds_dir().unwrap_or_else(|| "worlds".into());
    let mut app = App::new();
    app.insert_resource(match args.launch {
        launch::Launch::Direct => open::Route::Direct,
        launch::Launch::List => open::Route::List,
    })
    .insert_resource(open::UserArgs(args.user))
    .init_resource::<launch::Switch>()
    .insert_resource(client::worlds::WorldsDir(worlds))
    .add_plugins(
        DefaultPlugins
            .set(window_plugin())
            .set(task_pools())
            .set(log_settings()),
    )
    // Before the plugins that register through it: they check for its state.
    .add_plugins(session::plugin)
    .add_plugins((
        render::plugin,
        server::plugin,
        client::plugin,
        client::worlds::plugin,
        open::plugin,
    ))
    .add_plugins(dev_plugins);
    #[cfg(feature = "profiling")]
    app.insert_resource(process_start);
    match app.run() {
        AppExit::Success => std::process::ExitCode::SUCCESS,
        AppExit::Error(code) => std::process::ExitCode::from(code.get()),
    }
}

fn window_plugin() -> WindowPlugin {
    WindowPlugin {
        close_when_requested: false,
        primary_window: Some(Window {
            title: "Tesserae".into(),
            ..default()
        }),
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
