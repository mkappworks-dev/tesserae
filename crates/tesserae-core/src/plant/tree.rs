//! The parametric tree and the Felling region: pure functions of a Plant row,
//! a position and the Voxel size, so worldgen and the Server place and cut the
//! same Voxels from any Chunk's viewpoint.

use std::collections::HashSet;

use super::reach::voxels_in;
use super::{
    Change, GrowEnv, Growth, Kind, MAX_CANOPY_RADIUS_M, Plant, PlantTable, Role, Surroundings,
    Tree, conditions_met, mix,
};
use crate::light::Light;
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
/// sapling's own cell upward, then the ellipsoidal canopy centred on the
/// trunk's top, some of it fruit. A pure function of the row, the position and
/// the Voxel size.
pub fn tree_voxels(tree: &Tree, seed: u64, base: [i32; 3], voxel_size: f32) -> Vec<TreeVoxel> {
    let height = trunk_height(tree, seed, base, voxel_size);
    let mut voxels: Vec<TreeVoxel> = (0..height)
        .map(|dy| TreeVoxel {
            at: [base[0], base[1] + dy, base[2]],
            material: tree.trunk,
            part: Part::Trunk,
            shape: SHAPE_FULL_CUBE,
            orientation: 0,
        })
        .collect();
    let top = [base[0], base[1] + height - 1, base[2]];
    let wide = (tree.canopy_radius_m / voxel_size).max(1.0);
    let tall = (tree.canopy_radius_m * CANOPY_HEIGHT_RATIO / voxel_size).max(1.0);
    // The canopy leaves the lower trunk bare, and never reaches the ground.
    let lowest = base[1] + (height / 2).max(1);
    let (rx, ry) = (wide.ceil() as i32, tall.ceil() as i32);
    for dx in -rx..=rx {
        for dz in -rx..=rx {
            for dy in -ry..=ry {
                let inside =
                    (dx * dx + dz * dz) as f32 / (wide * wide) + (dy * dy) as f32 / (tall * tall);
                let at = [top[0] + dx, top[1] + dy, top[2] + dz];
                if inside > 1.0 + 1e-4 || at[1] < lowest || (dx == 0 && dz == 0 && dy <= 0) {
                    continue;
                }
                let fruit = tree
                    .fruit
                    .as_ref()
                    .filter(|f| bears_fruit(seed, f.density, at, voxel_size));
                voxels.push(match fruit {
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
                });
            }
        }
    }
    voxels
}

/// The Voxel a sapling that has passed its roll grows into: the whole tree as
/// one group, or `None` when any cell is not Air or the tree's own Materials,
/// is unknown, or its Chunk's light is pending. A blocked sapling stays and
/// Samples again.
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
    let own = |material| {
        env.plants
            .of(material)
            .is_some_and(|r| r.plant == index && is_part(r.role))
    };
    let mut changes = Vec::new();
    for v in tree_voxels(tree, env.seed, at, env.voxel_size) {
        let here = view.occupants(v.at)?;
        view.light(v.at)?;
        let clear = here.solid.is_air() && here.fluid.is_none() && here.burning.is_none();
        if !(clear || v.at == at || own(here.solid.material())) {
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
    /// Whether a Material is a trunk, canopy or fruit Voxel of some tree.
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
/// standing on a Voxel that is neither Air nor a Plant's. `None` when `from`
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
    let wide = 2 * voxels_in(tree.canopy_radius_m.min(MAX_CANOPY_RADIUS_M), voxel_size) + 3;
    let tall = voxels_in(tree.trunk_height_m.1 + tree.canopy_radius_m, voxel_size) + 3;
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
            if below != AIR && plants.of(below).is_none() {
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
    use crate::plant::{CountRange, Fruit, Needs, Stage, Yield};
    use crate::shape::ShapeTable;
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
                harvest_to: 0,
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
}
