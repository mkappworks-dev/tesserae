//! Plants in generation: a Biome's surface feature may name a Plant, and a tree is whole from any
//! Chunk's viewpoint.

#![allow(
    clippy::disallowed_methods,
    reason = "a test's own maths is not generation"
)]

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use tesserae_core::plant::tree::{Part, fruit_stage, tree_voxels, vine_voxels};
use tesserae_core::plant::{Branching, CountRange, Fruit, Stage, Tree};
use tesserae_core::save::{Generation, Mapping};
use tesserae_core::testgen::fixtures;
use tesserae_core::worldgen::WaterParams;
use tesserae_core::{
    AIR, Biome, BiomeName, BiomeTable, CHUNK_SIZE, Chunk, ClimateParams, Clump, MAX_SURFACE_STAGES,
    MaterialId, SurfaceFeature, SurfaceFeatures, SurfaceFruit, SurfacePlant, SurfacePlantKind,
    SurfaceTree, SurfaceVine, SurfaceVines, Voxel, Worldgen, WorldgenParams,
};

const TRUNK: MaterialId = 210;
const CANOPY: MaterialId = 211;
const FRUIT: [MaterialId; 3] = [212, 213, 214];
const CROP: [MaterialId; 3] = [220, 221, 222];
const CANE: MaterialId = 223;
const KELP: MaterialId = 224;
const VINE: MaterialId = 225;
const VINE_SHAPE: u8 = 8;
const VINE_LENGTH_M: f32 = 6.0;
const VINE_FACE_BITS: [u8; 4] = [2, 7, 11, 18];
const MELON_VINE: MaterialId = 230;
const MELON: [MaterialId; 2] = [231, 232];
const CROSS_SHAPE: u8 = 5;
const FRUIT_SHAPE: u8 = 6;
const SMALL_FRUIT_SHAPE: u8 = 9;
const STAGE_SHAPES: [Option<u8>; 4] = [Some(SMALL_FRUIT_SHAPE), None, Some(FRUIT_SHAPE), None];

mod common;

const SEED: u64 = 0x7e55_e2ae_0000_0001;
const SIZES: [f32; 3] = [1.0, 0.5, 0.25];
const SIZE_I: i32 = CHUNK_SIZE as i32;

fn folded(seed: u64) -> u64 {
    u64::from((seed ^ (seed >> 32)) as u32)
}

/// Forks, leans and carries limbs: every part of the shape, 6.7 m from the foot at most.
const BRANCHING: Branching = Branching {
    limbs: CountRange { min: 3, max: 4 },
    start: 0.3,
    reach_m: 3.0,
    rise: 0.2,
    fork: Some(0.3),
    lean_m: 1.0,
    tiers: false,
    lobe_radius_m: 1.2,
};

fn oak_with(fruit: bool, branching: Option<Branching>) -> SurfaceTree {
    SurfaceTree {
        storeys_index: 0,
        canopies_index: 0,
        branching,
        crowding_m: None,
        trunk: TRUNK,
        canopy: CANOPY,
        trunk_height_m: (3.0, 5.0),
        canopy_radius_m: 2.5,
        vines: None,
        fruit: fruit.then_some(SurfaceFruit {
            stages: [FRUIT[0], FRUIT[1], FRUIT[2], AIR],
            len: 3,
            density: 0.2,
            shape: FRUIT_SHAPE,
            stage_shapes: [None; 4],
        }),
    }
}

fn plant(kind: SurfacePlantKind, first: MaterialId, spacing_m: f32) -> SurfaceFeature {
    SurfaceFeature {
        material: first,
        spacing_m,
        plant: Some(SurfacePlant {
            shape: CROSS_SHAPE,
            kind,
        }),
        clump: None,
        field: None,
    }
}

fn tree_feature(spacing_m: f32, fruit: bool) -> SurfaceFeature {
    branching_feature(spacing_m, fruit, None)
}

fn branching_feature(spacing_m: f32, fruit: bool, branching: Option<Branching>) -> SurfaceFeature {
    let mut f = plant(
        SurfacePlantKind::Tree(oak_with(fruit, branching)),
        TRUNK,
        spacing_m,
    );
    f.plant.as_mut().unwrap().shape = 0;
    f
}

fn crop_feature(spacing_m: f32) -> SurfaceFeature {
    let mut materials = [AIR; MAX_SURFACE_STAGES];
    materials[..3].copy_from_slice(&CROP);
    plant(
        SurfacePlantKind::Stages {
            materials,
            len: 3,
            stage_shapes: [None; MAX_SURFACE_STAGES],
        },
        CROP[0],
        spacing_m,
    )
}

fn biome(features: &[SurfaceFeature]) -> Biome {
    Biome {
        name: BiomeName::new("t:plain").unwrap(),
        temperature: 0.5,
        humidity: 0.5,
        ground: None,
        sky_fog: None,
        cave_scale: None,
        surface_features: SurfaceFeatures::new(features).unwrap(),
    }
}

/// Low land, so the scans at the origin hold the ground.
fn flat(features: &[SurfaceFeature]) -> WorldgenParams {
    flat_with(features, &tesserae_core::TreeVariety::NONE)
}

/// [`flat`] whose trees name storeys and canopy variants `variety` holds.
fn flat_with(features: &[SurfaceFeature], variety: &tesserae_core::TreeVariety) -> WorldgenParams {
    common::lowland(WorldgenParams {
        caves: None,
        smoothing: None,
        climate: ClimateParams {
            temperature_size_m: 64.0,
            humidity_size_m: 64.0,
        },
        biomes: BiomeTable::new_with_variety(&[biome(features)], variety).unwrap(),
        water: None,
        ..tesserae_core::testgen::world_params()
    })
}

/// A World generated twice, with and without its Plants, read one cell at a time from whichever
/// Chunk holds it.
struct Probe {
    with: Worldgen,
    bare: Worldgen,
    chunks: RefCell<HashMap<[i32; 3], (Chunk, Chunk)>>,
}

impl Probe {
    fn new(seed: u64, size: f32, params: WorldgenParams) -> Self {
        Probe {
            with: Worldgen::new(seed, size, params),
            bare: Worldgen::new(seed, size, params.without_surface_features()),
            chunks: RefCell::default(),
        }
    }

    fn at(&self, cell: [i32; 3]) -> (Voxel, Voxel) {
        let chunk = cell.map(|c| c.div_euclid(SIZE_I));
        let local = cell.map(|c| c.rem_euclid(SIZE_I) as usize);
        let mut chunks = self.chunks.borrow_mut();
        let (with, bare) = chunks.entry(chunk).or_insert_with(|| {
            (
                self.with.chunk(chunk[0], chunk[1], chunk[2]),
                self.bare.chunk(chunk[0], chunk[1], chunk[2]),
            )
        });
        (
            with.get(local[0], local[1], local[2]),
            bare.get(local[0], local[1], local[2]),
        )
    }

    /// Every cell the Plants changed, in `half_m` of the origin.
    fn changed(&self, half_m: f32, reverse: bool) -> HashMap<[i32; 3], (Voxel, Voxel)> {
        let chunk_m = SIZE_I as f32 * self.with.voxel_size();
        let (low, high) = self.with.surface_bounds_m();
        let rows = (low / chunk_m).floor() as i32 - 1..=(high / chunk_m).ceil() as i32 + 1;
        let reach = (half_m / chunk_m).ceil() as i32;
        let mut coords: Vec<[i32; 3]> = rows
            .flat_map(|cy| {
                (-reach..reach).flat_map(move |cz| (-reach..reach).map(move |cx| [cx, cy, cz]))
            })
            .collect();
        if reverse {
            coords.reverse();
        }
        let mut out = HashMap::new();
        for c in coords {
            let (with, bare) = (
                self.with.chunk(c[0], c[1], c[2]),
                self.bare.chunk(c[0], c[1], c[2]),
            );
            for z in 0..CHUNK_SIZE {
                for y in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        let (w, b) = (with.get(x, y, z), bare.get(x, y, z));
                        if w != b {
                            let at = [
                                c[0] * SIZE_I + x as i32,
                                c[1] * SIZE_I + y as i32,
                                c[2] * SIZE_I + z as i32,
                            ];
                            out.insert(at, (w, b));
                        }
                    }
                }
            }
        }
        out
    }
}

fn row_of(fruit: bool, branching: Option<Branching>) -> Tree {
    let o = oak_with(fruit, branching);
    Tree {
        storeys: tesserae_core::plant::Storeys::NONE,
        vines: None,
        branching,
        crowding_m: None,
        trunk: o.trunk,
        canopy: tesserae_core::plant::Canopies::one(o.canopy),
        trunk_height_m: o.trunk_height_m,
        canopy_radius_m: o.canopy_radius_m,
        mean_seconds: 0.0,
        trunk_yield: Vec::new(),
        canopy_yield: Vec::new(),
        sapling_yield: CountRange { min: 1, max: 1 },
        fruit: o.fruit.map(|f| Fruit {
            stages: f.stages[..f.len as usize]
                .iter()
                .map(|&material| Stage {
                    material,
                    mean_seconds: None,
                    yields: Vec::new(),
                    shape: None,
                })
                .collect(),
            density: f.density,
            regrow_seconds: 120.0,
            shape: Some(f.shape),
        }),
    }
}

