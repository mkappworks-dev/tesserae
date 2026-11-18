//! Ember's sea, shore, river banks and high ground, and its underground.

use crate::tesserae::mods::content::{BiomeDef, CaveBiomeDef};
use demo_kit::{ALPINE_M, Cave, Landform, OCEAN_M, RIVER_CUT_M, RIVER_M, SHORE_M};

demo_kit::landforms!();

pub fn biomes() -> Vec<BiomeDef> {
    let river = BiomeDef {
        structures: Vec::new(),
        ..landform(
            &Landform {
                name: "river",
                climate: (0.5, 0.5),
                altitude_m: RIVER_M,
                river_m: Some(RIVER_CUT_M),
                ground: ["strata:basalt", "strata:clay", "strata:clay"],
                sky: [128, 160, 182],
                fog: [146, 176, 196],
                caves: false,
            },
            Vec::new(),
        )
    };
    vec![
        landform(
            &Landform {
                name: "ocean",
                climate: (0.5, 0.5),
                altitude_m: OCEAN_M,
                river_m: None,
                ground: ["strata:basalt", "strata:sand", "strata:sand"],
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
                altitude_m: SHORE_M,
                river_m: None,
                ground: ["strata:basalt", "hearth:ash", "hearth:ash"],
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
                ground: ["strata:basalt", "strata:basalt", "hearth:ash"],
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
            name: "magma_tube",
            climate: (0.8, 0.2),
            altitude_m: None,
            rock: Some("strata:basalt"),
            floor: Some("hearth:ash"),
            sky: [96, 40, 24],
            fog: [140, 60, 36],
            features: &[],
        },
        Cave {
            name: "cinder_vault",
            climate: (0.4, 0.5),
            altitude_m: None,
            rock: None,
            floor: Some("hearth:ash"),
            sky: [64, 50, 46],
            fog: [92, 72, 64],
            features: &[("flora:mushroom_red", Some("flora:mushroom"), 6.0)],
        },
    ]
    .iter()
    .map(cave)
    .collect()
}
