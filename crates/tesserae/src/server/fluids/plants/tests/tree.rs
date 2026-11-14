//! Trees on the Farm rig: a sapling grows the whole parametric tree or none of
//! it, and its fruit ripens by the Sample tick.

use super::*;
use tesserae_core::plant::tree::{Part, tree_voxels};
use tesserae_core::plant::{Branching, CountRange, Fruit, Tree, Yield};

pub(super) const BASE: [i32; 3] = [16, CROP_Y, 16];

pub(super) struct Orchard {
    pub(super) tables: Tables,
    pub(super) sapling: MaterialId,
    trunk: MaterialId,
    fruit: [MaterialId; 2],
    placed: MaterialId,
    pub(super) tree: Tree,
    apple: tesserae_core::ShapeId,
    /// The unripe Stage's own, smaller Shape.
    small: tesserae_core::ShapeId,
}

fn orchard(fruit: Option<f32>) -> Orchard {
    let radius = if fruit.is_some() { 2.0 } else { 1.0 };
    orchard_of(fruit, (1.0, 2.0), radius, None)
}

pub(super) fn orchard_of(
    fruit: Option<f32>,
    trunk_height_m: (f32, f32),
    canopy_radius_m: f32,
    branching: Option<Branching>,
) -> Orchard {
    regrowing(fruit, trunk_height_m, canopy_radius_m, branching, 120.0)
}

pub(super) fn regrowing(
    fruit: Option<f32>,
    trunk_height_m: (f32, f32),
    canopy_radius_m: f32,
    branching: Option<Branching>,
    regrow_seconds: f32,
) -> Orchard {
    let mut tables = crate::server::fixture_tables();
    let apple = tables
        .shapes
        .register(tesserae_core::ShapeDef {
            boxes: vec![tesserae_core::NodeBox::new([4, 2, 4], [12, 12, 12])],
            ..tesserae_core::ShapeDef::default()
        })
        .unwrap();
    let small = tables
        .shapes
        .register(tesserae_core::ShapeDef {
            boxes: vec![tesserae_core::NodeBox::new([5, 5, 5], [11, 12, 11])],
            ..tesserae_core::ShapeDef::default()
        })
        .unwrap();
    let def = |occlusion| MaterialDef {
        emission: 0,
        dimming: 4,
        occlusion,
        render: RenderClass::Cutout,
        solid: true,
    };
    let mut material = |occlusion| tables.materials.register(def(occlusion)).unwrap();
    let [sapling, trunk, canopy] = [(); 3].map(|_| material(Occlusion::Never));
    let fruit_materials = [(); 2].map(|_| material(Occlusion::Never));
    let placed = material(Occlusion::Never);
    let every = Some(tesserae_core::plant::SAMPLE_INTERVAL_S);
    let tree = Tree {
        vines: None,
        branching,
        trunk,
        canopy,
        trunk_height_m,
        canopy_radius_m,
        mean_seconds: tesserae_core::plant::SAMPLE_INTERVAL_S,
        trunk_yield: vec![Yield {
            item: "t:log".into(),
            count: CountRange { min: 1, max: 1 },
        }],
        canopy_yield: Vec::new(),
        sapling_yield: CountRange { min: 1, max: 2 },
        fruit: fruit.map(|density| Fruit {
            stages: [every, None]
                .into_iter()
                .zip(fruit_materials)
                .enumerate()
                .map(|(n, (mean_seconds, material))| Stage {
                    material,
                    mean_seconds,
                    yields: Vec::new(),
                    shape: (n == 0).then_some(small),
                })
                .collect(),
            density,
            regrow_seconds,
            shape: Some(apple),
        }),
    };
    tables
        .plants
        .push(Plant {
            name: "oak".into(),
            seed: "t:sapling".into(),
            first: sapling,
            shape: None,
            needs: Needs {
                soil: vec![soil()],
                light_min: 8,
                light_max: 15,
                ..Needs::default()
            },
            kind: Kind::Tree(tree.clone()),
            sapling_yields: Vec::new(),
        })
        .unwrap();
    Orchard {
        tables,
        sapling,
        trunk,
        fruit: fruit_materials,
        placed,
        tree,
        apple,
        small,
    }
}

fn ground() -> Chunk {
    let mut chunk = Chunk::air();
    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            chunk.set(x, SOIL_Y, z, Voxel::cube(soil()));
        }
    }
    chunk
}

