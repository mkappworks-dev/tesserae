//! Plants: declared growth rules, by Material, resolved and validated once
//! every Mod has declared.
//!
//! A Plant's state is its Material, so a Stage is a Material and nothing is
//! stored per plant. The table answers "which Plant, and what role, is this
//! Material?" for growth, harvest, support and the undo journal.

mod reach;
pub mod tree;

use std::collections::HashMap;

use crate::chunk::{CHUNK_SIZE, CHUNK_VOLUME};
use crate::climate::Climate;
use crate::light::{Light, MAX_LIGHT};
use crate::occupants::Occupants;
use crate::shape::ShapeTable;
use crate::voxel::{MaterialId, ShapeId, Voxel};

/// Samples a held Chunk that holds a Plant Material draws each Logical step.
pub const PLANT_SAMPLES: u32 = 32;
/// Logical steps in one second.
pub const STEPS_PER_S: u32 = 64;
/// Mean seconds between Samples of one Voxel: the fastest a Plant can roll, so
/// the floor of every `mean-seconds`.
pub const SAMPLE_INTERVAL_S: f32 = CHUNK_VOLUME as f32 / (PLANT_SAMPLES * STEPS_PER_S) as f32;

/// Plant changes one Logical step lands; past it they wait for the next step.
pub const PLANTS_LANDED_PER_STEP: usize = 64;
/// Whole trees one Logical step lands, apart from the bound above: a tree is
/// hundreds of Voxels and relights once. One, because an oak at 0.25 m is 3341
/// Voxels and 6.2 ms of CPU, 40% of a 15.6 ms step, and two would be 80%.
pub const TREES_PER_STEP: usize = 1;
/// Voxels of a felled tree one Logical step breaks, top-down.
pub const FELLED_PER_STEP: usize = 32;

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

/// Standing water within reach; any of `fluids` counts.
#[derive(Clone, PartialEq, Debug)]
pub struct Water {
    pub fluids: Vec<MaterialId>,
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
    /// The Shape of every fruit Voxel; `None` is a cube.
    pub shape: Option<ShapeId>,
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
    /// Climbers worldgen hangs from the canopy; a growing sapling never makes them.
    pub vines: Option<TreeVines>,
}

/// A tree's vines as declared: the climber by full name, not yet resolved.
#[derive(Clone, PartialEq, Debug)]
pub struct TreeVines {
    /// The climber Plant's `<mod id>:<name>`.
    pub plant: String,
    /// Share of the outer canopy cells that hang one, from 0 to 0.5.
    pub density: f32,
    pub length_m: f32,
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
        /// The fluid it grows through and stands waterlogged in; `None` grows
        /// in Air.
        /// The fluids it stands in; empty stands on the ground.
        stands_in: Vec<MaterialId>,
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
    /// The Shape of every single-Voxel Stage the Plant places, grows and
    /// generates, whatever the Player chose; `None` keeps the Player's choice.
    pub shape: Option<ShapeId>,
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

