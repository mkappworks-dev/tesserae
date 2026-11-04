//! Eases the sky and fog to the pair the Server sends for where the Eye is.
//!
//! Sky and fog take one step together, so the terrain never ends in an edge against the sky
//! mid-ease.

use bevy::prelude::*;

use crate::content::SkyFogTarget;
use crate::render::fog::{self, FogSettings};
use crate::session::{SessionApp, SessionTeardown};

/// Time constant: slow enough that a border walked across reads as the air changing, not flicker.
const EASE_TIME_S: f32 = 1.5;

/// A hitch (a World loading, a window dragged) must not jump the air in one frame.
const MAX_STEP_S: f32 = 0.1;

/// Under half a byte is invisible, and stopping there lets [`FogSettings`] settle instead of
/// changing forever.
const SETTLED: f32 = 0.5 / 255.0;

pub(super) fn plugin(app: &mut App) {
    app.init_session_state::<Goal>()
        .add_session_systems(Update, ease.before(fog::follow_settings))
        .add_session_systems(SessionTeardown, restore_haze);
}

/// `None` until the Server asks, which a World whose Biomes carry no colours never does.
#[derive(Resource, Default)]
struct Goal(Option<SkyFogTarget>);

fn step(now: [f32; 3], goal: [f32; 3], k: f32) -> [f32; 3] {
    std::array::from_fn(|i| now[i] + (goal[i] - now[i]) * k)
}

fn within(a: [f32; 3], b: [f32; 3]) -> bool {
    (0..3).all(|i| (a[i] - b[i]).abs() < SETTLED)
}

fn ease(
    mut targets: MessageReader<SkyFogTarget>,
    time: Res<Time>,
    mut goal: ResMut<Goal>,
    mut settings: ResMut<FogSettings>,
) {
    if let Some(target) = targets.read().last() {
        goal.0 = Some(*target);
    }
    let Some(goal) = goal.0 else {
        return;
    };
    if settings.sky == goal.sky && settings.color == goal.fog {
        return;
    }
    let k = 1.0 - (-time.delta_secs().min(MAX_STEP_S) / EASE_TIME_S).exp();
    let (sky, fog) = (
        step(settings.sky, goal.sky, k),
        step(settings.color, goal.fog, k),
    );
    // Both or neither arrive, so the pair is never apart even for a frame.
    let (sky, fog) = if within(sky, goal.sky) && within(fog, goal.fog) {
        (goal.sky, goal.fog)
    } else {
        (sky, fog)
    };
    settings.sky = sky;
    settings.color = fog;
}

