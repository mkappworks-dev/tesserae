//! Fields: ground cover that gathers a species per region, and planted plots of one crop with paths
//! and a ditch between them, both whole across Chunk borders at every Voxel size.

use std::collections::{HashMap, HashSet, VecDeque};

use tesserae_core::save::{Generation, Mapping};
use tesserae_core::testgen::fixtures;
use tesserae_core::worldgen::WaterParams;
use tesserae_core::{
    Biome, BiomeName, BiomeTable, CHUNK_SIZE, ClimateParams, CropField, Field, FieldCrop,
    MaterialId, SurfaceFeature, SurfaceFeatures, Voxel, Worldgen, WorldgenParams,
};

const SEED: u64 = 0x7e55_e2ae_0000_0001;
const SIZES: [f32; 3] = [1.0, 0.5, 0.25];
const SIZE_I: i32 = CHUNK_SIZE as i32;
const FLOWERS: MaterialId = 230;
const CROP_A: [MaterialId; 3] = [220, 221, 222];
const CROP_B: [MaterialId; 3] = [240, 241, 242];
const PATH: MaterialId = 250;
const TRUNK: MaterialId = 210;
const CROSS: u8 = 5;

fn biome(features: SurfaceFeatures) -> Biome {
    Biome {
        name: BiomeName::new("t:plain").unwrap(),
        temperature: 0.5,
        humidity: 0.5,
        ground: None,
        sky_fog: None,
        cave_scale: None,
        surface_features: features,
    }
}

/// Ground that never rises a metre, so every plot is level at every Voxel size.
fn level(features: SurfaceFeatures, water: Option<WaterParams>) -> WorldgenParams {
    WorldgenParams {
        caves: None,
        smoothing: None,
        relief_m: 1.0,
        water,
        climate: ClimateParams {
            temperature_size_m: 64.0,
            humidity_size_m: 64.0,
        },
        biomes: BiomeTable::new(&[biome(features)]).unwrap(),
        ..tesserae_core::testgen::world_params()
    }
}

fn grouped(material: MaterialId, group: u8, strays: f32) -> SurfaceFeature {
    SurfaceFeature {
        field: Some(Field {
            group,
            scale_m: 24.0,
            strays,
        }),
        ..SurfaceFeature::cube(material, 3.0)
    }
}

/// Every cell `params` changes against bare ground, within `half_m` of the origin, by global cell.
fn changed(seed: u64, size: f32, params: WorldgenParams, half_m: f32) -> HashMap<[i32; 3], Voxel> {
    let with = Worldgen::new(seed, size, params);
    let bare = Worldgen::new(seed, size, params.without_surface_features());
    let chunk_m = SIZE_I as f32 * size;
    let (low, high) = with.surface_bounds_m();
    let rows = (low / chunk_m).floor() as i32 - 1..=(high / chunk_m).ceil() as i32 + 1;
    let reach = (half_m / chunk_m).ceil() as i32;
    let mut out = HashMap::new();
    for cy in rows {
        for cz in -reach..reach {
            for cx in -reach..reach {
                let (a, b) = (with.chunk(cx, cy, cz), bare.chunk(cx, cy, cz));
                for z in 0..CHUNK_SIZE {
                    for y in 0..CHUNK_SIZE {
                        for x in 0..CHUNK_SIZE {
                            let voxel = a.get(x, y, z);
                            if voxel != b.get(x, y, z) {
                                let at = [
                                    cx * SIZE_I + x as i32,
                                    cy * SIZE_I + y as i32,
                                    cz * SIZE_I + z as i32,
                                ];
                                out.insert(at, voxel);
                            }
                        }
                    }
                }
            }
        }
    }
    out
}

