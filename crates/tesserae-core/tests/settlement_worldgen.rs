//! Settlements in generation: a Biome lists a Settlement, and worldgen lays out its roads and Plots
//! whole from any Chunk's viewpoint, the same in metres at every Voxel size.

use std::collections::{HashMap, HashSet};

use tesserae_core::settlement::{Building, PlotRule, Settlement};
use tesserae_core::template::{Cell, Template, Turn};
use tesserae_core::testgen::fixtures;
use tesserae_core::{
    Biome, BiomeName, BiomeSettlement, BiomeStructure, BiomeTable, CHUNK_SIZE, ClimateParams,
    CropField, FieldCrop, MaterialId, SurfaceFeature, SurfaceFeatures, SurfacePlant,
    SurfacePlantKind, SurfaceTree, Voxel, Worldgen, WorldgenParams,
};

const FLOOR: MaterialId = 180;
const WALL: MaterialId = 181;
const DOOR: MaterialId = 182;
const PATH: MaterialId = 183;
const WELL: MaterialId = 184;
const HOUSE: MaterialId = 185;
const SHED: MaterialId = 186;
const LONE: MaterialId = 187;
const TRUNK: MaterialId = 188;
const CANOPY: MaterialId = 189;
const FLOWERS: MaterialId = 190;
const CROP: [MaterialId; 3] = [191, 192, 193];
const FURROW: MaterialId = 194;

const SEED: u64 = 0x7e55_e2ae_0000_0001;
const SIZES: [f32; 3] = [1.0, 0.5, 0.25];
const SIZE_I: i32 = CHUNK_SIZE as i32;
const RADIUS_M: f32 = 24.0;
const MARGIN_M: f32 = 8.0;
/// Twice the radius and margin, so every lattice cell's centre is the cell's own.
const SPACING_M: f32 = 64.0;

/// A Template drawn as rows along +Z of cells along +X, layers from the bottom up; the Ground layer
/// is the bottom one, its first cell `m` marking where it is and how it is turned.
fn drawn(name: &str, mark: MaterialId, layers: &[&[&str]]) -> Template {
    let cell = |c: char| match c {
        'm' => Cell::Voxel(Voxel::cube(mark)),
        'f' => Cell::Voxel(Voxel::cube(FLOOR)),
        'w' => Cell::Voxel(Voxel::cube(WALL)),
        'd' => Cell::Voxel(Voxel::cube(DOOR)),
        '.' => Cell::Air,
        'k' => Cell::Keep,
        _ => unreachable!(),
    };
    let layers = layers
        .iter()
        .map(|rows| {
            rows.iter()
                .map(|row| row.chars().map(cell).collect())
                .collect()
        })
        .collect();
    Template::new(name.to_owned(), 1.0, layers, 0, None, 1.0).unwrap()
}

/// 3 m square, no door.
fn well() -> Template {
    drawn(
        "t:well",
        WELL,
        &[&["mff", "fff", "fff"], &["www", "w.w", "www"]],
    )
}

/// 5 m square, its door on the -X edge.
fn house() -> Template {
    drawn(
        "t:house",
        HOUSE,
        &[
            &["mffff", "fffff", "fffff", "fffff", "fffff"],
            &["wwwww", "w...w", "d...w", "w...w", "wwwww"],
        ],
    )
}

/// 3 m along x by 4 m along z, its door on the +X edge.
fn shed() -> Template {
    drawn(
        "t:shed",
        SHED,
        &[&["mff", "fff", "fff", "fff"], &["www", "w.d", "w.w", "www"]],
    )
}

/// A single Structure for the wild, 2 m square.
fn lone() -> Template {
    drawn("t:lone", LONE, &[&["mm", "mm"]])
}

fn templates() -> Vec<Template> {
    vec![well(), house(), shed(), lone()]
}

fn building(templates: &[Template], index: u16) -> Building {
    Building {
        template: index,
        door: templates[usize::from(index)].door_cell(|m| m == DOOR),
    }
}

fn village() -> Settlement {
    let t = templates();
    Settlement {
        name: "t:village".into(),
        radius_m: RADIUS_M,
        margin_m: MARGIN_M,
        centre: Some(building(&t, 0)),
        plots: 8,
        rules: vec![
            PlotRule {
                building: building(&t, 1),
                weight: 2.0,
                min: 1,
                max: 5,
            },
            PlotRule {
                building: building(&t, 2),
                weight: 1.0,
                min: 1,
                max: 3,
            },
        ],
        minimum: 3,
        path: Voxel::cube(PATH),
    }
}