/// What a tree whose trunk stands on `base` must be, from the pure function the generator calls.
fn expected(
    seed: u64,
    size: f32,
    fruit: bool,
    branching: Option<Branching>,
    base: [i32; 3],
) -> Vec<([i32; 3], MaterialId, Part)> {
    tree_voxels(
        &row_of(fruit, branching),
        folded(seed),
        [base[0], 0, base[2]],
        size,
    )
    .into_iter()
    .map(|v| {
        let material = if v.part == Part::Fruit {
            FRUIT[fruit_stage(folded(seed), v.at, size, 3)]
        } else {
            v.material
        };
        ([v.at[0], v.at[1] + base[1], v.at[2]], material, v.part)
    })
    .collect()
}

fn is_tree_material(m: MaterialId) -> bool {
    m == TRUNK || m == CANOPY || FRUIT.contains(&m)
}

fn bases(changed: &HashMap<[i32; 3], (Voxel, Voxel)>, material: MaterialId) -> Vec<[i32; 3]> {
    let mut found: Vec<[i32; 3]> = changed
        .iter()
        .filter(|(at, (w, _))| {
            w.material() == material
                && changed
                    .get(&[at[0], at[1] - 1, at[2]])
                    .is_none_or(|(w, _)| w.material() != material)
        })
        .map(|(at, _)| *at)
        .collect();
    found.sort();
    found
}

/// The feet among the trunk cells with ground under them: a limb tip can rest on rising ground
/// and look like one, but its tree's own cells never all stand there. Each other candidate must
/// be a cell of a tree that does.
fn feet(
    probe: &Probe,
    changed: &HashMap<[i32; 3], (Voxel, Voxel)>,
    fruit: bool,
    branching: Option<Branching>,
    size: f32,
    candidates: Vec<[i32; 3]>,
) -> Vec<[i32; 3]> {
    let whole = |base: [i32; 3]| {
        expected(SEED, size, fruit, branching, base)
            .iter()
            .all(|&(at, ..)| {
                let (with, bare) = probe.at(at);
                !bare.is_air() || is_tree_material(with.material())
            })
    };
    let candidates: Vec<_> = candidates
        .into_iter()
        .filter(|b| !probe.at([b[0], b[1] - 1, b[2]]).1.is_air())
        .collect();
    let feet: Vec<_> = candidates.iter().copied().filter(|&b| whole(b)).collect();
    let owned: HashSet<_> = feet
        .iter()
        .flat_map(|&b| expected(SEED, size, fruit, branching, b))
        .map(|c| c.0)
        .collect();
    for c in &candidates {
        assert!(
            feet.contains(c)
                || owned.contains(c)
                || !changed.contains_key(c)
                // The scan stops at 64 m, so a limb there may belong to a foot beyond it.
                || [c[0], c[2]].iter().any(|&a| (a.abs() as f32 * size) > 64.0 - 9.0),
            "{size} m: trunk Voxel {c:?} belongs to no whole tree"
        );
    }
    feet
}

fn metres(cell: [i32; 3], size: f32) -> [f32; 3] {
    cell.map(|c| c as f32 * size)
}

/// Every tree, however many Chunks it spans, has every cell the pure function says and that was
/// Air before, at every Voxel size, and generating the Chunks in the other order changes nothing.
fn assert_whole(params: WorldgenParams, fruit: bool, sizes: &[f32]) {
    assert_whole_with(params, fruit, None, sizes);
}

fn assert_whole_with(
    params: WorldgenParams,
    fruit: bool,
    branching: Option<Branching>,
    sizes: &[f32],
) {
    let mut crossings = 0;
    let mut fruit_cells = 0;
    for &size in sizes {
        let probe = Probe::new(SEED, size, params);
        let half_m = 64.0;
        let changed = probe.changed(half_m, false);
        assert_eq!(changed, probe.changed(half_m, true), "{size} m: order");
        let mut trunks = bases(&changed, TRUNK);
        if branching.is_some() {
            trunks = feet(&probe, &changed, fruit, branching, size, trunks);
        }
        assert!(trunks.len() > 20, "{size} m: only {} trees", trunks.len());
        for base in trunks {
            let cells = expected(SEED, size, fruit, branching, base);
            let chunks: HashSet<_> = cells
                .iter()
                .map(|(at, ..)| at.map(|c| c.div_euclid(SIZE_I)))
                .collect();
            crossings += usize::from(chunks.len() > 1);
            for &(at, material, _) in &cells {
                let (with, bare) = probe.at(at);
                if bare.is_air() {
                    assert!(
                        is_tree_material(with.material()),
                        "{size} m: tree at {base:?} missing {at:?}, found {with:?}"
                    );
                    // A neighbouring tree may hold the cell first.
                    let _ = material;
                    let fruit_here = FRUIT.contains(&with.material());
                    assert_eq!(
                        with.shape(),
                        if fruit_here { FRUIT_SHAPE } else { 0 },
                        "{size} m: tree at {base:?} has the wrong Shape at {at:?}"
                    );
                    fruit_cells += usize::from(fruit_here);
                } else {
                    assert_eq!(with, bare, "{size} m: a tree replaced terrain at {at:?}");
                }
            }
            if let Some(b) = branching {
                let far = cells
                    .iter()
                    .map(|(at, ..)| {
                        let across = [at[0] - base[0], at[2] - base[2]];
                        ((across[0].pow(2) + across[1].pow(2)) as f32).sqrt() * size
                    })
                    .fold(0.0, f32::max);
                let reach = b.extent_m(2.5) + 4.0 * size;
                assert!(far <= reach, "{size} m: {far} m is past {reach} m");
                continue;
            }
            let trunk = cells.iter().filter(|c| c.2 == Part::Trunk).count() as f32 * size;
            assert!(
                (2.999..=5.001).contains(&trunk),
                "{size} m: trunk {trunk} m"
            );
            let top = cells.iter().rfind(|c| c.2 == Part::Trunk).unwrap().0;
            for (at, _, part) in &cells {
                if *part != Part::Trunk {
                    let across = ((at[0] - top[0]).pow(2) + (at[2] - top[2]).pow(2)) as f32;
                    assert!(across.sqrt() * size <= 2.5 + 1e-3, "{size} m: canopy wide");
                }
            }
            let _ = metres(base, size);
        }
    }
    assert!(crossings > 5, "{crossings} trees crossed a Chunk border");
    assert_eq!(fruit_cells > 0, fruit, "fruit voxels drawn: {fruit_cells}");
}

#[test]
fn a_tree_across_a_chunk_border_is_whole_at_every_voxel_size() {
    assert_whole(flat(&[tree_feature(12.0, false)]), false, &SIZES);
}

#[test]
fn a_tree_is_whole_over_caves_and_smoothed_ground() {
    let params = WorldgenParams {
        caves: Some(Default::default()),
        smoothing: Some(Default::default()),
        ..flat(&[tree_feature(12.0, false)])
    };
    assert_whole(params, false, &[1.0, 0.5]);
}

#[test]
fn fruit_is_drawn_at_hash_stages_inside_its_density_whichever_chunk_makes_it() {
    let params = flat(&[tree_feature(12.0, true)]);
    assert_whole(params, true, &[1.0, 0.5]);
    let probe = Probe::new(SEED, 1.0, params);
    let changed = probe.changed(64.0, false);
    let canopy: Vec<_> = changed
        .values()
        .map(|(w, _)| w.material())
        .filter(|&m| m == CANOPY || FRUIT.contains(&m))
        .collect();
    let fruit = canopy.iter().filter(|m| FRUIT.contains(m)).count() as f32;
    let share = fruit / canopy.len() as f32;
    assert!((0.1..0.3).contains(&share), "fruit share {share}");
    for stage in FRUIT {
        assert!(canopy.contains(&stage), "no fruit at {stage}");
    }
}

#[test]
fn fruit_is_its_box_shape_whole_across_chunk_borders_at_every_voxel_size() {
    assert_whole(flat(&[tree_feature(12.0, true)]), true, &SIZES);
}

#[test]
fn a_branching_tree_across_a_chunk_border_is_whole_at_every_voxel_size() {
    assert_whole_with(
        flat(&[branching_feature(14.0, false, Some(BRANCHING))]),
        false,
        Some(BRANCHING),
        &SIZES,
    );
}

#[test]
fn a_branching_fruit_tree_is_whole_over_caves_and_smoothed_ground() {
    let params = WorldgenParams {
        caves: Some(Default::default()),
        smoothing: Some(Default::default()),
        ..flat(&[branching_feature(14.0, true, Some(BRANCHING))])
    };
    assert_whole_with(params, true, Some(BRANCHING), &[1.0, 0.5]);
}

#[test]
fn a_tree_is_the_declared_metres_whatever_the_voxel_size() {
    // `assert_whole` checks every tree's trunk and canopy in metres at each size.
    assert_whole(flat(&[tree_feature(16.0, false)]), false, &SIZES);
}

