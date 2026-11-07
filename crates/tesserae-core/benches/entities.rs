use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use tesserae_core::Voxel;
use tesserae_core::collide::{Terrain, VoxelVolume};
use tesserae_core::entity::{DropSpec, Entities, MAX_ENTITIES};
use tesserae_core::testgen::fixtures;

const SIZE_M: f32 = 0.5;

/// Rock at y <= 0 across a 128-Voxel square, air above, unknown outside: the
/// cost measured is the entities', not a store's.
struct Floor;

impl VoxelVolume for Floor {
    fn voxel(&self, at: [i32; 3]) -> Option<Voxel> {
        let inside = at[0].abs() < 64 && at[2].abs() < 64 && (-4..64).contains(&at[1]);
        inside.then(|| {
            if at[1] <= 0 {
                Voxel::cube(fixtures().ground.rock)
            } else {
                Voxel::AIR
            }
        })
    }
}

fn full(steps_before: u32) -> Entities {
    let tables = &fixtures().tables;
    let mut e = Entities::default();
    for i in 0..MAX_ENTITIES as i32 {
        let at = [i % 40 - 20, 3 + i % 5, i / 40 - 25];
        let spec = DropSpec {
            item: (i % 4) as u32,
            count: 1,
            size_m: 0.25,
        };
        e.spawn_drop(
            &Terrain {
                voxels: &Floor,
                tables,
                voxel_size: SIZE_M,
            },
            at,
            spec,
        );
    }
    for _ in 0..steps_before {
        e.step(&Terrain {
            voxels: &Floor,
            tables,
            voxel_size: SIZE_M,
        });
    }
    e
}

fn entities(c: &mut Criterion) {
    let tables = &fixtures().tables;
    let mut group = c.benchmark_group("entities");
    group.bench_function("step_2048_falling", |b| {
        b.iter_batched(
            || full(0),
            |mut e| {
                e.step(black_box(&Terrain {
                    voxels: &Floor,
                    tables,
                    voxel_size: SIZE_M,
                }))
            },
            BatchSize::LargeInput,
        )
    });
    group.bench_function("step_2048_resting", |b| {
        let mut e = full(600);
        b.iter(|| {
            e.step(black_box(&Terrain {
                voxels: &Floor,
                tables,
                voxel_size: SIZE_M,
            }))
        })
    });
    group.finish();
}

criterion_group!(benches, entities);
criterion_main!(benches);