/// The share of a block's features held by its commonest species, averaged over blocks with
/// enough of them, and the share of all features that are not their block's commonest.
fn dominance(found: &HashMap<[i32; 3], Voxel>, size: f32, half_m: f32, block_m: f32) -> (f64, f64) {
    let across = (2.0 * half_m / block_m) as usize;
    let mut blocks = vec![[0u32; 3]; across * across];
    for (at, voxel) in found {
        let species = voxel.material().wrapping_sub(FLOWERS) as usize;
        if species >= 3 {
            continue;
        }
        let to = |g: i32| (((g as f32 * size + half_m) / block_m) as usize).min(across - 1);
        blocks[to(at[2]) * across + to(at[0])][species] += 1;
    }
    let (mut share, mut counted, mut total, mut minority) = (0.0, 0.0, 0.0, 0.0);
    for block in &blocks {
        let n: u32 = block.iter().sum();
        let most = *block.iter().max().unwrap();
        total += f64::from(n);
        minority += f64::from(n - most);
        if n >= 4 {
            share += f64::from(most) / f64::from(n);
            counted += 1.0;
        }
    }
    (share / counted, minority / total)
}

#[test]
fn a_wild_field_is_mostly_one_species_per_region_with_a_few_strays_at_every_voxel_size() {
    let flowers = |strays| {
        SurfaceFeatures::new(&[
            grouped(FLOWERS, 0, strays),
            grouped(FLOWERS + 1, 0, strays),
            grouped(FLOWERS + 2, 0, strays),
        ])
        .unwrap()
    };
    let mixed = SurfaceFeatures::new(&[
        SurfaceFeature::cube(FLOWERS, 3.0),
        SurfaceFeature::cube(FLOWERS + 1, 3.0),
        SurfaceFeature::cube(FLOWERS + 2, 3.0),
    ])
    .unwrap();
    for size in SIZES {
        let half_m = 24.0;
        let (share, strays) = dominance(
            &changed(SEED, size, level(flowers(0.1), None), half_m),
            size,
            half_m,
            4.0,
        );
        let (mixed_share, _) = dominance(
            &changed(SEED, size, level(mixed, None), half_m),
            size,
            half_m,
            4.0,
        );
        let (_, loose) = dominance(
            &changed(SEED, size, level(flowers(0.4), None), half_m),
            size,
            half_m,
            4.0,
        );
        println!(
            "{size}: {share:.2} one species a block against {mixed_share:.2} mixed; strays {strays:.2}"
        );
        assert!(
            share > 0.65 && share > mixed_share + 0.2,
            "{size} m: {share:.2} of a block is one species, {mixed_share:.2} unfielded"
        );
        assert!(
            mixed_share < 0.6,
            "{size} m: the control is {mixed_share:.2}"
        );
        assert!(
            (0.02..0.3).contains(&strays) && strays < loose,
            "{size} m: {strays:.2} strays, {loose:.2} with more"
        );
    }
}

#[test]
fn every_species_of_a_group_holds_regions_and_a_lone_member_is_unchanged() {
    let size = 1.0;
    let found = changed(
        SEED,
        size,
        level(
            SurfaceFeatures::new(&[
                grouped(FLOWERS, 0, 0.0),
                grouped(FLOWERS + 1, 0, 0.0),
                grouped(FLOWERS + 2, 0, 0.0),
            ])
            .unwrap(),
            None,
        ),
        96.0,
    );
    let species: HashSet<_> = found.values().map(|v| v.material()).collect();
    assert_eq!(species, HashSet::from([FLOWERS, FLOWERS + 1, FLOWERS + 2]));

    let lone = changed(
        SEED,
        size,
        level(
            SurfaceFeatures::new(&[grouped(FLOWERS, 0, 0.0)]).unwrap(),
            None,
        ),
        48.0,
    );
    let plain = changed(
        SEED,
        size,
        level(
            SurfaceFeatures::new(&[SurfaceFeature::cube(FLOWERS, 3.0)]).unwrap(),
            None,
        ),
        48.0,
    );
    assert_eq!(lone, plain, "a group of one competes with nothing");
}

#[test]
fn a_group_must_agree_on_its_region_and_strays() {
    let odd = SurfaceFeature {
        field: Some(Field {
            group: 0,
            scale_m: 32.0,
            strays: 0.1,
        }),
        ..SurfaceFeature::cube(FLOWERS + 1, 3.0)
    };
    for bad in [
        SurfaceFeatures::new(&[grouped(FLOWERS, 0, 0.1), odd]),
        SurfaceFeatures::new(&[SurfaceFeature {
            field: Some(Field {
                group: 0,
                scale_m: 7.0,
                strays: 0.1,
            }),
            ..SurfaceFeature::cube(FLOWERS, 3.0)
        }]),
        SurfaceFeatures::new(&[SurfaceFeature {
            field: Some(Field {
                group: 0,
                scale_m: 16.0,
                strays: 0.6,
            }),
            ..SurfaceFeature::cube(FLOWERS, 3.0)
        }]),
    ] {
        assert!(bad.is_err());
    }
}

