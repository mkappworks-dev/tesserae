//! Tidewater's open sea, shore, river banks and high ground, and its underground, which Generator version 3 alone reads.

use crate::tesserae::mods::content::{BiomeDef, CaveBiomeDef, SurfaceFeature};
use demo_kit::{ALPINE_M, Cave, Landform, OCEAN_M, RIVER_CUT_M, RIVER_M, SHORE_M};

demo_kit::landforms!();

fn cane() -> SurfaceFeature {
    SurfaceFeature {
        material: None,
        plant: Some("flora:cane".into()),
        shape: None,
        spacing_m: 5.0,
        clump: None,
        field: None,
    }
}

pub fn biomes() -> Vec<BiomeDef> {
    let river = BiomeDef {
        structures: Vec::new(),
        ..landform(
            &Landform {
                name: "river",
                climate: (0.5, 0.5),
                altitude_m: RIVER_M,
                river_m: Some(RIVER_CUT_M),
                ground: ["strata:stone", "strata:soil", "strata:sand"],
                sky: [128, 160, 182],
                fog: [146, 176, 196],
                caves: false,
            },
            vec![cane()],
        )
    };
    vec![
        landform(
            &Landform {
                name: "ocean",
                climate: (0.5, 0.5),
                altitude_m: OCEAN_M,
                river_m: None,
                ground: ["strata:stone", "strata:sand", "strata:sand"],
                sky: [118, 150, 178],
                fog: [130, 164, 190],
                caves: false,
            },
            Vec::new(),
        ),
        landform(
            &Landform {
                name: "shore",
                climate: (0.5, 0.5),
                // Narrower than demo-kit's strand, which takes the islands' ground from the lagoon.
                altitude_m: (SHORE_M.0, 1.0),
                river_m: None,
                ground: ["strata:stone", "strata:sand", "strata:sand"],
                sky: [140, 156, 176],
                fog: [160, 176, 196],
                caves: true,
            },
            Vec::new(),
        ),
        river,
        landform(
            &Landform {
                name: "alpine",
                climate: (0.5, 0.5),
                altitude_m: ALPINE_M,
                river_m: None,
                ground: ["strata:stone", "strata:stone", "strata:stone"],
                sky: [150, 168, 192],
                fog: [180, 194, 214],
                caves: false,
            },
            Vec::new(),
        ),
    ]
}

pub fn caves() -> Vec<CaveBiomeDef> {
    [
        Cave {
            name: "sea_grotto",
            climate: (0.5, 0.8),
            altitude_m: None,
            rock: None,
            floor: Some("strata:sand"),
            sky: [40, 80, 96],
            fog: [56, 110, 128],
            features: &[("crystals:aquamarine", None, 4.0)],
        },
        Cave {
            name: "amethyst_hollow",
            climate: (0.4, 0.3),
            altitude_m: Some((-200.0, -10.0)),
            rock: Some("strata:basalt"),
            floor: Some("strata:basalt"),
            sky: [60, 36, 84],
            fog: [86, 54, 118],
            features: &[("crystals:amethyst", None, 4.0)],
        },
    ]
    .iter()
    .map(cave)
    .collect()
}
