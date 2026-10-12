//! The Server: the sole authority over World state.
//!
//! Runs in the Client's process — an integrated server, not a different Server.
//! Its tick is on `FixedUpdate` because that is what the network transport in
//! build-order step 10 will drive, and because `RunFixedMainLoop` runs before
//! `Update`, so a message the Server writes reaches the Client in the same
//! frame. The reverse direction can be dropped by a frame that runs no fixed
//! tick, which is a second reason the Client only ever receives.
//!
//! See `docs/adr/0002-client-server-from-day-one.md`.

#[cfg(feature = "dev")]
mod builder;
pub mod edit;
#[cfg(feature = "dev")]
mod gallery;
mod jobs;
pub mod streaming;
pub mod world_state;

use std::sync::Arc;

use bevy::prelude::*;
use tesserae_core::{ShapeTable, Worldgen, WorldgenParams};

use crate::world::WorldSettings;
use world_state::SpawnPoint;

/// Metres of clearance above the surface. In metres, not Voxels: a clearance of
/// "two Voxels" is calibrated to one Voxel size, which is the mistake
/// `docs/adr/0003-voxel-size-is-per-world.md` exists to prevent.
const SPAWN_CLEARANCE_M: f32 = 2.0;

/// The World's generator. A newtype because `Worldgen` lives in
/// `tesserae-core`, which must acquire no `bevy` edge even transitively.
#[derive(Resource, Deref)]
pub struct WorldGenerator(pub Worldgen);

/// The World's Shape table, read by everything that has to know what a Voxel
/// covers: the mesher, both light solvers, collision and picking. A newtype for
/// the same reason as [`WorldGenerator`].
///
/// Server-only. The Client is sent finished meshes and needs no Shape to draw
/// one, and a Mod registers its Shapes here at load. Behind an `Arc` so a
/// streaming job can hold the table; it is not written once the World is open.
#[derive(Resource, Deref, Default)]
pub struct Shapes(pub Arc<ShapeTable>);

pub fn plugin(app: &mut App) {
    app.init_resource::<world_state::ChunkStore>()
        .init_resource::<Shapes>()
        .add_systems(Startup, open_world)
        .add_plugins((streaming::plugin, edit::plugin));
    #[cfg(feature = "dev")]
    app.add_plugins(gallery::plugin);
}

/// What the World is generated with. Under `dev`, `TESSERAE_CAVES=0` turns the
/// caves off, so a streaming change can be measured with and without them on one
/// binary and not across two builds.
#[cfg(feature = "dev")]
fn worldgen_params() -> WorldgenParams {
    let params = WorldgenParams::default();
    if std::env::var("TESSERAE_CAVES").is_ok_and(|v| v == "0") {
        return WorldgenParams {
            caves: None,
            ..params
        };
    }
    params
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
    let surface_m = generator.surface_height_m(0.0, 0.0);
    commands.insert_resource(SpawnPoint::new(Vec3::new(
        0.0,
        surface_m + SPAWN_CLEARANCE_M,
        0.0,
    )));
    commands.insert_resource(WorldGenerator(generator));
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
    fn the_spawn_point_barely_moves_when_voxel_size_changes() {
        // The clearance is in metres and the surface is in metres, so halving
        // Voxel size must not move the player. A clearance in Voxels would.
        let (coarse, _) = spawn_point_at(1.0);
        let (fine, _) = spawn_point_at(0.25);
        assert_eq!(coarse, fine);
    }
}
