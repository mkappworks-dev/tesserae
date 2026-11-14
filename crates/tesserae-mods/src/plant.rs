//! Every Mod's `plants`, resolved against the World's Materials and Items.

use tesserae_core::plant::{
    CountRange, Direction, Fruit, Kind, Needs, Plant, PlantTable, Stage, Tree, Water, Yield,
};
use tesserae_core::{MaterialId, ShapeId, Tables, Voxel};

use crate::bindings::tesserae::mods::content;
use crate::declare::{Behaviour, ItemDef};
use crate::error::ModError;
use crate::registry::{Kind as NameKind, Registry};

pub const MAX_PLANTS_PER_MOD: usize = 128;
pub const MAX_PLANTS: usize = 512;

/// Every Mod's Plant rows in load order, taken from its Behaviour before
/// [`crate::resolve`] consumes it. Materials resolve first, because which
/// Materials are grown decides which implicit Items exist; Items are checked
/// once those are known.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct DeclaredPlants(Vec<(String, Vec<content::PlantDef>)>);

impl DeclaredPlants {
    pub fn take(behaviours: &mut [Behaviour]) -> Self {
        Self(
            behaviours
                .iter_mut()
                .map(|b| (b.mod_id.clone(), std::mem::take(&mut b.plants)))
                .collect(),
        )
    }

    fn rows(&self) -> impl Iterator<Item = (&str, usize, &content::PlantDef)> {
        self.0.iter().flat_map(|(id, rows)| {
            rows.iter()
                .enumerate()
                .map(move |(row, def)| (id.as_str(), row, def))
        })
    }

    /// The Plant table, with every Material name resolved. `tables` must hold
    /// every Material and fluid already.
    pub fn resolve(&self, registry: &Registry, tables: &Tables) -> Result<PlantTable, ModError> {
        let mut table = PlantTable::default();
        for (mod_id, row, def) in self.rows() {
            if table.len() == MAX_PLANTS {
                return Err(ModError::TooManyRows {
                    id: mod_id.to_owned(),
                    what: "plant",
                    count: table.len() + 1,
                    max: MAX_PLANTS,
                });
            }
            let fail = |reason: String| ModError::Plant {
                id: mod_id.to_owned(),
                row,
                name: def.name.clone(),
                reason,
            };
            let mut plant = plant(def, registry, tables).map_err(&fail)?;
            plant.name = format!("{mod_id}:{}", def.name);
            table.push(plant).map_err(|e| fail(e.to_string()))?;
        }
        Ok(table)
    }

    /// Checks every Item a Plant names, now that the implicit Items exist: the
    /// seed places the first Stage, every yield is an Item, and no Item names a
    /// grown Material.
    pub fn check_items(&self, table: &PlantTable, items: &[ItemDef]) -> Result<(), ModError> {
        for (index, (mod_id, row, def)) in self.rows().enumerate() {
            let fail = |reason: String| ModError::Plant {
                id: mod_id.to_owned(),
                row,
                name: def.name.clone(),
                reason,
            };
            let Some(plant) = table.plant(index) else {
                continue;
            };
            let seed = items
                .iter()
                .find(|item| item.name == plant.seed)
                .ok_or_else(|| {
                    fail(format!(
                        "seed `{}` is no Item a loaded Mod declares",
                        plant.seed
                    ))
                })?;
            if seed.material != Some(plant.first) {
                return Err(fail(format!(
                    "seed `{}` must place the Plant's first Stage",
                    plant.seed
                )));
            }
            for name in yield_items(plant) {
                if !items.iter().any(|item| item.name == name) {
                    return Err(fail(format!(
                        "yield names Item `{name}`, which no loaded Mod declares"
                    )));
                }
            }
        }
        for item in items {
            if let Some(material) = item.material
                && table.is_grown(material)
            {
                return Err(ModError::Plant {
                    id: item.name.split(':').next().unwrap_or_default().to_owned(),
                    row: 0,
                    name: item.name.clone(),
                    reason: "an Item names a grown Material, which only growth and worldgen make"
                        .into(),
                });
            }
        }
        Ok(())
    }
}

fn yield_items(plant: &Plant) -> Vec<&str> {
    let mut all: Vec<&Yield> = plant.sapling_yields.iter().collect();
    match &plant.kind {
        Kind::Crop { stages, .. } => all.extend(stages.iter().flat_map(|s| &s.yields)),
        Kind::Tree(tree) => {
            all.extend(&tree.trunk_yield);
            all.extend(&tree.canopy_yield);
            if let Some(fruit) = &tree.fruit {
                all.extend(fruit.stages.iter().flat_map(|s| &s.yields));
            }
        }
        Kind::Column { yields, .. }
        | Kind::Spreader { yields, .. }
        | Kind::Climber { yields, .. } => all.extend(yields),
    }
    all.into_iter().map(|y| y.item.as_str()).collect()
}

fn material(registry: &Registry, name: &str) -> Result<MaterialId, String> {
    registry
        .id_of(NameKind::Material, name)
        .map(|id| id as MaterialId)
        .ok_or_else(|| format!("names Material `{name}`, which no loaded Mod declares"))
}

fn solid(registry: &Registry, tables: &Tables, name: &str) -> Result<MaterialId, String> {
    let id = material(registry, name)?;
    if tables.materials.solid(Voxel::cube(id)) {
        Ok(id)
    } else {
        Err(format!("`{name}` must be a solid Material"))
    }
}

