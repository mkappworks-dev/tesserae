//! Ore is level 0's, derived by the downsample at levels 1 and 2; the column-only
//! generators from level 3 up carry none, as they carry no caves.

use super::*;
use tesserae_core::testgen::fixtures;
use tesserae_core::{BiomeSet, Ore, OreTable};

const LEVELS: u32 = 4;

fn params() -> WorldgenParams {
    let ore = Ore {
        material: fixtures().lantern,
        min_depth_m: 0.0,
        max_depth_m: 400.0,
        vein_size_m: 12.0,
        rarity: 1.0,
        biomes: BiomeSet::default(),
    };
    WorldgenParams {
        ores: OreTable::new(&[ore]).unwrap(),
        ..tesserae_core::testgen::world_params()
    }
}

fn ore_at(generators: &LodGenerators, level: u32) -> usize {
    let tables = &fixtures().tables;
    let mut found = 0;
    for cy in -1..2 {
        for cz in -1..1 {
            for cx in -1..1 {
                let c = ChunkCoord::new(cx, cy, cz);
                let chunk = match generators.direct(level) {
                    Some(direct) => direct.chunk(c.x, c.y, c.z),
                    None => {
                        generate_coarse(
                            &generators.sources[level as usize - 1],
                            c,
                            &Default::default(),
                            tables,
                            None,
                            None,
                        )
                        .chunk
                    }
                };
                for z in 0..CHUNK_SIZE {
                    for y in 0..CHUNK_SIZE {
                        for x in 0..CHUNK_SIZE {
                            found +=
                                usize::from(chunk.get(x, y, z).material() == fixtures().lantern);
                        }
                    }
                }
            }
        }
    }
    found
}

#[test]
fn levels_one_and_two_show_ore_and_the_column_generators_have_none() {
    let world = WorldSettings::default();
    let level_zero = Worldgen::new(world.seed, world.voxel_size, params());
    let generators = LodGenerators::new(&world, &level_zero, LEVELS);
    assert!(generators.direct(3).is_some(), "level 3 is column-only");
    assert!(ore_at(&generators, 1) > 100);
    assert!(ore_at(&generators, 2) > 100);
    assert_eq!(ore_at(&generators, 3), 0);
}
