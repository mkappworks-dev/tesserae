//! What a generator keeps between Chunks only saves time: a Chunk is the same from a warm
//! generator, a cold one and in any order, with Biomes, Plants, ores, water and caves.

use tesserae_core::{CHUNK_SIZE, Chunk, Voxel, Worldgen};

#[path = "../benches/support/v3_world.rs"]
mod world;

fn voxels(chunk: &Chunk) -> Vec<Voxel> {
    (0..CHUNK_SIZE.pow(3))
        .map(|i| {
            chunk.get(
                i % CHUNK_SIZE,
                i / CHUNK_SIZE % CHUNK_SIZE,
                i / CHUNK_SIZE.pow(2),
            )
        })
        .collect()
}

fn forward(g: &Worldgen, coords: &[[i32; 3]]) -> Vec<Vec<Voxel>> {
    coords
        .iter()
        .map(|&[x, y, z]| voxels(&g.chunk(x, y, z)))
        .collect()
}

/// Backwards, every cache dropped before each Chunk.
fn cold(g: &Worldgen, coords: &[[i32; 3]]) -> Vec<Vec<Voxel>> {
    let mut chunks: Vec<_> = coords
        .iter()
        .rev()
        .map(|&[x, y, z]| {
            g.forget_regions();
            voxels(&g.chunk(x, y, z))
        })
        .collect();
    chunks.reverse();
    chunks
}

fn count(chunks: &[Vec<Voxel>], material: u16) -> usize {
    chunks
        .iter()
        .flatten()
        .filter(|v| v.material() == material)
        .count()
}

#[test]
fn a_chunk_is_the_same_warm_cold_and_in_any_order() {
    for size in [1.0, 0.5] {
        for case in world::cases(size) {
            let warm = forward(&case.generator(), &case.chunks);
            assert_eq!(
                warm,
                cold(&case.generator(), &case.chunks),
                "{} at {size} m",
                case.name
            );
            if case.name.starts_with("forest") {
                assert!(count(&warm, world::TRUNK) > 0, "no trees at {size} m");
            }
            if case.name == "straddling dry" {
                assert!(count(&warm, world::SAND) > 0, "no dry ground at {size} m");
            }
        }
    }
}

/// Every Chunk of the band over a 3x3 patch of columns round `centre`.
fn patch(g: &Worldgen, centre: [i32; 2]) -> Vec<[i32; 3]> {
    let cell_m = CHUNK_SIZE as f32 * g.voxel_size();
    (-1..=1)
        .flat_map(|dz| (-1..=1).map(move |dx| [centre[0] + dx, centre[1] + dz]))
        .flat_map(|at| {
            let [x, z] = world::middle_m(g, at);
            let ground_m = g.surface_height_m(x, z);
            let (lo, hi) = (
                world::layer(g, ground_m - 2.0 * cell_m),
                world::layer(g, ground_m + cell_m),
            );
            (lo..=hi).map(move |cy| [at[0], cy, at[1]])
        })
        .collect()
}

#[test]
fn a_coarse_source_sharing_level_zero_makes_the_chunks_it_makes_alone() {
    for size in [1.0, 0.5] {
        let level_zero = world::generator(size);
        let forest = world::scan(&level_zero, |[x, z]| {
            level_zero.biome_at(x, z) == Some(world::FOREST)
        });
        // Level 0's own Chunks first, so the sources read caches it filled.
        forward(&level_zero, &patch(&level_zero, forest));
        for level in 1..=3 {
            let alone = world::coarse(size, level);
            assert!(alone.can_share(&level_zero), "level {level} at {size} m");
            let shared = world::coarse(size, level).sharing(&level_zero);
            let scale = 2i32.pow(level - 1);
            let coords = patch(&shared, forest.map(|c| c.div_euclid(scale)));
            let alone = forward(&alone, &coords);
            assert_eq!(
                forward(&shared, &coords),
                alone,
                "level {level} warm at {size} m"
            );
            assert_eq!(
                cold(&shared, &coords),
                alone,
                "level {level} cold at {size} m"
            );
        }
    }
}

#[test]
fn sharing_another_worlds_generator_is_refused() {
    let level_zero = world::generator(1.0);
    let reseeded = Worldgen::new(world::SEED + 1, 1.0, world::params());
    assert!(!reseeded.can_share(&level_zero));
    let params = world::params();
    let reshaped = tesserae_core::WorldgenParams {
        landforms: tesserae_core::terrain::LandformParams {
            height_m: params.landforms.height_m + 1.0,
            ..params.landforms
        },
        ..params
    };
    assert!(!Worldgen::new(world::SEED, 1.0, reshaped).can_share(&level_zero));
    assert!(world::coarse(1.0, 2).can_share(&level_zero));
}
