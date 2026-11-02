use std::time::{Duration, Instant};

use bevy::{
    prelude::*,
    window::{Monitor, MonitorSelection, OnMonitor, WindowMode, WindowPosition},
};

#[derive(Resource)]
struct Request {
    millihertz: u32,
    started: Option<Instant>,
    selected: Option<Entity>,
    size: UVec2,
    borderless: bool,
}

pub(super) fn plugin(app: &mut App) {
    let Ok(hz) = std::env::var("TESSERAE_PROFILE_MONITOR_HZ") else {
        return;
    };
    assert!(std::env::var_os("TESSERAE_SCREENSHOT").is_some());
    assert!(std::env::var_os("TESSERAE_SCREENSHOT_OFFSCREEN").is_none());
    let hz: u32 = hz.parse().expect("monitor Hz must be a positive integer");
    let millihertz = hz
        .checked_mul(1000)
        .filter(|hz| *hz > 0)
        .expect("invalid monitor Hz");
    app.insert_resource(Request {
        millihertz,
        borderless: std::env::var_os("TESSERAE_PROFILE_BORDERLESS").is_some(),
        started: None,
        selected: None,
        size: UVec2::new(
            dimension("TESSERAE_SCREENSHOT_WIDTH"),
            dimension("TESSERAE_SCREENSHOT_HEIGHT"),
        ),
    })
    .add_systems(PreUpdate, select.run_if(resource_exists::<Request>));
}

fn dimension(name: &str) -> u32 {
    std::env::var(name)
        .expect("monitor selection needs physical target dimensions")
        .parse::<u32>()
        .ok()
        .filter(|n| *n > 0)
        .expect("invalid physical target dimension")
}

fn matches(actual: Option<u32>, requested: u32) -> bool {
    actual.is_some_and(|actual| actual.abs_diff(requested) <= 1000)
}

fn select(
    mut commands: Commands,
    mut request: ResMut<Request>,
    monitors: Query<(Entity, &Monitor)>,
    mut windows: Query<(&mut Window, Option<&OnMonitor>)>,
) {
    let started = *request.started.get_or_insert_with(Instant::now);
    if let Some(selected) = request.selected {
        if windows
            .iter()
            .any(|(_, on)| on.is_some_and(|on| on.0 == selected))
        {
            let (_, monitor) = monitors
                .get(selected)
                .expect("selected monitor disappeared");
            assert!(matches(monitor.refresh_rate_millihertz, request.millihertz));
            if request.borderless {
                assert_eq!(
                    monitor.physical_size(),
                    request.size,
                    "borderless target must match the monitor's physical size"
                );
            }
            for (mut window, _) in &mut windows {
                if request.borderless {
                    window.mode =
                        WindowMode::BorderlessFullscreen(MonitorSelection::Entity(selected));
                }
                window
                    .resolution
                    .set_physical_resolution(request.size.x, request.size.y);
            }
            info!(
                "profile_monitor_selected refresh_millihertz={} physical={}x{} name={:?} borderless={}",
                monitor.refresh_rate_millihertz.unwrap(),
                monitor.physical_width,
                monitor.physical_height,
                monitor.name,
                request.borderless
            );
            commands.remove_resource::<Request>();
            return;
        }
    } else {
        let mut candidates = monitors
            .iter()
            .filter(|(_, monitor)| matches(monitor.refresh_rate_millihertz, request.millihertz));
        if let Some((entity, _)) = candidates.next() {
            assert!(
                candidates.next().is_none(),
                "multiple monitors match requested refresh rate"
            );
            for (mut window, _) in &mut windows {
                window.position = WindowPosition::Centered(MonitorSelection::Entity(entity));
            }
            request.selected = Some(entity);
        }
    }
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "requested monitor not found or window did not reach it within five seconds"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_uses_requested_monitor_and_sizes_after_arrival() {
        check_selection(false);
        check_selection(true);
    }

    fn check_selection(borderless: bool) {
        let mut app = App::new();
        app.insert_resource(Request {
            millihertz: 144_000,
            borderless,
            started: None,
            selected: None,
            size: UVec2::new(2560, 1440),
        });
        let monitor = |hz| Monitor {
            name: None,
            physical_height: 1440,
            physical_width: 2560,
            physical_position: IVec2::ZERO,
            refresh_rate_millihertz: Some(hz),
            scale_factor: 1.0,
            video_modes: Vec::new(),
        };
        let internal = app.world_mut().spawn(monitor(120_000)).id();
        let external = app.world_mut().spawn(monitor(144_000)).id();
        let window = app
            .world_mut()
            .spawn((Window::default(), OnMonitor(internal)))
            .id();
        app.add_systems(Update, select.run_if(resource_exists::<Request>));
        app.update();
        assert_eq!(app.world().resource::<Request>().selected, Some(external));
        assert_eq!(
            app.world().get::<Window>(window).unwrap().position,
            WindowPosition::Centered(MonitorSelection::Entity(external))
        );
        app.world_mut()
            .entity_mut(window)
            .insert(OnMonitor(external));
        app.update();
        assert!(!app.world().contains_resource::<Request>());
        assert_eq!(
            app.world().get::<Window>(window).unwrap().mode,
            if borderless {
                WindowMode::BorderlessFullscreen(MonitorSelection::Entity(external))
            } else {
                WindowMode::Windowed
            }
        );
        assert_eq!(
            app.world().get::<Window>(window).unwrap().physical_size(),
            UVec2::new(2560, 1440)
        );
    }

    #[test]
    fn rounded_refresh_matches_but_missing_or_other_display_does_not() {
        assert!(matches(Some(143_970), 144_000));
        assert!(!matches(Some(120_000), 144_000));
        assert!(!matches(None, 144_000));
    }
}