    /// What Use on a plant Voxel of `material` does, if it is a crop Stage with a
    /// `harvest-to` or a tree's fruit Stage.
    pub fn harvest(&self, material: MaterialId) -> Option<Harvest<'_>> {
        let PlantRef { plant, role } = self.of(material)?;
        match (&self.plants.get(plant)?.kind, role) {
            (
                Kind::Crop {
                    stages,
                    harvest_to: Some(to),
                },
                Role::Stage(n),
            ) => Some(Harvest {
                to: stages[*to].material,
                ripe: n + 1 == stages.len(),
                yields: &stages[n].yields,
            }),
            (
                Kind::Tree(Tree {
                    fruit: Some(fruit), ..
                }),
                Role::Fruit(n),
            ) => Some(Harvest {
                to: fruit.stages[fruit.harvest_to].material,
                ripe: n + 1 == fruit.stages.len(),
                yields: &fruit.stages[n].yields,
            }),
            _ => None,
        }
    }

    /// What removing a Voxel of `material` drops, or nothing.
    pub fn yields(&self, material: MaterialId) -> &[Yield] {
        let Some(PlantRef { plant, role }) = self.of(material) else {
            return &[];
        };
        let Some(plant) = self.plants.get(plant) else {
            return &[];
        };
        match (&plant.kind, role) {
            (Kind::Crop { stages, .. }, Role::Stage(n)) => &stages[n].yields,
            (Kind::Tree(_), Role::Sapling) => &plant.sapling_yields,
            (Kind::Tree(tree), Role::Trunk) => &tree.trunk_yield,
            (Kind::Tree(tree), Role::Canopy) => &tree.canopy_yield,
            (
                Kind::Tree(Tree {
                    fruit: Some(fruit), ..
                }),
                Role::Fruit(n),
            ) => &fruit.stages[n].yields,
            (Kind::Column { yields, .. }, _)
            | (Kind::Spreader { yields, .. }, _)
            | (Kind::Climber { yields, .. }, _) => yields,
            _ => &[],
        }
    }

    /// The canopy a removed fruit Voxel leaves in its place.
    pub fn canopy_of_fruit(&self, material: MaterialId) -> Option<MaterialId> {
        let PlantRef { plant, role } = self.of(material)?;
        match (&self.plants.get(plant)?.kind, role) {
            (Kind::Tree(tree), Role::Fruit(_)) => Some(tree.canopy),
            _ => None,
        }
    }

    /// What a Player's Place of `voxel` puts down when it is a Plant's seed
    /// with a declared Shape, struck on `struck_face` of the Voxel it grows
    /// from; any other Voxel comes back as it is. The Player's Shape and
    /// Orientation are ignored. A `plane` Shape is laid against that Voxel, so
    /// a climber holds the face it was placed on.
    pub fn placed(&self, shapes: &ShapeTable, voxel: Voxel, struck_face: usize) -> Voxel {
        let material = voxel.material();
        let Some(PlantRef { plant, role }) = self.of(material) else {
            return voxel;
        };
        let Some(shape) = self.plants.get(plant).and_then(|p| p.shape) else {
            return voxel;
        };
        if !matches!(role, Role::Stage(0) | Role::Single | Role::Sapling) {
            return voxel;
        }
        let orientation = shapes
            .orientation_holding(material, shape, struck_face ^ 1)
            .unwrap_or_default();
        Voxel::shaped(material, shape, orientation)
    }

    /// What worldgen hangs for `vines`: the named climber's Material and its `plane` Shape, with
    /// the Orientation that lays it against each side. The reason is an error message naming what
    /// is wrong with the climber.
    pub fn vines(
        &self,
        shapes: &ShapeTable,
        vines: &TreeVines,
    ) -> Result<crate::SurfaceVines, String> {
        let plant = self
            .plants
            .iter()
            .find(|p| p.name == vines.plant)
            .ok_or_else(|| format!("vines name Plant `{}`, which is not declared", vines.plant))?;
        if !matches!(plant.kind, Kind::Climber { .. }) {
            return Err(format!(
                "vines name `{}`, which is not a climber",
                vines.plant
            ));
        }
        let shape = plant.shape.ok_or_else(|| {
            format!(
                "vines name `{}`, which has no plane Shape to hang by",
                vines.plant
            )
        })?;
        let mut orientations = [0u8; 4];
        for (slot, face) in orientations.iter_mut().zip(crate::VINE_FACES) {
            let held = shapes.orientation_holding(plant.first, shape, face);
            *slot = held
                .ok_or_else(|| format!("vines name `{}`, whose Shape is not a plane", vines.plant))?
                .bits();
        }
        Ok(crate::SurfaceVines {
            material: plant.first,
            shape,
            orientations,
            density: vines.density,
            length_m: vines.length_m,
        })
    }

    /// The Shape a Player's Place of `material`, a Plant's seed, always uses.
    pub fn fixed_shape(&self, material: MaterialId) -> Option<ShapeId> {
        let PlantRef { plant, role } = self.of(material)?;
        matches!(role, Role::Stage(0) | Role::Single | Role::Sapling)
            .then(|| self.plants.get(plant)?.shape)
            .flatten()
    }

    /// The first Stage of the chain `material` is in, so undo and redo read
    /// every Stage of one crop, or of one tree's fruit, as the same Voxel. A
    /// tree's trunk is not in its sapling's chain.
    pub fn canonical(&self, material: MaterialId) -> MaterialId {
        let Some(PlantRef { plant, role }) = self.of(material) else {
            return material;
        };
        match (self.plants.get(plant).map(|p| &p.kind), role) {
            (Some(Kind::Crop { stages, .. }), Role::Stage(_)) => stages[0].material,
            (
                Some(Kind::Tree(Tree {
                    fruit: Some(fruit), ..
                })),
                Role::Fruit(_),
            ) => fruit.stages[0].material,
            _ => material,
        }
    }

    /// The Plant whose soil rule a Voxel of `material` at `at` fails, or `None`
    /// when it holds, needs none, or rests on terrain `occupants` does not know:
    /// an unheld Chunk is never read as Air.
    ///
    /// A crop, sapling or spreader needs soil in the Voxel below, a downward
    /// column in the Voxel above, and a column also stands on its own Material
    /// across that face. A climber needs soil in any one of its six
    /// neighbours. Light, Climate and water are growth needs, never this.
    pub fn lacks_soil(
        &self,
        material: MaterialId,
        at: [i32; 3],
        occupants: &dyn Fn([i32; 3]) -> Option<Occupants>,
    ) -> Option<&Plant> {
        let PlantRef { plant, role } = self.of(material)?;
        let plant = self.plants.get(plant)?;
        let soil = &plant.needs.soil;
        let on = |cell: [i32; 3], own: bool| {
            occupants(cell).map(|o| {
                let m = o.solid.material();
                soil.contains(&m) || (own && m == material)
            })
        };
        let held = match (&plant.kind, role) {
            (_, Role::Trunk | Role::Canopy | Role::Fruit(_)) => return None,
            (Kind::Climber { .. }, _) => {
                let sides = FACES.map(|d| on([at[0] + d[0], at[1] + d[1], at[2] + d[2]], false));
                held_any(
                    sides
                        .into_iter()
                        .chain([on([at[0], at[1] + 1, at[2]], true)]),
                )
            }
            (
                Kind::Column {
                    direction: Direction::Down,
                    ..
                },
                _,
            ) => on([at[0], at[1] + 1, at[2]], true),
            (Kind::Column { .. }, _) => on([at[0], at[1] - 1, at[2]], true),
            _ => on([at[0], at[1] - 1, at[2]], false),
        };
        (held == Some(false)).then_some(plant)
    }

    /// [`PlantTable::lacks_soil`] for the Voxel as it stands: a climber whose
    /// Shape is a `plane` needs soil on the one face its Orientation names, or
    /// another of its Material above it to hang from.
    pub fn lacks_support(
        &self,
        shapes: &ShapeTable,
        voxel: Voxel,
        at: [i32; 3],
        occupants: &dyn Fn([i32; 3]) -> Option<Occupants>,
    ) -> Option<&Plant> {
        let material = voxel.material();
        let PlantRef { plant, role } = self.of(material)?;
        let plant = self.plants.get(plant)?;
        if let (Kind::Climber { .. }, Role::Single, Some(face)) =
            (&plant.kind, role, shapes.plane_face(voxel))
        {
            let step = crate::mesh::FACE_STEPS[face];
            let held = |cell: [i32; 3], own: bool| {
                occupants(cell).map(|o| {
                    let m = o.solid.material();
                    (!own && plant.needs.soil.contains(&m)) || (own && m == material)
                })
            };
            let face_cell = [at[0] + step[0], at[1] + step[1], at[2] + step[2]];
            let held = held_any([
                held(face_cell, false),
                held([at[0], at[1] + 1, at[2]], true),
            ]);
            return (held == Some(false)).then_some(plant);
        }
        self.lacks_soil(material, at, occupants)
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

/// Whether any of several supports holds: true if one does, unknown if none does
/// and one is unknown, false when every one is known not to.
fn held_any(held: impl IntoIterator<Item = Option<bool>>) -> Option<bool> {
    let mut unknown = false;
    for h in held {
        match h {
            Some(true) => return Some(true),
            None => unknown = true,
            Some(false) => {}
        }
    }
    (!unknown).then_some(false)
}

const FACES: [[i32; 3]; 6] = [
    [1, 0, 0],
    [-1, 0, 0],
    [0, 1, 0],
    [0, -1, 0],
    [0, 0, 1],
    [0, 0, -1],
];

/// What Use does to a ripening Voxel: where it resets, whether it is ripe, and
/// what the pick gives.
#[derive(Clone, Copy, Debug)]
pub struct Harvest<'a> {
    pub to: MaterialId,
    pub ripe: bool,
    pub yields: &'a [Yield],
}

