//! Structures in generation: a Biome lists Templates, and worldgen stands each one whole from any
//! Chunk's viewpoint, the same in metres at every Voxel size.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use tesserae_core::template::{Cell, Template, Turn};
use tesserae_core::terrain::LandformParams;
use tesserae_core::{
    AIR, Biome, BiomeName, BiomeStructure, BiomeTable, CHUNK_SIZE, CaveParams, Chunk,
    ClimateParams, GroundParams, MaterialId, SmoothingParams, SurfaceFeatures, Voxel, WaterParams,
    Worldgen, WorldgenParams,
};

mod common;

const CELLAR: MaterialId = 240;
const FLOOR: MaterialId = 241;
const MARK: MaterialId = 242;
const WALL: MaterialId = 243;
const STEP: MaterialId = 244;
const FOUNDATION: MaterialId = 245;
const STAIR_SHAPE: u8 = 2;
const STAIR_FACING: u8 = 2;

const SEED: u64 = 0x7e55_e2ae_0000_0001;
const SIZES: [f32; 3] = [1.0, 0.5, 0.25];
const SIZE_I: i32 = CHUNK_SIZE as i32;
/// Where scans are centred, along x and z: well clear of the origin, which keeps Structures out.
const AWAY_M: f32 = 128.0;
/// How far from a Structure's footprint version 3 fills any cave the ground would open.
const CAVE_CLEAR_M: f32 = 16.0;

/// 4 m along x, 5 m along z: a cellar, a floor marked at one corner so its turn shows, and walls
/// with a stair and a doorway. `k` is Keep, `.` Air.
fn hut() -> Template {
    hut_level_to(1.0)
}

fn hut_level_to(tolerance_m: f32) -> Template {
    let layers = [
        ["ccccc", "c..kc", "c...c", "ccccc"],
        ["mffff", "fffff", "fffff", "ffffk"],
        ["wswkw", "w...w", "w...w", "wwwww"],
    ];
    drawn("t:hut", &layers, 1, None, tolerance_m)
}

/// 4 m square with no cellar, taking 3 m of rise: a marked floor on the ground and a post at each
/// corner.
fn deck(foundation: Option<MaterialId>) -> Template {
    let layers = [
        ["mfff", "ffff", "ffff", "ffff"],
        ["w..w", "....", "....", "w..w"],
    ];
    drawn("t:deck", &layers, 0, foundation, 3.0)
}

/// Layers from the bottom up, each written as rows along x of cells along z.
fn drawn<const ROWS: usize>(
    name: &str,
    layers: &[[&str; ROWS]],
    ground_layer: usize,
    foundation: Option<MaterialId>,
    tolerance_m: f32,
) -> Template {
    let cell = |c: char| match c {
        'k' => Cell::Keep,
        '.' => Cell::Air,
        'c' => Cell::Voxel(Voxel::cube(CELLAR)),
        'f' => Cell::Voxel(Voxel::cube(FLOOR)),
        'm' => Cell::Voxel(Voxel::cube(MARK)),
        'w' => Cell::Voxel(Voxel::cube(WALL)),
        's' => Cell::Voxel(Voxel::new(STEP, STAIR_SHAPE, STAIR_FACING)),
        'g' => Cell::Ground,
        _ => unreachable!(),
    };
    // Written as rows along x of cells along z, so transpose into rows along z.
    let layers = layers
        .iter()
        .map(|rows| {
            (0..rows[0].len())
                .map(|z| {
                    rows.iter()
                        .map(|row| cell(row.as_bytes()[z] as char))
                        .collect()
                })
                .collect()
        })
        .collect();
    Template::new(
        name.to_owned(),
        1.0,
        layers,
        ground_layer,
        foundation,
        tolerance_m,
    )
    .unwrap()
}

fn row(spacing_m: f32, chance: f32) -> BiomeStructure {
    BiomeStructure {
        template: 0,
        spacing_m,
        chance,
    }
}

