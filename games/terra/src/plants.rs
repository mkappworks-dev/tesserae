//! Where Terra's Biomes scatter the Plants of the shared `flora`, `crops` and `aquatic` Mods.

use crate::tesserae::mods::content::SurfaceFeature;

fn feature(plant: &str, spacing_m: f32) -> SurfaceFeature {
    SurfaceFeature {
        material: None,
        plant: Some(plant.into()),
        shape: None,
        spacing_m,
    }
}

pub fn temperate() -> Vec<SurfaceFeature> {
    vec![
        feature("flora:oak", 9.0),
        feature("flora:apple", 26.0),
        feature("crops:wheat", 22.0),
    ]
}

pub fn dry() -> Vec<SurfaceFeature> {
    vec![
        feature("flora:cane", 18.0),
        feature("crops:berry", 20.0),
        feature("aquatic:kelp", 8.0),
    ]
}

pub fn cold() -> Vec<SurfaceFeature> {
    vec![
        feature("flora:mushroom_red", 16.0),
        feature("aquatic:kelp", 8.0),
    ]
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
