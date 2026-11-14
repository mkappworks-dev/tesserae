//! The parametric tree and the Felling region: pure functions of a Plant row,
//! a position and the Voxel size, so worldgen and the Server place and cut the
//! same Voxels from any Chunk's viewpoint.

use std::collections::HashSet;

use super::reach::voxels_in;
use super::{
    Branching, Change, FORK_SPREAD, GrowEnv, Growth, Kind, MAX_CANOPY_RADIUS_M, Plant, PlantRef,
    PlantTable, Role, Surroundings, Tree, conditions_met, mix,
};
use crate::climate::{SurfaceVines, VINE_FACES};
use crate::light::Light;
use crate::mesh::FACE_STEPS;
use crate::voxel::{AIR, MaterialId, SHAPE_FULL_CUBE, ShapeId, Voxel};

/// Which of a tree's grown Materials a Voxel is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Part {
    Trunk,
    Canopy,
    /// A canopy Voxel that carries fruit, at its first Stage.
    Fruit,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TreeVoxel {
    pub at: [i32; 3],
    pub material: MaterialId,
    pub part: Part,
    /// Fruit takes the fruit's Shape; trunk and canopy are cubes.
    pub shape: ShapeId,
    pub orientation: u8,
}

impl TreeVoxel {
    /// The Voxel this cell holds.
    pub fn voxel(&self) -> Voxel {
        Voxel::new(self.material, self.shape, self.orientation)
    }
}

const FACES: [[i32; 3]; 6] = [
    [1, 0, 0],
    [-1, 0, 0],
    [0, 1, 0],
    [0, -1, 0],
    [0, 0, 1],
    [0, 0, -1],
];

/// The canopy is flatter than it is wide.
const CANOPY_HEIGHT_RATIO: f32 = 0.8;

/// A hash of the World seed and a position in millimetres, so the same metres
/// hash alike at every Voxel size and from every Chunk.
fn hash(seed: u64, at: [i32; 3], voxel_size: f32, salt: u64) -> u64 {
    at.map(|c| (c as f32 * voxel_size * 1000.0).round() as i32 as u32 as u64)
        .into_iter()
        .chain([salt])
        .fold(mix(seed ^ 0x5851_f42d_4c95_7f2d), |h, v| {
            mix(h ^ v.wrapping_mul(0x9e37_79b9_7f4a_7c15))
        })
}

/// Voxels in the trunk of the tree whose sapling stands at `base`: a hash
/// picks a whole number of Voxels inside `trunk-height-m`.
pub fn trunk_height(tree: &Tree, seed: u64, base: [i32; 3], voxel_size: f32) -> i32 {
    let (low, high) = tree.trunk_height_m;
    let fewest = ((low / voxel_size) - 1e-3).ceil().max(1.0) as i32;
    let most = (((high / voxel_size) + 1e-3).floor() as i32).max(fewest);
    fewest + (hash(seed, base, voxel_size, 1) % (most - fewest + 1) as u64) as i32
}

/// Whether the canopy Voxel at `at` carries fruit: a share `density` of them,
/// by hash of seed and position.
pub fn bears_fruit(seed: u64, density: f32, at: [i32; 3], voxel_size: f32) -> bool {
    let share = (hash(seed, at, voxel_size, 2) >> 11) as f64 / (1u64 << 53) as f64;
    share < f64::from(density)
}

/// Which of `stages` fruit Stage the fruit Voxel at `at` is generated at, by hash of seed and
/// position.
pub fn fruit_stage(seed: u64, at: [i32; 3], voxel_size: f32, stages: usize) -> usize {
    (hash(seed, at, voxel_size, 3) % stages.max(1) as u64) as usize
}

/// Every Voxel of the tree whose sapling stands at `base`: the trunk from the
/// sapling's own cell upward, then the canopy, some of it fruit. A tree with no `branching` is a
/// straight trunk under one ellipsoid centred on its top; one with it grows limbs, leaders and
/// lobes too. A pure function of the row, the position and the Voxel size.
pub fn tree_voxels(tree: &Tree, seed: u64, base: [i32; 3], voxel_size: f32) -> Vec<TreeVoxel> {
    let height = trunk_height(tree, seed, base, voxel_size);
    if let Some(b) = &tree.branching {
        return branched(tree, b, seed, base, voxel_size, height);
    }
    let mut voxels: Vec<TreeVoxel> = (0..height)
        .map(|dy| wood([base[0], base[1] + dy, base[2]], tree.trunk))
        .collect();
    let top = [base[0], base[1] + height - 1, base[2]];
    // The canopy leaves the lower trunk bare, and never reaches the ground.
    let lowest = base[1] + (height / 2).max(1);
    ellipsoid(top, tree.canopy_radius_m, voxel_size, |at, [dx, dy, dz]| {
        if at[1] >= lowest && !(dx == 0 && dz == 0 && dy <= 0) {
            voxels.push(leaf(tree, seed, at, voxel_size));
        }
    });
    voxels
}

fn wood(at: [i32; 3], material: MaterialId) -> TreeVoxel {
    TreeVoxel {
        at,
        material,
        part: Part::Trunk,
        shape: SHAPE_FULL_CUBE,
        orientation: 0,
    }
}

/// The canopy Voxel at `at`, or the fruit that replaces it.
fn leaf(tree: &Tree, seed: u64, at: [i32; 3], voxel_size: f32) -> TreeVoxel {
    let fruit = tree
        .fruit
        .as_ref()
        .filter(|f| bears_fruit(seed, f.density, at, voxel_size));
    match fruit {
        Some(f) => TreeVoxel {
            at,
            material: f.stages[0].material,
            part: Part::Fruit,
            shape: f.shape.unwrap_or(SHAPE_FULL_CUBE),
            orientation: 0,
        },
        None => TreeVoxel {
            at,
            material: tree.canopy,
            part: Part::Canopy,
            shape: SHAPE_FULL_CUBE,
            orientation: 0,
        },
    }
}

/// Calls `f` with each cell inside the ellipsoid of horizontal radius `radius_m` centred on
/// `centre`, and its offset from the centre, x then z then y. It is flatter than it is wide.
fn ellipsoid(
    centre: [i32; 3],
    radius_m: f32,
    voxel_size: f32,
    mut f: impl FnMut([i32; 3], [i32; 3]),
) {
    let wide = (radius_m / voxel_size).max(1.0);
    let tall = (radius_m * CANOPY_HEIGHT_RATIO / voxel_size).max(1.0);
    let (rx, ry) = (wide.ceil() as i32, tall.ceil() as i32);
    for dx in -rx..=rx {
        for dz in -rx..=rx {
            for dy in -ry..=ry {
                let inside =
                    (dx * dx + dz * dz) as f32 / (wide * wide) + (dy * dy) as f32 / (tall * tall);
                if inside <= 1.0 + 1e-4 {
                    f(
                        [centre[0] + dx, centre[1] + dy, centre[2] + dz],
                        [dx, dy, dz],
                    );
                }
            }
        }
    }
}

/// Sixteen horizontal directions as literals: a trigonometric call may differ in its last bit
/// between platforms, and one bit can move a limb's tip a Voxel.
const COMPASS: [[f32; 2]; 16] = [
    [1.0, 0.0],
    [0.923_879_5, 0.382_683_43],
    [0.707_106_77, 0.707_106_77],
    [0.382_683_43, 0.923_879_5],
    [0.0, 1.0],
    [-0.382_683_43, 0.923_879_5],
    [-0.707_106_77, 0.707_106_77],
    [-0.923_879_5, 0.382_683_43],
    [-1.0, 0.0],
    [-0.923_879_5, -0.382_683_43],
    [-0.707_106_77, -0.707_106_77],
    [-0.382_683_43, -0.923_879_5],
    [0.0, -1.0],
    [0.382_683_43, -0.923_879_5],
    [0.707_106_77, -0.707_106_77],
    [0.923_879_5, -0.382_683_43],
];

/// Where the limbs stop short of the trunk's top, as a fraction of its height.
const LIMBS_END: f32 = 0.9;
/// A tier's reach at the top of the tree, as a fraction of the bottom tier's.
const TIER_TOP_REACH: f32 = 0.25;
const TIER_LIMBS: u32 = 4;
/// A leaders' count is two or three.
const LEADERS: u64 = 2;

/// The cells of a face-connected line from `a` to `b`, both included: each step moves one axis,
/// the one whose next cell boundary the straight line crosses first, so no two cells touch only
/// along an edge and a limb is one piece for Felling.
fn line(a: [i32; 3], b: [i32; 3]) -> Vec<[i32; 3]> {
    let d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let len = d.map(|v| i64::from(v.abs()));
    let mut moved = [0i64; 3];
    let mut at = a;
    let mut cells = vec![a];
    for _ in 0..len.iter().sum::<i64>() {
        let mut axis = None;
        for i in 0..3 {
            if len[i] == 0 {
                continue;
            }
            let sooner = axis
                .is_none_or(|j: usize| (2 * moved[i] + 1) * len[j] < (2 * moved[j] + 1) * len[i]);
            if sooner {
                axis = Some(i);
            }
        }
        let i = axis.expect("a step remains");
        moved[i] += 1;
        at[i] += d[i].signum();
        cells.push(at);
    }
    cells
}

/// The cells a tree has taken, as a flat box over where it can stand: a lobe is hundreds of
/// cells and a tree has several, so a hashed set costs more than the cells themselves.
/// A cell outside the box goes to a set, so a wrong bound is slow and never wrong.
struct Taken {
    low: [i32; 3],
    size: [i32; 3],
    cells: Vec<bool>,
    outside: HashSet<[i32; 3]>,
}

impl Taken {
    fn new(low: [i32; 3], size: [i32; 3]) -> Self {
        Self {
            low,
            size,
            cells: vec![false; size.iter().product::<i32>() as usize],
            outside: HashSet::new(),
        }
    }

    /// Whether `at` was free.
    fn insert(&mut self, at: [i32; 3]) -> bool {
        let d = [
            at[0] - self.low[0],
            at[1] - self.low[1],
            at[2] - self.low[2],
        ];
        if (0..3).any(|i| !(0..self.size[i]).contains(&d[i])) {
            return self.outside.insert(at);
        }
        let slot = &mut self.cells[((d[1] * self.size[2] + d[2]) * self.size[0] + d[0]) as usize];
        !std::mem::replace(slot, true)
    }
}

