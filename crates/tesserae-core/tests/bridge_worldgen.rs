//! Bridges in generation: a Template that spans stands only where it crosses a gap, a valley dry
//! or with water in it, from one dry landing to the other, and leaves the gap under it as it was.

use std::cell::RefCell;
use std::collections::HashMap;

use tesserae_core::template::{Cell, Template, Turn};
use tesserae_core::terrain::LandformParams;
use tesserae_core::{
    Biome, BiomeName, BiomeStructure, BiomeTable, CHUNK_SIZE, Chunk, ClimateParams, MaterialId,
    SurfaceFeatures, Voxel, WaterParams, Worldgen, WorldgenParams,
};

mod common;

const DECK: MaterialId = 241;
const MARK: MaterialId = 242;
const RAIL: MaterialId = 243;
const PIER: MaterialId = 244;
const FOUNDATION: MaterialId = 245;
const SOLID: [MaterialId; 4] = [DECK, MARK, RAIL, PIER];

const SEED: u64 = 0x7e55_e2ae_0000_0001;
const SIZES: [f32; 3] = [1.0, 0.5, 0.25];
const SIZE_I: i32 = CHUNK_SIZE as i32;
/// Where the scans look for bridges, each side of [`centre_m`].
const HALF_M: f32 = 128.0;
const SPAN: usize = 15;

/// 15 m along x and 3 m across: a deck on the Ground layer marked at one corner, and rails. Under
/// the deck, piers at its ends and its middle, and between them air and the ground's own cells.
fn bridge() -> Template {
    let x_rows = |end: &'static str, middle: &'static str| {
        let mut rows = vec![end; SPAN];
        rows[1..SPAN - 1].fill(middle);
        rows
    };
    let mut under = x_rows("ppp", "g.g");
    under[SPAN / 2] = "ppp";
    let mut deck = x_rows("fff", "fff");
    deck[0] = "mff";
    let layers = [under, deck, x_rows("w.w", "w.w")];
    let cell = |c: char| match c {
        '.' => Cell::Air,
        'g' => Cell::Ground,
        'f' => Cell::Voxel(Voxel::cube(DECK)),
        'm' => Cell::Voxel(Voxel::cube(MARK)),
        'w' => Cell::Voxel(Voxel::cube(RAIL)),
        'p' => Cell::Voxel(Voxel::cube(PIER)),
        _ => unreachable!(),
    };
    let layers = layers
        .iter()
        .map(|x_rows| {
            (0..3)
                .map(|z| {
                    x_rows
                        .iter()
                        .map(|r| cell(r.as_bytes()[z] as char))
                        .collect()
                })
                .collect()
        })
        .collect();
    Template::new("t:bridge".into(), 1.0, layers, 1, Some(FOUNDATION), 2.0).unwrap()
}

fn spanning() -> Template {
    bridge().spanning().unwrap()
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Land {
    /// Hills and river valleys, no water.
    Dry,
    /// The same, its rivers in their valleys.
    Wet,
    /// Ground that rises and falls by less than a gap.
    Flat,
}

/// Without overhangs, whose rock a gap is not measured by, so the ground under a span is the
/// ground the generator measured.
fn params(structures: &[BiomeStructure], land: Land) -> WorldgenParams {
    let base = WorldgenParams {
        relief_m: if land == Land::Flat { 1.5 } else { 32.0 },
        feature_size_m: 24.0,
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
            surface_features: SurfaceFeatures::NONE.with_structures(structures).unwrap(),
        }])
        .unwrap(),
        water: None,
        landforms: LandformParams {
            overhangs: false,
            ..LandformParams::default()
        },
        ..tesserae_core::testgen::world_params()
    };
    let params = WorldgenParams {
        water: (land == Land::Wet).then(|| WaterParams {
            level_m: base.sea_level_m,
            material: tesserae_core::testgen::fixtures().water,
        }),
        ..base
    };
    match land {
        Land::Flat => common::lowland(params),
        _ => params,
    }
}

const ROW: BiomeStructure = BiomeStructure {
    template: 0,
    spacing_m: 32.0,
    chance: 1.0,
};

