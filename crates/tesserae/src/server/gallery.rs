//! A gallery of every built-in Shape near the spawn point. `dev` only.
//!
//! Terrain contains no shaped Voxel, so nothing in build-order step 5 is
//! observable without this, and the `dev` capture cannot click. It places
//! through the same `edit::set_voxel` a player's click ends in, so what it shows
//! is the edit path, the second mesher pass and per-face light on the running
//! app and not only in tests.
//!
//! What is on show, left to right and front to back:
//!
//! - two rows of the Shapes in their orientations, so a wrong rotation is a
//!   thing that is visibly wrong;
//! - a flight of stairs rising away from the camera over a solid mass, with a
//!   slab step on top, so a stair's sealed back and a cube beside it can be
//!   seen to join without a gap or a doubled face;
//! - a floor of slabs with a wall behind it, whose interior faces are culled by
//!   coverage and not left as four hidden quads on every slab;
//! - a lantern, so block light falls on shaped faces.
//!
//! **The pattern is in Voxels and the placement is in metres.** The point is the
//! shape of individual Voxels, so the layout is a count of them, as `testgen`'s
//! is; where it stands is a metre position ahead of the spawn point, converted
//! once.

use bevy::prelude::*;
use tesserae_core::mesh::{NEG_Y, POS_X, POS_Y};
use tesserae_core::worldgen::{DIRT, GRASS, LANTERN, STONE};
use tesserae_core::{MaterialId, Orientation, SHAPE_POST, SHAPE_SLAB, SHAPE_STAIR, Voxel};

use super::builder::{self, Progress, Step};
use super::streaming::MeshState;
use super::world_state::{ChunkStore, SpawnPoint};
use super::{ContentTables, WorldGenerator};
use crate::world::WorldSettings;

/// Where the gallery's near corner is, in metres from the spawn point: ahead
/// along -Z, where the Client points its camera, and to one side. Far enough
/// ahead that the camera, which starts well above the spawn point and tipped
/// down, has the whole of it in frame rather than its near edge below the bottom
/// of the picture.
const AHEAD_M: f32 = 44.0;
const ASIDE_M: f32 = 11.0;

/// The footprint in Voxels, along x and z.
const WIDTH: i32 = 11;
const DEPTH: i32 = 14;

type Plan = Vec<([i32; 3], Voxel)>;

pub(super) fn plugin(app: &mut App) {
    // After the tick's own streaming.
    app.add_systems(FixedPostUpdate, build);
}

fn build(
    mut store: ResMut<ChunkStore>,
    mut meshes: ResMut<MeshState>,
    mut progress: Local<Progress>,
    generator: Res<WorldGenerator>,
    tables: Res<ContentTables>,
    world: Res<WorldSettings>,
    spawn: Option<Res<SpawnPoint>>,
) {
    let Some(spawn) = spawn else {
        return;
    };
    let corner = spawn.metres() + Vec3::new(ASIDE_M, 0.0, -AHEAD_M);
    let plan = plan(&world, corner, |x, z| generator.surface_height_m(x, z));
    if builder::step(&plan, &mut progress, &mut store, &mut meshes, &tables) == Step::Completed {
        info!("gallery: {} Voxels at {corner:?}", plan.len());
    }
}

fn shaped(material: MaterialId, shape: u8, up: usize, turn: u8) -> Voxel {
    Voxel::shaped(material, shape, Orientation::new(up, turn))
}

