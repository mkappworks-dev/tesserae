//! Version 3's underground: lava near the band's floor, perched aquifers and their springs, and
//! cave Biomes chosen in 3D.

#![allow(
    clippy::disallowed_methods,
    reason = "a test's own maths is not generation"
)]

use std::collections::HashMap;

use tesserae_core::terrain::LandformParams;
use tesserae_core::{
    BiomeName, CHUNK_SIZE, CaveBiome, CaveBiomeTable, CaveFeature, Chunk, MaterialId, SkyFog,
    Voxel, Worldgen, WorldgenParams,
};

const SEED: u64 = 0x7e55_e2ae_0000_0009;
const LAVA: MaterialId = 210;
const CRYSTAL: MaterialId = 211;
const MOSS: MaterialId = 212;
const GLOW: MaterialId = 213;
const N: i32 = CHUNK_SIZE as i32;

fn water() -> MaterialId {
    tesserae_core::testgen::fixtures().water
}

fn crystal_cave() -> CaveBiome {
    CaveBiome::new(
        BiomeName::new("t:crystal").unwrap(),
        (0.5, 0.5),
        None,
        Some(CRYSTAL),
        Some(MOSS),
        Some(SkyFog {
            sky: [10, 20, 60],
            fog: [20, 40, 120],
        }),
        &[CaveFeature {
            material: GLOW,
            shape: 0,
            spacing_m: 3.0,
        }],
    )
    .unwrap()
}

fn params(caves: &[CaveBiome], lava: bool) -> WorldgenParams {
    WorldgenParams {
        sea_level_m: 24.0,
        relief_m: 24.0,
        landforms: LandformParams::default(),
        smoothing: None,
        caves: Some(tesserae_core::CaveParams::default()),
        water: Some(tesserae_core::worldgen::WaterParams {
            level_m: 9999.0,
            material: water(),
        }),
        lava: lava.then_some(LAVA),
        cave_biomes: CaveBiomeTable::new(caves).unwrap(),
        ..tesserae_core::testgen::world_params()
    }
}

/// Voxels by global position over a few Chunk columns, from the band's floor up past the caverns.
struct Region {
    voxels: HashMap<[i32; 3], Voxel>,
    size: f32,
}

impl Region {
    fn new(g: &Worldgen, columns: &[[i32; 2]], ys: std::ops::Range<i32>, reverse: bool) -> Self {
        let mut coords: Vec<[i32; 3]> = columns
            .iter()
            .flat_map(|&[cx, cz]| ys.clone().map(move |cy| [cx, cy, cz]))
            .collect();
        if reverse {
            coords.reverse();
        }
        let mut voxels = HashMap::new();
        for [cx, cy, cz] in coords {
            let chunk: Chunk = g.chunk(cx, cy, cz);
            for z in 0..CHUNK_SIZE {
                for y in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        let at = [cx * N + x as i32, cy * N + y as i32, cz * N + z as i32];
                        voxels.insert(at, chunk.get(x, y, z));
                    }
                }
            }
        }
        Region {
            voxels,
            size: g.voxel_size(),
        }
    }

    fn count(&self, material: MaterialId) -> usize {
        self.voxels
            .values()
            .filter(|v| v.material() == material)
            .count()
    }

    fn metres(&self, at: [i32; 3]) -> [f32; 3] {
        at.map(|g| g as f32 * self.size)
    }
}

/// A square of Chunk columns at 1 m, and the same metres at 0.5 m.
fn columns(size: f32) -> Vec<[i32; 2]> {
    let per = (1.0 / size) as i32;
    (0..3 * per)
        .flat_map(|i| (0..3 * per).map(move |j| [8 * per + i, -5 * per + j]))
        .collect()
}

fn band(size: f32) -> std::ops::Range<i32> {
    let per = (1.0 / size) as i32;
    -4 * per..2 * per
}

