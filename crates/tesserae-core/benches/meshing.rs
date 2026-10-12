use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::sync::{Arc, OnceLock};
use tesserae_core::light::{Light, MAX_LIGHT, light_chunk};
use tesserae_core::{
    CHUNK_SIZE, Chunk, ChunkLight, LitChunk, MAX_SHAPED_PER_CHUNK, Neighborhood, Orientation,
    SHAPE_STAIR, ShapeTable, Voxel, Worldgen, WorldgenParams, greedy_mesh, mesh_chunk, testgen,
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
///
/// Caveless, so every number from before caves existed still measures the same
/// Chunks; `caved` is the same World with its tunnels.
fn worldgen() -> (Worldgen, i32, i32, i32) {
    let params = WorldgenParams {
        caves: None,
        ..WorldgenParams::default()
    };
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

/// The same World as [`worldgen`] with its ground left as cubes, so a row can
/// separate what the generator itself costs from the shaped Voxels it now makes.
fn unsmoothed() -> Worldgen {
    Worldgen::new(
        0x7e55_e2ae,
        1.0,
        WorldgenParams {
            caves: None,
            smoothing: None,
            ..WorldgenParams::default()
        },
    )
}

/// The Chunk, in a fixed scan at the surface layer, with the most slope-smoothing
/// stairs and slabs: the worst a default World hands the mesher.
fn densest_ramp(g: &Worldgen, cy: i32) -> (i32, i32, i32) {
    let mut best = (0, (0, cy, 0));
    for cz in -8..8 {
        for cx in -8..8 {
            let n = g.chunk(cx, cy, cz).shaped_count();
            if n > best.0 {
                best = (n, (cx, cy, cz));
            }
        }
    }
    assert!(best.0 > 0, "the default World has no ramp in the scan");
    best.1
}

/// A Chunk's six face neighbours, in the mesher's order.
fn chunks_around(g: &Worldgen, (cx, cy, cz): (i32, i32, i32)) -> [Chunk; 6] {
    [
        g.chunk(cx - 1, cy, cz),
        g.chunk(cx + 1, cy, cz),
        g.chunk(cx, cy - 1, cz),
        g.chunk(cx, cy + 1, cz),
        g.chunk(cx, cy, cz - 1),
        g.chunk(cx, cy, cz + 1),
    ]
}

/// Chunk coordinates of the cave cases streaming now meets, found by scanning
/// in a fixed order so a number means the same Chunk on every run.
struct CaveFixtures {
    /// A deep Chunk inside the cave band that a tunnel crosses, and one that no
    /// tunnel does.
    crossed: (i32, i32, i32),
    empty: (i32, i32, i32),
    /// A surface Chunk where a tunnel breaches the Grass cap.
    mouth: (i32, i32, i32),
    /// The layer of deep Chunks in the band, for a mean over the mix.
    layer_cy: i32,
}

fn cave_fixtures(g: &Worldgen) -> CaveFixtures {
    let params = cave_params();
    let caves = params.caves.expect("the default World has caves");
    let cell = CHUNK_SIZE as f32;
    // The layer just under the soil line that the cave floor reaches.
    let layer_cy = ((params.sea_level_m - params.soil_depth_m) / cell).floor() as i32 - 1;
    assert!(
        (layer_cy as f32 + 1.0) * cell > caves.floor_m,
        "the default band has no deep layer"
    );
    let bare = Worldgen::new(
        0x7e55_e2ae,
        1.0,
        WorldgenParams {
            caves: None,
            ..params
        },
    );
    let at_surface = (g.surface_height_m(0.0, 0.0) / cell).floor() as i32;

    let (mut crossed, mut empty, mut mouth) = (None, None, None);
    for cz in -8..8 {
        for cx in -8..8 {
            let deep = g.chunk(cx, layer_cy, cz);
            if deep.uniform().is_none() {
                crossed.get_or_insert((cx, layer_cy, cz));
            } else {
                empty.get_or_insert((cx, layer_cy, cz));
            }
            if mouth.is_none() {
                let (with, without) = (g.chunk(cx, at_surface, cz), bare.chunk(cx, at_surface, cz));
                let breached = (0..CHUNK_SIZE).any(|z| {
                    (0..CHUNK_SIZE).any(|y| {
                        (0..CHUNK_SIZE).any(|x| {
                            with.get(x, y, z).is_air()
                                && without.get(x, y, z).material() == testgen::GRASS
                        })
                    })
                });
                if breached {
                    mouth = Some((cx, at_surface, cz));
                }
            }
        }
    }
    CaveFixtures {
        crossed: crossed.expect("a tunnel crosses a deep Chunk in the scan"),
        empty: empty.expect("a deep Chunk in the scan holds no tunnel"),
        mouth: mouth.expect("a tunnel breaches the surface in the scan"),
        layer_cy,
    }
}

/// The cave benches measure tunnels, so they keep terrain of cubes: they read
/// the same Chunks as when caves were added.
fn cave_params() -> WorldgenParams {
    WorldgenParams {
        smoothing: None,
        ..WorldgenParams::default()
    }
}

fn caved() -> Worldgen {
    Worldgen::new(0x7e55_e2ae, 1.0, cave_params())
}

/// Terrain whose top `n` Voxels - the exposed ones, one per column - are stairs
/// facing every which way: pass 2 as it would meet a building, which is more
/// than a Chunk is allowed to hold once `n` passes `MAX_SHAPED_PER_CHUNK`.
fn terrain_with_stairs(n: usize) -> Chunk {
    let mut chunk = testgen::terrain_chunk(0, 0, 0);
    let mut placed = 0;
    'columns: for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            if placed == n {
                break 'columns;
            }
            let Some(y) = (0..CHUNK_SIZE)
                .rev()
                .find(|&y| !chunk.get(x, y, z).is_air())
            else {
                continue;
            };
            let o = Orientation::from_bits(((x * 7 + z * 3) % 24) as u8).unwrap();
            chunk.set(x, y, z, Voxel::shaped(testgen::STONE, SHAPE_STAIR, o));
            placed += 1;
        }
    }
    chunk
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

    // Pass 2, which terrain never reaches. The first is the most a Chunk may
    // hold; the second is why there is a cap.
    let stairs = terrain_with_stairs(MAX_SHAPED_PER_CHUNK);
    let stairs_nb = Neighborhood::isolated(LitChunk::new(&stairs, &sky));
    group.bench_function("terrain_stairs_at_cap", |b| {
        b.iter(|| black_box(greedy_mesh(black_box(&stairs_nb), shapes())))
    });
    // What the generator may hand the mesher at most: its half of the cap.
    let budget = terrain_with_stairs(tesserae_core::MAX_SMOOTHED_PER_CHUNK);
    let budget_nb = Neighborhood::isolated(LitChunk::new(&budget, &sky));
    group.bench_function("terrain_stairs_at_budget", |b| {
        b.iter(|| black_box(greedy_mesh(black_box(&budget_nb), shapes())))
    });
    let all = Chunk::filled(Voxel::shaped(
        testgen::STONE,
        SHAPE_STAIR,
        Orientation::IDENTITY,
    ));
    let all_nb = Neighborhood::isolated(LitChunk::new(&all, &sky));
    group.bench_function("all_stairs", |b| {
        b.iter(|| black_box(greedy_mesh(black_box(&all_nb), shapes())))
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
    let air_above = Chunk::air();
    let mut border = [None; 6];
    border[tesserae_core::mesh::POS_Y] = Some(LitChunk::new(&air_above, &sky));

    let center = g.chunk(0, straddling, 0);
    group.bench_function("straddling", |b| {
        b.iter(|| black_box(light_chunk(black_box(&center), black_box(border), shapes())))
    });

    // The same Chunk once its neighbours have light of their own, which is what
    // a Chunk at the surface is re-solved against after the first pass. Every
    // face delivers something and nothing is uniform, so no fast path applies
    // and both channels flood — the case the relight budget is sized from.
    let beside: Vec<Chunk> = [
        (-1, 0, 0),
        (1, 0, 0),
        (0, -1, 0),
        (0, 1, 0),
        (0, 0, -1),
        (0, 0, 1),
    ]
    .iter()
    .map(|(dx, dy, dz)| g.chunk(*dx, straddling + dy, *dz))
    .collect();
    let around: Vec<ChunkLight> = beside
        .iter()
        .map(|chunk| {
            let mut lit = light_chunk(chunk, border, shapes());
            // A lantern's worth of block light, so the second channel floods too.
            lit.set(16, 16, 16, Light::new(MAX_LIGHT, MAX_LIGHT));
            lit
        })
        .collect();
    let settled: [Option<LitChunk>; 6] =
        std::array::from_fn(|i| Some(LitChunk::new(&beside[i], &around[i])));
    group.bench_function("straddling_settled", |b| {
        b.iter(|| {
            black_box(light_chunk(
                black_box(&center),
                black_box(settled),
                shapes(),
            ))
        })
    });

    // The general edge rule, which a Chunk with a shaped Voxel in it pays for and
    // terrain does not: the surface Chunk with a stair on top of every column.
    let stairs = terrain_with_stairs(MAX_SHAPED_PER_CHUNK);
    group.bench_function("straddling_shaped", |b| {
        b.iter(|| black_box(light_chunk(black_box(&stairs), black_box(border), shapes())))
    });

    // The same Chunk with the ground left as cubes, and the densest ramp the
    // default World makes, so the input change and the code change can be read
    // apart. Both arrive against open sky above, as `straddling` does.
    let bare = unsmoothed();
    let bare_center = bare.chunk(0, straddling, 0);
    group.bench_function("straddling_unsmoothed", |b| {
        b.iter(|| {
            black_box(light_chunk(
                black_box(&bare_center),
                black_box(border),
                shapes(),
            ))
        })
    });
    let ramp_at = densest_ramp(&g, straddling);
    let ramp = g.chunk(ramp_at.0, ramp_at.1, ramp_at.2);
    group.bench_function("ramp_densest", |b| {
        b.iter(|| black_box(light_chunk(black_box(&ramp), black_box(border), shapes())))
    });

    // What a cave costs the relight stage. A deep Chunk arrives against
    // neighbours nobody has solved, so every face delivers darkness; the mouth
    // Chunk has sky arriving from above and a tunnel to carry it down.
    // From here on the World has its tunnels.
    let g = caved();
    let cave = cave_fixtures(&g);
    let dark = ChunkLight::dark();
    let neighbours_of = |(cx, cy, cz): (i32, i32, i32)| -> Vec<Chunk> {
        [
            (-1, 0, 0),
            (1, 0, 0),
            (0, -1, 0),
            (0, 1, 0),
            (0, 0, -1),
            (0, 0, 1),
        ]
        .iter()
        .map(|(dx, dy, dz)| g.chunk(cx + dx, cy + dy, cz + dz))
        .collect()
    };
    let crossed = g.chunk(cave.crossed.0, cave.crossed.1, cave.crossed.2);
    let crossed_around = neighbours_of(cave.crossed);
    let crossed_border: [Option<LitChunk>; 6] =
        std::array::from_fn(|i| Some(LitChunk::new(&crossed_around[i], &dark)));
    group.bench_function("cave_dark_neighbours", |b| {
        b.iter(|| {
            black_box(light_chunk(
                black_box(&crossed),
                black_box(crossed_border),
                shapes(),
            ))
        })
    });

    let mouth = g.chunk(cave.mouth.0, cave.mouth.1, cave.mouth.2);
    group.bench_function("cave_mouth", |b| {
        b.iter(|| black_box(light_chunk(black_box(&mouth), black_box(border), shapes())))
    });

    let air = Chunk::air();
    group.bench_function("open_sky", |b| {
        b.iter(|| black_box(light_chunk(black_box(&air), black_box(border), shapes())))
    });

    let solid = Chunk::filled(Voxel::cube(testgen::STONE));
    group.bench_function("opaque", |b| {
        b.iter(|| black_box(light_chunk(black_box(&solid), black_box(border), shapes())))
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
    partial[tesserae_core::mesh::POS_Y] = Some(LitChunk::new(&air_above, &ragged));
    group.bench_function("flooded_air", |b| {
        b.iter(|| black_box(light_chunk(black_box(&air), black_box(partial), shapes())))
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

    // The same Chunk with the ground left as cubes, and the densest ramp.
    let bare = unsmoothed();
    let bare_center = bare.chunk(0, straddling, 0);
    let bare_around = chunks_around(&bare, (0, straddling, 0));
    let bare_refs: [Option<LitChunk>; 6] =
        std::array::from_fn(|i| Some(LitChunk::new(&bare_around[i], &sky)));
    let lit_bare = LitChunk::new(&bare_center, &sky);
    group.bench_function("straddling_unsmoothed", |b| {
        b.iter(|| {
            black_box(mesh_chunk(
                black_box(lit_bare),
                black_box(bare_refs),
                shapes(),
            ))
        })
    });
    let ramp_at = densest_ramp(&g, straddling);
    let ramp = g.chunk(ramp_at.0, ramp_at.1, ramp_at.2);
    let ramp_around = chunks_around(&g, ramp_at);
    let ramp_refs: [Option<LitChunk>; 6] =
        std::array::from_fn(|i| Some(LitChunk::new(&ramp_around[i], &sky)));
    let lit_ramp = LitChunk::new(&ramp, &sky);
    group.bench_function("ramp_densest", |b| {
        b.iter(|| {
            black_box(mesh_chunk(
                black_box(lit_ramp),
                black_box(ramp_refs),
                shapes(),
            ))
        })
    });

    // A tunnel through rock, lit as an enclosed one is (dark), and a mouth lit
    // by the sky. The deep Chunk was the 6 ns `declined_enclosed` case before.
    // From here on the World has its tunnels.
    let g = caved();
    let cave = cave_fixtures(&g);
    let dark = ChunkLight::dark();
    let crossed = g.chunk(cave.crossed.0, cave.crossed.1, cave.crossed.2);
    let (cx, cy, cz) = cave.crossed;
    let crossed_around = [
        g.chunk(cx - 1, cy, cz),
        g.chunk(cx + 1, cy, cz),
        g.chunk(cx, cy - 1, cz),
        g.chunk(cx, cy + 1, cz),
        g.chunk(cx, cy, cz - 1),
        g.chunk(cx, cy, cz + 1),
    ];
    let crossed_refs: [Option<LitChunk>; 6] =
        std::array::from_fn(|i| Some(LitChunk::new(&crossed_around[i], &dark)));
    let lit_crossed = LitChunk::new(&crossed, &dark);
    group.bench_function("cave_crossed", |b| {
        b.iter(|| {
            black_box(mesh_chunk(
                black_box(lit_crossed),
                black_box(crossed_refs),
                shapes(),
            ))
        })
    });

    let (cx, cy, cz) = cave.mouth;
    let mouth = g.chunk(cx, cy, cz);
    let mouth_around = [
        g.chunk(cx - 1, cy, cz),
        g.chunk(cx + 1, cy, cz),
        g.chunk(cx, cy - 1, cz),
        g.chunk(cx, cy + 1, cz),
        g.chunk(cx, cy, cz - 1),
        g.chunk(cx, cy, cz + 1),
    ];
    let mouth_refs: [Option<LitChunk>; 6] =
        std::array::from_fn(|i| Some(LitChunk::new(&mouth_around[i], &sky)));
    let lit_mouth = LitChunk::new(&mouth, &sky);
    group.bench_function("cave_mouth", |b| {
        b.iter(|| {
            black_box(mesh_chunk(
                black_box(lit_mouth),
                black_box(mouth_refs),
                shapes(),
            ))
        })
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
    // Without the smoothing pass and its border of surface heights: what
    // `straddling` was before ramps, so the pass's own cost is the difference.
    let bare = unsmoothed();
    group.bench_function("straddling_unsmoothed", |b| {
        b.iter(|| black_box(bare.chunk(black_box(0), straddling, 0)))
    });
    let (rx, ry, rz) = densest_ramp(&g, straddling);
    group.bench_function("ramp_densest", |b| {
        b.iter(|| black_box(g.chunk(black_box(rx), ry, rz)))
    });
    // Both early-outs: most Chunks in a streamed 3D ring are one of these.
    group.bench_function("early_out_air", |b| {
        b.iter(|| black_box(g.chunk(black_box(0), sky, 0)))
    });
    group.bench_function("early_out_stone", |b| {
        b.iter(|| black_box(g.chunk(black_box(0), deep, 0)))
    });

    // The Chunks a tunnel makes dearer. `cave_crossed` is the carved path.
    // `cave_band_uniform` is the first Chunk in scan order that is inside the
    // band and stays uniform; the range test rules it out, which is the best
    // case. `cave_band_layer_of_64` is the mean over a layer, false positives
    // included: what a streamed ring pays per deep Chunk in the band.
    // From here on the World has its tunnels.
    let g = caved();
    let cave = cave_fixtures(&g);
    // The same surface Chunk as `straddling`, with the Chunk-level range test.
    group.bench_function("straddling_caved", |b| {
        b.iter(|| black_box(g.chunk(black_box(0), straddling, 0)))
    });
    let (cx, cy, cz) = cave.crossed;
    group.bench_function("cave_crossed", |b| {
        b.iter(|| black_box(g.chunk(black_box(cx), cy, cz)))
    });
    let (cx, cy, cz) = cave.empty;
    group.bench_function("cave_band_uniform", |b| {
        b.iter(|| black_box(g.chunk(black_box(cx), cy, cz)))
    });
    let (cx, cy, cz) = cave.mouth;
    group.bench_function("cave_mouth", |b| {
        b.iter(|| black_box(g.chunk(black_box(cx), cy, cz)))
    });
    group.sample_size(20);
    group.bench_function("cave_band_layer_of_64", |b| {
        b.iter(|| {
            for cz in 0..8 {
                for cx in 0..8 {
                    black_box(g.chunk(black_box(cx), cave.layer_cy, cz));
                }
            }
        })
    });

    group.finish();

    // Renamed from `worldgen_chunk`, which it never was: this is the bench
    // fixture, tuned in Voxel indices, and the three benches above are the
    // generator. The id change is a rename, not a regression.
    c.bench_function("testgen_terrain_chunk", |b| {
        b.iter(|| black_box(testgen::terrain_chunk(black_box(0), 0, 0)))
    });
}

/// Registering the built-in Shapes: derived once at load, so the number only has
/// to be small next to starting up. A Mod registering all 255 pays 64 times it.
fn shapes_table(c: &mut Criterion) {
    c.bench_function("shape_table_builtin", |b| {
        b.iter(|| black_box(ShapeTable::builtin()))
    });
}

/// What handing a mesh job its data costs the thread that dispatches it: the
/// seven Chunks and seven `ChunkLight`s a mesh reads, copied or shared. The
/// surface-crossing Chunk with its dense light is the expensive case for the
/// copy; the second pair is what a write costs once the store holds `Arc`s.
fn snapshot(c: &mut Criterion) {
    let mut group = c.benchmark_group("snapshot");
    let (g, straddling, _, _) = worldgen();
    let chunk = g.chunk(0, straddling, 0);
    let sky = open_sky();
    let air_above = Chunk::air();
    let mut border = [None; 6];
    border[tesserae_core::mesh::POS_Y] = Some(LitChunk::new(&air_above, &sky));
    let light = light_chunk(&chunk, border, shapes());

    let chunks: Vec<Chunk> = vec![chunk.clone(); 7];
    let lights: Vec<ChunkLight> = vec![light.clone(); 7];
    group.bench_function("clone_seven_chunks_and_lights", |b| {
        b.iter(|| black_box((black_box(&chunks).clone(), black_box(&lights).clone())))
    });

    let shared_chunks: Vec<Arc<Chunk>> = chunks.iter().cloned().map(Arc::new).collect();
    let shared_lights: Vec<Arc<ChunkLight>> = lights.iter().cloned().map(Arc::new).collect();
    group.bench_function("arc_clone_seven_chunks_and_lights", |b| {
        b.iter(|| {
            black_box((
                black_box(&shared_chunks).clone(),
                black_box(&shared_lights).clone(),
            ))
        })
    });

    let stone = Voxel::cube(testgen::STONE);
    let mut unshared = Arc::new(chunk.clone());
    let mut flip = false;
    group.bench_function("make_mut_unshared", |b| {
        b.iter(|| {
            flip = !flip;
            let voxel = if flip { stone } else { Voxel::AIR };
            Arc::make_mut(&mut unshared).set(1, 1, 1, voxel);
        })
    });

    let base = Arc::new(chunk.clone());
    group.bench_function("make_mut_shared", |b| {
        b.iter(|| {
            let mut held = Arc::clone(&base);
            Arc::make_mut(&mut held).set(1, 1, 1, stone);
            black_box(held)
        })
    });

    let base_light = Arc::new(light.clone());
    group.bench_function("make_mut_shared_light", |b| {
        b.iter(|| {
            let mut held = Arc::clone(&base_light);
            Arc::make_mut(&mut held).set(1, 1, 1, Light::new(MAX_LIGHT, 0));
            black_box(held)
        })
    });
    group.finish();
}

criterion_group!(
    benches,
    snapshot,
    shapes_table,
    mesher,
    streamed_mesher,
    neighborhood,
    generation,
    lighting
);
criterion_main!(benches);
