//! A small structure near the spawn point that makes both light channels
//! visible. `dev` only.
//!
//! Nothing in build-order step 4 is observable without it. `worldgen` is a
//! heightmap — no caves, no overhangs — so every visible face has an open
//! column above it and sky light is uniformly 15 everywhere. A flood-fill is
//! only *visible* where something encloses space, and enclosing space is
//! worldgen's job, not light's.
//!
//! It exercises both halves of the solver on the running app rather than only
//! in tests: placing the roof takes sky light away from the column under it,
//! which is the de-propagation pass, and the lantern adds block light, which is
//! the torch case DESIGN.md names.
//!
//! On the Server because it mutates World state, which rules out the Client
//! side of `docs/adr/0002-client-server-from-day-one.md` at any price. It is
//! not an edit *API* — that is build-order step 5 — but a `dev` system in the
//! same family as `streaming::stats`.

use bevy::prelude::*;
use tesserae_core::worldgen::{LANTERN, STONE};
use tesserae_core::{Voxel, relight};

use super::streaming::MeshState;
use super::world_state::{ChunkStore, LightLattice, SpawnPoint, VoxelCoord};
use super::{Shapes, WorldGenerator};
use crate::world::{ChunkCoord, WorldSettings};

/// Where the structure stands and how big it is, in **metres** — so it is the
/// same physical shelter at any Voxel size, per ADR-0003. A span in Voxels
/// would shrink to a doorstep at 0.25 m Voxels.
///
/// Placed ahead of the spawn point along -Z because that is where the Client
/// points its camera, so `cargo dev` opens looking at it. Far enough ahead that
/// the camera, which starts well above the spawn point and tipped down, looks
/// *into* the open side rather than down onto the roof.
const AHEAD_M: f32 = 46.0;
/// Clearance under the roof. Tall rather than head height, and the reason is
/// the camera: a low opening viewed from above is foreshortened into a sliver,
/// so the gradient the structure exists to show would be two Voxels of it.
const ROOF_CLEARANCE_M: f32 = 12.0;
/// Half the roof's edge, so the shelter spans `2 * this` metres. Deeper than
/// the 15 Voxels sky light carries, so the back really does reach zero.
const HALF_SPAN_M: f32 = 9.0;
/// Where the lantern sits along the shelter's depth, as a fraction of the way
/// from the open side to the back wall. Towards the back, where sky light has
/// run out and only block light is left.
const LANTERN_DEPTH: f32 = 0.7;

/// Voxels placed per tick.
///
/// The structure is a few hundred Voxels and each one re-floods light, so
/// placing them all in one tick costs more than a frame. A budget for the same
/// reason `StreamingBudget` is one — and the cost is this tool building a
/// building, not what a single edit costs: that is bounded by the 15 Voxels
/// light reaches.
const EDITS_PER_TICK: usize = 24;

pub(super) fn plugin(app: &mut App) {
    // `FixedPostUpdate`, so the tick's own streaming has already run and the
    // Chunks this needs are either there or not yet. Ordering through the
    // schedule rather than an edge onto `streaming`'s private system, which
    // would have to be made public for a `dev` feature's benefit.
    app.add_systems(FixedPostUpdate, build);
}

/// How far through the plan this has got.
#[derive(Default)]
struct Progress {
    placed: usize,
}

/// Places the structure a few Voxels a tick, starting once the Chunks it falls
/// in are loaded and their light has settled.
///
/// Waiting on the light is not politeness: an edit re-floods incrementally, and
/// the bulk solve `streaming::relight` runs for a Chunk that is still `unlit`
/// would land afterwards and overwrite that work.
fn build(
    mut store: ResMut<ChunkStore>,
    mut meshes: ResMut<MeshState>,
    mut progress: Local<Progress>,
    generator: Res<WorldGenerator>,
    shapes: Res<Shapes>,
    world: Res<WorldSettings>,
    spawn: Option<Res<SpawnPoint>>,
) {
    let Some(spawn) = spawn else {
        return;
    };

    let ahead = spawn.metres() - Vec3::Z * AHEAD_M;
    let surface_m = generator.surface_height_m(ahead.x, ahead.z);
    let ground = Vec3::new(ahead.x, surface_m, ahead.z);
    let plan = plan(&world, ground);
    if progress.placed >= plan.len() {
        return;
    }

    let touched: Vec<ChunkCoord> = plan
        .iter()
        .map(|(at, _)| VoxelCoord::of(*at).chunk)
        .collect();
    if touched
        .iter()
        .any(|&c| !store.contains(c) || meshes.is_unlit(c))
    {
        return;
    }

    let until = (progress.placed + EDITS_PER_TICK).min(plan.len());
    let mut lattice = LightLattice::new(&mut store);
    for (at, voxel) in &plan[progress.placed..until] {
        let was = lattice.store.light_at(*at);
        lattice.store.set_voxel(VoxelCoord::of(*at), *voxel);
        relight(&mut lattice, &shapes, *at, was);
    }
    let lit = std::mem::take(&mut lattice.touched);
    for coord in lit {
        meshes.edited(coord);
    }
    for (at, _) in &plan[progress.placed..until] {
        meshes.edited(VoxelCoord::of(*at).chunk);
    }
    progress.placed = until;
    if progress.placed == plan.len() {
        info!("lightbox: a shelter and a lantern at {ground:?}");
    }
}

