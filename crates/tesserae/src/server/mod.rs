//! The Server: the sole authority over World state.
//!
//! Runs in the Client's process — an integrated server, not a different Server.
//! Its tick is on `FixedUpdate` because that is what the network transport in
//! build-order step 10 will drive, and because `RunFixedMainLoop` runs before
//! `Update`, so a message the Server writes reaches the Client in the same
//! frame. The reverse direction can be dropped by a frame that runs no fixed
//! tick, which is a second reason the Client only ever receives.
//!
//! Where a Player is, is the Server's too: the Client sends movement intent and the
//! Server streams the World around, and checks Edits against, its own `Player`.
//!
//! See `docs/adr/0002-client-server-from-day-one.md`.

#[cfg(any(feature = "dev", feature = "profiling"))]
mod builder;
pub mod edit;
mod fluids;
#[cfg(feature = "profiling")]
mod profile_fixture;
#[cfg(feature = "profiling")]
pub use fluids::SimulationProfile;
#[cfg(feature = "profiling")]
pub use profile_fixture::ProfileFixtureStatus;
#[cfg(feature = "dev")]
mod gallery;
mod jobs;
mod lod;
mod lod_rings;
pub mod mods;
pub mod persistence;
pub mod player;
pub mod streaming;
pub mod tool;
pub mod undo;
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
    fn default() -> Self {
        Self::from_loaded(Tables::builtin(), false)
    }
}

impl ContentTables {
    fn from_loaded(tables: Tables, _game: bool) -> Self {
        ContentTables(Arc::new(tables))
    }
}

pub fn plugin(app: &mut App) {
    #[cfg(feature = "profiling")]
    app.add_plugins(profile_fixture::plugin);
    app.add_plugins(mods::plugin)
        .init_resource::<world_state::ChunkStore>()
        .init_resource::<ContentTables>()
        .add_systems(Startup, open_world)
        .add_plugins((
            streaming::plugin,
            edit::plugin,
            tool::plugin,
            fluids::plugin,
            player::plugin,
        ));
    app.add_plugins(persistence::plugin);
    #[cfg(feature = "dev")]
    if std::env::var_os("TESSERAE_PROFILE_FIXTURE").is_none() {
        app.add_plugins(gallery::plugin);
    }
}

/// What the World is generated with. Under `dev`, `TESSERAE_CAVES=0` and
/// `TESSERAE_SMOOTHING=0` turn the caves and the slope-smoothing stairs off, so
/// a streaming change can be measured with and without them on one binary and
/// not across two builds. `TESSERAE_WATER_LEVEL_M` moves the lake to that
/// height, and `=off` takes it out.
#[cfg(feature = "dev")]
fn worldgen_params(params: Option<WorldgenParams>) -> WorldgenParams {
    dev_worldgen_params(params.unwrap_or_default(), |name| std::env::var(name).ok())
}

#[cfg(feature = "dev")]
fn dev_worldgen_params(
    params: WorldgenParams,
    env: impl Fn(&str) -> Option<String>,
) -> WorldgenParams {
    let off = |name: &str| env(name).is_some_and(|v| v == "0");
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
        water: match (env("TESSERAE_WATER_LEVEL_M").as_deref(), params.water) {
            (Some("off"), _) => None,
            (Some(_), None) => {
                warn!(
                    "TESSERAE_WATER_LEVEL_M: the Game names no lake fluid, so there is no lake to move"
                );
                None
            }
            (Some(v), Some(water)) => {
                Some(
                    v.parse()
                        .ok()
                        .map_or(water, |level_m| tesserae_core::WaterParams {
                            level_m,
                            ..water
                        }),
                )
            }
            (None, water) => water,
        },
        ..params
    }
}

#[cfg(not(feature = "dev"))]
fn worldgen_params(params: Option<WorldgenParams>) -> WorldgenParams {
    params.unwrap_or_default()
}

