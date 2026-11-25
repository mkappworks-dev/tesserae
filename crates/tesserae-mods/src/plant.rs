//! Every Mod's `plants`, resolved against the World's Materials and Items.

use tesserae_core::plant::{
    Branching, Canopies, CountRange, DEFAULT_VINE_FRUIT, DEFAULT_VINE_GAP_M, DEFAULT_VINE_REACH_M,
    Direction, Fruit, Kind, MAX_CANOPIES, MAX_STOREYS, Needs, Plant, PlantTable, Stage, Storey,
    Storeys, Tree, TreeVines, VineFruit, Water, Yield,
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
pub struct DeclaredPlants {
    rows: Vec<(String, Vec<content::PlantDef>)>,
    /// Every tagged Material of every Mod, by full name.
    tags: Vec<(String, Vec<String>)>,
}

impl DeclaredPlants {
    pub fn take(behaviours: &mut [Behaviour]) -> Self {
        Self {
            rows: behaviours
                .iter_mut()
                .map(|b| (b.mod_id.clone(), std::mem::take(&mut b.plants)))
                .collect(),
            tags: behaviours
                .iter()
                .flat_map(|b| b.material_tags.iter().cloned())
                .collect(),
        }
    }

    fn rows(&self) -> impl Iterator<Item = (&str, usize, &content::PlantDef)> {
        self.rows.iter().flat_map(|(id, rows)| {
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
            let mut plant = plant(def, registry, tables, &self.tags).map_err(&fail)?;
            plant.name = format!("{mod_id}:{}", def.name);
            table.push(plant).map_err(|e| fail(e.to_string()))?;
        }
        // Vines name a Plant of any Mod, declared before or after the tree.
        for (index, (mod_id, row, def)) in self.rows().enumerate() {
            let Some(Kind::Tree(tree)) = table.plant(index).map(|p| &p.kind) else {
                continue;
            };
            let Some(vines) = &tree.vines else {
                continue;
            };
            let fail = |reason: String| ModError::Plant {
                id: mod_id.to_owned(),
                row,
                name: def.name.clone(),
                reason,
            };
            table.vines(&tables.shapes, vines).map_err(&fail)?;
            let holds = |plant: &Plant| {
                std::iter::once(&tree.trunk)
                    .chain(tree.canopy.as_slice())
                    .any(|m| plant.needs.soil.contains(m))
            };
            if !table
                .plants()
                .iter()
                .any(|p| p.name == vines.plant && holds(p))
            {
                return Err(fail(format!(
                    "vines name `{}`, whose soil lists neither this tree's trunk nor its canopy, \
                     so they would fall at once",
                    vines.plant
                )));
            }
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
        Kind::Crop {
            stages, fruit_vine, ..
        } => {
            all.extend(stages.iter().flat_map(|s| &s.yields));
            if let Some(fruit) = fruit_vine {
                all.extend(fruit.stages.iter().flat_map(|s| &s.yields));
            }
        }
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

fn canopies(registry: &Registry, tables: &Tables, names: &[String]) -> Result<Canopies, String> {
    let materials = names
        .iter()
        .map(|n| solid(registry, tables, n))
        .collect::<Result<Vec<_>, _>>()?;
    Canopies::new(&materials).ok_or_else(|| format!("canopy names 1 to {MAX_CANOPIES} Materials"))
}

/// A storey's crown defaults to the row's; the core checks the rest once the row is whole.
fn storeys(tree: &content::TreeDef) -> Result<Storeys, String> {
    let list: Vec<Storey> = tree
        .storeys
        .iter()
        .map(|s| Storey {
            weight: s.weight,
            trunk_height_m: s.trunk_height_m,
            canopy_radius_m: s.canopy_radius_m.unwrap_or(tree.canopy_radius_m),
        })
        .collect();
    Storeys::new(&list).ok_or_else(|| format!("a tree declares at most {MAX_STOREYS} storeys"))
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

/// What `entry` names: one Material, or with a leading `#` every Material of any Mod carrying that
/// tag. `keep` filters the tagged ones (solid soil, fluids); a tag left with none is an error.
fn expand(
    registry: &Registry,
    tags: &[(String, Vec<String>)],
    entry: &str,
    kind: &str,
    keep: impl Fn(MaterialId) -> bool,
) -> Result<Vec<MaterialId>, String> {
    let Some(tag) = entry.strip_prefix('#') else {
        let id = material(registry, entry)?;
        return if keep(id) {
            Ok(vec![id])
        } else {
            Err(format!("`{entry}` must be a {kind} Material"))
        };
    };
    let found: Vec<MaterialId> = tags
        .iter()
        .filter(|(_, own)| own.iter().any(|t| t == tag))
        .filter_map(|(name, _)| registry.id_of(NameKind::Material, name))
        .map(|id| id as MaterialId)
        .filter(|&id| keep(id))
        .collect();
    if found.is_empty() {
        return Err(format!("tag `{entry}` matches no {kind} Material"));
    }
    Ok(found)
}

fn plant(
    def: &content::PlantDef,
    registry: &Registry,
    tables: &Tables,
    tags: &[(String, Vec<String>)],
) -> Result<Plant, String> {
    let shape = resolve_shape(registry, &def.shape)?;
    let needs = needs(&def.needs, registry, tables, tags)?;
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
                    fruit_vine: crop
                        .fruit_vine
                        .as_ref()
                        .map(|fruit| {
                            Ok::<_, String>(VineFruit {
                                stages: crate::plant::stages(&fruit.stages, registry, tables)?,
                                regrow_seconds: fruit.regrow_seconds,
                                shape: resolve_shape(registry, &fruit.shape)?,
                                reach_m: fruit.reach_m.unwrap_or(DEFAULT_VINE_REACH_M),
                                gap_m: fruit.gap_m.unwrap_or(DEFAULT_VINE_GAP_M),
                                max_fruit: fruit.max_fruit.unwrap_or(DEFAULT_VINE_FRUIT),
                            })
                        })
                        .transpose()?,
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
                        regrow_seconds: fruit.regrow_seconds,
                        shape: resolve_shape(registry, &fruit.shape)?,
                    })
                })
                .transpose()?;
            (
                solid(registry, tables, &tree.sapling)?,
                Kind::Tree(Tree {
                    trunk: solid(registry, tables, &tree.trunk)?,
                    canopy: canopies(registry, tables, &tree.canopy)?,
                    storeys: storeys(tree)?,
                    trunk_height_m: tree.trunk_height_m,
                    canopy_radius_m: tree.canopy_radius_m,
                    mean_seconds: tree.mean_seconds,
                    trunk_yield: yields(&tree.trunk_yield),
                    canopy_yield: yields(&tree.canopy_yield),
                    sapling_yield: count(tree.sapling_yield),
                    fruit,
                    vines: tree.vines.as_ref().map(|v| TreeVines {
                        plant: v.plant.clone(),
                        density: v.density,
                        length_m: v.length_m,
                    }),
                    branching: tree.branching.as_ref().map(|b| Branching {
                        limbs: count(b.limbs),
                        start: b.start,
                        reach_m: b.reach_m,
                        rise: b.rise,
                        fork: b.fork,
                        lean_m: b.lean_m,
                        tiers: b.tiers.unwrap_or(false),
                        lobe_radius_m: b.lobe_radius_m,
                    }),
                    crowding_m: tree.crowding_m,
                }),
            )
        }
        content::PlantKind::Column(column) => {
            let stands_in = column
                .stands_in
                .as_ref()
                .map(|name| {
                    expand(registry, tags, name, "fluid", |m| {
                        tables.fluids.rank(m).is_some()
                    })
                })
                .transpose()?
                .unwrap_or_default();
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
                min_neighbours: spreader.min_neighbours.unwrap_or(0),
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
    tags: &[(String, Vec<String>)],
) -> Result<Needs, String> {
    let water = needs
        .water
        .as_ref()
        .map(|w| {
            Ok::<_, String>(Water {
                fluids: expand(registry, tags, &w.fluid, "fluid", |m| {
                    tables.fluids.rank(m).is_some()
                })?,
                reach_m: w.reach_m,
            })
        })
        .transpose()?;
    let mut soil: Vec<MaterialId> = Vec::new();
    for name in &needs.soil {
        for id in expand(registry, tags, name, "solid", |m| {
            tables.materials.solid(Voxel::cube(m))
        })? {
            if !soil.contains(&id) {
                soil.push(id);
            }
        }
    }
    Ok(Needs {
        soil,
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
        .enumerate()
        .map(|(n, stage)| {
            Ok(Stage {
                material: solid(registry, tables, &stage.material)?,
                mean_seconds: stage.mean_seconds,
                yields: yields(&stage.yields),
                shape: resolve_shape(registry, &stage.shape)
                    .map_err(|e| format!("Stage {n} {e}"))?,
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
