//! Coarse levels leave Structures out: their generators are given no Templates, so a Structure
//! neither shows nor votes in a coarse Voxel.

use super::*;
use tesserae_core::template::{Cell, Template};
use tesserae_core::testgen::fixtures;
use tesserae_core::{Biome, BiomeName, BiomeStructure, BiomeTable, SurfaceFeatures};

const LEVELS: u32 = 4;

/// 4 m square and 40 m tall, solid, so at level 0 it would win every vote it stands in.
fn tower() -> Template {
    let lantern = Cell::Voxel(Voxel::cube(fixtures().lantern));
    let layers = vec![vec![vec![lantern; 4]; 4]; 40];
    Template::new("t:tower".to_owned(), 1.0, layers, 0, None, 1.0).unwrap()
}

fn params() -> WorldgenParams {
    let row = BiomeStructure {
        template: 0,
        spacing_m: 16.0,
        chance: 1.0,
    };
    let towers = Biome {
        name: BiomeName::new("t:towers").unwrap(),
        temperature: 0.5,
        humidity: 0.5,
        ground: None,
        sky_fog: None,
        cave_scale: None,
        surface_features: SurfaceFeatures::NONE.with_structures(&[row]).unwrap(),
    };
    WorldgenParams {
        biomes: BiomeTable::new(&[towers]).unwrap(),
        ..tesserae_core::testgen::world_params()
    }
}

/// Lantern Voxels in the Chunks of `level` within 64 m of the origin, from the band's bottom to
/// past the towers' tops.
fn towers_at(generators: &LodGenerators, level_zero: &Worldgen, level: u32) -> usize {
    let tables = &fixtures().tables;
    let cell_m = CHUNK_SIZE as f32 * level_zero.voxel_size() * 2f32.powi(level as i32);
    let (lowest, highest) = level_zero.surface_bounds_m();
    let reach = (64.0 / cell_m).ceil() as i32;
    let mut found = 0;
    for cy in (lowest / cell_m).floor() as i32 - 1..=(highest / cell_m).floor() as i32 + 1 {
        for cz in -reach..reach {
            for cx in -reach..reach {
                let c = ChunkCoord::new(cx, cy, cz);
                let chunk = match (
                    level,
                    (level > 0).then(|| generators.direct(level)).flatten(),
                ) {
                    (0, _) => level_zero.chunk(c.x, c.y, c.z),
                    (_, Some(direct)) => direct.chunk(c.x, c.y, c.z),
                    (_, None) => {
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
fn coarse_levels_hold_no_structure_voxel() {
    let world = land();
    let level_zero =
        Worldgen::new(world.seed, world.voxel_size, params()).with_templates(&[tower()]);
    let generators = LodGenerators::new(&world, &level_zero, LEVELS);
    assert!(
        towers_at(&generators, &level_zero, 0) > 1000,
        "no tower at level 0"
    );
    for level in 1..LEVELS {
        assert_eq!(
            towers_at(&generators, &level_zero, level),
            0,
            "level {level}"
        );
    }
}
