//! Distance fog: two plain numbers, the density they imply, and a haze in
//! metres beside them ([`HAZE`]).
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
use bevy::shader::ShaderDefVal;

use tesserae_core::SkyFog;

use crate::session::SessionApp;

use crate::server::quality::DeclaredStreaming;
use crate::server::streaming::StreamingSettings;

/// Floor for the reach, so a World that draws nothing cannot divide by zero.
const MIN_REACH_M: f32 = 1.0;

/// Ceiling for `reach_opacity`: `ln(1 - 1)` is minus infinity, and one ULP
/// below 1 is the largest opacity that still gives a finite density.
const MAX_REACH_OPACITY: f32 = 1.0 - f32::EPSILON;

/// How much of the light the fog's glow takes: Bevy scales it by the light's lux and the exposure,
/// which puts the full sun at about 3.6 before this.
const GLOW_SHARE: f32 = 0.08;

/// The most Low, the Game's own rings, fogs at its reach: Low keeps the look it had before a
/// Player could choose a farther curve.
const LOW_REACH_OPACITY: f32 = 0.88;

/// How much the tonemapper darkens the fog colour, which runs after the fog
/// while the clear colour skips it. Measured offscreen to within 1/255 on a
/// grey-blue haze; re-measure if the tonemapper or the exposure changes.
const TONEMAP_DARKENING_SRGB: [u8; 3] = [10, 14, 22];

fn srgb(bytes: [u8; 3]) -> [f32; 3] {
    bytes.map(|b| f32::from(b) / 255.0)
}

/// The fog a Look pack sets, as plain data.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct FogSettings {
    /// sRGB, what the shader fogs toward. Not the colour the fog is seen as:
    /// see [`TONEMAP_DARKENING_SRGB`].
    pub color: [f32; 3],
    /// sRGB, the clear colour. A pack that changes `color` must change this
    /// with it, or the terrain ends in a visible edge against the sky.
    pub sky: [f32; 3],
    /// The share of a surface's colour that fog replaces at the reach. 0 is no
    /// fog. 1 is clamped to just under it, so the reach is fully fogged but the
    /// density stays finite. Outside 0 to 1, and NaN, clamp to the nearest end
    /// (NaN to 0).
    pub reach_opacity: f32,
    /// Linear: the colour of the light the fog glows with where the view looks toward it, so a
    /// low sun warms the fog round it. `None` for no glow. The glow already scales with the
    /// light's strength, so it fades at night by itself.
    pub glow: Option<[f32; 3]>,
}

