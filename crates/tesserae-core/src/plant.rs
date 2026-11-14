//! Plants: declared growth rules, by Material, resolved and validated once
//! every Mod has declared.
//!
//! A Plant's state is its Material, so a Stage is a Material and nothing is
//! stored per plant. The table answers "which Plant, and what role, is this
//! Material?" for growth, harvest, support and the undo journal.

use std::collections::HashMap;

use crate::chunk::CHUNK_VOLUME;
use crate::light::MAX_LIGHT;
use crate::voxel::MaterialId;

/// Samples a held Chunk that holds a Plant Material draws each Logical step.
pub const PLANT_SAMPLES: u32 = 32;
/// Logical steps in one second.
pub const STEPS_PER_S: u32 = 64;
/// Mean seconds between Samples of one Voxel: the fastest a Plant can roll, so
/// the floor of every `mean-seconds`.
pub const SAMPLE_INTERVAL_S: f32 = CHUNK_VOLUME as f32 / (PLANT_SAMPLES * STEPS_PER_S) as f32;

pub const MAX_STAGES: usize = 8;
pub const MAX_FRUIT_STAGES: usize = 4;
/// Largest `canopy-radius-m`, `trunk-height-m` and `max-height-m` in metres.
pub const MAX_CANOPY_RADIUS_M: f32 = 8.0;
pub const MAX_PLANT_HEIGHT_M: f32 = 32.0;
pub const MAX_SPREAD_REACH_M: f32 = 8.0;
pub const MAX_FRUIT_DENSITY: f32 = 0.3;

/// A count from `min` to `max` inclusive.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CountRange {
    pub min: u32,
    pub max: u32,
}

/// What one Plant Voxel gives when it is removed or picked.
#[derive(Clone, PartialEq, Debug)]
pub struct Yield {
    pub item: String,
    pub count: CountRange,
}

