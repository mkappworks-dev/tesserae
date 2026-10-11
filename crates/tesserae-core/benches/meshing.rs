use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use tesserae_core::{
    Chunk, Neighborhood, Voxel, Worldgen, WorldgenParams, greedy_mesh, mesh_chunk, testgen,
};

/// A generator at the default Voxel size, and the Chunk coordinates of the
/// three cases streaming actually asks for.
fn worldgen() -> (Worldgen, i32, i32, i32) {
    let params = WorldgenParams::default();
    let g = Worldgen::new(0x7e55_e2ae, 1.0, params);
    let cell = tesserae_core::CHUNK_SIZE as f32;
    let (lowest, highest) = (params.sea_level_m, params.sea_level_m + params.relief_m);
    let straddling = (highest / cell) as i32;
    (g, straddling, straddling + 2, (lowest / cell) as i32 - 2)
}

fn mesher(c: &mut Criterion) {
    let mut group = c.benchmark_group("greedy_mesh");

    let terrain = testgen::terrain_chunk(0, 0, 0);
    let terrain_nb = Neighborhood::isolated(&terrain);
    group.bench_function("terrain", |b| {
        b.iter(|| black_box(greedy_mesh(black_box(&terrain_nb))))
    });

    let solid = Chunk::filled(Voxel::cube(testgen::STONE));
    let solid_nb = Neighborhood::isolated(&solid);
    group.bench_function("solid", |b| {
        b.iter(|| black_box(greedy_mesh(black_box(&solid_nb))))
    });

    // Nothing merges here; this is the ceiling on per-chunk mesh cost.
    let checker = testgen::checkerboard_chunk();
    let checker_nb = Neighborhood::isolated(&checker);
    group.bench_function("checkerboard", |b| {
        b.iter(|| black_box(greedy_mesh(black_box(&checker_nb))))
    });

    group.finish();
}

/// What streaming pays per Chunk, including the two cases it declines. The gap
/// between `declined_*` and `straddling` is what makes a vertical extent
/// affordable.
fn streamed_mesher(c: &mut Criterion) {
    let mut group = c.benchmark_group("mesh_chunk");
    let (g, straddling, _, _) = worldgen();

    let center = g.chunk(0, straddling, 0);
    let around = [
        g.chunk(-1, straddling, 0),
        g.chunk(1, straddling, 0),
        g.chunk(0, straddling - 1, 0),
        g.chunk(0, straddling + 1, 0),
        g.chunk(0, straddling, -1),
        g.chunk(0, straddling, 1),
    ];
    let refs: [Option<&Chunk>; 6] = std::array::from_fn(|i| Some(&around[i]));
    group.bench_function("straddling", |b| {
        b.iter(|| black_box(mesh_chunk(black_box(&center), black_box(refs))))
    });

    let air = Chunk::air();
    group.bench_function("declined_air", |b| {
        b.iter(|| black_box(mesh_chunk(black_box(&air), black_box(refs))))
    });

    let solid = Chunk::filled(Voxel::cube(testgen::STONE));
    let enclosed: [Option<&Chunk>; 6] = [Some(&solid); 6];
    group.bench_function("declined_enclosed", |b| {
        b.iter(|| black_box(mesh_chunk(black_box(&solid), black_box(enclosed))))
    });

    group.finish();
}

fn neighborhood(c: &mut Criterion) {
    let terrain = testgen::terrain_chunk(0, 0, 0);
    c.bench_function("neighborhood_isolated", |b| {
        b.iter(|| black_box(Neighborhood::isolated(black_box(&terrain))))
    });
}

fn generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("worldgen");
    let (g, straddling, sky, deep) = worldgen();

    group.bench_function("straddling", |b| {
        b.iter(|| black_box(g.chunk(black_box(0), straddling, 0)))
    });
    // Both early-outs: most Chunks in a streamed 3D ring are one of these.
    group.bench_function("early_out_air", |b| {
        b.iter(|| black_box(g.chunk(black_box(0), sky, 0)))
    });
    group.bench_function("early_out_stone", |b| {
        b.iter(|| black_box(g.chunk(black_box(0), deep, 0)))
    });

    group.finish();

    // Renamed from `worldgen_chunk`, which it never was: this is the bench
    // fixture, tuned in Voxel indices, and the three benches above are the
    // generator. The id change is a rename, not a regression.
    c.bench_function("testgen_terrain_chunk", |b| {
        b.iter(|| black_box(testgen::terrain_chunk(black_box(0), 0, 0)))
    });
}

criterion_group!(benches, mesher, streamed_mesher, neighborhood, generation);
criterion_main!(benches);
