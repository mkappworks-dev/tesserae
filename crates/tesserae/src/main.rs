//! The Tesserae engine's Bevy application.

mod author;
mod client;
mod content;
#[cfg(any(feature = "dev", feature = "profiling"))]
mod cost;
mod dev;
mod launch;
mod open;
#[cfg(feature = "profiling")]
mod profiling;
mod render;
mod save_dump;
mod server;
mod session;
mod world;

use bevy::app::TaskPoolPlugin;
use bevy::prelude::*;

fn main() -> std::process::ExitCode {
    #[cfg(feature = "profiling")]
    let process_start = profiling::ProcessStart(std::time::Instant::now());
    let argv: Vec<_> = std::env::args_os().skip(1).collect();
    if let Some(code) = author::run(&argv).or_else(|| save_dump::run(&argv)) {
        return code;
    }
    let watching = match dev::parse(&argv) {
        None => None,
        Some(Ok(request)) => Some(request),
        Some(Err(e)) => {
            eprintln!("error: {e}\n\n{}", dev::USAGE);
            return std::process::ExitCode::from(2);
        }
    };
    // A dev run opens its own World, so neither its arguments nor the direct-open variables apply.
    let args = if watching.is_some() {
        launch::parse([], |_| false)
    } else {
        launch::parse(argv, |name| std::env::var_os(name).is_some())
    };
    let worlds = server::persistence::worlds_dir().unwrap_or_else(|| "worlds".into());
    let mut app = App::new();
    app.insert_resource(args.launch)
        .insert_resource(open::UserArgs(args.user))
        .init_resource::<launch::Switch>()
        .insert_resource(client::worlds::WorldsDir(worlds))
        .add_plugins(
            DefaultPlugins
                .set(window_plugin())
                .set(task_pools())
                .set(log_settings())
                .set(render_settings()),
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
    if let Some(request) = watching {
        app.add_plugins(dev::plugin(request));
    }
    #[cfg(feature = "profiling")]
    app.insert_resource(process_start);
    match app.run() {
        AppExit::Success => std::process::ExitCode::SUCCESS,
        AppExit::Error(code) => std::process::ExitCode::from(code.get()),
    }
}

fn window_plugin() -> WindowPlugin {
    let window = Window {
        title: "Tesserae".into(),
        ..default()
    };
    #[cfg(feature = "profiling")]
    let window = profiling::monitor_window(window);
    WindowPlugin {
        close_when_requested: false,
        primary_window: Some(window),
        ..default()
    }
}

/// Bevy keeps wgpu validating every indirect draw in release whenever its
/// default backends name DX12, which they do on macOS too: a compute pass, a
/// staging buffer and a copy before each indirect render pass. Bevy writes
/// those arguments itself; `WGPU_VALIDATION_INDIRECT_CALL=1` turns it back on.
#[cfg(target_os = "macos")]
fn render_settings() -> bevy::render::RenderPlugin {
    use bevy::render::settings::{Backends, InstanceFlags, WgpuSettings};

    let mut settings = WgpuSettings {
        backends: Some(Backends::from_env().unwrap_or(Backends::METAL)),
        ..default()
    };
    if !cfg!(debug_assertions) {
        settings
            .instance_flags
            .remove(InstanceFlags::VALIDATION_INDIRECT_CALL);
        settings.instance_flags = settings.instance_flags.with_env();
    }
    bevy::render::RenderPlugin {
        render_creation: settings.into(),
        ..default()
    }
}

#[cfg(not(target_os = "macos"))]
fn render_settings() -> bevy::render::RenderPlugin {
    bevy::render::RenderPlugin::default()
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

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use bevy::render::settings::{Backends, InstanceFlags, RenderCreation};

    #[test]
    fn a_mac_draws_on_metal_and_validates_indirect_draws_in_debug_only() {
        let RenderCreation::Automatic(settings) = super::render_settings().render_creation else {
            panic!("the renderer creates its own device");
        };
        if std::env::var_os("WGPU_BACKEND").is_none() {
            assert_eq!(settings.backends, Some(Backends::METAL));
        }
        if std::env::var_os("WGPU_VALIDATION_INDIRECT_CALL").is_none() {
            assert_eq!(
                settings
                    .instance_flags
                    .contains(InstanceFlags::VALIDATION_INDIRECT_CALL),
                cfg!(debug_assertions)
            );
        }
    }
}
