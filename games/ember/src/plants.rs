//! Where ember's Biomes scatter the Plants of the shared `flora` and `crops` Mods: hardy ones on
//! the ash, and a fern and mushroom bog round the hot springs. `scorched` is left bare on purpose.

use crate::tesserae::mods::content::SurfaceFeature;

demo_kit::plants!();

pub fn ash_flats() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:birch", 12.0),
        ("crops:berry", 22.0),
        ("flora:poppy", 12.0),
        ("flora:dry_grass", 4.0),
    ])
}

pub fn highland() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:pine", 8.0),
        ("flora:birch", 14.0),
        ("flora:mushroom_brown", 12.0),
        ("flora:fern", 8.0),
        ("flora:tuft", 5.0),
    ])
}

pub fn cinder_bog() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:swamp", 10.0),
        ("flora:cane", 14.0),
        ("flora:mushroom_red", 9.0),
        ("flora:fern", 6.0),
        ("flora:tall_grass", 4.0),
    ])
}