/// A trunk that leans, forks and carries limbs, with a foliage lobe on every tip and the crown
/// on top. Each draw is a hash of the seed and the base in millimetres, each length is metres
/// over the Voxel size, so the tree is the same shape at every size.
fn branched(
    tree: &Tree,
    b: &Branching,
    seed: u64,
    base: [i32; 3],
    size: f32,
    height: i32,
) -> Vec<TreeVoxel> {
    let roll = |salt: u64| hash(seed, base, size, salt);
    let top_y = height - 1;
    let lean = COMPASS[(roll(7) % 16) as usize];
    let lean_voxels = b.lean_m / size;
    let axis = |dy: i32| -> [i32; 3] {
        let t = if top_y > 0 {
            dy as f32 / top_y as f32
        } else {
            0.0
        };
        [
            base[0] + (lean[0] * lean_voxels * t).round() as i32,
            base[1] + dy,
            base[2] + (lean[1] * lean_voxels * t).round() as i32,
        ]
    };
    let fork_y = b
        .fork
        .filter(|_| top_y >= 3)
        .map(|f| ((f * top_y as f32).round() as i32).clamp(1, top_y - 1));

    let trunk = line(axis(0), axis(fork_y.unwrap_or(top_y)));
    let mut leaders: Vec<Vec<[i32; 3]>> = Vec::new();
    if let Some(fy) = fork_y {
        let count = LEADERS + roll(8) % 2;
        let spread = b.reach_m * FORK_SPREAD / size;
        let first = roll(9) % 16;
        let crown = axis(top_y);
        for k in 0..count {
            let dir = COMPASS[((first + k * 16 / count) % 16) as usize];
            let tip = [
                crown[0] + (dir[0] * spread).round() as i32,
                crown[1],
                crown[2] + (dir[1] * spread).round() as i32,
            ];
            leaders.push(line(axis(fy), tip));
        }
    }

    let mut limbs: Vec<Vec<[i32; 3]>> = Vec::new();
    let span = b.limbs.max - b.limbs.min + 1;
    let count = b.limbs.min + (roll(10) % u64::from(span)) as u32;
    let lowest = 1.max((b.start * top_y as f32).round() as i32);
    let highest = lowest.max((LIMBS_END * top_y as f32).round() as i32);
    let facing = (roll(11) % 16) as u32;
    let anchor = |y: i32, n: u32| -> [i32; 3] {
        let along = match fork_y {
            Some(fy) if y > fy => &leaders[n as usize % leaders.len()],
            _ => &trunk,
        };
        *along
            .iter()
            .find(|c| c[1] == base[1] + y)
            .unwrap_or_else(|| along.last().expect("a line holds its ends"))
    };
    for i in 0..count {
        let up = if count > 1 {
            i as f32 / (count - 1) as f32
        } else {
            0.5
        };
        let y = lowest + (up * (highest - lowest) as f32).round() as i32;
        let rings = if b.tiers { TIER_LIMBS } else { 1 };
        for j in 0..rings {
            let (dir, reach) = if b.tiers {
                let turn = facing + j * 4 + (i % 2) * 2;
                (turn % 16, b.reach_m * (1.0 - (1.0 - TIER_TOP_REACH) * up))
            } else {
                let jitter = (roll(16 + u64::from(i) * 2) % 2) as u32;
                let long = (roll(17 + u64::from(i) * 2) % 4) as f32 / 3.0;
                (
                    (facing + i * 6 + jitter) % 16,
                    b.reach_m * (0.7 + 0.3 * long),
                )
            };
            let root = anchor(y, i);
            let out = COMPASS[dir as usize];
            let tip_y = (root[1] + (b.rise * reach * 0.5 / size).round() as i32)
                .clamp(base[1] + 1, base[1] + top_y);
            let tip = [
                root[0] + (out[0] * reach / size).round() as i32,
                tip_y,
                root[2] + (out[1] * reach / size).round() as i32,
            ];
            limbs.push(line(root, tip));
        }
    }

    let mut voxels: Vec<TreeVoxel> = Vec::new();
    let around = (tree.reach_m() / size).ceil() as i32 + 5;
    let rise = top_y + (tree.crown_rise_m() / size).ceil() as i32 + 3;
    let mut taken = Taken::new(
        [base[0] - around, base[1], base[2] - around],
        [2 * around + 1, rise + 1, 2 * around + 1],
    );
    for at in trunk
        .iter()
        .chain(leaders.iter().flatten())
        .chain(limbs.iter().flatten())
    {
        if taken.insert(*at) {
            voxels.push(wood(*at, tree.trunk));
        }
    }
    let mut foliage = |centre: [i32; 3], radius_m: f32, lowest: i32| {
        ellipsoid(centre, radius_m, size, |at, _| {
            if at[1] >= lowest && taken.insert(at) {
                voxels.push(leaf(tree, seed, at, size));
            }
        });
    };
    let ground = base[1] + 1;
    for tip in leaders.iter().chain(&limbs).filter_map(|l| l.last()) {
        foliage(*tip, b.lobe_radius_m, ground);
    }
    if b.tiers {
        for limb in &limbs {
            foliage(limb[limb.len() / 2], b.lobe_radius_m, ground);
        }
    }
    foliage(
        axis(top_y),
        tree.canopy_radius_m,
        base[1] + (height / 2).max(1),
    );
    voxels
}

/// One vine as generation hangs it: `length` Voxels straight down from `top`, in the one
/// Orientation that lays its plane against the canopy or trunk beside `top`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Vine {
    pub top: [i32; 3],
    pub length: i32,
    pub orientation: u8,
}

impl Vine {
    pub fn cells(&self) -> impl Iterator<Item = [i32; 3]> + use<> {
        let top = self.top;
        (0..self.length).map(move |dy| [top[0], top[1] - dy, top[2]])
    }
}

/// The vines of the tree whose Voxels are `cells`, standing at `base`: each outer canopy cell
/// (one with a side open to the air) hangs one with probability `vines.density`, by hash of seed
/// and position. The vine takes the open cell beside it, lies against the canopy cell, and hangs
/// down a hash-chosen run of up to `vines.length_m` that stops at the tree's own Voxels. A pure
/// function of the tree, seed, base and Voxel size, so every Chunk sees the same vines; the
/// caller still owns what is under a vine (ground, other trees).
pub fn vine_voxels(
    cells: &[TreeVoxel],
    vines: &SurfaceVines,
    seed: u64,
    voxel_size: f32,
) -> Vec<Vine> {
    let own: HashSet<[i32; 3]> = cells.iter().map(|v| v.at).collect();
    let longest = voxels_in(vines.length_m, voxel_size);
    let mut hung = Vec::new();
    for cell in cells.iter().filter(|c| c.part != Part::Trunk) {
        let share = (hash(seed, cell.at, voxel_size, 4) >> 11) as f64 / (1u64 << 53) as f64;
        if share >= f64::from(vines.density) {
            continue;
        }
        let open: Vec<usize> = (0..VINE_FACES.len())
            .filter(|&i| !own.contains(&beside(cell.at, VINE_FACES[i])))
            .collect();
        if open.is_empty() {
            continue;
        }
        let face = open[(hash(seed, cell.at, voxel_size, 5) % open.len() as u64) as usize];
        let top = beside(cell.at, VINE_FACES[face]);
        let length = 1 + (hash(seed, cell.at, voxel_size, 6) % longest as u64) as i32;
        let run = (0..length)
            .take_while(|&dy| !own.contains(&[top[0], top[1] - dy, top[2]]))
            .count() as i32;
        if run > 0 {
            hung.push(Vine {
                top,
                length: run,
                orientation: vines.orientations[face],
            });
        }
    }
    hung
}

/// The cell a vine takes to lie against `cell` with `face`: across the face's opposite side.
fn beside(cell: [i32; 3], face: usize) -> [i32; 3] {
    let step = FACE_STEPS[face];
    [cell[0] - step[0], cell[1] - step[1], cell[2] - step[2]]
}

/// The Voxel a sapling that has passed its roll grows into: the whole tree as
/// one group, or `None` when any cell holds a solid the tree may not replace,
/// fluid or fire, is unknown, or its Chunk's light is pending. A blocked
/// sapling stays and Samples again. Foliage gives way ([`replaces`]); terrain,
/// placed Voxels, crops and another tree's trunk do not.
pub(super) fn sprout(
    env: &GrowEnv,
    index: usize,
    plant: &Plant,
    tree: &Tree,
    view: &impl Surroundings,
    at: [i32; 3],
) -> Option<Growth> {
    if !conditions_met(&plant.needs, view, env.voxel_size, at) {
        return None;
    }
    let mut changes = Vec::new();
    for v in tree_voxels(tree, env.seed, at, env.voxel_size) {
        let here = view.occupants(v.at)?;
        view.light(v.at)?;
        let clear = here.solid.is_air() && here.fluid.is_none() && here.burning.is_none();
        if !(clear || v.at == at || replaces(env.plants, index, here.solid.material())) {
            return None;
        }
        changes.push(Change {
            at: v.at,
            voxel: v.voxel(),
        });
    }
    Some(Growth {
        changes,
        tree: true,
    })
}

/// Fixed bases whose trees [`expected_canopy_cells`] averages, so limb counts and lean, which
/// hash from the base, are averaged rather than one tree's.
const REFERENCE_BASES: [[i32; 3]; 4] = [[0, 0, 0], [3, 0, 1], [-5, 0, 7], [11, 0, -2]];

/// Leaf and fruit cells of a typical `tree` at `voxel_size`: the mean over [`REFERENCE_BASES`]
/// of the Voxels `tree_voxels` makes that are not trunk. A pure function of the row and the size,
/// so every canopy Voxel of a species agrees on it without knowing its own tree.
pub fn expected_canopy_cells(tree: &Tree, voxel_size: f32) -> u32 {
    let total: usize = REFERENCE_BASES
        .iter()
        .map(|&base| {
            tree_voxels(tree, 0, base, voxel_size)
                .iter()
                .filter(|v| v.part != Part::Trunk)
                .count()
        })
        .sum();
    ((total as f32 / REFERENCE_BASES.len() as f32).round() as u32).max(1)
}

/// How many fruit a tree with `cells` canopy cells holds at most when it re-bears: the density
/// its fruit is generated at, times the cells.
pub fn fruit_cap(density: f32, cells: u32) -> f32 {
    density * cells as f32
}

/// Mean seconds between two Samples of one canopy Voxel's re-bearing roll, `None` for a tree
/// that bears no fruit. Each of a tree's `cells` canopy Voxels is Sampled at the same rate, so
/// giving each a mean of `cells * regrow-seconds` makes the tree as a whole roll one passing
/// Sample about every `regrow-seconds`, whatever the Voxel size.
pub(super) fn canopy_mean_seconds(env: &GrowEnv, index: usize, tree: &Tree) -> Option<f32> {
    let fruit = tree.fruit.as_ref()?;
    let cells = env.plants.canopy_cells(index, env.voxel_size)?;
    Some(fruit.regrow_seconds * cells as f32)
}

