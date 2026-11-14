//! Where ember's Biomes scatter the Plants of the shared `flora` and `crops` Mods: hardy ones on
//! the ash, and a fern and mushroom bog round the hot springs. `scorched` is left bare on purpose.

use crate::tesserae::mods::content::SurfaceFeature;

demo_kit::plants!();

// Rare trees come first and ground cover last, so where chances sum past 1 the cover gives way and
// no tree is starved. Spacings follow the approved density classes: dense (trees 4 m), moderate
// (7 to 10 m, in groves), sparse (20 to 40 m). A clump is `(scale_m, strength)`.

pub fn ash_flats() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:birch", 22.0, 96.0, 0.6),
        ("crops:berry", 20.0, 32.0, 0.8),
        ("flora:poppy", 10.0, 24.0, 0.7),
        ("flora:dry_grass", 3.0, 48.0, 0.8),
    ])
}

pub fn highland() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:pine", 24.0, 96.0, 0.6),
        ("flora:birch", 40.0, 96.0, 0.6),
        ("flora:mushroom_brown", 14.0, 24.0, 0.7),
        ("flora:fern", 6.0, 24.0, 0.7),
        ("flora:tuft", 4.0, 48.0, 0.8),
    ])
}

pub fn cinder_bog() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:swamp", 4.0, 32.0, 0.7),
        ("flora:cane", 4.0, 32.0, 0.7),
        ("flora:mushroom_red", 7.0, 24.0, 0.7),
        ("flora:fern", 3.0, 24.0, 0.7),
        ("flora:tall_grass", 2.0, 24.0, 0.7),
    ])
}