/// The first river well inland on a coarse walk: version 3's valleys narrow enough to bridge are
/// its rivers' cuts, dry where the World has no water. A whole number of 1 m Chunks, so it is a
/// Chunk corner at every size.
fn centre_m() -> [f32; 2] {
    let bare = Worldgen::new(SEED, 1.0, params(&[], Land::Wet));
    let inland_m = bare.params().sea_level_m + 20.0;
    let (x_m, z_m) = (-60..60)
        .flat_map(|i| (-60..60).map(move |j| (i as f32 * 97.0, j as f32 * 89.0)))
        .find(|&(x, z)| bare.water_surface_m(x, z).is_some_and(|w| w > inland_m))
        .expect("a river");
    [x_m, z_m].map(|m| (m / SIZE_I as f32).floor() * SIZE_I as f32)
}

/// A World with the Template listed and the same World without, Chunk by Chunk.
struct Probe {
    with: Worldgen,
    bare: Worldgen,
    bare_chunks: RefCell<HashMap<[i32; 3], Chunk>>,
    centre_m: [f32; 2],
}

impl Probe {
    fn new(size: f32, template: Template, land: Land) -> Self {
        Probe {
            with: Worldgen::new(SEED, size, params(&[ROW], land))
                .with_templates(std::slice::from_ref(&template)),
            bare: Worldgen::new(SEED, size, params(&[], land)),
            bare_chunks: RefCell::default(),
            centre_m: centre_m(),
        }
    }

    /// The Chunk holding the centre, along x and z.
    fn centre_chunk(&self) -> [i32; 2] {
        let chunk_m = SIZE_I as f32 * self.with.voxel_size();
        self.centre_m.map(|m| (m / chunk_m) as i32)
    }

    /// Every cell in `HALF_M` of the centre the Template changed, generated in one order or its
    /// reverse, as it is with the Template and without.
    fn changed(&self, reverse: bool) -> HashMap<[i32; 3], (Voxel, Voxel)> {
        let chunk_m = SIZE_I as f32 * self.with.voxel_size();
        // The ground's own band here, and room for a Template's height above and its piers below.
        let (mut low, mut high) = (f32::MAX, f32::MIN);
        for i in -32..=32 {
            for j in -32..=32 {
                let [x, z] = [i, j].map(|k| k as f32 * HALF_M / 32.0);
                let h = self
                    .bare
                    .surface_height_m(self.centre_m[0] + x, self.centre_m[1] + z);
                (low, high) = (low.min(h), high.max(h));
            }
        }
        let rows =
            ((low - 24.0) / chunk_m).floor() as i32..=((high + 24.0) / chunk_m).ceil() as i32;
        let reach = (HALF_M / chunk_m).ceil() as i32;
        let [mx, mz] = self.centre_chunk();
        let mut coords: Vec<[i32; 3]> = rows
            .flat_map(|cy| {
                (mz - reach..mz + reach)
                    .flat_map(move |cz| (mx - reach..mx + reach).map(move |cx| [cx, cy, cz]))
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
            if with.same_packing(&bare) {
                continue;
            }
            for z in 0..CHUNK_SIZE {
                for y in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        let (w, b) = (with.get(x, y, z), bare.get(x, y, z));
                        if w != b {
                            let at = [x, y, z].map(|l| l as i32);
                            out.insert([0, 1, 2].map(|a| c[a] * SIZE_I + at[a]), (w, b));
                        }
                    }
                }
            }
        }
        out
    }

    /// The bare World's cell at `at`.
    fn bare_at(&self, at: [i32; 3]) -> Voxel {
        let chunk = at.map(|c| c.div_euclid(SIZE_I));
        let [x, y, z] = at.map(|c| c.rem_euclid(SIZE_I) as usize);
        self.bare_chunks
            .borrow_mut()
            .entry(chunk)
            .or_insert_with(|| self.bare.chunk(chunk[0], chunk[1], chunk[2]))
            .get(x, y, z)
    }
}

/// A bridge found standing: its footprint's corner and its turn, from its mark.
#[derive(Clone, Copy, Debug)]
struct Found {
    corner: [i32; 3],
    turn: Turn,
}

