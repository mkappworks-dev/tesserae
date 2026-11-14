//! Where Terra's Biomes scatter the Plants of the shared `flora`, `crops` and `aquatic` Mods.

use crate::tesserae::mods::content::{Clump, SurfaceFeature};

/// `(plant, spacing_m, clump_scale_m, clump_strength)`; a strength of 0 scatters evenly.
fn plants(list: &[(&str, f32, f32, f32)]) -> Vec<SurfaceFeature> {
    list.iter()
        .map(|&(plant, spacing_m, scale_m, strength)| SurfaceFeature {
            material: None,
            plant: Some(plant.into()),
            shape: None,
            spacing_m,
            clump: (strength > 0.0).then_some(Clump { scale_m, strength }),
        })
        .collect()
}

// Rare trees come first and ground cover last, so where chances sum past 1 the cover gives way and
// no tree is starved. Density classes follow the approved mockup: dense jungle and swamp (trees 4 m,
// understory 1.6 to 3 m), moderate temperate and cold forest (trees about 7 to 9 m, in groves),
// sparse savanna and desert (trees 26 to 40 m). A clump is `(scale_m, strength)`: groves 48 m wide,
// sparse trees in loose 96 m patches, crops 32 m, ground cover 24 to 48 m.

pub fn temperate() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:cherry", 30.0, 96.0, 0.6),
        ("flora:apple", 24.0, 96.0, 0.6),
        ("flora:birch", 14.0, 48.0, 0.8),
        ("flora:oak", 9.0, 48.0, 0.8),
        ("crops:potato", 24.0, 32.0, 0.8),
        ("crops:carrot", 20.0, 32.0, 0.8),
        ("crops:wheat", 18.0, 32.0, 0.8),
        ("crops:berry", 18.0, 32.0, 0.8),
        ("flora:mushroom_red", 6.0, 24.0, 0.7),
        ("flora:daisy", 4.0, 24.0, 0.7),
        ("flora:poppy", 5.0, 24.0, 0.7),
        ("flora:tuft", 2.5, 48.0, 0.8),
    ])
}

pub fn dry() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:orange", 40.0, 96.0, 0.6),
        ("flora:palm", 40.0, 96.0, 0.6),
        ("flora:cactus", 20.0, 64.0, 0.6),
        ("crops:berry", 40.0, 32.0, 0.8),
        ("flora:cane", 24.0, 32.0, 0.7),
        ("flora:dandelion", 8.0, 24.0, 0.7),
        ("flora:dry_grass", 6.0, 24.0, 0.7),
    ])
}

pub fn cold() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:birch", 16.0, 48.0, 0.8),
        ("flora:pine", 8.0, 48.0, 0.8),
        ("crops:berry", 12.0, 32.0, 0.8),
        ("flora:mushroom_brown", 10.0, 24.0, 0.7),
        ("flora:daisy", 6.0, 24.0, 0.7),
        ("flora:fern", 3.0, 24.0, 0.7),
        ("flora:tuft", 3.5, 48.0, 0.8),
    ])
}

pub fn jungle() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:jungle", 4.0, 32.0, 0.5),
        ("flora:palm", 8.0, 32.0, 0.5),
        ("flora:orchid", 3.0, 24.0, 0.7),
        ("crops:berry", 14.0, 32.0, 0.8),
        ("flora:cane", 6.0, 32.0, 0.7),
        ("flora:mushroom_red", 4.0, 24.0, 0.7),
        ("flora:fern", 2.0, 24.0, 0.7),
        ("flora:tall_grass", 1.6, 24.0, 0.7),
    ])
}

pub fn swamp() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:swamp", 4.0, 32.0, 0.7),
        ("flora:orchid", 6.0, 24.0, 0.7),
        ("crops:berry", 20.0, 32.0, 0.8),
        ("flora:mushroom_brown", 8.0, 24.0, 0.7),
        ("flora:cane", 4.0, 32.0, 0.7),
        ("aquatic:kelp", 3.0, 24.0, 0.7),
        ("aquatic:seagrass", 3.0, 24.0, 0.7),
        ("aquatic:red_weed", 4.0, 24.0, 0.7),
        ("flora:fern", 3.0, 24.0, 0.7),
        ("flora:tuft", 2.0, 24.0, 0.7),
    ])
}

pub fn savanna() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:acacia", 26.0, 96.0, 0.6),
        ("flora:cactus", 40.0, 96.0, 0.6),
        ("crops:wheat", 24.0, 32.0, 0.8),
        ("flora:poppy", 12.0, 24.0, 0.7),
        ("flora:dandelion", 8.0, 24.0, 0.7),
        ("flora:tuft", 5.0, 24.0, 0.7),
        ("flora:dry_grass", 2.5, 48.0, 0.8),
    ])
}

/// Soft underfoot like the leaves; logs ring like Wood.
pub fn sounds() -> Vec<(String, &'static str)> {
    [
        "oak", "birch", "pine", "palm", "acacia", "jungle", "swamp", "apple", "orange", "cherry",
    ]
    .into_iter()
    .flat_map(|tree| {
        [
            (format!("flora:{tree}_log"), "wood"),
            (format!("flora:{tree}_leaves"), "soft"),
        ]
    })
    .collect()
}
