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

// Nothing outside the tests uses this until the camera setup does. Gated on
// `not(test)` because the tests use everything, and an unfulfilled `expect` is
// itself a warning.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "no camera attaches the fog yet")
)]

use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;

/// Floor for the reach, so a World that draws nothing cannot divide by zero.
const MIN_REACH_M: f32 = 1.0;

/// Ceiling for `reach_opacity`: `ln(1 - 1)` is minus infinity, and one ULP
/// below 1 is the largest opacity that still gives a finite density.
const MAX_REACH_OPACITY: f32 = 1.0 - f32::EPSILON;

/// The fog a Look pack sets, as plain data.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct FogSettings {
    /// sRGB, the space of `ClearColor`.
    pub color: [f32; 3],
    /// The share of a surface's colour that fog replaces at the reach. 0 is no
    /// fog. 1 is clamped to just under it, so the reach is fully fogged but the
    /// density stays finite. Outside 0 to 1, and NaN, clamp to the nearest end
    /// (NaN to 0).
    pub reach_opacity: f32,
}

impl Default for FogSettings {
    fn default() -> Self {
        FogSettings {
            // Today's `ClearColor`, which the fog colour replaces.
            color: [43.0 / 255.0, 44.0 / 255.0, 47.0 / 255.0],
            reach_opacity: 0.85,
        }
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
            assert!((got - 0.85).abs() < 1e-5, "lod {lod_distance_m} m: {got}");
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
