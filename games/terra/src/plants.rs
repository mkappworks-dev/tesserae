//! Where Terra's Biomes scatter the Plants of the shared `flora`, `crops` and `aquatic` Mods.

use crate::tesserae::mods::content::SurfaceFeature;

fn features(list: &[(&str, f32)]) -> Vec<SurfaceFeature> {
    list.iter()
        .map(|&(plant, spacing_m)| SurfaceFeature {
            material: None,
            plant: Some(plant.into()),
            shape: None,
            spacing_m,
        })
        .collect()
}

// Rare trees come first and ground cover last, so where chances sum past 1 the cover gives way.
// Trees stand 6 to 14 m apart, ground cover 4 to 12 m and crops in patches 16 to 24 m.

pub fn temperate() -> Vec<SurfaceFeature> {
    features(&[
        ("flora:cherry", 30.0),
        ("flora:apple", 24.0),
        ("flora:birch", 14.0),
        ("flora:oak", 9.0),
        ("crops:potato", 24.0),
        ("crops:carrot", 20.0),
        ("crops:wheat", 18.0),
        ("crops:berry", 18.0),
        ("flora:mushroom_red", 14.0),
        ("flora:daisy", 10.0),
        ("flora:poppy", 8.0),
        ("flora:tuft", 4.0),
    ])
}

pub fn dry() -> Vec<SurfaceFeature> {
    features(&[
        ("flora:orange", 22.0),
        ("flora:palm", 12.0),
        ("flora:cactus", 14.0),
        ("crops:berry", 20.0),
        ("flora:cane", 18.0),
        ("flora:dandelion", 10.0),
        ("flora:dry_grass", 4.0),
    ])
}

pub fn cold() -> Vec<SurfaceFeature> {
    features(&[
        ("flora:birch", 14.0),
        ("flora:pine", 8.0),
        ("crops:berry", 22.0),
        ("flora:mushroom_brown", 14.0),
        ("flora:daisy", 12.0),
        ("flora:fern", 8.0),
        ("flora:tuft", 5.0),
    ])
}

pub fn jungle() -> Vec<SurfaceFeature> {
    features(&[
        ("flora:jungle", 13.0),
        ("flora:palm", 16.0),
        ("flora:orchid", 12.0),
        ("crops:berry", 22.0),
        ("flora:cane", 16.0),
        ("flora:mushroom_red", 12.0),
        ("flora:fern", 5.0),
        ("flora:tall_grass", 4.0),
    ])
}

pub fn swamp() -> Vec<SurfaceFeature> {
    features(&[
        ("flora:swamp", 9.0),
        ("flora:orchid", 14.0),
        ("crops:berry", 24.0),
        ("flora:mushroom_brown", 10.0),
        ("flora:cane", 12.0),
        ("aquatic:kelp", 6.0),
        ("aquatic:seagrass", 6.0),
        ("aquatic:red_weed", 8.0),
        ("flora:fern", 6.0),
        ("flora:tuft", 4.0),
    ])
}

pub fn savanna() -> Vec<SurfaceFeature> {
    features(&[
        ("flora:acacia", 9.0),
        ("flora:cactus", 20.0),
        ("crops:wheat", 24.0),
        ("flora:poppy", 12.0),
        ("flora:dandelion", 9.0),
        ("flora:tuft", 6.0),
        ("flora:dry_grass", 4.0),
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