#[test]
fn a_tree_stands_only_on_ground_the_features_rules_allow() {
    let params = WorldgenParams {
        caves: Some(Default::default()),
        smoothing: Some(Default::default()),
        ..flat(&[tree_feature(8.0, false), crop_feature(8.0)])
    };
    let probe = Probe::new(SEED, 1.0, params);
    let changed = probe.changed(96.0, false);
    let surface = params.ground.unwrap().surface;
    let (_, highest) = probe.with.surface_bounds_m();
    let band = params.sea_level_m + params.relief_m;
    assert!(highest > band, "a tree raises the bound");
    for base in bases(&changed, TRUNK) {
        let below = probe.at([base[0], base[1] - 1, base[2]]).1;
        assert_eq!(
            below,
            Voxel::cube(surface),
            "{base:?} stands on the surface"
        );
        assert!(base[1] as f32 <= band, "{base:?} inside the surface bound");
        assert!(probe.at(base).1.is_air());
    }
    for (at, (w, b)) in &changed {
        assert!(b.is_air(), "{at:?} replaced {b:?} with {w:?}");
    }
}

#[test]
fn a_world_without_a_plant_feature_is_bounded_as_before() {
    let plain = flat(&[SurfaceFeature::cube(MaterialId::from(200u16), 8.0)]);
    let bare = Worldgen::new(SEED, 1.0, plain.without_surface_features());
    assert_eq!(
        Worldgen::new(SEED, 1.0, plain).surface_bounds_m(),
        bare.surface_bounds_m()
    );
    let planted = Worldgen::new(SEED, 1.0, flat(&[tree_feature(8.0, false)]));
    assert!(planted.surface_bounds_m().1 > bare.surface_bounds_m().1);
}

#[test]
fn crops_stand_at_hash_drawn_stages_in_their_shape() {
    for size in [1.0, 0.5] {
        let probe = Probe::new(SEED, size, flat(&[crop_feature(8.0)]));
        let changed = probe.changed(64.0, false);
        let mut seen = HashSet::new();
        for (at, (w, b)) in &changed {
            assert!(CROP.contains(&w.material()), "{at:?}");
            assert_eq!(w.shape(), CROSS_SHAPE);
            assert!(b.is_air());
            assert!(!probe.at([at[0], at[1] - 1, at[2]]).1.is_air());
            seen.insert(w.material());
        }
        assert_eq!(seen.len(), 3, "{size} m: every Stage appears");
        let expected = 128.0f32 * 128.0 / 64.0;
        let found = changed.len() as f32;
        assert!(
            (found - expected).abs() < 4.0 * expected.sqrt(),
            "{size} m: {found} crops, expected about {expected}"
        );
    }
}

fn lake(level_m: f32) -> Option<WaterParams> {
    Some(WaterParams {
        level_m,
        material: fixtures().water,
    })
}

#[test]
fn a_column_stands_on_the_ground_up_to_its_height_in_metres() {
    let cane = plant(
        SurfacePlantKind::Column {
            material: CANE,
            max_height_m: 3.0,
            stands_in: None,
        },
        CANE,
        6.0,
    );
    for size in [1.0, 0.5, 0.25] {
        let probe = Probe::new(SEED, size, flat(&[cane]));
        let changed = probe.changed(48.0, false);
        let mut runs = HashSet::new();
        for base in bases(&changed, CANE) {
            let mut n = 0;
            while changed
                .get(&[base[0], base[1] + n, base[2]])
                .is_some_and(|(w, _)| w.material() == CANE)
            {
                n += 1;
            }
            assert!(n as f32 * size <= 3.0 + 1e-3, "{size} m: run of {n}");
            assert!(!probe.at([base[0], base[1] - 1, base[2]]).1.is_air());
            runs.insert(n);
        }
        assert!(runs.len() > 1, "{size} m: runs {runs:?}");
    }
}

#[test]
fn kelp_stands_in_the_lake_and_nothing_else_does() {
    let kelp = plant(
        SurfacePlantKind::Column {
            material: KELP,
            max_height_m: 4.0,
            stands_in: Some(fixtures().water),
        },
        KELP,
        6.0,
    );
    // The water is the sea, so the low land's floor is let down under it.
    let with_lake = |features: &[SurfaceFeature]| {
        let land = common::sea_floor(flat(features));
        WorldgenParams {
            water: lake(land.sea_level_m),
            ..land
        }
    };
    let level = flat(&[]).sea_level_m;
    for size in [1.0, 0.5] {
        let probe = Probe::new(SEED, size, with_lake(&[kelp]));
        let changed = probe.changed(64.0, false);
        assert!(!changed.is_empty(), "{size} m: no kelp");
        for (at, (w, b)) in &changed {
            assert_eq!(w.material(), KELP);
            assert_eq!(*b, Voxel::cube(fixtures().water), "{at:?} replaced water");
            assert!((at[1] as f32 * size) < level, "{at:?} above the lake");
        }
        for base in bases(&changed, KELP) {
            let floor = probe.at([base[0], base[1] - 1, base[2]]).1;
            assert!(!floor.is_air() && floor != Voxel::cube(fixtures().water));
        }
        // Trees, crops and the rest stay out of the water.
        let dry = Probe::new(
            SEED,
            size,
            with_lake(&[tree_feature(8.0, false), crop_feature(8.0)]),
        );
        for (at, (w, _)) in dry.changed(64.0, false) {
            if w.material() == TRUNK || CROP.contains(&w.material()) {
                assert!(at[1] as f32 * size >= level, "{at:?} stands under water");
            }
        }
    }
}

#[test]
fn the_coarse_levels_leave_plants_out() {
    let params = flat(&[tree_feature(8.0, true), crop_feature(8.0)]);
    let coarse = Worldgen::new(SEED, 2.0, params.without_surface_features());
    assert_eq!(
        coarse.surface_bounds_m(),
        Worldgen::new(SEED, 2.0, flat(&[])).surface_bounds_m()
    );
    for cy in 0..4 {
        for cx in -2..2 {
            let chunk = coarse.chunk(cx, cy, 0);
            for z in 0..CHUNK_SIZE {
                for y in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        let m = chunk.get(x, y, z).material();
                        assert!(!is_tree_material(m) && !CROP.contains(&m));
                    }
                }
            }
        }
    }
}

fn mapping() -> Mapping {
    let f = fixtures();
    let named = [
        (0, "core:air"),
        (f.ground.rock, "t:rock"),
        (f.ground.soil, "t:soil"),
        (f.ground.surface, "t:surface"),
        (f.water, "t:water"),
        (TRUNK, "t:trunk"),
        (CANOPY, "t:canopy"),
        (FRUIT[0], "t:apple0"),
        (FRUIT[1], "t:apple1"),
        (FRUIT[2], "t:apple2"),
        (CROP[0], "t:wheat0"),
        (CROP[1], "t:wheat1"),
        (CROP[2], "t:wheat2"),
        (CANE, "t:cane"),
        (KELP, "t:kelp"),
        (MELON_VINE, "t:melon_vine"),
        (MELON[0], "t:melon0"),
        (MELON[1], "t:melon1"),
    ];
    Mapping {
        materials: named.map(|(id, n)| (id, n.to_owned())).into(),
        shapes: [
            (0, "core:cube".to_owned()),
            (CROSS_SHAPE, "t:cross".to_owned()),
            (FRUIT_SHAPE, "t:apple".to_owned()),
            (SMALL_FRUIT_SHAPE, "t:apple_small".to_owned()),
        ]
        .into(),
    }
}

#[test]
fn a_snapshot_freezes_what_generation_needs_of_each_plant() {
    let kelp = plant(
        SurfacePlantKind::Column {
            material: KELP,
            max_height_m: 4.0,
            stands_in: Some(fixtures().water),
        },
        KELP,
        6.0,
    );
    let params = WorldgenParams {
        water: lake(40.0),
        ..flat(&[tree_feature(16.0, true), crop_feature(16.0), kelp])
    };
    let saved = Generation::capture(params, &[], &[], &mapping()).unwrap();
    let json = serde_json::to_string(&saved).unwrap();
    let reopened: Generation = serde_json::from_str(&json).unwrap();
    let restored = reopened.restore(&mapping(), 3).unwrap();
    assert_eq!(restored.biomes, params.biomes);
    let (a, b) = (
        Worldgen::new(SEED, 1.0, params),
        Worldgen::new(SEED, 1.0, restored),
    );
    for cy in 0..3 {
        for cz in -1..1 {
            for cx in -1..1 {
                let (x, y) = (a.chunk(cx, cy, cz), b.chunk(cx, cy, cz));
                for i in 0..CHUNK_SIZE.pow(3) {
                    let at = (
                        i % CHUNK_SIZE,
                        i / CHUNK_SIZE % CHUNK_SIZE,
                        i / CHUNK_SIZE.pow(2),
                    );
                    assert_eq!(x.get(at.0, at.1, at.2), y.get(at.0, at.1, at.2));
                }
            }
        }
    }
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    let features = &value["biomes"][0]["surface_features"];
    assert_eq!(features[0]["plant"]["form"]["kind"], "tree");
    assert_eq!(features[0]["plant"]["form"]["trunk"], "t:trunk");
    assert_eq!(features[0]["plant"]["form"]["fruit"]["shape"], "t:apple");
}

