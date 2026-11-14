//! Where the meadow's Biomes scatter the Plants of the shared `flora` and `crops` Mods. The World
//! has no lake, so no water plant is placed.

use crate::tesserae::mods::content::SurfaceFeature;

demo_kit::plants!();

// Rare trees come first and ground cover last, so where chances sum past 1 the cover gives way and
// no tree is starved. Spacings follow the approved density classes: dense (trees 4 m, understory
// 30 to 40% of the ground), moderate (trees 7 to 10 m, in groves, cover 10 to 15%), sparse (trees
// 20 to 40 m, cover under 8%). A clump is `(scale_m, strength)`: 12 to 16 m makes cover patches
// 4 to 10 m across with bare ground between, 22 m crops, 48 m groves, 96 m sparse trees.

pub fn meadow() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:cherry", 30.0, 96.0, 0.6),
        ("flora:apple", 30.0, 96.0, 0.6),
        ("flora:oak", 22.0, 96.0, 0.6),
        ("crops:potato", 13.0, 22.0, 0.95),
        ("crops:carrot", 13.0, 22.0, 0.95),
        ("crops:wheat", 12.0, 22.0, 0.95),
        ("flora:dandelion", 11.0, 12.0, 0.95),
        ("flora:daisy", 10.0, 12.0, 0.95),
        ("flora:poppy", 12.0, 12.0, 0.95),
        ("flora:tuft", 3.5, 14.0, 0.95),
    ])
}

pub fn heath() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:swamp", 12.0, 48.0, 0.8),
        ("flora:birch", 10.0, 48.0, 0.8),
        ("crops:berry", 10.0, 22.0, 0.95),
        ("flora:mushroom_brown", 12.0, 16.0, 0.95),
        ("flora:fern", 4.0, 14.0, 0.95),
        ("flora:tall_grass", 3.6, 16.0, 0.95),
    ])
}

pub fn dunes() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:palm", 40.0, 96.0, 0.6),
        ("flora:cactus", 20.0, 64.0, 0.6),
        ("flora:cane", 20.0, 16.0, 0.95),
        ("flora:dry_grass", 4.6, 14.0, 0.9),
    ])
}

pub fn thicket() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:jungle", 4.0, 32.0, 0.5),
        ("flora:orchid", 5.5, 12.0, 0.95),
        ("crops:berry", 12.0, 22.0, 0.95),
        ("flora:mushroom_red", 6.5, 16.0, 0.95),
        ("flora:fern", 2.3, 14.0, 0.9),
        ("flora:tuft", 1.7, 14.0, 0.9),
    ])
}

pub fn fen() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:swamp", 4.0, 32.0, 0.7),
        ("flora:cane", 3.5, 16.0, 0.95),
        ("flora:mushroom_brown", 5.5, 16.0, 0.95),
        ("flora:fern", 2.4, 14.0, 0.9),
        ("flora:tuft", 1.9, 14.0, 0.9),
    ])
}

pub fn prairie() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:acacia", 26.0, 96.0, 0.6),
        ("crops:wheat", 18.0, 22.0, 0.95),
        ("flora:poppy", 13.0, 12.0, 0.95),
        ("flora:dandelion", 10.0, 12.0, 0.95),
        ("flora:tuft", 7.0, 14.0, 0.9),
        ("flora:dry_grass", 3.6, 14.0, 0.9),
    ])
}