pub(super) fn planted(size: f32, o: &Orchard, bases: &[[i32; 3]]) -> Farm {
    let mut chunk = ground();
    for b in bases {
        chunk.set(
            b[0] as usize,
            b[1] as usize,
            b[2] as usize,
            Voxel::cube(o.sapling),
        );
    }
    Farm::open(size, o.tables.clone(), [o.sapling; 3], chunk)
}

fn at(farm: &Farm, cell: [i32; 3]) -> MaterialId {
    let v = VoxelCoord::of(cell);
    farm.store()
        .get(v.chunk)
        .unwrap()
        .get(v.x, v.y, v.z)
        .material()
}

/// Runs a step at a time until `done`, failing after `limit` steps.
fn until(farm: &mut Farm, limit: u64, done: impl Fn(&Farm) -> bool) -> bool {
    for _ in 0..limit {
        if done(farm) {
            return true;
        }
        farm.run(1);
    }
    done(farm)
}

fn is_whole(farm: &Farm, o: &Orchard, base: [i32; 3]) -> bool {
    tree_voxels(&o.tree, farm.seed(), base, farm.voxel_m)
        .iter()
        .all(|v| at(farm, v.at) == v.material)
}

fn trunks(farm: &Farm, o: &Orchard, bases: &[[i32; 3]]) -> usize {
    bases.iter().filter(|&&b| at(farm, b) == o.trunk).count()
}

impl Farm {
    fn seed(&self) -> u64 {
        SEED
    }
}

#[test]
fn a_sapling_with_room_grows_the_tree_of_its_metres_whole_at_every_voxel_size() {
    for size in SIZES {
        let o = orchard(None);
        let mut farm = planted(size, &o, &[BASE]);
        let mut sprouted = false;
        for _ in 0..40_000 {
            farm.run(1);
            if at(&farm, BASE) != o.sapling {
                sprouted = true;
                break;
            }
            let above = at(&farm, [BASE[0], BASE[1] + 1, BASE[2]]);
            assert_eq!(above, 0, "{size}: grown before the sapling changed");
        }
        assert!(sprouted, "{size}: the sapling never grew");
        assert!(is_whole(&farm, &o, BASE), "{size}: not the whole tree");
        let voxels = tree_voxels(&o.tree, SEED, BASE, size);
        let height = voxels.iter().filter(|v| v.part == Part::Trunk).count();
        let metres = height as f32 * size;
        assert!(
            (0.999..=2.001).contains(&metres),
            "{size}: trunk {metres} m"
        );
        let top = voxels.iter().rfind(|v| v.part == Part::Trunk).unwrap().at;
        let reach = voxels
            .iter()
            .map(|v| (v.at[0] - top[0]).abs())
            .max()
            .unwrap();
        assert!(
            (reach as f32 * size - 1.0).abs() <= size + 1e-3,
            "{size}: canopy reach {reach}"
        );
        assert!(
            farm.sim().plants.landed >= 1,
            "{size}: landed through the one World change"
        );
    }
}

#[test]
fn a_blocked_cell_keeps_the_sapling_and_it_grows_once_the_way_is_clear() {
    let o = orchard(None);
    let voxels = tree_voxels(&o.tree, SEED, BASE, 1.0);
    let blocker = voxels.last().unwrap().at;
    let mut farm = planted(1.0, &o, &[BASE]);
    farm.solid(blocker, Voxel::cube(rock()));
    farm.run(8000);
    assert_eq!(at(&farm, BASE), o.sapling, "stays a sapling");
    assert_eq!(
        voxels
            .iter()
            .filter(|v| v.at != blocker && at(&farm, v.at) != 0)
            .count(),
        1,
        "nothing of the tree landed but the sapling's own cell"
    );
    farm.solid(blocker, Voxel::AIR);
    assert!(until(&mut farm, 40_000, |f| at(f, BASE) == o.trunk));
    assert!(is_whole(&farm, &o, BASE));
}

#[test]
fn a_tree_may_grow_over_its_own_grown_materials_but_not_a_placed_look_alike() {
    let o = orchard(None);
    let voxels = tree_voxels(&o.tree, SEED, BASE, 1.0);
    let inside = voxels[1].at;
    let mut farm = planted(1.0, &o, &[BASE]);
    farm.solid(inside, Voxel::cube(o.placed));
    farm.run(6000);
    assert_eq!(at(&farm, BASE), o.sapling, "a placed log is a wall");
    farm.solid(inside, Voxel::cube(o.trunk));
    assert!(until(&mut farm, 40_000, |f| at(f, BASE) == o.trunk));
    assert!(is_whole(&farm, &o, BASE));
}