/// Rows that grow the same storeys and variants name one entry of the World's table, a row with
/// one canopy and its own band names none, and a snapshot reopens to the same table.
#[test]
fn tree_rows_share_the_worlds_storeys_and_variants_and_a_snapshot_keeps_them() {
    use tesserae_core::plant::{Canopies, Storey, Storeys};
    const DARK: MaterialId = 216;
    let storey = |weight, trunk_height_m, canopy_radius_m| Storey {
        weight,
        trunk_height_m,
        canopy_radius_m,
    };
    let storeys = Storeys::new(&[storey(0.4, (5.0, 6.0), 3.0), storey(0.6, (2.0, 3.0), 2.0)]);
    let (storeys, canopies) = (storeys.unwrap(), Canopies::new(&[CANOPY, DARK]).unwrap());
    let mut variety = tesserae_core::TreeVariety::NONE;
    let mut storeyed = |spacing_m| {
        let mut f = tree_feature(spacing_m, false);
        if let Some(SurfacePlant {
            kind: SurfacePlantKind::Tree(t),
            ..
        }) = &mut f.plant
        {
            *t = variety.freeze(*t, &canopies, &storeys).unwrap();
        }
        f
    };
    let features = [storeyed(8.0), storeyed(12.0), tree_feature(16.0, false)];
    let params = flat_with(&features, &variety);
    let held = params.biomes.variety();
    let rows: Vec<_> = params.biomes.as_slice()[0]
        .surface_features
        .as_slice()
        .iter()
        .map(|f| match f.plant.unwrap().kind {
            SurfacePlantKind::Tree(t) => (
                t.canopies_index,
                t.storeys_index,
                t.canopies(held),
                *t.storeys(held),
            ),
            _ => unreachable!("only trees"),
        })
        .collect();
    let plain = (0, 0, Canopies::one(CANOPY), Storeys::NONE);
    assert_eq!(
        rows,
        [(1, 1, canopies, storeys), (1, 1, canopies, storeys), plain]
    );

    let mut mapping = mapping();
    mapping.materials.insert(DARK, "t:canopy_dark".into());
    let saved = Generation::capture(params, &[], &[], &mapping).unwrap();
    let json = serde_json::to_string(&saved).unwrap();
    let reopened: Generation = serde_json::from_str(&json).unwrap();
    assert_eq!(reopened.restore(&mapping, 3).unwrap().biomes, params.biomes);
    assert!(
        BiomeTable::new(params.biomes.as_slice()).is_err(),
        "rows naming lists the World does not hold"
    );
}

#[test]
fn a_snapshot_without_a_fruit_shape_restores_cubes() {
    let saved = r#"{"stages":["t:apple0"],"density":0.1}"#;
    let fruit: tesserae_core::save::SavedFruit = serde_json::from_str(saved).unwrap();
    assert_eq!(fruit.shape, None);
    assert!(!serde_json::to_string(&fruit).unwrap().contains("shape"));
}

#[test]
fn a_snapshot_of_material_features_writes_no_plant() {
    let params = flat(&[SurfaceFeature::cube(fixtures().leaves, 8.0)]);
    let saved = Generation::capture(params, &[], &[], &mapping_with_leaves()).unwrap();
    let json = serde_json::to_string(&saved).unwrap();
    assert!(!json.contains("plant"), "{json}");
}

fn mapping_with_leaves() -> Mapping {
    let mut m = mapping();
    m.materials.insert(fixtures().leaves, "t:leaves".into());
    m
}

fn vines() -> SurfaceVines {
    SurfaceVines {
        material: std::num::NonZeroU16::new(VINE).unwrap(),
        shape: VINE_SHAPE,
        orientations: VINE_FACE_BITS,
        density: 0.3,
        length_m: VINE_LENGTH_M,
    }
}

fn jungle_feature(spacing_m: f32) -> SurfaceFeature {
    let mut tree = tree_feature(spacing_m, false);
    let Some(SurfacePlant {
        kind: SurfacePlantKind::Tree(t),
        ..
    }) = tree.plant.as_mut()
    else {
        unreachable!()
    };
    t.vines = Some(vines());
    tree
}

fn is_vine(v: Voxel) -> bool {
    v.material() == VINE
}

/// Every vine the pure function hangs from each tree is there, from whichever Chunk is made first,
/// at three Voxel sizes, and no longer in metres than declared.
#[test]
fn vines_are_whole_across_chunk_borders_at_every_voxel_size() {
    let params = flat(&[jungle_feature(14.0)]);
    let mut crossings = 0;
    for size in SIZES {
        let probe = Probe::new(SEED, size, params);
        let changed = probe.changed(64.0, false);
        assert_eq!(changed, probe.changed(64.0, true), "{size} m: order");
        let (mut hung, mut total_m, mut runs) = (0, 0.0, 0);
        for base in bases(&changed, TRUNK) {
            let unshifted = tree_voxels(
                &row_of(false, None),
                folded(SEED),
                [base[0], 0, base[2]],
                size,
            );
            for vine in vine_voxels(&unshifted, &vines(), folded(SEED), size) {
                let cells: Vec<[i32; 3]> =
                    vine.cells().map(|c| [c[0], c[1] + base[1], c[2]]).collect();
                let chunks: HashSet<_> = cells
                    .iter()
                    .map(|c| c.map(|v| v.div_euclid(SIZE_I)))
                    .collect();
                crossings += usize::from(chunks.len() > 1);
                let mut standing = 0;
                for at in cells {
                    let (with, bare) = probe.at(at);
                    if !bare.is_air() {
                        break;
                    }
                    if is_tree_material(with.material()) {
                        continue;
                    }
                    assert!(
                        is_vine(with),
                        "{size} m: vine at {at:?} missing, found {with:?}"
                    );
                    assert_eq!(with.shape(), VINE_SHAPE);
                    assert!(VINE_FACE_BITS.contains(&with.orientation()), "{with:?}");
                    standing += 1;
                }
                hung += usize::from(standing > 0);
                total_m += standing as f32 * size;
                runs += 1;
                assert!(
                    standing as f32 * size <= VINE_LENGTH_M + 1e-3,
                    "{size} m: too long"
                );
            }
        }
        assert!(hung > 20, "{size} m: only {hung} vines");
        let mean = total_m / runs as f32;
        assert!((1.0..5.0).contains(&mean), "{size} m: mean vine {mean} m");
        let in_world = changed.values().filter(|(w, _)| is_vine(*w)).count();
        assert!(in_world > 0, "{size} m");
    }
    assert!(crossings > 2, "{crossings} vines crossed a Chunk border");
}

#[test]
fn a_world_without_vines_generates_exactly_the_trees_it_did() {
    let with = flat(&[tree_feature(14.0, true)]);
    let probe = Probe::new(SEED, 1.0, with);
    let changed = probe.changed(64.0, false);
    assert!(changed.values().all(|(w, _)| !is_vine(*w)));
    assert!(!changed.is_empty());
}

fn cube_features(spacing_m: f32, count: usize) -> Vec<SurfaceFeature> {
    (0..count as MaterialId)
        .map(|n| SurfaceFeature::cube(230 + n, spacing_m))
        .collect()
}

#[test]
fn a_biome_declares_sixteen_features_and_seventeen_is_refused() {
    assert!(SurfaceFeatures::new(&cube_features(8.0, 16)).is_ok());
    let err = SurfaceFeatures::new(&cube_features(8.0, 17)).unwrap_err();
    assert!(err.to_string().contains("at most 16"), "{err}");
    assert_eq!(tesserae_core::MAX_SURFACE_FEATURES, 16);
}

#[test]
fn sixteen_features_each_stand_in_proportion_to_their_spacing() {
    let params = flat(&cube_features(8.0, 16));
    let probe = Probe::new(SEED, 1.0, params);
    let changed = probe.changed(96.0, false);
    assert_eq!(changed, probe.changed(96.0, true));
    let mut counts = [0usize; 16];
    for (with, _) in changed.values() {
        if (230..246).contains(&with.material()) {
            counts[(with.material() - 230) as usize] += 1;
        }
    }
    let mean = counts.iter().sum::<usize>() as f32 / 16.0;
    for (n, c) in counts.iter().enumerate() {
        assert!(
            (*c as f32 - mean).abs() < mean * 0.35,
            "feature {n} stands {c} times, mean {mean}"
        );
    }
}

/// Chances laid end to end past 1 leave the last features no draw: the earlier declaration takes
/// the column, the same from every Chunk.
#[test]
fn chances_past_one_column_favour_the_earlier_features_deterministically() {
    let params = flat(&cube_features(4.0, 16));
    let probe = Probe::new(SEED, 2.0, params);
    let changed = probe.changed(96.0, false);
    assert_eq!(changed, probe.changed(96.0, true));
    let kinds: HashSet<_> = changed
        .values()
        .map(|(w, _)| w.material())
        .filter(|m| (230..246).contains(m))
        .collect();
    assert_eq!(kinds, (230..234).collect::<HashSet<_>>());
}

