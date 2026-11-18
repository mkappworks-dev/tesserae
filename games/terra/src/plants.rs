//! Where Terra's Biomes scatter the Plants of the shared `flora`, `crops` and `aquatic` Mods.

use crate::tesserae::mods::content::{Clump, CropField, FieldDef, SurfaceFeature};

/// `(plant, spacing_m, clump_scale_m, clump_strength)`; a strength of 0 scatters evenly.
fn plants(list: &[(&str, f32, f32, f32)]) -> Vec<SurfaceFeature> {
    list.iter()
        .map(|&(plant, spacing_m, scale_m, strength)| SurfaceFeature {
            material: None,
            plant: Some(plant.into()),
            shape: None,
            spacing_m,
            clump: (strength > 0.0).then_some(Clump { scale_m, strength }),
            field: None,
        })
        .collect()
}

/// Puts `features` in one field `group`. A member stands only in the regions that hold it and among
/// a share of strays, so its spacing closes by the root of that share to keep the group's cover.
fn grouped(
    group: &str,
    scale_m: f32,
    strays: f32,
    features: Vec<SurfaceFeature>,
) -> Vec<SurfaceFeature> {
    let members = features.len().max(1) as f32;
    let share = (1.0 + strays * (members - 1.0)) / members;
    features
        .into_iter()
        .map(|f| SurfaceFeature {
            spacing_m: f.spacing_m * share.sqrt(),
            field: Some(FieldDef {
                group: group.into(),
                scale_m,
                strays,
            }),
            ..f
        })
        .collect()
}

/// A planted field of the named crops with their weights.
fn crop_field(
    crops: &[(&str, f32)],
    plot_m: (f32, f32),
    spacing_m: f32,
    ditch: Option<f32>,
) -> CropField {
    CropField {
        crops: crops.iter().map(|&(c, w)| (c.to_owned(), w)).collect(),
        plot_m,
        spacing_m,
        path: Some("terra:sand".into()),
        ditch: ditch.map(|_| "terra:water".into()),
        near_water_m: ditch,
    }
}

// Rare trees come first and ground cover last, so where chances sum past 1 the cover gives way and
// no tree is starved. Density classes follow the approved mockup: dense jungle and swamp (trees 4 m,
// understory 30 to 40% of the ground), moderate temperate and cold forest (trees about 7 to 10 m, in
// groves, ground cover 10 to 15%), sparse savanna and desert (trees 26 to 40 m, cover under 8%).
// A clump is `(scale_m, strength)`; a 12 to 16 m scale makes ground cover patches 4 to 10 m across
// with bare ground between, a 24 m scale crops and berries, a 48 m scale groves, 96 m sparse trees.
// A clump never concentrates a feature past twice its mean, so a flower is a few blooms to a patch.

pub fn temperate() -> Vec<SurfaceFeature> {
    [
        plants(&[
            ("flora:cherry", 32.0, 96.0, 0.6),
            ("flora:apple", 26.0, 96.0, 0.6),
            ("flora:birch", 17.0, 56.0, 0.5),
            ("flora:oak", 10.0, 56.0, 0.5),
        ]),
        grouped(
            "vegetables",
            28.0,
            0.12,
            plants(&[
                ("crops:potato", 10.0, 22.0, 0.95),
                ("crops:carrot", 10.0, 22.0, 0.95),
                ("crops:wheat", 7.5, 22.0, 0.95),
            ]),
        ),
        plants(&[
            ("crops:berry", 10.0, 22.0, 0.95),
            ("crops:watermelon", 26.0, 22.0, 0.95),
            ("crops:pumpkin", 28.0, 22.0, 0.95),
            ("flora:mushroom_red", 12.0, 16.0, 0.95),
        ]),
        grouped(
            "flowers",
            20.0,
            0.12,
            plants(&[
                ("flora:daisy", 7.5, 12.0, 0.95),
                ("flora:poppy", 9.0, 14.0, 0.95),
            ]),
        ),
        plants(&[("flora:tuft", 2.9, 14.0, 0.95)]),
    ]
    .concat()
}