/// The settings outlive a World, so the next one starts from the haze, not the last one's air.
fn restore_haze(mut settings: ResMut<FogSettings>) {
    let haze = FogSettings::default();
    settings.sky = haze.sky;
    settings.color = haze.color;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::camera::MainCamera;
    use crate::client::pose::PublishedBody;
    use crate::content::MaterialPalette;
    use crate::server::streaming::StreamingSettings;
    use bevy::pbr::DistanceFog;
    use bevy::time::TimeUpdateStrategy;
    use std::time::Duration;
    use tesserae_core::immersion::FluidSurface;

    const FRAME: Duration = Duration::from_millis(16);

    const DRY: SkyFogTarget = SkyFogTarget {
        sky: [0.69, 0.63, 0.5],
        fog: [0.77, 0.7, 0.56],
    };

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(TimeUpdateStrategy::ManualDuration(FRAME))
            .init_resource::<StreamingSettings>()
            .add_message::<SkyFogTarget>()
            .add_plugins((fog::plugin, plugin));
        // The first update only starts the clock.
        app.update();
        app
    }

    fn settings(app: &App) -> FogSettings {
        *app.world().resource::<FogSettings>()
    }

    fn send(app: &mut App, target: SkyFogTarget) {
        app.world_mut().write_message(target);
    }

    fn remaining(now: [f32; 3], goal: [f32; 3]) -> f32 {
        (0..3).map(|i| (goal[i] - now[i]).abs()).fold(0.0, f32::max)
    }

    #[test]
    fn with_nothing_sent_the_haze_stays_as_it_was() {
        let mut app = app();
        for _ in 0..200 {
            app.update();
        }
        assert_eq!(settings(&app), FogSettings::default());
    }

    #[test]
    fn a_target_is_eased_towards_and_reached_without_overshoot() {
        let mut app = app();
        let start = settings(&app);
        send(&mut app, DRY);
        app.update();
        let first = settings(&app);
        assert_ne!(first.sky, start.sky, "it starts moving at once");
        assert!(remaining(first.sky, DRY.sky) > 0.5 * remaining(start.sky, DRY.sky));

        let mut last = remaining(first.sky, DRY.sky);
        for _ in 0..1500 {
            app.update();
            let now = remaining(settings(&app).sky, DRY.sky);
            assert!(now <= last, "it never moves away");
            last = now;
        }
        assert_eq!(settings(&app).sky, DRY.sky);
        assert_eq!(settings(&app).color, DRY.fog);
        assert_eq!(settings(&app).reach_opacity, start.reach_opacity);
    }

    #[test]
    fn sky_and_fog_cover_the_same_share_of_their_way_every_frame() {
        let mut app = app();
        let start = settings(&app);
        send(&mut app, DRY);
        for frame in 0..400 {
            app.update();
            let now = settings(&app);
            for c in 0..3 {
                let sky = (now.sky[c] - start.sky[c]) / (DRY.sky[c] - start.sky[c]);
                let fog = (now.color[c] - start.color[c]) / (DRY.fog[c] - start.color[c]);
                assert!((sky - fog).abs() < 1e-3, "frame {frame}, channel {c}");
            }
        }
    }

    #[test]
    fn a_long_frame_cannot_snap_the_air() {
        let mut app = app();
        let start = settings(&app);
        send(&mut app, DRY);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs(30)));
        app.update();
        let after = settings(&app);
        let moved = remaining(start.sky, after.sky);
        let way = remaining(start.sky, DRY.sky);
        assert!(moved > 0.0 && moved < 0.15 * way, "{moved} of {way}");
        assert_ne!(after.sky, DRY.sky);
    }

    #[test]
    fn a_new_target_mid_ease_turns_the_air_round_without_a_jump() {
        let mut app = app();
        send(&mut app, DRY);
        for _ in 0..60 {
            app.update();
        }
        let before = settings(&app);
        let cold = SkyFogTarget {
            sky: [0.4, 0.55, 0.78],
            fog: [0.47, 0.63, 0.86],
        };
        send(&mut app, cold);
        app.update();
        let after = settings(&app);
        assert!(remaining(before.sky, after.sky) < 0.02);
        for _ in 0..1500 {
            app.update();
        }
        assert_eq!(settings(&app).sky, cold.sky);
        assert_eq!(settings(&app).color, cold.fog);
    }

    #[test]
    fn the_clear_colour_and_every_cameras_fog_follow_the_pair() {
        let mut app = app();
        let camera = app
            .world_mut()
            .spawn(fog::distance_fog(&FogSettings::default(), 192.0, 4096.0))
            .id();
        app.update();
        send(&mut app, DRY);
        for _ in 0..1500 {
            app.update();
        }
        let [r, g, b] = DRY.sky;
        assert_eq!(app.world().resource::<ClearColor>().0, Color::srgb(r, g, b));
        let [r, g, b] = DRY.fog;
        assert_eq!(
            app.world().get::<DistanceFog>(camera).unwrap().color,
            Color::srgb(r, g, b)
        );
    }

    #[test]
    fn a_new_world_starts_from_the_haze_whatever_the_last_one_left() {
        let mut app = app();
        send(&mut app, DRY);
        for _ in 0..1500 {
            app.update();
        }
        app.world_mut().run_schedule(SessionTeardown);
        assert_eq!(settings(&app), FogSettings::default());
    }

    mod underwater {
        use super::*;
        use crate::client::underwater;

        const WATER: u16 = 14;
        const WATER_RGBA: [u8; 4] = [20, 60, 120, 128];

        fn wet_app() -> (App, Entity) {
            let mut app = app();
            app.init_resource::<PublishedBody>()
                .add_message::<MaterialPalette>()
                .add_plugins(underwater::plugin);
            let camera = app
                .world_mut()
                .spawn((
                    MainCamera,
                    Camera3d::default(),
                    Transform::default(),
                    fog::distance_fog(&FogSettings::default(), 192.0, 4096.0),
                ))
                .id();
            app.world_mut()
                .write_message(MaterialPalette(vec![(WATER, WATER_RGBA)]));
            app.update();
            (app, camera)
        }

        fn dive(app: &mut App, camera: Entity, y: f32) {
            app.world_mut()
                .get_mut::<Transform>(camera)
                .unwrap()
                .translation
                .y = y;
            app.world_mut().resource_mut::<PublishedBody>().0.immersion = Some(FluidSurface {
                material: WATER,
                surface_y_m: 10.0,
                dimming: 2,
            });
        }

        #[test]
        fn biome_air_never_shows_underwater_and_is_in_place_on_surfacing() {
            let (mut app, camera) = wet_app();
            dive(&mut app, camera, 9.0);
            app.update();
            let water = app.world().resource::<ClearColor>().0;
            let water_fog = app.world().get::<DistanceFog>(camera).unwrap().color;

            send(&mut app, DRY);
            for _ in 0..1500 {
                app.update();
                assert_eq!(app.world().resource::<ClearColor>().0, water);
                assert_eq!(
                    app.world().get::<DistanceFog>(camera).unwrap().color,
                    water_fog
                );
            }

            dive(&mut app, camera, 12.0);
            app.update();
            let [r, g, b] = DRY.sky;
            assert_eq!(app.world().resource::<ClearColor>().0, Color::srgb(r, g, b));
            let [r, g, b] = DRY.fog;
            assert_eq!(
                app.world().get::<DistanceFog>(camera).unwrap().color,
                Color::srgb(r, g, b)
            );
        }

        #[test]
        fn surfacing_mid_ease_shows_the_pair_as_eased_underwater_not_the_haze() {
            let (mut app, camera) = wet_app();
            dive(&mut app, camera, 9.0);
            app.update();
            send(&mut app, DRY);
            for _ in 0..40 {
                app.update();
            }
            let eased = settings(&app);
            assert_ne!(
                eased.sky,
                FogSettings::default().sky,
                "it kept easing under water"
            );
            assert_ne!(eased.sky, DRY.sky, "and is not there yet");

            dive(&mut app, camera, 12.0);
            app.update();
            let eased = settings(&app);
            let [r, g, b] = eased.sky;
            assert_eq!(app.world().resource::<ClearColor>().0, Color::srgb(r, g, b));
            let [r, g, b] = eased.color;
            assert_eq!(
                app.world().get::<DistanceFog>(camera).unwrap().color,
                Color::srgb(r, g, b)
            );
        }
    }
}
