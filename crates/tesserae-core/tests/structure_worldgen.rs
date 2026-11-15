//! Structures in generation: a Biome lists Templates, and worldgen stands each one whole from any
//! Chunk's viewpoint, the same in metres at every Voxel size.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use tesserae_core::template::{Cell, Template, Turn};
use tesserae_core::{
    AIR, Biome, BiomeName, BiomeStructure, BiomeTable, CHUNK_SIZE, Chunk, ClimateParams,
    MaterialId, SurfaceFeatures, Voxel, Worldgen, WorldgenParams,
};

const CELLAR: MaterialId = 240;
const FLOOR: MaterialId = 241;
const MARK: MaterialId = 242;
const WALL: MaterialId = 243;
const STEP: MaterialId = 244;
const STAIR_SHAPE: u8 = 2;
const STAIR_FACING: u8 = 2;

const SEED: u64 = 0x7e55_e2ae_0000_0001;
const SIZES: [f32; 3] = [1.0, 0.5, 0.25];
const SIZE_I: i32 = CHUNK_SIZE as i32;

/// 4 m along x, 5 m along z: a cellar, a floor marked at one corner so its turn shows, and walls
/// with a stair and a doorway. `k` is Keep, `.` Air.
fn hut() -> Template {
    let layers = [
        ["ccccc", "c..kc", "c...c", "ccccc"],
        ["mffff", "fffff", "fffff", "ffffk"],
        ["wswkw", "w...w", "w...w", "wwwww"],
    ];
    let cell = |c: char| match c {
        'k' => Cell::Keep,
        '.' => Cell::Air,
        'c' => Cell::Voxel(Voxel::cube(CELLAR)),
        'f' => Cell::Voxel(Voxel::cube(FLOOR)),
        'm' => Cell::Voxel(Voxel::cube(MARK)),
        'w' => Cell::Voxel(Voxel::cube(WALL)),
        's' => Cell::Voxel(Voxel::new(STEP, STAIR_SHAPE, STAIR_FACING)),
        _ => unreachable!(),
    };
    // Written as rows along x of cells along z, so transpose into rows along z.
    let layers = layers
        .iter()
        .map(|rows| {
            (0..5)
                .map(|z| {
                    rows.iter()
                        .map(|row| cell(row.as_bytes()[z] as char))
                        .collect()
                })
                .collect()
        })
        .collect();
    Template::new("t:hut".to_owned(), 1.0, layers, 1, None, 1.0).unwrap()
}

fn row(spacing_m: f32, chance: f32) -> BiomeStructure {
    BiomeStructure {
        template: 0,
        spacing_m,
        chance,
    }
}

fn biome(structures: &[BiomeStructure]) -> Biome {
    Biome {
        name: BiomeName::new("t:plain").unwrap(),
        temperature: 0.5,
        humidity: 0.5,
        ground: None,
        sky_fog: None,
        cave_scale: None,
        surface_features: SurfaceFeatures::NONE.with_structures(structures).unwrap(),
    }
}

fn flat(structures: &[BiomeStructure]) -> WorldgenParams {
    WorldgenParams {
        caves: None,
        smoothing: None,
        climate: ClimateParams {
            temperature_size_m: 64.0,
            humidity_size_m: 64.0,
        },
        biomes: BiomeTable::new(&[biome(structures)]).unwrap(),
        ..tesserae_core::testgen::world_params()
    }
}

/// A World generated twice, with and without its Structures, read one cell at a time from
/// whichever Chunk holds it.
struct Probe {
    with: Worldgen,
    bare: Worldgen,
    chunks: RefCell<HashMap<[i32; 3], (Chunk, Chunk)>>,
}

impl Probe {
    fn new(size: f32, params: WorldgenParams) -> Self {
        Self::of(size, params, hut())
    }

    fn of(size: f32, params: WorldgenParams, template: Template) -> Self {
        Probe {
            with: Worldgen::new(SEED, size, params).with_templates(&[template]),
            bare: Worldgen::new(SEED, size, flat(&[])),
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

    /// Every cell the Structures changed, in `half_m` of the origin, generating the Chunks in
    /// one order or its reverse.
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
                            out.insert(
                                [
                                    c[0] * SIZE_I + x as i32,
                                    c[1] * SIZE_I + y as i32,
                                    c[2] * SIZE_I + z as i32,
                                ],
                                (w, b),
                            );
                        }
                    }
                }
            }
        }
        out
    }
}