fn wheat(weight: f32) -> FieldCrop {
    FieldCrop::new(CROSS, &CROP_A, &[], weight).unwrap()
}

fn barley(weight: f32) -> FieldCrop {
    FieldCrop::new(CROSS, &CROP_B, &[], weight).unwrap()
}

fn crop_field(near_water_m: Option<f32>) -> CropField {
    let mut field = CropField::new(
        (6.0, 10.0),
        48.0,
        Some(PATH),
        Some(fixtures().water),
        near_water_m,
    );
    field.push(wheat(3.0)).unwrap();
    field.push(barley(1.0)).unwrap();
    field
}

fn fielded(near_water_m: Option<f32>, features: &[SurfaceFeature]) -> SurfaceFeatures {
    SurfaceFeatures::new(features)
        .unwrap()
        .with_crop_fields(&[crop_field(near_water_m)])
        .unwrap()
}

/// One planted field as it stands in the cells: its footprint and what it holds.
struct Found {
    min: [i32; 2],
    max: [i32; 2],
    lanes: HashSet<[i32; 2]>,
    ditch: HashSet<[i32; 2]>,
    crops: HashMap<[i32; 2], MaterialId>,
}

/// The cells `changed` reads along each axis, either side of the origin.
fn window(size: f32, half_m: f32) -> i32 {
    (half_m / (SIZE_I as f32 * size)).ceil() as i32 * SIZE_I
}

/// The fields wholly inside `window` cells of the origin: one at its edge is cut off.
fn fields_in(found: &HashMap<[i32; 3], Voxel>, window: i32) -> Vec<Found> {
    let ditch = fixtures().water;
    let column = |at: &[i32; 3]| [at[0], at[2]];
    let lanes: HashSet<[i32; 2]> = found
        .iter()
        .filter(|(_, v)| v.material() == PATH || v.material() == ditch)
        .map(|(at, _)| column(at))
        .collect();
    let crops: HashMap<[i32; 2], MaterialId> = found
        .iter()
        .filter(|(_, v)| CROP_A.contains(&v.material()) || CROP_B.contains(&v.material()))
        .map(|(at, v)| (column(at), v.material()))
        .collect();
    let mut seen = HashSet::new();
    let mut all = Vec::new();
    for &start in &lanes {
        if !seen.insert(start) {
            continue;
        }
        let mut part = HashSet::from([start]);
        let mut queue = VecDeque::from([start]);
        while let Some([x, z]) = queue.pop_front() {
            for next in [[x + 1, z], [x - 1, z], [x, z + 1], [x, z - 1]] {
                if lanes.contains(&next) && seen.insert(next) {
                    part.insert(next);
                    queue.push_back(next);
                }
            }
        }
        let min = [0, 1].map(|a| part.iter().map(|c| c[a]).min().unwrap());
        let max = [0, 1].map(|a| part.iter().map(|c| c[a]).max().unwrap() + 1);
        if (0..2).any(|a| min[a] <= -window || max[a] >= window) {
            continue;
        }
        all.push(Found {
            min,
            max,
            ditch: part
                .iter()
                .copied()
                .filter(|c| {
                    found
                        .iter()
                        .any(|(at, v)| column(at) == *c && v.material() == ditch)
                })
                .collect(),
            lanes: part,
            crops: crops
                .iter()
                .filter(|(c, _)| (0..2).all(|a| (min[a]..max[a]).contains(&c[a])))
                .map(|(&c, &m)| (c, m))
                .collect(),
        });
    }
    all
}

