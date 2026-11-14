//! Where lumen's Biomes scatter the Plants of the shared `flora` and `crops` Mods. The World is a
//! flat court with no lake, so no water plant is placed.

use crate::tesserae::mods::content::SurfaceFeature;

demo_kit::plants!();

// Rare trees come first and ground cover last, so where chances sum past 1 the cover gives way.

pub fn garden() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:cherry", 20.0),
        ("flora:apple", 24.0),
        ("crops:carrot", 20.0),
        ("crops:wheat", 22.0),
        ("flora:dandelion", 10.0),
        ("flora:daisy", 8.0),
        ("flora:poppy", 9.0),
        ("flora:fern", 7.0),
        ("flora:tuft", 4.0),
    ])
}

pub fn arid_court() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:palm", 14.0),
        ("flora:orange", 20.0),
        ("flora:cactus", 14.0),
        ("flora:dry_grass", 5.0),
    ])
}

pub fn frost_court() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:pine", 9.0),
        ("flora:birch", 13.0),
        ("crops:berry", 20.0),
        ("flora:mushroom_brown", 14.0),
        ("flora:tall_grass", 5.0),
    ])
}

pub fn conservatory() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:jungle", 14.0),
        ("flora:cherry", 22.0),
        ("flora:orange", 18.0),
        ("flora:palm", 14.0),
        ("flora:orchid", 11.0),
        ("flora:fern", 5.0),
    ])
}

pub fn lily_pond() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:swamp", 10.0),
        ("flora:cherry", 24.0),
        ("flora:cane", 12.0),
        ("flora:orchid", 11.0),
        ("flora:fern", 6.0),
        ("flora:tuft", 4.0),
    ])
}
