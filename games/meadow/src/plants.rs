//! Where the meadow's Biomes scatter the Plants of the shared `flora` and `crops` Mods. The World
//! has no lake, so no water plant is placed.

use crate::tesserae::mods::content::SurfaceFeature;

demo_kit::plants!();

// Rare trees come first and ground cover last, so where chances sum past 1 the cover gives way and
// no tree is starved. Spacings follow the approved density classes: dense (trees 4 m), moderate
// (7 to 10 m, in groves), sparse (20 to 40 m). A clump is `(scale_m, strength)`.

pub fn meadow() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:cherry", 30.0, 96.0, 0.6),
        ("flora:apple", 30.0, 96.0, 0.6),
        ("flora:oak", 22.0, 96.0, 0.6),
        ("crops:potato", 24.0, 32.0, 0.8),
        ("crops:carrot", 20.0, 32.0, 0.8),
        ("crops:wheat", 18.0, 32.0, 0.8),
        ("flora:dandelion", 6.0, 24.0, 0.7),
        ("flora:daisy", 5.0, 24.0, 0.7),
        ("flora:poppy", 6.0, 24.0, 0.7),
        ("flora:tuft", 2.0, 48.0, 0.8),
    ])
}

pub fn heath() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:swamp", 12.0, 48.0, 0.8),
        ("flora:birch", 10.0, 48.0, 0.8),
        ("crops:berry", 12.0, 32.0, 0.8),
        ("flora:mushroom_brown", 10.0, 24.0, 0.7),
        ("flora:fern", 3.0, 24.0, 0.7),
        ("flora:tall_grass", 3.0, 48.0, 0.8),
    ])
}

pub fn dunes() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:palm", 40.0, 96.0, 0.6),
        ("flora:cactus", 20.0, 64.0, 0.6),
        ("flora:cane", 24.0, 32.0, 0.7),
        ("flora:dry_grass", 6.0, 24.0, 0.7),
    ])
}

pub fn thicket() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:jungle", 4.0, 32.0, 0.5),
        ("flora:orchid", 3.5, 24.0, 0.7),
        ("crops:berry", 14.0, 32.0, 0.8),
        ("flora:mushroom_red", 5.0, 24.0, 0.7),
        ("flora:fern", 2.0, 24.0, 0.7),
        ("flora:tuft", 1.6, 24.0, 0.7),
    ])
}

pub fn fen() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:swamp", 4.0, 32.0, 0.7),
        ("flora:cane", 4.0, 32.0, 0.7),
        ("flora:mushroom_brown", 8.0, 24.0, 0.7),
        ("flora:fern", 3.0, 24.0, 0.7),
        ("flora:tuft", 2.0, 24.0, 0.7),
    ])
}

pub fn prairie() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:acacia", 26.0, 96.0, 0.6),
        ("crops:wheat", 22.0, 32.0, 0.8),
        ("flora:poppy", 12.0, 24.0, 0.7),
        ("flora:dandelion", 8.0, 24.0, 0.7),
        ("flora:tuft", 5.0, 24.0, 0.7),
        ("flora:dry_grass", 2.5, 48.0, 0.8),
    ])
}
