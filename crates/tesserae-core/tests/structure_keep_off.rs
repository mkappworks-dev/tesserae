//! What is laid after Structures keeps off them: no tree, scatter Voxel or field cell stands in a
//! column a Structure covers, and a tree that would is left out whole from every Chunk's viewpoint.

use std::collections::{HashMap, HashSet};

use tesserae_core::plant::tree::tree_voxels;
use tesserae_core::plant::{CountRange, Tree};
use tesserae_core::template::{Cell, Template};
use tesserae_core::testgen::fixtures;
use tesserae_core::{
    Biome, BiomeName, BiomeStructure, BiomeTable, CHUNK_SIZE, ClimateParams, CropField, FieldCrop,
    MaterialId, SurfaceFeature, SurfaceFeatures, SurfacePlant, SurfacePlantKind, SurfaceTree,
    Voxel, Worldgen, WorldgenParams,
};

const SEED: u64 = 0x7e55_e2ae_0000_0001;
const SIZE_I: i32 = CHUNK_SIZE as i32;
const WALL: MaterialId = 243;
const TRUNK: MaterialId = 210;
const CANOPY: MaterialId = 211;
const FLOWERS: MaterialId = 230;
const CROP: [MaterialId; 3] = [220, 221, 222];
const PATH: MaterialId = 250;
const CROSS: u8 = 5;

/// 4 m by 5 m: a buried floor under a Ground layer of Keep, so the ground's own surface is left
/// for anything that does not keep off to stand on.
fn block() -> Template {
    let layer = |cell: Cell| (0..5).map(|_| vec![cell; 4]).collect();
    let floor = layer(Cell::Voxel(Voxel::cube(WALL)));
    Template::new(
        "t:block".to_owned(),
        1.0,
        vec![floor, layer(Cell::Keep)],
        1,
        None,
        1.0,
    )
    .unwrap()
}

fn oak() -> SurfaceTree {
    SurfaceTree {
        branching: None,
        trunk: TRUNK,
        canopy: CANOPY,
        trunk_height_m: (3.0, 5.0),
        canopy_radius_m: 2.5,
        vines: None,
        fruit: None,
    }
}

fn trees(spacing_m: f32) -> SurfaceFeature {
    SurfaceFeature {
        material: TRUNK,
        spacing_m,
        plant: Some(SurfacePlant {
            shape: 0,
            kind: SurfacePlantKind::Tree(oak()),
        }),
        clump: None,
        field: None,
    }
}

fn crop_field() -> CropField {
    let mut field = CropField::new((6.0, 10.0), 32.0, Some(PATH), Some(fixtures().water), None);
    field
        .push(FieldCrop::new(CROSS, &CROP, &[], 1.0).unwrap())
        .unwrap();
    field
}

fn params(features: SurfaceFeatures, structures: bool) -> WorldgenParams {
    let rows = [BiomeStructure {
        template: 0,
        spacing_m: 16.0,
        chance: 1.0,
    }];
    let surface_features = features
        .with_structures(if structures { &rows } else { &[] })
        .unwrap();
    WorldgenParams {
        caves: None,
        smoothing: None,
        relief_m: 1.0,
        water: None,
        climate: ClimateParams {
            temperature_size_m: 64.0,
            humidity_size_m: 64.0,
        },
        biomes: BiomeTable::new(&[Biome {
            name: BiomeName::new("t:plain").unwrap(),
            temperature: 0.5,
            humidity: 0.5,
            ground: None,
            sky_fog: None,
            cave_scale: None,
            surface_features,
        }])
        .unwrap(),
        ..tesserae_core::testgen::world_params()
    }
}

/// Every non-Air cell in `half_m` of the origin by global cell, generating the Chunks in one order
/// or its reverse.
fn cells(generator: &Worldgen, half_m: f32, reverse: bool) -> HashMap<[i32; 3], Voxel> {
    let chunk_m = SIZE_I as f32 * generator.voxel_size();
    let (low, high) = generator.surface_bounds_m();
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
        let chunk = generator.chunk(c[0], c[1], c[2]);
        for z in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let v = chunk.get(x, y, z);
                    if !v.is_air() {
                        let at = [x, y, z].map(|l| l as i32);
                        out.insert([0, 1, 2].map(|a| c[a] * SIZE_I + at[a]), v);
                    }
                }
            }
        }
    }
    out
}

/// The same World generated with its Structure rows and without them.
struct Pair {
    with: HashMap<[i32; 3], Voxel>,
    wild: HashMap<[i32; 3], Voxel>,
    /// Every column a Structure covers.
    covered: HashSet<[i32; 2]>,
}

