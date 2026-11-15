//! Where ember's Biomes scatter the Plants of the shared `flora` and `crops` Mods: hardy ones on
//! the ash, and a fern and mushroom bog round the hot springs. `scorched` is left bare on purpose.

use crate::tesserae::mods::content::{CropField, SurfaceFeature};

demo_kit::plants!();

// Rare trees come first and ground cover last, so where chances sum past 1 the cover gives way and
// no tree is starved. Spacings follow the approved density classes: dense (trees 4 m, understory
// 30 to 40% of the ground), moderate (trees 7 to 10 m, in groves, cover 10 to 15%), sparse (trees
// 20 to 40 m, cover under 8%). A clump is `(scale_m, strength)`: 12 to 16 m makes cover patches
// 4 to 10 m across with bare ground between, 22 m crops, 48 m groves, 96 m sparse trees.

pub fn ash_flats() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:birch", 22.0, 96.0, 0.6),
        ("crops:berry", 22.0, 22.0, 0.95),
        ("crops:pumpkin", 30.0, 22.0, 0.95),
        ("flora:poppy", 12.0, 12.0, 0.95),
        ("flora:dry_grass", 3.8, 14.0, 0.9),
    ])
}

/// Hardy roots on the ash, a ditch of the lake's water where it is near, and basalt for a path.
pub fn ash_fields() -> Vec<CropField> {
    let crops = [
        ("crops:potato", 5.0),
        ("crops:carrot", 3.0),
        ("crops:wheat", 2.0),
    ];
    vec![
        crop_field(
            &crops,
            (6.0, 12.0),
            72.0,
            "strata:basalt",
            Some(("waters:water", 40.0)),
        ),
        crop_field(&crops, (6.0, 12.0), 112.0, "strata:basalt", None),
    ]
}

pub fn highland() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:pine", 24.0, 96.0, 0.6),
        ("flora:birch", 40.0, 96.0, 0.6),
        ("flora:mushroom_brown", 12.0, 16.0, 0.95),
        ("flora:fern", 4.2, 14.0, 0.95),
        ("flora:tuft", 3.0, 14.0, 0.95),
    ])
}

pub fn cinder_bog() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:swamp", 4.0, 32.0, 0.7),
        ("flora:cane", 3.5, 16.0, 0.95),
        ("flora:mushroom_red", 6.0, 16.0, 0.95),
        ("flora:fern", 2.4, 14.0, 0.9),
        ("flora:tall_grass", 1.8, 14.0, 0.9),
    ])
}