/// The Voxels to place, in lattice coordinates: a roof, three walls, and a
/// lantern at the back of the floor.
///
/// The side facing the camera is left open on purpose, twice over. It is the
/// only way to see in, and a sealed box would be uniformly black inside — which
/// shows only that the solver can reach zero. Open on one side, sky light
/// enters at the mouth at full strength and loses a level a Voxel going in, so
/// the floor carries the gradient the flood-fill exists to produce, and the
/// lantern takes over where it runs out.
fn plan(world: &WorldSettings, ground: Vec3) -> Vec<([i32; 3], Voxel)> {
    let span = HALF_SPAN_M;
    let steps = (2.0 * span / world.voxel_size).round() as i32;
    let height = (ROOF_CLEARANCE_M / world.voxel_size).round() as i32;
    let at = |i: i32, j: i32, k: i32| {
        world.voxel_at(
            ground
                + Vec3::new(
                    -span + i as f32 * world.voxel_size,
                    j as f32 * world.voxel_size,
                    -span + k as f32 * world.voxel_size,
                ),
        )
    };
    let mut plan = Vec::new();

    for i in 0..=steps {
        for k in 0..=steps {
            plan.push((at(i, height, k), Voxel::cube(STONE)));
            // Left, right and back. `k == steps` is the near side, which faces
            // the camera and stays open.
            if i == 0 || i == steps || k == 0 {
                for j in 1..height {
                    plan.push((at(i, j, k), Voxel::cube(STONE)));
                }
            }
        }
    }

    // Towards the back, in the dark the three walls and the roof make.
    let depth = ((1.0 - LANTERN_DEPTH) * steps as f32).round() as i32;
    plan.push((at(steps / 2, 1, depth), Voxel::cube(LANTERN)));
    plan
}

#[cfg(test)]
mod tests {
    use super::*;
    use tesserae_core::light::MAX_LIGHT;

    fn world() -> WorldSettings {
        WorldSettings::default()
    }

    #[test]
    fn the_structure_is_the_same_physical_size_at_any_voxel_size() {
        let ground = Vec3::new(0.0, 40.0, 0.0);
        let extent = |voxel_size: f32| {
            let w = WorldSettings {
                voxel_size,
                ..default()
            };
            let plan = plan(&w, ground);
            let span = |axis: usize| {
                let lo = plan.iter().map(|(at, _)| at[axis]).min().unwrap();
                let hi = plan.iter().map(|(at, _)| at[axis]).max().unwrap();
                (hi - lo) as f32 * voxel_size
            };
            (span(0), span(1), span(2))
        };
        let (cx, cy, cz) = extent(1.0);
        let (fx, fy, fz) = extent(0.5);
        // A Voxel of slack at each end, from the floor division.
        assert!((cx - fx).abs() <= 1.0, "{cx} m vs {fx} m across");
        assert!((cy - fy).abs() <= 1.0, "{cy} m vs {fy} m tall");
        assert!((cz - fz).abs() <= 1.0, "{cz} m vs {fz} m deep");
    }

    #[test]
    fn the_roof_is_solid_and_the_lantern_stands_under_it() {
        let ground = Vec3::new(0.0, 40.0, 0.0);
        let w = world();
        let plan = plan(&w, ground);

        let lantern: Vec<[i32; 3]> = plan
            .iter()
            .filter(|(_, v)| v.material() == LANTERN)
            .map(|(at, _)| *at)
            .collect();
        assert_eq!(lantern.len(), 1, "one lantern");

        let roof_y = w.voxel_at(ground + Vec3::Y * ROOF_CLEARANCE_M)[1];
        let roof: Vec<[i32; 3]> = plan
            .iter()
            .filter(|(at, _)| at[1] == roof_y)
            .map(|(at, _)| *at)
            .collect();
        let span = (2.0 * HALF_SPAN_M / w.voxel_size).round() as usize + 1;
        assert_eq!(roof.len(), span * span, "the roof has a hole in it");
        // Directly above the lantern, which is what takes its sky light away.
        assert!(
            roof.iter()
                .any(|at| at[0] == lantern[0][0] && at[2] == lantern[0][2])
        );
    }

    /// The side facing the camera is the only way to see in, and it is also
    /// what keeps the interior a gradient rather than a uniform black.
    #[test]
    fn exactly_one_side_is_left_open() {
        let ground = Vec3::new(0.0, 40.0, 0.0);
        let w = world();
        let plan = plan(&w, ground);
        let roof_y = w.voxel_at(ground + Vec3::Y * ROOF_CLEARANCE_M)[1];

        let walls: Vec<[i32; 3]> = plan
            .iter()
            .filter(|(at, _)| at[1] < roof_y && at[1] > w.voxel_at(ground)[1])
            .map(|(at, _)| *at)
            .collect();
        let near_z = walls.iter().map(|at| at[2]).max().unwrap();
        let far_z = walls.iter().map(|at| at[2]).min().unwrap();
        let on = |z: i32| walls.iter().filter(|at| at[2] == z).count();
        assert!(on(far_z) > on(near_z), "the back wall is the open one");
        // Nothing on the near side but where the two side walls end: one
        // column each, from the ground to just under the roof.
        let height = (ROOF_CLEARANCE_M / w.voxel_size) as usize;
        assert_eq!(on(near_z), 2 * (height - 1), "the open side is walled");
    }

    /// Both channels at once, which is the point of the structure: the lantern
    /// emits and the roof takes the sky away from the same Voxel.
    #[test]
    fn the_structure_uses_both_channels() {
        let plan = plan(&world(), Vec3::new(0.0, 40.0, 0.0));
        assert!(
            plan.iter()
                .any(|(_, v)| tesserae_core::light::emission(*v) == MAX_LIGHT),
            "nothing emits block light"
        );
        assert!(
            plan.iter().any(|(_, v)| v.is_full_cube()),
            "nothing occludes the sky"
        );
    }
}