/// One Structure found in a generated World: its lowest corner in Voxels and its turn.
#[derive(Clone, Copy, Debug)]
struct Found {
    corner: [i32; 3],
    mark: [i32; 3],
    turn: Turn,
}

/// The corner of each block of `MARK` Voxels: one per Structure.
fn marks(changed: &HashMap<[i32; 3], (Voxel, Voxel)>) -> Vec<[i32; 3]> {
    let is_mark = |at: [i32; 3]| changed.get(&at).is_some_and(|(w, _)| w.material() == MARK);
    let mut found: Vec<_> = changed
        .keys()
        .copied()
        .filter(|&at| is_mark(at))
        .filter(|&[x, y, z]| {
            !is_mark([x - 1, y, z]) && !is_mark([x, y - 1, z]) && !is_mark([x, y, z - 1])
        })
        .collect();
    found.sort();
    found
}

/// What the Structure turned by `turn` with its mark's corner at `mark` holds, cell by cell, by
/// the Template's own sampling.
fn expected(size: f32, turn: Turn, mark: [i32; 3]) -> Vec<([i32; 3], Cell)> {
    let template = hut();
    let sampled = template.sample(size, turn);
    let [sx, sy, sz] = sampled.size();
    let cells: Vec<_> = (0..sy)
        .flat_map(|y| (0..sz).flat_map(move |z| (0..sx).map(move |x| [x, y, z])))
        .map(|at| (at, sampled.cell(at)))
        .collect();
    let first_mark = cells
        .iter()
        .filter(|(_, c)| matches!(c, Cell::Voxel(v) if v.material() == MARK))
        .map(|(at, _)| *at)
        .min()
        .unwrap();
    let corner = [0, 1, 2].map(|a| mark[a] - first_mark[a]);
    cells
        .into_iter()
        .map(|(at, c)| ([0, 1, 2].map(|a| corner[a] + at[a]), c))
        .collect()
}

/// Whether the World holds exactly this Structure: every Voxel and Air cell written, every Keep
/// cell left as the terrain was.
fn holds(probe: &Probe, cells: &[([i32; 3], Cell)]) -> bool {
    cells.iter().all(|&(at, cell)| {
        let (with, bare) = probe.at(at);
        match cell {
            Cell::Keep => with == bare,
            Cell::Air => with.is_air(),
            Cell::Voxel(v) => with == v,
        }
    })
}

/// Every Structure in `half_m` of the origin, each whole by its Template however many Chunks it
/// spans, and nothing else changed; the same whichever order the Chunks are generated in.
fn structures(probe: &Probe, size: f32, half_m: f32) -> Vec<Found> {
    let changed = probe.changed(half_m, false);
    assert_eq!(changed, probe.changed(half_m, true), "{size} m: order");
    let mut owned = HashSet::new();
    let mut found = Vec::new();
    for mark in marks(&changed) {
        let turns: Vec<Turn> = Turn::all()
            .filter(|&turn| holds(probe, &expected(size, turn, mark)))
            .collect();
        assert_eq!(
            turns.len(),
            1,
            "{size} m: the Structure at {mark:?} is not whole"
        );
        let cells = expected(size, turns[0], mark);
        owned.extend(cells.iter().map(|c| c.0));
        let corner = [0, 1, 2].map(|a| cells.iter().map(|c| c.0[a]).min().unwrap());
        found.push(Found {
            corner,
            mark,
            turn: turns[0],
        });
    }
    for at in changed.keys() {
        // The scan stops at `half_m`, so a cell near it may belong to a Structure beyond.
        let edge = [at[0], at[2]]
            .iter()
            .any(|&a| a.abs() as f32 * size > half_m - 8.0);
        assert!(
            owned.contains(at) || edge,
            "{size} m: {at:?} changed outside any Structure"
        );
    }
    found
}

fn metres(cells: [i32; 3], size: f32) -> [f32; 3] {
    cells.map(|c| c as f32 * size)
}