/// Each bridge, from the corner of each block of `MARK` Voxels: the turn whose deck, laid from
/// there, the World holds.
fn bridges(changed: &HashMap<[i32; 3], (Voxel, Voxel)>, size: f32) -> Vec<Found> {
    let centre = centre_m().map(|m| (m / size) as i32);
    let is = |at: [i32; 3], m| changed.get(&at).is_some_and(|(w, _)| w.material() == m);
    let template = spanning();
    let mut found = Vec::new();
    for (&[x, y, z], _) in changed.iter().filter(|(_, (w, _))| w.material() == MARK) {
        if is([x - 1, y, z], MARK) || is([x, y - 1, z], MARK) || is([x, y, z - 1], MARK) {
            continue;
        }
        // The scan stops at `HALF_M`, so a bridge near it may be cut.
        if [x - centre[0], z - centre[1]]
            .iter()
            .any(|&g| g.abs() as f32 * size > HALF_M - 16.0)
        {
            continue;
        }
        let turns: Vec<Found> = Turn::all()
            .filter_map(|turn| {
                let sampled = template.sample(size, turn);
                let [sx, _, sz] = sampled.size();
                let ground = sampled.ground();
                let deck: Vec<([i32; 2], Cell)> = (0..sz)
                    .flat_map(|dz| (0..sx).map(move |dx| [dx, dz]))
                    .map(|[dx, dz]| ([dx, dz], sampled.cell([dx, ground, dz])))
                    .collect();
                let mark = deck
                    .iter()
                    .find(|(_, c)| matches!(c, Cell::Voxel(v) if v.material() == MARK))?
                    .0;
                let corner = [x - mark[0], y - ground, z - mark[1]];
                deck.iter()
                    .all(|&([dx, dz], c)| match c {
                        Cell::Voxel(v) => is([corner[0] + dx, y, corner[2] + dz], v.material()),
                        _ => true,
                    })
                    .then_some(Found { corner, turn })
            })
            .collect();
        assert_eq!(
            turns.len(),
            1,
            "{size} m: the bridge at {:?} is not whole",
            [x, y, z]
        );
        found.push(turns[0]);
    }
    found.sort_by_key(|f| f.corner);
    found
}

/// Every bridge of a World of `land`, at every Voxel size: whole in any Chunk order, the same in
/// metres, its landings dry, a gap under its middle third, and nothing under its span, ground or
/// water, changed but by its own solid cells. Returns how many have water under them.
fn crossings(land: Land) -> usize {
    let water = tesserae_core::testgen::fixtures().water;
    let mut by_size = Vec::new();
    let mut wet = 0;
    for size in SIZES {
        let probe = Probe::new(size, spanning(), land);
        let changed = probe.changed(false);
        assert_eq!(changed, probe.changed(true), "{size} m: order");
        let found = bridges(&changed, size);
        assert!(
            found.len() >= 2,
            "{land:?}, {size} m: only {} bridges",
            found.len()
        );
        let per_m = (1.0 / size) as i32;
        wet = 0;
        for f in &found {
            let [sx, _, sz] = spanning().sample(size, f.turn).size();
            let (span, across) = if sx > sz { (sx, sz) } else { (sz, sx) };
            let column = |k: i32, j: i32| {
                if sx > sz {
                    [f.corner[0] + k, f.corner[2] + j]
                } else {
                    [f.corner[0] + j, f.corner[2] + k]
                }
            };
            // The deck's top Voxel layer, which stands on the landings' ground.
            let deck = f.corner[1] + spanning().sample(size, f.turn).ground_top();
            let depth = (deck - 20 * per_m..deck + 2 * per_m).rev();
            let ground = |[x, z]: [i32; 2]| {
                depth
                    .clone()
                    .find(|&y| {
                        let v = probe.bare_at([x, y, z]);
                        !v.is_air() && v.material() != water
                    })
                    .unwrap()
            };
            let flooded = |[x, z]: [i32; 2]| {
                depth
                    .clone()
                    .any(|y| probe.bare_at([x, y, z]).material() == water)
            };
            // As the generator asks it: the ground below its column's water level, which may lie
            // within a Voxel of it and so show no water Voxel.
            let under_water = |[x, z]: [i32; 2]| {
                probe
                    .bare
                    .water_surface_m(x as f32 * size, z as f32 * size)
                    .is_some()
            };
            let span_m = span as f32 * size;
            // The ground rules ask whole metres.
            let middle_third = (0..span)
                .step_by(per_m as usize)
                .filter(|&k| (span_m / 3.0..2.0 * span_m / 3.0).contains(&(k as f32 * size)));
            for j in (0..across).step_by(per_m as usize) {
                for k in (0..2 * per_m)
                    .chain(span - 2 * per_m..span)
                    .step_by(per_m as usize)
                {
                    assert!(!flooded(column(k, j)), "{size} m: a wet landing at {f:?}");
                }
                // Ground two metres below the deck's Voxel, or under the water.
                for k in middle_third.clone() {
                    let top = ground(column(k, j)) as f32 * size;
                    let gap = (deck + 1) as f32 * size - top;
                    assert!(
                        gap > 2.0 || under_water(column(k, j)),
                        "{size} m: no gap under the middle at {f:?}"
                    );
                }
            }
            for k in 2 * per_m..span - 2 * per_m {
                for j in 0..across {
                    let [x, z] = column(k, j);
                    for y in deck - 20 * per_m..deck {
                        if let Some((with, _)) = changed.get(&[x, y, z]) {
                            assert!(
                                SOLID.contains(&with.material()),
                                "{size} m: {with:?} at {:?} under the span of {f:?}",
                                [x, y, z]
                            );
                        }
                    }
                }
            }
            wet += usize::from(middle_third.clone().any(|k| flooded(column(k, across / 2))));
        }
        let mut places: Vec<_> = found
            .iter()
            .map(|f| {
                let m = f.corner.map(|c| c as f32 * size);
                (m[0].to_bits(), m[2].to_bits(), f.turn.quarters())
            })
            .collect();
        places.sort();
        by_size.push(places);
    }
    assert_eq!(
        by_size[0], by_size[1],
        "1 m and 0.5 m stand different bridges"
    );
    assert_eq!(
        by_size[0], by_size[2],
        "1 m and 0.25 m stand different bridges"
    );
    wet
}