#[test]
fn a_snapshot_keeps_sixteen_features_and_a_trees_vines() {
    let mut features = cube_features(24.0, 13);
    features.extend([
        jungle_feature(16.0),
        tree_feature(20.0, true),
        crop_feature(18.0),
    ]);
    let params = flat(&features);
    let mut mapping = mapping();
    for n in 0..13 {
        mapping.materials.insert(230 + n, format!("t:feature{n}"));
    }
    mapping.materials.insert(VINE, "t:vine".into());
    mapping.shapes.insert(VINE_SHAPE, "t:vine_plane".into());
    let saved = Generation::capture(params, &[], &[], &mapping).unwrap();
    let json = serde_json::to_string(&saved).unwrap();
    let restored = serde_json::from_str::<Generation>(&json)
        .unwrap()
        .restore(&mapping, 3)
        .unwrap();
    assert_eq!(restored.biomes, params.biomes);
    assert_eq!(
        restored.biomes.as_slice()[0]
            .surface_features
            .as_slice()
            .len(),
        16
    );
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    let vines = &value["biomes"][0]["surface_features"][13]["plant"]["form"]["vines"];
    assert_eq!(vines["material"], "t:vine");
    assert_eq!(vines["shape"], "t:vine_plane");
    assert_eq!(vines["orientations"], serde_json::json!(VINE_FACE_BITS));
    assert!(
        value["biomes"][0]["surface_features"][14]["plant"]["form"]
            .get("vines")
            .is_none(),
        "a tree without vines writes none"
    );
    let (a, b) = (
        Worldgen::new(SEED, 1.0, params),
        Worldgen::new(SEED, 1.0, restored),
    );
    for cz in -1..1 {
        for cx in -1..1 {
            for cy in 0..3 {
                let (x, y) = (a.chunk(cx, cy, cz), b.chunk(cx, cy, cz));
                for i in 0..CHUNK_SIZE.pow(3) {
                    let at = (
                        i % CHUNK_SIZE,
                        i / CHUNK_SIZE % CHUNK_SIZE,
                        i / CHUNK_SIZE.pow(2),
                    );
                    assert_eq!(x.get(at.0, at.1, at.2), y.get(at.0, at.1, at.2));
                }
            }
        }
    }
}

#[test]
fn a_snapshot_keeps_a_trees_branching_and_a_tree_without_it_saves_none() {
    let capture = |branching| {
        let params = flat(&[branching_feature(16.0, true, branching)]);
        let json =
            serde_json::to_string(&Generation::capture(params, &[], &[], &mapping()).unwrap())
                .unwrap();
        (params, json)
    };
    let (params, json) = capture(Some(BRANCHING));
    let reopened: Generation = serde_json::from_str(&json).unwrap();
    let restored = reopened.restore(&mapping(), 3).unwrap();
    assert_eq!(restored.biomes, params.biomes);
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    let form = &value["biomes"][0]["surface_features"][0]["plant"]["form"];
    assert_eq!(form["branching"]["limbs"], serde_json::json!([3, 4]));
    assert_eq!(form["branching"]["fork"], 0.3f32);
    let (_, plain) = capture(None);
    assert!(!plain.contains("branching"), "an old tree saves as it did");
}

const FEATURE_BASE: MaterialId = 230;

fn clumped(mut f: SurfaceFeature, scale_m: f32, strength: f32) -> SurfaceFeature {
    f.clump = Some(Clump { scale_m, strength });
    f
}

/// Where each of `count` cube features, from material 230 on, stands over `half_m` of the origin.
fn feature_columns(
    params: WorldgenParams,
    size: f32,
    half_m: f32,
    count: usize,
) -> Vec<Vec<[f32; 2]>> {
    let probe = Probe::new(SEED, size, params);
    let mut found = vec![Vec::new(); count];
    for (at, (with, _)) in probe.changed(half_m, false) {
        let n = with.material().wrapping_sub(FEATURE_BASE) as usize;
        if n < count {
            found[n].push([at[0] as f32 * size, at[2] as f32 * size]);
        }
    }
    found
}

/// The sides of the square a feature's columns fall in, counted per `cell_m` block.
fn block_counts(columns: &[[f32; 2]], half_m: f32, cell_m: f32) -> Vec<f64> {
    let across = (2.0 * half_m / cell_m) as usize;
    let mut counts = vec![0.0; across * across];
    for c in columns {
        let at = |v: f32| (((v + half_m) / cell_m) as usize).min(across - 1);
        counts[at(c[1]) * across + at(c[0])] += 1.0;
    }
    counts
}

fn mean_and_variance(v: &[f64]) -> (f64, f64) {
    let mean = v.iter().sum::<f64>() / v.len() as f64;
    (
        mean,
        v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / v.len() as f64,
    )
}

#[test]
fn a_clumps_factor_has_a_mean_of_one_over_the_plane() {
    for (scale_m, strength) in [
        (16.0, 0.3),
        (16.0, 1.0),
        (48.0, 0.8),
        (64.0, 0.5),
        (12.0, 0.95),
    ] {
        let clump = Clump { scale_m, strength };
        let (mut sum, mut n) = (0.0, 0.0);
        for seed in 0..8u32 {
            for i in 0..300 {
                for j in 0..300 {
                    let f = clump.factor(
                        seed.wrapping_mul(0x9e37_79b9),
                        i as f32 * 7.3,
                        j as f32 * 7.3,
                    );
                    assert!(
                        (1.0 - f64::from(strength) - 1e-6..=clump.max_factor() + 1e-6).contains(&f),
                        "{scale_m} {strength}: {f} past {}",
                        clump.max_factor()
                    );
                    sum += f;
                    n += 1.0;
                }
            }
        }
        let mean = sum / n;
        assert!(
            (mean - 1.0).abs() < 0.05,
            "{scale_m} m, {strength}: mean {mean}"
        );
    }
}

#[test]
fn an_ungrouped_features_mean_spacing_is_its_declared_spacing_at_every_voxel_size() {
    let spacings = [4.0, 5.0, 6.0, 8.0, 10.0];
    let features: Vec<_> = spacings
        .iter()
        .enumerate()
        .map(|(n, &sp)| SurfaceFeature::cube(FEATURE_BASE + n as MaterialId, sp))
        .collect();
    for size in SIZES {
        let half_m = if size < 0.5 { 48.0 } else { 96.0 };
        let found = feature_columns(flat(&features), size, half_m, spacings.len());
        for (columns, sp) in found.iter().zip(spacings) {
            let spacing = (f64::from(2.0 * half_m).powi(2) / columns.len() as f64).sqrt();
            assert!(
                (spacing / f64::from(sp) - 1.0).abs() < 0.15,
                "{size} m Voxels: declared {sp} m, measured {spacing:.2} m over {} columns",
                columns.len()
            );
        }
    }
}

#[test]
fn a_clumped_feature_keeps_its_mean_spacing_and_gathers_into_groves() {
    let plain = SurfaceFeature::cube(FEATURE_BASE, 5.0);
    let grove = clumped(SurfaceFeature::cube(FEATURE_BASE + 1, 5.0), 16.0, 0.9);
    for size in SIZES {
        let half_m = if size < 0.5 { 48.0 } else { 96.0 };
        let found = feature_columns(flat(&[plain, grove]), size, half_m, 2);
        let [even, gathered] = [&found[0], &found[1]];
        let expected = f64::from(2.0 * half_m).powi(2) / 25.0;
        assert!(
            (gathered.len() as f64 / expected - 1.0).abs() < 0.15,
            "{size} m: {} grove columns, expected {expected}",
            gathered.len()
        );
        let (mean_even, var_even) = mean_and_variance(&block_counts(even, half_m, 8.0));
        let (mean_grove, var_grove) = mean_and_variance(&block_counts(gathered, half_m, 8.0));
        assert!(
            (mean_grove / mean_even - 1.0).abs() < 0.2,
            "{size} m: {mean_grove} against {mean_even} a block"
        );
        assert!(
            var_grove > 2.0 * var_even,
            "{size} m: variance {var_grove} is not clearly past {var_even}"
        );
        let empty = block_counts(gathered, half_m, 8.0)
            .iter()
            .filter(|c| **c == 0.0)
            .count();
        let empty_even = block_counts(even, half_m, 8.0)
            .iter()
            .filter(|c| **c == 0.0)
            .count();
        assert!(
            empty > empty_even + 3,
            "{size} m: {empty} clearings against {empty_even}"
        );
    }
}

