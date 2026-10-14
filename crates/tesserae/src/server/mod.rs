//! The Server: the sole authority over World state.
//!
//! Runs in the Client's process — an integrated server, not a different Server.
//! Its tick is on `FixedUpdate` because that is what the network transport in
//! build-order step 10 will drive, and because `RunFixedMainLoop` runs before
//! `Update`, so a message the Server writes reaches the Client in the same
//! frame. The reverse direction can be dropped by a frame that runs no fixed
//! tick, which is a second reason the Client only ever receives.
//!
//! Where a Player is, is the Server's too: the Client reports a pose and the
//! Server streams the World around, and checks Edits against, its own `Player`.
//!
//! See `docs/adr/0002-client-server-from-day-one.md`.

#[cfg(feature = "dev")]
mod builder;
pub mod edit;
#[cfg(feature = "dev")]
mod gallery;
mod jobs;
mod lod;
mod lod_rings;
pub mod player;
pub mod streaming;
pub mod world_state;

use std::sync::Arc;

use bevy::prelude::*;
use tesserae_core::{Tables, Worldgen, WorldgenParams};

use crate::world::WorldSettings;
pub use lod_rings::{cuts, level_count};
use world_state::SpawnPoint;

/// Metres of clearance above the surface. In metres, not Voxels: a clearance of
/// "two Voxels" is calibrated to one Voxel size, which is the mistake
/// `docs/adr/0003-voxel-size-is-per-world.md` exists to prevent.
const SPAWN_CLEARANCE_M: f32 = 2.0;

/// The World's generator. A newtype because `Worldgen` lives in
/// `tesserae-core`, which must acquire no `bevy` edge even transitively.
#[derive(Resource, Deref)]
pub struct WorldGenerator(pub Worldgen);

/// The World's Shape and Material tables, read by everything that has to know
/// what a Voxel covers or is made of: the mesher, both light solvers, collision
/// and picking. A newtype for the same reason as [`WorldGenerator`].
///
/// Server-only. The Client is sent finished meshes and needs no table to draw
/// one, and a Mod registers its Shapes and Materials here at load. Behind an
/// `Arc` so a streaming job can hold the tables; they are not written once the
/// World is open.
#[derive(Resource, Deref)]
pub struct ContentTables(pub Arc<Tables>);

impl Default for ContentTables {
    /// The builtin Shapes and Materials, and under `dev` a fence registered as a
    /// Mod will, so the gallery and the hotbar have a connected Shape to show.
    fn default() -> Self {
        #[cfg_attr(not(feature = "dev"), allow(unused_mut))]
        let mut tables = Tables::builtin();
        #[cfg(feature = "dev")]
        assert_eq!(
            tables
                .shapes
                .register(tesserae_core::shape::examples::fence()),
            Ok(tesserae_core::shape::examples::FENCE_ID),
            "the demo fence is the first Shape after the builtins"
        );
        ContentTables(Arc::new(tables))
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<world_state::ChunkStore>()
        .init_resource::<ContentTables>()
        .add_systems(Startup, open_world)
        .add_plugins((streaming::plugin, edit::plugin, player::plugin));
    #[cfg(feature = "dev")]
    app.add_plugins(gallery::plugin);
}

/// What the World is generated with. Under `dev`, `TESSERAE_CAVES=0` and
/// `TESSERAE_SMOOTHING=0` turn the caves and the slope-smoothing stairs off, so
/// a streaming change can be measured with and without them on one binary and
/// not across two builds. `TESSERAE_WATER_LEVEL_M` moves the water to that
/// height, and `=off` takes it out.
#[cfg(feature = "dev")]
fn worldgen_params() -> WorldgenParams {
    let off = |name: &str| std::env::var(name).is_ok_and(|v| v == "0");
    let params = WorldgenParams::default();
    WorldgenParams {
        caves: if off("TESSERAE_CAVES") {
            None
        } else {
            params.caves
        },
        smoothing: if off("TESSERAE_SMOOTHING") {
            None
        } else {
            params.smoothing
        },
        water: match std::env::var("TESSERAE_WATER_LEVEL_M").as_deref() {
            Ok("off") => None,
            Ok(v) => v
                .parse()
                .ok()
                .map(|level_m| tesserae_core::WaterParams { level_m })
                .or(params.water),
            Err(_) => params.water,
        },
        ..params
    }
}

#[cfg(not(feature = "dev"))]
fn worldgen_params() -> WorldgenParams {
    WorldgenParams::default()
}

/// Builds the generator and picks the spawn point. One system so there is no
/// Startup ordering edge to maintain; its commands are applied before
/// `PostStartup`, where the Client reads the spawn point.
fn open_world(mut commands: Commands, world: Res<WorldSettings>) {
    let generator = Worldgen::new(world.seed, world.voxel_size, worldgen_params());
    commands.insert_resource(SpawnPoint::new(spawn_eye(&generator)));
    commands.insert_resource(WorldGenerator(generator));
}

/// Over the origin, clear of the ground and of any water standing on it.
fn spawn_eye(generator: &Worldgen) -> Vec3 {
    let ground_m = generator.surface_height_m(0.0, 0.0);
    let top_m = generator
        .water_level_m()
        .map_or(ground_m, |w| w.max(ground_m));
    Vec3::new(0.0, top_m + SPAWN_CLEARANCE_M, 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spawn_point_at(voxel_size: f32) -> (Vec3, Worldgen) {
        let settings = WorldSettings {
            voxel_size,
            ..default()
        };
        let mut app = App::new();
        app.insert_resource(settings)
            .add_systems(Startup, open_world);
        app.update();
        let spawn = app.world().resource::<SpawnPoint>().metres();
        let generator = Worldgen::new(
            settings.seed,
            settings.voxel_size,
            WorldgenParams::default(),
        );
        (spawn, generator)
    }

    #[test]
    fn the_spawn_point_is_above_the_surface_not_merely_above_zero() {
        let (spawn, generator) = spawn_point_at(1.0);
        let surface_m = generator.surface_height_m(spawn.x, spawn.z);
        assert!(spawn.y > surface_m, "{} m is inside the ground", spawn.y);
        assert_eq!(spawn.y, surface_m + SPAWN_CLEARANCE_M);
    }

    #[test]
    fn the_spawn_point_is_above_water_that_covers_the_ground_there() {
        let generator = Worldgen::new(
            WorldSettings::default().seed,
            1.0,
            WorldgenParams {
                water: Some(tesserae_core::WaterParams { level_m: 70.0 }),
                ..WorldgenParams::default()
            },
        );
        assert!(
            generator.surface_height_m(0.0, 0.0) < 70.0,
            "the fixture floods the spawn"
        );
        assert_eq!(spawn_eye(&generator).y, 70.0 + SPAWN_CLEARANCE_M);
    }

    #[test]
    fn the_spawn_point_barely_moves_when_voxel_size_changes() {
        // The clearance is in metres and the surface is in metres, so halving
        // Voxel size must not move the player. A clearance in Voxels would.
        let (coarse, _) = spawn_point_at(1.0);
        let (fine, _) = spawn_point_at(0.25);
        assert_eq!(coarse, fine);
    }
}