fn biome(structures: &[BiomeStructure]) -> Biome {
    named("t:plain", 0.5, structures)
}

fn named(name: &str, temperature: f32, structures: &[BiomeStructure]) -> Biome {
    Biome {
        name: BiomeName::new(name).unwrap(),
        temperature,
        humidity: 0.5,
        ground: None,
        sky_fog: None,
        cave_scale: None,
        surface_features: SurfaceFeatures::NONE.with_structures(structures).unwrap(),
    }
}

/// Gentle ground, so most draws stand.
fn flat(structures: &[BiomeStructure]) -> WorldgenParams {
    common::lowland(WorldgenParams {
        relief_m: 6.0,
        caves: None,
        smoothing: None,
        climate: ClimateParams {
            temperature_size_m: 64.0,
            humidity_size_m: 64.0,
        },
        biomes: BiomeTable::new(&[biome(structures)]).unwrap(),
        water: None,
        ..tesserae_core::testgen::world_params()
    })
}

/// A World generated twice, with and without its Structures, read one cell at a time from
/// whichever Chunk holds it.
struct Probe {
    template: Template,
    with: Worldgen,
    bare: Worldgen,
    chunks: RefCell<HashMap<[i32; 3], (Chunk, Chunk)>>,
    /// Where [`Probe::changed`] is centred, along x and z.
    centre_m: f32,
}

impl Probe {
    fn new(size: f32, rows: &[BiomeStructure]) -> Self {
        Self::of(size, hut(), flat, rows)
    }

    /// The World `world` makes with `rows`, against the one it makes with none.
    fn of(
        size: f32,
        template: Template,
        world: impl Fn(&[BiomeStructure]) -> WorldgenParams,
        rows: &[BiomeStructure],
    ) -> Self {
        Self::seeded(SEED, size, template, world, rows)
    }

    fn seeded(
        seed: u64,
        size: f32,
        template: Template,
        world: impl Fn(&[BiomeStructure]) -> WorldgenParams,
        rows: &[BiomeStructure],
    ) -> Self {
        Probe {
            with: Worldgen::new(seed, size, world(rows))
                .with_templates(std::slice::from_ref(&template)),
            bare: Worldgen::new(seed, size, world(&[])),
            template,
            chunks: RefCell::default(),
            centre_m: AWAY_M,
        }
    }

    /// The ground the Biome of column `[x, z]` lays, as declared.
    fn ground_of(&self, [x, z]: [i32; 2], size: f32) -> GroundParams {
        let biomes = self.bare.biomes();
        self.bare
            .biome_at(x as f32 * size, z as f32 * size)
            .and_then(|id| biomes.get(id))
            .and_then(|b| b.ground)
            .or(self.bare.params().ground)
            .unwrap()
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

    /// The same Probe scanning around (`centre_m`, `centre_m`).
    fn centred(self, centre_m: f32) -> Self {
        Probe { centre_m, ..self }
    }

    /// Every cell the Structures changed, in `half_m` of the centre, generating the Chunks in one
    /// order or its reverse.
    fn changed(&self, half_m: f32, reverse: bool) -> HashMap<[i32; 3], (Voxel, Voxel)> {
        self.changed_around(self.centre_m, half_m, reverse)
    }

    /// [`Probe::changed`] in `half_m` of (`centre_m`, `centre_m`), on a Chunk border.
    fn changed_around(
        &self,
        centre_m: f32,
        half_m: f32,
        reverse: bool,
    ) -> HashMap<[i32; 3], (Voxel, Voxel)> {
        let chunk_m = SIZE_I as f32 * self.with.voxel_size();
        let (low, high) = self.with.surface_bounds_m();
        let rows = (low / chunk_m).floor() as i32 - 1..=(high / chunk_m).ceil() as i32 + 1;
        let reach = (half_m / chunk_m).ceil() as i32;
        let centre = (centre_m / chunk_m).round() as i32;
        let (from, to) = (centre - reach, centre + reach);
        let mut coords: Vec<[i32; 3]> = rows
            .flat_map(|cy| (from..to).flat_map(move |cz| (from..to).map(move |cx| [cx, cy, cz])))
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
/// the Template's own sampling, a `ground` cell as the column's Biome lays its ground.
fn expected(probe: &Probe, size: f32, turn: Turn, mark: [i32; 3]) -> Vec<([i32; 3], Cell)> {
    let sampled = probe.template.sample(size, turn);
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
        .map(|(at, c)| {
            let at = [0, 1, 2].map(|a| corner[a] + at[a]);
            let c = match c {
                Cell::Ground => {
                    let ground = probe.ground_of([at[0], at[2]], size);
                    let below = at[1] < corner[1] + sampled.ground();
                    Cell::Voxel(Voxel::cube(if below {
                        ground.soil
                    } else {
                        ground.surface
                    }))
                }
                c => c,
            };
            (at, c)
        })
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
            Cell::Ground => unreachable!("resolved by `expected`"),
        }
    })
}

