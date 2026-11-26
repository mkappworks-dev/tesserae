//! Eases the Biome pair the Server sends for where the Eye is, and presents the Sky state over it
//! every frame: the light, the ambient dome, the sky and the fog. Presentation only; nothing here
//! writes World state (ADR-0002).
//!
//! Sky and fog take one step together, so the terrain never ends in an edge against the sky
//! mid-ease. The pair is eased as declared and tinted only on the way out: easing the tinted
//! colours would tint them again every frame.

use bevy::prelude::*;
use tesserae_core::sky::{Air, SkyInputs, SkyState, sky_state};

use super::chunk_view::{SKY_AMBIENT, SUN_LUX};
use super::time_of_day::{self, ClientTimeOfDay};
use crate::content::SkyFogTarget;
use crate::render::fog::{self, FogSettings};
use crate::session::{SessionApp, SessionTeardown};

/// Time constant: slow enough that a border walked across reads as the air changing, not flicker.
const EASE_TIME_S: f32 = 1.5;

/// A hitch (a World loading, a window dragged) must not jump the air in one frame.
const MAX_STEP_S: f32 = 0.1;

/// Under half a byte is invisible, and stopping there lets the pair settle instead of changing
/// forever.
const SETTLED: f32 = 0.5 / 255.0;

pub(super) fn plugin(app: &mut App) {
    app.init_session_state::<Goal>()
        .init_session_state::<EasedAir>()
        .init_session_state::<SkyNow>()
        .add_session_systems(
            Update,
            (ease, present)
                .chain()
                .after(time_of_day::follow)
                .before(fog::follow_settings),
        )
        .add_session_systems(SessionTeardown, restore_haze);
}

/// `None` until the Server asks, which a World with no Biomes never does.
#[derive(Resource, Default)]
struct Goal(Option<SkyFogTarget>);

/// The Biome pair as eased toward the Server's, untinted.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct EasedAir(pub Air);

impl Default for EasedAir {
    fn default() -> Self {
        let haze = FogSettings::default();
        Self(Air {
            sky: haze.sky,
            fog: haze.color,
            underground: 0.0,
        })
    }
}

/// The Sky state this frame, which the sky shader and the underwater look read.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct SkyNow(pub SkyState);

impl Default for SkyNow {
    fn default() -> Self {
        Self(sky_state(&SkyInputs {
            hour: 12.0,
            moon_phase: 0,
            sky: None,
            air: EasedAir::default().0,
        }))
    }
}

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
    mut eased: ResMut<EasedAir>,
) {
    if let Some(target) = targets.read().last() {
        goal.0 = Some(*target);
    }
    let Some(goal) = goal.0 else {
        return;
    };
    let want = Air {
        sky: goal.sky,
        fog: goal.fog,
        underground: if goal.cave { 1.0 } else { 0.0 },
    };
    if eased.0 == want {
        return;
    }
    let k = 1.0 - (-time.delta_secs().min(MAX_STEP_S) / EASE_TIME_S).exp();
    let now = eased.0;
    // The cave's share eases with the colours, so the hour's tint fades as the air turns.
    let next = Air {
        sky: step(now.sky, want.sky, k),
        fog: step(now.fog, want.fog, k),
        underground: now.underground + (want.underground - now.underground) * k,
    };
    // All or nothing arrives, so the pair is never apart even for a frame.
    let arrived = within(next.sky, want.sky)
        && within(next.fog, want.fog)
        && (next.underground - want.underground).abs() < SETTLED;
    eased.0 = if arrived { want } else { next };
}