/// Wheat fields most often. The first stand by the lake with a ditch of its water, the second
/// anywhere on the meadow.
pub fn temperate_fields() -> Vec<CropField> {
    let crops = [
        ("crops:wheat", 6.0),
        ("crops:carrot", 2.0),
        ("crops:potato", 2.0),
    ];
    vec![
        crop_field(&crops, (6.0, 12.0), 72.0, Some(40.0)),
        crop_field(&crops, (6.0, 12.0), 112.0, None),
    ]
}

pub fn dry() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:orange", 40.0, 96.0, 0.6),
        ("flora:palm", 40.0, 96.0, 0.6),
        ("flora:cactus", 22.0, 64.0, 0.6),
        ("crops:berry", 40.0, 22.0, 0.95),
        ("flora:cane", 18.0, 16.0, 0.95),
        ("flora:dandelion", 9.0, 12.0, 0.95),
        ("flora:dry_grass", 4.6, 14.0, 0.95),
    ])
}

pub fn cold() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:birch", 20.0, 56.0, 0.5),
        ("flora:pine", 9.0, 56.0, 0.5),
        ("crops:berry", 12.0, 22.0, 0.95),
        ("flora:mushroom_brown", 10.0, 16.0, 0.95),
        ("flora:daisy", 8.5, 12.0, 0.95),
        ("flora:fern", 3.4, 14.0, 0.95),
        ("flora:tuft", 4.2, 16.0, 0.95),
    ])
}

pub fn jungle() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:jungle", 4.0, 32.0, 0.5),
        ("flora:palm", 8.0, 32.0, 0.5),
        ("flora:orchid", 5.5, 14.0, 0.9),
        ("crops:berry", 14.0, 22.0, 0.95),
        ("flora:cane", 6.5, 16.0, 0.95),
        ("flora:mushroom_red", 6.5, 16.0, 0.95),
        ("flora:fern", 2.3, 14.0, 0.9),
        ("flora:tall_grass", 1.7, 14.0, 0.9),
    ])
}

pub fn swamp() -> Vec<SurfaceFeature> {
    plants(&[
        ("flora:swamp", 4.0, 32.0, 0.7),
        ("flora:orchid", 6.5, 14.0, 0.9),
        ("crops:berry", 20.0, 22.0, 0.95),
        ("flora:mushroom_brown", 5.5, 16.0, 0.95),
        ("flora:cane", 3.2, 16.0, 0.95),
        ("aquatic:kelp", 3.0, 16.0, 0.9),
        ("aquatic:seagrass", 3.0, 16.0, 0.9),
        ("aquatic:red_weed", 4.0, 16.0, 0.9),
        ("flora:fern", 2.1, 14.0, 0.9),
        ("flora:tuft", 1.8, 14.0, 0.9),
    ])
}

/// Kelp forests and seagrass meadows in patches across the sea floor, with bare sand between, and
/// red weed scattered on the shelf.
pub fn ocean() -> Vec<SurfaceFeature> {
    plants(&[
        ("aquatic:kelp", 4.0, 48.0, 0.85),
        ("aquatic:seagrass", 3.0, 28.0, 0.85),
        ("aquatic:red_weed", 9.0, 18.0, 0.9),
    ])
}

pub fn savanna() -> Vec<SurfaceFeature> {
    [
        plants(&[
            ("flora:acacia", 26.0, 96.0, 0.6),
            ("flora:cactus", 40.0, 96.0, 0.6),
            ("crops:wheat", 28.0, 22.0, 0.95),
            ("crops:watermelon", 36.0, 22.0, 0.95),
        ]),
        grouped(
            "flowers",
            20.0,
            0.12,
            plants(&[
                ("flora:poppy", 13.0, 12.0, 0.95),
                ("flora:dandelion", 10.0, 14.0, 0.95),
            ]),
        ),
        plants(&[
            ("flora:tuft", 8.4, 14.0, 0.95),
            ("flora:dry_grass", 4.6, 16.0, 0.95),
        ]),
    ]
    .concat()
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