#[test]
fn a_bridge_crosses_a_dry_valley_from_one_landing_to_the_other_and_leaves_it_as_it_was() {
    assert_eq!(crossings(Land::Dry), 0);
}

#[test]
fn a_bridge_crosses_a_valley_with_water_in_it_and_keeps_the_water_under_it() {
    assert!(crossings(Land::Wet) > 0, "no bridge crossed water");
}

#[test]
fn a_bridge_never_stands_on_flat_ground() {
    let probe = Probe::new(1.0, spanning(), Land::Flat);
    assert!(
        probe.changed(false).is_empty(),
        "a bridge stood on flat ground"
    );
    let dry = Probe::new(1.0, bridge(), Land::Flat);
    assert!(
        !bridges(&dry.changed(false), 1.0).is_empty(),
        "the same Template not spanning stands there"
    );
}

/// A hilly stretch, found on a coarse walk, stands bridges over water and dry gaps alike.
#[test]
fn bridges_stand_over_valleys() {
    for land in [Land::Dry, Land::Wet] {
        let with = Worldgen::new(SEED, 1.0, params(&[ROW], land)).with_templates(&[spanning()]);
        let bare = Worldgen::new(SEED, 1.0, params(&[], land));
        let (x_m, z_m) = (-60..60)
            .flat_map(|i| (-60..60).map(move |j| (i as f32 * 97.0, j as f32 * 89.0)))
            .find(|&(x, z)| {
                bare.surface_height_m(x, z) > 60.0 && bare.water_surface_m(x, z).is_none()
            })
            .expect("hilly land");
        let (low, high) = bare.surface_bounds_m();
        let centre = [x_m, z_m].map(|m| (m / SIZE_I as f32).floor() as i32);
        let mut marks = 0;
        for cz in centre[1] - 12..centre[1] + 12 {
            for cx in centre[0] - 12..centre[0] + 12 {
                let rows = (low / SIZE_I as f32).floor() as i32 - 1
                    ..=(high / SIZE_I as f32).ceil() as i32 + 1;
                for cy in rows {
                    let chunk = with.chunk(cx, cy, cz);
                    marks += (0..CHUNK_SIZE)
                        .flat_map(|z| {
                            (0..CHUNK_SIZE)
                                .flat_map(move |y| (0..CHUNK_SIZE).map(move |x| (x, y, z)))
                        })
                        .filter(|&(x, y, z)| chunk.get(x, y, z).material() == MARK)
                        .count();
                }
            }
        }
        assert!(marks > 0, "{land:?}: no bridge near ({x_m}, {z_m})");
    }
}