/// Every write is skipped when nothing changed, so a still sky costs no re-extraction.
fn present(
    clock: Res<ClientTimeOfDay>,
    eased: Res<EasedAir>,
    mut now: ResMut<SkyNow>,
    mut settings: ResMut<FogSettings>,
    ambient: Option<ResMut<GlobalAmbientLight>>,
    mut lights: Query<(&mut DirectionalLight, &mut Transform)>,
) {
    let state = sky_state(&SkyInputs {
        hour: clock.hour(),
        moon_phase: clock.moon_phase(),
        sky: clock.sky(),
        air: eased.0,
    });
    now.set_if_neq(SkyNow(state));
    let fogged = FogSettings {
        sky: state.sky,
        color: state.fog,
        // None with no sky, so a World without one keeps today's fog to the bit.
        glow: clock.sky().map(|_| state.light.colour),
        ..*settings
    };
    settings.set_if_neq(fogged);
    if let Some(mut ambient) = ambient {
        let [r, g, b] = state.ambient.colour;
        let (color, brightness) = (
            Color::linear_rgb(r, g, b),
            SKY_AMBIENT * state.ambient.strength,
        );
        if ambient.color != color || ambient.brightness != brightness {
            ambient.color = color;
            ambient.brightness = brightness;
        }
    }
    // One shadowed light, the sun by day and the moon by night, so there is one shadow caster at
    // every hour. Rolled about the pole, which is never along the light, so the shadow map does not
    // spin as the light crosses overhead.
    let [r, g, b] = state.light.colour;
    let color = Color::linear_rgb(r, g, b);
    let illuminance = SUN_LUX * state.light.strength;
    let place_now = Transform::from_translation(Vec3::from(state.toward_light))
        .looking_at(Vec3::ZERO, Vec3::from(state.pole));
    for (mut light, mut place) in &mut lights {
        if light.color != color || light.illuminance != illuminance {
            light.color = color;
            light.illuminance = illuminance;
        }
        place.set_if_neq(place_now);
    }
}

