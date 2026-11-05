//! Ore through the generator's chunk function: where a vein appears, and
//! everything it must leave alone.

use std::collections::HashMap;
use tesserae_core::testgen::fixtures;
use tesserae_core::{
    Biome, BiomeId, BiomeName, BiomeSet, BiomeTable, CHUNK_SIZE, Chunk, ClimateParams, Ore,
    OreTable, Voxel, Worldgen, WorldgenParams,
};

const SEED: u64 = 0x7e55_e2ae_0000_0001;

fn ore(material: u16, min_depth_m: f32, max_depth_m: f32) -> Ore {
    Ore {
        material,
        min_depth_m,
        max_depth_m,
        vein_size_m: 6.0,
        rarity: 1.0,
        biomes: BiomeSet::default(),
    }
}

/// An ore that fits anywhere in the ground, of a Material the ground never uses.
fn anywhere() -> Ore {
    ore(fixtures().lantern, 0.0, 400.0)
}

/// Caves, smoothing and a lake, so each property is checked against everything shaping a Chunk.
fn params(ores: &[Ore]) -> WorldgenParams {
    WorldgenParams {
        ores: OreTable::new(ores).unwrap(),
        ..tesserae_core::testgen::world_params().with_default_lake(fixtures().water)
    }
}

fn bare(ores: &[Ore]) -> WorldgenParams {
    WorldgenParams {
        caves: None,
        smoothing: None,
        water: None,
        ..params(ores)
    }
}

fn generator(params: WorldgenParams, voxel_size: f32) -> Worldgen {
    Worldgen::new(SEED, voxel_size, params)
}

/// Each Voxel's lower corner, in metres.
fn each_voxel(
    g: &Worldgen,
    rows: std::ops::Range<i32>,
    reach: i32,
    mut visit: impl FnMut([f32; 3], Voxel),
) {
    let size = CHUNK_SIZE as i32;
    let voxel = g.voxel_size();
    for cy in rows {
        for cz in -reach..reach {
            for cx in -reach..reach {
                let chunk = g.chunk(cx, cy, cz);
                for z in 0..CHUNK_SIZE {
                    for y in 0..CHUNK_SIZE {
                        for x in 0..CHUNK_SIZE {
                            let at = [
                                (cx * size + x as i32) as f32 * voxel,
                                (cy * size + y as i32) as f32 * voxel,
                                (cz * size + z as i32) as f32 * voxel,
                            ];
                            visit(at, chunk.get(x, y, z));
                        }
                    }
                }
            }
        }
    }
}

fn ore_voxels(g: &Worldgen, material: u16) -> Vec<[f32; 3]> {
    let mut found = Vec::new();
    each_voxel(g, -2..3, 1, |at, v| {
        if v.material() == material {
            found.push(at);
        }
    });
    found
}

fn depth_m(g: &Worldgen, [x, y, z]: [f32; 3]) -> f32 {
    g.surface_height_m(x, z) - y
}

#[test]
fn a_declared_ore_appears_in_the_rock() {
    let g = generator(params(&[anywhere()]), 1.0);
    assert!(ore_voxels(&g, fixtures().lantern).len() > 500);
}

#[test]
fn ore_replaces_rock_and_nothing_else() {
    let (with, without) = (
        generator(params(&[anywhere()]), 1.0),
        generator(params(&[]), 1.0),
    );
    let rock = fixtures().ground.rock;
    let mut replaced = 0;
    for cy in -2..3 {
        for cz in -2..2 {
            for cx in -2..2 {
                let (a, b) = (with.chunk(cx, cy, cz), without.chunk(cx, cy, cz));
                for z in 0..CHUNK_SIZE {
                    for y in 0..CHUNK_SIZE {
                        for x in 0..CHUNK_SIZE {
                            let (ore, plain) = (a.get(x, y, z), b.get(x, y, z));
                            if ore == plain {
                                continue;
                            }
                            // Soil, surface, water, air, stairs and slabs are all
                            // somebody else's Voxel.
                            assert_eq!(plain, Voxel::cube(rock), "{x},{y},{z} in {cx},{cy},{cz}");
                            assert_eq!(ore, Voxel::cube(fixtures().lantern));
                            replaced += 1;
                        }
                    }
                }
            }
        }
    }
    assert!(replaced > 500, "{replaced}");
}

