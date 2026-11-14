//! Where lumen's Biomes scatter the Plants of the shared `flora` and `crops` Mods. The World is a
//! flat court with no lake, so no water plant is placed.

use crate::tesserae::mods::content::SurfaceFeature;

demo_kit::plants!();

// Rare trees come first and ground cover last, so where chances sum past 1 the cover gives way and
// no tree is starved. Spacings follow the approved density classes: dense (trees 4 m), moderate
// (7 to 10 m, in groves), sparse (20 to 40 m). A clump is `(scale_m, strength)`.

pub fn garden() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:cherry", 24.0, 96.0, 0.6),
        ("flora:apple", 28.0, 96.0, 0.6),
        ("crops:carrot", 20.0, 32.0, 0.8),
        ("crops:wheat", 22.0, 32.0, 0.8),
        ("flora:dandelion", 6.0, 24.0, 0.7),
        ("flora:daisy", 5.0, 24.0, 0.7),
        ("flora:poppy", 6.0, 24.0, 0.7),
        ("flora:fern", 4.0, 24.0, 0.7),
        ("flora:tuft", 2.0, 48.0, 0.8),
    ])
}

pub fn arid_court() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:palm", 40.0, 96.0, 0.6),
        ("flora:orange", 40.0, 96.0, 0.6),
        ("flora:cactus", 20.0, 64.0, 0.6),
        ("flora:dry_grass", 6.0, 24.0, 0.7),
    ])
}

pub fn frost_court() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:pine", 24.0, 96.0, 0.6),
        ("flora:birch", 40.0, 96.0, 0.6),
        ("crops:berry", 20.0, 32.0, 0.8),
        ("flora:mushroom_brown", 14.0, 24.0, 0.7),
        ("flora:tall_grass", 4.0, 48.0, 0.8),
    ])
}

pub fn conservatory() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:jungle", 4.0, 32.0, 0.5),
        ("flora:cherry", 22.0, 96.0, 0.6),
        ("flora:orange", 18.0, 96.0, 0.6),
        ("flora:palm", 10.0, 32.0, 0.5),
        ("flora:orchid", 3.5, 24.0, 0.7),
        ("flora:fern", 2.0, 24.0, 0.7),
    ])
}

pub fn lily_pond() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:swamp", 4.0, 32.0, 0.7),
        ("flora:cherry", 24.0, 96.0, 0.6),
        ("flora:cane", 4.0, 32.0, 0.7),
        ("flora:orchid", 5.0, 24.0, 0.7),
        ("flora:fern", 3.0, 24.0, 0.7),
        ("flora:tuft", 2.0, 24.0, 0.7),
    ])
}