#[test]
fn trees_land_under_their_own_bound_and_a_deferred_one_lands_the_next_step() {
    let o = orchard(None);
    let bases: Vec<[i32; 3]> = (0..3)
        .flat_map(|i| (0..3).map(move |j| [4 + i * 11, CROP_Y, 4 + j * 11]))
        .collect();
    let mut farm = planted(1.0, &o, &bases);
    farm.sim_mut().plants.set_tree_bound(0);
    assert!(until(&mut farm, 60_000, |f| f.sim().plants.waiting() >= 3));
    assert_eq!(trunks(&farm, &o, &bases), 0, "past the bound nothing lands");
    farm.sim_mut().plants.set_tree_bound(1);
    farm.run(1);
    assert_eq!(trunks(&farm, &o, &bases), 1, "one tree this step");
    farm.run(1);
    assert_eq!(trunks(&farm, &o, &bases), 2, "a deferred one the next");
    for &b in &bases {
        if at(&farm, b) == o.trunk {
            assert!(is_whole(&farm, &o, b));
        }
    }
}

#[test]
fn a_tree_that_fails_to_land_halfway_puts_back_what_it_wrote() {
    use tesserae_core::plant::Change;
    let o = orchard(None);
    let mut farm = planted(1.0, &o, &[]);
    let tables = Arc::clone(&farm.app.world().resource::<ContentTables>().0);
    let outside = [CHUNK_SIZE as i32 + 3, CROP_Y, 2];
    let changes: Vec<Change> = [[2, CROP_Y, 2], [3, CROP_Y, 2], outside]
        .into_iter()
        .map(|at| Change {
            at,
            voxel: Voxel::cube(o.trunk),
        })
        .collect();
    farm.app
        .world_mut()
        .resource_scope(|world, mut store: Mut<ChunkStore>| {
            let mut meshes = world.resource_mut::<MeshState>();
            let mut batch = edit::Batch::default();
            let landed = super::super::land(&mut batch, &mut store, &mut meshes, &tables, &changes);
            batch.finish(&mut store, &mut meshes, &tables);
            assert!(!landed, "the third cell is in a Chunk nobody holds");
        });
    assert_eq!(at(&farm, [2, CROP_Y, 2]), 0);
    assert_eq!(at(&farm, [3, CROP_Y, 2]), 0);
}

#[test]
fn fruit_replaces_canopy_at_stage_zero_ripens_by_the_sample_tick_and_ripens_again_after_a_pick() {
    for size in SIZES {
        let o = orchard(Some(0.3));
        let mut farm = planted(size, &o, &[BASE]);
        assert!(until(&mut farm, 40_000, |f| at(f, BASE) == o.trunk));
        let fruit: Vec<[i32; 3]> = tree_voxels(&o.tree, SEED, BASE, size)
            .into_iter()
            .filter(|v| v.part == Part::Fruit)
            .map(|v| v.at)
            .collect();
        assert!(
            !fruit.is_empty(),
            "{size}: a tree of density 0.3 bears fruit"
        );
        assert!(
            fruit
                .iter()
                .all(|&c| at(&farm, c) == o.fruit[0] || at(&farm, c) == o.fruit[1]),
            "{size}: fruit sits where the pure function put it"
        );
        let ripe = |f: &Farm| fruit.iter().filter(|&&c| at(f, c) == o.fruit[1]).count();
        assert!(
            until(&mut farm, 60_000, |f| ripe(f) > 0),
            "{size}: no fruit ripened"
        );
        assert_eq!(at(&farm, BASE), o.trunk, "the tree stands");
        let picked = *fruit.iter().find(|&&c| at(&farm, c) == o.fruit[1]).unwrap();
        farm.solid(picked, Voxel::cube(o.fruit[0]));
        assert!(
            until(&mut farm, 60_000, |f| at(f, picked) == o.fruit[1]),
            "{size}: a picked fruit ripens again"
        );
    }
}

