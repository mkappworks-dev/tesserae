//! Presentation policy never changes World, streaming, or rendering quality.
use super::profile::Preferences;
use bevy::{prelude::*, window::PresentMode};
use std::time::{Duration, Instant};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(First, throttle)
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
fn policy(preferences: Res<Preferences>, mut windows: Query<&mut Window>) {
    if !preferences.is_changed() || std::env::var_os("TESSERAE_SCREENSHOT").is_some() {
        return;
    }
    let mode = if preferences.display_sync {
        PresentMode::AutoVsync
    } else {
        PresentMode::AutoNoVsync
    };
    for mut window in &mut windows {
        window.present_mode = mode;
    }
}
