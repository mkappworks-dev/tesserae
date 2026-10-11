use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use tesserae_core::{Chunk, Neighborhood, Voxel, greedy_mesh, testgen};

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

fn neighborhood(c: &mut Criterion) {
    let terrain = testgen::terrain_chunk(0, 0, 0);
    c.bench_function("neighborhood_isolated", |b| {
        b.iter(|| black_box(Neighborhood::isolated(black_box(&terrain))))
    });
}

fn storage(c: &mut Criterion) {
    c.bench_function("worldgen_chunk", |b| {
        b.iter(|| black_box(testgen::terrain_chunk(black_box(0), 0, 0)))
    });
}

criterion_group!(benches, mesher, neighborhood, storage);
criterion_main!(benches);
