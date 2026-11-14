//! Terra's plants: their Materials, flat Shapes, Items, Plant rows, food Recipes and the Biome
//! features that scatter them. A tree's grown Materials name no Item, so nobody can place them.

use crate::tesserae::mods::content::{
    Appearance, ClimberDef, ColumnDef, ColumnDirection, CountRange, CropDef, FruitDef, ItemDef,
    MaterialDef, Occlusion, PlantDef, PlantKind, PlantNeeds, PlantStage, PlantWater, PlantYield,
    RecipeDef, RecipeStack, RenderClass, ShapeDef, ShapeForm, SpreaderDef, SurfaceFeature, TreeDef,
};
use crate::tesserae::mods::types::{NodeBox, Srgba};
use crate::{appearance, solid};

/// Cutout and translucent, so light passes a field and the cross shows its sprite.
fn flat(name: &str, dimming: u8) -> MaterialDef {
    MaterialDef {
        dimming,
        occlusion: Occlusion::Never,
        render: RenderClass::Cutout,
        remove_time_s: Some(0.05),
        ..solid(name)
    }
}

pub fn materials() -> Vec<MaterialDef> {
    let mut all: Vec<MaterialDef> = [
        "oak_sapling",
        "apple_sapling",
        "wheat_sprout",
        "wheat_young",
        "wheat_green",
        "wheat_ripe",
        "berry_sprout",
        "berry_bush",
        "berry_ripe",
        "cane",
        "mushroom",
        "vine",
        "kelp",
        "apple_unripe",
        "apple_ripe",
    ]
    .map(|name| flat(name, 0))
    .into();
    all.extend([
        solid("oak_log"),
        solid("apple_log"),
        MaterialDef {
            remove_time_s: Some(0.1),
            ..flat("oak_leaves", 1)
        },
        MaterialDef {
            remove_time_s: Some(0.1),
            ..flat("apple_leaves", 1)
        },
    ]);
    all
}

pub fn appearances() -> Vec<Appearance> {
    [
        ("oak_sapling", [70, 130, 48, 255]),
        ("apple_sapling", [90, 140, 56, 255]),
        ("wheat_sprout", [110, 170, 60, 255]),
        ("wheat_young", [128, 170, 56, 255]),
        ("wheat_green", [146, 170, 52, 255]),
        ("wheat_ripe", [214, 178, 70, 255]),
        ("berry_sprout", [52, 112, 44, 255]),
        ("berry_bush", [52, 112, 44, 255]),
        ("berry_ripe", [186, 36, 62, 255]),
        ("cane", [140, 190, 90, 255]),
        ("mushroom", [138, 74, 52, 255]),
        ("vine", [56, 118, 44, 255]),
        ("kelp", [44, 110, 70, 255]),
        ("apple_unripe", [128, 168, 58, 255]),
        ("apple_ripe", [196, 44, 40, 255]),
        ("oak_log", [150, 104, 60, 255]),
        ("apple_log", [150, 104, 60, 255]),
        ("oak_leaves", [41, 107, 36, 255]),
        ("apple_leaves", [41, 107, 36, 255]),
    ]
    .map(|(name, color)| appearance(name, color))
    .into()
}

/// Soft underfoot like the leaves and grass; logs ring like Wood.
pub fn sounds() -> Vec<(String, &'static str)> {
    materials()
        .into_iter()
        .map(|m| {
            let set = if m.name.ends_with("_log") {
                "wood"
            } else {
                "soft"
            };
            (m.name, set)
        })
        .collect()
}

pub fn shapes() -> Vec<ShapeDef> {
    let flat_shape = |name: &str, form, min, max| ShapeDef {
        form: Some(form),
        name: name.into(),
        boxes: vec![NodeBox { min, max }],
        collision: None,
        connect: None,
    };
    vec![
        flat_shape("cross", ShapeForm::Cross, (3, 0, 3), (13, 14, 13)),
        flat_shape("plane", ShapeForm::Plane, (1, 1, 14), (15, 15, 16)),
        apple_shape(),
    ]
}

/// Hanging fruit as stacked boxes, body about 0.6 of a cell: the mockup's apple in sixteenths. It
/// seals no face and is walked through, so canopy fruit never blocks a Player or light.
fn apple_shape() -> ShapeDef {
    let boxes = [
        ((4, 1, 4), (12, 3, 12)),
        ((3, 3, 3), (13, 8, 13)),
        ((4, 8, 4), (12, 11, 12)),
        ((7, 11, 7), (9, 14, 9)),
    ];
    ShapeDef {
        form: None,
        name: "apple".into(),
        boxes: boxes.map(|(min, max)| NodeBox { min, max }).into(),
        collision: Some(Vec::new()),
        connect: None,
    }
}

