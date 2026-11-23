//! Presentation policy never changes World, streaming, or rendering quality.
use super::profile::Preferences;
use bevy::{
    prelude::*,
    window::{MonitorSelection, PresentMode, VideoModeSelection},
};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// How the window takes the screen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum WindowMode {
    #[default]
    Windowed,
    Borderless,
    Fullscreen,
}
impl WindowMode {
    pub(super) const ALL: [Self; 3] = [Self::Windowed, Self::Borderless, Self::Fullscreen];
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Windowed => "Windowed",
            Self::Borderless => "Borderless fullscreen",
            Self::Fullscreen => "Fullscreen",
        }
    }
    fn bevy(self) -> bevy::window::WindowMode {
        match self {
            Self::Windowed => bevy::window::WindowMode::Windowed,
            Self::Borderless => {
                bevy::window::WindowMode::BorderlessFullscreen(MonitorSelection::Current)
            }
            Self::Fullscreen => bevy::window::WindowMode::Fullscreen(
                MonitorSelection::Current,
                VideoModeSelection::Current,
            ),
        }
    }
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct FramePacing;
pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        First,
        throttle.in_set(FramePacing).before(bevy::time::TimeSystems),
    )
    .add_systems(PostUpdate, policy);
}
fn throttle(preferences: Res<Preferences>, mut previous: Local<Option<Instant>>) {
    if std::env::var_os("TESSERAE_SCREENSHOT").is_some() {
        return;
    }
    if let (Some(cap), Some(previous)) = (preferences.fps_cap, *previous) {
        let period = Duration::from_secs_f64(1.0 / f64::from(cap));
        if let Some(remaining) = period.checked_sub(previous.elapsed()) {
            std::thread::sleep(remaining);
        }
    }
    *previous = Some(Instant::now());
}
pub(super) fn policy(
    preferences: Res<Preferences>,
    mut windows: Query<&mut Window, With<bevy::window::PrimaryWindow>>,
) {
    if !preferences.is_changed() || std::env::var_os("TESSERAE_SCREENSHOT").is_some() {
        return;
    }
    let mode = if preferences.display_sync {
        PresentMode::AutoVsync
    } else {
        PresentMode::AutoNoVsync
    };
    let fill = preferences.window_mode.bevy();
    // Only on a difference: any write to a Window reconfigures its surface.
    // On macOS winit enters both fullscreens through AppKit's own toggle,
    // which keeps the full-size content view and transparent title bar for
    // the return to Windowed.
    for mut window in &mut windows {
        if window.present_mode != mode {
            window.present_mode = mode;
        }
        if window.mode != fill {
            window.mode = fill;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_saved_window_mode_is_restored_at_startup() {
        let mut app = App::new();
        app.insert_resource(Preferences {
            window_mode: WindowMode::Fullscreen,
            ..default()
        })
        .add_systems(PostUpdate, policy);
        let window = app
            .world_mut()
            .spawn((Window::default(), bevy::window::PrimaryWindow))
            .id();
        app.update();
        assert_eq!(
            app.world().get::<Window>(window).unwrap().mode,
            WindowMode::Fullscreen.bevy()
        );
    }
}
