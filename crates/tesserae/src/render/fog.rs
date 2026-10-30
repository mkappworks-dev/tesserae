//! Distance fog: two plain numbers, and the density they imply.
//!
//! What a Look pack sets is a colour and an opacity at the reach: the share of a
//! surface's colour that fog replaces at the farthest drawn terrain. Neither is
//! a Voxel quantity. The density Bevy wants is **derived** from them and from
//! the two streaming distances, so the opacity at the reach is the same at any
//! reach, and a pack never gives a density that is right for one reach only.
//!
//! Everything physical here is in metres (ADR-0003): the fog is built from the
//! two distances and the settings alone, with no Voxel size, so the same
//! settings over a World with 0.25 m Voxels give the same fog.

use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;

use crate::server::streaming::StreamingSettings;

/// Floor for the reach, so a World that draws nothing cannot divide by zero.
const MIN_REACH_M: f32 = 1.0;

/// Ceiling for `reach_opacity`: `ln(1 - 1)` is minus infinity, and one ULP
/// below 1 is the largest opacity that still gives a finite density.
const MAX_REACH_OPACITY: f32 = 1.0 - f32::EPSILON;

/// The haze a fully fogged surface shows, as sRGB bytes.
const HAZE_SKY_SRGB: [u8; 3] = [126, 146, 168];

/// What the shader fogs toward so that a fully fogged surface comes out as
/// [`HAZE_SKY_SRGB`]: the tonemapper runs after the fog and the clear colour
/// skips it, and it darkens this colour by (10, 15, 21). Measured offscreen
/// to within 1/255; re-measure if the tonemapper or the exposure changes.
const HAZE_FOG_SRGB: [u8; 3] = [136, 160, 190];

fn srgb(bytes: [u8; 3]) -> [f32; 3] {
    bytes.map(|b| f32::from(b) / 255.0)
}

/// The fog a Look pack sets, as plain data.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct FogSettings {
    /// sRGB, what the shader fogs toward. Not the colour the fog is seen as:
    /// see [`HAZE_FOG_SRGB`].
    pub color: [f32; 3],
    /// sRGB, the clear colour. A pack that changes `color` must change this
    /// with it, or the terrain ends in a visible edge against the sky.
    pub sky: [f32; 3],
    /// The share of a surface's colour that fog replaces at the reach. 0 is no
    /// fog. 1 is clamped to just under it, so the reach is fully fogged but the
    /// density stays finite. Outside 0 to 1, and NaN, clamp to the nearest end
    /// (NaN to 0).
    pub reach_opacity: f32,
}

impl Default for FogSettings {
    fn default() -> Self {
        FogSettings {
            color: srgb(HAZE_FOG_SRGB),
            sky: srgb(HAZE_SKY_SRGB),
            reach_opacity: 0.88,
        }
    }
}

/// Inserts the settings and sets the clear colour from them, so the terrain ends
/// in the colour behind it. At build time, so a later plugin's `ClearColor`
/// (the capture probe's) wins; `follow_settings` keeps the two in step
/// afterwards without taking that away.
pub fn plugin(app: &mut App) {
    app.insert_resource(built_settings());
    let [r, g, b] = app.world().resource::<FogSettings>().sky;
    app.insert_resource(ClearColor(Color::srgb(r, g, b)));
    app.add_systems(Update, follow_settings);
}

/// Re-derives the clear colour and every camera's fog when [`FogSettings`] is
/// changed after startup, so a Look pack that sets it at run time takes effect
/// without respawning the cameras.
pub(crate) fn follow_settings(
    settings: Res<FogSettings>,
    streaming: Res<StreamingSettings>,
    mut clear: ResMut<ClearColor>,
    mut cameras: Query<&mut DistanceFog>,
) {
    // A resource reads as changed on the first run after it is inserted, and the
    // capture probe inserts its own `ClearColor` after this plugin: re-applying
    // the sky then would overwrite the probe's magenta.
    if !settings.is_changed() || settings.is_added() {
        return;
    }
    let [r, g, b] = settings.sky;
    clear.0 = Color::srgb(r, g, b);
    let fog = distance_fog(
        &settings,
        streaming.view_distance_m,
        streaming.lod_distance_m,
    );
    for mut camera in &mut cameras {
        *camera = fog.clone();
    }
}