/// How many of a yield one Voxel gives: a hash of the World seed, the Voxel's
/// position, its Material (the Stage) and the yield's place in the list, so a
/// drop never advances a generator and a replay agrees.
pub fn drop_count(
    seed: u64,
    at: [i32; 3],
    material: MaterialId,
    slot: usize,
    count: CountRange,
) -> u32 {
    let h = [
        at[0] as u32 as u64,
        at[1] as u32 as u64,
        at[2] as u32 as u64,
        u64::from(material),
        slot as u64,
    ]
    .into_iter()
    .fold(mix(seed ^ 0x6a09_e667_f3bc_c908), |h, v| {
        mix(h ^ v.wrapping_mul(0x9e37_79b9_7f4a_7c15))
    });
    count.min + (h % (u64::from(count.max) - u64::from(count.min) + 1)) as u32
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
    if let Some(water) = &needs.water
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
            if let Some(vines) = &tree.vines
                && !((0.0..=crate::MAX_VINE_DENSITY).contains(&vines.density)
                    && vines.length_m > 0.0
                    && vines.length_m <= crate::MAX_VINE_LENGTH_M)
            {
                return Err(PlantError::Size(
                    "vines density is from 0 to 0.5 and length-m above 0 and at most 8",
                ));
            }
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

/// One position a Chunk's Sample drew, and the roll its Voxel makes if it holds
/// a Plant.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Sample {
    /// The Voxel's offset from the Chunk's corner.
    pub local: [usize; 3],
    /// Uniform in `[0, 1)`; a roll passes when it is below the Voxel's chance.
    pub roll: f32,
}

fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// The positions and rolls a Chunk draws at one Logical step: a pure function of
/// the World seed, the Chunk, the step and the index, so equal inputs give equal
/// draws however the work is divided. It never reads the World.
pub fn samples(seed: u64, chunk: [i32; 3], step: u64) -> impl Iterator<Item = Sample> {
    let key = [
        chunk[0] as u32 as u64,
        chunk[1] as u32 as u64,
        chunk[2] as u32 as u64,
        step,
    ]
    .into_iter()
    .fold(mix(seed ^ 0x91a7_7e5a_3b1d_0001), |h, v| {
        mix(h ^ v.wrapping_mul(0x9e37_79b9_7f4a_7c15))
    });
    (0..PLANT_SAMPLES as u64).map(move |i| {
        let h = mix(key ^ i.wrapping_mul(0x9e37_79b9_7f4a_7c15));
        let p = (h as usize) % CHUNK_VOLUME;
        Sample {
            local: [
                p % CHUNK_SIZE,
                (p / CHUNK_SIZE) % CHUNK_SIZE,
                p / (CHUNK_SIZE * CHUNK_SIZE),
            ],
            roll: (h >> 40) as f32 / (1u64 << 24) as f32,
        }
    })
}

/// What a growth rule reads, as the Server holds it. `None` is terrain that is
/// not held or not lit yet: never read as Air, so the Voxel holds its Stage.
pub trait Surroundings {
    fn occupants(&self, at: [i32; 3]) -> Option<Occupants>;
    fn light(&self, at: [i32; 3]) -> Option<Light>;
    /// The World's Climate at a column, in metres.
    fn climate_at(&self, x_m: f32, z_m: f32) -> Option<Climate>;
}

/// The chance one Sample of a Voxel advances a Stage that takes `mean_seconds`.
pub fn roll_chance(mean_seconds: f32) -> f32 {
    (SAMPLE_INTERVAL_S / mean_seconds).min(1.0)
}

/// What a growth rule reads besides the World's cells.
pub struct GrowEnv<'a> {
    pub plants: &'a PlantTable,
    pub shapes: &'a ShapeTable,
    pub seed: u64,
    /// Converts every metre a Plant declares to Voxels.
    pub voxel_size: f32,
}

/// One Voxel a growth sets.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Change {
    pub at: [i32; 3],
    pub voxel: Voxel,
}

/// What one passing Sample does to the World: every change lands together or
/// not at all.
#[derive(Clone, PartialEq, Debug)]
pub struct Growth {
    pub changes: Vec<Change>,
    /// A whole tree, which lands under [`TREES_PER_STEP`] rather than
    /// [`PLANTS_LANDED_PER_STEP`].
    pub tree: bool,
}

impl Growth {
    pub fn one(at: [i32; 3], voxel: Voxel) -> Self {
        Growth {
            changes: vec![Change { at, voxel }],
            tree: false,
        }
    }
}

