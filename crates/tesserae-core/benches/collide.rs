use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::sync::OnceLock;
use tesserae_core::collide::{Aabb, VoxelVolume, raycast, sweep};
use tesserae_core::{
    CHUNK_SIZE, Chunk, Orientation, SHAPE_SLAB, ShapeTable, Voxel, mesh::POS_Y, testgen,
};

fn shapes() -> &'static ShapeTable {
    static TABLE: OnceLock<ShapeTable> = OnceLock::new();
    TABLE.get_or_init(ShapeTable::builtin)
}

/// One Chunk at the origin, unknown (`None`) outside it, which is how the Server's
/// store answers for an unloaded Chunk.
struct Scene(Chunk);

impl VoxelVolume for Scene {
    fn voxel(&self, at: [i32; 3]) -> Option<Voxel> {
        let n = CHUNK_SIZE as i32;
        if at.iter().all(|&c| (0..n).contains(&c)) {
            Some(self.0.get(at[0] as usize, at[1] as usize, at[2] as usize))
        } else {
            None
        }
    }
}

const FLOOR_Y: usize = 8;
const WALL_X: usize = 20;

/// A one-Voxel floor at `FLOOR_Y`, and a wall of the same Voxel across x =
/// `WALL_X` standing on it.
fn room(voxel: Voxel) -> Scene {
    let mut chunk = Chunk::air();
    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            chunk.set(x, FLOOR_Y, z, voxel);
        }
        for y in FLOOR_Y + 1..FLOOR_Y + 4 {
            chunk.set(WALL_X, y, z, voxel);
        }
    }
    Scene(chunk)
}

/// A body 0.4 m square and 1.8 m tall with its feet at `(x, y, z)`.
fn body(x: f32, y: f32, z: f32) -> Aabb {
    Aabb::new([x - 0.2, y, z - 0.2], [x + 0.2, y + 1.8, z + 0.2])
}

/// The sweep and the ray on a floor and a wall of cubes, then of slabs, and on
/// terrain. The slab cases are the ones that visit a shaped Voxel's boxes.
fn collide(c: &mut Criterion) {
    let table = shapes();
    let cubes = room(Voxel::cube(testgen::STONE));
    let slabs = room(Voxel::shaped(
        testgen::STONE,
        SHAPE_SLAB,
        Orientation::new(POS_Y, 0),
    ));

    let mut group = c.benchmark_group("collide");
    for (name, scene) in [("cubes", &cubes), ("slabs", &slabs)] {
        group.bench_function(format!("fall_{name}"), |b| {
            b.iter(|| {
                black_box(sweep(
                    scene,
                    table,
                    1.0,
                    black_box(body(5.5, 20.0, 5.5)),
                    [0.0, -15.0, 0.0],
                ))
            })
        });
        group.bench_function(format!("walk_into_wall_{name}"), |b| {
            b.iter(|| {
                black_box(sweep(
                    scene,
                    table,
                    1.0,
                    black_box(body(5.5, 10.0, 12.5)),
                    [14.0, 0.0, 0.0],
                ))
            })
        });
        group.bench_function(format!("ray_{name}"), |b| {
            b.iter(|| {
                black_box(raycast(
                    scene,
                    table,
                    1.0,
                    black_box([2.5, 25.5, 2.5]),
                    [1.0, -0.6, 0.8],
                    32.0,
                ))
            })
        });
    }

    let terrain = Scene(testgen::terrain_chunk(0, 0, 0));
    group.bench_function("ray_terrain", |b| {
        b.iter(|| {
            black_box(raycast(
                &terrain,
                table,
                1.0,
                black_box([0.5, 31.5, 0.5]),
                [1.0, -0.3, 1.0],
                40.0,
            ))
        })
    });
    group.bench_function("fall_terrain", |b| {
        b.iter(|| {
            black_box(sweep(
                &terrain,
                table,
                1.0,
                black_box(body(10.5, 31.0, 10.5)),
                [0.0, -30.0, 0.0],
            ))
        })
    });
    group.finish();
}

criterion_group!(benches, collide);
criterion_main!(benches);
