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