fn color([r, g, b]: [u8; 3]) -> Srgba {
    Srgba { r, g, b, a: 255 }
}

fn item(name: &str, material: Option<&str>, rgb: [u8; 3], food: Option<f32>) -> ItemDef {
    ItemDef {
        name: name.into(),
        size_m: 0.2,
        color: color(rgb),
        material: material.map(Into::into),
        max_stack: None,
        food,
        texture: (material.is_none()).then(|| crate::textures::item_texture(name)),
    }
}

pub fn items() -> Vec<ItemDef> {
    vec![
        item(
            "wheat_seeds",
            Some("terra:wheat_sprout"),
            [150, 140, 70],
            None,
        ),
        item("wheat", None, [214, 178, 70], None),
        item("bread", None, [190, 130, 68], Some(7.0)),
        item(
            "berry_cutting",
            Some("terra:berry_sprout"),
            [70, 120, 50],
            None,
        ),
        item("berries", None, [186, 36, 62], Some(3.0)),
        item("berry_pie", None, [150, 60, 80], Some(9.0)),
        item("apple", None, [196, 44, 40], Some(5.0)),
    ]
}

fn stack(item: &str, count: u32) -> RecipeStack {
    RecipeStack {
        item: format!("terra:{item}"),
        count,
    }
}

pub fn recipes() -> Vec<RecipeDef> {
    vec![
        RecipeDef {
            inputs: vec![stack("wheat", 3)],
            output: stack("bread", 1),
            station: "hand".into(),
        },
        RecipeDef {
            inputs: vec![stack("berries", 2), stack("wheat", 1)],
            output: stack("berry_pie", 1),
            station: "workbench".into(),
        },
    ]
}

fn give(item: &str, min: u32, max: u32) -> PlantYield {
    PlantYield {
        item: format!("terra:{item}"),
        count: CountRange { min, max },
    }
}

fn needs(soil: &[&str]) -> PlantNeeds {
    PlantNeeds {
        soil: soil.iter().map(|m| format!("terra:{m}")).collect(),
        light_min: None,
        light_max: None,
        temperature: None,
        humidity: None,
        water: None,
    }
}

fn stage(material: &str, mean_seconds: Option<f32>, yields: Vec<PlantYield>) -> PlantStage {
    PlantStage {
        material: format!("terra:{material}"),
        mean_seconds,
        yields,
    }
}

fn tree(name: &str, fruit: Option<FruitDef>) -> PlantDef {
    let sapling = format!("terra:{name}_sapling");
    PlantDef {
        name: name.into(),
        seed: sapling.clone(),
        shape: Some(CROSS.into()),
        needs: PlantNeeds {
            light_min: Some(8),
            ..needs(&["grass", "dirt"])
        },
        kind: PlantKind::Tree(TreeDef {
            sapling: sapling.clone(),
            sapling_yields: vec![give(&format!("{name}_sapling"), 1, 1)],
            trunk: format!("terra:{name}_log"),
            canopy: format!("terra:{name}_leaves"),
            trunk_height_m: (4.0, 6.0),
            canopy_radius_m: 2.5,
            mean_seconds: 90.0,
            trunk_yield: vec![give("wood", 1, 1)],
            canopy_yield: vec![give("leaves", 0, 1), give(&format!("{name}_sapling"), 0, 1)],
            sapling_yield: CountRange { min: 1, max: 2 },
            fruit,
            vines: None,
        }),
    }
}

const CROSS: &str = "terra:cross";
const PLANE: &str = "terra:plane";