/// Builds the generator and picks the spawn point. One system so there is no
/// Startup ordering edge to maintain; its commands are applied before
/// `PostStartup`, where the Client reads the spawn point.
fn open_world(
    mut commands: Commands,
    world: Res<WorldSettings>,
    params: Option<Res<mods::WorldParameters>>,
    persistence: Option<Res<persistence::Persistence>>,
) {
    let params = params.map_or_else(|| worldgen_params(None), |params| params.0);
    if params.ground.is_none() {
        info!("World has no ground: no Game named rock, soil and surface");
    }
    let generator = Worldgen::new(world.seed, world.voxel_size, params);
    let mut spawn = SpawnPoint::new(spawn_eye(&generator));
    if let Some(eye) = persistence.and_then(|p| p.reopened_eye) {
        spawn = spawn.with_entry(eye);
    }
    commands.insert_resource(spawn);
    commands.insert_resource(WorldGenerator(generator));
}

/// Over the origin, clear of the ground and of any lake standing on it.
fn spawn_eye(generator: &Worldgen) -> Vec3 {
    let ground_m = generator.surface_height_m(0.0, 0.0);
    let top_m = generator
        .water()
        .map_or(ground_m, |w| w.level_m.max(ground_m));
    Vec3::new(0.0, top_m + SPAWN_CLEARANCE_M, 0.0)
}

#[cfg(test)]
pub(crate) fn fixture_tables() -> Tables {
    let mut tables = tesserae_core::testgen::fixtures().tables.clone();
    tables
        .shapes
        .register(tesserae_core::shape::examples::fence())
        .unwrap();
    tables
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "dev")]
    #[test]
    fn dev_overrides_apply_after_the_loaded_world_parameters() {
        let loaded = WorldgenParams {
            relief_m: 12.0,
            ..WorldgenParams::default()
        }
        .with_default_lake(LAKE);
        assert!(loaded.caves.is_some());
        let params = dev_worldgen_params(loaded, |name| match name {
            "TESSERAE_CAVES" | "TESSERAE_SMOOTHING" => Some("0".into()),
            "TESSERAE_WATER_LEVEL_M" => Some("27".into()),
            _ => None,
        });
        assert!(params.caves.is_none());
        assert!(params.smoothing.is_none());
        let water = params.water.unwrap();
        assert_eq!((water.level_m, water.material), (27.0, LAKE));
        assert_eq!(params.relief_m, 12.0);
        assert!(
            dev_worldgen_params(loaded, |_| Some("off".into()))
                .water
                .is_none()
        );
    }

    /// Any id: these parameters are never generated or validated.
    #[cfg(feature = "dev")]
    const LAKE: tesserae_core::MaterialId = 9;

    #[cfg(feature = "dev")]
    #[test]
    fn a_water_level_with_no_lake_fluid_changes_nothing() {
        let dry = WorldgenParams::default();
        let params = dev_worldgen_params(dry, |name| {
            (name == "TESSERAE_WATER_LEVEL_M").then(|| "27".into())
        });
        assert_eq!(params, dry);
    }

    #[test]
    fn a_world_with_no_game_is_dry() {
        assert_eq!(worldgen_params(None).water, None);
    }

    #[cfg(feature = "dev")]
    #[test]
    fn the_caves_environment_override_reaches_loaded_parameters() {
        const CHILD: &str = "TESSERAE_TEST_CAVES_CHILD";
        if std::env::var_os(CHILD).is_some() {
            let loaded = WorldgenParams {
                relief_m: 12.0,
                ..WorldgenParams::default()
            };
            let params = worldgen_params(Some(loaded));
            assert!(params.caves.is_none());
            assert_eq!(params.relief_m, 12.0);
            return;
        }
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "server::tests::the_caves_environment_override_reaches_loaded_parameters",
            ])
            .env(CHILD, "1")
            .env("TESSERAE_CAVES", "0")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
    }

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

    /// A lake of any fluid; spawning reads only its level.
    const LAVA: tesserae_core::MaterialId = 10;

    #[test]
    fn the_spawn_point_is_above_water_that_covers_the_ground_there() {
        let generator = Worldgen::new(
            WorldSettings::default().seed,
            1.0,
            WorldgenParams {
                water: Some(tesserae_core::WaterParams {
                    level_m: 70.0,
                    material: LAVA,
                }),
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
