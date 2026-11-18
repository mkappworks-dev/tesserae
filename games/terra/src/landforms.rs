//! Terra's river banks and its underground.

use crate::tesserae::mods::content::{
    BiomeDef, BiomeSelectors, CaveBiomeDef, CaveFeature, GroundParams, SelectorRange, SkyFog,
    SurfaceFeature,
};

/// Sand along a river's cut below the high ground, with cane by the water. It names a range, so
/// version 3 prefers it where a valley cuts and earlier versions never choose it.
pub fn river() -> BiomeDef {
    BiomeDef {
        soundscape: None,
        selectors: Some(BiomeSelectors {
            altitude: Some(SelectorRange {
                min: 0.0,
                max: 90.0,
            }),
            coast: None,
            erosion: None,
            river: Some(SelectorRange {
                min: 1.5,
                max: 1000.0,
            }),
            relief: None,
        }),
        strata: None,
        name: "river".into(),
        temperature: 0.55,
        humidity: 0.7,
        ground: Some(GroundParams {
            rock: "terra:stone".into(),
            soil: "terra:sand".into(),
            surface: "terra:sand".into(),
        }),
        sky_fog: Some(SkyFog {
            sky: (128, 160, 182),
            fog: (146, 176, 196),
        }),
        cave_scale: Some(0.0),
        crop_fields: Vec::new(),
        structures: Vec::new(),
        settlements: Vec::new(),
        surface_features: vec![SurfaceFeature {
            material: None,
            plant: Some("flora:cane".into()),
            shape: None,
            spacing_m: 5.0,
            clump: None,
            field: None,
        }],
        loot: None,
    }
}

/// Name, cave Climate, altitude range, rock, floor, sky, fog and floor features (Material, Shape,
/// spacing in metres).
type Cave = (
    &'static str,
    (f32, f32),
    Option<(f32, f32)>,
    Option<&'static str>,
    Option<&'static str>,
    (u8, u8, u8),
    (u8, u8, u8),
    &'static [(&'static str, Option<&'static str>, f32)],
);

const CAVES: [Cave; 3] = [
    (
        "mushroom_grotto",
        (0.5, 0.8),
        None,
        None,
        Some("terra:dirt"),
        (60, 52, 70),
        (78, 66, 92),
        &[
            ("flora:mushroom_red", Some("flora:mushroom"), 4.0),
            ("flora:mushroom_brown", Some("flora:mushroom"), 3.0),
        ],
    ),
    (
        "crystal_cavern",
        (0.3, 0.3),
        Some((-200.0, -20.0)),
        Some("strata:basalt"),
        Some("strata:basalt"),
        (40, 60, 110),
        (60, 90, 150),
        &[("terra:crystal", None, 5.0)],
    ),
    (
        "ash_depths",
        (0.85, 0.15),
        Some((-200.0, -50.0)),
        Some("terra:ash"),
        Some("terra:ash"),
        (90, 40, 30),
        (130, 60, 40),
        &[],
    ),
];

pub fn caves() -> Vec<CaveBiomeDef> {
    CAVES
        .iter()
        .map(
            |&(name, (temperature, humidity), altitude, rock, floor, sky, fog, features)| {
                CaveBiomeDef {
                    name: name.into(),
                    temperature,
                    humidity,
                    altitude: altitude.map(|(min, max)| SelectorRange { min, max }),
                    rock: rock.map(Into::into),
                    floor: floor.map(Into::into),
                    sky_fog: Some(SkyFog { sky, fog }),
                    features: features
                        .iter()
                        .map(|&(material, shape, spacing_m)| CaveFeature {
                            material: material.into(),
                            shape: shape.map(Into::into),
                            spacing_m,
                        })
                        .collect(),
                }
            },
        )
        .collect()
}
