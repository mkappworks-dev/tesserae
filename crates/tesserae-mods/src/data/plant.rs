use serde::Deserialize;

use crate::bindings::tesserae::mods::content;

/// Tagged by `kind`. The common fields repeat in each kind so that unknown
/// fields are still refused, which a flattened struct would not allow.
// Rows are parsed once at load and converted at once, so boxing the large
// variant would only add an indirection to serde's tagged layout.
#[allow(clippy::large_enum_variant)]
#[derive(Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "kebab-case",
    deny_unknown_fields
)]
pub(super) enum Plant {
    Crop {
        name: String,
        seed: String,
        shape: Option<String>,
        needs: Needs,
        stages: Vec<Stage>,
        harvest_to: Option<u32>,
        fruit_vine: Option<FruitVine>,
    },
    Tree {
        name: String,
        seed: String,
        shape: Option<String>,
        needs: Needs,
        sapling: String,
        #[serde(default)]
        sapling_yields: Vec<Yield>,
        trunk: String,
        canopy: String,
        trunk_height_m: [f32; 2],
        canopy_radius_m: f32,
        mean_seconds: f32,
        #[serde(default)]
        trunk_yield: Vec<Yield>,
        #[serde(default)]
        canopy_yield: Vec<Yield>,
        sapling_yield: [u32; 2],
        fruit: Option<Fruit>,
        vines: Option<Vines>,
        branching: Option<Branching>,
    },
    Column {
        name: String,
        seed: String,
        shape: Option<String>,
        needs: Needs,
        material: String,
        max_height_m: f32,
        direction: Direction,
        mean_seconds: f32,
        stands_in: Option<String>,
        #[serde(default)]
        yields: Vec<Yield>,
    },
    Spreader {
        name: String,
        seed: String,
        shape: Option<String>,
        needs: Needs,
        material: String,
        reach_m: f32,
        mean_seconds: f32,
        density: u32,
        min_neighbours: Option<u32>,
        #[serde(default)]
        yields: Vec<Yield>,
    },
    Climber {
        name: String,
        seed: String,
        shape: Option<String>,
        needs: Needs,
        material: String,
        mean_seconds: f32,
        length_m: f32,
        #[serde(default)]
        yields: Vec<Yield>,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(super) struct Needs {
    soil: Vec<String>,
    light_min: Option<u8>,
    light_max: Option<u8>,
    temperature: Option<[f32; 2]>,
    humidity: Option<[f32; 2]>,
    water: Option<Water>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(super) struct Water {
    fluid: String,
    reach_m: f32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(super) struct Stage {
    material: String,
    mean_seconds: Option<f32>,
    #[serde(default)]
    yields: Vec<Yield>,
    shape: Option<String>,
}

/// Seconds between a tree's new fruit when a row names none.
fn default_regrow_seconds() -> f32 {
    120.0
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(super) struct Fruit {
    stages: Vec<Stage>,
    density: f32,
    #[serde(default = "default_regrow_seconds")]
    regrow_seconds: f32,
    shape: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(super) struct FruitVine {
    stages: Vec<Stage>,
    #[serde(default = "default_regrow_seconds")]
    regrow_seconds: f32,
    shape: Option<String>,
    reach_m: Option<f32>,
    gap_m: Option<f32>,
    max_fruit: Option<u32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(super) struct Branching {
    limbs: [u32; 2],
    start: f32,
    reach_m: f32,
    rise: f32,
    fork: Option<f32>,
    lean_m: f32,
    tiers: Option<bool>,
    lobe_radius_m: f32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(super) struct Vines {
    plant: String,
    density: f32,
    length_m: f32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(super) struct Yield {
    item: String,
    count: [u32; 2],
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
pub(super) enum Direction {
    Up,
    Down,
}

fn count([min, max]: [u32; 2]) -> content::CountRange {
    content::CountRange { min, max }
}

fn yields(rows: Vec<Yield>) -> Vec<content::PlantYield> {
    rows.into_iter()
        .map(|y| content::PlantYield {
            item: y.item,
            count: count(y.count),
        })
        .collect()
}

fn stages(rows: Vec<Stage>) -> Vec<content::PlantStage> {
    rows.into_iter()
        .map(|s| content::PlantStage {
            material: s.material,
            mean_seconds: s.mean_seconds,
            yields: yields(s.yields),
            shape: s.shape,
        })
        .collect()
}

fn needs(n: Needs) -> content::PlantNeeds {
    content::PlantNeeds {
        soil: n.soil,
        light_min: n.light_min,
        light_max: n.light_max,
        temperature: n.temperature.map(|[a, b]| (a, b)),
        humidity: n.humidity.map(|[a, b]| (a, b)),
        water: n.water.map(|w| content::PlantWater {
            fluid: w.fluid,
            reach_m: w.reach_m,
        }),
    }
}

impl From<Plant> for content::PlantDef {
    fn from(plant: Plant) -> Self {
        match plant {
            Plant::Crop {
                name,
                seed,
                shape,
                needs: n,
                stages: rows,
                harvest_to,
                fruit_vine,
            } => Self {
                name,
                seed,
                shape,
                needs: needs(n),
                kind: content::PlantKind::Crop(content::CropDef {
                    stages: stages(rows),
                    harvest_to,
                    fruit_vine: fruit_vine.map(|f| content::FruitVineDef {
                        stages: stages(f.stages),
                        regrow_seconds: f.regrow_seconds,
                        shape: f.shape,
                        reach_m: f.reach_m,
                        gap_m: f.gap_m,
                        max_fruit: f.max_fruit,
                    }),
                }),
            },
            Plant::Tree {
                name,
                seed,
                shape,
                needs: n,
                sapling,
                sapling_yields,
                trunk,
                canopy,
                trunk_height_m: [low, high],
                canopy_radius_m,
                mean_seconds,
                trunk_yield,
                canopy_yield,
                sapling_yield,
                fruit,
                vines,
                branching,
            } => Self {
                name,
                seed,
                shape,
                needs: needs(n),
                kind: content::PlantKind::Tree(content::TreeDef {
                    sapling,
                    sapling_yields: yields(sapling_yields),
                    trunk,
                    canopy,
                    trunk_height_m: (low, high),
                    canopy_radius_m,
                    mean_seconds,
                    trunk_yield: yields(trunk_yield),
                    canopy_yield: yields(canopy_yield),
                    sapling_yield: count(sapling_yield),
                    fruit: fruit.map(|f| content::FruitDef {
                        stages: stages(f.stages),
                        density: f.density,
                        regrow_seconds: f.regrow_seconds,
                        shape: f.shape,
                    }),
                    vines: vines.map(|v| content::TreeVines {
                        plant: v.plant,
                        density: v.density,
                        length_m: v.length_m,
                    }),
                    branching: branching.map(|b| content::BranchingDef {
                        limbs: count(b.limbs),
                        start: b.start,
                        reach_m: b.reach_m,
                        rise: b.rise,
                        fork: b.fork,
                        lean_m: b.lean_m,
                        tiers: b.tiers,
                        lobe_radius_m: b.lobe_radius_m,
                    }),
                }),
            },
            Plant::Column {
                name,
                seed,
                shape,
                needs: n,
                material,
                max_height_m,
                direction,
                mean_seconds,
                stands_in,
                yields: rows,
            } => Self {
                name,
                seed,
                shape,
                needs: needs(n),
                kind: content::PlantKind::Column(content::ColumnDef {
                    material,
                    max_height_m,
                    direction: match direction {
                        Direction::Up => content::ColumnDirection::Up,
                        Direction::Down => content::ColumnDirection::Down,
                    },
                    mean_seconds,
                    stands_in,
                    yields: yields(rows),
                }),
            },
            Plant::Spreader {
                name,
                seed,
                shape,
                needs: n,
                material,
                reach_m,
                mean_seconds,
                density,
                min_neighbours,
                yields: rows,
            } => Self {
                name,
                seed,
                shape,
                needs: needs(n),
                kind: content::PlantKind::Spreader(content::SpreaderDef {
                    material,
                    reach_m,
                    mean_seconds,
                    density,
                    min_neighbours,
                    yields: yields(rows),
                }),
            },
            Plant::Climber {
                name,
                seed,
                shape,
                needs: n,
                material,
                mean_seconds,
                length_m,
                yields: rows,
            } => Self {
                name,
                seed,
                shape,
                needs: needs(n),
                kind: content::PlantKind::Climber(content::ClimberDef {
                    material,
                    mean_seconds,
                    length_m,
                    yields: yields(rows),
                }),
            },
        }
    }
}