/// The first `want` Chunk columns, on a coarse walk, that `keep` takes, each with its neighbours.
fn find(want: usize, keep: impl Fn(f32, f32) -> bool) -> Vec<Vec<[i32; 2]>> {
    let mut found = Vec::new();
    for i in -40..40 {
        for j in -40..40 {
            let [x, z] = [i as f32 * 157.0 + 16.0, j as f32 * 149.0 + 16.0];
            if keep(x, z) {
                let [cx, cz] = [x, z].map(|m| (m / 32.0).floor() as i32);
                found.push(
                    (-1..2)
                        .flat_map(|a| (-1..2).map(move |b| [cx + a, cz + b]))
                        .collect(),
                );
                if found.len() == want {
                    return found;
                }
            }
        }
    }
    found
}

#[test]
fn caverns_near_the_bands_floor_hold_lava_sealed_off_from_every_cavern_above() {
    let g = Worldgen::new(SEED, 1.0, params(&[], true));
    // The band's floor, 108 m under Sea level, and 10 m of lava over it.
    let lava_top_m = 24.0 - 96.0 - 12.0 + 10.0;
    let mut lava = 0;
    for columns in find(12, |_, _| true) {
        let region = Region::new(&g, &columns, -4..-1, false);
        for (&at, v) in &region.voxels {
            if v.material() != LAVA {
                continue;
            }
            lava += 1;
            assert!(region.metres(at)[1] < lava_top_m, "lava at {at:?}");
            for d in [
                [1, 0, 0],
                [-1, 0, 0],
                [0, 0, 1],
                [0, 0, -1],
                [0, -1, 0],
                [0, 1, 0],
            ] {
                let next = [0, 1, 2].map(|a| at[a] + d[a]);
                if let Some(n) = region.voxels.get(&next) {
                    assert_ne!(n.material(), water(), "lava at {at:?} meets water");
                }
            }
        }
    }
    assert!(lava > 100, "{lava} lava Voxels");
    let dry = Region::new(
        &Worldgen::new(SEED, 1.0, params(&[], false)),
        &columns(1.0),
        band(1.0),
        false,
    );
    assert_eq!(dry.count(LAVA), 0, "no lava without the Game's");
}

#[test]
fn a_perched_aquifer_raises_its_regions_lakes_and_wells_up_in_sealed_springs() {
    let g = Worldgen::new(SEED, 1.0, params(&[], false));
    let world_table_m = 24.0 - 2.0;
    let raised = (-40..40)
        .flat_map(|i| (-40..40).map(move |j| [i as f32 * 96.0, j as f32 * 96.0]))
        .filter(|&[x, z]| g.water_table_m(x, z) > world_table_m)
        .count();
    assert!(
        (1600..=2900).contains(&raised),
        "{raised} of 6400 columns stand over a perched aquifer"
    );
    // Level wherever a region's lake stands.
    for [x, z] in [[10.0, 10.0], [500.0, -300.0], [-900.0, 700.0]] {
        let table = g.water_table_m(x, z);
        for d in [0.0, 30.0, 90.0, 150.0] {
            let cell = |m: f32| (m / 192.0).floor();
            if cell(x + d) == cell(x) {
                assert_eq!(g.water_table_m(x + d, z), table);
            }
        }
    }
}

#[test]
fn springs_stand_in_rock_and_never_beside_air() {
    let g = Worldgen::new(SEED, 1.0, params(&[], false));
    // High ground over a perched aquifer, whose caverns stand dry over its lakes.
    let wet = find(30, |x, z| {
        g.water_table_m(x, z) > 40.0 && g.surface_height_m(x, z) > 100.0
    });
    let mut springs = 0;
    for cols in &wet {
        let region = Region::new(&g, cols, 0..3, false);
        for (&at, v) in &region.voxels {
            if v.material() != water() {
                continue;
            }
            let table_m = g.water_table_m(at[0] as f32, at[2] as f32);
            if (at[1] as f32) < table_m {
                continue;
            }
            springs += 1;
            for d in [[1, 0, 0], [-1, 0, 0], [0, 0, 1], [0, 0, -1], [0, -1, 0]] {
                let next = [0, 1, 2].map(|a| at[a] + d[a]);
                if let Some(n) = region.voxels.get(&next) {
                    assert!(!n.is_air(), "the spring at {at:?} pours into {next:?}");
                }
            }
        }
    }
    assert!(
        springs > 0,
        "no spring in {} aquifer regions",
        wet.len().min(40)
    );
}

