use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use tesserae_core::lod::downsample;
use tesserae_core::{CHUNK_SIZE, Chunk, Orientation, SHAPE_STAIR, ShapeTable, Voxel, Worldgen};
use tesserae_core::{WorldgenParams, testgen};

/// The eight Chunks of the 2x2x2 block whose lowest corner is `(cx, cy, cz)`,
/// in octant order `x + 2y + 4z`.
fn octants(chunk_at: impl Fn(i32, i32, i32) -> Chunk, [cx, cy, cz]: [i32; 3]) -> [Chunk; 8] {
    std::array::from_fn(|i| {
        let (x, y, z) = (i as i32 & 1, i as i32 >> 1 & 1, i as i32 >> 2);
        chunk_at(cx + x, cy + y, cz + z)
    })
}

fn refs(children: &[Chunk; 8]) -> [&Chunk; 8] {
    std::array::from_fn(|i| &children[i])
}

/// Every Voxel a stair, in all 24 Orientations over two Materials: the most
/// the palette and the volume lookup can be asked for.
fn stairs() -> Chunk {
    let orientations: Vec<Orientation> = Orientation::all().collect();
    let mut chunk = Chunk::air();
    for y in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let i = x + 3 * y + 5 * z;
                chunk.set(
                    x,
                    y,
                    z,
                    Voxel::shaped(
                        testgen::STONE + (i % 2) as u16,
                        SHAPE_STAIR,
                        orientations[i % orientations.len()],
                    ),
                );
            }
        }
    }
    chunk
}

/// Cubes under a layer of stairs under air: a ramp's worth of shaped Voxels,
/// over uniform halves.
fn stair_surface() -> [Chunk; 8] {
    let ramp = |cy: i32| {
        let mut chunk = Chunk::air();
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let voxel = match (cy, y) {
                        (0, 0..=23) => Voxel::cube(testgen::STONE),
                        (0, _) => Voxel::shaped(
                            testgen::DIRT,
                            SHAPE_STAIR,
                            Orientation::new(0, ((x + z) % 4) as u8),
                        ),
                        _ => Voxel::AIR,
                    };
                    chunk.set(x, y, z, voxel);
                }
            }
        }
        chunk
    };
    octants(|_, y, _| ramp(y), [0, 0, 0])
}

/// What a coarse-Chunk Job downsamples at the surface: the default World's
/// eight Chunks around the one its ground crosses, caves and stairs included.
fn terrain(c: &mut Criterion) {
    let shapes = ShapeTable::builtin();
    let g = Worldgen::new(0x7e55_e2ae, 1.0, WorldgenParams::default());
    let surface = (g.surface_height_m(0.0, 0.0) / CHUNK_SIZE as f32).floor() as i32;
    let world = octants(|x, y, z| g.chunk(x, y, z), [0, surface, 0]);
    let fixture = octants(testgen::terrain_chunk, [0, 0, 0]);
    let stair_surface = stair_surface();
    let stairs = std::array::from_fn::<_, 8, _>(|_| stairs());
    let stone = std::array::from_fn::<_, 8, _>(|_| Chunk::filled(Voxel::cube(testgen::STONE)));
    let air = std::array::from_fn::<_, 8, _>(|_| Chunk::air());
    let halves = std::array::from_fn::<_, 8, _>(|i| {
        Chunk::filled(if i < 4 {
            Voxel::cube(testgen::STONE)
        } else {
            Voxel::AIR
        })
    });

    let mut group = c.benchmark_group("lod/downsample");
    for (name, children) in [
        ("worldgen", &world),
        ("testgen", &fixture),
        ("stair_surface", &stair_surface),
        ("stairs", &stairs),
        ("uniform_stone", &stone),
        ("uniform_air", &air),
        ("uniform_halves", &halves),
    ] {
        group.bench_function(name, |b| {
            b.iter(|| black_box(downsample(black_box(refs(children)), &shapes)))
        });
    }
    group.finish();
}

criterion_group!(benches, terrain);
criterion_main!(benches);