/// What the Plant Voxel `voxel` at `at` does if this Sample's `roll` passes at
/// Logical `step` and every need holds; `None` holds it as it is. A crop swaps
/// its own Voxel for the next Stage. A column, spreader or climber leaves its
/// own Voxel and adds one beside it, with the same Shape, so the Mod's Shape
/// for the Plant is kept; `step` only varies which target a Sample picks.
pub fn grow(
    env: &GrowEnv,
    view: &impl Surroundings,
    step: u64,
    at: [i32; 3],
    voxel: Voxel,
    roll: f32,
) -> Option<Growth> {
    let material = voxel.material();
    let PlantRef { plant: index, role } = env.plants.of(material)?;
    let plant = env.plants.plant(index)?;
    let mean = match (&plant.kind, role) {
        (Kind::Crop { stages, .. }, Role::Stage(n)) => stages.get(n)?.mean_seconds?,
        (Kind::Column { mean_seconds, .. }, Role::Single)
        | (Kind::Spreader { mean_seconds, .. }, Role::Single)
        | (Kind::Climber { mean_seconds, .. }, Role::Single) => *mean_seconds,
        (Kind::Tree(tree), Role::Sapling) => tree.mean_seconds,
        (
            Kind::Tree(Tree {
                fruit: Some(fruit), ..
            }),
            Role::Fruit(n),
        ) => fruit.stages.get(n)?.mean_seconds?,
        _ => return None,
    };
    if roll >= roll_chance(mean) {
        return None;
    }
    let size = env.voxel_size;
    match &plant.kind {
        Kind::Crop { .. } => {
            let next = env.plants.next_stage(material)?;
            needs_met(&plant.needs, view, size, at)
                .then(|| Growth::one(at, Voxel::new(next, voxel.shape(), voxel.orientation())))
        }
        Kind::Column {
            max_height_m,
            direction,
            stands_in,
            ..
        } => reach::column(
            plant,
            view,
            size,
            at,
            voxel,
            reach::Column {
                max_height_m: *max_height_m,
                direction: *direction,
                stands_in,
            },
        ),
        Kind::Spreader {
            reach_m, density, ..
        } => reach::spread(env, plant, view, step, at, voxel, *reach_m, *density),
        Kind::Climber { length_m, .. } => {
            reach::climb(env, plant, view, step, at, voxel, *length_m)
        }
        Kind::Tree(tree) => match role {
            Role::Fruit(n) => tree::ripen(env, plant, tree, view, at, voxel, n),
            _ => tree::sprout(env, index, plant, tree, view, at),
        },
    }
}

/// Whether soil, light, Climate and water all hold for a Plant at `at`.
pub fn needs_met(needs: &Needs, view: &impl Surroundings, voxel_size: f32, at: [i32; 3]) -> bool {
    let Some(below) = view.occupants([at[0], at[1] - 1, at[2]]) else {
        return false;
    };
    needs.soil.contains(&below.solid.material()) && conditions_met(needs, view, voxel_size, at)
}

/// [`needs_met`] without the soil: light, Climate and water, for a Plant that
/// stands on something other than the ground below its Voxel.
pub fn conditions_met(
    needs: &Needs,
    view: &impl Surroundings,
    voxel_size: f32,
    at: [i32; 3],
) -> bool {
    let Some(light) = view.light(at) else {
        return false;
    };
    let level = light.sky().max(light.block());
    if level < needs.light_min || level > needs.light_max {
        return false;
    }
    if needs.temperature.is_some() || needs.humidity.is_some() {
        let Some(climate) = view.climate_at(at[0] as f32 * voxel_size, at[2] as f32 * voxel_size)
        else {
            return false;
        };
        let within = |range: Option<(f32, f32)>, v: f32| {
            range.is_none_or(|(low, high)| (low..=high).contains(&v))
        };
        if !within(needs.temperature, climate.temperature)
            || !within(needs.humidity, climate.humidity)
        {
            return false;
        }
    }
    needs
        .water
        .as_ref()
        .is_none_or(|w| water_within(w, view, voxel_size, at))
}