#[test]
fn a_strong_clump_packs_cover_into_patches_with_bare_ground_between() {
    let plain = SurfaceFeature::cube(FEATURE_BASE, 4.0);
    let patchy = clumped(SurfaceFeature::cube(FEATURE_BASE + 1, 4.0), 14.0, 0.95);
    for size in SIZES {
        let half_m = if size < 0.5 { 48.0 } else { 96.0 };
        let found = feature_columns(flat(&[plain, patchy]), size, half_m, 2);
        let blocks = |columns: &[[f32; 2]]| {
            let mut counts = block_counts(columns, half_m, 8.0);
            counts.sort_by(f64::total_cmp);
            counts
        };
        let (even, gathered) = (blocks(&found[0]), blocks(&found[1]));
        let mean = gathered.iter().sum::<f64>() / gathered.len() as f64;
        let bare = gathered.iter().filter(|c| **c < 0.25 * mean).count();
        let dense = gathered[gathered.len() * 9 / 10..].iter().sum::<f64>();
        let share = dense / gathered.iter().sum::<f64>();
        let bare_even = even.iter().filter(|c| **c < 0.25 * mean).count();
        println!(
            "{size}: {bare} of {} blocks bare against {bare_even}, densest tenth holds {share:.2}",
            gathered.len()
        );
        assert!(
            bare * 100 >= gathered.len() * 30 && bare > 3 * bare_even,
            "{size} m: {bare} of {} blocks bare",
            gathered.len()
        );
        assert!(share > 0.3, "{size} m: the densest tenth holds {share:.2}");
    }
}

#[test]
fn a_clump_without_strength_generates_exactly_what_no_clump_does() {
    let plain = flat(&[tree_feature(10.0, true), crop_feature(6.0)]);
    let inert = flat(&[
        clumped(tree_feature(10.0, true), 32.0, 0.0),
        clumped(crop_feature(6.0), 32.0, 0.0),
    ]);
    let (a, b) = (
        Worldgen::new(SEED, 1.0, plain),
        Worldgen::new(SEED, 1.0, inert),
    );
    for cy in 0..3 {
        for cz in -2..2 {
            for cx in -2..2 {
                let (x, y) = (a.chunk(cx, cy, cz), b.chunk(cx, cy, cz));
                for i in 0..CHUNK_SIZE.pow(3) {
                    let at = (
                        i % CHUNK_SIZE,
                        i / CHUNK_SIZE % CHUNK_SIZE,
                        i / CHUNK_SIZE.pow(2),
                    );
                    assert_eq!(x.get(at.0, at.1, at.2), y.get(at.0, at.1, at.2));
                }
            }
        }
    }
}

#[test]
fn a_clump_is_the_same_field_at_every_voxel_size() {
    let params = flat(&[clumped(SurfaceFeature::cube(FEATURE_BASE, 4.0), 24.0, 0.9)]);
    let counts: Vec<Vec<f64>> = [1.0, 0.5]
        .map(|size| {
            let found = feature_columns(params, size, 64.0, 1);
            block_counts(&found[0], 64.0, 32.0)
        })
        .into();
    // Groves sit in the same blocks whatever the Voxel size, so the two grids track each other.
    let (a, b) = (&counts[0], &counts[1]);
    let n = a.len() as f64;
    let (ma, mb) = (a.iter().sum::<f64>() / n, b.iter().sum::<f64>() / n);
    let cov: f64 = a.iter().zip(b).map(|(x, y)| (x - ma) * (y - mb)).sum();
    let var = |v: &[f64], m: f64| v.iter().map(|x| (x - m).powi(2)).sum::<f64>();
    let r = cov / (var(a, ma) * var(b, mb)).sqrt();
    assert!(r > 0.8, "block counts correlate {r}");
}

#[test]
fn a_clumped_tree_across_a_chunk_border_is_whole_at_every_voxel_size() {
    let params = flat(&[clumped(tree_feature(9.0, false), 24.0, 0.8)]);
    assert_whole(params, false, &SIZES);
}

