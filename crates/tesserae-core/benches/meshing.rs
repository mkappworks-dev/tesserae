use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::sync::OnceLock;
use tesserae_core::light::{Light, MAX_LIGHT, light_chunk};
use tesserae_core::{
    CHUNK_SIZE, Chunk, ChunkLight, LitChunk, Neighborhood, ShapeTable, Voxel, Worldgen,
    WorldgenParams, greedy_mesh, mesh_chunk, testgen,
};

fn shapes() -> &'static ShapeTable {
    static TABLE: OnceLock<ShapeTable> = OnceLock::new();
    TABLE.get_or_init(ShapeTable::builtin)
}

/// Flat full sky, which is what `light_chunk` answers for open terrain and so
/// what the mesher is handed in the common case.
fn open_sky() -> ChunkLight {
    ChunkLight::filled(Light::new(MAX_LIGHT, 0))
}

/// A generator at the default Voxel size, and the Chunk coordinates of the
/// three cases streaming actually asks for.
fn worldgen() -> (Worldgen, i32, i32, i32) {
    let params = WorldgenParams::default();
    let g = Worldgen::new(0x7e55_e2ae, 1.0, params);
    let cell = tesserae_core::CHUNK_SIZE as f32;
    // The Chunk the surface actually crosses over this column, not the top of
    // the band it could reach: that one is usually empty sky, and benching it
    // as "straddling" would measure the air early-out twice.
    let straddling = (g.surface_height_m(0.0, 0.0) / cell).floor() as i32;
    let sky = ((params.sea_level_m + params.relief_m) / cell) as i32 + 2;
    let deep = (params.sea_level_m / cell) as i32 - 2;
    (g, straddling, sky, deep)
}

fn mesher(c: &mut Criterion) {
    let mut group = c.benchmark_group("greedy_mesh");

    let sky = open_sky();
    let terrain = testgen::terrain_chunk(0, 0, 0);
    let terrain_nb = Neighborhood::isolated(LitChunk::new(&terrain, &sky));
    group.bench_function("terrain", |b| {
        b.iter(|| black_box(greedy_mesh(black_box(&terrain_nb), shapes())))
    });

    let solid = Chunk::filled(Voxel::cube(testgen::STONE));
    let solid_nb = Neighborhood::isolated(LitChunk::new(&solid, &sky));
    group.bench_function("solid", |b| {
        b.iter(|| black_box(greedy_mesh(black_box(&solid_nb), shapes())))
    });

    // Nothing merges here; this is the ceiling on per-chunk mesh cost.
    let checker = testgen::checkerboard_chunk();
    let checker_nb = Neighborhood::isolated(LitChunk::new(&checker, &sky));
    group.bench_function("checkerboard", |b| {
        b.iter(|| black_box(greedy_mesh(black_box(&checker_nb), shapes())))
    });

    // What the merge key actually costs. Light is in the key, so a gradient
    // across the terrain splits runs that flat light leaves merged - the number
    // that says whether baking light per vertex was affordable.
    let mut gradient = ChunkLight::dark();
    for y in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                gradient.set(x, y, z, Light::new(MAX_LIGHT, ((x + z) % 16) as u8));
            }
        }
    }
    let gradient_nb = Neighborhood::isolated(LitChunk::new(&terrain, &gradient));
    group.bench_function("terrain_light_gradient", |b| {
        b.iter(|| black_box(greedy_mesh(black_box(&gradient_nb), shapes())))
    });

    group.finish();
}