#[test]
fn a_cave_biome_is_chosen_the_same_at_every_voxel_size_and_chunk_order() {
    let caves = [crystal_cave()];
    let rock_cells = |size: f32, reverse: bool| {
        let g = Worldgen::new(SEED, size, params(&caves, false));
        let region = Region::new(&g, &columns(size), band(size), reverse);
        let per = (1.0 / size) as i32;
        let mut cells: Vec<[i32; 3]> = region
            .voxels
            .iter()
            .filter(|(at, v)| v.material() == CRYSTAL && at.iter().all(|g| g % per == 0))
            .map(|(at, _)| at.map(|g| g / per))
            .collect();
        cells.sort_unstable();
        cells
    };
    let at_1 = rock_cells(1.0, false);
    assert!(at_1.len() > 1000, "{} crystal Voxels", at_1.len());
    assert_eq!(rock_cells(1.0, true), at_1, "Chunk order");
    let at_half: std::collections::HashSet<_> = rock_cells(0.5, false).into_iter().collect();
    let shared = at_1.iter().filter(|c| at_half.contains(*c)).count();
    assert!(
        shared * 100 >= at_1.len() * 97,
        "{shared} of {} crystal metres alike at 0.5 m",
        at_1.len()
    );
}

#[test]
fn a_cave_biome_floors_its_caverns_stands_features_on_them_and_fogs_its_air() {
    let caves = [crystal_cave()];
    let g = Worldgen::new(SEED, 1.0, params(&caves, false));
    let high = find(6, |x, z| g.surface_height_m(x, z) > 100.0);
    let region = Region::new(&g, &high.concat(), 0..3, false);
    let (mut floors, mut glows) = (0, 0);
    for (&at, v) in &region.voxels {
        let below = region.voxels.get(&[at[0], at[1] - 1, at[2]]);
        match v.material() {
            MOSS => floors += 1,
            GLOW => {
                glows += 1;
                assert!(
                    below.is_some_and(|b| !b.is_air() && b.material() != water()),
                    "a feature at {at:?} stands on nothing"
                );
            }
            _ => {}
        }
    }
    assert!(floors > 200, "{floors} floor Voxels");
    assert!(glows > 10, "{glows} features");
    let air = region
        .voxels
        .iter()
        .find(|(at, v)| {
            v.is_air() && g.surface_height_m(at[0] as f32, at[2] as f32) - at[1] as f32 > 30.0
        })
        .map(|(at, _)| *at)
        .expect("a cavern");
    let fog = g
        .cave_fog_at(air[0] as f32, air[1] as f32, air[2] as f32)
        .expect("the cave's fog");
    assert_eq!(fog.fog.map(|c| (c * 255.0).round() as u8), [20, 40, 120]);
    let surface = g.surface_height_m(0.0, 0.0);
    assert_eq!(g.cave_fog_at(0.0, surface + 1.0, 0.0), None, "the open air");
}

#[test]
fn a_table_holds_at_most_16_cave_biomes_each_named_once() {
    let named = |i: usize| {
        CaveBiome::new(
            BiomeName::new(&format!("t:cave{i}")).unwrap(),
            (0.5, 0.5),
            None,
            None,
            None,
            None,
            &[],
        )
        .unwrap()
    };
    let seventeen: Vec<_> = (0..17).map(named).collect();
    assert_eq!(
        CaveBiomeTable::new(&seventeen).unwrap_err().field,
        "cave_biomes"
    );
    assert!(CaveBiomeTable::new(&seventeen[..16]).is_ok());
    assert!(CaveBiomeTable::new(&[named(1), named(1)]).is_err());
}