#[test]
fn fruit_waits_in_the_dark_and_ripens_once_light_reaches_it() {
    let o = orchard(Some(0.3));
    let (trunk, fruit) = ([10, CROP_Y, 10], [11, CROP_Y + 1, 10]);
    let roof_y = CROP_Y as usize + 3;
    let mut chunk = ground();
    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            chunk.set(x, roof_y, z, Voxel::cube(rock()));
        }
    }
    for dy in 0..2 {
        chunk.set(10, CROP_Y as usize + dy, 10, Voxel::cube(o.trunk));
    }
    chunk.set(11, CROP_Y as usize + 1, 10, Voxel::cube(o.fruit[0]));
    let mut farm = Farm::open(1.0, o.tables.clone(), [o.sapling; 3], chunk);
    farm.run(ENOUGH);
    assert_eq!(at(&farm, fruit), o.fruit[0], "held in the dark, not dead");
    assert_eq!(at(&farm, trunk), o.trunk);
    farm.solid([11, roof_y as i32, 10], Voxel::AIR);
    assert!(until(&mut farm, 60_000, |f| at(f, fruit) == o.fruit[1]));
}

#[test]
fn a_grown_tree_is_relit_as_a_whole_the_way_a_fresh_solve_lights_it() {
    let o = orchard(None);
    let mut farm = planted(1.0, &o, &[BASE]);
    assert!(until(&mut farm, 40_000, |f| at(f, BASE) == o.trunk));
    let mut chunk = ground();
    for v in tree_voxels(&o.tree, SEED, BASE, 1.0) {
        chunk.set(
            v.at[0] as usize,
            v.at[1] as usize,
            v.at[2] as usize,
            Voxel::cube(v.material),
        );
    }
    let fresh = Farm::open(1.0, o.tables.clone(), [o.sapling; 3], chunk);
    let mut shaded = 0;
    for x in 0..CHUNK_SIZE as i32 {
        for y in 0..CHUNK_SIZE as i32 {
            for z in 0..CHUNK_SIZE as i32 {
                let (a, b) = (
                    farm.store().light_at([x, y, z]),
                    fresh.store().light_at([x, y, z]),
                );
                assert_eq!(a, b, "light at {:?}", [x, y, z]);
                shaded += usize::from(a.sky() < 15);
            }
        }
    }
    assert!(shaded > 0, "the canopy shades something");
}

fn voxel_at(farm: &Farm, cell: [i32; 3]) -> Voxel {
    let v = VoxelCoord::of(cell);
    farm.store().get(v.chunk).unwrap().get(v.x, v.y, v.z)
}

#[test]
fn unripe_fruit_is_its_small_shape_ripe_fruit_the_full_one_and_trunk_and_canopy_stay_cubes() {
    for size in SIZES {
        let o = orchard(Some(0.3));
        let mut farm = planted(size, &o, &[BASE]);
        assert!(until(&mut farm, 40_000, |f| at(f, BASE) == o.trunk));
        let cells = tree_voxels(&o.tree, SEED, BASE, size);
        for v in &cells {
            let got = voxel_at(&farm, v.at);
            let want = match (v.part, got.material() == o.fruit[1]) {
                (Part::Fruit, true) => o.apple,
                (Part::Fruit, false) => o.small,
                _ => 0,
            };
            assert_eq!(got.shape(), want, "{size}: {:?} grown", v.part);
        }
        let ripe = |f: &Farm| {
            cells
                .iter()
                .filter(|v| voxel_at(f, v.at).material() == o.fruit[1])
                .count()
        };
        assert!(until(&mut farm, 60_000, |f| ripe(f) > 0), "{size}: no ripe");
        for v in cells.iter().filter(|v| v.part == Part::Fruit) {
            let held = voxel_at(&farm, v.at);
            let want = if held.material() == o.fruit[1] {
                o.apple
            } else {
                o.small
            };
            assert_eq!(held.shape(), want, "{size}: a Stage keeps its own Shape");
        }
    }
}

/// A short tree that leans, forks and carries limbs, small enough for one Chunk at 0.25 m.
fn branched() -> Orchard {
    orchard_of(
        Some(0.2),
        (2.0, 3.0),
        1.0,
        Some(Branching {
            limbs: CountRange { min: 2, max: 3 },
            start: 0.3,
            reach_m: 1.6,
            rise: 0.2,
            fork: Some(0.4),
            lean_m: 0.5,
            tiers: false,
            lobe_radius_m: 0.6,
        }),
    )
}