/// The settings outlive a World, so the next one starts from the haze, not the last one's air.
fn restore_haze(mut settings: ResMut<FogSettings>) {
    let haze = FogSettings::default();
    settings.sky = haze.sky;
    settings.color = haze.color;
    settings.glow = haze.glow;
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
        cave: false,
    };

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(TimeUpdateStrategy::ManualDuration(FRAME))
            .init_resource::<StreamingSettings>()
            .add_message::<SkyFogTarget>()
            .add_plugins((fog::plugin, time_of_day::plugin, plugin));
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
            cave: false,
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

    mod day_and_night {
        use super::*;
        use crate::content::{TimeOfDaySync, WorldSky};
        use bevy::camera::visibility::RenderLayers;
        use bevy::ecs::system::RunSystemOnce;
        use tesserae_core::sky::{LightSource, Pin, SkyParams};

        const TERRA: SkyParams = SkyParams {
            day_length_s: 1200.0,
            start_hour: 7.0,
            day_share: 0.7,
            tilt_deg: 25.0,
        };

        /// Holds the Client's clock at `when` ("18:30", "4 00:00") under Terra's sky.
        fn hold(app: &mut App, when: &str) {
            let time = Pin::parse(when).unwrap().time(&TERRA);
            app.world_mut().write_message(WorldSky(Some(TERRA)));
            app.world_mut().write_message(TimeOfDaySync {
                time,
                steps_per_s: 0.0,
            });
        }

        fn settle(app: &mut App) {
            for _ in 0..1500 {
                app.update();
            }
        }

        fn expected(app: &App, target: SkyFogTarget) -> SkyState {
            let clock = app.world().resource::<ClientTimeOfDay>();
            sky_state(&SkyInputs {
                hour: clock.hour(),
                moon_phase: clock.moon_phase(),
                sky: clock.sky(),
                air: Air {
                    sky: target.sky,
                    fog: target.fog,
                    underground: if target.cave { 1.0 } else { 0.0 },
                },
            })
        }

        /// With the light and the dome the Client spawns.
        fn lit_app() -> (App, Entity) {
            let mut app = app();
            app.init_resource::<GlobalAmbientLight>();
            app.world_mut()
                .run_system_once(crate::client::chunk_view::spawn_light)
                .unwrap();
            let light = app
                .world_mut()
                .query_filtered::<Entity, With<DirectionalLight>>()
                .single(app.world())
                .unwrap();
            (app, light)
        }

        fn toward(app: &App, light: Entity) -> Vec3 {
            app.world()
                .get::<Transform>(light)
                .unwrap()
                .back()
                .as_vec3()
        }

        #[test]
        fn at_dusk_the_light_the_dome_the_fog_and_the_clear_colour_follow_the_sky_state() {
            let (mut app, light) = lit_app();
            hold(&mut app, "18:30");
            send(&mut app, DRY);
            settle(&mut app);
            let want = expected(&app, DRY);
            assert_ne!(want.sky, DRY.sky, "dusk tints the air");
            assert_eq!(app.world().resource::<SkyNow>().0, want);
            let fog = settings(&app);
            assert_eq!((fog.sky, fog.color), (want.sky, want.fog));
            assert_eq!(fog.glow, Some(want.light.colour));
            let [r, g, b] = want.sky;
            assert_eq!(app.world().resource::<ClearColor>().0, Color::srgb(r, g, b));
            let ambient = app.world().resource::<GlobalAmbientLight>();
            assert_eq!(ambient.brightness, SKY_AMBIENT * want.ambient.strength);
            let sun = app.world().get::<DirectionalLight>(light).unwrap();
            assert_eq!(sun.illuminance, SUN_LUX * want.light.strength);
            assert!(toward(&app, light).distance(Vec3::from(want.toward_light)) < 1e-5);
        }

        #[test]
        fn the_eased_pair_is_tinted_once_and_never_fed_back() {
            let mut app = app();
            hold(&mut app, "18:30");
            send(&mut app, DRY);
            settle(&mut app);
            let eased = app.world().resource::<EasedAir>().0;
            assert_eq!(
                (eased.sky, eased.fog, eased.underground),
                (DRY.sky, DRY.fog, 0.0),
                "the ease holds the Biome's own colours"
            );
            let once = settings(&app);
            for _ in 0..300 {
                app.update();
            }
            assert_eq!(
                settings(&app),
                once,
                "a held dusk does not darken frame by frame"
            );
            assert_eq!(once.sky, expected(&app, DRY).sky);
        }

        #[test]
        fn a_cave_biomes_air_is_the_same_at_noon_and_at_midnight() {
            let cave = SkyFogTarget {
                sky: [0.04, 0.08, 0.24],
                fog: [0.08, 0.16, 0.47],
                cave: true,
            };
            for when in ["12:00", "00:00", "4 00:00"] {
                let mut app = app();
                hold(&mut app, when);
                send(&mut app, cave);
                settle(&mut app);
                let fog = settings(&app);
                assert_eq!((fog.sky, fog.color), (cave.sky, cave.fog), "{when}");
            }
        }

        #[test]
        fn at_midnight_the_one_shadowed_light_is_the_moon_and_the_dome_dims() {
            let (mut app, light) = lit_app();
            let layers = app.world().get::<RenderLayers>(light).cloned();
            hold(&mut app, "00:00");
            app.update();
            app.update();
            let want = app.world().resource::<SkyNow>().0;
            assert_eq!(want.light_source, LightSource::Moon);
            let lights: Vec<Entity> = app
                .world_mut()
                .query_filtered::<Entity, With<DirectionalLight>>()
                .iter(app.world())
                .collect();
            assert_eq!(lights, [light], "one shadow caster at every hour");
            let moon = app.world().get::<DirectionalLight>(light).unwrap();
            assert!(moon.shadow_maps_enabled);
            assert!(moon.illuminance > 0.0 && moon.illuminance < 0.5 * SUN_LUX);
            assert!(toward(&app, light).distance(Vec3::from(want.toward_moon)) < 1e-5);
            assert_eq!(
                app.world().get::<RenderLayers>(light).cloned(),
                layers,
                "it lights every level it did by day"
            );
            assert!(app.world().resource::<GlobalAmbientLight>().brightness < SKY_AMBIENT);
        }

        #[test]
        fn with_no_sky_the_light_and_the_dome_are_todays_noon() {
            let (mut app, light) = lit_app();
            for _ in 0..3 {
                app.update();
            }
            let sun = app.world().get::<DirectionalLight>(light).unwrap();
            assert_eq!(sun.illuminance, SUN_LUX);
            assert_eq!(
                app.world().resource::<GlobalAmbientLight>().brightness,
                SKY_AMBIENT
            );
            let today = Vec3::new(1.0, 2.2, 0.8).normalize();
            assert!(toward(&app, light).distance(today) < 1e-5);
            assert_eq!(settings(&app).glow, None);
        }
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
                emits: false,
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