pub fn plants() -> Vec<PlantDef> {
    let wheat = PlantDef {
        name: "wheat".into(),
        seed: "terra:wheat_seeds".into(),
        shape: Some(CROSS.into()),
        needs: PlantNeeds {
            light_min: Some(9),
            ..needs(&["grass", "dirt"])
        },
        kind: PlantKind::Crop(CropDef {
            stages: vec![
                stage("wheat_sprout", Some(45.0), vec![give("wheat_seeds", 1, 1)]),
                stage("wheat_young", Some(45.0), vec![give("wheat_seeds", 1, 1)]),
                stage("wheat_green", Some(45.0), vec![give("wheat_seeds", 1, 1)]),
                stage(
                    "wheat_ripe",
                    None,
                    vec![give("wheat", 1, 2), give("wheat_seeds", 1, 2)],
                ),
            ],
            harvest_to: None,
        }),
    };
    let berry = PlantDef {
        name: "berry".into(),
        seed: "terra:berry_cutting".into(),
        shape: Some(CROSS.into()),
        needs: PlantNeeds {
            light_min: Some(9),
            ..needs(&["grass", "dirt", "sand"])
        },
        kind: PlantKind::Crop(CropDef {
            stages: vec![
                stage(
                    "berry_sprout",
                    Some(40.0),
                    vec![give("berry_cutting", 1, 1)],
                ),
                stage("berry_bush", Some(50.0), vec![give("berry_cutting", 1, 1)]),
                stage("berry_ripe", None, vec![give("berries", 2, 3)]),
            ],
            harvest_to: Some(1),
        }),
    };
    let apple_fruit = FruitDef {
        stages: vec![
            stage("apple_unripe", Some(40.0), Vec::new()),
            stage("apple_ripe", None, vec![give("apple", 1, 2)]),
        ],
        density: 0.12,
        harvest_to: 0,
        shape: Some("terra:apple".into()),
    };
    let cane = PlantDef {
        name: "cane".into(),
        seed: "terra:cane".into(),
        shape: Some(CROSS.into()),
        needs: PlantNeeds {
            water: Some(PlantWater {
                fluid: "terra:water".into(),
                reach_m: 3.0,
            }),
            ..needs(&["sand", "dirt", "grass"])
        },
        kind: PlantKind::Column(ColumnDef {
            material: "terra:cane".into(),
            max_height_m: 3.0,
            direction: ColumnDirection::Up,
            mean_seconds: 60.0,
            stands_in: None,
            yields: Vec::new(),
        }),
    };
    let kelp = PlantDef {
        name: "kelp".into(),
        seed: "terra:kelp".into(),
        shape: Some(CROSS.into()),
        needs: needs(&["sand", "dirt", "grass", "stone"]),
        kind: PlantKind::Column(ColumnDef {
            material: "terra:kelp".into(),
            max_height_m: 5.0,
            direction: ColumnDirection::Up,
            mean_seconds: 90.0,
            stands_in: Some("terra:water".into()),
            yields: Vec::new(),
        }),
    };
    let mushroom = PlantDef {
        name: "mushroom".into(),
        seed: "terra:mushroom".into(),
        shape: Some(CROSS.into()),
        needs: PlantNeeds {
            light_max: Some(7),
            ..needs(&["dirt", "grass", "stone", "snow"])
        },
        kind: PlantKind::Spreader(SpreaderDef {
            material: "terra:mushroom".into(),
            reach_m: 2.0,
            mean_seconds: 120.0,
            density: 4,
            yields: Vec::new(),
        }),
    };
    let vine = PlantDef {
        name: "vine".into(),
        seed: "terra:vine".into(),
        shape: Some(PLANE.into()),
        needs: needs(&["stone", "dirt", "brick", "wood", "grass"]),
        kind: PlantKind::Climber(ClimberDef {
            material: "terra:vine".into(),
            mean_seconds: 90.0,
            length_m: 6.0,
            yields: Vec::new(),
        }),
    };
    vec![
        wheat,
        berry,
        tree("oak", None),
        tree("apple", Some(apple_fruit)),
        cane,
        kelp,
        mushroom,
        vine,
    ]
}

fn feature(plant: &str, spacing_m: f32) -> SurfaceFeature {
    SurfaceFeature {
        material: None,
        plant: Some(format!("terra:{plant}")),
        shape: None,
        spacing_m,
    }
}

/// A Biome holds at most four features, so the Plants are shared out between the three.
pub fn temperate() -> Vec<SurfaceFeature> {
    vec![
        feature("oak", 9.0),
        feature("apple", 26.0),
        feature("wheat", 22.0),
    ]
}

pub fn dry() -> Vec<SurfaceFeature> {
    vec![
        feature("cane", 18.0),
        feature("berry", 20.0),
        feature("kelp", 8.0),
    ]
}

pub fn cold() -> Vec<SurfaceFeature> {
    vec![feature("mushroom", 16.0), feature("kelp", 8.0)]
}