/// Whether the fluid lies within `reach_m` of `at`, measured between Voxel
/// centres.
fn water_within(water: &Water, view: &impl Surroundings, voxel_size: f32, at: [i32; 3]) -> bool {
    let reach = (water.reach_m / voxel_size).floor() as i32;
    let limit = (water.reach_m / voxel_size).powi(2);
    for dy in -reach..=reach {
        for dz in -reach..=reach {
            for dx in -reach..=reach {
                if ((dx * dx + dy * dy + dz * dz) as f32) > limit {
                    continue;
                }
                let here = [at[0] + dx, at[1] + dy, at[2] + dz];
                if view
                    .occupants(here)
                    .is_some_and(|o| o.fluid.is_some_and(|f| water.fluids.contains(&f.material)))
                {
                    return true;
                }
            }
        }
    }
    false
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
            shape: None,
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

    fn solid_at(cells: &[([i32; 3], MaterialId)]) -> impl Fn([i32; 3]) -> Option<Occupants> {
        let cells = cells.to_vec();
        move |at| {
            Some(Occupants {
                solid: cells
                    .iter()
                    .find(|(c, _)| *c == at)
                    .map_or(crate::voxel::Voxel::AIR, |&(_, m)| {
                        crate::voxel::Voxel::cube(m)
                    }),
                fluid: None,
                burning: None,
            })
        }
    }

    fn one(kind: Kind, first: MaterialId) -> PlantTable {
        let mut table = PlantTable::default();
        table
            .push(Plant {
                name: "p".into(),
                seed: "t:seed".into(),
                first,
                shape: None,
                needs: Needs {
                    soil: vec![1],
                    light_max: 15,
                    ..Needs::default()
                },
                kind,
                sapling_yields: Vec::new(),
            })
            .unwrap();
        table
    }

    #[test]
    fn a_crop_needs_soil_below_and_an_unheld_voxel_is_never_read_as_air() {
        let table = one(
            Kind::Crop {
                stages: vec![stage(10, Some(60.0)), stage(11, None)],
                harvest_to: None,
            },
            10,
        );
        let at = [4, 4, 4];
        assert!(table.lacks_soil(10, at, &solid_at(&[])).is_some());
        assert!(
            table
                .lacks_soil(11, at, &solid_at(&[([4, 3, 4], 2)]))
                .is_some()
        );
        assert!(
            table
                .lacks_soil(10, at, &solid_at(&[([4, 3, 4], 1)]))
                .is_none()
        );
        assert!(table.lacks_soil(10, at, &|_| None).is_none());
        assert!(
            table.lacks_soil(1, at, &solid_at(&[])).is_none(),
            "no plant"
        );
    }

    #[test]
    fn a_column_stands_on_soil_or_itself_and_a_downward_one_hangs_from_above() {
        let up = |direction| {
            one(
                Kind::Column {
                    max_height_m: 4.0,
                    direction,
                    mean_seconds: 60.0,
                    stands_in: Vec::new(),
                    yields: Vec::new(),
                },
                10,
            )
        };
        let table = up(Direction::Up);
        let at = [4, 4, 4];
        assert!(
            table
                .lacks_soil(10, at, &solid_at(&[([4, 3, 4], 10)]))
                .is_none()
        );
        assert!(
            table
                .lacks_soil(10, at, &solid_at(&[([4, 5, 4], 1)]))
                .is_some()
        );
        let table = up(Direction::Down);
        assert!(
            table
                .lacks_soil(10, at, &solid_at(&[([4, 5, 4], 1)]))
                .is_none()
        );
        assert!(
            table
                .lacks_soil(10, at, &solid_at(&[([4, 3, 4], 1)]))
                .is_some()
        );
    }

    #[test]
    fn a_climber_needs_soil_in_any_one_neighbour() {
        let table = one(
            Kind::Climber {
                mean_seconds: 60.0,
                length_m: 4.0,
                yields: Vec::new(),
            },
            10,
        );
        let at = [4, 4, 4];
        for side in FACES {
            let cell = [at[0] + side[0], at[1] + side[1], at[2] + side[2]];
            assert!(table.lacks_soil(10, at, &solid_at(&[(cell, 1)])).is_none());
        }
        assert!(
            table
                .lacks_soil(10, at, &solid_at(&[([4, 3, 4], 2)]))
                .is_some()
        );
    }

    #[test]
    fn a_drop_count_is_a_stable_hash_inside_its_range() {
        let range = CountRange { min: 2, max: 5 };
        let at = [3, -7, 11];
        let first = drop_count(9, at, 12, 0, range);
        assert_eq!(first, drop_count(9, at, 12, 0, range));
        assert!((2..=5).contains(&first));
        let seen: std::collections::HashSet<_> = (0..64)
            .map(|x| drop_count(9, [x, 0, 0], 12, 0, range))
            .collect();
        assert!(seen.iter().all(|c| (2..=5).contains(c)));
        assert!(seen.len() > 1, "varies with the position");
        assert_eq!(drop_count(9, at, 12, 0, CountRange { min: 3, max: 3 }), 3);
    }

    #[test]
    fn harvest_and_yields_follow_the_stage() {
        let mut table = PlantTable::default();
        table
            .push(crop(
                vec![
                    stage(10, Some(60.0)),
                    stage(11, Some(60.0)),
                    Stage {
                        material: 12,
                        mean_seconds: None,
                        yields: vec![Yield {
                            item: "t:wheat".into(),
                            count: CountRange { min: 1, max: 1 },
                        }],
                    },
                ],
                Some(1),
            ))
            .unwrap();
        assert!(!table.harvest(11).unwrap().ripe);
        let ripe = table.harvest(12).unwrap();
        assert_eq!((ripe.ripe, ripe.to, ripe.yields.len()), (true, 11, 1));
        assert!(table.harvest(99).is_none());
        assert_eq!(table.yields(12).len(), 1);
        assert!(table.yields(10).is_empty());
        assert_eq!(table.canonical(12), 10);
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

    use crate::occupants::Fluid;
    use crate::voxel::Voxel;

    const SOIL: MaterialId = 1;
    const WATER: MaterialId = 2;

    struct Scene {
        solids: HashMap<[i32; 3], MaterialId>,
        water: Vec<[i32; 3]>,
        light: Option<u8>,
        climate: Option<Climate>,
    }

    impl Scene {
        fn on_soil() -> Self {
            Scene {
                solids: HashMap::from([([0, 0, 0], SOIL)]),
                water: Vec::new(),
                light: Some(10),
                climate: Some(Climate {
                    temperature: 0.5,
                    humidity: 0.5,
                }),
            }
        }
    }

    impl Surroundings for Scene {
        fn occupants(&self, at: [i32; 3]) -> Option<Occupants> {
            Some(Occupants {
                solid: self.solids.get(&at).map_or(Voxel::AIR, |&m| Voxel::cube(m)),
                fluid: self.water.contains(&at).then(|| Fluid::source(WATER)),
                burning: None,
            })
        }

        fn light(&self, _: [i32; 3]) -> Option<Light> {
            self.light.map(|l| Light::new(l, 0))
        }

        fn climate_at(&self, _: f32, _: f32) -> Option<Climate> {
            self.climate
        }
    }

    fn wheat_table(needs: Needs) -> PlantTable {
        let mut table = PlantTable::default();
        let mut plant = crop(
            vec![
                stage(10, Some(32.0)),
                stage(11, Some(32.0)),
                stage(12, None),
            ],
            None,
        );
        plant.needs = needs;
        table.push(plant).unwrap();
        table
    }

    fn needs() -> Needs {
        Needs {
            soil: vec![SOIL],
            light_min: 8,
            light_max: 15,
            ..Needs::default()
        }
    }

    const ABOVE: [i32; 3] = [0, 1, 0];

    /// The Stage a crop of `material` advances to.
    fn grow(
        table: &PlantTable,
        view: &impl Surroundings,
        voxel_size: f32,
        at: [i32; 3],
        material: MaterialId,
        roll: f32,
    ) -> Option<MaterialId> {
        let env = GrowEnv {
            plants: table,
            shapes: &ShapeTable::builtin(),
            seed: 1,
            voxel_size,
        };
        super::grow(&env, view, 0, at, Voxel::cube(material), roll)
            .map(|g| g.changes[0].voxel.material())
    }

    #[test]
    fn a_crop_stage_change_keeps_the_plants_shape() {
        let table = wheat_table(needs());
        let scene = Scene::on_soil();
        let shapes = ShapeTable::builtin();
        let env = GrowEnv {
            plants: &table,
            shapes: &shapes,
            seed: 1,
            voxel_size: 1.0,
        };
        let cross = Voxel::new(10, 5, 0);
        let grown = super::grow(&env, &scene, 0, ABOVE, cross, 0.0).unwrap();
        assert_eq!(grown.changes[0].voxel, Voxel::new(11, 5, 0));
    }

    #[test]
    fn samples_are_a_pure_function_of_their_inputs() {
        let draw = |seed, chunk, step| samples(seed, chunk, step).collect::<Vec<_>>();
        let a = draw(7, [1, -2, 3], 100);
        assert_eq!(a.len(), PLANT_SAMPLES as usize);
        assert_eq!(a, draw(7, [1, -2, 3], 100));
        for other in [
            draw(8, [1, -2, 3], 100),
            draw(7, [1, -2, 4], 100),
            draw(7, [1, -2, 3], 101),
        ] {
            assert_ne!(a, other);
        }
        assert!(a.iter().all(|s| s.local.iter().all(|&c| c < CHUNK_SIZE)));
        assert!(a.iter().all(|s| (0.0..1.0).contains(&s.roll)));
    }

    #[test]
    fn samples_spread_over_the_chunk_and_the_rolls_over_the_unit_interval() {
        let drawn: Vec<Sample> = (0..2000).flat_map(|t| samples(3, [0, 0, 0], t)).collect();
        let mean = drawn.iter().map(|s| s.roll).sum::<f32>() / drawn.len() as f32;
        assert!((mean - 0.5).abs() < 0.02, "mean roll {mean}");
        for axis in 0..3 {
            let high = drawn
                .iter()
                .filter(|s| s.local[axis] >= CHUNK_SIZE / 2)
                .count();
            let share = high as f32 / drawn.len() as f32;
            assert!((share - 0.5).abs() < 0.02, "axis {axis} share {share}");
        }
    }

    #[test]
    fn a_roll_below_the_chance_advances_the_stage() {
        let table = wheat_table(needs());
        let scene = Scene::on_soil();
        let chance = roll_chance(32.0);
        assert_eq!(chance, 0.5);
        assert_eq!(
            grow(&table, &scene, 1.0, ABOVE, 10, chance - 0.01),
            Some(11)
        );
        assert_eq!(grow(&table, &scene, 1.0, ABOVE, 10, chance), None);
        assert_eq!(grow(&table, &scene, 1.0, ABOVE, 11, 0.0), Some(12));
        assert_eq!(grow(&table, &scene, 1.0, ABOVE, 12, 0.0), None);
        assert_eq!(grow(&table, &scene, 1.0, ABOVE, 99, 0.0), None);
    }

    #[test]
    fn a_mean_at_the_interval_always_rolls_through() {
        assert_eq!(roll_chance(SAMPLE_INTERVAL_S), 1.0);
        assert_eq!(roll_chance(SAMPLE_INTERVAL_S * 4.0), 0.25);
    }

    #[test]
    fn wrong_soil_or_none_holds_the_stage() {
        let table = wheat_table(needs());
        let mut scene = Scene::on_soil();
        scene.solids.insert([0, 0, 0], 5);
        assert_eq!(grow(&table, &scene, 1.0, ABOVE, 10, 0.0), None);
        scene.solids.clear();
        assert_eq!(grow(&table, &scene, 1.0, ABOVE, 10, 0.0), None);
    }

    #[test]
    fn light_outside_its_band_holds_the_stage() {
        let table = wheat_table(needs());
        for (level, grows) in [(7, false), (8, true), (15, true)] {
            let mut scene = Scene::on_soil();
            scene.light = Some(level);
            assert_eq!(grow(&table, &scene, 1.0, ABOVE, 10, 0.0).is_some(), grows);
        }
        let mut scene = Scene::on_soil();
        scene.light = None;
        assert_eq!(grow(&table, &scene, 1.0, ABOVE, 10, 0.0), None);
    }

    #[test]
    fn climate_outside_its_range_holds_the_stage() {
        let table = wheat_table(Needs {
            temperature: Some((0.4, 0.6)),
            humidity: Some((0.0, 0.3)),
            ..needs()
        });
        let mut scene = Scene::on_soil();
        assert_eq!(grow(&table, &scene, 1.0, ABOVE, 10, 0.0), None);
        scene.climate = Some(Climate {
            temperature: 0.5,
            humidity: 0.2,
        });
        assert_eq!(grow(&table, &scene, 1.0, ABOVE, 10, 0.0), Some(11));
        scene.climate = None;
        assert_eq!(grow(&table, &scene, 1.0, ABOVE, 10, 0.0), None);
    }

    #[test]
    fn water_reach_is_in_metres_at_every_voxel_size() {
        let table = wheat_table(Needs {
            water: Some(Water {
                fluids: vec![WATER],
                reach_m: 2.0,
            }),
            ..needs()
        });
        for size in [1.0f32, 0.5, 0.25] {
            let voxels = (2.0 / size) as i32;
            let mut scene = Scene::on_soil();
            assert_eq!(grow(&table, &scene, size, ABOVE, 10, 0.0), None);
            scene.water = vec![[ABOVE[0] + voxels, ABOVE[1], ABOVE[2]]];
            assert_eq!(
                grow(&table, &scene, size, ABOVE, 10, 0.0),
                Some(11),
                "{size}"
            );
            scene.water = vec![[ABOVE[0] + voxels + 1, ABOVE[1], ABOVE[2]]];
            assert_eq!(grow(&table, &scene, size, ABOVE, 10, 0.0), None, "{size}");
        }
    }
}
