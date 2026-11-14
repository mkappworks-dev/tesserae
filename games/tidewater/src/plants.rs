//! Where tidewater's Biomes scatter the Plants of the shared `flora`, `crops` and `aquatic` Mods.
//! A water plant stands only where the sea covers its column.

use crate::tesserae::mods::content::SurfaceFeature;

demo_kit::plants!();

pub fn lagoon() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:jungle", 16.0),
        ("flora:palm", 10.0),
        ("crops:berry", 22.0),
        ("flora:cane", 16.0),
        ("aquatic:seagrass", 9.0),
        ("aquatic:kelp", 7.0),
        ("flora:fern", 6.0),
    ])
}

pub fn salt_flats() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:palm", 14.0),
        ("flora:cactus", 16.0),
        ("flora:cane", 20.0),
        ("flora:dry_grass", 5.0),
    ])
}

pub fn cold_shallows() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:pine", 10.0),
        ("crops:berry", 22.0),
        ("aquatic:kelp", 8.0),
        ("flora:tuft", 5.0),
    ])
}

pub fn rainforest() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:jungle", 13.0),
        ("flora:palm", 16.0),
        ("flora:orchid", 12.0),
        ("flora:mushroom_red", 12.0),
        ("flora:cane", 16.0),
        ("aquatic:kelp", 8.0),
        ("flora:fern", 5.0),
    ])
}

pub fn tidal_marsh() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:swamp", 9.0),
        ("flora:cane", 12.0),
        ("flora:mushroom_brown", 12.0),
        ("aquatic:red_weed", 7.0),
        ("aquatic:kelp", 7.0),
        ("flora:fern", 6.0),
    ])
}