/// Whether `lines` cut `[min, max)` into plots: the offsets along an axis that are lanes the whole
/// way across.
fn lane_lines(field: &Found, axis: usize) -> Vec<i32> {
    let other = 1 - axis;
    (field.min[axis]..field.max[axis])
        .filter(|&a| {
            (field.min[other]..field.max[other]).all(|b| {
                let cell = if axis == 0 { [a, b] } else { [b, a] };
                field.lanes.contains(&cell)
            })
        })
        .collect()
}

fn assert_field_is_whole(field: &Found, size: f32) {
    let (w, d) = (field.max[0] - field.min[0], field.max[1] - field.min[1]);
    for x in field.min[0]..field.max[0] {
        for z in [field.min[1], field.max[1] - 1] {
            assert!(field.lanes.contains(&[x, z]), "the ring is open at {x},{z}");
        }
    }
    for z in field.min[1]..field.max[1] {
        for x in [field.min[0], field.max[0] - 1] {
            assert!(field.lanes.contains(&[x, z]), "the ring is open at {x},{z}");
        }
    }
    let (xs, zs) = (lane_lines(field, 0), lane_lines(field, 1));
    assert!(xs.len() >= 2 && zs.len() >= 2, "{w}x{d} has no plots");
    let mut crops_seen = 0;
    for x in xs.windows(2).filter(|p| p[1] - p[0] > 1) {
        for z in zs.windows(2).filter(|p| p[1] - p[0] > 1) {
            let (width, depth) = (x[1] - x[0] - 1, z[1] - z[0] - 1);
            for side in [width, depth] {
                let metres = side as f32 * size;
                assert!(
                    (6.0 - size..=10.0 + size).contains(&metres),
                    "a plot side of {metres} m"
                );
            }
            let inside: Vec<([i32; 2], MaterialId)> = field
                .crops
                .iter()
                .filter(|(c, _)| {
                    (x[0] + 1..x[1]).contains(&c[0]) && (z[0] + 1..z[1]).contains(&c[1])
                })
                .map(|(&c, &m)| (c, m))
                .collect();
            let family = |m: MaterialId| CROP_A.contains(&m);
            assert!(
                inside
                    .iter()
                    .all(|(_, m)| family(*m) == family(inside[0].1)),
                "a plot holds one crop"
            );
            let along_z = (0..width).step_by(2).count() as i32 * depth;
            let along_x = (0..depth).step_by(2).count() as i32 * width;
            let every_second =
                |axis: usize, from: i32| inside.iter().all(|(c, _)| (c[axis] - from) % 2 == 0);
            assert!(
                (inside.len() as i32 == along_z && every_second(0, x[0] + 1))
                    || (inside.len() as i32 == along_x && every_second(1, z[0] + 1)),
                "a plot of {width}x{depth} holds {} crops, not every second row",
                inside.len()
            );
            crops_seen += inside.len();
        }
    }
    assert_eq!(crops_seen, field.crops.len(), "every crop is in a plot");
}

#[test]
fn planted_plots_are_rectangles_of_one_crop_in_rows_with_a_path_and_a_ditch() {
    for size in SIZES {
        let params = level(fielded(None, &[]), None);
        let found = changed(SEED, size, params, 120.0);
        let fields = fields_in(&found, window(size, 120.0));
        println!("{size}: {} fields", fields.len());
        assert!(!fields.is_empty(), "{size} m: a field stands");
        let border = |f: &Found| {
            (0..2).any(|a| f.min[a].div_euclid(SIZE_I) != (f.max[a] - 1).div_euclid(SIZE_I))
        };
        assert!(
            fields.iter().any(border),
            "{size} m: one crosses a Chunk border"
        );
        for field in &fields {
            assert_field_is_whole(field, size);
            let edges = [
                (0..2).map(|a| field.min[a]).collect::<Vec<_>>(),
                (0..2).map(|a| field.max[a] - 1).collect::<Vec<_>>(),
            ];
            let on = |c: &[i32; 2]| (0..2).any(|a| c[a] == edges[0][a] || c[a] == edges[1][a]);
            assert!(!field.ditch.is_empty(), "a ditch runs along it");
            assert!(
                field.ditch.iter().all(on),
                "the ditch is on the field's edge"
            );
            let one_edge = (0..2).any(|a| {
                [edges[0][a], edges[1][a]]
                    .iter()
                    .any(|&e| field.ditch.iter().all(|c| c[a] == e))
            });
            assert!(one_edge, "the ditch runs along one edge");
        }
    }
}