/// What a Chunk arriving costs the Server's relight stage, including the two
/// cases the bulk solver answers from `uniform()` alone. Those two are most of
/// a streamed 3D ring: it is mostly open sky and mostly deep rock.
fn lighting(c: &mut Criterion) {
    let mut group = c.benchmark_group("light_chunk");
    let (g, straddling, _, _) = worldgen();

    let sky = open_sky();
    let mut border = [None; 6];
    border[tesserae_core::mesh::POS_Y] = Some(&sky);

    let center = g.chunk(0, straddling, 0);
    group.bench_function("straddling", |b| {
        b.iter(|| black_box(light_chunk(black_box(&center), black_box(border))))
    });

    // The same Chunk once its neighbours have light of their own, which is what
    // a Chunk at the surface is re-solved against after the first pass. Every
    // face delivers something and nothing is uniform, so no fast path applies
    // and both channels flood — the case the relight budget is sized from.
    let around: Vec<ChunkLight> = [
        (-1, 0, 0),
        (1, 0, 0),
        (0, -1, 0),
        (0, 1, 0),
        (0, 0, -1),
        (0, 0, 1),
    ]
    .iter()
    .map(|(dx, dy, dz)| {
        let mut lit = light_chunk(&g.chunk(*dx, straddling + dy, *dz), border);
        // A lantern's worth of block light, so the second channel floods too.
        lit.set(16, 16, 16, Light::new(MAX_LIGHT, MAX_LIGHT));
        lit
    })
    .collect();
    let settled: [Option<&ChunkLight>; 6] = std::array::from_fn(|i| Some(&around[i]));
    group.bench_function("straddling_settled", |b| {
        b.iter(|| black_box(light_chunk(black_box(&center), black_box(settled))))
    });

    let air = Chunk::air();
    group.bench_function("open_sky", |b| {
        b.iter(|| black_box(light_chunk(black_box(&air), black_box(border))))
    });

    let solid = Chunk::filled(Voxel::cube(testgen::STONE));
    group.bench_function("opaque", |b| {
        b.iter(|| black_box(light_chunk(black_box(&solid), black_box(border))))
    });

    // An air Chunk beside a wall, so the sky arriving from above is no longer
    // flat and the fast path declines: the full 32768-Voxel flood runs. The
    // worst case the ring can ask for, and the number the relight budget is
    // sized against. A *dark* side face would not do it - it delivers nothing,
    // so the fast path is still exact and still fires.
    let mut ragged = ChunkLight::dark();
    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            ragged.set(x, 0, z, Light::new(MAX_LIGHT, 0));
        }
    }
    let mut partial = border;
    partial[tesserae_core::mesh::POS_Y] = Some(&ragged);
    group.bench_function("flooded_air", |b| {
        b.iter(|| black_box(light_chunk(black_box(&air), black_box(partial))))
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
    let sky = open_sky();
    let refs: [Option<LitChunk>; 6] =
        std::array::from_fn(|i| Some(LitChunk::new(&around[i], &sky)));
    let lit_center = LitChunk::new(&center, &sky);
    group.bench_function("straddling", |b| {
        b.iter(|| black_box(mesh_chunk(black_box(lit_center), black_box(refs), shapes())))
    });

    let air = Chunk::air();
    let lit_air = LitChunk::new(&air, &sky);
    group.bench_function("declined_air", |b| {
        b.iter(|| black_box(mesh_chunk(black_box(lit_air), black_box(refs), shapes())))
    });

    let solid = Chunk::filled(Voxel::cube(testgen::STONE));
    let lit_solid = LitChunk::new(&solid, &sky);
    let enclosed: [Option<LitChunk>; 6] = [Some(lit_solid); 6];
    group.bench_function("declined_enclosed", |b| {
        b.iter(|| {
            black_box(mesh_chunk(
                black_box(lit_solid),
                black_box(enclosed),
                shapes(),
            ))
        })
    });

    group.finish();
}

fn neighborhood(c: &mut Criterion) {
    let terrain = testgen::terrain_chunk(0, 0, 0);
    let sky = open_sky();
    let lit = LitChunk::new(&terrain, &sky);
    c.bench_function("neighborhood_isolated", |b| {
        b.iter(|| black_box(Neighborhood::isolated(black_box(lit))))
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

criterion_group!(
    benches,
    mesher,
    streamed_mesher,
    neighborhood,
    generation,
    lighting
);
criterion_main!(benches);