#[test]
fn ore_appears_only_inside_its_depth_band() {
    let g = generator(params(&[ore(fixtures().lantern, 12.0, 20.0)]), 1.0);
    let found = ore_voxels(&g, fixtures().lantern);
    assert!(found.len() > 100, "{}", found.len());
    for at in found {
        let depth = depth_m(&g, at);
        assert!((12.0..=20.0).contains(&depth), "{depth} m at {at:?}");
    }
}

#[test]
fn ore_below_the_surface_band_fills_the_deep_chunks_it_reaches() {
    // Every Voxel of this Chunk is deeper than 25 m, below the soil.
    let deep = |band: (f32, f32)| {
        generator(
            WorldgenParams {
                caves: None,
                ..params(&[ore(fixtures().lantern, band.0, band.1)])
            },
            1.0,
        )
        .chunk(0, -1, 0)
    };
    let count = |chunk: &Chunk| {
        (0..CHUNK_SIZE)
            .flat_map(|z| {
                (0..CHUNK_SIZE).flat_map(move |y| (0..CHUNK_SIZE).map(move |x| (x, y, z)))
            })
            .filter(|&(x, y, z)| chunk.get(x, y, z).material() == fixtures().lantern)
            .count()
    };
    assert!(count(&deep((30.0, 60.0))) > 100);
    // A band no Voxel of the Chunk reaches leaves it the uniform rock the
    // meshing and light fast paths key off.
    let shallow = deep((0.0, 5.0));
    assert_eq!(shallow.uniform(), Some(Voxel::cube(fixtures().ground.rock)));
}

fn biome(name: &str, temperature: f32) -> Biome {
    Biome {
        name: BiomeName::new(name).unwrap(),
        temperature,
        humidity: 0.5,
        ground: None,
        sky_fog: None,
        cave_scale: None,
        surface_features: Default::default(),
    }
}

/// Cold (id 0) and warm (id 1), in regions small enough that the sweep crosses a border.
fn two_biomes(ores: &[Ore]) -> WorldgenParams {
    WorldgenParams {
        climate: ClimateParams {
            temperature_size_m: 48.0,
            humidity_size_m: 48.0,
        },
        biomes: BiomeTable::new(&[biome("cold", 0.2), biome("warm", 0.8)]).unwrap(),
        ..bare(ores)
    }
}

fn in_biomes(ids: &[u8]) -> BiomeSet {
    BiomeSet::new(&ids.iter().map(|&i| BiomeId::new(i)).collect::<Vec<_>>()).unwrap()
}

#[test]
fn ore_appears_only_in_its_biomes() {
    let warm_only = Ore {
        biomes: in_biomes(&[1]),
        ..anywhere()
    };
    let g = generator(two_biomes(&[warm_only]), 1.0);
    let found = ore_voxels(&g, fixtures().lantern);
    assert!(found.len() > 100);
    for [x, _, z] in found {
        assert_eq!(g.biome_at(x, z), Some(BiomeId::new(1)), "{x},{z}");
    }
}

#[test]
fn an_unrestricted_ore_appears_in_every_biome() {
    let g = generator(two_biomes(&[anywhere()]), 1.0);
    let mut seen = [0usize; 2];
    for [x, _, z] in ore_voxels(&g, fixtures().lantern) {
        seen[g.biome_at(x, z).expect("a Biome").index()] += 1;
    }
    assert!(seen.iter().all(|&n| n > 100), "{seen:?}");
}

#[test]
fn an_ore_restricted_to_biomes_is_absent_from_a_world_without_any() {
    let warm_only = Ore {
        biomes: in_biomes(&[1]),
        ..anywhere()
    };
    let g = generator(bare(&[warm_only]), 1.0);
    assert!(ore_voxels(&g, fixtures().lantern).is_empty());
}

struct Probe<'a> {
    g: &'a Worldgen,
    chunks: HashMap<[i32; 3], Chunk>,
}

impl<'a> Probe<'a> {
    fn new(g: &'a Worldgen) -> Self {
        Probe {
            g,
            chunks: HashMap::new(),
        }
    }

