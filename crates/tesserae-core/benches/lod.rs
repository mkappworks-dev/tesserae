use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use tesserae_core::light::{Light, MAX_LIGHT};
use tesserae_core::lod::{Child, coarse_light, downsample, downsample_changed};
use tesserae_core::mesh::FACE_STEPS;
use tesserae_core::testgen::fixtures;
use tesserae_core::{CHUNK_SIZE, Chunk, ChunkLight, ChunkOverlay, LitChunk, Orientation};
use tesserae_core::{Fluid, Level, Overlay, SHAPE_STAIR};
use tesserae_core::{ShapeTable, Voxel, Worldgen, WorldgenParams, mesh_chunk, testgen};

const SEED: u64 = 0x7e55_e2ae_0000_0001;
const DEEPEST_LEVEL: u32 = 5;

/// A default World with its lakes left out: the terrain every figure in this
/// file measured before lakes, so a change stays comparable with `main`.
fn dry() -> WorldgenParams {
    WorldgenParams {
        water: None,
        ..tesserae_core::testgen::world_params()
    }
}

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
    let fixture_rock = fixtures().ground.rock;
    let fixture_soil = fixtures().ground.soil;
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
                        [fixture_rock, fixture_soil][i % 2],
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
    let fixture_rock = fixtures().ground.rock;
    let fixture_soil = fixtures().ground.soil;
    let ramp = |cy: i32| {
        let mut chunk = Chunk::air();
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let voxel = match (cy, y) {
                        (0, 0..=23) => Voxel::cube(fixture_rock),
                        (0, _) => Voxel::shaped(
                            fixture_soil,
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
    let fixture_ground = fixtures().ground;
    let fixture_water = fixtures().water;
    let shapes = ShapeTable::builtin();
    let g = Worldgen::new(0x7e55_e2ae, 1.0, dry());
    let surface = (g.surface_height_m(0.0, 0.0) / CHUNK_SIZE as f32).floor() as i32;
    let world = octants(|x, y, z| g.chunk(x, y, z), [0, surface, 0]);
    let fixture = octants(
        |x, y, z| testgen::terrain_chunk(x, y, z, fixture_ground),
        [0, 0, 0],
    );
    let stair_surface = stair_surface();
    let stairs = std::array::from_fn::<_, 8, _>(|_| stairs());
    let stone = std::array::from_fn::<_, 8, _>(|_| Chunk::filled(Voxel::cube(fixture_ground.rock)));
    let air = std::array::from_fn::<_, 8, _>(|_| Chunk::air());
    let halves = std::array::from_fn::<_, 8, _>(|i| {
        Chunk::filled(if i < 4 {
            Voxel::cube(fixture_ground.rock)
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

    // Level 1 over held Chunks something changed: read with their overlays, dry
    // and with every open Voxel of the lower four flooded by flowing water.
    let tables = fixtures().tables.clone();
    let water = fixture_water;
    let dry: [ChunkOverlay; 8] = Default::default();
    let flowing = Overlay {
        fluid: Fluid::flowing(
            water,
            [
                Level {
                    surface: 8,
                    reach: 3,
                    descent: 0,
                },
                Level::DRY,
                Level::DRY,
                Level::DRY,
            ],
        ),
        burning: None,
    };
    let flooded: [ChunkOverlay; 8] = std::array::from_fn(|i| {
        let mut overlay = ChunkOverlay::none();
        if i >> 1 & 1 == 0 {
            for z in 0..CHUNK_SIZE {
                for y in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        if world[i].get(x, y, z).is_air() {
                            overlay.set(x, y, z, flowing);
                        }
                    }
                }
            }
        }
        overlay
    });
    for (name, overlays) in [("changed_dry", &dry), ("changed_flooded", &flooded)] {
        group.bench_function(name, |b| {
            b.iter(|| {
                let children: [Child; 8] =
                    std::array::from_fn(|i| Child::Changed(&world[i], &overlays[i]));
                black_box(downsample_changed(
                    black_box(children),
                    surface,
                    &tables,
                    None,
                ))
            })
        });
    }
    group.finish();
}

/// Mirrors the Server's sources: level 0 is production, those above it skip smoothing
/// because a coarse Voxel rounds stairs back to cubes.
fn source(level: u32) -> Worldgen {
    let params = dry();
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
    let tables = fixtures().tables.clone();
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

/// A coarse Chunk and its six face neighbours.
fn around(source: &Worldgen, shapes: &ShapeTable, [cx, cy, cz]: [i32; 3]) -> (Chunk, [Chunk; 6]) {
    let center = coarse(source, shapes, [cx, cy, cz]);
    let around = std::array::from_fn(|face| {
        let [dx, dy, dz] = FACE_STEPS[face];
        coarse(source, shapes, [cx + dx, cy + dy, cz + dz])
    });
    (center, around)
}

/// The covered-column rule alone: a Chunk the surface crosses, at every level, and one wholly
/// above the band, which answers without a height.
fn coarse_light_rule(c: &mut Criterion) {
    let shapes = ShapeTable::builtin();
    let mut group = c.benchmark_group("lod/coarse_light");
    let mut bench = |name: String, level: u32, at: [i32; 3]| {
        let source = source(level);
        let (center, around) = around(&source, &shapes, at);
        let voxel_m = 2f32.powi(level as i32);
        group.bench_function(name, |b| {
            b.iter(|| {
                black_box(coarse_light(
                    &source,
                    voxel_m,
                    black_box(at),
                    black_box(&center),
                    std::array::from_fn(|face| &around[face]),
                ))
            })
        });
    };
    for level in 1..=DEEPEST_LEVEL {
        bench(format!("surface_level_{level}"), level, surface_at(level));
    }
    let [x, y, z] = surface_at(1);
    bench("above_band_level_1".into(), 1, [x, y + 3, z]);
    group.finish();
}

/// What a coarse mesh job costs now: the rule, then the mesher. `mouth_level_1` is the Chunk
/// of the default seed's cave mouth at (-26, -352) m, which has covered Voxels to light.
fn mesh_coarse_lit(c: &mut Criterion) {
    let shapes = ShapeTable::builtin();
    let tables = fixtures().tables.clone();
    let mut group = c.benchmark_group("lod/mesh_coarse_lit");
    let mut bench = |name: String, level: u32, at: [i32; 3]| {
        let source = source(level);
        let (center, around) = around(&source, &shapes, at);
        let voxel_m = 2f32.powi(level as i32);
        group.bench_function(name, |b| {
            b.iter(|| {
                let lit = coarse_light(
                    &source,
                    voxel_m,
                    black_box(at),
                    black_box(&center),
                    std::array::from_fn(|face| &around[face]),
                );
                let neighbors = std::array::from_fn(|face| {
                    Some(LitChunk::new(&around[face], &lit.around[face]))
                });
                black_box(mesh_chunk(
                    LitChunk::new(black_box(&center), &lit.center),
                    neighbors,
                    &tables,
                ))
            })
        });
    };
    for level in 1..=DEEPEST_LEVEL {
        bench(format!("surface_level_{level}"), level, surface_at(level));
    }
    bench("mouth_level_1".into(), 1, [-1, 0, -6]);
    group.finish();
}

criterion_group!(
    benches,
    terrain,
    coarse_chunk,
    mesh_coarse,
    coarse_light_rule,
    mesh_coarse_lit
);
criterion_main!(benches);