#[test]
fn a_structure_is_whole_across_chunk_borders_and_equal_in_metres_at_every_voxel_size() {
    let half_m = 48.0;
    let mut crossings = 0;
    let mut by_size = Vec::new();
    for size in SIZES {
        let probe = Probe::new(size, flat(&[row(16.0, 0.8)]));
        let found = structures(&probe, size, half_m);
        assert!(
            found.len() > 10,
            "{size} m: only {} Structures",
            found.len()
        );
        let mut places = Vec::new();
        for f in &found {
            let [sx, sy, sz] = hut().sample(size, f.turn).size();
            let far = [
                f.corner[0] + sx - 1,
                f.corner[1] + sy - 1,
                f.corner[2] + sz - 1,
            ];
            crossings += usize::from(
                (0..3).any(|a| f.corner[a].div_euclid(SIZE_I) != far[a].div_euclid(SIZE_I)),
            );
            let [x, _, z] = metres(f.corner, size);
            // Away from the scan's edge, where a Structure may be cut.
            if x.abs() < half_m - 8.0 && z.abs() < half_m - 8.0 {
                places.push((x.to_bits(), z.to_bits(), f.turn));
            }
        }
        places.sort_by_key(|p| (p.0, p.1, p.2.quarters()));
        by_size.push(places);
    }
    assert!(
        crossings > 5,
        "{crossings} Structures crossed a Chunk border"
    );
    assert_eq!(
        by_size[0], by_size[1],
        "1 m and 0.5 m stand different Structures"
    );
    assert_eq!(
        by_size[0], by_size[2],
        "1 m and 0.25 m stand different Structures"
    );
}

#[test]
fn all_four_turns_appear_and_a_turned_footprint_is_the_same_in_metres() {
    let size = 0.5;
    let probe = Probe::new(size, flat(&[row(16.0, 1.0)]));
    let found = structures(&probe, size, 64.0);
    let turns: HashSet<u8> = found.iter().map(|f| f.turn.quarters()).collect();
    assert_eq!(turns.len(), 4, "turns seen: {turns:?}");
    for f in found {
        let [x, y, z] = hut().sample(size, f.turn).size().map(|n| n as f32 * size);
        let along = if f.turn.quarters().is_multiple_of(2) {
            [x, z]
        } else {
            [z, x]
        };
        assert_eq!((along, y), ([4.0, 5.0], 3.0), "turn {:?}", f.turn);
    }
}

#[test]
fn keep_cells_leave_the_terrain_and_air_cells_carve_it() {
    let size = 1.0;
    let probe = Probe::new(size, flat(&[row(16.0, 1.0)]));
    let found = structures(&probe, size, 32.0);
    assert!(!found.is_empty());
    let (mut kept_ground, mut carved) = (0, 0);
    for f in found {
        for (at, cell) in expected(size, f.turn, f.mark) {
            let (with, bare) = probe.at(at);
            match cell {
                Cell::Keep if !bare.is_air() => {
                    assert_eq!(with, bare);
                    kept_ground += 1;
                }
                Cell::Air if !bare.is_air() => {
                    assert_eq!(with, Voxel::cube(AIR));
                    carved += 1;
                }
                _ => {}
            }
        }
    }
    assert!(kept_ground > 0, "no Keep cell stood in the ground");
    assert!(carved > 0, "no Air cell carved the ground");
}

/// 2 m square and 40 m tall, marked at its foot.
fn tower() -> Template {
    let layer = |y: usize| {
        (0..2)
            .map(|z| {
                (0..2)
                    .map(|x| {
                        let m = if (x, y, z) == (0, 0, 0) { MARK } else { WALL };
                        Cell::Voxel(Voxel::cube(m))
                    })
                    .collect()
            })
            .collect()
    };
    Template::new(
        "t:tower".to_owned(),
        1.0,
        (0..40).map(layer).collect(),
        0,
        None,
        1.0,
    )
    .unwrap()
}

