//! Where lumen's Biomes scatter the Plants of the shared `flora` and `crops` Mods. The World is a
//! flat court with no lake, so no water plant is placed.

use crate::tesserae::mods::content::{CropField, SurfaceFeature};

demo_kit::plants!();

// Rare trees come first and ground cover last, so where chances sum past 1 the cover gives way and
// no tree is starved. Spacings follow the approved density classes: dense (trees 4 m, understory
// 30 to 40% of the ground), moderate (trees 7 to 10 m, in groves, cover 10 to 15%), sparse (trees
// 20 to 40 m, cover under 8%). A clump is `(scale_m, strength)`: 12 to 16 m makes cover patches
// 4 to 10 m across with bare ground between, 22 m crops, 48 m groves, 96 m sparse trees.

pub fn garden() -> Vec<SurfaceFeature> {
    [
        plants(&[
            ("flora:cherry", 24.0, 96.0, 0.6),
            ("flora:apple", 28.0, 96.0, 0.6),
        ]),
        grouped(
            "vegetables",
            28.0,
            0.12,
            plants(&[
                ("crops:carrot", 13.0, 22.0, 0.95),
                ("crops:wheat", 12.0, 22.0, 0.95),
            ]),
        ),
        plants(&[
            ("crops:pumpkin", 26.0, 22.0, 0.95),
            ("crops:watermelon", 30.0, 22.0, 0.95),
        ]),
        grouped(
            "flowers",
            20.0,
            0.12,
            plants(&[
                ("flora:dandelion", 11.0, 12.0, 0.95),
                ("flora:daisy", 10.0, 12.0, 0.95),
                ("flora:poppy", 12.0, 12.0, 0.95),
            ]),
        ),
        plants(&[
            ("flora:fern", 5.5, 14.0, 0.95),
            ("flora:tuft", 3.6, 14.0, 0.95),
        ]),
    ]
    .concat()
}

/// The court is flat, so its fields are many; no lake, so no ditch.
pub fn garden_fields() -> Vec<CropField> {
    vec![crop_field(
        &[
            ("crops:wheat", 6.0),
            ("crops:carrot", 2.0),
            ("crops:potato", 2.0),
        ],
        (8.0, 16.0),
        64.0,
        "strata:sand",
        None,
    )]
}

pub fn arid_court() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:palm", 40.0, 96.0, 0.6),
        ("flora:orange", 40.0, 96.0, 0.6),
        ("flora:cactus", 20.0, 64.0, 0.6),
        ("flora:dry_grass", 4.6, 14.0, 0.9),
    ])
}

pub fn frost_court() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:pine", 24.0, 96.0, 0.6),
        ("flora:birch", 40.0, 96.0, 0.6),
        ("crops:berry", 16.0, 22.0, 0.95),
        ("flora:mushroom_brown", 12.0, 16.0, 0.95),
        ("flora:tall_grass", 3.8, 14.0, 0.9),
    ])
}

pub fn conservatory() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:jungle", 4.0, 32.0, 0.5),
        ("flora:cherry", 22.0, 96.0, 0.6),
        ("flora:orange", 18.0, 96.0, 0.6),
        ("flora:palm", 10.0, 32.0, 0.5),
        ("flora:orchid", 4.5, 12.0, 0.95),
        ("flora:fern", 1.7, 14.0, 0.9),
    ])
}

pub fn lily_pond() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:swamp", 4.0, 32.0, 0.7),
        ("flora:cherry", 24.0, 96.0, 0.6),
        ("flora:cane", 3.5, 16.0, 0.95),
        ("flora:orchid", 5.0, 12.0, 0.95),
        ("flora:fern", 2.4, 14.0, 0.9),
        ("flora:tuft", 1.8, 14.0, 0.9),
    ])
}