#[test]
fn a_snapshot_carries_the_clump_and_an_older_one_loads_without() {
    let grove = clumped(crop_feature(16.0), 48.0, 0.7);
    let params = flat(&[grove, crop_feature(20.0)]);
    let saved = Generation::capture(params, &[], &[], &mapping()).unwrap();
    let json = serde_json::to_string(&saved).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    let features = &value["biomes"][0]["surface_features"];
    assert_eq!(features[0]["clump"]["scale_m"], 48.0);
    assert!(features[1].get("clump").is_none(), "{features}");
    let restored = serde_json::from_str::<Generation>(&json)
        .unwrap()
        .restore(&mapping(), 3)
        .unwrap();
    assert_eq!(restored.biomes, params.biomes);
    let older = json.replace(r#","clump":{"scale_m":48.0,"strength":0.7}"#, "");
    assert_ne!(older, json);
    let older: Generation = serde_json::from_str(&older).unwrap();
    let loaded = older.restore(&mapping(), 3).unwrap();
    assert!(
        loaded.biomes.as_slice()[0].surface_features.as_slice()[0]
            .clump
            .is_none()
    );
}

fn staged_fruit_feature(spacing_m: f32) -> SurfaceFeature {
    let mut f = tree_feature(spacing_m, true);
    let Some(SurfacePlantKind::Tree(tree)) = f.plant.as_mut().map(|p| &mut p.kind) else {
        unreachable!()
    };
    tree.fruit.as_mut().unwrap().stage_shapes = STAGE_SHAPES;
    f
}

#[test]
fn generated_fruit_takes_the_shape_of_the_stage_it_is_drawn_at() {
    let params = flat(&[staged_fruit_feature(12.0)]);
    for size in [1.0, 0.5] {
        let probe = Probe::new(SEED, size, params);
        let mut shapes = [HashSet::new(), HashSet::new(), HashSet::new()];
        for (voxel, _) in probe.changed(64.0, false).values() {
            if let Some(stage) = FRUIT.iter().position(|&m| m == voxel.material()) {
                shapes[stage].insert(voxel.shape());
            }
        }
        let [small, plain, full] = shapes;
        assert_eq!(
            small,
            HashSet::from([SMALL_FRUIT_SHAPE]),
            "{size} m: stage 0"
        );
        assert_eq!(
            plain,
            HashSet::from([FRUIT_SHAPE]),
            "{size} m: a Stage with none takes the fruit's"
        );
        assert_eq!(full, HashSet::from([FRUIT_SHAPE]), "{size} m: stage 2");
    }
}

#[test]
fn a_snapshot_keeps_each_fruit_stages_shape_and_an_older_one_loads_without() {
    let params = flat(&[staged_fruit_feature(16.0)]);
    let saved = Generation::capture(params, &[], &[], &mapping()).unwrap();
    let json = serde_json::to_string(&saved).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    let fruit = &value["biomes"][0]["surface_features"][0]["plant"]["form"]["fruit"];
    assert_eq!(
        fruit["stage_shapes"],
        serde_json::json!(["t:apple_small", null, "t:apple"])
    );
    let reopened: Generation = serde_json::from_str(&json).unwrap();
    let restored = reopened.restore(&mapping(), 3).unwrap();
    assert_eq!(restored.biomes, params.biomes);

    let plain =
        Generation::capture(flat(&[tree_feature(16.0, true)]), &[], &[], &mapping()).unwrap();
    let json = serde_json::to_string(&plain).unwrap();
    assert!(
        !json.contains("stage_shapes"),
        "none saved when no Stage declares one"
    );
    let older: tesserae_core::save::SavedFruit =
        serde_json::from_str(r#"{"stages":["t:apple0"],"density":0.1,"shape":"t:apple"}"#).unwrap();
    assert!(older.stage_shapes.is_empty());
}

fn staged_crop_feature(spacing_m: f32) -> SurfaceFeature {
    let mut f = crop_feature(spacing_m);
    let Some(SurfacePlantKind::Stages { stage_shapes, .. }) = f.plant.as_mut().map(|p| &mut p.kind)
    else {
        unreachable!()
    };
    stage_shapes[..3].copy_from_slice(&[Some(SMALL_FRUIT_SHAPE), None, Some(FRUIT_SHAPE)]);
    f
}

#[test]
fn a_generated_crop_takes_the_shape_of_the_stage_it_is_drawn_at() {
    let params = flat(&[staged_crop_feature(8.0)]);
    for size in [1.0, 0.5] {
        let probe = Probe::new(SEED, size, params);
        let mut shapes = [HashSet::new(), HashSet::new(), HashSet::new()];
        for (voxel, _) in probe.changed(64.0, false).values() {
            if let Some(stage) = CROP.iter().position(|&m| m == voxel.material()) {
                shapes[stage].insert(voxel.shape());
            }
        }
        let [seed, young, ripe] = shapes;
        assert_eq!(
            seed,
            HashSet::from([SMALL_FRUIT_SHAPE]),
            "{size} m: stage 0"
        );
        assert_eq!(
            young,
            HashSet::from([CROSS_SHAPE]),
            "{size} m: a Stage with none takes the Plant's"
        );
        assert_eq!(ripe, HashSet::from([FRUIT_SHAPE]), "{size} m: stage 2");
    }
}

#[test]
fn a_snapshot_keeps_each_crop_stages_shape_and_an_older_one_loads_without() {
    let params = flat(&[staged_crop_feature(16.0)]);
    let saved = Generation::capture(params, &[], &[], &mapping()).unwrap();
    let json = serde_json::to_string(&saved).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    let form = &value["biomes"][0]["surface_features"][0]["plant"]["form"];
    assert_eq!(
        form["stage_shapes"],
        serde_json::json!(["t:apple_small", null, "t:apple"])
    );
    let restored = serde_json::from_str::<Generation>(&json)
        .unwrap()
        .restore(&mapping(), 3)
        .unwrap();
    assert_eq!(restored.biomes, params.biomes);

    let plain = Generation::capture(flat(&[crop_feature(16.0)]), &[], &[], &mapping()).unwrap();
    assert!(
        !serde_json::to_string(&plain)
            .unwrap()
            .contains("stage_shapes"),
        "none saved when no Stage declares one"
    );
    let older: tesserae_core::save::SavedPlantKind =
        serde_json::from_str(r#"{"kind":"stages","materials":["t:wheat0"]}"#).unwrap();
    assert!(matches!(
        older,
        tesserae_core::save::SavedPlantKind::Stages { stage_shapes, .. } if stage_shapes.is_empty()
    ));
}

const VINE_REACH_M: f32 = 1.5;
const VINE_GAP_M: f32 = 0.5;
const VINE_FRUIT: u32 = 4;

fn melon_feature(spacing_m: f32) -> SurfaceFeature {
    plant(
        SurfacePlantKind::Vine(SurfaceVine {
            material: MELON_VINE,
            fruit: [MELON[0], MELON[1], AIR, AIR],
            len: 2,
            fruit_shape: FRUIT_SHAPE,
            stage_shapes: [Some(SMALL_FRUIT_SHAPE), None, None, None],
            reach_m: VINE_REACH_M,
            gap_m: VINE_GAP_M,
            max_fruit: VINE_FRUIT,
        }),
        MELON_VINE,
        spacing_m,
    )
}

fn horizontal_m(a: [i32; 3], b: [i32; 3], size: f32) -> f32 {
    (((a[0] - b[0]).pow(2) + (a[2] - b[2]).pow(2)) as f32).sqrt() * size
}

#[test]
fn a_generated_vine_bears_whole_apart_fruit_across_chunk_borders_at_every_voxel_size() {
    let mut crossings = 0;
    let mut bearing = 0;
    for size in SIZES {
        let probe = Probe::new(SEED, size, flat(&[melon_feature(24.0)]));
        let changed = probe.changed(64.0, false);
        assert_eq!(changed, probe.changed(64.0, true), "{size} m: order");
        let at_material = |m: &[MaterialId]| -> Vec<[i32; 3]> {
            let mut found: Vec<_> = changed
                .iter()
                .filter(|(_, (w, _))| m.contains(&w.material()))
                .map(|(at, _)| *at)
                .collect();
            found.sort();
            found
        };
        let vines = at_material(&[MELON_VINE]);
        let fruit = at_material(&MELON);
        assert!(vines.len() > 10, "{size} m: only {} vines", vines.len());
        let gap = (VINE_GAP_M / size).ceil().max(1.0) as i32;
        for &cell in &fruit {
            let (w, bare) = changed[&cell];
            assert!(
                bare.is_air(),
                "{size} m: fruit replaced terrain at {cell:?}"
            );
            let expected = if w.material() == MELON[0] {
                SMALL_FRUIT_SHAPE
            } else {
                FRUIT_SHAPE
            };
            assert_eq!(w.shape(), expected, "{size} m: {cell:?}");
            assert!(
                !probe.at([cell[0], cell[1] - 1, cell[2]]).1.is_air(),
                "{size} m: fruit at {cell:?} floats"
            );
        }
        for &vine in &vines {
            let (w, _) = changed[&vine];
            assert_eq!(w.shape(), CROSS_SHAPE);
            // Two vines within a double reach share fruit, so only a lone one is counted.
            let lone = vines
                .iter()
                .all(|&o| o == vine || horizontal_m(o, vine, size) > 2.0 * VINE_REACH_M + 1.0);
            if !lone {
                continue;
            }
            let mine: Vec<[i32; 3]> = fruit
                .iter()
                .copied()
                .filter(|&f| horizontal_m(f, vine, size) <= VINE_REACH_M + 1e-3)
                .collect();
            assert!(
                mine.len() <= VINE_FRUIT as usize,
                "{size} m: {} fruit",
                mine.len()
            );
            bearing += usize::from(!mine.is_empty());
            for (i, a) in mine.iter().enumerate() {
                crossings += usize::from(
                    a.map(|c| c.div_euclid(SIZE_I)) != vine.map(|c| c.div_euclid(SIZE_I)),
                );
                for b in &mine[i + 1..] {
                    let apart = (a[0] - b[0]).abs().max((a[2] - b[2]).abs());
                    assert!(apart > gap, "{size} m: {a:?} and {b:?} touch");
                }
            }
        }
        for &cell in fruit.iter().filter(|c| {
            let [x, _, z] = metres(**c, size);
            x.abs() < 62.0 && z.abs() < 62.0
        }) {
            assert!(
                vines
                    .iter()
                    .any(|&v| horizontal_m(cell, v, size) <= VINE_REACH_M + 1e-3),
                "{size} m: fruit at {cell:?} has no vine"
            );
        }
    }
    assert!(bearing > 20, "{bearing} vines bore fruit");
    assert!(
        crossings > 3,
        "{crossings} fruit crossed a Chunk border from their vine"
    );
}

#[test]
fn a_snapshot_carries_a_generated_vine() {
    let params = flat(&[melon_feature(16.0)]);
    let saved = Generation::capture(params, &[], &[], &mapping()).unwrap();
    let json = serde_json::to_string(&saved).unwrap();
    let reopened: Generation = serde_json::from_str(&json).unwrap();
    let restored = reopened.restore(&mapping(), 3).unwrap();
    assert_eq!(restored.biomes, params.biomes);
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    let form = &value["biomes"][0]["surface_features"][0]["plant"]["form"];
    assert_eq!(form["kind"], "vine");
    assert_eq!(form["fruit"][0], "t:melon0");
    assert_eq!(form["max_fruit"], VINE_FRUIT);
}

/// Where each trunk stands in the square of `half` Chunks round the origin: a trunk Voxel with
/// ground under it.
fn trunk_feet(g: &Worldgen, half: i32) -> Vec<[i32; 2]> {
    let (low, high) = g.surface_bounds_m();
    let chunk_m = CHUNK_SIZE as f32 * g.voxel_size();
    let rows = (low / chunk_m).floor() as i32 - 1..=(high / chunk_m).ceil() as i32;
    let mut feet = Vec::new();
    for cz in -half..half {
        for cx in -half..half {
            let column: Vec<Chunk> = rows.clone().map(|cy| g.chunk(cx, cy, cz)).collect();
            let at =
                |x: usize, y: usize, z: usize| column[y / CHUNK_SIZE].get(x, y % CHUNK_SIZE, z);
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let foot = (1..column.len() * CHUNK_SIZE).any(|y| {
                        at(x, y, z).material() == TRUNK && {
                            let under = at(x, y - 1, z);
                            !under.is_air() && !is_tree_material(under.material())
                        }
                    });
                    if foot {
                        let size = CHUNK_SIZE as i32;
                        feet.push([cx * size + x as i32, cz * size + z as i32]);
                    }
                }
            }
        }
    }
    feet
}

/// Dry ground under trees declared 3 m apart, which no crown allows.
fn grove(tree: SurfaceTree) -> WorldgenParams {
    let mut feature = tree_feature(3.0, false);
    feature.plant.as_mut().unwrap().kind = SurfacePlantKind::Tree(tree);
    WorldgenParams {
        sea_level_m: -40.0,
        landforms: tesserae_core::terrain::LandformParams {
            overhangs: false,
            ..Default::default()
        },
        ..flat(&[feature])
    }
}

/// The trunks' feet, each pair at least `apart_m` apart, and the closest pair's distance.
fn assert_feet_apart(g: &Worldgen, apart_m: f32) -> (usize, f32) {
    let size = g.voxel_size();
    let feet = trunk_feet(g, 2);
    let mut closest = f32::MAX;
    for (i, a) in feet.iter().enumerate() {
        for b in &feet[i + 1..] {
            let d = ((a[0] - b[0]) as f32).hypot((a[1] - b[1]) as f32) * size;
            assert!(
                d >= apart_m,
                "{size} m: trunks at {a:?} and {b:?} stand {d} m apart"
            );
            closest = closest.min(d);
        }
    }
    (feet.len(), closest)
}

#[test]
fn trees_declared_closer_than_their_crowns_stand_apart() {
    for size in [1.0, 0.5] {
        let g = Worldgen::new(SEED, size, grove(oak_with(false, None)));
        // Worldgen keeps one and a half crown radii between trunks.
        let crown_m = 1.5 * oak_with(false, None).canopy_radius_m;
        let (trees, _) = assert_feet_apart(&g, crown_m);
        assert!(trees > 40, "{size} m: only {trees} trees");
    }
}

#[test]
fn a_tree_that_declares_its_crowding_stands_that_close_and_no_closer() {
    for size in [1.0, 0.5] {
        let crowns = assert_feet_apart(
            &Worldgen::new(SEED, size, grove(oak_with(false, None))),
            0.0,
        );
        let close = SurfaceTree {
            crowding_m: Some(2.0),
            ..oak_with(false, None)
        };
        let (trees, closest) = assert_feet_apart(&Worldgen::new(SEED, size, grove(close)), 2.0);
        assert!(
            closest < 1.5 * close.canopy_radius_m && trees > crowns.0 * 3 / 2,
            "{size} m: {trees} trees against {}, the closest {closest} m apart",
            crowns.0
        );
    }
}

#[test]
fn a_snapshot_keeps_a_trees_crowding_and_a_tree_without_it_saves_none() {
    let capture = |crowding_m| {
        let params = grove(SurfaceTree {
            crowding_m,
            ..oak_with(false, None)
        });
        let json =
            serde_json::to_string(&Generation::capture(params, &[], &[], &mapping()).unwrap())
                .unwrap();
        (params, json)
    };
    let (params, json) = capture(Some(2.0));
    let reopened: Generation = serde_json::from_str(&json).unwrap();
    assert_eq!(
        reopened.restore(&mapping(), 3).unwrap().biomes,
        params.biomes
    );
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    let form = &value["biomes"][0]["surface_features"][0]["plant"]["form"];
    assert_eq!(form["crowding_m"], 2.0f32);
    let (_, plain) = capture(None);
    assert!(!plain.contains("crowding"), "an old tree saves as it did");
}

/// The draw a coarse level stands its trees from: every trunk and canopy cell level 0 grows in a
/// box, fruit as canopy, and nothing level 0 leaves out, over smoothed ground and across Chunks.
#[test]
fn the_tree_parts_drawn_in_a_box_are_the_trunk_and_canopy_voxels_level_zero_stands_there() {
    let params = WorldgenParams {
        smoothing: Some(Default::default()),
        ..flat(&[
            tree_feature(8.0, true),
            branching_feature(10.0, false, Some(BRANCHING)),
        ])
    };
    let probe = Probe::new(SEED, 1.0, params);
    let (low, high) = probe.with.surface_bounds_m();
    let min = [-48, (low.floor() as i32).div_euclid(SIZE_I) * SIZE_I, -40];
    let max = [
        56,
        (high.ceil() as i32).div_euclid(SIZE_I) * SIZE_I + SIZE_I,
        72,
    ];
    let mut level_zero = HashMap::new();
    for x in min[0]..max[0] {
        for z in min[2]..max[2] {
            for y in min[1]..max[1] {
                let (with, bare) = probe.at([x, y, z]);
                let m = with.material();
                if bare.is_air() && is_tree_material(m) {
                    level_zero.insert([x, y, z], m != TRUNK);
                }
            }
        }
    }
    let drawn: HashMap<[i32; 3], bool> = probe
        .with
        .tree_parts(min, max)
        .into_iter()
        .map(|p| {
            let material = if p.canopy { CANOPY } else { TRUNK };
            assert_eq!(p.material, material, "{:?}", p.at);
            (p.at, p.canopy)
        })
        .collect();
    assert!(level_zero.len() > 2000, "{} tree cells", level_zero.len());
    let missing: Vec<_> = level_zero
        .iter()
        .filter(|(at, c)| drawn.get(*at) != Some(c))
        .take(8)
        .collect();
    let extra: Vec<_> = drawn
        .iter()
        .filter(|(at, _)| !level_zero.contains_key(*at))
        .take(8)
        .collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "missing or wrong {missing:?}, extra {extra:?}"
    );
    assert_eq!(drawn.len(), level_zero.len());
}

