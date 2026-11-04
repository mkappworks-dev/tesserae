//! Coarse levels show the Biome ground that level 0 does at the same metres, at the Voxel
//! sizes a World may have, down the derived levels and the column-only generators alike.

use super::*;
use tesserae_core::testgen::fixtures;
use tesserae_core::{Biome, BiomeName, BiomeTable, ClimateParams, GroundParams};

const LEVELS: u32 = 4;

fn desert() -> GroundParams {
    GroundParams {
        rock: fixtures().lantern,
        soil: fixtures().glass,
        surface: fixtures().leaves,
    }
}

fn tundra() -> GroundParams {
    GroundParams {
        rock: fixtures().glass,
        soil: fixtures().leaves,
        surface: fixtures().lantern,
    }
}

fn biome(name: &str, temperature: f32, ground: GroundParams) -> Biome {
    Biome {
        name: BiomeName::new(name).unwrap(),
        temperature,
        humidity: 0.5,
        ground: Some(ground),
        sky_fog: None,
        cave_scale: None,
    }
}

/// A lake too, so the half-Voxel shift the column generators apply to it is on the path.
fn params() -> WorldgenParams {
    WorldgenParams {
        climate: ClimateParams {
            temperature_size_m: 160.0,
            humidity_size_m: 160.0,
        },
        biomes: BiomeTable::new(&[biome("cold", 0.2, tundra()), biome("warm", 0.8, desert())])
            .unwrap(),
        ..tesserae_core::testgen::world_params()
            .with_default_lake(tesserae_core::testgen::fixtures().water)
    }
}

/// The Biome over a coarse Voxel when every level-0 column under it agrees, else `None`: the
/// derived levels vote across a border, so only a Voxel clear of one has a Biome to show.
fn biome_under(level_zero: &Worldgen, x_m: f32, z_m: f32, edge_m: f32) -> Option<usize> {
    let step = level_zero.voxel_size();
    let n = (edge_m / step).round() as usize;
    let first = level_zero.biome_at(x_m, z_m)?;
    (0..n)
        .flat_map(|i| (0..n).map(move |j| (i, j)))
        .all(|(i, j)| {
            level_zero.biome_at(x_m + i as f32 * step, z_m + j as f32 * step) == Some(first)
        })
        .then_some(first.index())
}

fn chunk_of(generators: &LodGenerators, tables: &Tables, level: u32, c: ChunkCoord) -> Chunk {
    match generators.direct(level) {
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
    }
}

fn check(voxel_size: f32) {
    let world = WorldSettings {
        voxel_size,
        ..WorldSettings::default()
    };
    let level_zero = Worldgen::new(world.seed, voxel_size, params());
    let generators = LodGenerators::new(&world, &level_zero, LEVELS);
    // The ground Materials are the testgen fixtures', whose rows the Server's fixture tables lack.
    let tables = &fixtures().tables;
    let (_, highest) = level_zero.surface_bounds_m();
    let all = [tundra(), desert()];
    for level in 1..LEVELS {
        let edge_m = voxel_size * 2f32.powi(level as i32);
        let cell_m = CHUNK_SIZE as f32 * edge_m;
        let rows = (highest / cell_m).ceil() as i32;
        let reach = (192.0 / cell_m).ceil() as i32;
        let mut seen = [0usize; 2];
        let mut rocky = [0usize; 2];
        let mut capped = [0usize; 2];
        for cz in -reach..reach {
            for cx in -reach..reach {
                let chunks: Vec<_> = (0..=rows)
                    .map(|cy| chunk_of(&generators, tables, level, ChunkCoord::new(cx, cy, cz)))
                    .collect();
                for z in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        let x_m = (cx * CHUNK_SIZE as i32 + x as i32) as f32 * edge_m;
                        let z_m = (cz * CHUNK_SIZE as i32 + z as i32) as f32 * edge_m;
                        let Some(id) = biome_under(&level_zero, x_m, z_m, edge_m) else {
                            continue;
                        };
                        let ground = all[id];
                        let column: Vec<Voxel> = chunks
                            .iter()
                            .flat_map(|c| (0..CHUNK_SIZE).map(move |y| c.get(x, y, z)))
                            .collect();
                        let solid: Vec<_> = column
                            .iter()
                            .filter(|v| !v.is_air() && v.material() != fixtures().water)
                            .collect();
                        // Nothing in it is a Material of any other ground, the World's included.
                        assert!(
                            solid
                                .iter()
                                .all(|v| [ground.rock, ground.soil, ground.surface]
                                    .contains(&v.material())),
                            "level {level} at {x_m},{z_m} m, {voxel_size} m Voxels"
                        );
                        seen[id] += 1;
                        rocky[id] += usize::from(solid.iter().any(|v| v.material() == ground.rock));
                        capped[id] += usize::from(
                            solid
                                .iter()
                                .any(|v| [ground.soil, ground.surface].contains(&v.material())),
                        );
                    }
                }
            }
        }
        assert!(
            seen.iter().all(|&n| n > 20) && capped.iter().all(|&n| n > 20),
            "level {level} at {voxel_size} m: {seen:?} columns, {capped:?} with soil or surface"
        );
        // A cave shaft opens some columns to the bottom, so rock is asked of nearly all of them.
        assert!(
            (0..2).all(|i| rocky[i] * 10 >= seen[i] * 9),
            "level {level} at {voxel_size} m: {rocky:?} of {seen:?} columns have their Biome's rock"
        );
    }
}

#[test]
fn coarse_levels_show_the_biome_ground_of_level_zero_at_one_metre_voxels() {
    check(1.0);
}

#[test]
fn coarse_levels_show_the_biome_ground_of_level_zero_at_half_metre_voxels() {
    check(0.5);
}

#[test]
fn the_column_generators_cap_each_column_with_its_biomes_surface() {
    let world = WorldSettings::default();
    let level_zero = Worldgen::new(world.seed, 1.0, params());
    let generators = LodGenerators::new(&world, &level_zero, LEVELS);
    let direct = generators.direct(3).expect("level 3 is column-only");
    let edge_m = 8.0;
    let all = [tundra(), desert()];
    let mut seen = [0usize; 2];
    for cz in -2..2 {
        for cx in -2..2 {
            let chunk = direct.chunk(cx, 0, cz);
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let x_m = (cx * CHUNK_SIZE as i32 + x as i32) as f32 * edge_m;
                    let z_m = (cz * CHUNK_SIZE as i32 + z as i32) as f32 * edge_m;
                    let id = level_zero.biome_at(x_m, z_m).unwrap().index();
                    let top = (0..CHUNK_SIZE)
                        .rev()
                        .find(|&y| {
                            let v = chunk.get(x, y, z);
                            !v.is_air() && v.material() != fixtures().water
                        })
                        .expect("ground in every column");
                    assert_eq!(chunk.get(x, top, z).material(), all[id].surface);
                    seen[id] += 1;
                }
            }
        }
    }
    assert!(seen.iter().all(|&n| n > 200), "{seen:?}");
}
