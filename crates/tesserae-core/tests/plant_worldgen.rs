//! Plants in generation: a Biome's surface feature may name a Plant, and a tree is whole from any
//! Chunk's viewpoint.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use tesserae_core::plant::tree::{Part, fruit_stage, tree_voxels};
use tesserae_core::plant::{CountRange, Fruit, Stage, Tree};
use tesserae_core::save::{Generation, Mapping};
use tesserae_core::testgen::fixtures;
use tesserae_core::worldgen::WaterParams;
use tesserae_core::{
    AIR, Biome, BiomeName, BiomeTable, CHUNK_SIZE, Chunk, ClimateParams, MAX_SURFACE_STAGES,
    MaterialId, SurfaceFeature, SurfaceFeatures, SurfaceFruit, SurfacePlant, SurfacePlantKind,
    SurfaceTree, Voxel, Worldgen, WorldgenParams,
};

const TRUNK: MaterialId = 210;
const CANOPY: MaterialId = 211;
const FRUIT: [MaterialId; 3] = [212, 213, 214];
const CROP: [MaterialId; 3] = [220, 221, 222];
const CANE: MaterialId = 223;
const KELP: MaterialId = 224;
const CROSS_SHAPE: u8 = 5;

const SEED: u64 = 0x7e55_e2ae_0000_0001;
const SIZES: [f32; 3] = [1.0, 0.5, 0.25];
const SIZE_I: i32 = CHUNK_SIZE as i32;

fn folded(seed: u64) -> u64 {
    u64::from((seed ^ (seed >> 32)) as u32)
}

fn oak(fruit: bool) -> SurfaceTree {
    SurfaceTree {
        trunk: TRUNK,
        canopy: CANOPY,
        trunk_height_m: (3.0, 5.0),
        canopy_radius_m: 2.5,
        fruit: fruit.then_some(SurfaceFruit {
            stages: [FRUIT[0], FRUIT[1], FRUIT[2], AIR],
            len: 3,
            density: 0.2,
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
    }
}

fn tree_feature(spacing_m: f32, fruit: bool) -> SurfaceFeature {
    let mut f = plant(SurfacePlantKind::Tree(oak(fruit)), TRUNK, spacing_m);
    f.plant.as_mut().unwrap().shape = 0;
    f
}

fn crop_feature(spacing_m: f32) -> SurfaceFeature {
    let mut materials = [AIR; MAX_SURFACE_STAGES];
    materials[..3].copy_from_slice(&CROP);
    plant(
        SurfacePlantKind::Stages { materials, len: 3 },
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

fn flat(features: &[SurfaceFeature]) -> WorldgenParams {
    WorldgenParams {
        caves: None,
        smoothing: None,
        climate: ClimateParams {
            temperature_size_m: 64.0,
            humidity_size_m: 64.0,
        },
        biomes: BiomeTable::new(&[biome(features)]).unwrap(),
        ..tesserae_core::testgen::world_params()
    }
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

fn row_of(fruit: bool) -> Tree {
    let o = oak(fruit);
    Tree {
        trunk: o.trunk,
        canopy: o.canopy,
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
                })
                .collect(),
            density: f.density,
            harvest_to: 0,
        }),
    }
}

/// What a tree whose trunk stands on `base` must be, from the pure function the generator calls.
fn expected(
    seed: u64,
    size: f32,
    fruit: bool,
    base: [i32; 3],
) -> Vec<([i32; 3], MaterialId, Part)> {
    tree_voxels(&row_of(fruit), folded(seed), [base[0], 0, base[2]], size)
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

fn metres(cell: [i32; 3], size: f32) -> [f32; 3] {
    cell.map(|c| c as f32 * size)
}

/// Every tree, however many Chunks it spans, has every cell the pure function says and that was
/// Air before, at every Voxel size, and generating the Chunks in the other order changes nothing.
fn assert_whole(params: WorldgenParams, fruit: bool, sizes: &[f32]) {
    let mut crossings = 0;
    for &size in sizes {
        let probe = Probe::new(SEED, size, params);
        let half_m = 64.0;
        let changed = probe.changed(half_m, false);
        assert_eq!(changed, probe.changed(half_m, true), "{size} m: order");
        let trunks = bases(&changed, TRUNK);
        assert!(trunks.len() > 20, "{size} m: only {} trees", trunks.len());
        for base in trunks {
            let cells = expected(SEED, size, fruit, base);
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
                } else {
                    assert_eq!(with, bare, "{size} m: a tree replaced terrain at {at:?}");
                }
            }
            let trunk = cells.iter().filter(|c| c.2 == Part::Trunk).count() as f32 * size;
            assert!(
                (2.999..=5.001).contains(&trunk),
                "{size} m: trunk {trunk} m"
            );
            let top = cells
                .iter()
                .filter(|c| c.2 == Part::Trunk)
                .last()
                .unwrap()
                .0;
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
    let level = 56.0;
    for size in [1.0, 0.5] {
        let with_lake = |features: &[SurfaceFeature]| WorldgenParams {
            water: lake(level),
            ..flat(features)
        };
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
    ];
    Mapping {
        materials: named.map(|(id, n)| (id, n.to_owned())).into(),
        shapes: [
            (0, "core:cube".to_owned()),
            (CROSS_SHAPE, "t:cross".to_owned()),
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
    let saved = Generation::capture(params, &mapping()).unwrap();
    let json = serde_json::to_string(&saved).unwrap();
    let reopened: Generation = serde_json::from_str(&json).unwrap();
    let restored = reopened.restore(&mapping(), 2).unwrap();
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
}

#[test]
fn a_snapshot_of_material_features_writes_no_plant() {
    let params = flat(&[SurfaceFeature::cube(fixtures().leaves, 8.0)]);
    let saved = Generation::capture(params, &mapping_with_leaves()).unwrap();
    let json = serde_json::to_string(&saved).unwrap();
    assert!(!json.contains("plant"), "{json}");
}

fn mapping_with_leaves() -> Mapping {
    let mut m = mapping();
    m.materials.insert(fixtures().leaves, "t:leaves".into());
    m
}