fn plain(features: SurfaceFeatures) -> WorldgenParams {
    WorldgenParams {
        relief_m: 3.0,
        caves: None,
        smoothing: None,
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
            surface_features: features,
        }])
        .unwrap(),
        ..tesserae_core::testgen::world_params()
    }
}

fn listing(settlements: bool) -> SurfaceFeatures {
    let rows = [BiomeSettlement {
        settlement: 0,
        spacing_m: SPACING_M,
    }];
    SurfaceFeatures::NONE
        .with_settlements(if settlements { &rows } else { &[] })
        .unwrap()
}

fn generator(size: f32, params: WorldgenParams, settlement: Settlement) -> Worldgen {
    Worldgen::new(SEED, size, params)
        .with_templates(&templates())
        .with_settlements(&[settlement])
}

/// Every cell `with` makes differently from `bare` in `half_m` of the origin, generating the
/// Chunks in one order or its reverse.
fn changed(
    with: &Worldgen,
    bare: &Worldgen,
    half_m: f32,
    reverse: bool,
) -> HashMap<[i32; 3], (Voxel, Voxel)> {
    let chunk_m = SIZE_I as f32 * with.voxel_size();
    let (low, high) = with.surface_bounds_m();
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
        let (w, b) = (with.chunk(c[0], c[1], c[2]), bare.chunk(c[0], c[1], c[2]));
        for z in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let (wv, bv) = (w.get(x, y, z), b.get(x, y, z));
                    if wv != bv {
                        let at = [x, y, z].map(|l| l as i32);
                        out.insert([0, 1, 2].map(|a| c[a] * SIZE_I + at[a]), (wv, bv));
                    }
                }
            }
        }
    }
    out
}

/// A Structure found standing: which Template, its lowest corner in Voxels and its turn.
#[derive(Clone, Copy, Debug)]
struct Found {
    template: usize,
    corner: [i32; 3],
    turn: Turn,
}

/// What the Settlement laid in a scan: its Structures and its road columns.
struct Laid {
    size: f32,
    found: Vec<Found>,
    road: HashSet<[i32; 2]>,
}

impl Laid {
    fn scan(size: f32, params: impl Fn(SurfaceFeatures) -> WorldgenParams, half_m: f32) -> Self {
        let with = generator(size, params(listing(true)), village());
        let bare = generator(size, params(listing(false)), village());
        let changed = changed(&with, &bare, half_m, false);
        assert_eq!(
            changed,
            self::changed(&with, &bare, half_m, true),
            "{size} m: order"
        );
        let templates = templates();
        let mut found = Vec::new();
        for (index, template) in templates.iter().enumerate().take(3) {
            let mark = [WELL, HOUSE, SHED][index];
            for at in changed.keys().copied() {
                if changed[&at].0.material() != mark {
                    continue;
                }
                let is_mark = |p: [i32; 3]| changed.get(&p).is_some_and(|c| c.0.material() == mark);
                if is_mark([at[0] - 1, at[1], at[2]])
                    || is_mark([at[0], at[1] - 1, at[2]])
                    || is_mark([at[0], at[1], at[2] - 1])
                {
                    continue;
                }
                let fits: Vec<Found> = Turn::all()
                    .filter_map(|turn| {
                        let sampled = template.sample(size, turn);
                        let first = first_mark(template, size, turn, mark)?;
                        let corner = [0, 1, 2].map(|a| at[a] - first[a]);
                        let [sx, sy, sz] = sampled.size();
                        let whole = (0..sy).all(|y| {
                            (0..sz).all(|z| {
                                (0..sx).all(|x| {
                                    let p = [corner[0] + x, corner[1] + y, corner[2] + z];
                                    match sampled.cell([x, y, z]) {
                                        Cell::Voxel(v) => {
                                            changed.get(&p).is_some_and(|c| c.0 == v)
                                                || (!changed.contains_key(&p) && v.is_air())
                                        }
                                        _ => true,
                                    }
                                })
                            })
                        });
                        whole.then_some(Found {
                            template: index,
                            corner,
                            turn,
                        })
                    })
                    .collect();
                assert_eq!(fits.len(), 1, "{size} m: {mark} at {at:?} is not whole");
                found.push(fits[0]);
            }
        }
        let road = changed
            .iter()
            .filter(|(_, (w, _))| w.material() == PATH)
            .map(|(at, _)| [at[0], at[2]])
            .collect();
        Laid { size, found, road }
    }