fn resolve_shape(registry: &Registry, name: &Option<String>) -> Result<Option<ShapeId>, String> {
    name.as_deref()
        .map(|name| {
            registry
                .id_of(NameKind::Shape, name)
                .map(|id| id as ShapeId)
                .ok_or_else(|| format!("names Shape `{name}`, which no loaded Mod declares"))
        })
        .transpose()
}

fn plant(def: &content::PlantDef, registry: &Registry, tables: &Tables) -> Result<Plant, String> {
    let shape = resolve_shape(registry, &def.shape)?;
    let needs = needs(&def.needs, registry, tables)?;
    let mut sapling_yields = Vec::new();
    let (first, kind) = match &def.kind {
        content::PlantKind::Crop(crop) => {
            let stages = stages(&crop.stages, registry, tables)?;
            let first = stages
                .first()
                .map(|s| s.material)
                .ok_or("stages is empty")?;
            (
                first,
                Kind::Crop {
                    stages,
                    harvest_to: crop.harvest_to.map(|n| n as usize),
                },
            )
        }
        content::PlantKind::Tree(tree) => {
            sapling_yields = yields(&tree.sapling_yields);
            let fruit = tree
                .fruit
                .as_ref()
                .map(|fruit| {
                    Ok::<_, String>(Fruit {
                        stages: stages(&fruit.stages, registry, tables)?,
                        density: fruit.density,
                        harvest_to: fruit.harvest_to as usize,
                        shape: resolve_shape(registry, &fruit.shape)?,
                    })
                })
                .transpose()?;
            (
                solid(registry, tables, &tree.sapling)?,
                Kind::Tree(Tree {
                    trunk: solid(registry, tables, &tree.trunk)?,
                    canopy: solid(registry, tables, &tree.canopy)?,
                    trunk_height_m: tree.trunk_height_m,
                    canopy_radius_m: tree.canopy_radius_m,
                    mean_seconds: tree.mean_seconds,
                    trunk_yield: yields(&tree.trunk_yield),
                    canopy_yield: yields(&tree.canopy_yield),
                    sapling_yield: count(tree.sapling_yield),
                    fruit,
                }),
            )
        }
        content::PlantKind::Column(column) => {
            let stands_in = column
                .stands_in
                .as_ref()
                .map(|name| {
                    let fluid = material(registry, name)?;
                    if tables.fluids.rank(fluid).is_none() {
                        return Err(format!("stands-in `{name}` must be a fluid Material"));
                    }
                    Ok(fluid)
                })
                .transpose()?;
            (
                solid(registry, tables, &column.material)?,
                Kind::Column {
                    max_height_m: column.max_height_m,
                    direction: match column.direction {
                        content::ColumnDirection::Up => Direction::Up,
                        content::ColumnDirection::Down => Direction::Down,
                    },
                    mean_seconds: column.mean_seconds,
                    stands_in,
                    yields: yields(&column.yields),
                },
            )
        }
        content::PlantKind::Spreader(spreader) => (
            solid(registry, tables, &spreader.material)?,
            Kind::Spreader {
                reach_m: spreader.reach_m,
                mean_seconds: spreader.mean_seconds,
                density: spreader.density,
                yields: yields(&spreader.yields),
            },
        ),
        content::PlantKind::Climber(climber) => (
            solid(registry, tables, &climber.material)?,
            Kind::Climber {
                mean_seconds: climber.mean_seconds,
                length_m: climber.length_m,
                yields: yields(&climber.yields),
            },
        ),
    };
    Ok(Plant {
        name: def.name.clone(),
        seed: def.seed.clone(),
        first,
        shape,
        needs,
        kind,
        sapling_yields,
    })
}

fn needs(
    needs: &content::PlantNeeds,
    registry: &Registry,
    tables: &Tables,
) -> Result<Needs, String> {
    let water = needs
        .water
        .as_ref()
        .map(|w| {
            let fluid = material(registry, &w.fluid)?;
            if tables.fluids.rank(fluid).is_none() {
                return Err(format!("`{}` must be a fluid Material", w.fluid));
            }
            Ok(Water {
                fluid,
                reach_m: w.reach_m,
            })
        })
        .transpose()?;
    Ok(Needs {
        soil: needs
            .soil
            .iter()
            .map(|name| solid(registry, tables, name))
            .collect::<Result<_, _>>()?,
        light_min: needs.light_min.unwrap_or(0),
        light_max: needs.light_max.unwrap_or(tesserae_core::light::MAX_LIGHT),
        temperature: needs.temperature,
        humidity: needs.humidity,
        water,
    })
}

fn stages(
    rows: &[content::PlantStage],
    registry: &Registry,
    tables: &Tables,
) -> Result<Vec<Stage>, String> {
    rows.iter()
        .map(|stage| {
            Ok(Stage {
                material: solid(registry, tables, &stage.material)?,
                mean_seconds: stage.mean_seconds,
                yields: yields(&stage.yields),
            })
        })
        .collect()
}

fn count(range: content::CountRange) -> CountRange {
    CountRange {
        min: range.min,
        max: range.max,
    }
}

fn yields(rows: &[content::PlantYield]) -> Vec<Yield> {
    rows.iter()
        .map(|y| Yield {
            item: y.item.clone(),
            count: count(y.count),
        })
        .collect()
}
