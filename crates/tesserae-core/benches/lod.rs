use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use tesserae_core::light::{Light, MAX_LIGHT};
use tesserae_core::lod::downsample;
use tesserae_core::mesh::FACE_STEPS;
use tesserae_core::{CHUNK_SIZE, Chunk, ChunkLight, LitChunk, Orientation, SHAPE_STAIR};
use tesserae_core::{ShapeTable, Tables, Voxel, Worldgen, WorldgenParams, mesh_chunk, testgen};

const SEED: u64 = 0x7e55_e2ae_0000_0001;
const DEEPEST_LEVEL: u32 = 5;

fn octants(chunk_at: impl Fn(i32, i32, i32) -> Chunk, [cx, cy, cz]: [i32; 3]) -> [Chunk; 8] {
    std::array::from_fn(|i| {
        let (x, y, z) = (i as i32 & 1, i as i32 >> 1 & 1, i as i32 >> 2);
        chunk_at(cx + x, cy + y, cz + z)
    })
}

fn refs(children: &[Chunk; 8]) -> [&Chunk; 8] {
    std::array::from_fn(|i| &children[i])
}

/// Worst case for the palette and the volume lookup: all 24 Orientations over two Materials.
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

/// Mirrors the Server's sources: level 0 is production, those above it skip smoothing
/// because a coarse Voxel rounds stairs back to cubes.
fn source(level: u32) -> Worldgen {
    let params = WorldgenParams::default();
    if level == 1 {
        return Worldgen::new(SEED, 1.0, params);
    }
    let params = WorldgenParams {
        smoothing: None,
        ..params
    };
    Worldgen::new(SEED, 2f32.powi(level as i32 - 1), params)
}

fn coarse(source: &Worldgen, shapes: &ShapeTable, [cx, cy, cz]: [i32; 3]) -> Chunk {
    let children = octants(|x, y, z| source.chunk(x, y, z), [2 * cx, 2 * cy, 2 * cz]);
    downsample(refs(&children), shapes)
}

fn surface_at(level: u32) -> [i32; 3] {
    let cell_m = (CHUNK_SIZE as f32) * 2f32.powi(level as i32);
    let ground_m = source(1).surface_height_m(0.0, 0.0);
    [0, (ground_m / cell_m).floor() as i32, 0]
}

/// Air and rock are benched too: their early-outs are what most of a level holds.
fn coarse_chunk(c: &mut Criterion) {
    let shapes = ShapeTable::builtin();
    let mut group = c.benchmark_group("lod/coarse_chunk");
    for level in 1..=DEEPEST_LEVEL {
        let (source, at) = (source(level), surface_at(level));
        group.bench_function(format!("surface_level_{level}"), |b| {
            b.iter(|| black_box(coarse(&source, &shapes, black_box(at))))
        });
    }
    let source = source(1);
    let top = surface_at(1)[1];
    for (name, y) in [("air_level_1", top + 4), ("rock_level_1", top - 4)] {
        group.bench_function(name, |b| {
            b.iter(|| black_box(coarse(&source, &shapes, black_box([0, y, 0]))))
        });
    }
    group.finish();
}

/// Lit as open sky, as the Server lights every coarse Chunk.
fn mesh_coarse(c: &mut Criterion) {
    let shapes = ShapeTable::builtin();
    let tables = Tables::builtin();
    let sky = ChunkLight::filled(Light::new(MAX_LIGHT, 0));
    let mut group = c.benchmark_group("lod/mesh_coarse");
    for level in 1..=DEEPEST_LEVEL {
        let (source, [cx, cy, cz]) = (source(level), surface_at(level));
        let center = coarse(&source, &shapes, [cx, cy, cz]);
        let around: [Chunk; 6] = std::array::from_fn(|face| {
            let [dx, dy, dz] = FACE_STEPS[face];
            coarse(&source, &shapes, [cx + dx, cy + dy, cz + dz])
        });
        group.bench_function(format!("surface_level_{level}"), |b| {
            b.iter(|| {
                let neighbors =
                    std::array::from_fn(|face| Some(LitChunk::new(&around[face], &sky)));
                black_box(mesh_chunk(
                    LitChunk::new(black_box(&center), &sky),
                    neighbors,
                    &tables,
                ))
            })
        });
    }
    group.finish();
}

criterion_group!(benches, terrain, coarse_chunk, mesh_coarse);
criterion_main!(benches);