    fn rect(&self, f: &Found) -> [[i32; 2]; 2] {
        let [sx, _, sz] = templates()[f.template].sample(self.size, f.turn).size();
        [
            [f.corner[0], f.corner[2]],
            [f.corner[0] + sx, f.corner[2] + sz],
        ]
    }

    /// The centres, in metres, of the Settlements that stood fully inside `inner_m`.
    fn towns(&self, inner_m: f32) -> Vec<[f32; 2]> {
        let lattice = (0..)
            .map(|k| SPACING_M / 2.0 + k as f32 * SPACING_M)
            .take_while(|&c| c < inner_m);
        let centres: Vec<f32> = lattice.flat_map(|c| [c, -c]).collect();
        let mut towns = Vec::new();
        for &x in &centres {
            for &z in &centres {
                if x.abs() + RADIUS_M > inner_m || z.abs() + RADIUS_M > inner_m {
                    continue;
                }
                if self.road.iter().any(|c| self.within(*c, [x, z], RADIUS_M)) {
                    towns.push([x, z]);
                }
            }
        }
        towns
    }

    /// Whether `f` stands in the Settlement at `centre`.
    fn holds(&self, f: &Found, centre: [f32; 2]) -> bool {
        let [min, max] = self.rect(f);
        self.within([0, 1].map(|a| (min[a] + max[a]) / 2), centre, RADIUS_M)
    }

    fn within(&self, [x, z]: [i32; 2], centre: [f32; 2], radius_m: f32) -> bool {
        let p = [x, z].map(|g| (g as f32 + 0.5) * self.size);
        let [dx, dz] = [0, 1].map(|a| p[a] - centre[a]);
        dx * dx + dz * dz <= radius_m * radius_m
    }
}

fn first_mark(template: &Template, size: f32, turn: Turn, mark: MaterialId) -> Option<[i32; 3]> {
    let sampled = template.sample(size, turn);
    let [sx, sy, sz] = sampled.size();
    (0..sy)
        .flat_map(|y| (0..sz).flat_map(move |z| (0..sx).map(move |x| [x, y, z])))
        .find(|&at| matches!(sampled.cell(at), Cell::Voxel(v) if v.material() == mark))
}

#[test]
fn a_settlement_is_whole_across_chunk_borders_and_equal_in_metres_at_every_voxel_size() {
    let half_m = 64.0;
    let mut by_size = Vec::new();
    for size in SIZES {
        let laid = Laid::scan(size, plain, half_m);
        assert!(laid.towns(half_m).len() >= 2, "{size} m: few Settlements");
        let crossings = laid
            .found
            .iter()
            .filter(|f| {
                let [min, max] = laid.rect(f);
                (0..2).any(|a| min[a].div_euclid(SIZE_I) != (max[a] - 1).div_euclid(SIZE_I))
            })
            .count();
        assert!(
            crossings > 2,
            "{size} m: {crossings} Plots crossed a Chunk border"
        );
        let mut places: Vec<_> = laid
            .found
            .iter()
            .map(|f| {
                let [x, z] = [f.corner[0], f.corner[2]].map(|c| c as f32 * size);
                (f.template, x.to_bits(), z.to_bits(), f.turn.quarters())
            })
            .collect();
        places.sort();
        let mut road: Vec<[i32; 2]> = laid
            .road
            .iter()
            .map(|c| c.map(|g| ((g as f32 + 0.5) * size).floor() as i32))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        road.sort();
        by_size.push((places, road));
    }
    assert_eq!(by_size[0], by_size[1], "1 m and 0.5 m lay out differently");
    assert_eq!(by_size[0], by_size[2], "1 m and 0.25 m lay out differently");
}

