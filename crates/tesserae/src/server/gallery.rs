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
//! - a lantern, so block light falls on shaped faces;
//! - a fence line that turns a corner, ends at a wall of cubes and starts beside
//!   a stair, so the arms it grows towards each can be seen: rails to the next
//!   fence, a rail into the cube, and one into the stair's sealed back.
//!
//! **The pattern is in Voxels and the placement is in metres.** The point is the
//! shape of individual Voxels, so the layout is a count of them, as `testgen`'s
//! is; where it stands is a metre position ahead of the spawn point, converted
//! once.

use bevy::prelude::*;
use tesserae_core::material::{GLASS, LEAVES};
use tesserae_core::mesh::{NEG_Y, POS_X, POS_Y};
use tesserae_core::shape::examples::FENCE_ID;
use tesserae_core::worldgen::{DIRT, GRASS, LANTERN, STONE};
use tesserae_core::{MaterialId, Orientation, SHAPE_POST, SHAPE_SLAB, SHAPE_STAIR, Voxel};

use super::builder::{self, Progress, Step};
use super::mods::WorldParameters;
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

/// The translucent row's footprint, and how far left of the shapes it stands.
const ROW_WIDTH: i32 = 11;
const ROW_DEPTH: i32 = 12;
const ROW_ASIDE_M: f32 = ASIDE_M - ROW_WIDTH as f32 - 3.0;

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
    (tables, fence): (Res<ContentTables>, Res<crate::content::DevFence>),
    (world, params): (Res<WorldSettings>, Res<WorldParameters>),
    spawn: Option<Res<SpawnPoint>>,
) {
    let Some(spawn) = spawn else {
        return;
    };
    let corner = spawn.metres() + Vec3::new(ASIDE_M, 0.0, -AHEAD_M);
    let surface = |x, z| generator.surface_height_m(x, z);
    let mut plan = plan(&world, corner, surface);
    // A Game's own fence is its content, so with a Game the line is left out.
    plan.retain_mut(|(_, voxel)| {
        if voxel.shape() != FENCE_ID {
            return true;
        }
        fence.0.is_some_and(|fence| {
            *voxel = Voxel::new(voxel.material(), fence, voxel.orientation());
            true
        })
    });
    // `TESSERAE_TRANSLUCENT_ROW=0` leaves the row out, so a capture can hold
    // no blended Voxel and be compared with one from before the blended pass.
    // `TESSERAE_TRANSLUCENT_ROW_ASIDE_M` moves it, so the fixed camera can see
    // it from a diagonal.
    if !std::env::var("TESSERAE_TRANSLUCENT_ROW").is_ok_and(|v| v == "0") {
        let aside_m = std::env::var("TESSERAE_TRANSLUCENT_ROW_ASIDE_M")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(ROW_ASIDE_M);
        let row = spawn.metres() + Vec3::new(aside_m, 0.0, -AHEAD_M);
        let lake = params.0.water.map(|lake| lake.material);
        plan.extend(translucent_plan(&world, row, surface, lake));
    }
    if builder::step(&plan, &mut progress, &mut store, &mut meshes, &tables) == Step::Completed {
        info!("gallery: {} Voxels at {corner:?}", plan.len());
    }
}

fn shaped(material: MaterialId, shape: u8, up: usize, turn: u8) -> Voxel {
    Voxel::shaped(material, shape, Orientation::new(up, turn))
}

/// A plinth of stone raised to the highest ground under a `width` by `depth`
/// footprint whose near corner is at `corner` metres, so nothing standing on it
/// is buried however the terrain slopes. Returns the plinth's Voxels and the
/// lattice position of the footprint's corner on top of it.
fn plinth(
    world: &WorldSettings,
    corner: Vec3,
    surface_m: impl Fn(f32, f32) -> f32,
    width: i32,
    depth: i32,
) -> (Plan, [i32; 3]) {
    let size = world.voxel_size;
    let [ox, _, oz] = world.voxel_at(corner);

    let ground = |i: i32, k: i32| {
        let x = (ox + i) as f32 * size + size / 2.0;
        let z = (oz + k) as f32 * size + size / 2.0;
        (surface_m(x, z) / size).floor() as i32
    };
    let columns = (0..width).flat_map(|i| (0..depth).map(move |k| (i, k)));
    let highest = columns.clone().map(|(i, k)| ground(i, k)).max().unwrap();

    let mut plan = Plan::new();
    for (i, k) in columns {
        for y in ground(i, k)..=highest {
            plan.push(([ox + i, y, oz + k], Voxel::cube(STONE)));
        }
    }
    (plan, [ox, highest + 1, oz])
}