/// The Voxels to place, in lattice coordinates, given the metre position of the
/// gallery's near corner and the height of the ground at any point.
///
/// It stands on a plinth of stone raised to the highest ground under it, so no
/// part of it is buried however the terrain slopes.
fn plan(world: &WorldSettings, corner: Vec3, surface_m: impl Fn(f32, f32) -> f32) -> Plan {
    let size = world.voxel_size;
    let [ox, _, oz] = world.voxel_at(corner);

    let ground = |i: i32, k: i32| {
        let x = (ox + i) as f32 * size + size / 2.0;
        let z = (oz + k) as f32 * size + size / 2.0;
        (surface_m(x, z) / size).floor() as i32
    };
    let columns = (0..WIDTH).flat_map(|i| (0..DEPTH).map(move |k| (i, k)));
    let highest = columns.clone().map(|(i, k)| ground(i, k)).max().unwrap();

    let mut plan = Plan::new();
    for (i, k) in columns {
        for y in ground(i, k)..=highest {
            plan.push(([ox + i, y, oz + k], Voxel::cube(STONE)));
        }
    }

    // `at(x, j, k)`: j counts up from the plinth.
    let at = |i: i32, j: i32, k: i32| [ox + i, highest + 1 + j, oz + k];
    let mut put = |i, j, k, voxel| plan.push((at(i, j, k), voxel));

    // The catalogue: front row, then the row behind it.
    put(0, 0, 12, Voxel::cube(STONE));
    put(2, 0, 12, shaped(DIRT, SHAPE_SLAB, POS_Y, 0));
    put(4, 0, 12, shaped(STONE, SHAPE_SLAB, NEG_Y, 0));
    put(6, 0, 12, shaped(DIRT, SHAPE_STAIR, POS_Y, 0));
    put(8, 0, 12, shaped(STONE, SHAPE_STAIR, POS_Y, 1));
    put(0, 0, 9, shaped(DIRT, SHAPE_STAIR, POS_Y, 2));
    put(2, 0, 9, shaped(STONE, SHAPE_STAIR, POS_Y, 3));
    put(4, 0, 9, shaped(DIRT, SHAPE_STAIR, NEG_Y, 0));
    put(6, 0, 9, shaped(STONE, SHAPE_SLAB, POS_X, 0));
    put(8, 0, 9, shaped(DIRT, SHAPE_POST, POS_Y, 0));
    put(10, 0, 10, Voxel::cube(LANTERN));

    // A flight rising away from the camera: each step's tall back faces -Z, over
    // cubes up to its own height.
    for step in 0..5 {
        for x in 0..3 {
            for y in 0..step {
                put(x, y, 6 - step, Voxel::cube(STONE));
            }
            put(x, step, 6 - step, shaped(STONE, SHAPE_STAIR, POS_Y, 2));
        }
    }
    // And a platform at the top with a slab for its last half step.
    for z in 0..2 {
        for x in 0..3 {
            for y in 0..5 {
                put(x, y, z, Voxel::cube(STONE));
            }
            put(x, 5, z, shaped(STONE, SHAPE_SLAB, POS_Y, 0));
        }
    }
    put(1, 6, 0, Voxel::cube(LANTERN));

    // A slab floor, and a wall standing behind it.
    for z in 2..=4 {
        for x in 5..=8 {
            put(x, 0, z, shaped(GRASS, SHAPE_SLAB, POS_Y, 0));
        }
    }
    for x in 4..=9 {
        for y in 0..2 {
            put(x, y, 1, Voxel::cube(STONE));
        }
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::world_state::VoxelCoord;
    use std::collections::{BTreeSet, HashMap};
    use tesserae_core::{MAX_SHAPED_PER_CHUNK, ShapeTable};

    /// Sloping ground, so the plinth has something to do.
    fn hill(x: f32, _z: f32) -> f32 {
        40.0 + x * 0.2
    }

    fn world(voxel_size: f32) -> WorldSettings {
        WorldSettings {
            voxel_size,
            ..default()
        }
    }

    fn corner() -> Vec3 {
        Vec3::new(11.0, 0.0, -44.0)
    }

    #[test]
    fn it_shows_every_built_in_shape_and_the_stair_in_more_than_one_orientation() {
        let plan = plan(&world(1.0), corner(), hill);
        let shapes: BTreeSet<u8> = plan
            .iter()
            .filter(|(_, v)| v.is_shaped())
            .map(|(_, v)| v.shape())
            .collect();
        assert_eq!(
            shapes,
            BTreeSet::from([SHAPE_SLAB, SHAPE_STAIR, SHAPE_POST])
        );
        let stairs: BTreeSet<u8> = plan
            .iter()
            .filter(|(_, v)| v.shape() == SHAPE_STAIR)
            .map(|(_, v)| v.orientation())
            .collect();
        assert!(stairs.len() >= 4, "{stairs:?}");
    }

    #[test]
    fn it_stays_well_under_the_shaped_cap_in_every_chunk() {
        let plan = plan(&world(1.0), corner(), hill);
        let mut per_chunk: HashMap<_, usize> = HashMap::new();
        for (at, v) in &plan {
            if v.is_shaped() {
                *per_chunk.entry(VoxelCoord::of(*at).chunk).or_default() += 1;
            }
        }
        assert!(!per_chunk.is_empty());
        for (chunk, n) in per_chunk {
            assert!(n * 10 < MAX_SHAPED_PER_CHUNK, "{chunk}: {n} shaped");
        }
    }

    #[test]
    fn every_voxel_in_it_is_one_the_edit_path_would_accept() {
        let table = ShapeTable::builtin();
        for (at, v) in plan(&world(1.0), corner(), hill) {
            assert!(!v.is_air(), "{at:?}");
            assert!(table.contains(v.shape()), "{at:?}");
            assert!(Orientation::from_bits(v.orientation()).is_some(), "{at:?}");
        }
    }

    #[test]
    fn it_stands_where_it_says_and_nowhere_else() {
        let w = world(1.0);
        let [ox, _, oz] = w.voxel_at(corner());
        for (at, _) in plan(&w, corner(), hill) {
            assert!((ox..ox + WIDTH).contains(&at[0]), "{at:?} in x");
            assert!((oz..oz + DEPTH).contains(&at[2]), "{at:?} in z");
        }
    }

    #[test]
    fn the_things_on_show_stand_above_the_highest_ground_under_them() {
        let w = world(1.0);
        let plan = plan(&w, corner(), hill);
        let [ox, _, oz] = w.voxel_at(corner());
        let highest = (0..WIDTH)
            .flat_map(|i| (0..DEPTH).map(move |k| (i, k)))
            .map(|(i, k)| hill((ox + i) as f32 + 0.5, (oz + k) as f32 + 0.5).floor() as i32)
            .max()
            .unwrap();
        for (at, v) in &plan {
            if v.is_shaped() || v.material() == LANTERN {
                assert!(at[1] > highest, "{at:?} is buried at ground {highest}");
            }
        }
    }

    /// ADR-0003: where it stands is a metre position, so halving the Voxel size
    /// puts its corner at the same place, not at the same Voxel index.
    #[test]
    fn its_corner_is_a_metre_position_not_a_voxel_index() {
        for voxel_size in [1.0f32, 0.5, 0.25] {
            let w = world(voxel_size);
            let plan = plan(&w, corner(), hill);
            let min_x = plan.iter().map(|(at, _)| at[0]).min().unwrap();
            let min_z = plan.iter().map(|(at, _)| at[2]).min().unwrap();
            assert!((min_x as f32 * voxel_size - corner().x).abs() <= voxel_size);
            assert!((min_z as f32 * voxel_size - corner().z).abs() <= voxel_size);
        }
    }
}