/// How far the farthest drawn terrain is: `lod_distance_m`, unless the World
/// has no coarse levels (`lod_distance_m <= view_distance_m`).
///
/// A non-positive or NaN result is floored to [`MIN_REACH_M`].
fn reach_m(view_distance_m: f32, lod_distance_m: f32) -> f32 {
    // `f32::max` returns the other operand for a NaN, so a NaN distance
    // falls through to the floor instead of poisoning the density.
    view_distance_m.max(lod_distance_m).max(MIN_REACH_M)
}

/// `1 - exp(-(d D)^2) = reach_opacity` at `d = reach_m`, solved for `D`.
fn density(settings: &FogSettings, reach_m: f32) -> f32 {
    let opacity = if settings.reach_opacity.is_nan() {
        0.0
    } else {
        settings.reach_opacity.clamp(0.0, MAX_REACH_OPACITY)
    };
    // `-ln(1)` is `-0.0` and its root is `-0.0`; adding zero makes it `+0.0`.
    ((-(1.0 - opacity).ln()).sqrt() + 0.0) / reach_m
}

/// The fog for a camera, from the settings and the two streaming distances in
/// metres.
///
/// No Voxel size is taken, deliberately: the fog is the same at any Voxel size
/// (ADR-0003). The falloff is exponential squared, which is under 2% out to
/// the join of levels 1 and 2 at the default distances, so the full-detail
/// rings are not hazed. An opacity of 0 gives a density of 0 and no fog.
pub fn distance_fog(
    settings: &FogSettings,
    view_distance_m: f32,
    lod_distance_m: f32,
) -> DistanceFog {
    let [r, g, b] = settings.color;
    DistanceFog {
        color: Color::srgb(r, g, b),
        falloff: FogFalloff::ExponentialSquared {
            density: density(settings, reach_m(view_distance_m, lod_distance_m)),
        },
        ..default()
    }
}

/// What the shader fogs toward so that a surface fully fogged comes out as
/// `seen` (sRGB): the tonemapper darkens the fog colour, see [`HAZE_FOG_SRGB`].
pub fn fog_toward(seen: [f32; 3]) -> [f32; 3] {
    let lift = |i: usize| f32::from(HAZE_FOG_SRGB[i] - HAZE_SKY_SRGB[i]) / 255.0;
    [0, 1, 2].map(|i| (seen[i] + lift(i)).min(1.0))
}

/// The fog every terrain camera gets, or `None` for none at all. A zero
/// opacity is not "off": it would still pay for the fog binding and the branch.
pub fn camera_fog(
    enabled: bool,
    settings: &FogSettings,
    view_distance_m: f32,
    lod_distance_m: f32,
) -> Option<DistanceFog> {
    enabled.then(|| distance_fog(settings, view_distance_m, lod_distance_m))
}

/// `TESSERAE_FOG=0` turns the fog off, to measure what it costs and to capture
/// the same view without it.
#[cfg(feature = "dev")]
pub fn enabled() -> bool {
    !std::env::var("TESSERAE_FOG").is_ok_and(|v| v == "0")
}

#[cfg(not(feature = "dev"))]
pub fn enabled() -> bool {
    true
}

/// `TESSERAE_FOG_OPACITY=<float>` overrides `reach_opacity`, so the fog and sky
/// pair can be re-measured at a nearly opaque fog — which a change to the
/// tonemapper, the exposure or the colours calls for — without editing this
/// file for the measurement. [`FogSettings::default`] stays untouched data.
#[cfg(feature = "dev")]
fn built_settings() -> FogSettings {
    with_opacity_override(
        FogSettings::default(),
        std::env::var("TESSERAE_FOG_OPACITY").ok().as_deref(),
    )
}