/// A canopy Voxel that has passed its roll becomes unripe fruit when the tree's light and Climate
/// hold and the fruit within a tree's span of it is below [`fruit_cap`]. A span is twice the
/// tree's reach, since any two cells of a tree are that near; a same-species tree closer than
/// that shares the cap, which only holds fruit back.
pub(super) fn rebear(
    env: &GrowEnv,
    index: usize,
    plant: &Plant,
    tree: &Tree,
    view: &impl Surroundings,
    at: [i32; 3],
) -> Option<Growth> {
    let fruit = tree.fruit.as_ref()?;
    let size = env.voxel_size;
    if !conditions_met(&plant.needs, &Open(view), size, at) {
        return None;
    }
    let cap = fruit_cap(fruit.density, env.plants.canopy_cells(index, size)?);
    if cap <= 0.0 {
        return None;
    }
    let span = voxels_in(2.0 * tree.reach_m(), size);
    let mut bearing = 0usize;
    for dy in -span..=span {
        for dz in -span..=span {
            for dx in -span..=span {
                if dx * dx + dy * dy + dz * dz > span * span {
                    continue;
                }
                let cell = [at[0] + dx, at[1] + dy, at[2] + dz];
                let Some(held) = view.occupants(cell) else {
                    continue;
                };
                let is_fruit = env
                    .plants
                    .of(held.solid.material())
                    .is_some_and(|r| r.plant == index && matches!(r.role, Role::Fruit(_)));
                if is_fruit {
                    bearing += 1;
                    if bearing as f32 >= cap {
                        return None;
                    }
                }
            }
        }
    }
    let first = fruit.stages[0].material;
    let shape = fruit.shape.unwrap_or(SHAPE_FULL_CUBE);
    Some(Growth::one(at, Voxel::new(first, shape, 0)))
}

/// Whether the tree `index` grows over a Voxel of `material`: its own grown
/// Materials, ground cover and small plants, any tree's leaves and fruit, and
/// a sapling. A crop, which is a Player's work, and a trunk, which holds a
/// tree up, stay. Dense understory and a neighbour's crown would otherwise
/// block a jungle sapling for ever.
fn replaces(plants: &PlantTable, index: usize, material: MaterialId) -> bool {
    let Some(PlantRef { plant, role }) = plants.of(material) else {
        return false;
    };
    match role {
        Role::Trunk => plant == index,
        Role::Canopy | Role::Fruit(_) | Role::Sapling => true,
        Role::Single => plants.plant(plant).is_some_and(|p| {
            matches!(
                p.kind,
                Kind::Column { .. } | Kind::Spreader { .. } | Kind::Climber { .. }
            )
        }),
        Role::Stage(_) => false,
    }
}

/// A fruit Voxel that has passed its roll moves to its next Stage when the
/// tree's light and Climate hold. Light is the brightest of the Voxel and its
/// neighbours, since fruit is embedded in canopy.
pub(super) fn ripen(
    env: &GrowEnv,
    plant: &Plant,
    tree: &Tree,
    view: &impl Surroundings,
    at: [i32; 3],
    voxel: Voxel,
    stage: usize,
) -> Option<Growth> {
    let next = tree.fruit.as_ref()?.stages.get(stage + 1)?.material;
    conditions_met(&plant.needs, &Open(view), env.voxel_size, at)
        .then(|| Growth::one(at, Voxel::new(next, voxel.shape(), voxel.orientation())))
}

struct Open<'a, V>(&'a V);

impl<V: Surroundings> Surroundings for Open<'_, V> {
    fn occupants(&self, at: [i32; 3]) -> Option<crate::occupants::Occupants> {
        self.0.occupants(at)
    }

    fn light(&self, at: [i32; 3]) -> Option<Light> {
        let mut best = self.0.light(at)?;
        for d in FACES {
            if let Some(l) = self.0.light([at[0] + d[0], at[1] + d[1], at[2] + d[2]]) {
                best = Light::new(best.sky().max(l.sky()), best.block().max(l.block()));
            }
        }
        Some(best)
    }

    fn climate_at(&self, x_m: f32, z_m: f32) -> Option<crate::climate::Climate> {
        self.0.climate_at(x_m, z_m)
    }
}

fn is_part(role: Role) -> bool {
    matches!(role, Role::Trunk | Role::Canopy | Role::Fruit(_))
}

impl PlantTable {
    pub fn is_tree_part(&self, material: MaterialId) -> bool {
        self.of(material).is_some_and(|r| is_part(r.role))
    }

    /// Whether any Plant is a tree, so a World with none never looks for a
    /// Felling.
    pub fn has_trees(&self) -> bool {
        self.plants()
            .iter()
            .any(|p| matches!(p.kind, Kind::Tree(_)))
    }
}

/// Grown Voxels of one tree that no longer reach the ground.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Region {
    pub plant: usize,
    /// Top-down: highest first, then by x and z.
    pub cells: Vec<[i32; 3]>,
    pub has_trunk: bool,
}