/// The Voxels to place, in lattice coordinates, given the metre position of the
/// gallery's near corner and the height of the ground at any point.
fn plan(world: &WorldSettings, corner: Vec3, surface_m: impl Fn(f32, f32) -> f32) -> Plan {
    let (mut plan, [ox, top, oz]) = plinth(world, corner, surface_m, WIDTH, DEPTH);

    // `at(x, j, k)`: j counts up from the plinth.
    let at = |i: i32, j: i32, k: i32| [ox + i, top + j, oz + k];
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

    // A fence from a stair's back along x to a wall of cubes, with a corner
    // turning towards the camera. Arms are not placed: each is grown by the
    // fence's own neighbours, which is the thing on show.
    let fence = Voxel::new(STONE, FENCE_ID, 0);
    put(4, 0, 5, shaped(STONE, SHAPE_STAIR, POS_Y, 0));
    for x in 4..=9 {
        put(x, 0, 6, fence);
    }
    put(9, 0, 7, fence);
    put(9, 0, 8, fence);
    put(10, 0, 6, Voxel::cube(STONE));
    put(10, 1, 6, Voxel::cube(STONE));

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

/// Glass, the lake's fluid and leaves, the first Materials the blended pass
/// draws: a pool with a glass house in front of it, so a wrong blend order has
/// something to show itself on, and a canopy on a trunk beside them. With no
/// lake fluid there is no pool.
fn translucent_plan(
    world: &WorldSettings,
    corner: Vec3,
    surface_m: impl Fn(f32, f32) -> f32,
    lake: Option<MaterialId>,
) -> Plan {
    let (mut plan, [ox, top, oz]) = plinth(world, corner, surface_m, ROW_WIDTH, ROW_DEPTH);
    let at = |i: i32, j: i32, k: i32| [ox + i, top + j, oz + k];
    let mut put = |i, j, k, voxel| plan.push((at(i, j, k), voxel));

    // A basin of stone with the fluid level with its rim, at the back.
    if let Some(fluid) = lake {
        for k in 0..=5 {
            for i in 0..=8 {
                let rim = i == 0 || i == 8 || k == 0 || k == 5;
                put(i, 0, k, Voxel::cube(if rim { STONE } else { fluid }));
            }
        }
    }

    // A closed house of glass, 5 wide and 4 deep, with a lantern inside.
    for k in 7..=10 {
        for i in 1..=5 {
            let wall = i == 1 || i == 5 || k == 7 || k == 10;
            for j in 0..=2 {
                if wall {
                    put(i, j, k, Voxel::cube(GLASS));
                }
            }
            put(i, 3, k, Voxel::cube(GLASS));
        }
    }
    put(3, 0, 8, Voxel::cube(LANTERN));

    // A trunk with a canopy, to the side.
    for j in 0..=2 {
        put(9, j, 9, Voxel::cube(DIRT));
    }
    for k in 8..=10 {
        for i in 8..=10 {
            for j in 3..=4 {
                put(i, j, k, Voxel::cube(LEAVES));
            }
        }
    }
    plan
}

#[cfg(test)]
pub(super) fn measurement_plan(world: &WorldSettings, generator: &tesserae_core::Worldgen) -> Plan {
    let spawn = super::spawn_eye(generator);
    let surface = |x, z| generator.surface_height_m(x, z);
    let mut result = plan(world, spawn + Vec3::new(ASIDE_M, 0.0, -AHEAD_M), surface);
    result.extend(translucent_plan(
        world,
        spawn + Vec3::new(ROW_ASIDE_M, 0.0, -AHEAD_M),
        surface,
        generator.water().map(|lake| lake.material),
    ));
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::world_state::VoxelCoord;
    use std::collections::{BTreeSet, HashMap};
    use tesserae_core::MAX_SHAPED_PER_CHUNK;

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
            BTreeSet::from([SHAPE_SLAB, SHAPE_STAIR, SHAPE_POST, FENCE_ID])
        );
        let stairs: BTreeSet<u8> = plan
            .iter()
            .filter(|(_, v)| v.shape() == SHAPE_STAIR)
            .map(|(_, v)| v.orientation())
            .collect();
        assert!(stairs.len() >= 4, "{stairs:?}");
    }

    #[test]
    fn its_fences_meet_a_fence_a_cube_and_a_stairs_sealed_back() {
        let plan = plan(&world(1.0), corner(), hill);
        let at: HashMap<[i32; 3], Voxel> = plan.iter().copied().collect();
        let table = ContentTables::default();
        let fences: Vec<[i32; 3]> = at
            .iter()
            .filter(|(_, v)| v.shape() == FENCE_ID)
            .map(|(&p, _)| p)
            .collect();
        assert!(fences.len() >= 8, "{}", fences.len());

        // For each kind of neighbour, some fence has one on a side it grows an arm to.
        let mut met = BTreeSet::new();
        for p in &fences {
            let around = tesserae_core::mesh::FACE_STEPS.map(|d| {
                at.get(&[p[0] + d[0], p[1] + d[1], p[2] + d[2]])
                    .copied()
                    .unwrap_or(Voxel::AIR)
            });
            let c = table.shapes.connections(at[p], around);
            for (_, across) in around.iter().enumerate().filter(|&(f, _)| c.has(f)) {
                met.insert(match across.shape() {
                    FENCE_ID => "fence",
                    SHAPE_STAIR => "stair",
                    _ => "cube",
                });
            }
        }
        assert_eq!(met, BTreeSet::from(["fence", "stair", "cube"]));
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
        let table = ContentTables::default();
        for (at, v) in plan(&world(1.0), corner(), hill) {
            assert!(!v.is_air(), "{at:?}");
            assert!(table.shapes.contains(v.shape()), "{at:?}");
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

    /// The fixture water as the Game's lake fluid, in tables that hold it.
    fn watered() -> (ContentTables, MaterialId) {
        let mut tables = tesserae_core::Tables::builtin();
        let water = tesserae_core::testgen::register_water(&mut tables);
        (ContentTables(std::sync::Arc::new(tables)), water)
    }

    fn translucent() -> Plan {
        translucent_plan(&world(1.0), corner(), hill, Some(watered().1))
    }

    fn lattice(plan: &Plan, material: MaterialId) -> Vec<[i32; 3]> {
        plan.iter()
            .filter(|(_, v)| v.material() == material)
            .map(|(at, _)| *at)
            .collect()
    }

    #[test]
    fn the_translucent_row_shows_glass_the_lake_fluid_and_leaves() {
        let plan = translucent();
        for material in [GLASS, watered().1, LEAVES] {
            assert!(!lattice(&plan, material).is_empty(), "Material {material}");
        }
    }

    /// Lava lakes make a lava pool; a World with no lake fluid has no pool and
    /// no basin, and keeps its glass house and canopy.
    #[test]
    fn the_pool_is_the_lake_fluid_and_absent_without_one() {
        const LAVA: MaterialId = 9;
        let lava = translucent_plan(&world(1.0), corner(), hill, Some(LAVA));
        assert_eq!(lattice(&lava, LAVA).len(), 7 * 4);
        let dry = translucent_plan(&world(1.0), corner(), hill, None);
        assert_eq!(dry.len(), translucent().len() - 9 * 6, "basin and pool");
        for material in [GLASS, LEAVES] {
            assert!(!lattice(&dry, material).is_empty(), "Material {material}");
        }
    }

    /// The case that would show a wrong blend order: one translucent surface
    /// seen through another, from where the camera starts.
    #[test]
    fn the_glass_house_stands_between_the_camera_and_the_pool() {
        let plan = translucent();
        let nearest_water = lattice(&plan, watered().1)
            .iter()
            .map(|at| at[2])
            .max()
            .unwrap();
        let glass: Vec<_> = lattice(&plan, GLASS);
        assert!(glass.iter().all(|at| at[2] > nearest_water), "{glass:?}");
    }

    #[test]
    fn every_voxel_in_the_translucent_row_is_a_registered_material() {
        let (table, _) = watered();
        for (at, v) in translucent() {
            assert!(!v.is_air(), "{at:?}");
            assert!(table.materials.contains(v.material()), "{at:?}");
            assert!(table.shapes.contains(v.shape()), "{at:?}");
        }
    }

    #[test]
    fn the_translucent_row_stands_beside_the_shapes_and_not_on_them() {
        let w = world(1.0);
        let shapes = plan(&w, corner(), hill);
        let row_corner = corner() + Vec3::new(ROW_ASIDE_M - ASIDE_M, 0.0, 0.0);
        let row = translucent_plan(&w, row_corner, hill, Some(watered().1));
        let x_max = |plan: &Plan| plan.iter().map(|(at, _)| at[0]).max().unwrap();
        let x_min = |plan: &Plan| plan.iter().map(|(at, _)| at[0]).min().unwrap();
        assert!(x_max(&row) < x_min(&shapes), "the two overlap in x");
        let [_, _, oz] = w.voxel_at(row_corner);
        for (at, _) in &row {
            assert!((oz..oz + ROW_DEPTH).contains(&at[2]), "{at:?} in z");
        }
    }

    #[test]
    fn the_translucent_things_stand_on_the_plinth_and_the_pool_holds_its_water() {
        let w = world(1.0);
        let plan = translucent();
        let [ox, _, oz] = w.voxel_at(corner());
        let highest = (0..ROW_WIDTH)
            .flat_map(|i| (0..ROW_DEPTH).map(move |k| (i, k)))
            .map(|(i, k)| hill((ox + i) as f32 + 0.5, (oz + k) as f32 + 0.5).floor() as i32)
            .max()
            .unwrap();
        for (at, v) in &plan {
            if v.material() != STONE {
                assert!(at[1] > highest, "{at:?} is buried at ground {highest}");
            }
        }
        // Water with nothing under or beside it would be a sheet in the air.
        let at: HashMap<[i32; 3], Voxel> = plan.iter().copied().collect();
        let water = watered().1;
        for [x, y, z] in lattice(&plan, water) {
            assert_eq!(at.get(&[x, y - 1, z]).map(|v| v.material()), Some(STONE));
            for [dx, dz] in [[1, 0], [-1, 0], [0, 1], [0, -1]] {
                let side = at.get(&[x + dx, y, z + dz]).map(|v| v.material());
                assert!(
                    side == Some(STONE) || side == Some(water),
                    "water at {:?} spills to {side:?}",
                    [x, y, z]
                );
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
