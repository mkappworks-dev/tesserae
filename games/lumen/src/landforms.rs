//! Lumen's sea, shore, river banks and high ground, and its underground.

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
                ground: ["strata:stone", "frost:packed_ice", "frost:snow"],
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
            name: "crystal_vault",
            climate: (0.6, 0.4),
            altitude_m: None,
            rock: None,
            floor: Some("strata:stone"),
            sky: [70, 40, 100],
            fog: [100, 64, 140],
            features: &[
                ("crystals:amethyst", None, 4.0),
                ("crystals:citrine", None, 6.0),
            ],
        },
        Cave {
            name: "ice_cave",
            climate: (0.2, 0.6),
            altitude_m: None,
            rock: Some("frost:packed_ice"),
            floor: Some("frost:ice"),
            sky: [120, 150, 180],
            fog: [150, 182, 210],
            features: &[("crystals:aquamarine", None, 5.0)],
        },
    ]
    .iter()
    .map(cave)
    .collect()
}