#[test]
fn a_structure_taller_than_the_terrain_band_is_whole_above_it() {
    let size = 0.5;
    let probe = Probe::of(size, flat(&[row(8.0, 1.0)]), tower());
    let (_, highest) = probe.bare.surface_bounds_m();
    let changed = probe.changed(24.0, false);
    let feet = marks(&changed);
    assert!(feet.len() > 4, "only {} towers", feet.len());
    let mut above = 0;
    for foot in feet {
        let top = [foot[0], foot[1] + (40.0 / size) as i32 - 1, foot[2]];
        assert_eq!(
            probe.at(top).0.material(),
            WALL,
            "tower at {foot:?} cut short"
        );
        assert!(probe.at([top[0], top[1] + 1, top[2]]).0.is_air());
        above += usize::from(top[1] as f32 * size > highest);
    }
    assert!(above > 0, "no tower rose above the terrain band");
}

#[test]
fn two_rows_of_one_spacing_never_stand_structures_that_touch() {
    let size = 1.0;
    let probe = Probe::new(size, flat(&[row(16.0, 1.0), row(16.0, 1.0)]));
    let found = structures(&probe, size, 48.0);
    let one_row = structures(&Probe::new(size, flat(&[row(16.0, 1.0)])), size, 48.0);
    assert!(found.len() > one_row.len(), "the second row stood nothing");
    let boxes: Vec<_> = found
        .iter()
        .map(|f| {
            let [sx, _, sz] = hut().sample(size, f.turn).size();
            [f.corner[0], f.corner[2], f.corner[0] + sx, f.corner[2] + sz]
        })
        .collect();
    for (i, a) in boxes.iter().enumerate() {
        for b in &boxes[..i] {
            let apart = a[2] < b[0] || b[2] < a[0] || a[3] < b[1] || b[3] < a[1];
            assert!(apart, "{a:?} touches {b:?}");
        }
    }
}

#[test]
fn templates_no_biome_lists_change_nothing() {
    let params = tesserae_core::testgen::world_params();
    for size in SIZES {
        let plain = Worldgen::new(SEED, size, params);
        let given = Worldgen::new(SEED, size, params).with_templates(&[hut()]);
        let (low, high) = plain.surface_bounds_m();
        let chunk_m = SIZE_I as f32 * size;
        for cy in (low / chunk_m).floor() as i32 - 1..=(high / chunk_m).ceil() as i32 + 1 {
            for cz in -2..2 {
                for cx in -2..2 {
                    let (a, b) = (plain.chunk(cx, cy, cz), given.chunk(cx, cy, cz));
                    assert_eq!(a.uniform(), b.uniform());
                    for i in 0..CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE {
                        let at = (
                            i % CHUNK_SIZE,
                            i / CHUNK_SIZE % CHUNK_SIZE,
                            i / CHUNK_SIZE.pow(2),
                        );
                        assert_eq!(a.get(at.0, at.1, at.2), b.get(at.0, at.1, at.2));
                    }
                }
            }
        }
    }
}

#[test]
fn a_biome_lists_four_structures_and_a_fifth_is_refused() {
    assert!(
        SurfaceFeatures::NONE
            .with_structures(&[row(16.0, 1.0); 4])
            .is_ok()
    );
    assert!(
        SurfaceFeatures::NONE
            .with_structures(&[row(16.0, 1.0); 5])
            .is_err()
    );
}

#[test]
fn a_spacing_under_twice_the_longer_side_or_a_chance_outside_one_is_refused() {
    let hut = hut();
    assert!(row(10.0, 1.0).validate(&hut).is_ok());
    assert!(row(9.9, 1.0).validate(&hut).is_err());
    assert!(row(16.0, 0.0).validate(&hut).is_err());
    assert!(row(16.0, 1.5).validate(&hut).is_err());
    assert!(row(f32::NAN, 1.0).validate(&hut).is_err());
}

#[test]
fn a_biome_with_four_structures_sixteen_features_and_two_fields_stays_inline_in_three_kib() {
    let size = std::mem::size_of::<Biome>();
    assert!(size < 3072, "{size} bytes");
}

/// 3 m square, its Ground layer on top of a shaft 39 m deep: a cellar open to the sky, floored
/// and marked at its foot.
fn well() -> Template {
    let layer = |y: usize| {
        (0..3)
            .map(|z| {
                (0..3)
                    .map(|x| match (x, y, z) {
                        (0, 0, 0) => Cell::Voxel(Voxel::cube(MARK)),
                        (_, 0, _) => Cell::Voxel(Voxel::cube(CELLAR)),
                        (1, _, 1) => Cell::Air,
                        _ => Cell::Voxel(Voxel::cube(WALL)),
                    })
                    .collect()
            })
            .collect()
    };
    Template::new(
        "t:well".to_owned(),
        1.0,
        (0..40).map(layer).collect(),
        39,
        None,
        1.0,
    )
    .unwrap()
}