/// The Felling region holding `from`, a grown Voxel of a tree: the grown
/// Voxels of that tree face-connected to it, if none of them is a trunk Voxel
/// standing on a Voxel that is neither Air nor grown. `None` when `from`
/// is no tree Voxel, the region reaches the ground, it is wider or taller than
/// the Plant's size in metres allows, or any cell it needs is unknown.
///
/// Only the tree's own grown Materials join the region, so a placed Voxel of
/// the same look is never in it and never breaks. Every Voxel visited by a
/// search that settles (grounded, over the bound or a region) goes into
/// `settled`, so a caller asking about many neighbours searches each tree once.
pub fn felling_region(
    plants: &PlantTable,
    voxel_size: f32,
    from: [i32; 3],
    material_at: &dyn Fn([i32; 3]) -> Option<MaterialId>,
    settled: &mut HashSet<[i32; 3]>,
) -> Option<Region> {
    let start = plants.of(material_at(from)?)?;
    let Some(Plant {
        kind: Kind::Tree(tree),
        ..
    }) = plants.plant(start.plant)
    else {
        return None;
    };
    if !is_part(start.role) || settled.contains(&from) {
        return None;
    }
    let slack = if tree.branching.is_some() { 4 } else { 0 };
    let wide = 2 * (voxels_in(tree.reach_m().min(MAX_CANOPY_RADIUS_M), voxel_size) + slack) + 3;
    let tall = voxels_in(tree.trunk_height_m.1 + tree.crown_rise_m(), voxel_size) + 3;
    let mut seen: HashSet<[i32; 3]> = HashSet::from([from]);
    let mut stack = vec![from];
    let (mut low, mut high) = (from, from);
    let mut has_trunk = false;
    while let Some(at) = stack.pop() {
        for axis in 0..3 {
            low[axis] = low[axis].min(at[axis]);
            high[axis] = high[axis].max(at[axis]);
        }
        let over = |a: usize, limit: i32| high[a] - low[a] + 1 > limit;
        if over(0, wide) || over(2, wide) || over(1, tall) {
            settled.extend(seen);
            return None;
        }
        let material = material_at(at)?;
        if plants.of(material).is_some_and(|r| r.role == Role::Trunk) {
            has_trunk = true;
            let below = material_at([at[0], at[1] - 1, at[2]])?;
            if below != AIR && !plants.is_grown(below) {
                settled.extend(seen);
                return None;
            }
        }
        for d in FACES {
            let next = [at[0] + d[0], at[1] + d[1], at[2] + d[2]];
            if seen.contains(&next) {
                continue;
            }
            let m = material_at(next)?;
            if plants
                .of(m)
                .is_some_and(|r| r.plant == start.plant && is_part(r.role))
            {
                seen.insert(next);
                stack.push(next);
            }
        }
    }
    let mut cells: Vec<[i32; 3]> = seen.iter().copied().collect();
    cells.sort_unstable_by_key(|c| (std::cmp::Reverse(c[1]), c[0], c[2]));
    settled.extend(seen);
    Some(Region {
        plant: start.plant,
        cells,
        has_trunk,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::climate::Climate;
    use crate::occupants::Occupants;
    use crate::plant::TreeVines;
    use crate::plant::{CountRange, Fruit, Needs, Stage, Yield};
    use crate::shape::{Orientation, ShapeTable};
    use std::collections::HashMap;

    const SOIL: MaterialId = 1;
    const ROCK: MaterialId = 2;
    const SAPLING: MaterialId = 10;
    const TRUNK: MaterialId = 20;
    const CANOPY: MaterialId = 21;
    const FRUIT: [MaterialId; 2] = [22, 23];
    const PLACED_LOG: MaterialId = 30;
    const APPLE: ShapeId = 7;
    const SIZES: [f32; 3] = [1.0, 0.5, 0.25];
    const SEED: u64 = 0xfeed;

    fn tree(fruit: Option<f32>) -> Tree {
        Tree {
            vines: None,
            branching: None,
            trunk: TRUNK,
            canopy: CANOPY,
            trunk_height_m: (3.0, 5.0),
            canopy_radius_m: 2.0,
            mean_seconds: 60.0,
            trunk_yield: vec![Yield {
                item: "t:log".into(),
                count: CountRange { min: 1, max: 1 },
            }],
            canopy_yield: Vec::new(),
            sapling_yield: CountRange { min: 1, max: 2 },
            fruit: fruit.map(|density| Fruit {
                stages: [FRUIT[0], FRUIT[1]]
                    .iter()
                    .enumerate()
                    .map(|(n, &material)| Stage {
                        material,
                        mean_seconds: (n == 0).then_some(60.0),
                        yields: Vec::new(),
                    })
                    .collect(),
                density,
                regrow_seconds: 120.0,
                shape: Some(APPLE),
            }),
        }
    }

    fn plants(fruit: Option<f32>) -> PlantTable {
        let mut table = PlantTable::default();
        table
            .push(Plant {
                name: "oak".into(),
                seed: "t:sapling".into(),
                first: SAPLING,
                shape: None,
                needs: Needs {
                    soil: vec![SOIL],
                    light_max: 15,
                    ..Needs::default()
                },
                kind: Kind::Tree(tree(fruit)),
                sapling_yields: Vec::new(),
            })
            .unwrap();
        table
    }

    #[derive(Default)]
    struct Scene {
        cells: HashMap<[i32; 3], MaterialId>,
        unknown: HashSet<[i32; 3]>,
    }

    impl Scene {
        fn grown(tree: &Tree, base: [i32; 3], size: f32) -> Self {
            let mut scene = Scene::default();
            for x in -40..=40 {
                for z in -40..=40 {
                    scene
                        .cells
                        .insert([base[0] + x, base[1] - 1, base[2] + z], SOIL);
                }
            }
            for v in tree_voxels(tree, SEED, base, size) {
                scene.cells.insert(v.at, v.material);
            }
            scene
        }

        fn material(&self) -> impl Fn([i32; 3]) -> Option<MaterialId> + '_ {
            |at| (!self.unknown.contains(&at)).then(|| self.cells.get(&at).copied().unwrap_or(AIR))
        }
    }

    impl Surroundings for Scene {
        fn occupants(&self, at: [i32; 3]) -> Option<Occupants> {
            (!self.unknown.contains(&at)).then(|| Occupants {
                solid: self.cells.get(&at).map_or(Voxel::AIR, |&m| Voxel::cube(m)),
                fluid: None,
                burning: None,
            })
        }

        fn light(&self, at: [i32; 3]) -> Option<Light> {
            (!self.unknown.contains(&at)).then(|| Light::new(15, 0))
        }

        fn climate_at(&self, _: f32, _: f32) -> Option<Climate> {
            None
        }
    }

    fn felling(table: &PlantTable, scene: &Scene, size: f32, from: [i32; 3]) -> Option<Region> {
        felling_region(table, size, from, &scene.material(), &mut HashSet::new())
    }

    #[test]
    fn a_tree_is_the_declared_metres_at_every_voxel_size() {
        let tree = tree(None);
        for size in SIZES {
            for x in 0..40 {
                let base = [x * 7, 4, -x * 3];
                let voxels = tree_voxels(&tree, SEED, base, size);
                let trunk: Vec<_> = voxels.iter().filter(|v| v.part == Part::Trunk).collect();
                let metres = trunk.len() as f32 * size;
                assert!(
                    (2.999..=5.001).contains(&metres),
                    "{size}: trunk {metres} m"
                );
                assert_eq!(trunk[0].at, base, "the sapling's cell is the trunk's foot");
                let top = trunk.last().unwrap().at;
                for v in voxels.iter().filter(|v| v.part != Part::Trunk) {
                    let across = ((v.at[0] - top[0]).pow(2) + (v.at[2] - top[2]).pow(2)) as f32;
                    assert!(
                        across.sqrt() * size <= 2.0 + 1e-3,
                        "{size}: canopy too wide"
                    );
                    assert!((v.at[1] - top[1]) as f32 * size <= 2.0 * 0.8 + 1e-3);
                    assert!(v.at[1] > base[1], "never at the ground");
                }
            }
        }
    }

    #[test]
    fn a_tree_is_a_pure_function_of_the_row_the_position_and_the_size() {
        let tree = tree(Some(0.2));
        let a = tree_voxels(&tree, SEED, [3, 4, 5], 0.5);
        assert_eq!(a, tree_voxels(&tree, SEED, [3, 4, 5], 0.5));
        assert_ne!(a, tree_voxels(&tree, SEED + 1, [3, 4, 5], 0.5));
    }

    #[test]
    fn fruit_replaces_a_share_of_the_canopy_by_hash_and_only_with_a_fruit_row() {
        let bare = tree_voxels(&tree(None), SEED, [0, 4, 0], 0.25);
        assert!(bare.iter().all(|v| v.part != Part::Fruit));
        let tree = tree(Some(0.2));
        let mut canopy = 0;
        let mut fruit = 0;
        for x in 0..6 {
            for v in tree_voxels(&tree, SEED, [x * 40, 4, 0], 0.25) {
                match v.part {
                    Part::Canopy => canopy += 1,
                    Part::Fruit => {
                        assert_eq!(v.material, FRUIT[0], "fruit starts at Stage 0");
                        fruit += 1;
                    }
                    Part::Trunk => {}
                }
            }
        }
        let share = fruit as f32 / (fruit + canopy) as f32;
        assert!((0.15..0.25).contains(&share), "share {share}");
    }

    fn sprout_at(table: &PlantTable, scene: &Scene, size: f32, at: [i32; 3]) -> Option<Growth> {
        let shapes = ShapeTable::builtin();
        let env = GrowEnv {
            plants: table,
            shapes: &shapes,
            seed: SEED,
            voxel_size: size,
        };
        crate::plant::grow(&env, scene, 0, at, Voxel::cube(SAPLING), 0.0)
    }

    #[test]
    fn a_sapling_with_room_grows_the_whole_tree_as_one_group() {
        let table = plants(Some(0.1));
        let Kind::Tree(row) = &table.plants()[0].kind else {
            unreachable!()
        };
        for size in SIZES {
            let at = [0, 4, 0];
            let mut scene = Scene::default();
            scene.cells.insert([0, 3, 0], SOIL);
            scene.cells.insert(at, SAPLING);
            let growth = sprout_at(&table, &scene, size, at).expect("room");
            assert!(growth.tree);
            let want: Vec<_> = tree_voxels(row, SEED, at, size)
                .into_iter()
                .map(|v| (v.at, v.voxel()))
                .collect();
            let got: Vec<_> = growth.changes.iter().map(|c| (c.at, c.voxel)).collect();
            assert_eq!(got, want, "{size}");
            assert!(
                got.iter()
                    .any(|(_, v)| v.shape() == APPLE && v.material() == FRUIT[0]),
                "{size}: no apple"
            );
            assert!(
                got.iter()
                    .filter(|(_, v)| v.material() == TRUNK || v.material() == CANOPY)
                    .all(|(_, v)| v.shape() == 0),
                "{size}: trunk and canopy stay cubes"
            );
            assert!(growth.changes.len() > 3);
        }
    }

    #[test]
    fn any_blocked_unknown_or_unlit_cell_leaves_the_sapling_and_nothing_lands() {
        let table = plants(None);
        let Kind::Tree(row) = &table.plants()[0].kind else {
            unreachable!()
        };
        let at = [0, 4, 0];
        let cells = tree_voxels(row, SEED, at, 1.0);
        for blocker in [cells[1].at, cells.last().unwrap().at] {
            let mut scene = Scene::default();
            scene.cells.insert([0, 3, 0], SOIL);
            scene.cells.insert(at, SAPLING);
            scene.cells.insert(blocker, ROCK);
            assert_eq!(sprout_at(&table, &scene, 1.0, at), None, "{blocker:?}");
            scene.cells.remove(&blocker);
            scene.unknown.insert(blocker);
            assert_eq!(
                sprout_at(&table, &scene, 1.0, at),
                None,
                "unknown {blocker:?}"
            );
            scene.unknown.clear();
            assert!(sprout_at(&table, &scene, 1.0, at).is_some());
        }
    }

    #[test]
    fn a_tree_may_grow_through_its_own_grown_materials_but_not_a_placed_look_alike() {
        let table = plants(None);
        let Kind::Tree(row) = &table.plants()[0].kind else {
            unreachable!()
        };
        let at = [0, 4, 0];
        let cells = tree_voxels(row, SEED, at, 1.0);
        let mut scene = Scene::default();
        scene.cells.insert([0, 3, 0], SOIL);
        scene.cells.insert(at, SAPLING);
        scene.cells.insert(cells[1].at, TRUNK);
        assert!(sprout_at(&table, &scene, 1.0, at).is_some());
        scene.cells.insert(cells[1].at, PLACED_LOG);
        assert_eq!(sprout_at(&table, &scene, 1.0, at), None);
    }

    const FERN: MaterialId = 50;
    const WHEAT: MaterialId = 51;
    const OTHER_SAPLING: MaterialId = 60;
    const OTHER_TRUNK: MaterialId = 61;
    const OTHER_CANOPY: MaterialId = 62;

    /// The oak, plus a fern that spreads, a wheat crop and a second tree.
    fn crowded() -> PlantTable {
        let mut table = plants(None);
        let needs = Needs {
            soil: vec![SOIL],
            light_max: 15,
            ..Needs::default()
        };
        let mut other = tree(None);
        (other.trunk, other.canopy) = (OTHER_TRUNK, OTHER_CANOPY);
        let rows = [
            (
                "fern",
                FERN,
                Kind::Spreader {
                    reach_m: 1.0,
                    mean_seconds: 60.0,
                    density: 1,
                    yields: Vec::new(),
                },
            ),
            (
                "wheat",
                WHEAT,
                Kind::Crop {
                    stages: [(WHEAT, Some(60.0)), (WHEAT + 1, None)]
                        .map(|(material, mean_seconds)| Stage {
                            material,
                            mean_seconds,
                            yields: Vec::new(),
                        })
                        .to_vec(),
                    harvest_to: None,
                },
            ),
            ("pine", OTHER_SAPLING, Kind::Tree(other)),
        ];
        for (name, first, kind) in rows {
            table
                .push(Plant {
                    name: name.into(),
                    seed: format!("t:{name}"),
                    first,
                    shape: None,
                    needs: needs.clone(),
                    kind,
                    sapling_yields: Vec::new(),
                })
                .unwrap();
        }
        table
    }

    #[test]
    fn ground_cover_and_another_trees_crown_give_way_but_its_trunk_and_a_crop_do_not() {
        let table = crowded();
        let Kind::Tree(row) = &table.plants()[0].kind else {
            unreachable!()
        };
        let at = [0, 4, 0];
        let cells = tree_voxels(row, SEED, at, 1.0);
        let (trunk, crown) = (cells[1].at, cells.last().unwrap().at);
        for cell in [trunk, crown] {
            for foliage in [FERN, OTHER_CANOPY, OTHER_SAPLING] {
                let mut scene = Scene::default();
                scene.cells.insert([0, 3, 0], SOIL);
                scene.cells.insert(at, SAPLING);
                scene.cells.insert(cell, foliage);
                let growth = sprout_at(&table, &scene, 1.0, at).expect("foliage gives way");
                assert!(growth.changes.iter().any(|c| c.at == cell), "{cell:?}");
            }
            for held in [OTHER_TRUNK, WHEAT, ROCK] {
                let mut scene = Scene::default();
                scene.cells.insert([0, 3, 0], SOIL);
                scene.cells.insert(at, SAPLING);
                scene.cells.insert(cell, held);
                assert_eq!(sprout_at(&table, &scene, 1.0, at), None, "{held} {cell:?}");
            }
        }
    }

    #[test]
    fn fruit_ripens_through_its_stages_with_the_trees_light_and_never_past_the_last() {
        let table = plants(Some(0.2));
        let mut scene = Scene::default();
        let at = [0, 8, 0];
        scene.cells.insert(at, FRUIT[0]);
        let shapes = ShapeTable::builtin();
        let env = GrowEnv {
            plants: &table,
            shapes: &shapes,
            seed: SEED,
            voxel_size: 1.0,
        };
        let apple = Voxel::new(FRUIT[0], APPLE, 0);
        let g = crate::plant::grow(&env, &scene, 0, at, apple, 0.0).unwrap();
        assert_eq!(g.changes[0].voxel, Voxel::new(FRUIT[1], APPLE, 0));
        assert!(!g.tree);
        assert_eq!(
            crate::plant::grow(&env, &scene, 0, at, Voxel::cube(FRUIT[1]), 0.0),
            None
        );
        assert_eq!(
            crate::plant::grow(&env, &scene, 0, at, Voxel::cube(FRUIT[0]), 0.99),
            None,
            "a roll above the chance holds"
        );
    }

    fn rebear_env<'a>(table: &'a PlantTable, shapes: &'a ShapeTable, size: f32) -> GrowEnv<'a> {
        GrowEnv {
            plants: table,
            shapes,
            seed: SEED,
            voxel_size: size,
        }
    }

    /// The tree grown at `base` with every fruit Voxel turned back into canopy.
    fn bare(tree: &Tree, base: [i32; 3], size: f32) -> (Scene, Vec<[i32; 3]>) {
        let mut scene = Scene::grown(tree, base, size);
        let leaves: Vec<_> = tree_voxels(tree, SEED, base, size)
            .into_iter()
            .filter(|v| v.part != Part::Trunk)
            .map(|v| v.at)
            .collect();
        for &at in &leaves {
            scene.cells.insert(at, CANOPY);
        }
        (scene, leaves)
    }

    #[test]
    fn the_expected_canopy_is_the_same_tree_in_metres_at_every_size() {
        let table = plants(Some(0.1));
        let Kind::Tree(row) = &table.plants()[0].kind else {
            unreachable!()
        };
        let cubic_metres: Vec<f32> = SIZES
            .iter()
            .map(|&size| expected_canopy_cells(row, size) as f32 * size.powi(3))
            .collect();
        for v in &cubic_metres[1..] {
            assert!((v / cubic_metres[0] - 1.0).abs() < 0.25, "{cubic_metres:?}");
        }
        assert_eq!(
            table.canopy_cells(0, 0.5),
            Some(expected_canopy_cells(row, 0.5)),
            "cached"
        );
    }

    #[test]
    fn a_canopy_voxels_roll_makes_the_tree_grow_one_fruit_per_regrow_seconds() {
        let table = plants(Some(0.1));
        let shapes = ShapeTable::builtin();
        let Kind::Tree(row) = &table.plants()[0].kind else {
            unreachable!()
        };
        for size in SIZES {
            let env = rebear_env(&table, &shapes, size);
            let cells = expected_canopy_cells(row, size) as f32;
            let mean = canopy_mean_seconds(&env, 0, row).unwrap();
            assert_eq!(mean, 120.0 * cells, "{size}");
            let tree_rate =
                cells * crate::plant::roll_chance(mean) / crate::plant::SAMPLE_INTERVAL_S;
            assert!((tree_rate * 120.0 - 1.0).abs() < 1e-3, "{size}");
        }
        let mut barren = tree(None);
        barren.fruit = None;
        assert_eq!(
            canopy_mean_seconds(&rebear_env(&table, &shapes, 1.0), 0, &barren),
            None
        );
    }

    #[test]
    fn a_bare_canopy_voxel_that_passes_its_roll_becomes_unripe_fruit_in_the_fruits_shape() {
        let table = plants(Some(0.1));
        let shapes = ShapeTable::builtin();
        let Kind::Tree(row) = &table.plants()[0].kind else {
            unreachable!()
        };
        for size in SIZES {
            let env = rebear_env(&table, &shapes, size);
            let (scene, leaves) = bare(row, [0, 4, 0], size);
            let at = leaves[leaves.len() / 2];
            let canopy = Voxel::cube(CANOPY);
            let g = crate::plant::grow(&env, &scene, 0, at, canopy, 0.0).expect("room");
            assert_eq!(
                g.changes,
                [Change {
                    at,
                    voxel: Voxel::new(FRUIT[0], APPLE, 0)
                }]
            );
            assert!(!g.tree);
            assert_eq!(
                crate::plant::grow(&env, &scene, 0, at, canopy, 0.999_999),
                None,
                "{size}: a roll above the chance holds"
            );
        }
    }

    #[test]
    fn a_tree_holds_no_more_fruit_than_its_density_of_its_canopy_at_any_size() {
        let table = plants(Some(0.1));
        let shapes = ShapeTable::builtin();
        let Kind::Tree(row) = &table.plants()[0].kind else {
            unreachable!()
        };
        let mut grown = Vec::new();
        for size in SIZES {
            let env = rebear_env(&table, &shapes, size);
            let cap = fruit_cap(0.1, expected_canopy_cells(row, size));
            let (mut scene, leaves) = bare(row, [0, 4, 0], size);
            let (at, rest) = leaves.split_last().unwrap();
            let mut fruit = 0;
            for &cell in rest {
                if crate::plant::grow(&env, &scene, 0, *at, Voxel::cube(CANOPY), 0.0).is_none() {
                    break;
                }
                scene.cells.insert(cell, FRUIT[0]);
                fruit += 1;
            }
            assert!(
                fruit as f32 >= cap && (fruit - 1) as f32 <= cap,
                "{size}: {fruit} of {cap}"
            );
            grown.push(fruit as f32 * size.powi(3));
        }
        for v in &grown[1..] {
            assert!((v / grown[0] - 1.0).abs() < 0.3, "{grown:?}");
        }
    }

    #[test]
    fn a_canopy_voxel_holds_when_the_light_is_unknown_and_a_fruitless_tree_never_bears() {
        let shapes = ShapeTable::builtin();
        let table = plants(Some(0.1));
        let Kind::Tree(row) = &table.plants()[0].kind else {
            unreachable!()
        };
        let env = rebear_env(&table, &shapes, 1.0);
        let (mut scene, leaves) = bare(row, [0, 4, 0], 1.0);
        let at = leaves[0];
        scene.unknown.insert(at);
        assert_eq!(
            crate::plant::grow(&env, &scene, 0, at, Voxel::cube(CANOPY), 0.0),
            None
        );
        let plain = plants(None);
        let env = rebear_env(&plain, &shapes, 1.0);
        let (scene, leaves) = bare(row, [0, 4, 0], 1.0);
        assert_eq!(
            crate::plant::grow(&env, &scene, 0, leaves[0], Voxel::cube(CANOPY), 0.0),
            None
        );
    }

    fn cut(scene: &mut Scene, at: [i32; 3]) {
        scene.cells.remove(&at);
    }

    #[test]
    fn cutting_the_trunk_fells_everything_above_the_cut_and_only_that() {
        let table = plants(None);
        let Kind::Tree(row) = &table.plants()[0].kind else {
            unreachable!()
        };
        for size in SIZES {
            let base = [0, 4, 0];
            let mut scene = Scene::grown(row, base, size);
            let all = tree_voxels(row, SEED, base, size);
            let height = all.iter().filter(|v| v.part == Part::Trunk).count() as i32;
            let cut_at = [0, 4 + height / 2, 0];
            cut(&mut scene, cut_at);
            let above = felling(&table, &scene, size, [0, cut_at[1] + 1, 0]).expect("falls");
            assert!(above.has_trunk);
            assert!(
                above
                    .cells
                    .iter()
                    .all(|c| c[1] > cut_at[1] || c[1] >= cut_at[1])
            );
            let want = all
                .iter()
                .filter(|v| v.part == Part::Canopy || v.at[1] > cut_at[1])
                .count();
            assert_eq!(above.cells.len(), want, "{size}");
            assert!(
                above.cells.windows(2).all(|w| w[0][1] >= w[1][1]),
                "{size}: top-down"
            );
            assert_eq!(
                felling(&table, &scene, size, [0, cut_at[1] - 1, 0]),
                None,
                "{size}: the stump stands"
            );
        }
    }

    #[test]
    fn a_tree_row_outside_its_limits_is_refused_and_one_inside_loads() {
        let load = |edit: &dyn Fn(&mut Tree)| {
            let mut tree = tree(Some(0.2));
            edit(&mut tree);
            PlantTable::default().push(Plant {
                name: "oak".into(),
                seed: "t:sapling".into(),
                first: SAPLING,
                shape: None,
                needs: Needs {
                    soil: vec![SOIL],
                    light_max: 15,
                    ..Needs::default()
                },
                kind: Kind::Tree(tree),
                sapling_yields: Vec::new(),
            })
        };
        assert!(load(&|_| {}).is_ok());
        assert!(load(&|t| t.sapling_yield.min = 0).is_err());
        assert!(load(&|t| t.canopy_radius_m = 0.0).is_err());
        assert!(load(&|t| t.canopy_radius_m = MAX_CANOPY_RADIUS_M + 1.0).is_err());
        assert!(load(&|t| t.trunk_height_m = (0.0, 2.0)).is_err());
        assert!(load(&|t| t.trunk_height_m = (3.0, 2.0)).is_err());
        assert!(load(&|t| t.trunk_height_m.1 = crate::plant::MAX_PLANT_HEIGHT_M + 1.0).is_err());
        assert!(load(&|t| t.mean_seconds = crate::plant::SAMPLE_INTERVAL_S - 1.0).is_err());
        assert!(load(&|t| t.fruit.as_mut().unwrap().density = 0.31).is_err());
        assert!(load(&|t| t.fruit.as_mut().unwrap().density = -0.1).is_err());
        assert!(load(&|t| t.fruit.as_mut().unwrap().stages.truncate(1)).is_err());
        assert!(load(&|t| t.fruit.as_mut().unwrap().regrow_seconds = 15.0).is_err());
        assert!(
            load(&|t| {
                let fruit = t.fruit.as_mut().unwrap();
                fruit.stages.truncate(1);
                fruit.stages.extend((0..3).map(|n| Stage {
                    material: 40 + n,
                    mean_seconds: Some(60.0),
                    yields: Vec::new(),
                }));
                fruit.stages.last_mut().unwrap().mean_seconds = None;
            })
            .is_ok()
        );
        assert!(
            load(&|t| {
                let fruit = t.fruit.as_mut().unwrap();
                fruit.stages.extend((0..3).map(|n| Stage {
                    material: 40 + n,
                    mean_seconds: None,
                    yields: Vec::new(),
                }));
            })
            .is_err(),
            "five fruit Stages are past the limit of four"
        );
        assert!(load(&|t| t.canopy = TRUNK).is_err(), "a Material twice");
    }

    #[test]
    fn a_cut_that_leaves_the_tree_grounded_breaks_nothing() {
        let table = plants(None);
        let Kind::Tree(row) = &table.plants()[0].kind else {
            unreachable!()
        };
        let base = [0, 4, 0];
        let mut scene = Scene::grown(row, base, 1.0);
        let canopy: Vec<_> = tree_voxels(row, SEED, base, 1.0)
            .into_iter()
            .filter(|v| v.part == Part::Canopy)
            .collect();
        let leaf = canopy[canopy.len() / 2].at;
        cut(&mut scene, leaf);
        for d in FACES {
            let n = [leaf[0] + d[0], leaf[1] + d[1], leaf[2] + d[2]];
            assert_eq!(felling(&table, &scene, 1.0, n), None);
        }
    }

    #[test]
    fn a_trunk_voxel_standing_on_a_grown_one_is_not_grounded_and_a_placed_base_is() {
        let table = plants(None);
        let Kind::Tree(row) = &table.plants()[0].kind else {
            unreachable!()
        };
        let base = [0, 4, 0];
        let mut scene = Scene::grown(row, base, 1.0);
        cut(&mut scene, base);
        let g = felling(&table, &scene, 1.0, [0, 5, 0]).expect("the whole tree falls");
        assert!(g.has_trunk);
        scene.cells.insert(base, PLACED_LOG);
        assert_eq!(
            felling(&table, &scene, 1.0, [0, 5, 0]),
            None,
            "a placed Voxel holds it up"
        );
        scene.cells.insert(base, SAPLING);
        assert_eq!(
            felling(&table, &scene, 1.0, [0, 5, 0]),
            None,
            "a Voxel that is not grown holds it up even when it is a Plant's"
        );
    }

    #[test]
    fn placed_voxels_of_any_look_are_never_in_a_region() {
        let table = plants(None);
        let Kind::Tree(row) = &table.plants()[0].kind else {
            unreachable!()
        };
        let base = [0, 4, 0];
        let mut scene = Scene::grown(row, base, 1.0);
        let top = tree_voxels(row, SEED, base, 1.0)
            .iter()
            .rfind(|v| v.part == Part::Trunk)
            .unwrap()
            .at;
        scene
            .cells
            .insert([top[0] + 1, top[1] + 4, top[2]], PLACED_LOG);
        scene
            .cells
            .insert([top[0] + 20, top[1], top[2]], PLACED_LOG);
        cut(&mut scene, [0, 5, 0]);
        let r = felling(&table, &scene, 1.0, [0, 6, 0]).expect("falls");
        assert!(r.cells.iter().all(|c| scene.cells[c] != PLACED_LOG));
        assert!(!r.cells.contains(&[top[0] + 1, top[1] + 4, top[2]]));

        let mut cabin = Scene::default();
        for x in 0..4 {
            for y in 0..4 {
                for z in 0..4 {
                    let m = if (x + y + z) % 2 == 0 {
                        TRUNK + 10
                    } else {
                        CANOPY + 10
                    };
                    cabin.cells.insert([x, y + 5, z], m);
                }
            }
        }
        assert_eq!(
            felling(&table, &cabin, 1.0, [1, 6, 1]),
            None,
            "not even a plant"
        );
    }

    #[test]
    fn a_region_over_the_declared_size_does_not_break_and_an_unknown_cell_defers() {
        let table = plants(None);
        let mut big = Scene::default();
        for x in 0..40 {
            big.cells.insert([x, 20, 0], CANOPY);
        }
        assert_eq!(felling(&table, &big, 1.0, [0, 20, 0]), None);
        let mut small = Scene::default();
        for x in 0..4 {
            small.cells.insert([x, 20, 0], CANOPY);
        }
        assert!(felling(&table, &small, 1.0, [0, 20, 0]).is_some());
        small.unknown.insert([4, 20, 0]);
        assert_eq!(felling(&table, &small, 1.0, [0, 20, 0]), None);
    }

    #[test]
    fn a_search_settles_every_voxel_it_visited_so_a_tree_is_searched_once() {
        let table = plants(None);
        let Kind::Tree(row) = &table.plants()[0].kind else {
            unreachable!()
        };
        let base = [0, 4, 0];
        let scene = Scene::grown(row, base, 1.0);
        let mut settled = HashSet::new();
        let material = scene.material();
        assert_eq!(
            felling_region(&table, 1.0, [0, 6, 0], &material, &mut settled),
            None
        );
        assert!(settled.contains(&[0, 6, 0]));
        assert!(settled.contains(&[0, 4, 0]) || settled.len() > 1);
    }

    /// The tree as it was generated before `branching` existed, kept to prove a row without it
    /// still generates exactly so.
    fn before_branching(tree: &Tree, seed: u64, base: [i32; 3], size: f32) -> Vec<TreeVoxel> {
        let height = trunk_height(tree, seed, base, size);
        let mut voxels: Vec<TreeVoxel> = (0..height)
            .map(|dy| wood([base[0], base[1] + dy, base[2]], tree.trunk))
            .collect();
        let top = [base[0], base[1] + height - 1, base[2]];
        let wide = (tree.canopy_radius_m / size).max(1.0);
        let tall = (tree.canopy_radius_m * 0.8 / size).max(1.0);
        let lowest = base[1] + (height / 2).max(1);
        let (rx, ry) = (wide.ceil() as i32, tall.ceil() as i32);
        for dx in -rx..=rx {
            for dz in -rx..=rx {
                for dy in -ry..=ry {
                    let inside = (dx * dx + dz * dz) as f32 / (wide * wide)
                        + (dy * dy) as f32 / (tall * tall);
                    let at = [top[0] + dx, top[1] + dy, top[2] + dz];
                    if inside > 1.0 + 1e-4 || at[1] < lowest || (dx == 0 && dz == 0 && dy <= 0) {
                        continue;
                    }
                    voxels.push(leaf(tree, seed, at, size));
                }
            }
        }
        voxels
    }

    #[test]
    fn a_tree_row_without_branching_generates_exactly_as_it_did() {
        for fruit in [None, Some(0.2)] {
            let row = tree(fruit);
            assert!(row.branching.is_none());
            for size in SIZES {
                for n in 0..40 {
                    let base = [n * 7, 4, -n * 3];
                    assert_eq!(
                        tree_voxels(&row, SEED, base, size),
                        before_branching(&row, SEED, base, size),
                        "{size} at {base:?}"
                    );
                }
            }
        }
    }

    /// The flora species' branching, in metres, as `mods/flora/content.toml` declares them.
    fn species() -> Vec<(&'static str, Tree)> {
        let row = |name, trunk: (f32, f32), canopy, b: Branching| {
            let mut t = tree(None);
            t.trunk_height_m = trunk;
            t.canopy_radius_m = canopy;
            t.branching = Some(b);
            (name, t)
        };
        let limbs = |min, max| CountRange { min, max };
        let plain = Branching {
            limbs: limbs(0, 0),
            start: 0.0,
            reach_m: 0.0,
            rise: 0.0,
            fork: None,
            lean_m: 0.0,
            tiers: false,
            lobe_radius_m: 1.0,
        };
        vec![
            row(
                "oak",
                (4.0, 6.0),
                2.5,
                Branching {
                    limbs: limbs(3, 4),
                    start: 0.25,
                    reach_m: 3.5,
                    rise: 0.3,
                    fork: Some(0.3),
                    lobe_radius_m: 1.5,
                    ..plain
                },
            ),
            row(
                "birch",
                (5.0, 7.5),
                1.5,
                Branching {
                    limbs: limbs(2, 3),
                    start: 0.5,
                    reach_m: 2.0,
                    rise: 0.6,
                    lean_m: 1.0,
                    lobe_radius_m: 0.9,
                    ..plain
                },
            ),
            row(
                "pine",
                (7.0, 10.0),
                1.2,
                Branching {
                    limbs: limbs(3, 5),
                    start: 0.25,
                    reach_m: 3.0,
                    rise: -0.5,
                    tiers: true,
                    lobe_radius_m: 0.9,
                    ..plain
                },
            ),
            row(
                "palm",
                (6.0, 9.0),
                2.2,
                Branching {
                    lean_m: 2.0,
                    ..plain
                },
            ),
            row(
                "acacia",
                (3.5, 5.5),
                3.5,
                Branching {
                    reach_m: 3.0,
                    fork: Some(0.25),
                    lean_m: 0.5,
                    lobe_radius_m: 2.0,
                    ..plain
                },
            ),
            row(
                "jungle",
                (10.0, 16.0),
                5.0,
                Branching {
                    limbs: limbs(3, 4),
                    start: 0.55,
                    reach_m: 4.5,
                    rise: 0.2,
                    lobe_radius_m: 1.8,
                    ..plain
                },
            ),
            row(
                "swamp",
                (3.0, 4.5),
                3.5,
                Branching {
                    limbs: limbs(3, 4),
                    start: 0.3,
                    reach_m: 3.5,
                    rise: -0.3,
                    lean_m: 1.0,
                    lobe_radius_m: 1.5,
                    ..plain
                },
            ),
            row(
                "apple",
                (4.0, 6.0),
                2.5,
                Branching {
                    limbs: limbs(3, 4),
                    start: 0.3,
                    reach_m: 2.5,
                    rise: 0.2,
                    lobe_radius_m: 1.2,
                    ..plain
                },
            ),
        ]
    }

    fn horizontal_m(at: [i32; 3], from: [i32; 3], size: f32) -> f32 {
        (((at[0] - from[0]).pow(2) + (at[2] - from[2]).pow(2)) as f32).sqrt() * size
    }

    /// Whether the cells are one piece through shared faces.
    fn one_piece(cells: &[[i32; 3]]) -> bool {
        let all: HashSet<_> = cells.iter().copied().collect();
        let mut seen = HashSet::from([cells[0]]);
        let mut stack = vec![cells[0]];
        while let Some(c) = stack.pop() {
            for d in FACES {
                let n = [c[0] + d[0], c[1] + d[1], c[2] + d[2]];
                if all.contains(&n) && seen.insert(n) {
                    stack.push(n);
                }
            }
        }
        seen.len() == all.len()
    }

    #[test]
    fn a_stepped_line_is_face_connected_between_its_ends_and_symmetric_in_length() {
        for (a, b) in [
            ([0, 0, 0], [0, 7, 0]),
            ([0, 0, 0], [5, 2, -3]),
            ([2, 3, 4], [-6, 3, 4]),
            ([0, 0, 0], [3, 3, 3]),
            ([1, 1, 1], [1, 1, 1]),
        ] {
            let cells = line(a, b);
            let steps: i32 = (0..3).map(|i| (b[i] - a[i]).abs()).sum();
            assert_eq!(cells.len() as i32, steps + 1);
            assert_eq!((cells[0], *cells.last().unwrap()), (a, b));
            for w in cells.windows(2) {
                let moved: i32 = (0..3).map(|i| (w[1][i] - w[0][i]).abs()).sum();
                assert_eq!(moved, 1, "{a:?} to {b:?}");
            }
        }
    }

    #[test]
    fn every_species_is_one_piece_inside_its_extent_at_every_voxel_size() {
        for (name, row) in species() {
            let b = row.branching.unwrap();
            assert!(
                b.check(row.canopy_radius_m, row.trunk_height_m.0).is_ok(),
                "{name}"
            );
            for size in SIZES {
                for n in 0..12 {
                    let base = [n * 31, 4, -n * 17];
                    let voxels = tree_voxels(&row, SEED, base, size);
                    assert_eq!(voxels, tree_voxels(&row, SEED, base, size), "{name}: pure");
                    let cells: Vec<_> = voxels.iter().map(|v| v.at).collect();
                    let unique: HashSet<_> = cells.iter().collect();
                    assert_eq!(unique.len(), cells.len(), "{name} {size}: a cell twice");
                    assert_eq!(voxels[0].at, base, "{name}: the sapling's cell is the foot");
                    assert!(one_piece(&cells), "{name} {size} m at {base:?}: in pieces");
                    for v in &voxels {
                        assert!(v.at[1] >= base[1], "{name}: below the ground");
                        let across = horizontal_m(v.at, base, size);
                        assert!(
                            across <= row.reach_m() + 4.0 * size + 1e-3,
                            "{name} {size}: {across} m is past {} m",
                            row.reach_m()
                        );
                        if v.part != Part::Trunk {
                            assert!(v.at[1] > base[1], "{name}: foliage on the ground");
                        }
                    }
                    let top = voxels.iter().map(|v| v.at[1]).max().unwrap();
                    let rise_m = (top - base[1] + 1) as f32 * size;
                    assert!(
                        rise_m <= row.trunk_height_m.1 + row.crown_rise_m() + size + 1e-3,
                        "{name} {size}: {rise_m} m tall"
                    );
                }
            }
        }
    }

    fn wood_of(row: &Tree, base: [i32; 3], size: f32) -> Vec<[i32; 3]> {
        tree_voxels(row, SEED, base, size)
            .into_iter()
            .filter(|v| v.part == Part::Trunk)
            .map(|v| v.at)
            .collect()
    }

    #[test]
    fn limbs_begin_at_their_declared_height_and_reach_as_far_as_declared() {
        for (name, row) in species() {
            let b = row.branching.unwrap();
            if b.limbs.max == 0 {
                continue;
            }
            for size in SIZES {
                let mut longest: f32 = 0.0;
                for n in 0..12 {
                    let base = [n * 29, 4, n * 13];
                    let wood = wood_of(&row, base, size);
                    let top = wood.iter().map(|c| c[1]).max().unwrap() - base[1];
                    let height_m = (top + 1) as f32 * size;
                    // A limb droops below its root by half its reach times the droop.
                    let droop = (-b.rise).max(0.0) * b.reach_m / 2.0;
                    let lowest_m = b.start * height_m - droop - 2.0 * size;
                    let spread = b.fork.map_or(0.0, |_| b.reach_m * FORK_SPREAD);
                    for c in &wood {
                        let across = horizontal_m(*c, base, size);
                        if across > b.lean_m + spread + 2.0 * size {
                            longest = longest.max(across);
                            assert!(
                                ((c[1] - base[1]) as f32 * size) >= lowest_m,
                                "{name} {size}: wood {across} m out at {} m, below limbs' start",
                                (c[1] - base[1]) as f32 * size
                            );
                        }
                    }
                }
                assert!(
                    longest >= b.reach_m * 0.65,
                    "{name} {size}: longest limb {longest} m of {} m",
                    b.reach_m
                );
            }
        }
    }

    #[test]
    fn a_leaning_trunk_stands_its_declared_lean_over_and_a_straight_one_stays_over_its_foot() {
        for (name, row) in species() {
            let b = row.branching.unwrap();
            if b.limbs.max > 0 || b.fork.is_some() {
                continue;
            }
            for size in SIZES {
                for n in 0..8 {
                    let base = [n * 11, 4, n * 5];
                    let wood = wood_of(&row, base, size);
                    let top = *wood.last().unwrap();
                    let leant = horizontal_m(top, base, size);
                    assert!(
                        (leant - b.lean_m).abs() <= 1.5 * size,
                        "{name} {size}: top {leant} m over, wanted {} m",
                        b.lean_m
                    );
                    assert!(one_piece(&wood), "{name} {size}: the trunk is broken");
                    assert!(
                        wood.len() as f32 * size >= row.trunk_height_m.0,
                        "{name} {size}: short"
                    );
                }
            }
        }
        let mut straight = tree(None);
        straight.branching = Some(Branching {
            limbs: CountRange { min: 0, max: 0 },
            start: 0.0,
            reach_m: 0.0,
            rise: 0.0,
            fork: None,
            lean_m: 0.0,
            tiers: false,
            lobe_radius_m: 1.0,
        });
        for size in SIZES {
            let wood = wood_of(&straight, [0, 4, 0], size);
            assert!(wood.iter().all(|c| c[0] == 0 && c[2] == 0), "{size}");
        }
    }

    #[test]
    fn a_fork_splits_the_trunk_into_two_or_three_leaders_at_its_height() {
        for name in ["oak", "acacia"] {
            let (_, mut row) = species().into_iter().find(|s| s.0 == name).unwrap();
            let b = row.branching.as_mut().unwrap();
            // Limbs would reach the top row too; the leaders alone are under test.
            b.limbs = CountRange { min: 0, max: 0 };
            let b = *b;
            for size in SIZES {
                for n in 0..8 {
                    let base = [n * 23, 4, -n * 7];
                    let wood = wood_of(&row, base, size);
                    let top = wood.iter().map(|c| c[1]).max().unwrap();
                    let height_m = (top - base[1] + 1) as f32 * size;
                    let columns = |y: i32| -> Vec<[i32; 2]> {
                        wood.iter()
                            .filter(|c| c[1] == y)
                            .map(|c| [c[0], c[2]])
                            .collect()
                    };
                    let leaders = columns(top);
                    let fork_y = base[1] + (b.fork.unwrap() * height_m / size) as i32;
                    // At a coarse size the leaders touch near the fork, so only the tips count.
                    let above: Vec<_> = wood
                        .iter()
                        .copied()
                        .filter(|c| c[1] > (fork_y + top) / 2)
                        .collect();
                    let mut pieces = 0;
                    let mut left: HashSet<_> = above.iter().copied().collect();
                    while let Some(&start) = left.iter().next() {
                        pieces += 1;
                        let mut stack = vec![start];
                        left.remove(&start);
                        while let Some(c) = stack.pop() {
                            for d in FACES {
                                let n = [c[0] + d[0], c[1] + d[1], c[2] + d[2]];
                                if left.remove(&n) {
                                    stack.push(n);
                                }
                            }
                        }
                    }
                    if size <= 0.5 {
                        assert!((2..=3).contains(&pieces), "{name} {size}: {pieces} leaders");
                    }
                    assert!(leaders.len() >= 2, "{name} {size}: one tip");
                    // A leaning trunk may take one sideways step in a row.
                    assert!(
                        columns(fork_y - 1).len() <= 2,
                        "{name} {size}: one trunk below the fork"
                    );
                    let apart = (0..leaders.len())
                        .flat_map(|i| (0..i).map(move |j| (i, j)))
                        .map(|(i, j)| {
                            let d = [leaders[i][0] - leaders[j][0], leaders[i][1] - leaders[j][1]];
                            ((d[0] * d[0] + d[1] * d[1]) as f32).sqrt() * size
                        })
                        .fold(f32::MAX, f32::min);
                    assert!(
                        apart >= b.reach_m * FORK_SPREAD * 0.5 - size,
                        "{name} {size}: leaders {apart} m apart"
                    );
                }
            }
        }
    }

    #[test]
    fn a_pine_is_a_cone_of_tiers_that_shorten_towards_the_top() {
        let (_, row) = species().into_iter().find(|s| s.0 == "pine").unwrap();
        for size in SIZES {
            let (mut limbs, mut needles) = ([0.0f32; 2], [0.0f32; 2]);
            for n in 0..8 {
                let base = [n * 19, 4, n * 3];
                let voxels = tree_voxels(&row, SEED, base, size);
                let top = voxels.iter().map(|v| v.at[1]).max().unwrap();
                let third = (top - base[1]) / 3;
                let widest = |from: i32, to: i32, wood: bool| {
                    voxels
                        .iter()
                        .filter(|v| (v.part == Part::Trunk) == wood)
                        .filter(|v| (from..to).contains(&(v.at[1] - base[1])))
                        .map(|v| horizontal_m(v.at, base, size))
                        .fold(0.0, f32::max)
                };
                limbs[0] += widest(third, 2 * third, true);
                limbs[1] += widest(2 * third, top, true);
                needles[0] += widest(1, 2 * third, false);
                needles[1] += widest(2 * third, top + 1, false);
            }
            assert!(limbs[0] > limbs[1] * 1.15, "{size}: limbs {limbs:?}");
            assert!(
                needles[0] > needles[1] * 1.15,
                "{size}: needles {needles:?}"
            );
        }
    }

    #[test]
    fn a_tree_is_equal_in_metres_at_every_voxel_size() {
        for (name, row) in species() {
            for n in 0..8 {
                let base_m = [n as f32 * 8.0, 0.0, -n as f32 * 4.0];
                let measure = |size: f32| {
                    let base = base_m.map(|m| (m / size).round() as i32);
                    let voxels = tree_voxels(&row, SEED, base, size);
                    let rise = voxels.iter().map(|v| v.at[1] - base[1] + 1).max().unwrap();
                    let rise_m = rise as f32 * size;
                    assert!(
                        (row.trunk_height_m.0..=row.trunk_height_m.1 + row.crown_rise_m() + size)
                            .contains(&rise_m),
                        "{name} {size}: {rise_m} m tall"
                    );
                    let wood = voxels.iter().filter(|v| v.part == Part::Trunk);
                    let reach = wood
                        .map(|v| horizontal_m(v.at, base, size))
                        .fold(0.0, f32::max);
                    reach
                };
                let fine = measure(0.25);
                for size in [0.5, 1.0] {
                    let got = measure(size);
                    assert!(
                        (got - fine).abs() <= 1.5 * size + 1e-3,
                        "{name} {size}: {got:?} against {fine:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_tree_row_whose_branching_cannot_be_drawn_is_refused() {
        let (_, oak) = species().into_iter().find(|s| s.0 == "oak").unwrap();
        let load = |edit: &dyn Fn(&mut Branching)| {
            let mut t = oak.clone();
            edit(t.branching.as_mut().unwrap());
            PlantTable::default().push(Plant {
                name: "oak".into(),
                seed: "t:sapling".into(),
                first: SAPLING,
                shape: None,
                needs: Needs {
                    soil: vec![SOIL],
                    light_max: 15,
                    ..Needs::default()
                },
                kind: Kind::Tree(t),
                sapling_yields: Vec::new(),
            })
        };
        assert!(load(&|_| {}).is_ok());
        assert!(load(&|b| b.limbs.max = 17).is_err());
        assert!(load(&|b| b.limbs = CountRange { min: 4, max: 3 }).is_err());
        assert!(load(&|b| b.start = 0.9).is_err());
        assert!(load(&|b| b.reach_m = 0.0).is_err());
        assert!(load(&|b| b.rise = 1.1).is_err());
        assert!(load(&|b| b.fork = Some(0.05)).is_err());
        assert!(
            load(&|b| b.lean_m = 4.1).is_err(),
            "more than the lowest trunk"
        );
        assert!(load(&|b| b.lobe_radius_m = 0.0).is_err());
        assert!(
            load(&|b| b.reach_m = 5.0).is_err(),
            "past 8 m with the fork and the lobe"
        );
        assert!(
            load(&|b| b.lean_m = 3.0).is_err(),
            "lean, reach and lobe past 8 m"
        );
        assert!(load(&|b| b.fork = Some(0.3)).is_ok());
        assert!(
            load(&|b| {
                b.tiers = true;
                b.limbs = CountRange { min: 9, max: 9 };
            })
            .is_err()
        );
    }

    fn table_of(row: &Tree) -> PlantTable {
        let mut table = PlantTable::default();
        table
            .push(Plant {
                kind: Kind::Tree(row.clone()),
                ..plants(None).plants()[0].clone()
            })
            .unwrap();
        table
    }

    #[test]
    fn cutting_a_branched_trunk_fells_the_limbs_and_lobes_above_it_and_leaves_the_stump() {
        for (name, row) in species()
            .into_iter()
            .filter(|s| ["oak", "pine", "apple", "jungle"].contains(&s.0))
        {
            let table = table_of(&row);
            let b = row.branching.unwrap();
            for size in SIZES {
                let base = [0, 4, 0];
                let all = tree_voxels(&row, SEED, base, size);
                let top = all.iter().map(|v| v.at[1]).max().unwrap();
                let trunk_top = all
                    .iter()
                    .filter(|v| v.part == Part::Trunk)
                    .map(|v| v.at[1])
                    .max()
                    .unwrap();
                // Just under the lowest limb, so the cut is on the trunk and not in a limb's root.
                let cut_y =
                    base[1] + 1.max((b.start * (trunk_top - base[1]) as f32).round() as i32) - 1;
                let cut_at = [0, cut_y, 0];
                assert!(
                    all.iter().any(|v| v.at == cut_at),
                    "{name} {size}: no trunk there"
                );
                let mut scene = Scene::grown(&row, base, size);
                cut(&mut scene, cut_at);
                let region = felling(&table, &scene, size, [0, cut_y + 1, 0])
                    .unwrap_or_else(|| panic!("{name} {size}: the part above the cut stands"));
                assert!(region.has_trunk);
                let broken: HashSet<_> = region.cells.iter().copied().collect();
                for v in all.iter().filter(|v| v.at[1] > cut_y) {
                    assert!(broken.contains(&v.at), "{name} {size}: {:?} stays", v.at);
                }
                assert!(
                    broken.iter().any(|c| c[1] == top),
                    "{name} {size}: the crown stays"
                );
                assert!(
                    all.iter()
                        .any(|v| v.part == Part::Canopy && broken.contains(&v.at)),
                    "{name} {size}: no lobe falls"
                );
                for v in all
                    .iter()
                    .filter(|v| v.part == Part::Trunk && v.at[0] == 0 && v.at[2] == 0)
                {
                    if v.at[1] < cut_y {
                        assert!(!broken.contains(&v.at), "{name} {size}: the stump falls");
                    }
                }
                assert_eq!(felling(&table, &scene, size, [0, cut_y - 1, 0]), None);
                assert!(
                    region.cells.windows(2).all(|w| w[0][1] >= w[1][1]),
                    "{name} {size}: top-down"
                );
            }
        }
    }

    #[test]
    fn a_limb_cut_through_drops_its_end_and_what_hung_on_it_and_the_trunk_stands() {
        for (name, row) in species()
            .into_iter()
            .filter(|s| ["apple", "jungle", "swamp"].contains(&s.0))
        {
            let table = table_of(&row);
            for size in SIZES {
                let base = [0, 4, 0];
                let all = tree_voxels(&row, SEED, base, size);
                let wood: Vec<_> = all.iter().filter(|v| v.part == Part::Trunk).collect();
                let limb: Vec<_> = wood
                    .iter()
                    .filter(|v| horizontal_m(v.at, base, size) > 1.5)
                    .collect();
                let cut_at = limb[limb.len() / 3].at;
                let mut scene = Scene::grown(&row, base, size);
                cut(&mut scene, cut_at);
                let trunk = wood[wood.len() / 2].at;
                assert_eq!(
                    felling(&table, &scene, size, trunk),
                    None,
                    "{name} {size}: the trunk stands"
                );
                let beyond = limb.last().unwrap().at;
                if let Some(fell) = felling(&table, &scene, size, beyond) {
                    assert!(!fell.cells.contains(&base), "{name} {size}: the foot falls");
                    assert!(
                        !fell.cells.contains(&trunk),
                        "{name} {size}: the trunk falls"
                    );
                    assert!(fell.cells.contains(&beyond), "{name} {size}: the tip stays");
                }
            }
        }
    }

    const VINE: MaterialId = 40;

    fn vine_shapes() -> (ShapeTable, ShapeId) {
        use crate::shape::{Form, NodeBox, ShapeDef};
        let mut shapes = ShapeTable::builtin();
        let id = shapes
            .register(ShapeDef {
                form: Form::Plane,
                boxes: vec![NodeBox::new([1, 1, 0], [15, 15, 2])],
                ..ShapeDef::default()
            })
            .unwrap();
        (shapes, id)
    }

    /// A jungle tree whose vines are the climber `t:vine`, held by canopy and trunk.
    fn jungle(density: f32, length_m: f32) -> (PlantTable, ShapeTable, SurfaceVines) {
        let (shapes, id) = vine_shapes();
        let mut table = plants(None);
        table
            .push(Plant {
                name: "t:vine".into(),
                seed: "t:vine_seed".into(),
                first: VINE,
                shape: Some(id),
                needs: Needs {
                    soil: vec![SOIL, CANOPY, TRUNK],
                    light_max: 15,
                    ..Needs::default()
                },
                kind: Kind::Climber {
                    mean_seconds: 20.0,
                    length_m: 4.0,
                    yields: Vec::new(),
                },
                sapling_yields: Vec::new(),
            })
            .unwrap();
        let vines = table
            .vines(
                &shapes,
                &TreeVines {
                    plant: "t:vine".into(),
                    density,
                    length_m,
                },
            )
            .unwrap();
        (table, shapes, vines)
    }

    #[test]
    fn vines_hang_from_the_outer_canopy_in_metres_at_every_voxel_size() {
        let (_, _, vines) = jungle(0.4, 6.0);
        for size in SIZES {
            let cells = tree_voxels(&tree(None), SEED, [0, 0, 0], size);
            let hung = vine_voxels(&cells, &vines, SEED, size);
            assert_eq!(hung, vine_voxels(&cells, &vines, SEED, size), "pure");
            assert!(hung.len() > 5, "{size} m: {} vines", hung.len());
            let own: HashSet<_> = cells.iter().map(|c| c.at).collect();
            let canopy_low = cells
                .iter()
                .filter(|c| c.part != Part::Trunk)
                .map(|c| c.at[1]);
            let lowest_canopy = canopy_low.min().unwrap();
            for vine in &hung {
                assert!(
                    vine.length as f32 * size <= 6.0 + 1e-3,
                    "{size} m: {vine:?}"
                );
                assert!(
                    vine.cells().all(|c| !own.contains(&c)),
                    "{size} m: {vine:?}"
                );
                assert!(vine.top[1] >= lowest_canopy, "hangs from the canopy");
                let held: Vec<usize> = VINE_FACES
                    .iter()
                    .enumerate()
                    .filter(|&(_, &f)| {
                        let n = FACE_STEPS[f];
                        let at = [vine.top[0] + n[0], vine.top[1] + n[1], vine.top[2] + n[2]];
                        own.contains(&at)
                    })
                    .map(|(i, _)| i)
                    .collect();
                assert!(
                    held.iter()
                        .any(|&i| vines.orientations[i] == vine.orientation),
                    "{size} m: {vine:?} lies against a face it touches"
                );
            }
            let longest = hung.iter().map(|v| v.length).max().unwrap();
            assert!(longest as f32 * size > 3.0, "{size} m: longest {longest}");
        }
    }

    #[test]
    fn a_generated_vine_breaks_when_its_anchor_is_removed() {
        let (table, shapes, vines) = jungle(0.4, 6.0);
        let cells = tree_voxels(&tree(None), SEED, [0, 0, 0], 1.0);
        let hung = vine_voxels(&cells, &vines, SEED, 1.0);
        let vine = hung.first().expect("a vine");
        let mut world: HashMap<[i32; 3], Voxel> = cells.iter().map(|c| (c.at, c.voxel())).collect();
        for hung in &hung {
            for at in hung.cells() {
                world.entry(at).or_insert(Voxel::shaped(
                    VINE,
                    vines.shape,
                    Orientation::from_bits(hung.orientation).unwrap(),
                ));
            }
        }
        let lacks = |world: &HashMap<[i32; 3], Voxel>, at: [i32; 3]| {
            let occupants = |c: [i32; 3]| {
                Some(Occupants {
                    solid: world.get(&c).copied().unwrap_or(Voxel::AIR),
                    fluid: None,
                    burning: None,
                })
            };
            table
                .lacks_support(&shapes, world[&at], at, &occupants)
                .is_some()
        };
        for at in vine.cells() {
            assert!(!lacks(&world, at), "a generated vine holds at {at:?}");
        }
        let face = shapes.plane_face(world[&vine.top]).expect("a plane");
        let step = FACE_STEPS[face];
        let held = [
            vine.top[0] + step[0],
            vine.top[1] + step[1],
            vine.top[2] + step[2],
        ];
        assert!(cells.iter().any(|c| c.at == held), "anchored to the tree");
        world.remove(&held);
        assert!(
            lacks(&world, vine.top),
            "the top vine falls with its anchor"
        );
    }

    #[test]
    fn vines_must_name_a_climber_with_a_plane_shape() {
        let (table, shapes, _) = jungle(0.2, 4.0);
        let vines = |plant: &str| TreeVines {
            plant: plant.into(),
            density: 0.2,
            length_m: 4.0,
        };
        assert!(
            table
                .vines(&shapes, &vines("t:nope"))
                .unwrap_err()
                .contains("not declared")
        );
        assert!(
            table
                .vines(&shapes, &vines("oak"))
                .unwrap_err()
                .contains("not a climber")
        );
    }
}