#[test]
fn a_branching_sapling_grows_its_limbs_and_lobes_whole_at_every_voxel_size() {
    for size in SIZES {
        let o = branched();
        let mut farm = planted(size, &o, &[BASE]);
        assert!(
            until(&mut farm, 60_000, |f| at(f, BASE) == o.trunk),
            "{size}: the sapling never grew"
        );
        assert!(is_whole(&farm, &o, BASE), "{size}: not the whole tree");
        let voxels = tree_voxels(&o.tree, SEED, BASE, size);
        let off_axis = voxels
            .iter()
            .filter(|v| v.part == Part::Trunk && (v.at[0], v.at[2]) != (BASE[0], BASE[2]))
            .count();
        assert!(off_axis > 3, "{size}: no limbs");
        assert!(
            voxels.iter().any(|v| v.part == Part::Fruit),
            "{size}: fruit on the lobes"
        );
        assert_eq!(farm.sim().plants.landed, 1, "{size}: one landing");
    }
}

#[test]
fn a_branching_tree_with_a_blocked_limb_cell_grows_nothing_and_grows_when_it_clears() {
    let o = branched();
    let voxels = tree_voxels(&o.tree, SEED, BASE, 0.5);
    let limb_tip = voxels
        .iter()
        .filter(|v| v.part == Part::Trunk)
        .max_by_key(|v| (v.at[0] - BASE[0]).abs() + (v.at[2] - BASE[2]).abs())
        .unwrap()
        .at;
    let mut farm = planted(0.5, &o, &[BASE]);
    farm.solid(limb_tip, Voxel::cube(rock()));
    farm.run(8000);
    assert_eq!(at(&farm, BASE), o.sapling, "stays a sapling");
    assert_eq!(
        voxels
            .iter()
            .filter(|v| v.at != limb_tip && at(&farm, v.at) != 0)
            .count(),
        1,
        "nothing but the sapling's own cell"
    );
    farm.solid(limb_tip, Voxel::AIR);
    assert!(until(&mut farm, 60_000, |f| at(f, BASE) == o.trunk));
    assert!(is_whole(&farm, &o, BASE));
}

#[test]
fn a_grown_branching_tree_is_relit_as_a_whole_the_way_a_fresh_solve_lights_it() {
    let o = branched();
    let mut farm = planted(0.5, &o, &[BASE]);
    assert!(until(&mut farm, 60_000, |f| at(f, BASE) == o.trunk));
    let mut chunk = ground();
    for v in tree_voxels(&o.tree, SEED, BASE, 0.5) {
        chunk.set(
            v.at[0] as usize,
            v.at[1] as usize,
            v.at[2] as usize,
            Voxel::cube(v.material),
        );
    }
    let fresh = Farm::open(0.5, o.tables.clone(), [o.sapling; 3], chunk);
    let mut shaded = 0;
    for x in 0..CHUNK_SIZE as i32 {
        for y in 0..CHUNK_SIZE as i32 {
            for z in 0..CHUNK_SIZE as i32 {
                let (a, b) = (
                    farm.store().light_at([x, y, z]),
                    fresh.store().light_at([x, y, z]),
                );
                assert_eq!(a, b, "light at {:?}", [x, y, z]);
                shaded += usize::from(a.sky() < 15);
            }
        }
    }
    assert!(shaded > 0, "the limbs and lobes shade something");
}

/// A grown tree at `BASE`, its fruit all canopy again, standing on a Farm.
fn bare_tree(size: f32, o: &Orchard) -> (Farm, Vec<[i32; 3]>) {
    let mut chunk = ground();
    let mut leaves = Vec::new();
    for v in tree_voxels(&o.tree, SEED, BASE, size) {
        let material = if v.part == Part::Trunk {
            v.material
        } else {
            o.tree.canopy
        };
        if v.part != Part::Trunk {
            leaves.push(v.at);
        }
        let [x, y, z] = v.at.map(|c| c as usize);
        chunk.set(x, y, z, Voxel::cube(material));
    }
    (
        Farm::open(size, o.tables.clone(), [o.sapling; 3], chunk),
        leaves,
    )
}

fn fruit_on(farm: &Farm, o: &Orchard, leaves: &[[i32; 3]]) -> usize {
    leaves
        .iter()
        .filter(|&&c| o.fruit.contains(&at(farm, c)))
        .count()
}

/// The most fruit a re-bearing tree holds: the cap, rounded up.
fn cap_of(o: &Orchard, size: f32) -> usize {
    use tesserae_core::plant::tree::{expected_canopy_cells, fruit_cap};
    let density = o.tree.fruit.as_ref().unwrap().density;
    fruit_cap(density, expected_canopy_cells(&o.tree, size)).ceil() as usize
}

