//! Meadow's sea, shore, river banks and high ground, and its underground.

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
        structures: crate::structures::bridge("footbridge"),
        loot: Some(crate::structures::loot(crate::structures::biome_loot(
            "fen",
        ))),
        ..landform(
            &Landform {
                name: "river",
                climate: (0.5, 0.5),
                altitude_m: RIVER_M,
                river_m: Some(RIVER_CUT_M),
                ground: ["strata:stone", "strata:clay", "strata:clay"],
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
                altitude_m: SHORE_M,
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
            name: "burrow_hollow",
            climate: (0.5, 0.6),
            altitude_m: None,
            rock: None,
            floor: Some("strata:soil"),
            sky: [70, 62, 52],
            fog: [96, 84, 70],
            features: &[("flora:mushroom_brown", Some("flora:mushroom"), 3.0)],
        },
        Cave {
            name: "clay_grotto",
            climate: (0.4, 0.3),
            altitude_m: None,
            rock: Some("strata:clay"),
            floor: Some("strata:clay"),
            sky: [80, 64, 56],
            fog: [110, 88, 74],
            features: &[("flora:mushroom_red", Some("flora:mushroom"), 5.0)],
        },
    ]
    .iter()
    .map(cave)
    .collect()
}