    fn at(&mut self, [x, y, z]: [i32; 3]) -> Voxel {
        let voxel = self.g.voxel_size();
        let index = [x, y, z].map(|m| (m as f32 / voxel).round() as i32);
        let size = CHUNK_SIZE as i32;
        let coord = index.map(|i| i.div_euclid(size));
        let g = self.g;
        let chunk = self
            .chunks
            .entry(coord)
            .or_insert_with(|| g.chunk(coord[0], coord[1], coord[2]));
        let [lx, ly, lz] = index.map(|i| i.rem_euclid(size) as usize);
        chunk.get(lx, ly, lz)
    }
}

#[test]
fn the_same_veins_appear_at_every_voxel_size() {
    let make = |voxel_size| generator(bare(&[anywhere()]), voxel_size);
    let reference = make(1.0);
    // Even metres, which every size samples; four metres under the surface or
    // deeper, where all of them see rock rather than the surface or soil.
    let points: Vec<[i32; 3]> = (-32..32)
        .step_by(2)
        .flat_map(|x| (-8..40).step_by(2).map(move |y| (x, y)))
        .flat_map(|(x, y)| (-32..32).step_by(2).map(move |z| [x, y, z]))
        .filter(|&[x, y, z]| depth_m(&reference, [x as f32, y as f32, z as f32]) >= 4.0)
        .collect();
    let ore_at = |size: f32| {
        let g = make(size);
        let mut probe = Probe::new(&g);
        points
            .iter()
            .map(|&p| probe.at(p).material() == fixtures().lantern)
            .collect::<Vec<_>>()
    };
    let one = ore_at(1.0);
    assert!(one.iter().filter(|&&o| o).count() > 500);
    for size in [2.0, 0.5] {
        assert!(ore_at(size) == one, "{size} m Voxels");
    }
}

#[test]
fn a_vein_is_a_blob_no_wider_than_its_size() {
    let g = generator(bare(&[anywhere()]), 0.5);
    let mut longest = 0.0f32;
    let mut run = 0usize;
    // A run of ore along x through the middle of a Chunk row.
    for cx in -2..2 {
        let chunk = g.chunk(cx, 0, 0);
        for z in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    if chunk.get(x, y, z).material() == fixtures().lantern {
                        run += 1;
                        longest = longest.max(run as f32 * 0.5);
                    } else {
                        run = 0;
                    }
                }
                run = 0;
            }
        }
    }
    assert!(longest > 2.0 && longest <= 6.0, "{longest} m");
}

#[test]
fn rarity_is_the_fraction_of_veins_that_are_real() {
    let count = |rarity| {
        let o = Ore {
            rarity,
            ..anywhere()
        };
        ore_voxels(&generator(bare(&[o]), 1.0), fixtures().lantern).len() as f32
    };
    let (all, quarter) = (count(1.0), count(0.25));
    assert!(
        (0.15..0.35).contains(&(quarter / all)),
        "{quarter} of {all}"
    );
}

#[test]
fn the_same_seed_makes_the_same_veins_and_another_seed_others() {
    let find = |seed| {
        let g = Worldgen::new(seed, 1.0, bare(&[anywhere()]));
        ore_voxels(&g, fixtures().lantern)
    };
    assert_eq!(find(SEED), find(SEED));
    assert_ne!(find(SEED), find(SEED + 1));
}

#[test]
fn two_ores_each_make_their_own_veins() {
    let (lantern, glass) = (fixtures().lantern, fixtures().glass);
    let g = generator(
        bare(&[ore(lantern, 0.0, 400.0), ore(glass, 0.0, 400.0)]),
        1.0,
    );
    let (a, b) = (ore_voxels(&g, lantern).len(), ore_voxels(&g, glass).len());
    assert!(a > 500 && b > 500, "{a} {b}");
}

#[test]
fn generator_version_1_has_no_ore() {
    let v1 = |ores: &[Ore]| WorldgenParams {
        generator: 1,
        ..bare(ores)
    };
    let (with, without) = (generator(v1(&[anywhere()]), 1.0), generator(v1(&[]), 1.0));
    for cy in -1..3 {
        for cz in -1..1 {
            for cx in -1..1 {
                let (a, b) = (with.chunk(cx, cy, cz), without.chunk(cx, cy, cz));
                assert!(a.kinds() == b.kinds());
                for z in 0..CHUNK_SIZE {
                    for y in 0..CHUNK_SIZE {
                        for x in 0..CHUNK_SIZE {
                            assert_eq!(a.get(x, y, z), b.get(x, y, z));
                        }
                    }
                }
            }
        }
    }
}