#[test]
fn the_bounds_hold_every_structure_voxel_and_only_air_lies_above_them() {
    let size = 0.5;
    let wells = BiomeStructure {
        template: 1,
        ..row(12.0, 1.0)
    };
    let probe = Probe {
        with: Worldgen::new(SEED, size, flat(&[row(8.0, 0.5), wells]))
            .with_templates(&[tower(), well()]),
        bare: Worldgen::new(SEED, size, flat(&[])),
        chunks: RefCell::default(),
    };
    let (lowest, highest) = probe.with.surface_bounds_m();
    let (band_low, band_high) = probe.bare.surface_bounds_m();
    let chunk_m = SIZE_I as f32 * size;
    let deep_rock_m = band_low - probe.bare.params().soil_depth_m;
    let changed = probe.changed(24.0, false);
    let floors = || changed.keys().map(|c| c[1] as f32 * size);
    assert!(
        floors().any(|f| f > band_high),
        "no tower rose above the band"
    );
    let in_deep_rock = |y: i32| (y.div_euclid(SIZE_I) * SIZE_I + SIZE_I - 1) as f32 * size;
    assert!(
        changed.keys().any(|c| in_deep_rock(c[1]) < deep_rock_m),
        "no well reached a deep-rock Chunk"
    );
    for floor in floors() {
        assert!(
            (lowest..=highest).contains(&floor),
            "{floor} m is out of bounds"
        );
    }
    let sky = (highest / chunk_m).floor() as i32 + 1;
    for cz in -2..2 {
        for cx in -2..2 {
            assert_eq!(
                probe.with.chunk(cx, sky, cz).uniform(),
                Some(Voxel::AIR),
                "({cx}, {sky}, {cz}) is above the bounds but not open sky"
            );
        }
    }
}

#[test]
fn a_world_without_a_listed_structure_keeps_its_exact_bounds_and_clearance() {
    let dangling = BiomeStructure {
        template: 7,
        ..row(16.0, 1.0)
    };
    for size in SIZES {
        let plain = Worldgen::new(SEED, size, flat(&[]));
        let p = plain.params();
        assert_eq!(
            plain.surface_bounds_m(),
            (p.sea_level_m, p.sea_level_m + p.relief_m)
        );
        let unlisted = Worldgen::new(SEED, size, flat(&[])).with_templates(&[hut(), tower()]);
        let missing = Worldgen::new(SEED, size, flat(&[dangling])).with_templates(&[hut()]);
        for other in [unlisted, missing] {
            assert_eq!(other.surface_bounds_m(), plain.surface_bounds_m());
            for i in -8..8 {
                let min = [i as f32 * 3.7, i as f32 * -2.3];
                let max = min.map(|m| m + 0.6);
                assert_eq!(
                    other.clear_height_m(min, max),
                    plain.clear_height_m(min, max)
                );
            }
        }
    }
}

#[test]
fn no_body_clears_below_the_top_of_a_structure_it_stands_over() {
    for size in [1.0, 0.5] {
        let probe = Probe::new(size, flat(&[row(16.0, 1.0)]));
        let found = structures(&probe, size, 48.0);
        assert!(!found.is_empty(), "{size} m: nothing stood");
        for f in found {
            let [sx, sy, sz] = hut().sample(size, f.turn).size();
            let top_m = (f.corner[1] + sy) as f32 * size;
            let [x_m, _, z_m] = metres(f.corner, size);
            let [far_x, _, far_z] = metres([f.corner[0] + sx, 0, f.corner[2] + sz], size);
            // A body's width just inside each corner of the footprint.
            for min in [[x_m, z_m], [far_x - 0.6, far_z - 0.6]] {
                let min = min.map(|m| m + 0.1);
                let clear_m = probe.with.clear_height_m(min, min.map(|m| m + 0.4));
                assert!(
                    clear_m >= top_m,
                    "{size} m: {clear_m} under a roof at {top_m}"
                );
            }
        }
    }
}
