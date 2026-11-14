//! Where tidewater's Biomes scatter the Plants of the shared `flora`, `crops` and `aquatic` Mods.
//! A water plant stands only where the sea covers its column.

use crate::tesserae::mods::content::SurfaceFeature;

demo_kit::plants!();

// Rare trees come first and ground cover last, so where chances sum past 1 the cover gives way and
// no tree is starved. Spacings follow the approved density classes: dense (trees 4 m), moderate
// (7 to 10 m, in groves), sparse (20 to 40 m). A clump is `(scale_m, strength)`.

pub fn lagoon() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:jungle", 26.0, 96.0, 0.6),
        ("flora:palm", 30.0, 96.0, 0.6),
        ("crops:berry", 22.0, 32.0, 0.8),
        ("flora:cane", 12.0, 32.0, 0.7),
        ("aquatic:seagrass", 9.0, 24.0, 0.7),
        ("aquatic:kelp", 7.0, 24.0, 0.7),
        ("flora:fern", 4.0, 24.0, 0.7),
    ])
}

pub fn salt_flats() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:palm", 40.0, 96.0, 0.6),
        ("flora:cactus", 20.0, 64.0, 0.6),
        ("flora:cane", 24.0, 32.0, 0.7),
        ("flora:dry_grass", 6.0, 24.0, 0.7),
    ])
}

pub fn cold_shallows() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:pine", 8.0, 48.0, 0.8),
        ("crops:berry", 12.0, 32.0, 0.8),
        ("aquatic:kelp", 8.0, 24.0, 0.7),
        ("flora:tuft", 3.5, 48.0, 0.8),
    ])
}

pub fn rainforest() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:jungle", 4.0, 32.0, 0.5),
        ("flora:palm", 8.0, 32.0, 0.5),
        ("flora:orchid", 3.5, 24.0, 0.7),
        ("flora:mushroom_red", 5.0, 24.0, 0.7),
        ("flora:cane", 6.0, 32.0, 0.7),
        ("aquatic:kelp", 8.0, 24.0, 0.7),
        ("flora:fern", 2.0, 24.0, 0.7),
    ])
}

pub fn tidal_marsh() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:swamp", 4.0, 32.0, 0.7),
        ("flora:cane", 4.0, 32.0, 0.7),
        ("flora:mushroom_brown", 8.0, 24.0, 0.7),
        ("aquatic:red_weed", 7.0, 24.0, 0.7),
        ("aquatic:kelp", 7.0, 24.0, 0.7),
        ("flora:fern", 3.0, 24.0, 0.7),
    ])
}