#[test]
fn a_field_generates_the_same_cells_whichever_chunks_are_made_first() {
    let params = level(fielded(None, &[]), None);
    let (a, b) = (
        Worldgen::new(SEED, 0.5, params),
        Worldgen::new(SEED, 0.5, params),
    );
    let (low, high) = a.surface_bounds_m();
    let rows = (low / 16.0).floor() as i32 - 1..=(high / 16.0).ceil() as i32 + 1;
    let coords: Vec<[i32; 3]> = rows
        .flat_map(|cy| (-6..6).flat_map(move |cz| (-6..6).map(move |cx| [cx, cy, cz])))
        .collect();
    let forward: Vec<_> = coords.iter().map(|c| a.chunk(c[0], c[1], c[2])).collect();
    let mut backward: Vec<_> = coords
        .iter()
        .rev()
        .map(|c| b.chunk(c[0], c[1], c[2]))
        .collect();
    backward.reverse();
    for (x, y) in forward.iter().zip(&backward) {
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

#[test]
fn a_field_stands_on_no_water_and_near_water_means_near_the_lake() {
    let lake = Some(WaterParams {
        level_m: 24.3,
        material: fixtures().water,
    });
    let reach_m = 16.0;
    for size in SIZES {
        let anywhere = changed(SEED, size, level(fielded(None, &[]), lake), 120.0);
        let near = changed(SEED, size, level(fielded(Some(reach_m), &[]), lake), 120.0);
        let world = Worldgen::new(SEED, size, level(fielded(None, &[]), lake));
        for (at, voxel) in &anywhere {
            if voxel.material() == PATH || CROP_A.contains(&voxel.material()) {
                assert!(
                    (at[1] + 1) as f32 * size >= 24.3,
                    "{size} m: a field cell under the lake at {at:?}"
                );
            }
        }
        let fields_near = fields_in(&near, window(size, 120.0));
        let fields_anywhere = fields_in(&anywhere, window(size, 120.0));
        assert!(
            fields_near.len() <= fields_anywhere.len(),
            "{size} m: near-water only narrows"
        );
        for field in &fields_near {
            let centre = |a: usize| (field.min[a] + field.max[a]) as f32 / 2.0 * size;
            let half = (field.max[0] - field.min[0]).max(field.max[1] - field.min[1]) as f32 * size;
            let wet = (0..360).step_by(15).any(|deg| {
                let angle = (deg as f32).to_radians();
                let reach = reach_m + half;
                (1..=8).any(|k| {
                    let r = reach * k as f32 / 8.0;
                    world.surface_height_m(centre(0) + r * angle.cos(), centre(1) + r * angle.sin())
                        < 24.3
                })
            });
            assert!(wet, "{size} m: a field with no water within {reach_m} m");
        }
    }
}

#[test]
fn a_tree_inside_the_ground_a_field_would_take_skips_the_field_whole() {
    let tree = SurfaceFeature {
        plant: Some(tesserae_core::SurfacePlant {
            shape: 0,
            kind: tesserae_core::SurfacePlantKind::Tree(tesserae_core::SurfaceTree {
                branching: None,
                trunk: TRUNK,
                canopy: TRUNK + 1,
                trunk_height_m: (3.0, 4.0),
                canopy_radius_m: 1.5,
                vines: None,
                fruit: None,
            }),
        }),
        ..SurfaceFeature::cube(TRUNK, 24.0)
    };
    let size = 1.0;
    let without = fields_in(
        &changed(SEED, size, level(fielded(None, &[]), None), 150.0),
        window(size, 150.0),
    );
    let found = changed(SEED, size, level(fielded(None, &[tree]), None), 150.0);
    let with = fields_in(&found, window(size, 150.0));
    assert!(
        with.len() < without.len(),
        "trees skip some fields: {} against {}",
        with.len(),
        without.len()
    );
    for field in &with {
        let trunk = found.iter().any(|(at, v)| {
            v.material() == TRUNK
                && (field.min[0]..field.max[0]).contains(&at[0])
                && (field.min[1]..field.max[1]).contains(&at[2])
        });
        assert!(!trunk, "a trunk stands in a field");
    }
}

fn mapping() -> Mapping {
    let f = fixtures();
    let mut names: Vec<(MaterialId, String)> = vec![
        (0, "core:air".into()),
        (f.ground.rock, "t:rock".into()),
        (f.ground.soil, "t:soil".into()),
        (f.ground.surface, "t:surface".into()),
        (f.water, "t:water".into()),
        (PATH, "t:path".into()),
    ];
    for (i, m) in CROP_A.iter().chain(&CROP_B).enumerate() {
        names.push((*m, format!("t:crop{i}")));
    }
    for i in 0..3 {
        names.push((FLOWERS + i, format!("t:flower{i}")));
    }
    Mapping {
        materials: names.into_iter().collect(),
        shapes: [(0, "core:cube".to_owned()), (CROSS, "t:cross".to_owned())].into(),
    }
}

#[test]
fn a_snapshot_freezes_the_fields_and_an_older_one_has_none() {
    let params = level(
        fielded(
            None,
            &[
                grouped(FLOWERS, 1, 0.2),
                grouped(FLOWERS + 1, 1, 0.2),
                SurfaceFeature::cube(FLOWERS + 2, 9.0),
            ],
        ),
        None,
    );
    let saved = Generation::capture(params, &mapping()).unwrap();
    let json = serde_json::to_string(&saved).unwrap();
    let restored = serde_json::from_str::<Generation>(&json)
        .unwrap()
        .restore(&mapping(), 2)
        .unwrap();
    assert_eq!(restored.biomes, params.biomes);
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    let biome = &value["biomes"][0];
    assert_eq!(biome["surface_features"][0]["field"]["group"], 1);
    assert!(biome["surface_features"][2].get("field").is_none());
    assert_eq!(biome["crop_fields"][0]["path"], "t:path");
    assert_eq!(
        biome["crop_fields"][0]["crops"][0]["materials"][0],
        "t:crop0"
    );

    let plain = level(
        SurfaceFeatures::new(&[SurfaceFeature::cube(FLOWERS, 9.0)]).unwrap(),
        None,
    );
    let json = serde_json::to_string(&Generation::capture(plain, &mapping()).unwrap()).unwrap();
    assert!(!json.contains("crop_fields") && !json.contains("\"field\""));
    assert_eq!(
        serde_json::from_str::<Generation>(&json)
            .unwrap()
            .restore(&mapping(), 2)
            .unwrap()
            .biomes,
        plain.biomes
    );
}

#[test]
fn a_biome_with_no_fields_generates_what_it_did_before_them() {
    let features = [
        SurfaceFeature::cube(FLOWERS, 5.0),
        SurfaceFeature::cube(FLOWERS + 1, 7.0),
    ];
    let before = level(SurfaceFeatures::new(&features).unwrap(), None);
    let after = level(
        SurfaceFeatures::new(&features)
            .unwrap()
            .with_crop_fields(&[])
            .unwrap(),
        None,
    );
    for size in SIZES {
        assert_eq!(
            changed(SEED, size, before, 32.0),
            changed(SEED, size, after, 32.0)
        );
    }
}

#[test]
fn crop_fields_are_checked() {
    let features = SurfaceFeatures::new(&[]).unwrap();
    let bad = |edit: &dyn Fn(&mut CropField)| {
        let mut field = crop_field(None);
        edit(&mut field);
        features.with_crop_fields(&[field]).is_err()
    };
    assert!(bad(&|f| f.plot_m = (5.0, 10.0)));
    assert!(bad(&|f| f.plot_m = (10.0, 6.0)));
    assert!(bad(&|f| f.spacing_m = 31.0));
    assert!(bad(&|f| f.near_water_m = Some(49.0)));
    assert!(bad(&|f| f.path = Some(0)));
    assert!(bad(&|f| f.len = 0));
    assert!(!bad(&|_| {}));
    assert!(
        features
            .with_crop_fields(&[crop_field(None), crop_field(None), crop_field(None)])
            .is_err()
    );
}
