use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use std::collections::HashMap;
use std::hint::black_box;
use tesserae_core::mesh::{FACE_STEPS, LitChunk};
use tesserae_core::testgen::fixtures;
use tesserae_core::{
    CHUNK_SIZE, Chunk, ChunkLight, Light, LightVolume, MAX_LIGHT, Voxel, light_chunk, relight,
    relight_many,
};

const N: i32 = CHUNK_SIZE as i32;

/// Chunks addressed in Voxel coordinates through a hash map, as the Server's
/// store is: the per-access cost is part of what is measured.
#[derive(Clone)]
struct Lattice {
    cells: HashMap<[i32; 3], (Chunk, ChunkLight)>,
}

fn locate(at: [i32; 3]) -> ([i32; 3], usize, usize, usize) {
    let c = at.map(|v| v.div_euclid(N));
    let l = at.map(|v| v.rem_euclid(N) as usize);
    (c, l[0], l[1], l[2])
}

impl LightVolume for Lattice {
    fn voxel(&self, at: [i32; 3]) -> Option<Voxel> {
        let (c, x, y, z) = locate(at);
        self.cells.get(&c).map(|(chunk, _)| chunk.get(x, y, z))
    }

    fn light(&self, at: [i32; 3]) -> Light {
        let (c, x, y, z) = locate(at);
        self.cells
            .get(&c)
            .map_or(Light::DARK, |(_, light)| light.get(x, y, z))
    }

    fn set_light(&mut self, at: [i32; 3], light: Light) {
        let (c, x, y, z) = locate(at);
        if let Some((_, stored)) = self.cells.get_mut(&c) {
            stored.set(x, y, z, light);
        }
    }
}

impl Lattice {
    fn set_voxel(&mut self, at: [i32; 3], voxel: Voxel) {
        let (c, x, y, z) = locate(at);
        self.cells.get_mut(&c).expect("held").0.set(x, y, z, voxel);
    }
}

fn stone() -> Voxel {
    Voxel::cube(fixtures().ground.rock)
}

/// A 3x2x3 block of Chunks under a layer of open sky, with stone filling the
/// bottom eight Voxels and `extra` placed on it, lit by relaxing the bulk
/// solver to its fixed point, as a World that streamed in is.
fn scene(extra: &[[i32; 3]]) -> Lattice {
    let mut cells = HashMap::new();
    for cy in 0..=2 {
        for cz in -1..=1 {
            for cx in -1..=1 {
                let mut chunk = Chunk::air();
                if cy == 0 {
                    for y in 0..8 {
                        for z in 0..CHUNK_SIZE {
                            for x in 0..CHUNK_SIZE {
                                chunk.set(x, y, z, stone());
                            }
                        }
                    }
                }
                let light = if cy == 2 {
                    ChunkLight::filled(Light::new(MAX_LIGHT, 0))
                } else {
                    ChunkLight::dark()
                };
                cells.insert([cx, cy, cz], (chunk, light));
            }
        }
    }
    let mut lattice = Lattice { cells };
    for &at in extra {
        lattice.set_voxel(at, stone());
    }
    let mut coords: Vec<[i32; 3]> = lattice.cells.keys().copied().filter(|c| c[1] < 2).collect();
    coords.sort_by_key(|c| std::cmp::Reverse(c[1]));
    loop {
        let mut changed = false;
        for &coord in &coords {
            let border: [Option<LitChunk<'_>>; 6] = std::array::from_fn(|face| {
                let d = FACE_STEPS[face];
                let (chunk, light) =
                    lattice
                        .cells
                        .get(&[coord[0] + d[0], coord[1] + d[1], coord[2] + d[2]])?;
                Some(LitChunk::new(chunk, light))
            });
            let solved = light_chunk(&lattice.cells[&coord].0, border, &fixtures().tables);
            if solved != lattice.cells[&coord].1 {
                lattice.cells.get_mut(&coord).unwrap().1 = solved;
                changed = true;
            }
        }
        if !changed {
            return lattice;
        }
    }
}

/// The positions of a box from `min` to `max` exclusive, in a fill Command's
/// order: x fastest, then z, then y, so it goes up a layer at a time.
fn box_order(min: [i32; 3], max: [i32; 3]) -> Vec<[i32; 3]> {
    let mut out = Vec::new();
    for y in min[1]..max[1] {
        for z in min[2]..max[2] {
            for x in min[0]..max[0] {
                out.push([x, y, z]);
            }
        }
    }
    out
}

/// One `relight` per Voxel, as `set_occupants` does today.
fn one_at_a_time(lattice: &mut Lattice, writes: &[[i32; 3]], voxel: Voxel) {
    let tables = &fixtures().tables;
    for &at in writes {
        let was = lattice.light(at);
        lattice.set_voxel(at, voxel);
        relight(lattice, tables, at, was);
    }
}

/// The same writes in batches of `size`, each relit once, as a drain tick does.
fn batched(lattice: &mut Lattice, writes: &[[i32; 3]], voxel: Voxel, size: usize) {
    let tables = &fixtures().tables;
    for batch in writes.chunks(size) {
        let was: Vec<_> = batch
            .iter()
            .map(|&at| {
                let was = lattice.light(at);
                lattice.set_voxel(at, voxel);
                (at, was)
            })
            .collect();
        relight_many(lattice, tables, &was);
    }
}

fn bench(c: &mut Criterion) {
    let solid = box_order([8, 8, 8], [24, 24, 24]);
    let roof = box_order([8, 24, 8], [24, 25, 24]);
    let cases = [
        ("place_box_16", scene(&[]), solid.clone(), stone()),
        ("remove_box_16", scene(&solid), solid, Voxel::AIR),
        ("roof_16x16", scene(&[]), roof, stone()),
    ];
    let mut group = c.benchmark_group("relight");
    group.sample_size(10);
    for (name, start, writes, voxel) in &cases {
        group.throughput(Throughput::Elements(writes.len() as u64));
        group.bench_function(format!("{name}/one_at_a_time"), |b| {
            b.iter_batched(
                || start.clone(),
                |mut lattice| {
                    one_at_a_time(&mut lattice, writes, *voxel);
                    black_box(lattice)
                },
                BatchSize::LargeInput,
            )
        });
        for size in [1, 64, 4096] {
            group.bench_function(format!("{name}/batch_{size}"), |b| {
                b.iter_batched(
                    || start.clone(),
                    |mut lattice| {
                        batched(&mut lattice, writes, *voxel, size);
                        black_box(lattice)
                    },
                    BatchSize::LargeInput,
                )
            });
        }
    }
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