#[test]
fn every_door_faces_a_road_no_two_plots_overlap_and_counts_keep_their_bounds() {
    let half_m = 96.0;
    let laid = Laid::scan(1.0, plain, half_m);
    let templates = templates();
    let towns = laid.towns(half_m);
    assert!(towns.len() >= 3, "only {} Settlements", towns.len());
    let mut doors = 0;
    // A Settlement the scan cuts may have its road outside it.
    for f in laid
        .found
        .iter()
        .filter(|f| towns.iter().any(|&c| laid.holds(f, c)))
    {
        let sampled = templates[f.template].sample(1.0, f.turn);
        let [sx, sy, sz] = sampled.size();
        let door = (0..sy)
            .flat_map(|y| (0..sz).flat_map(move |z| (0..sx).map(move |x| [x, y, z])))
            .find(|&at| matches!(sampled.cell(at), Cell::Voxel(v) if v.material() == DOOR));
        let Some([x, _, z]) = door else {
            continue;
        };
        doors += 1;
        // Out of the edge nearest the door.
        let edges = [
            (sz - 1 - z, [0, 1]),
            (sx - 1 - x, [1, 0]),
            (z, [0, -1]),
            (x, [-1, 0]),
        ];
        let (_, out) = edges.iter().min_by_key(|e| e.0).unwrap();
        let from = [f.corner[0] + x, f.corner[2] + z];
        let faces = (1..=6).any(|k| {
            laid.road
                .contains(&[from[0] + out[0] * k, from[1] + out[1] * k])
        });
        assert!(faces, "the door of {f:?} faces no road");
    }
    assert!(doors > 8, "only {doors} doors");
    let rects: Vec<_> = laid.found.iter().map(|f| laid.rect(f)).collect();
    for (i, a) in rects.iter().enumerate() {
        for b in &rects[i + 1..] {
            let apart = (0..2).any(|k| a[1][k] <= b[0][k] || b[1][k] <= a[0][k]);
            assert!(apart, "Plots {a:?} and {b:?} overlap");
        }
    }
    let rules = village().rules;
    for centre in towns {
        let mut counts = [0u8; 3];
        for f in &laid.found {
            if laid.holds(f, centre) {
                counts[f.template] += 1;
            }
        }
        assert_eq!(counts[0], 1, "{centre:?}: one centre");
        for (rule, &count) in rules.iter().zip(&counts[1..]) {
            assert!(
                (rule.min..=rule.max).contains(&count),
                "{centre:?}: {counts:?} outside its bounds"
            );
        }
        assert!(
            counts[1] + counts[2] >= village().minimum,
            "{centre:?}: {counts:?}"
        );
    }
}

#[test]
fn a_settlement_below_its_minimum_is_dropped() {
    let crowded = Settlement {
        plots: 8,
        minimum: 8,
        rules: vec![PlotRule {
            min: 8,
            max: 8,
            ..village().rules[0]
        }],
        ..village()
    };
    let size = 1.0;
    let with = generator(size, plain(listing(true)), crowded.clone());
    let bare = generator(size, plain(listing(false)), crowded);
    assert!(changed(&with, &bare, 64.0, false).is_empty());
}

