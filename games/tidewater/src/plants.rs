//! Where tidewater's Biomes scatter the Plants of the shared `flora`, `crops` and `aquatic` Mods.
//! A water plant stands only where the sea covers its column.

use crate::tesserae::mods::content::{CropField, SurfaceFeature};

demo_kit::plants!();

// Rare trees come first and ground cover last, so where chances sum past 1 the cover gives way and
// no tree is starved. Spacings follow the approved density classes: dense (trees 4 m, understory
// 30 to 40% of the ground), moderate (trees 7 to 10 m, in groves, cover 10 to 15%), sparse (trees
// 20 to 40 m, cover under 8%). A clump is `(scale_m, strength)`: 12 to 16 m makes cover patches
// 4 to 10 m across with bare ground between, 22 m crops, 48 m groves, 96 m sparse trees.

pub fn lagoon() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:jungle", 26.0, 96.0, 0.6),
        ("flora:palm", 30.0, 96.0, 0.6),
        ("crops:berry", 24.0, 22.0, 0.95),
        ("crops:watermelon", 34.0, 22.0, 0.95),
        ("flora:cane", 14.0, 16.0, 0.95),
        ("aquatic:seagrass", 9.0, 16.0, 0.9),
        ("aquatic:kelp", 7.0, 16.0, 0.9),
        ("flora:fern", 3.4, 14.0, 0.9),
    ])
}

/// Wheat fields most often, on the shore: a ditch of the sea's water along the low edge.
pub fn lagoon_fields() -> Vec<CropField> {
    vec![crop_field(
        &[
            ("crops:wheat", 6.0),
            ("crops:carrot", 2.0),
            ("crops:potato", 2.0),
        ],
        (6.0, 8.0),
        80.0,
        "strata:sand",
        Some(("waters:water", 48.0)),
    )]
}

pub fn salt_flats() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:palm", 40.0, 96.0, 0.6),
        ("flora:cactus", 20.0, 64.0, 0.6),
        ("flora:cane", 22.0, 16.0, 0.95),
        ("flora:dry_grass", 5.0, 14.0, 0.9),
    ])
}

pub fn cold_shallows() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:pine", 8.0, 48.0, 0.8),
        ("crops:berry", 10.0, 22.0, 0.95),
        ("aquatic:kelp", 8.0, 16.0, 0.9),
        ("flora:tuft", 2.6, 14.0, 0.9),
    ])
}

pub fn rainforest() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:jungle", 4.0, 32.0, 0.5),
        ("flora:palm", 8.0, 32.0, 0.5),
        ("flora:orchid", 4.5, 12.0, 0.95),
        ("flora:mushroom_red", 5.0, 16.0, 0.95),
        ("flora:cane", 5.0, 16.0, 0.95),
        ("aquatic:kelp", 8.0, 16.0, 0.9),
        ("flora:fern", 1.35, 14.0, 0.9),
    ])
}

pub fn tidal_marsh() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:swamp", 4.0, 32.0, 0.7),
        ("flora:cane", 3.2, 16.0, 0.95),
        ("flora:mushroom_brown", 5.5, 16.0, 0.95),
        ("aquatic:red_weed", 7.0, 16.0, 0.9),
        ("aquatic:kelp", 7.0, 16.0, 0.9),
        ("flora:fern", 2.0, 14.0, 0.9),
    ])
}