/// A dense forest's columns draw its leaves, lifted by its trees' height; a sparse one's none.
#[test]
fn canopy_cover_takes_a_dense_forests_leaves_and_leaves_sparse_trees_out() {
    let columns: Vec<[i32; 2]> = (0..32)
        .flat_map(|x| (0..32).map(move |z| [x * 4 - 64, z * 4 - 64]))
        .collect();
    let cover = |spacing_m: f32| {
        let mut f = tree_feature(spacing_m, false);
        if let Some(SurfacePlant {
            kind: SurfacePlantKind::Tree(tree),
            ..
        }) = &mut f.plant
        {
            // Crowns kept 1.5 radii apart never close over half the ground.
            tree.crowding_m = Some(1.0);
        }
        let g = Worldgen::new(SEED, 1.0, flat(&[f]));
        g.canopy_cover(&g.canopy_shapes(), &columns)
    };
    let dense = cover(3.0);
    for canopy in dense.iter().flatten() {
        assert_eq!(canopy.material, CANOPY);
        assert!(canopy.cover > 0.5 && canopy.cover <= 1.0, "{canopy:?}");
        // A 3 to 5 m trunk under a 2.5 m crown.
        assert!((4.0..=9.0).contains(&canopy.top_m), "{canopy:?}");
    }
    assert!(
        dense.iter().all(Option::is_some),
        "a dense forest left bare"
    );
    assert!(
        cover(16.0).iter().all(Option::is_none),
        "sparse trees covered"
    );
}

/// A forest of canopy variants shows one per crown-sized patch, not one per column, drawn from
/// the column in metres, so the same at another Voxel size.
#[test]
fn canopy_cover_shows_one_variant_a_crown_alike_at_every_voxel_size() {
    let variants = [CANOPY, 216, 217];
    let cover = |size: f32| {
        let mut f = tree_feature(3.0, false);
        let mut variety = tesserae_core::TreeVariety::NONE;
        if let Some(SurfacePlant {
            kind: SurfacePlantKind::Tree(tree),
            ..
        }) = &mut f.plant
        {
            tree.crowding_m = Some(1.0);
            *tree = variety
                .freeze(
                    *tree,
                    &tesserae_core::plant::Canopies::new(&variants).unwrap(),
                    &tesserae_core::plant::Storeys::NONE,
                )
                .unwrap();
        }
        let g = Worldgen::new(SEED, size, flat_with(&[f], &variety));
        let per_m = (1.0 / size) as i32;
        let columns: Vec<[i32; 2]> = (0..64)
            .flat_map(|x| (0..16).map(move |z| [(x - 32) * per_m, (z * 4 - 32) * per_m]))
            .collect();
        g.canopy_cover(&g.canopy_shapes(), &columns)
            .into_iter()
            .map(|c| c.map(|c| c.material))
            .collect::<Vec<_>>()
    };
    let covered = cover(1.0);
    assert_eq!(cover(0.5), covered);
    let drawn: Vec<MaterialId> = covered.into_iter().map(|m| m.expect("covered")).collect();
    assert!(drawn.iter().all(|m| variants.contains(m)), "{drawn:?}");
    let shown: std::collections::HashSet<_> = drawn.iter().collect();
    assert!(shown.len() > 1, "every crown alike");
    // Columns a metre apart along x: a patch about a crown wide changes at few of them.
    let changes = drawn
        .chunks(16)
        .zip(drawn.chunks(16).skip(1))
        .flat_map(|(a, b)| a.iter().zip(b))
        .filter(|(a, b)| a != b)
        .count();
    assert!(
        changes * 3 < drawn.len(),
        "{changes} of {} change",
        drawn.len()
    );
}

/// A mixed forest's sparser columns show the next species' leaves where its trees cover at least
/// half what the dominant species' do, and each column's answer is its own, whatever is asked
/// beside it.
#[test]
fn canopy_cover_shades_a_mixed_forests_sparser_columns_by_the_next_species_share() {
    const OTHER: MaterialId = 215;
    let columns: Vec<[i32; 2]> = (0..16)
        .flat_map(|x| (0..16).map(move |z| [x * 4 - 32, z * 4 - 32]))
        .collect();
    let tree = |spacing_m: f32, canopy: MaterialId| {
        let mut f = tree_feature(spacing_m, false);
        if let Some(SurfacePlant {
            kind: SurfacePlantKind::Tree(tree),
            ..
        }) = &mut f.plant
        {
            tree.crowding_m = Some(1.0);
            tree.canopy = canopy;
        }
        f
    };
    let cover = |next_spacing_m: f32| {
        let g = Worldgen::new(
            SEED,
            1.0,
            flat(&[tree(3.0, CANOPY), tree(next_spacing_m, OTHER)]),
        );
        let shapes = g.canopy_shapes();
        let all = g.canopy_cover(&shapes, &columns);
        for (i, column) in columns.iter().enumerate().step_by(37) {
            assert_eq!(g.canopy_cover(&shapes, &[*column]), [all[i]], "{column:?}");
        }
        all
    };
    // As many trees, or about a quarter as many, as the first species.
    for (next_spacing_m, sparse) in [(3.5, OTHER), (6.0, CANOPY)] {
        let drawn = cover(next_spacing_m);
        assert!(
            drawn.iter().all(Option::is_some),
            "a dense forest left bare"
        );
        for canopy in drawn.iter().flatten() {
            assert_eq!(
                (canopy.material, canopy.sparse),
                (CANOPY, sparse),
                "{next_spacing_m} m: {canopy:?}"
            );
        }
    }
}