#[test]
fn settlements_no_biome_lists_change_nothing() {
    let size = 0.5;
    let with = generator(size, plain(listing(false)), village());
    let bare = Worldgen::new(SEED, size, plain(listing(false)));
    assert!(changed(&with, &bare, 48.0, false).is_empty());
    assert_eq!(with.surface_bounds_m(), bare.surface_bounds_m());
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

/// Trees, flowers, crop fields and a single Structure every 16 m, around Settlements.
fn crowded(settlements: SurfaceFeatures) -> WorldgenParams {
    let mut field = CropField::new(
        (6.0, 10.0),
        32.0,
        Some(FURROW),
        Some(fixtures().water),
        None,
    );
    field
        .push(FieldCrop::new(5, &CROP, &[], 1.0).unwrap())
        .unwrap();
    let features = SurfaceFeatures::new(&[
        SurfaceFeature {
            material: TRUNK,
            spacing_m: 7.0,
            plant: Some(SurfacePlant {
                shape: 0,
                kind: SurfacePlantKind::Tree(oak()),
            }),
            clump: None,
            field: None,
        },
        SurfaceFeature::cube(FLOWERS, 3.0),
    ])
    .unwrap()
    .with_crop_fields(&[field])
    .unwrap()
    .with_structures(&[BiomeStructure {
        template: 3,
        spacing_m: 16.0,
        chance: 1.0,
    }])
    .unwrap()
    .with_settlements(settlements.settlements())
    .unwrap();
    plain(features)
}

#[test]
fn no_single_structure_field_or_tree_stands_in_a_settlement_and_no_structure_in_its_margin() {
    let half_m = 96.0;
    let size = 1.0;
    let laid = Laid::scan(size, crowded, half_m);
    let towns = laid.towns(half_m);
    assert!(!towns.is_empty(), "no Settlement stood among the trees");
    let with = generator(size, crowded(listing(true)), village());
    let wild = generator(size, crowded(listing(false)), village());
    let cells = changed(
        &with,
        &Worldgen::new(SEED, size, plain(listing(false))),
        half_m,
        false,
    );
    let wild_cells = changed(
        &wild,
        &Worldgen::new(SEED, size, plain(listing(false))),
        half_m,
        false,
    );
    let laid_in =
        |cells: &HashMap<[i32; 3], (Voxel, Voxel)>, materials: &[MaterialId], reach_m: f32| {
            cells
                .iter()
                .filter(|(at, (w, _))| {
                    materials.contains(&w.material())
                        && towns
                            .iter()
                            .any(|&c| laid.within([at[0], at[2]], c, reach_m))
                })
                .count()
        };
    let grown = [TRUNK, CANOPY, FLOWERS, FURROW, CROP[0], CROP[1], CROP[2]];
    assert_eq!(
        laid_in(&cells, &grown, RADIUS_M),
        0,
        "a tree, flower or field in a Settlement"
    );
    assert!(
        laid_in(&wild_cells, &grown, RADIUS_M) > 0,
        "nothing would have grown there"
    );
    assert_eq!(
        laid_in(&cells, &[LONE], RADIUS_M + MARGIN_M),
        0,
        "a single Structure in a Settlement's margin"
    );
    assert!(laid_in(&wild_cells, &[LONE], RADIUS_M + MARGIN_M) > 0);
    assert!(
        cells.values().any(|(w, _)| w.material() == LONE),
        "no single Structure anywhere"
    );
}

/// Hills enough that Plots are cut and filled and roads climb.
fn hilly(features: SurfaceFeatures) -> WorldgenParams {
    WorldgenParams {
        relief_m: 12.0,
        ..plain(features)
    }
}

/// A World's columns, read from whichever Chunk holds them.
struct Columns {
    generator: Worldgen,
    chunks: std::cell::RefCell<HashMap<[i32; 3], tesserae_core::Chunk>>,
}

impl Columns {
    fn new(generator: Worldgen) -> Self {
        Columns {
            generator,
            chunks: Default::default(),
        }
    }

    fn at(&self, at: [i32; 3]) -> Voxel {
        let chunk = at.map(|c| c.div_euclid(SIZE_I));
        let local = at.map(|c| c.rem_euclid(SIZE_I) as usize);
        let mut chunks = self.chunks.borrow_mut();
        chunks
            .entry(chunk)
            .or_insert_with(|| self.generator.chunk(chunk[0], chunk[1], chunk[2]))
            .get(local[0], local[1], local[2])
    }

    /// The highest non-air Voxel of the column and where it is.
    fn top(&self, [x, z]: [i32; 2]) -> (i32, Voxel) {
        let size = self.generator.voxel_size();
        let (_, high) = self.generator.surface_bounds_m();
        let mut y = (high / size).ceil() as i32 + 1;
        loop {
            let v = self.at([x, y, z]);
            if !v.is_air() {
                return (y, v);
            }
            y -= 1;
        }
    }

    /// The height, in metres, a body walks on in the column: half a Voxel lower on a slab or a
    /// stair's low side.
    fn walked_m(&self, column: [i32; 2]) -> f32 {
        let size = self.generator.voxel_size();
        let (y, v) = self.top(column);
        let half = if v.shape() == 0 { 1.0 } else { 0.5 };
        (y as f32 + half) * size
    }
}

/// The columns within `by` metres of a Plot, and outside it.
fn around(laid: &Laid, f: &Found, by: f32) -> Vec<[i32; 2]> {
    let reach = (by / laid.size).round() as i32;
    let [min, max] = laid.rect(f);
    (min[1] - reach..max[1] + reach)
        .flat_map(|z| (min[0] - reach..max[0] + reach).map(move |x| [x, z]))
        .filter(|&[x, z]| !((min[0]..max[0]).contains(&x) && (min[1]..max[1]).contains(&z)))
        .collect()
}

#[test]
fn every_plot_is_level_and_no_cut_or_fill_exceeds_two_metres() {
    let half_m = 96.0;
    for size in [1.0, 0.5] {
        let laid = Laid::scan(size, hilly, half_m);
        let towns = laid.towns(half_m);
        assert!(!towns.is_empty(), "{size} m: no Settlement on the hills");
        let with = Columns::new(generator(size, hilly(listing(true)), village()));
        let bare = Columns::new(generator(size, hilly(listing(false)), village()));
        let mut moved_m = 0.0f32;
        let plots: Vec<_> = laid
            .found
            .iter()
            .filter(|f| towns.iter().any(|&c| laid.holds(f, c)))
            .collect();
        assert!(plots.len() > 4, "{size} m: only {} Plots", plots.len());
        for f in plots {
            let [min, max] = laid.rect(f);
            let floor = f.corner[1] + templates()[f.template].sample(size, f.turn).ground();
            for z in min[1]..max[1] {
                for x in min[0]..max[0] {
                    assert!(
                        !with.at([x, floor - 1, z]).is_air(),
                        "{size} m: {f:?} is not on level ground at {x}, {z}"
                    );
                }
            }
            for column in around(&laid, f, 2.0) {
                if laid.road.contains(&column) {
                    continue;
                }
                let shift_m = (with.top(column).0 - bare.top(column).0) as f32 * size;
                assert!(
                    shift_m.abs() <= 2.0,
                    "{size} m: {f:?} moves {column:?} by {shift_m} m"
                );
                moved_m = moved_m.max(shift_m.abs());
            }
        }
        assert!(moved_m >= 0.5, "{size} m: no apron was cut or filled");
    }
}

#[test]
fn the_apron_blends_into_the_ground_with_no_cliff_at_its_edge() {
    let half_m = 96.0;
    for size in [1.0, 0.5] {
        let laid = Laid::scan(size, hilly, half_m);
        let towns = laid.towns(half_m);
        let with = Columns::new(generator(size, hilly(listing(true)), village()));
        let bare = Columns::new(generator(size, hilly(listing(false)), village()));
        let rects: Vec<_> = laid.found.iter().map(|f| laid.rect(f)).collect();
        let open = |[x, z]: [i32; 2]| {
            !laid.road.contains(&[x, z])
                && !rects.iter().any(|[min, max]| {
                    (min[0]..max[0]).contains(&x) && (min[1]..max[1]).contains(&z)
                })
        };
        let mut pairs = 0;
        for f in laid
            .found
            .iter()
            .filter(|f| towns.iter().any(|&c| laid.holds(f, c)))
        {
            for column in around(&laid, f, 3.0).into_iter().filter(|&c| open(c)) {
                for next in [[column[0] + 1, column[1]], [column[0], column[1] + 1]] {
                    if !open(next) {
                        continue;
                    }
                    pairs += 1;
                    let step = |c: &Columns| (c.top(column).0 - c.top(next).0).abs() as f32 * size;
                    // The apron eases 2 m over 3 m at most, on top of the ground's own slope.
                    let allowed = step(&bare) + 2.0 / 3.0 * size + size;
                    assert!(
                        step(&with) <= allowed,
                        "{size} m: a cliff of {} m between {column:?} and {next:?}",
                        step(&with)
                    );
                }
            }
        }
        assert!(pairs > 100, "{size} m: only {pairs} pairs");
    }
}

#[test]
fn a_road_never_rises_more_than_a_slab_per_step() {
    let half_m = 64.0;
    for size in SIZES {
        let laid = Laid::scan(size, hilly, half_m);
        let with = Columns::new(generator(size, hilly(listing(true)), village()));
        let (mut steps, mut climbs) = (0, 0);
        for &column in &laid.road {
            let here = with.walked_m(column);
            for next in [[column[0] + 1, column[1]], [column[0], column[1] + 1]] {
                if !laid.road.contains(&next) {
                    continue;
                }
                let rise = (with.walked_m(next) - here).abs();
                assert!(
                    rise <= size / 2.0 + 1e-4,
                    "{size} m: the road rises {rise} m from {column:?} to {next:?}"
                );
                steps += 1;
                climbs += usize::from(rise > 0.0);
            }
            assert_eq!(with.top(column).1.material(), PATH, "{size} m: {column:?}");
        }
        assert!(steps > 200, "{size} m: only {steps} road steps");
        assert!(climbs > 10, "{size} m: the roads never climb");
    }
}

#[test]
fn wilderness_structures_keep_the_strict_rule_and_are_never_levelled() {
    let half_m = 96.0;
    let size = 1.0;
    let hilly_crowded = |s| WorldgenParams {
        relief_m: 12.0,
        ..crowded(s)
    };
    let laid = Laid::scan(size, hilly_crowded, half_m);
    let towns = laid.towns(half_m);
    assert!(!towns.is_empty());
    let with = generator(size, hilly_crowded(listing(true)), village());
    let wild = generator(size, hilly_crowded(listing(false)), village());
    let changed = changed(&with, &wild, half_m - RADIUS_M - MARGIN_M, false);
    // Every centre on the lattice, whether its Settlement stood or not.
    let centres: Vec<[f32; 2]> = (-2..2)
        .flat_map(|i| (-2..2).map(move |j| [i, j].map(|k| (k as f32 + 0.5) * SPACING_M)))
        .collect();
    // A tree that would reach into a disc is left out whole, so its own cells change a little
    // further out.
    let tree_m = 6.0;
    for at in changed.keys() {
        assert!(
            centres
                .iter()
                .any(|&c| laid.within([at[0], at[2]], c, RADIUS_M + MARGIN_M + tree_m)),
            "{at:?} changed outside every Settlement's disc and margin"
        );
    }
    assert!(
        changed.values().any(|(w, _)| w.material() == LONE) || {
            let lone = Columns::new(with);
            (-60..60).any(|x| (-60..60).any(|z| lone.top([x, z]).1.material() == LONE))
        },
        "no single Structure stands"
    );
}

#[test]
fn chunks_are_the_same_from_threads_sharing_one_generator_in_any_order() {
    let size = 0.5;
    let make = || generator(size, crowded(listing(true)), village());
    let shared = make();
    let chunk_m = SIZE_I as f32 * size;
    let (low, high) = shared.surface_bounds_m();
    let rows = (low / chunk_m).floor() as i32..=(high / chunk_m).ceil() as i32;
    let reach = (48.0 / chunk_m).ceil() as i32;
    let coords: Vec<[i32; 3]> = rows
        .flat_map(|cy| {
            (-reach..reach).flat_map(move |cz| (-reach..reach).map(move |cx| [cx, cy, cz]))
        })
        .collect();
    // A generator of its own for each Chunk shares nothing with any other Chunk.
    let alone: Vec<_> = coords
        .iter()
        .map(|&[x, y, z]| make().chunk(x, y, z))
        .collect();
    let threads = 4;
    let seen: Vec<Vec<(usize, tesserae_core::Chunk)>> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..threads)
            .map(|t| {
                let (generator, coords) = (shared.clone(), &coords);
                scope.spawn(move || {
                    let mut order: Vec<usize> = (0..coords.len()).collect();
                    order.rotate_left(t * coords.len() / threads);
                    if t % 2 == 1 {
                        order.reverse();
                    }
                    order
                        .into_iter()
                        .map(|i| {
                            let [x, y, z] = coords[i];
                            (i, generator.chunk(x, y, z))
                        })
                        .collect()
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    assert!(
        alone.iter().any(|c| c.uniform().is_none()),
        "nothing generated"
    );
    let voxels = |c: &tesserae_core::Chunk| {
        let mut all = Vec::with_capacity(CHUNK_SIZE.pow(3));
        for z in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    all.push(c.get(x, y, z));
                }
            }
        }
        all
    };
    for (i, chunk) in seen.into_iter().flatten() {
        assert!(
            voxels(&chunk) == voxels(&alone[i]),
            "{:?} differs",
            coords[i]
        );
    }
}

#[test]
fn sky_and_deep_rock_beyond_a_settlement_keep_their_early_outs() {
    let rock = Voxel::cube(fixtures().ground.rock);
    for size in SIZES {
        let with = generator(size, crowded(listing(true)), village());
        let bare = Worldgen::new(SEED, size, plain(listing(false)));
        let chunk_m = SIZE_I as f32 * size;
        let (low, high) = with.surface_bounds_m();
        assert!(
            (low, high) != bare.surface_bounds_m(),
            "the Settlement does not widen the bounds"
        );
        let sky = (high / chunk_m).floor() as i32 + 1;
        let deep = ((low - with.params().soil_depth_m) / chunk_m).floor() as i32 - 1;
        let reach = (48.0 / chunk_m).ceil() as i32;
        for cz in -reach..reach {
            for cx in -reach..reach {
                assert_eq!(with.chunk(cx, sky, cz).uniform(), Some(Voxel::AIR));
                assert_eq!(with.chunk(cx, deep, cz).uniform(), Some(rock));
            }
        }
    }
}