/// An absent or unparsable value leaves the settings as they are; a value out of
/// range is clamped where the density is derived, as any other is.
#[cfg(feature = "dev")]
fn with_opacity_override(mut settings: FogSettings, opacity: Option<&str>) -> FogSettings {
    if let Some(reach_opacity) = opacity.and_then(|v| v.trim().parse().ok()) {
        settings.reach_opacity = reach_opacity;
    }
    settings
}

#[cfg(not(feature = "dev"))]
fn built_settings() -> FogSettings {
    FogSettings::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEFAULT_VIEW_M: f32 = 192.0;
    const DEFAULT_LOD_M: f32 = 4096.0;

    fn density_of(fog: &DistanceFog) -> f32 {
        match fog.falloff {
            FogFalloff::ExponentialSquared { density } => density,
            ref other => panic!("expected exponential squared, got {other:?}"),
        }
    }

    /// The shader's own formula, in f64 so the check is not limited by f32.
    fn opacity_at(fog: &DistanceFog, distance_m: f32) -> f64 {
        let x = f64::from(distance_m) * f64::from(density_of(fog));
        1.0 - (-(x * x)).exp()
    }

    fn with_opacity(reach_opacity: f32) -> FogSettings {
        FogSettings {
            reach_opacity,
            ..default()
        }
    }

    #[test]
    fn opacity_at_the_reach_is_reach_opacity_for_several_reaches_and_opacities() {
        for reach_m in [16.0, 192.0, 1000.0, 4096.0, 30_000.0] {
            for reach_opacity in [0.05, 0.3, 0.5, 0.85, 0.99] {
                let fog = distance_fog(&with_opacity(reach_opacity), 8.0, reach_m);
                let got = opacity_at(&fog, reach_m);
                assert!(
                    (got - f64::from(reach_opacity)).abs() < 1e-5,
                    "reach {reach_m} m, wanted {reach_opacity}, got {got}"
                );
            }
        }
    }

    #[test]
    fn opacity_is_zero_at_zero_and_grows_with_distance() {
        let fog = distance_fog(&FogSettings::default(), DEFAULT_VIEW_M, DEFAULT_LOD_M);
        assert_eq!(opacity_at(&fog, 0.0), 0.0);
        let mut last = 0.0;
        for step in 1..=80 {
            let opacity = opacity_at(&fog, step as f32 * 64.0);
            assert!(opacity > last, "step {step}: {opacity} after {last}");
            last = opacity;
        }
    }

    #[test]
    fn default_fog_leaves_the_full_detail_rings_unhazed() {
        let fog = distance_fog(&FogSettings::default(), DEFAULT_VIEW_M, DEFAULT_LOD_M);
        // Edit these with the reason if a default changes: they are the spec's figures.
        assert!(opacity_at(&fog, 192.0) < 0.01, "level 0 edge");
        assert!(opacity_at(&fog, 384.0) < 0.02, "join of levels 1 and 2");
    }

    #[test]
    fn fog_is_built_from_the_two_distances_and_the_settings_alone() {
        // Pinning the signature is the test: no parameter can carry a Voxel size.
        let _: fn(&FogSettings, f32, f32) -> DistanceFog = distance_fog;
        let a = distance_fog(&FogSettings::default(), DEFAULT_VIEW_M, DEFAULT_LOD_M);
        let b = distance_fog(&FogSettings::default(), DEFAULT_VIEW_M, DEFAULT_LOD_M);
        assert_eq!(density_of(&a), density_of(&b));
        assert_eq!(a.color, b.color);
    }

    #[test]
    fn a_world_with_no_coarse_levels_fogs_to_reach_opacity_at_the_view_distance() {
        for lod_distance_m in [DEFAULT_VIEW_M, 100.0, 0.0] {
            let fog = distance_fog(&FogSettings::default(), DEFAULT_VIEW_M, lod_distance_m);
            let got = opacity_at(&fog, DEFAULT_VIEW_M);
            let want = f64::from(FogSettings::default().reach_opacity);
            assert!((got - want).abs() < 1e-5, "lod {lod_distance_m} m: {got}");
        }
    }

    #[test]
    fn reach_opacity_of_zero_gives_no_fog() {
        let fog = distance_fog(&with_opacity(0.0), DEFAULT_VIEW_M, DEFAULT_LOD_M);
        assert_eq!(density_of(&fog), 0.0);
        assert!(density_of(&fog).is_sign_positive());
        assert_eq!(opacity_at(&fog, DEFAULT_LOD_M), 0.0);
    }

    #[test]
    fn reach_opacity_of_one_and_a_reach_of_zero_give_finite_numbers() {
        for settings in [with_opacity(1.0), with_opacity(7.0), with_opacity(f32::NAN)] {
            for (view_m, lod_m) in [
                (DEFAULT_VIEW_M, DEFAULT_LOD_M),
                (0.0, 0.0),
                (-5.0, -1.0),
                (f32::NAN, f32::NAN),
            ] {
                let density = density_of(&distance_fog(&settings, view_m, lod_m));
                assert!(
                    density.is_finite() && density >= 0.0,
                    "{settings:?} over {view_m}/{lod_m} gave {density}"
                );
            }
        }
    }

    #[test]
    fn reach_opacity_of_one_is_nearly_opaque_at_the_reach() {
        let fog = distance_fog(&with_opacity(1.0), DEFAULT_VIEW_M, DEFAULT_LOD_M);
        assert!(opacity_at(&fog, DEFAULT_LOD_M) > 0.9999);
    }

    #[test]
    fn the_clear_colour_is_the_sky_and_not_the_fog_colour_in_the_default_app() {
        let mut app = App::new();
        app.add_plugins(plugin);
        let [r, g, b] = FogSettings::default().sky;
        assert_eq!(app.world().resource::<ClearColor>().0, Color::srgb(r, g, b));
        assert_eq!(
            *app.world().resource::<FogSettings>(),
            FogSettings::default()
        );
    }

    #[test]
    fn the_default_fog_colour_is_the_sky_lifted_by_the_tonemapper_darkening() {
        let FogSettings { color, sky, .. } = FogSettings::default();
        assert_eq!(color, srgb(HAZE_FOG_SRGB));
        assert_eq!(sky, srgb(HAZE_SKY_SRGB));
        // The tonemapper only darkens a colour this bright, so equal values are the mistake.
        for (fog, sky) in HAZE_FOG_SRGB.into_iter().zip(HAZE_SKY_SRGB) {
            assert!(fog > sky, "fog {fog} against sky {sky}");
        }
    }

    #[test]
    fn a_later_clear_colour_wins_over_the_plugins() {
        let mut app = App::new();
        app.add_plugins(plugin);
        app.insert_resource(ClearColor(Color::srgb(1.0, 0.0, 1.0)));
        assert_eq!(
            app.world().resource::<ClearColor>().0,
            Color::srgb(1.0, 0.0, 1.0)
        );
    }

    #[test]
    fn switching_the_fog_off_gives_no_component_and_on_gives_the_derived_one() {
        let settings = FogSettings::default();
        assert!(camera_fog(false, &settings, DEFAULT_VIEW_M, DEFAULT_LOD_M).is_none());
        let fog = camera_fog(true, &settings, DEFAULT_VIEW_M, DEFAULT_LOD_M).unwrap();
        assert!(density_of(&fog) > 0.0);
    }

    /// `MinimalPlugins` is not needed: `follow_settings` is in `Update`, which a
    /// bare `App` runs.
    fn running_app() -> App {
        let mut app = App::new();
        app.add_plugins(plugin).init_resource::<StreamingSettings>();
        app
    }

    fn default_camera_fog() -> DistanceFog {
        let streaming = StreamingSettings::default();
        distance_fog(
            &FogSettings::default(),
            streaming.view_distance_m,
            streaming.lod_distance_m,
        )
    }

    const CHANGED: FogSettings = FogSettings {
        color: [0.1, 0.2, 0.3],
        sky: [0.4, 0.5, 0.6],
        reach_opacity: 0.5,
    };

    #[test]
    fn changing_the_settings_at_run_time_updates_the_sky_and_every_cameras_fog() {
        let mut app = running_app();
        let cameras: Vec<Entity> = (0..3)
            .map(|_| app.world_mut().spawn(default_camera_fog()).id())
            .collect();
        app.update();

        *app.world_mut().resource_mut::<FogSettings>() = CHANGED;
        app.update();

        let [r, g, b] = CHANGED.sky;
        assert_eq!(app.world().resource::<ClearColor>().0, Color::srgb(r, g, b));
        let streaming = StreamingSettings::default();
        let want = distance_fog(
            &CHANGED,
            streaming.view_distance_m,
            streaming.lod_distance_m,
        );
        for camera in cameras {
            let fog = app.world().get::<DistanceFog>(camera).expect("fog");
            assert_eq!(fog.color, want.color);
            assert_eq!(density_of(fog), density_of(&want));
            assert_ne!(density_of(fog), density_of(&default_camera_fog()));
        }
    }

    #[test]
    fn a_later_clear_colour_survives_until_the_settings_actually_change() {
        let mut app = running_app();
        // What the capture probe does: its own `ClearColor` after this plugin.
        let magenta = Color::srgb(1.0, 0.0, 1.0);
        app.insert_resource(ClearColor(magenta));
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(app.world().resource::<ClearColor>().0, magenta);

        *app.world_mut().resource_mut::<FogSettings>() = CHANGED;
        app.update();

        let [r, g, b] = CHANGED.sky;
        assert_eq!(app.world().resource::<ClearColor>().0, Color::srgb(r, g, b));
    }

    #[test]
    fn the_frame_after_startup_rewrites_neither_the_sky_nor_a_cameras_fog() {
        let mut app = running_app();
        let magenta = Color::srgb(1.0, 0.0, 1.0);
        app.insert_resource(ClearColor(magenta));
        let unrelated = DistanceFog {
            color: Color::srgb(1.0, 0.0, 0.0),
            falloff: FogFalloff::ExponentialSquared { density: 0.25 },
            ..default()
        };
        let camera = app.world_mut().spawn(unrelated.clone()).id();

        app.update();

        assert_eq!(app.world().resource::<ClearColor>().0, magenta);
        let fog = app.world().get::<DistanceFog>(camera).expect("fog");
        assert_eq!(fog.color, unrelated.color);
        assert_eq!(density_of(fog), 0.25);
    }

    #[cfg(feature = "dev")]
    #[test]
    fn the_opacity_override_replaces_the_reach_opacity_and_ignores_anything_else() {
        let default = FogSettings::default();
        assert_eq!(
            with_opacity_override(default, Some(" 0.99999 ")).reach_opacity,
            0.99999
        );
        // Only the opacity: a measurement must not move the pair it measures.
        let overridden = with_opacity_override(default, Some("0.5"));
        assert_eq!(overridden.color, default.color);
        assert_eq!(overridden.sky, default.sky);
        for ignored in [None, Some(""), Some("dense"), Some("0.5f")] {
            assert_eq!(with_opacity_override(default, ignored), default);
        }
    }

    #[test]
    fn colour_is_the_settings_colour_in_srgb_and_opaque() {
        let fog = distance_fog(&FogSettings::default(), DEFAULT_VIEW_M, DEFAULT_LOD_M);
        let srgb = fog.color.to_srgba();
        assert_eq!(
            [srgb.red, srgb.green, srgb.blue],
            FogSettings::default().color
        );
        // The shader multiplies the fog's alpha into its strength.
        assert_eq!(srgb.alpha, 1.0);
    }
}
