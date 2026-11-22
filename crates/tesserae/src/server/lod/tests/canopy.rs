//! Canopy cover: above the Impostor levels a forest's columns take its leaves, and over saved
//! changes only what those changes hold.

use super::trees::{canopy, forest_spaced, rows, trunk, voxels};
use super::*;
use tesserae_core::canopy_stand;
use tesserae_core::testgen::fixtures;

const LEVELS: u32 = 5;

/// Crowns that close over the ground, as a 1.5-radius crowding never lets them.
fn dense() -> WorldgenParams {
    forest_spaced(3.0, Some(1.0))
}

/// The global coarse row and Voxel of the topmost non-air Voxel over each column of `chunks`, a
/// column of coarse Chunks bottom to top.
fn tops(chunks: &[(i32, Chunk)]) -> Vec<Option<(i32, Voxel)>> {
    let n = CHUNK_SIZE as i32;
    (0..CHUNK_SIZE * CHUNK_SIZE)
        .map(|i| {
            let (x, z) = (i % CHUNK_SIZE, i / CHUNK_SIZE);
            chunks.iter().rev().find_map(|(cy, chunk)| {
                (0..CHUNK_SIZE)
                    .rev()
                    .map(|y| (cy * n + y as i32, chunk.get(x, y, z)))
                    .find(|(_, v)| !v.is_air())
            })
        })
        .collect()
}

#[test]
fn a_dense_forest_draws_its_leaves_as_high_over_its_ground_as_its_cover_and_trees_reach() {
    let world = land();
    let level_zero = Worldgen::new(world.seed, world.voxel_size, dense());
    let generators = LodGenerators::new(&world, &level_zero, LEVELS);
    let tables = &fixtures().tables;
    let shapes = generators.placement.canopy_shapes();
    for level in IMPOSTORS_TO + 1..LEVELS {
        let voxel_m = world.voxel_size * level_scale(level);
        let mut rises = std::collections::BTreeSet::new();
        for (cx, cz) in [(-1, -1), (0, 0)] {
            let n = CHUNK_SIZE;
            let centres: Vec<[i32; 2]> = (0..n * n)
                .map(|i| {
                    [
                        Answers::centre(level, cx, i % n),
                        Answers::centre(level, cz, i / n),
                    ]
                })
                .collect();
            let expected = generators.placement.canopy_cover(&shapes, &centres);
            let column = |placement: bool| {
                let makers = Makers {
                    placement: placement.then_some(&*generators.placement),
                    ..generators.makers(level)
                };
                let chunks: Vec<(i32, Chunk)> = rows(&level_zero, level)
                    .map(|cy| {
                        let coord = ChunkCoord::new(cx, cy, cz);
                        let made = make_coarse(level, coord, makers, &Default::default(), tables);
                        (cy, made.chunk)
                    })
                    .collect();
                tops(&chunks)
            };
            let (covered, ground) = (column(true), column(false));
            let mut leaves = 0;
            for ((top, under), drawn) in covered.iter().zip(&ground).zip(&expected) {
                let ((y, v), (ground_y, ground_v)) = (top.unwrap(), under.unwrap());
                if v.material() == canopy() {
                    let (rise, material) = canopy_stand(&drawn.unwrap(), voxel_m);
                    leaves += 1;
                    rises.insert(rise);
                    assert_eq!(
                        (y, v.material()),
                        (ground_y + rise, material),
                        "level {level}"
                    );
                } else {
                    assert_eq!(
                        (y, v),
                        (ground_y, ground_v),
                        "level {level}: changed ground"
                    );
                }
            }
            assert!(
                leaves * 10 > covered.len() * 9,
                "level {level}: {leaves} of {} columns covered",
                covered.len()
            );
        }
        // A 3 to 5 m trunk under a 2.5 m crown tops some 7 m up: two 4 m Voxels, one 8 m one, under
        // half a 16 m one.
        let reached = [(2, 2), (3, 1), (4, 0)];
        assert!(
            reached.contains(&(level, *rises.last().unwrap())),
            "level {level}: rises {rises:?}"
        );
    }
}

#[test]
fn a_felled_forest_in_a_saved_chunk_stays_bare_above_the_impostor_levels() {
    let world = land();
    let level_zero = Worldgen::new(world.seed, world.voxel_size, dense());
    let generators = LodGenerators::new(&world, &level_zero, LEVELS);
    let tables = &fixtures().tables;
    let is_tree = |v: Voxel| v.material() == trunk() || v.material() == canopy();
    let saved: std::collections::BTreeMap<ChunkCoord, Held> = rows(&level_zero, 0)
        .filter_map(|cy| {
            let mut chunk = level_zero.chunk(0, cy, 0);
            let felled: Vec<[usize; 3]> = voxels(&chunk)
                .filter(|&(_, v)| is_tree(v))
                .map(|(at, _)| at)
                .collect();
            for [x, y, z] in &felled {
                chunk.set(*x, *y, *z, Voxel::AIR);
            }
            (!felled.is_empty()).then(|| {
                let held = Held {
                    chunk: Arc::new(chunk),
                    changed: Some(Arc::new(ChunkOverlay::none())),
                };
                (ChunkCoord::new(0, cy, 0), held)
            })
        })
        .collect();
    assert!(!saved.is_empty(), "no tree in level-0 Chunk column (0, 0)");
    for level in IMPOSTORS_TO + 1..LEVELS {
        let span = 1usize << level;
        let felled = CHUNK_SIZE / span;
        let chunks: Vec<(i32, Chunk)> = rows(&level_zero, level)
            .map(|cy| {
                let coord = ChunkCoord::new(0, cy, 0);
                let made = generate_persistent(
                    level,
                    coord,
                    &generators.persistent(),
                    &saved,
                    &HashMap::new(),
                    tables,
                );
                (cy, made.chunk)
            })
            .collect();
        let tops = tops(&chunks);
        let wooded = |x: usize, z: usize| {
            chunks
                .iter()
                .any(|(_, c)| (0..CHUNK_SIZE).any(|y| is_tree(c.get(x, y, z))))
        };
        for z in 0..felled {
            for x in 0..felled {
                assert!(!wooded(x, z), "level {level}: a tree at ({x}, {z})");
            }
        }
        let covered = tops
            .iter()
            .filter(|t| t.is_some_and(|(_, v)| v.material() == canopy()))
            .count();
        assert!(
            covered * 2 > tops.len(),
            "level {level}: the unchanged forest is bare"
        );
    }
}