/// A Material a Plant passes through, and how long it stays.
#[derive(Clone, PartialEq, Debug)]
pub struct Stage {
    pub material: MaterialId,
    /// Mean seconds to reach the next Stage; `None` on the last.
    pub mean_seconds: Option<f32>,
    pub yields: Vec<Yield>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Water {
    pub fluid: MaterialId,
    pub reach_m: f32,
}

/// What a Plant needs to grow. Failing any holds its Stage.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Needs {
    pub soil: Vec<MaterialId>,
    pub light_min: u8,
    pub light_max: u8,
    pub temperature: Option<(f32, f32)>,
    pub humidity: Option<(f32, f32)>,
    pub water: Option<Water>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Fruit {
    pub stages: Vec<Stage>,
    pub density: f32,
    pub harvest_to: usize,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Tree {
    pub trunk: MaterialId,
    pub canopy: MaterialId,
    pub trunk_height_m: (f32, f32),
    pub canopy_radius_m: f32,
    pub mean_seconds: f32,
    pub trunk_yield: Vec<Yield>,
    pub canopy_yield: Vec<Yield>,
    pub sapling_yield: CountRange,
    pub fruit: Option<Fruit>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Direction {
    Up,
    Down,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Kind {
    Crop {
        stages: Vec<Stage>,
        harvest_to: Option<usize>,
    },
    /// `stages` holds the sapling alone: its Material is the seed's and its
    /// `yields` are what removing it gives.
    Tree(Tree),
    Column {
        max_height_m: f32,
        direction: Direction,
        mean_seconds: f32,
        yields: Vec<Yield>,
    },
    Spreader {
        reach_m: f32,
        mean_seconds: f32,
        density: u32,
        yields: Vec<Yield>,
    },
    Climber {
        mean_seconds: f32,
        length_m: f32,
        yields: Vec<Yield>,
    },
}

#[derive(Clone, PartialEq, Debug)]
pub struct Plant {
    pub name: String,
    /// The Item that places the first Stage.
    pub seed: String,
    /// The Material the seed places: a crop's first Stage, a tree's sapling,
    /// or the one Material of any other kind.
    pub first: MaterialId,
    pub needs: Needs,
    pub kind: Kind,
    /// A tree's sapling yields; empty for other kinds, whose yields live in
    /// their Stages or fields.
    pub sapling_yields: Vec<Yield>,
}

/// What a Material is to the Plant that owns it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    /// Stage `n` of a crop.
    Stage(usize),
    /// The one Material of a column, spreader or climber.
    Single,
    Sapling,
    Trunk,
    Canopy,
    /// Stage `n` of a tree's fruit.
    Fruit(usize),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PlantRef {
    pub plant: usize,
    pub role: Role,
}

#[derive(Clone, PartialEq, Debug)]
pub enum PlantError {
    NoSoil,
    Light,
    Range(&'static str),
    Stages(&'static str),
    Count(&'static str),
    Size(&'static str),
    HarvestTo,
    Material(MaterialId),
}

impl std::fmt::Display for PlantError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlantError::NoSoil => write!(f, "needs at least one soil Material"),
            PlantError::Light => write!(f, "light-min must not exceed light-max, at most 15"),
            PlantError::Range(what) => write!(f, "{what}: a range is from 0 to 1, low to high"),
            PlantError::Stages(rule) => write!(f, "stages: {rule}"),
            PlantError::Count(rule) => write!(f, "yield: {rule}"),
            PlantError::Size(rule) => write!(f, "{rule}"),
            PlantError::HarvestTo => write!(f, "harvest-to must name an earlier Stage"),
            PlantError::Material(id) => {
                write!(f, "Material {id} is already part of another Plant")
            }
        }
    }
}

impl std::error::Error for PlantError {}

/// Every Plant, and the lookup from a Material to its Plant and role.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct PlantTable {
    plants: Vec<Plant>,
    by_material: HashMap<MaterialId, PlantRef>,
}

impl PlantTable {
    pub fn is_empty(&self) -> bool {
        self.plants.is_empty()
    }

    pub fn len(&self) -> usize {
        self.plants.len()
    }

    pub fn plants(&self) -> &[Plant] {
        &self.plants
    }

    pub fn plant(&self, index: usize) -> Option<&Plant> {
        self.plants.get(index)
    }

    /// The Plant a Material belongs to and its role there.
    pub fn of(&self, material: MaterialId) -> Option<PlantRef> {
        self.by_material.get(&material).copied()
    }

    /// Whether a Material is made only by growth and worldgen: a later Stage,
    /// a tree's trunk, canopy or fruit. A Player cannot place it and no Item
    /// names it.
    pub fn is_grown(&self, material: MaterialId) -> bool {
        match self.of(material).map(|r| r.role) {
            Some(Role::Stage(n)) => n > 0,
            Some(Role::Trunk | Role::Canopy | Role::Fruit(_)) => true,
            Some(Role::Single | Role::Sapling) | None => false,
        }
    }

    /// The Material a crop advances to from `material`, if it is a crop Stage
    /// that has a next.
    pub fn next_stage(&self, material: MaterialId) -> Option<MaterialId> {
        let PlantRef { plant, role } = self.of(material)?;
        match (&self.plants.get(plant)?.kind, role) {
            (Kind::Crop { stages, .. }, Role::Stage(n)) => stages.get(n + 1).map(|s| s.material),
            _ => None,
        }
    }

    /// Registers a Plant, checking every rule that needs no name lookup.
    pub fn push(&mut self, plant: Plant) -> Result<(), PlantError> {
        check(&plant)?;
        let index = self.plants.len();
        let mut claimed: Vec<(MaterialId, Role)> = Vec::new();
        match &plant.kind {
            Kind::Crop { stages, .. } => {
                claimed.extend(
                    stages
                        .iter()
                        .enumerate()
                        .map(|(n, s)| (s.material, Role::Stage(n))),
                );
            }
            Kind::Tree(tree) => {
                claimed.push((plant.first, Role::Sapling));
                claimed.push((tree.trunk, Role::Trunk));
                claimed.push((tree.canopy, Role::Canopy));
                if let Some(fruit) = &tree.fruit {
                    claimed.extend(
                        fruit
                            .stages
                            .iter()
                            .enumerate()
                            .map(|(n, s)| (s.material, Role::Fruit(n))),
                    );
                }
            }
            Kind::Column { .. } | Kind::Spreader { .. } | Kind::Climber { .. } => {
                claimed.push((plant.first, Role::Single));
            }
        }
        for (i, &(material, _)) in claimed.iter().enumerate() {
            if self.by_material.contains_key(&material)
                || claimed[..i].iter().any(|&(m, _)| m == material)
            {
                return Err(PlantError::Material(material));
            }
        }
        for (material, role) in claimed {
            self.by_material
                .insert(material, PlantRef { plant: index, role });
        }
        self.plants.push(plant);
        Ok(())
    }
}

fn check(plant: &Plant) -> Result<(), PlantError> {
    let needs = &plant.needs;
    if needs.soil.is_empty() {
        return Err(PlantError::NoSoil);
    }
    if needs.light_max > MAX_LIGHT || needs.light_min > needs.light_max {
        return Err(PlantError::Light);
    }
    for (what, range) in [
        ("temperature", needs.temperature),
        ("humidity", needs.humidity),
    ] {
        if let Some((low, high)) = range
            && !((0.0..=1.0).contains(&low) && (0.0..=1.0).contains(&high) && low <= high)
        {
            return Err(PlantError::Range(what));
        }
    }
    if let Some(water) = needs.water
        && !(water.reach_m.is_finite()
            && water.reach_m > 0.0
            && water.reach_m <= MAX_SPREAD_REACH_M)
    {
        return Err(PlantError::Size(
            "water reach-m must be above 0 and at most 8",
        ));
    }
    match &plant.kind {
        Kind::Crop { stages, harvest_to } => {
            check_stages(stages, MAX_STAGES, 2)?;
            if let Some(to) = *harvest_to
                && to + 1 >= stages.len()
            {
                return Err(PlantError::HarvestTo);
            }
        }
        Kind::Tree(tree) => {
            check_yields(&tree.trunk_yield)?;
            check_yields(&tree.canopy_yield)?;
            check_yields(&plant.sapling_yields)?;
            check_count(tree.sapling_yield)?;
            if tree.sapling_yield.min < 1 {
                return Err(PlantError::Count("sapling-yield must give at least 1"));
            }
            let (low, high) = tree.trunk_height_m;
            if !(low.is_finite() && high.is_finite() && 0.0 < low && low <= high)
                || high > MAX_PLANT_HEIGHT_M
            {
                return Err(PlantError::Size(
                    "trunk-height-m is a range above 0 and at most 32",
                ));
            }
            if !(tree.canopy_radius_m.is_finite()
                && tree.canopy_radius_m > 0.0
                && tree.canopy_radius_m <= MAX_CANOPY_RADIUS_M)
            {
                return Err(PlantError::Size(
                    "canopy-radius-m must be above 0 and at most 8",
                ));
            }
            floor(tree.mean_seconds)?;
            if let Some(fruit) = &tree.fruit {
                check_stages(&fruit.stages, MAX_FRUIT_STAGES, 2)?;
                if !(0.0..=MAX_FRUIT_DENSITY).contains(&fruit.density) {
                    return Err(PlantError::Size("fruit density is from 0 to 0.3"));
                }
                if fruit.harvest_to + 1 >= fruit.stages.len() {
                    return Err(PlantError::HarvestTo);
                }
            }
        }
        Kind::Column {
            max_height_m,
            mean_seconds,
            yields,
            ..
        } => {
            check_yields(yields)?;
            floor(*mean_seconds)?;
            if !(max_height_m.is_finite()
                && *max_height_m > 0.0
                && *max_height_m <= MAX_PLANT_HEIGHT_M)
            {
                return Err(PlantError::Size(
                    "max-height-m must be above 0 and at most 32",
                ));
            }
        }
        Kind::Spreader {
            reach_m,
            mean_seconds,
            density,
            yields,
        } => {
            check_yields(yields)?;
            floor(*mean_seconds)?;
            if !(reach_m.is_finite() && (1.0..=MAX_SPREAD_REACH_M).contains(reach_m)) {
                return Err(PlantError::Size("reach-m is from 1 to 8"));
            }
            if *density == 0 {
                return Err(PlantError::Size("density must be at least 1"));
            }
        }
        Kind::Climber {
            mean_seconds,
            length_m,
            yields,
        } => {
            check_yields(yields)?;
            floor(*mean_seconds)?;
            if !(length_m.is_finite() && *length_m > 0.0 && *length_m <= MAX_PLANT_HEIGHT_M) {
                return Err(PlantError::Size("length-m must be above 0 and at most 32"));
            }
        }
    }
    Ok(())
}

fn floor(mean_seconds: f32) -> Result<(), PlantError> {
    if mean_seconds.is_finite() && mean_seconds >= SAMPLE_INTERVAL_S {
        Ok(())
    } else {
        Err(PlantError::Size(
            "mean-seconds is below the Sample interval of 16 s",
        ))
    }
}

fn check_stages(stages: &[Stage], max: usize, min: usize) -> Result<(), PlantError> {
    if stages.len() < min || stages.len() > max {
        return Err(PlantError::Stages("a chain holds 2 to its limit of Stages"));
    }
    for (n, stage) in stages.iter().enumerate() {
        check_yields(&stage.yields)?;
        match (stage.mean_seconds, n + 1 == stages.len()) {
            (Some(s), false) => floor(s)?,
            (None, true) => {}
            _ => {
                return Err(PlantError::Stages(
                    "every Stage but the last has mean-seconds, and the last has none",
                ));
            }
        }
    }
    Ok(())
}

fn check_yields(yields: &[Yield]) -> Result<(), PlantError> {
    for y in yields {
        check_count(y.count)?;
    }
    Ok(())
}

fn check_count(count: CountRange) -> Result<(), PlantError> {
    if count.min > count.max {
        return Err(PlantError::Count("min is above max"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stage(material: MaterialId, mean: Option<f32>) -> Stage {
        Stage {
            material,
            mean_seconds: mean,
            yields: Vec::new(),
        }
    }

    fn crop(stages: Vec<Stage>, harvest_to: Option<usize>) -> Plant {
        Plant {
            name: "wheat".into(),
            seed: "t:seed".into(),
            first: stages[0].material,
            needs: Needs {
                soil: vec![1],
                light_min: 8,
                light_max: 15,
                ..Needs::default()
            },
            kind: Kind::Crop { stages, harvest_to },
            sapling_yields: Vec::new(),
        }
    }

    #[test]
    fn a_crop_resolves_each_stage_to_its_plant() {
        let mut table = PlantTable::default();
        table
            .push(crop(
                vec![
                    stage(10, Some(60.0)),
                    stage(11, Some(60.0)),
                    stage(12, None),
                ],
                Some(1),
            ))
            .unwrap();
        assert_eq!(
            table.of(11),
            Some(PlantRef {
                plant: 0,
                role: Role::Stage(1)
            })
        );
        assert_eq!(table.next_stage(10), Some(11));
        assert_eq!(table.next_stage(12), None);
        assert!(!table.is_grown(10));
        assert!(table.is_grown(12));
    }

    #[test]
    fn a_material_belongs_to_one_plant() {
        let mut table = PlantTable::default();
        let chain = || vec![stage(10, Some(60.0)), stage(11, None)];
        table.push(crop(chain(), None)).unwrap();
        assert_eq!(
            table.push(crop(chain(), None)),
            Err(PlantError::Material(10))
        );
    }

    #[test]
    fn mean_seconds_cannot_beat_the_sample_interval() {
        let mut table = PlantTable::default();
        let fast = vec![stage(10, Some(SAMPLE_INTERVAL_S - 1.0)), stage(11, None)];
        assert!(matches!(
            table.push(crop(fast, None)),
            Err(PlantError::Size(_))
        ));
    }

    #[test]
    fn harvest_to_names_an_earlier_stage() {
        let mut table = PlantTable::default();
        let chain = vec![stage(10, Some(60.0)), stage(11, None)];
        assert_eq!(table.push(crop(chain, Some(1))), Err(PlantError::HarvestTo));
    }

    #[test]
    fn a_chain_of_one_is_refused() {
        let mut table = PlantTable::default();
        assert!(matches!(
            table.push(crop(vec![stage(10, None)], None)),
            Err(PlantError::Stages(_))
        ));
    }

    #[test]
    fn the_sample_interval_is_sixteen_seconds() {
        assert_eq!(SAMPLE_INTERVAL_S, 16.0);
    }
}