#[test]
fn a_standing_fruit_tree_grows_new_fruit_over_time_and_stops_at_its_density_cap() {
    for size in [1.0, 0.5] {
        let o = regrowing(Some(0.1), (3.0, 4.0), 3.0, None, 16.0);
        let (mut farm, leaves) = bare_tree(size, &o);
        let cap = cap_of(&o, size);
        assert_eq!(fruit_on(&farm, &o, &leaves), 0);
        assert!(
            until(&mut farm, 300_000, |f| fruit_on(f, &o, &leaves) >= cap),
            "{size}: {} of {cap}",
            fruit_on(&farm, &o, &leaves)
        );
        farm.run(20_000);
        assert_eq!(fruit_on(&farm, &o, &leaves), cap, "{size}: held at the cap");
        assert_eq!(at(&farm, BASE), o.trunk, "the tree stands");
    }
}

#[test]
fn fruit_picked_from_a_full_tree_grows_back_elsewhere() {
    let o = regrowing(Some(0.1), (3.0, 4.0), 3.0, None, 16.0);
    let (mut farm, leaves) = bare_tree(1.0, &o);
    let cap = cap_of(&o, 1.0);
    assert!(until(&mut farm, 300_000, |f| fruit_on(f, &o, &leaves) >= cap));
    let picked = *leaves
        .iter()
        .find(|&&c| o.fruit.contains(&at(&farm, c)))
        .unwrap();
    farm.solid(picked, Voxel::cube(o.tree.canopy));
    assert_eq!(fruit_on(&farm, &o, &leaves), cap - 1);
    assert!(until(&mut farm, 300_000, |f| fruit_on(f, &o, &leaves) >= cap));
}

#[test]
fn a_tree_with_part_of_its_canopy_cut_still_bears_on_what_remains() {
    let o = regrowing(Some(0.1), (3.0, 4.0), 3.0, None, 16.0);
    let (mut farm, leaves) = bare_tree(1.0, &o);
    let top = leaves.iter().map(|c| c[1]).max().unwrap();
    for &cut in leaves.iter().filter(|c| c[1] == top) {
        farm.solid(cut, Voxel::AIR);
    }
    let remains: Vec<_> = leaves.iter().copied().filter(|c| c[1] != top).collect();
    assert!(until(&mut farm, 300_000, |f| fruit_on(f, &o, &remains) > 0));
    assert!(fruit_on(&farm, &o, &leaves) > 0);
}

#[test]
fn a_felled_tree_grows_no_fruit() {
    let o = regrowing(Some(0.1), (3.0, 4.0), 3.0, None, 16.0);
    let (mut farm, leaves) = bare_tree(1.0, &o);
    farm.solid(BASE, Voxel::AIR);
    assert!(until(&mut farm, 20_000, |f| {
        leaves.iter().all(|&c| at(f, c) == 0)
    }));
    farm.run(20_000);
    assert_eq!(fruit_on(&farm, &o, &leaves), 0);
}

#[test]
fn a_tree_in_the_dark_grows_no_fruit_until_light_reaches_it() {
    let o = regrowing(Some(0.1), (3.0, 4.0), 3.0, None, 16.0);
    let mut chunk = ground();
    let roof_y = 20usize;
    let mut leaves = Vec::new();
    for v in tree_voxels(&o.tree, SEED, BASE, 1.0) {
        let material = if v.part == Part::Trunk {
            v.material
        } else {
            o.tree.canopy
        };
        if v.part != Part::Trunk {
            leaves.push(v.at);
        }
        let [x, y, z] = v.at.map(|c| c as usize);
        chunk.set(x, y, z, Voxel::cube(material));
    }
    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            chunk.set(x, roof_y, z, Voxel::cube(rock()));
        }
    }
    let mut farm = Farm::open(1.0, o.tables.clone(), [o.sapling; 3], chunk);
    farm.run(30_000);
    assert_eq!(fruit_on(&farm, &o, &leaves), 0, "held in the dark");
    for z in 0..CHUNK_SIZE as i32 {
        for x in 0..CHUNK_SIZE as i32 {
            farm.solid([x, roof_y as i32, z], Voxel::AIR);
        }
    }
    assert!(until(&mut farm, 300_000, |f| fruit_on(f, &o, &leaves) > 0));
}
