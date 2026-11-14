//! Where the meadow's Biomes scatter the Plants of the shared `flora` and `crops` Mods. The World
//! has no lake, so no water plant is placed.

use crate::tesserae::mods::content::SurfaceFeature;

demo_kit::plants!();

// Rare trees come first and ground cover last, so where chances sum past 1 the cover gives way.

pub fn meadow() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:cherry", 30.0),
        ("flora:apple", 26.0),
        ("flora:oak", 11.0),
        ("crops:potato", 24.0),
        ("crops:carrot", 20.0),
        ("crops:wheat", 18.0),
        ("flora:dandelion", 10.0),
        ("flora:daisy", 9.0),
        ("flora:poppy", 8.0),
        ("flora:tuft", 4.0),
    ])
}

pub fn heath() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:swamp", 16.0),
        ("flora:birch", 12.0),
        ("crops:berry", 20.0),
        ("flora:mushroom_brown", 12.0),
        ("flora:fern", 7.0),
        ("flora:tall_grass", 4.0),
    ])
}

pub fn dunes() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:palm", 14.0),
        ("flora:cactus", 16.0),
        ("flora:cane", 20.0),
        ("flora:dry_grass", 5.0),
    ])
}

pub fn thicket() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:jungle", 13.0),
        ("flora:orchid", 12.0),
        ("crops:berry", 22.0),
        ("flora:mushroom_red", 12.0),
        ("flora:fern", 5.0),
        ("flora:tuft", 4.0),
    ])
}

pub fn fen() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:swamp", 9.0),
        ("flora:cane", 12.0),
        ("flora:mushroom_brown", 10.0),
        ("flora:fern", 6.0),
        ("flora:tuft", 4.0),
    ])
}

pub fn prairie() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:acacia", 10.0),
        ("crops:wheat", 22.0),
        ("flora:poppy", 12.0),
        ("flora:dandelion", 9.0),
        ("flora:tuft", 6.0),
        ("flora:dry_grass", 4.0),
    ])
}