fn pair(size: f32, features: SurfaceFeatures, half_m: f32) -> Pair {
    let gen_with = Worldgen::new(SEED, size, params(features, true)).with_templates(&[block()]);
    let gen_wild = Worldgen::new(SEED, size, params(features, false)).with_templates(&[block()]);
    let with = cells(&gen_with, half_m, false);
    assert_eq!(with, cells(&gen_with, half_m, true), "{size} m: order");
    let covered: HashSet<_> = with
        .iter()
        .filter(|(_, v)| v.material() == WALL)
        .map(|(at, _)| [at[0], at[2]])
        .collect();
    assert!(covered.len() > 100, "{size} m: few Structures stood");
    Pair {
        with,
        wild: cells(&gen_wild, half_m, false),
        covered,
    }
}

/// The cells of `cells` holding one of `materials` in a covered column.
fn under(pair: &Pair, cells: &HashMap<[i32; 3], Voxel>, materials: &[MaterialId]) -> usize {
    cells
        .iter()
        .filter(|(at, v)| {
            materials.contains(&v.material()) && pair.covered.contains(&[at[0], at[2]])
        })
        .count()
}

fn row() -> Tree {
    let o = oak();
    Tree {
        vines: None,
        branching: None,
        trunk: o.trunk,
        canopy: o.canopy,
        trunk_height_m: o.trunk_height_m,
        canopy_radius_m: o.canopy_radius_m,
        mean_seconds: 0.0,
        trunk_yield: Vec::new(),
        canopy_yield: Vec::new(),
        sapling_yield: CountRange { min: 1, max: 1 },
        fruit: None,
    }
}

/// Every cell of the tree whose trunk stands on `base`, from the pure function the generator calls.
fn tree_at(size: f32, base: [i32; 3]) -> Vec<[i32; 3]> {
    let folded = u64::from((SEED ^ (SEED >> 32)) as u32);
    tree_voxels(&row(), folded, [base[0], 0, base[2]], size)
        .into_iter()
        .map(|v| [v.at[0], v.at[1] + base[1], v.at[2]])
        .collect()
}

#[test]
fn no_tree_stands_in_a_structure_and_every_tree_is_whole_across_chunk_borders() {
    let half_m = 32.0;
    for size in [1.0, 0.5] {
        let features = SurfaceFeatures::new(&[trees(5.0)]).unwrap();
        let pair = pair(size, features, half_m);
        let wood = [TRUNK, CANOPY];
        assert_eq!(
            under(&pair, &pair.with, &wood),
            0,
            "{size} m: a tree in a Structure"
        );
        assert!(
            under(&pair, &pair.wild, &wood) > 0,
            "{size} m: no tree would have stood there"
        );
        let is_wood = |at: &[i32; 3]| {
            pair.with
                .get(at)
                .is_some_and(|v| wood.contains(&v.material()))
        };
        let feet: Vec<[i32; 3]> = pair
            .with
            .iter()
            .filter(|(at, v)| {
                let below = [at[0], at[1] - 1, at[2]];
                v.material() == TRUNK && !is_wood(&below) && pair.with.contains_key(&below)
            })
            .map(|(at, _)| *at)
            .collect();
        assert!(feet.len() > 20, "{size} m: only {} trees", feet.len());
        let window = (half_m / (SIZE_I as f32 * size)).ceil() as i32 * SIZE_I;
        let scanned = |at: &[i32; 3]| [at[0], at[2]].iter().all(|a| (-window..window).contains(a));
        let mut owned = HashSet::new();
        for &foot in &feet {
            for at in tree_at(size, foot) {
                assert!(
                    pair.with.contains_key(&at) || !scanned(&at),
                    "{size} m: the tree at {foot:?} is cut at {at:?}"
                );
                owned.insert(at);
            }
        }
        for at in pair.with.keys().filter(|at| is_wood(at)) {
            let edge = [at[0], at[2]]
                .iter()
                .any(|&a| a.abs() as f32 * size > half_m - 6.0);
            assert!(
                owned.contains(at) || edge,
                "{size} m: wood at {at:?} belongs to no whole tree"
            );
        }
    }
}

#[test]
fn no_scatter_voxel_or_field_cell_stands_in_a_structure() {
    let laid = [FLOWERS, PATH, fixtures().water, CROP[0], CROP[1], CROP[2]];
    for size in [1.0, 0.5] {
        let features = SurfaceFeatures::new(&[SurfaceFeature::cube(FLOWERS, 2.0)])
            .unwrap()
            .with_crop_fields(&[crop_field()])
            .unwrap();
        let pair = pair(size, features, 48.0);
        assert_eq!(
            under(&pair, &pair.with, &laid),
            0,
            "{size} m: scatter or a field in a Structure"
        );
        for (what, materials) in [
            ("scatter", &[FLOWERS][..]),
            (
                "field",
                &[PATH, fixtures().water, CROP[0], CROP[1], CROP[2]][..],
            ),
        ] {
            assert!(
                under(&pair, &pair.wild, materials) > 0,
                "{size} m: no {what} would have stood there"
            );
        }
    }
}