impl Default for FogSettings {
    fn default() -> Self {
        FogSettings {
            color: srgb(SkyFog::DEFAULT.fog),
            sky: srgb(SkyFog::DEFAULT.sky),
            // At 0.88 and below a farther reach's square edge shows against the sky.
            reach_opacity: 0.95,
            glow: None,
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
    app.add_session_systems(Update, follow_settings);
}

/// Re-derives the clear colour and every camera's fog when [`FogSettings`] is
/// changed after startup, so a Look pack that sets it at run time takes effect
/// without respawning the cameras; and every camera's fog when the Quality
/// curve moves the reach.
pub(crate) fn follow_settings(
    settings: Res<FogSettings>,
    streaming: Res<StreamingSettings>,
    declared: Option<Res<DeclaredStreaming>>,
    mut clear: ResMut<ClearColor>,
    mut cameras: Query<&mut DistanceFog>,
) {
    // A resource reads as changed on the first run after it is inserted, and the
    // capture probe inserts its own `ClearColor` after this plugin: re-applying
    // the sky then would overwrite the probe's magenta.
    let look = settings.is_changed() && !settings.is_added();
    let reach = streaming.is_changed() && !streaming.is_added();
    if !look && !reach {
        return;
    }
    if look {
        let [r, g, b] = settings.sky;
        clear.0 = Color::srgb(r, g, b);
    }
    let fog = streamed_fog(&settings, &streaming, declared.as_deref());
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
/// rings are left to [`HAZE`]. An opacity of 0 gives a density of 0 and no fog.
pub fn distance_fog(
    settings: &FogSettings,
    view_distance_m: f32,
    lod_distance_m: f32,
) -> DistanceFog {
    let [r, g, b] = settings.color;
    DistanceFog {
        color: Color::srgb(r, g, b),
        directional_light_color: settings.glow.map_or(Color::NONE, |[r, g, b]| {
            Color::linear_rgba(r, g, b, GLOW_SHARE)
        }),
        falloff: FogFalloff::ExponentialSquared {
            density: density(settings, reach_m(view_distance_m, lod_distance_m)),
        },
        ..default()
    }
}

/// The fog for the rings the Server streams: [`distance_fog`], at most [`LOW_REACH_OPACITY`] at
/// the reach while they are the Game's Low, which they are wherever no Game declared any.
pub fn streamed_fog(
    settings: &FogSettings,
    streaming: &StreamingSettings,
    declared: Option<&DeclaredStreaming>,
) -> DistanceFog {
    let low = declared.is_none_or(|declared| declared.low() == *streaming);
    let settings = FogSettings {
        reach_opacity: if low {
            settings.reach_opacity.min(LOW_REACH_OPACITY)
        } else {
            settings.reach_opacity
        },
        ..*settings
    };
    distance_fog(
        &settings,
        streaming.view_distance_m,
        streaming.lod_distance_m,
    )
}

/// Aerial perspective: a haze in metres, beside the reach-sized fade, so a
/// surface at 100 m reads farther off than one at 2 m whatever the reach. It
/// fades toward the same fog colour, blue first, which cools what it fades. It
/// stops at [`Haze::most`], so the far terrain is still seen until the
/// reach-sized fade takes it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Haze {
    /// The share of a surface's colour the haze replaces, however far.
    pub most: f32,
    /// Per channel (red, green, blue), the distance in metres at which the
    /// haze is `1 - 1/e` of [`Haze::most`].
    pub scale_m: [f32; 3],
}

pub const HAZE: Haze = Haze {
    most: 0.2,
    scale_m: [460.0, 330.0, 230.0],
};

/// The haze as the chunk shader's `constants::CHUNK_HAZE_*`: a shader def holds
/// no float, so each is its bit pattern, which the shader bitcasts back.
pub fn haze_shader_defs() -> Vec<ShaderDefVal> {
    let bits = |name: &'static str, value: f32| ShaderDefVal::UInt(name.into(), value.to_bits());
    let [r, g, b] = HAZE.scale_m.map(|scale_m| 1.0 / scale_m);
    vec![
        bits("CHUNK_HAZE_MOST", HAZE.most),
        bits("CHUNK_HAZE_RATE_R", r),
        bits("CHUNK_HAZE_RATE_G", g),
        bits("CHUNK_HAZE_RATE_B", b),
    ]
}

/// What the shader fogs toward so that a surface fully fogged comes out as
/// `seen` (sRGB): the tonemapper darkens the fog colour, see [`TONEMAP_DARKENING_SRGB`].
pub fn fog_toward(seen: [f32; 3]) -> [f32; 3] {
    let lift = |i: usize| f32::from(TONEMAP_DARKENING_SRGB[i]) / 255.0;
    [0, 1, 2].map(|i| (seen[i] + lift(i)).min(1.0))
}

/// The fog every terrain camera gets, or `None` for none at all. A zero
/// opacity is not "off": it would still pay for the fog binding and the branch.
pub fn camera_fog(
    enabled: bool,
    settings: &FogSettings,
    streaming: &StreamingSettings,
    declared: Option<&DeclaredStreaming>,
) -> Option<DistanceFog> {
    enabled.then(|| streamed_fog(settings, streaming, declared))
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

    /// The haze's formula, `most * (1 - exp(-d / scale))` per channel, as
    /// `chunk_material.wesl` applies it before Bevy's fade.
    impl Haze {
        fn opacity_at(&self, distance_m: f32) -> [f32; 3] {
            let d = distance_m.max(0.0);
            self.scale_m
                .map(|scale_m| self.most * (1.0 - (-d / scale_m).exp()))
        }
    }

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
    fn the_reach_sized_fade_leaves_the_full_detail_rings_alone() {
        let reach_m = crate::server::quality::QualityCurve::MEDIUM.reach_m;
        let fog = distance_fog(&FogSettings::default(), DEFAULT_VIEW_M, reach_m);
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
    fn with_no_biome_colours_the_haze_is_the_default_sky_blue_and_white() {
        let FogSettings { color, sky, .. } = FogSettings::default();
        assert_eq!(color, srgb([0xff, 0xff, 0xff]));
        assert_eq!(sky, srgb([0x87, 0xce, 0xeb]));
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
        let streaming = StreamingSettings::default();
        assert!(camera_fog(false, &settings, &streaming, None).is_none());
        let fog = camera_fog(true, &settings, &streaming, None).unwrap();
        assert!(density_of(&fog) > 0.0);
    }

    #[test]
    fn low_fogs_its_reach_as_before_and_a_farther_curve_lighter() {
        use crate::server::quality::QualityCurve;
        let settings = FogSettings::default();
        let declared = DeclaredStreaming::default();
        let low = declared.low();
        let medium = QualityCurve::MEDIUM.applied_to(declared.ceiling);
        // At the engine's 192 m view Medium's unit is the view distance, as Low's is.
        assert_eq!(medium.lod_unit_m, low.lod_unit_m.max(low.view_distance_m));
        for (streaming, want) in [(low, 0.88), (medium, settings.reach_opacity)] {
            let fog = streamed_fog(&settings, &streaming, Some(&declared));
            let got = opacity_at(&fog, streaming.lod_distance_m);
            assert!((got - f64::from(want)).abs() < 1e-5, "{streaming:?}: {got}");
        }
        let lighter = FogSettings {
            reach_opacity: 0.5,
            ..settings
        };
        let fog = streamed_fog(&lighter, &low, Some(&declared));
        assert!((opacity_at(&fog, low.lod_distance_m) - 0.5).abs() < 1e-5);
    }

    /// `MinimalPlugins` is not needed: `follow_settings` is in `Update`, which a
    /// bare `App` runs.
    fn running_app() -> App {
        let mut app = App::new();
        app.add_plugins(plugin).init_resource::<StreamingSettings>();
        app
    }

    fn default_camera_fog() -> DistanceFog {
        streamed_fog(&FogSettings::default(), &StreamingSettings::default(), None)
    }

    const CHANGED: FogSettings = FogSettings {
        color: [0.1, 0.2, 0.3],
        sky: [0.4, 0.5, 0.6],
        reach_opacity: 0.5,
        glow: None,
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
        let want = streamed_fog(&CHANGED, &StreamingSettings::default(), None);
        for camera in cameras {
            let fog = app.world().get::<DistanceFog>(camera).expect("fog");
            assert_eq!(fog.color, want.color);
            assert_eq!(density_of(fog), density_of(&want));
            assert_ne!(density_of(fog), density_of(&default_camera_fog()));
        }
    }

    #[test]
    fn a_curve_change_refogs_every_camera_to_its_reach_and_leaves_the_sky() {
        let mut app = running_app();
        let magenta = Color::srgb(1.0, 0.0, 1.0);
        app.insert_resource(ClearColor(magenta));
        let camera = app.world_mut().spawn(default_camera_fog()).id();
        app.update();

        app.world_mut()
            .resource_mut::<StreamingSettings>()
            .lod_distance_m = 3000.0;
        app.update();

        let streaming = *app.world().resource::<StreamingSettings>();
        let want = streamed_fog(&FogSettings::default(), &streaming, None);
        let fog = app.world().get::<DistanceFog>(camera).expect("fog");
        assert_eq!(density_of(fog), density_of(&want));
        assert_ne!(density_of(fog), density_of(&default_camera_fog()));
        assert_eq!(app.world().resource::<ClearColor>().0, magenta);
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
    fn the_haze_is_nothing_at_the_eye_and_fades_blue_first() {
        assert_eq!(HAZE.opacity_at(0.0), [0.0; 3]);
        for step in 1..=60 {
            let [r, g, b] = HAZE.opacity_at(step as f32 * 10.0);
            assert!(b > g && g > r && r > 0.0, "{} m: {r} {g} {b}", step * 10);
        }
    }

    #[test]
    fn the_haze_shows_by_a_hundred_metres_and_never_hides_the_terrain() {
        // Edit these with the reason if the haze changes: a surface at 110 m must
        // read farther than one at 2 m, and the far terrain must still be seen.
        let [_, _, near] = HAZE.opacity_at(2.0);
        assert!(near < 0.01, "2 m: {near}");
        let [_, _, clearing] = HAZE.opacity_at(30.0);
        assert!(
            clearing < 0.04,
            "a tree across a clearing is not in fog: {clearing}"
        );
        let [r, _, b] = HAZE.opacity_at(110.0);
        assert!(b > 0.07 && r > 0.04, "110 m: {r} {b}");
        for channel in HAZE.opacity_at(100_000.0) {
            assert!(channel <= HAZE.most && channel > 0.95 * HAZE.most);
        }
        const { assert!(HAZE.most < 0.5) };
    }

    #[test]
    fn the_haze_is_in_metres_whatever_the_reach_or_the_voxel_size() {
        // Pinning the signature is the test: neither a reach nor a Voxel size can
        // reach it, so 110 m is as hazy at 0.25 m Voxels and at Low as at Ultra.
        let _: fn(&Haze, f32) -> [f32; 3] = Haze::opacity_at;
        let haze = HAZE.opacity_at(110.0);
        for (view_m, reach_m) in [(48.0, 1024.0), (192.0, 6144.0), (384.0, 30_000.0)] {
            let fog = distance_fog(&FogSettings::default(), view_m, reach_m);
            let seen = seen_opacity_at(&fog, 110.0);
            assert!((0..3).all(|c| seen[c] >= haze[c]), "{reach_m}: {seen:?}");
        }
        let reach_m = crate::server::quality::QualityCurve::MEDIUM.reach_m;
        let fog = distance_fog(&FogSettings::default(), DEFAULT_VIEW_M, reach_m);
        let seen = seen_opacity_at(&fog, 110.0);
        assert!(
            seen[2] - haze[2] < 0.01,
            "at Medium 110 m is the haze's alone"
        );
    }

    #[test]
    fn haze_and_fade_together_reach_the_reach_opacity_and_no_further() {
        let fog = distance_fog(&FogSettings::default(), DEFAULT_VIEW_M, DEFAULT_LOD_M);
        let reach = f64::from(FogSettings::default().reach_opacity);
        for channel in seen_opacity_at(&fog, DEFAULT_LOD_M) {
            assert!(f64::from(channel) > reach && channel < 1.0, "{channel}");
        }
    }

    /// The shader's order: the haze, then Bevy's reach-sized fade over it.
    fn seen_opacity_at(fog: &DistanceFog, distance_m: f32) -> [f32; 3] {
        let fade = opacity_at(fog, distance_m) as f32;
        HAZE.opacity_at(distance_m)
            .map(|haze| 1.0 - (1.0 - haze) * (1.0 - fade))
    }

    #[test]
    fn the_shader_reads_the_haze_it_is_given() {
        let defs = haze_shader_defs();
        let read = |name: &str| {
            defs.iter()
                .find_map(|def| match def {
                    ShaderDefVal::UInt(key, bits) if key == name => Some(f32::from_bits(*bits)),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("{name} missing"))
        };
        assert_eq!(read("CHUNK_HAZE_MOST"), HAZE.most);
        let rates = [
            "CHUNK_HAZE_RATE_R",
            "CHUNK_HAZE_RATE_G",
            "CHUNK_HAZE_RATE_B",
        ]
        .map(read);
        for (rate, scale_m) in rates.into_iter().zip(HAZE.scale_m) {
            assert!((rate * scale_m - 1.0).abs() < 1e-6);
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

    #[test]
    fn a_glow_is_the_lights_colour_and_none_is_none() {
        let none = distance_fog(&FogSettings::default(), DEFAULT_VIEW_M, DEFAULT_LOD_M);
        assert_eq!(none.directional_light_color, Color::NONE);
        let dusk = FogSettings {
            glow: Some([1.0, 0.55, 0.28]),
            ..default()
        };
        let glow = distance_fog(&dusk, DEFAULT_VIEW_M, DEFAULT_LOD_M)
            .directional_light_color
            .to_linear();
        assert_eq!([glow.red, glow.green, glow.blue], [1.0, 0.55, 0.28]);
        assert!(glow.alpha > 0.0 && glow.alpha < 1.0);
    }
}