#[test]
fn cave_biomes_near_the_corners_of_the_cave_climate_each_take_a_share() {
    let corner = |name: &str, at: (f32, f32)| {
        CaveBiome::new(
            BiomeName::new(name).unwrap(),
            at,
            None,
            None,
            None,
            None,
            &[],
        )
        .unwrap()
    };
    let caves = [
        corner("t:cold_dry", (0.2, 0.2)),
        corner("t:warm_wet", (0.8, 0.8)),
        corner("t:middle", (0.5, 0.5)),
    ];
    let g = Worldgen::new(SEED, 1.0, params(&caves, false));
    let mut counts: HashMap<String, u32> = HashMap::new();
    for i in 0..40 {
        for j in 0..40 {
            let [x, z] = [i as f32 * 53.0, j as f32 * 47.0];
            let y = g.surface_height_m(x, z) - 40.0;
            if let Some(name) = g.cave_biome_name_at(x, y, z) {
                *counts.entry(name.to_owned()).or_default() += 1;
            }
        }
    }
    let total: u32 = counts.values().sum();
    for cave in &caves {
        let n = counts.get(cave.name.as_str()).copied().unwrap_or(0);
        assert!(n * 8 > total, "{}: {n} of {total}", cave.name);
    }
}

const HUT: MaterialId = 214;

/// A 4 m square of one Material on the Ground layer, a Structure to keep caves from.
fn hut() -> tesserae_core::template::Template {
    use tesserae_core::template::Cell;
    let floor = vec![vec![Cell::Voxel(Voxel::cube(HUT)); 4]; 4];
    tesserae_core::template::Template::new("t:hut".into(), 1.0, vec![floor], 0, None, 4.0).unwrap()
}

fn with_huts(size: f32) -> Worldgen {
    let plain = tesserae_core::Biome {
        name: BiomeName::new("t:plain").unwrap(),
        temperature: 0.5,
        humidity: 0.5,
        ground: None,
        sky_fog: None,
        cave_scale: Some(4.0),
        surface_features: tesserae_core::SurfaceFeatures::NONE
            .with_structures(&[tesserae_core::BiomeStructure {
                template: 0,
                spacing_m: 48.0,
                chance: 1.0,
            }])
            .unwrap(),
    };
    let params = WorldgenParams {
        biomes: tesserae_core::BiomeTable::new(&[plain]).unwrap(),
        ..params(&[], false)
    };
    Worldgen::new(SEED, size, params).with_templates(&[hut()])
}

/// Within `near_m` of a hut, and every Voxel of open air more than `depth_m` under the ground.
fn caves_by_huts(g: &Worldgen, near_m: f32, depth_m: f32) -> (usize, usize, usize) {
    let size = g.voxel_size();
    // `find` answers in 1 m Chunks; a finer size holds the same metres in more.
    let per = (1.0 / size) as i32;
    let columns: Vec<[i32; 2]> = find(4, |x, z| g.surface_height_m(x, z) > 50.0)
        .concat()
        .into_iter()
        .flat_map(|[cx, cz]| {
            (0..per).flat_map(move |i| (0..per).map(move |j| [cx * per + i, cz * per + j]))
        })
        .collect();
    let region = Region::new(g, &columns, -2 * per..4 * per, false);
    let huts: Vec<[f32; 2]> = region
        .voxels
        .iter()
        .filter(|(_, v)| v.material() == HUT)
        .map(|(at, _)| [at[0] as f32 * size, at[2] as f32 * size])
        .collect();
    let (mut near, mut far) = (0, 0);
    for (&at, v) in &region.voxels {
        let [x, y, z] = region.metres(at);
        if !v.is_air() || g.surface_height_m(x, z) - y < depth_m {
            continue;
        }
        if huts.iter().any(|h| (h[0] - x).hypot(h[1] - z) < near_m) {
            near += 1;
        } else {
            far += 1;
        }
    }
    (huts.len(), near, far)
}

#[test]
fn no_cave_opens_under_or_beside_a_structure_in_version_3() {
    for size in [1.0, 0.5] {
        let (huts, near, far) = caves_by_huts(&with_huts(size), 16.0, 14.0);
        assert!(huts > 20, "{size} m: {huts} hut Voxels");
        assert_eq!(near, 0, "{size} m: cave Voxels within 16 m of a hut");
        assert!(far > 100, "{size} m: {far} cave Voxels elsewhere");
    }
}