/// Every Structure in `half_m` of the Probe's centre, each whole by its Template however many
/// Chunks it spans, and nothing else changed but caves filled near it; the same whichever order the
/// Chunks are generated in.
fn structures(probe: &Probe, size: f32, half_m: f32) -> Vec<Found> {
    let changed = probe.changed(half_m, false);
    assert_eq!(changed, probe.changed(half_m, true), "{size} m: order");
    let mut owned = HashSet::new();
    let mut found = Vec::new();
    let mut footprints = Vec::new();
    for mark in marks(&changed) {
        let turns: Vec<Turn> = Turn::all()
            .filter(|&turn| holds(probe, &expected(probe, size, turn, mark)))
            .collect();
        assert_eq!(
            turns.len(),
            1,
            "{size} m: the Structure at {mark:?} is not whole"
        );
        let cells = expected(probe, size, turns[0], mark);
        owned.extend(cells.iter().map(|c| c.0));
        let corner = [0, 1, 2].map(|a| cells.iter().map(|c| c.0[a]).min().unwrap());
        let far = [0, 2].map(|a| cells.iter().map(|c| c.0[a]).max().unwrap());
        footprints.push([corner[0], corner[2], far[0], far[1]]);
        owned.extend(changed.iter().filter_map(|(&at, (with, _))| {
            let under = at[1] < corner[1]
                && (corner[0]..=far[0]).contains(&at[0])
                && (corner[2]..=far[1]).contains(&at[2]);
            (under && Some(with.material()) == probe.template.foundation()).then_some(at)
        }));
        found.push(Found {
            corner,
            mark,
            turn: turns[0],
        });
    }
    let clear = (CAVE_CLEAR_M / size).ceil() as i32;
    for (at, (with, bare)) in &changed {
        // The scan stops at `half_m`, so a cell near it may belong to a Structure beyond, or be a
        // cave it filled.
        let from_edge = |m: f32| {
            [at[0], at[2]]
                .iter()
                .any(|&a| (a as f32 * size - probe.centre_m).abs() > half_m - m)
        };
        let filled = bare.is_air()
            && !with.is_air()
            && (from_edge(8.0 + CAVE_CLEAR_M)
                || footprints.iter().any(|f| {
                    (f[0] - clear..=f[2] + clear).contains(&at[0])
                        && (f[1] - clear..=f[3] + clear).contains(&at[2])
                }));
        assert!(
            owned.contains(at) || from_edge(8.0) || filled,
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
    whole_and_equal(flat);
}

#[test]
fn over_caves_and_smoothed_ground_a_structure_is_whole_and_equal_at_every_voxel_size() {
    whole_and_equal(|rows| WorldgenParams {
        smoothing: Some(SmoothingParams::default()),
        ..caved(rows)
    });
}

fn whole_and_equal(world: impl Fn(&[BiomeStructure]) -> WorldgenParams + Copy) {
    let half_m = 48.0;
    let mut crossings = 0;
    let mut by_size = Vec::new();
    for size in SIZES {
        let probe = Probe::of(size, hut(), world, &[row(16.0, 0.8)]);
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
            if (x - AWAY_M).abs() < half_m - 8.0 && (z - AWAY_M).abs() < half_m - 8.0 {
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
    let probe = Probe::new(size, &[row(16.0, 1.0)]);
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
    let probe = Probe::new(size, &[row(16.0, 1.0)]);
    let found = structures(&probe, size, 32.0);
    assert!(!found.is_empty());
    let (mut kept_ground, mut carved) = (0, 0);
    for f in found {
        for (at, cell) in expected(&probe, size, f.turn, f.mark) {
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
    let probe = Probe::of(size, tower(), flat, &[row(8.0, 1.0)]);
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
fn no_structure_comes_within_48_m_of_the_origin_where_games_build_their_spawn() {
    let size = 1.0;
    let probe = Probe::new(size, &[row(16.0, 1.0)]);
    let changed = probe.changed_around(0.0, 96.0, false);
    let nearest = changed
        .keys()
        .map(|at| ((at[0] as f32 + 0.5).hypot(at[2] as f32 + 0.5)) * size)
        .fold(f32::MAX, f32::min);
    assert!(nearest < 96.0, "nothing stood further out either");
    assert!(
        nearest >= 48.0,
        "a Structure changed ground {nearest} m from the origin"
    );
}

#[test]
fn two_rows_of_one_spacing_never_stand_structures_that_touch() {
    let size = 1.0;
    let probe = Probe::new(size, &[row(16.0, 1.0), row(16.0, 1.0)]);
    let found = structures(&probe, size, 48.0);
    let one_row = structures(&Probe::new(size, &[row(16.0, 1.0)]), size, 48.0);
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

/// About half the ground under water: the sea, its floor let down below the low land.
fn lake(structures: &[BiomeStructure]) -> WorldgenParams {
    let params = flat(structures);
    WorldgenParams {
        water: Some(WaterParams {
            level_m: params.sea_level_m,
            material: tesserae_core::testgen::fixtures().water,
        }),
        landforms: LandformParams {
            depth_m: params.relief_m,
            ..params.landforms
        },
        ..params
    }
}

/// The first stretch on the diagonal about half under [`lake`]'s water.
fn coast_m() -> f32 {
    let g = Worldgen::new(SEED, 1.0, lake(&[]));
    (0..400)
        .map(|i| 64.0 * i as f32)
        .find(|&m| {
            let wet = (-8..=8)
                .flat_map(|i| (-8..=8).map(move |j| (i as f32 * 12.0, j as f32 * 12.0)))
                .filter(|&(x, z)| g.water_surface_m(m + x, m + z).is_some())
                .count();
            (90..200).contains(&wet)
        })
        .expect("a coast on the diagonal")
}

/// Every column under a found Structure, x then z.
fn columns(probe: &Probe, f: &Found, size: f32) -> Vec<[i32; 2]> {
    let [sx, _, sz] = probe.template.sample(size, f.turn).size();
    (f.corner[2]..f.corner[2] + sz)
        .flat_map(|z| (f.corner[0]..f.corner[0] + sx).map(move |x| [x, z]))
        .collect()
}

/// The top Voxel of the bare ground in column `[x, z]`.
fn top(probe: &Probe, [x, z]: [i32; 2], size: f32) -> i32 {
    let h = probe
        .bare
        .surface_height_m(x as f32 * size, z as f32 * size);
    (h / size).floor() as i32
}

fn places(found: &[Found]) -> Vec<([i32; 3], u8)> {
    let mut places: Vec<_> = found
        .iter()
        .map(|f| (f.corner, f.turn.quarters()))
        .collect();
    places.sort();
    places
}

#[test]
fn no_structure_stands_in_a_lake() {
    let size = 1.0;
    let rows = [row(10.0, 1.0)];
    let water = tesserae_core::testgen::fixtures().water;
    let wet = Probe::of(size, hut(), lake, &rows).centred(coast_m());
    let found = structures(&wet, size, 128.0);
    assert!(found.len() > 4, "only {} Structures", found.len());
    for f in &found {
        for column in columns(&wet, f, size) {
            let above = [column[0], top(&wet, column, size) + 1, column[1]];
            assert_ne!(
                wet.at(above).1.material(),
                water,
                "the Structure at {:?} stands in the lake",
                f.corner
            );
        }
    }
    let drained = |rows: &[BiomeStructure]| WorldgenParams {
        water: None,
        ..lake(rows)
    };
    let dry = structures(
        &Probe::of(size, hut(), drained, &rows).centred(coast_m()),
        size,
        128.0,
    );
    assert_ne!(places(&found), places(&dry), "the lake turned no draw away");
}

/// A cold Biome listing `structures` beside a warm one listing none.
fn two_biomes(structures: &[BiomeStructure]) -> WorldgenParams {
    WorldgenParams {
        biomes: BiomeTable::new(&[named("t:cold", 0.2, structures), named("t:warm", 0.8, &[])])
            .unwrap(),
        ..flat(&[])
    }
}

#[test]
fn a_structure_stands_only_in_a_biome_that_lists_it() {
    let size = 1.0;
    let probe = Probe::of(size, hut(), two_biomes, &[row(16.0, 1.0)]);
    let found = structures(&probe, size, 96.0);
    assert!(found.len() > 4, "only {} Structures", found.len());
    let biome = |[x, z]: [i32; 2]| {
        probe
            .bare
            .biome_at(x as f32 * size, z as f32 * size)
            .map(|b| b.index())
    };
    let mut warm = 0;
    for f in &found {
        let biomes: HashSet<_> = columns(&probe, f, size).into_iter().map(biome).collect();
        assert_eq!(
            biomes,
            HashSet::from([Some(0)]),
            "the Structure at {:?} stands outside the cold Biome",
            f.corner
        );
    }
    for z in (-96..96).step_by(4) {
        for x in (-96..96).step_by(4) {
            warm += usize::from(biome([x, z]) == Some(1));
        }
    }
    assert!(warm > 100, "the warm Biome is hardly there: {warm} columns");
}

/// Version 3's own land at the default relief: steep hills, scanned at [`hills_m`]. Without
/// overhangs, whose rock a Structure clears round it, so the ground under one is the ground its
/// rules asked.
fn hilly(structures: &[BiomeStructure]) -> WorldgenParams {
    let land = tesserae_core::testgen::world_params();
    WorldgenParams {
        relief_m: land.relief_m,
        landforms: LandformParams {
            overhangs: false,
            ..land.landforms
        },
        ..flat(structures)
    }
}

/// The first dry, hilly but not mountainous stretch on the diagonal, a whole number of 1 m
/// Chunks out.
fn hills_m() -> f32 {
    let g = Worldgen::new(SEED, 1.0, hilly(&[]));
    let sea_m = g.params().sea_level_m;
    (0..400)
        .map(|i| 64.0 * i as f32)
        .find(|&m| {
            let heights: Vec<f32> = (-4..=4)
                .flat_map(|i| (-4..=4).map(move |j| (i as f32 * 12.0, j as f32 * 12.0)))
                .map(|(x, z)| g.surface_height_m(m + x, m + z))
                .collect();
            let low = heights.iter().copied().fold(f32::MAX, f32::min);
            let high = heights.iter().copied().fold(f32::MIN, f32::max);
            low > sea_m + 4.0 && (48.0..128.0).contains(&(high - low))
        })
        .expect("hills on the diagonal")
}

#[test]
fn no_structure_stands_on_ground_steeper_than_its_tolerance() {
    let size = 1.0;
    let rows = [row(16.0, 1.0)];
    let strict = Probe::of(size, hut_level_to(1.0), hilly, &rows).centred(hills_m());
    let found = structures(&strict, size, 96.0);
    assert!(!found.is_empty(), "nothing stood on the hills");
    for f in &found {
        let heights: Vec<f32> = columns(&strict, f, size)
            .into_iter()
            .map(|[x, z]| {
                strict
                    .bare
                    .surface_height_m(x as f32 * size, z as f32 * size)
            })
            .collect();
        let rise = heights.iter().copied().fold(f32::MIN, f32::max)
            - heights.iter().copied().fold(f32::MAX, f32::min);
        assert!(
            rise <= 1.0,
            "the Structure at {:?} stands on {rise} m of rise",
            f.corner
        );
    }
    let lax = structures(
        &Probe::of(size, hut_level_to(16.0), hilly, &rows).centred(hills_m()),
        size,
        96.0,
    );
    assert!(
        lax.len() > found.len(),
        "a lax tolerance stood {} and a strict one {}",
        lax.len(),
        found.len()
    );
}

fn caved(structures: &[BiomeStructure]) -> WorldgenParams {
    WorldgenParams {
        caves: Some(CaveParams::default()),
        ..flat(structures)
    }
}

#[test]
fn no_structure_stands_over_a_cave_mouth() {
    let size = 1.0;
    let rows = [row(16.0, 1.0)];
    let probe = Probe::of(size, hut(), caved, &rows);
    let found = structures(&probe, size, 96.0);
    assert!(found.len() > 4, "only {} Structures", found.len());
    for f in &found {
        for column in columns(&probe, f, size) {
            let surface = [column[0], top(&probe, column, size), column[1]];
            assert!(
                !probe.at(surface).1.is_air(),
                "the Structure at {:?} stands over a cave mouth at {column:?}",
                f.corner
            );
        }
    }
    let solid = structures(&Probe::new(size, &rows), size, 96.0);
    assert_ne!(
        places(&found),
        places(&solid),
        "the caves turned no draw away"
    );
}

#[test]
fn a_cellar_is_underground_and_the_floor_level_with_the_surface() {
    for size in SIZES {
        let probe = Probe::new(size, &[row(16.0, 1.0)]);
        let found = structures(&probe, size, 48.0);
        assert!(found.len() > 4, "{size} m: only {} Structures", found.len());
        let mut cellar = 0;
        for f in &found {
            // The columns a metre apart from the corner, which the ground rules ask.
            let mut tops: Vec<i32> = columns(&probe, f, size)
                .into_iter()
                .filter(|c| {
                    [0, 1].iter().all(|&a| {
                        ((c[a] - [f.corner[0], f.corner[2]][a]) as f32 * size).fract() == 0.0
                    })
                })
                .map(|c| top(&probe, c, size))
                .collect();
            tops.sort();
            let median = tops[(tops.len() - 1) / 2];
            let floor_top = f.mark[1] + (1.0 / size) as i32 - 1;
            assert_eq!(floor_top, median, "{size} m: the floor at {:?}", f.mark);
            for (at, cell) in expected(&probe, size, f.turn, f.mark) {
                if matches!(cell, Cell::Voxel(v) if v.material() == CELLAR) {
                    let (_, bare) = probe.at(at);
                    assert!(!bare.is_air(), "{size} m: cellar at {at:?} is above ground");
                    cellar += 1;
                }
            }
        }
        assert!(cellar > 0);
    }
}

#[test]
fn a_foundation_fills_the_gap_on_uneven_ground_and_is_absent_without_one() {
    for size in [1.0, 0.5] {
        let rows = [row(10.0, 1.0)];
        let (mut filled, mut open) = (0, 0);
        for foundation in [Some(FOUNDATION), None] {
            let probe = Probe::of(size, deck(foundation), hilly, &rows).centred(hills_m());
            let found = structures(&probe, size, 96.0);
            assert!(found.len() > 4, "{size} m: only {} Structures", found.len());
            for f in &found {
                for c in columns(&probe, f, size) {
                    for y in top(&probe, c, size) + 1..f.corner[1] {
                        let (with, bare) = probe.at([c[0], y, c[1]]);
                        if foundation.is_some() {
                            assert_eq!(with, Voxel::cube(FOUNDATION), "{size} m: gap at {c:?}");
                            filled += 1;
                        } else {
                            assert_eq!(with, bare, "{size} m: filled at {c:?}");
                            open += 1;
                        }
                    }
                }
            }
        }
        assert!(
            filled > 0 && open > 0,
            "{size} m: no gap under any Structure"
        );
    }
}

const MEADOW: GroundParams = GroundParams {
    rock: 250,
    soil: 251,
    surface: 252,
};
const DESERT: GroundParams = GroundParams {
    rock: 253,
    soil: 254,
    surface: 255,
};

/// A 4 by 5 m plot of the local ground a layer deep, marked at one corner, with a post at each
/// corner.
fn plot() -> Template {
    let layers = [
        ["ggggg", "ggggg", "ggggg", "ggggg"],
        ["mgggg", "ggggg", "ggggg", "ggggg"],
        ["w...w", ".....", ".....", "w...w"],
    ];
    drawn("t:plot", &layers, 1, None, 1.0)
}

/// A meadow and a desert, both listing `structures`.
fn meadow_and_desert(structures: &[BiomeStructure]) -> WorldgenParams {
    WorldgenParams {
        biomes: BiomeTable::new(&[
            Biome {
                ground: Some(MEADOW),
                ..named("t:meadow", 0.2, structures)
            },
            Biome {
                ground: Some(DESERT),
                ..named("t:desert", 0.8, structures)
            },
        ])
        .unwrap(),
        ..flat(&[])
    }
}

#[test]
fn a_ground_cell_takes_grass_in_a_meadow_sand_in_a_desert_and_each_columns_own_across_a_border() {
    for size in [1.0, 0.5] {
        let probe = Probe::of(size, plot(), meadow_and_desert, &[row(16.0, 1.0)]);
        // Each found Structure is whole, its ground cells as its columns' own Biomes lay them.
        let found = structures(&probe, size, 96.0);
        let (mut meadow, mut desert, mut across) = (0, 0, 0);
        for f in &found {
            let grounds: HashSet<_> = columns(&probe, f, size)
                .into_iter()
                .map(|c| probe.ground_of(c, size).surface)
                .collect();
            match grounds.len() {
                2 => across += 1,
                _ if grounds.contains(&MEADOW.surface) => meadow += 1,
                _ => desert += 1,
            }
        }
        assert!(
            meadow > 0 && desert > 0 && across > 0,
            "{size} m: {meadow} on grass, {desert} on sand, {across} across a border"
        );
    }
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
        template: tower(),
        with: Worldgen::new(SEED, size, flat(&[row(8.0, 0.5), wells]))
            .with_templates(&[tower(), well()]),
        bare: Worldgen::new(SEED, size, flat(&[])),
        chunks: RefCell::default(),
        centre_m: AWAY_M,
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
        let probe = Probe::new(size, &[row(16.0, 1.0)]);
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

/// The centre, in metres, of each found hut's footprint.
fn centres(found: &[Found], size: f32) -> Vec<[f32; 2]> {
    found
        .iter()
        .map(|f| {
            let [sx, _, sz] = hut().sample(size, f.turn).size();
            [(f.corner[0], sx), (f.corner[2], sz)].map(|(c, s)| (c as f32 + s as f32 / 2.0) * size)
        })
        .collect()
}

#[test]
fn no_two_structures_of_a_row_stand_closer_than_their_spacing_nor_on_a_grid() {
    let size = 1.0;
    let spacing_m = 10.0;
    let found = structures(&Probe::new(size, &[row(spacing_m, 1.0)]), size, 128.0);
    let centres = centres(&found, size);
    assert!(centres.len() > 200, "only {} Structures", centres.len());
    let mut nearest = Vec::new();
    for (i, a) in centres.iter().enumerate() {
        let d = centres
            .iter()
            .enumerate()
            .filter(|&(j, _)| j != i)
            .map(|(_, b)| ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt())
            .fold(f32::MAX, f32::min);
        assert!(
            d >= spacing_m,
            "the Structure at {a:?} is {d} m from another"
        );
        nearest.push(d);
    }
    // Neither a grid nor clumps: nearest neighbours at many distances.
    let distinct: HashSet<_> = nearest.iter().map(|d| (d * 2.0) as i32).collect();
    assert!(distinct.len() >= 8, "nearest distances {distinct:?}");
    let offsets: HashSet<_> = centres
        .iter()
        .flat_map(|c| c.map(|v| (v.rem_euclid(spacing_m) * 2.0) as i32))
        .collect();
    assert!(offsets.len() >= 16, "offsets in a cell {offsets:?}");
}

#[test]
fn two_seeds_stand_structures_differently() {
    let size = 1.0;
    let rows = [row(10.0, 1.0)];
    let ours = places(&structures(&Probe::new(size, &rows), size, 64.0));
    let probe = Probe::seeded(SEED ^ 0xdead_beef, size, hut(), flat, &rows);
    let theirs = places(&structures(&probe, size, 64.0));
    let shared = theirs.iter().filter(|p| ours.contains(p)).count();
    assert!(
        shared * 10 < theirs.len(),
        "{shared} of {} alike",
        theirs.len()
    );
}

/// Version 3's rugged land, whose density stands rock above the heightfield, with huts that take
/// a few metres of rise.
fn rugged(structures: &[BiomeStructure]) -> WorldgenParams {
    WorldgenParams {
        sea_level_m: 24.0,
        relief_m: 24.0,
        landforms: LandformParams {
            overhang_m: 12.0,
            ..Default::default()
        },
        ..flat(structures)
    }
}

#[test]
fn no_overhang_stands_over_a_structure_or_beside_it() {
    let size = 1.0;
    let probe = Probe::of(size, hut_level_to(4.0), rugged, &[row(10.0, 1.0)]);
    // Rugged ground well inland, where the density reaches furthest.
    let centre_m = (0..400)
        .map(|i| 64.0 * i as f32)
        .find(|&m| probe.bare.landform_fields(m, m).ruggedness > 0.6)
        .expect("rugged ground on the diagonal");
    let changed = probe.changed_around(centre_m, 96.0, false);
    let found = marks(&changed);
    assert!(
        found.len() >= 3,
        "only {} huts on rugged ground",
        found.len()
    );
    let mut asked = 0;
    let ours = |v: Voxel| [CELLAR, FLOOR, MARK, WALL, STEP].contains(&v.material());
    for mark in found {
        // The hut's footprint, whichever way it turned, and four Voxels round it.
        let mut cells = vec![mark];
        let mut seen = HashSet::from([mark]);
        let mut i = 0;
        while i < cells.len() {
            let at = cells[i];
            i += 1;
            for d in [
                [1, 0, 0],
                [-1, 0, 0],
                [0, 1, 0],
                [0, -1, 0],
                [0, 0, 1],
                [0, 0, -1],
            ] {
                let next = [0, 1, 2].map(|a| at[a] + d[a]);
                if !seen.contains(&next) && changed.get(&next).is_some_and(|(w, _)| ours(*w)) {
                    seen.insert(next);
                    cells.push(next);
                }
            }
        }
        let span = |a: usize| {
            let lo = cells.iter().map(|c| c[a]).min().unwrap();
            let hi = cells.iter().map(|c| c[a]).max().unwrap();
            lo - 4..=hi + 4
        };
        for z in span(2) {
            for x in span(0) {
                let top = (probe.with.surface_height_m(x as f32, z as f32) / size).floor() as i32;
                for y in top + 1..top + 14 {
                    let (with, _) = probe.at([x, y, z]);
                    let g = probe.ground_of([x, z], size);
                    assert!(
                        ![g.rock, g.soil, g.surface].contains(&with.material()),
                        "terrain stands over the ground at {:?} by the hut at {mark:?}",
                        [x, y, z]
                    );
                    asked += 1;
                }
            }
        }
    }
    assert!(asked > 1000);
}
