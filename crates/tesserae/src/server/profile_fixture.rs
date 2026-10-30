//! Diagnostic geometry uses declared content and the Server's normal edit path.
use super::{
    ContentTables, builder, edit,
    streaming::MeshState,
    world_state::{ChunkStore, SpawnPoint},
};
use crate::world::WorldSettings;
use bevy::prelude::*;

use crate::session::SessionApp;
use tesserae_core::{Occupants, RenderClass, Voxel};

pub(super) fn plugin(app: &mut App) {
    let Ok(kind) = std::env::var("TESSERAE_PROFILE_FIXTURE") else {
        return;
    };
    assert!(
        BUILT.contains(&kind.as_str()) || MOVED.contains(&kind.as_str()),
        "Unknown profiling fixture"
    );
    app.on_session_start(install)
        .add_session_systems(FixedPostUpdate, build.run_if(resource_exists::<Fixture>));
    if kind == "teleport" {
        app.add_session_systems(FixedUpdate, teleport.before(super::streaming::Streaming));
    }
}

const BUILT: [&str; 2] = ["transparency", "active"];
const MOVED: [&str; 6] = [
    "stationary",
    "flight",
    "sprint-flight",
    "sprint-walk",
    "dive",
    "teleport",
];

fn install(world: &mut World) {
    let Ok(kind) = std::env::var("TESSERAE_PROFILE_FIXTURE") else {
        return;
    };
    world.insert_resource(ProfileFixtureStatus {
        ready: MOVED.contains(&kind.as_str()),
        ..default()
    });
    if kind == "transparency" || kind == "active" {
        world.insert_resource(Fixture {
            kind,
            plan: Vec::new(),
            progress: builder::Progress::default(),
            ignition: None,
        });
    }
}
#[derive(Resource)]
struct Fixture {
    kind: String,
    plan: Vec<([i32; 3], Voxel)>,
    progress: builder::Progress,
    ignition: Option<u64>,
}
#[derive(Resource, Default, Clone, Copy)]
pub struct ProfileFixtureStatus {
    pub ready: bool,
    pub ignitions: u64,
    pub fire_burning: bool,
}

/// When the teleport fixture jumps, at the start of the measured phase.
const TELEPORT_AT_S: f64 = 22.0;
/// How far it jumps: past every level-0 and level-1 ring, so the view refills.
const TELEPORT_M: f32 = 2000.0;

/// Moves the Player once, keeping the Eye's height above the generated ground.
fn teleport(
    start: Res<crate::profiling::ProcessStart>,
    generator: Res<super::WorldGenerator>,
    mut player: Single<&mut super::player::Player>,
    mut done: Local<bool>,
) {
    if *done || start.0.elapsed().as_secs_f64() < TELEPORT_AT_S {
        return;
    }
    *done = true;
    let eye = player.eye();
    let above_m = eye.y - generator.0.surface_height_m(eye.x, eye.z);
    let to = eye + Vec3::X * TELEPORT_M;
    let to = to.with_y(generator.0.surface_height_m(to.x, to.z) + above_m);
    player.set_eye(to);
    info!("profile_fixture teleport from={eye} to={to}");
}

fn ignition_round(seconds: f64) -> Option<u64> {
    (20.0..32.0)
        .contains(&seconds)
        .then(|| ((seconds - 20.0) / 6.0) as u64)
}

fn build(
    (mut fixture, mut status): (ResMut<Fixture>, ResMut<ProfileFixtureStatus>),
    start: Res<crate::profiling::ProcessStart>,
    world: Res<WorldSettings>,
    spawn: Res<SpawnPoint>,
    tables: Res<ContentTables>,
    mut store: ResMut<ChunkStore>,
    mut meshes: ResMut<MeshState>,
) {
    if fixture.plan.is_empty() {
        let material = (8..tables.materials.len())
            .map(|id| id as u16)
            .find(|&id| {
                if fixture.kind == "transparency" {
                    tables.materials.def(id).solid
                        && tables.materials.def(id).render == RenderClass::Blend
                } else {
                    tables.fluids.get(id).is_some()
                }
            })
            .expect("Fixture requires declared blended solid / fluid content");
        let at = world.voxel_at(spawn.metres() + Vec3::new(-6.0, 1.0, -24.0));
        for z in (0..12).step_by(2) {
            for x in 0..12 {
                for y in 0..6 {
                    fixture
                        .plan
                        .push(([at[0] + x, at[1] + y, at[2] - z], Voxel::cube(material)));
                }
            }
        }
    }
    let fixture = &mut *fixture;
    if !status.ready
        && builder::step(
            &fixture.plan,
            &mut fixture.progress,
            &mut store,
            &mut meshes,
            &tables,
        ) == builder::Step::Completed
    {
        status.ready = true;
        info!(
            "profile_fixture construction complete: {} cells",
            fixture.plan.len()
        );
    }
    let round = ignition_round(start.0.elapsed().as_secs_f64());
    if status.ready && fixture.kind == "active" {
        if let Some(round) = round.filter(|&round| fixture.ignition != Some(round)) {
            // A distinct cell preserves existing burn timers instead of resetting them.

            if let Some(fuel) = (8..tables.materials.len())
                .map(|id| id as u16)
                .find(|&id| tables.combustion.get(id).is_some())
            {
                let at = fixture.plan[0].0;
                let at = [at[0] - 2, at[1], at[2] - round as i32 * 3];
                let occupants = Occupants {
                    solid: Voxel::cube(fuel),
                    fluid: None,
                    burning: Some(tesserae_core::Burning {
                        fuel: tables.combustion.get(fuel).unwrap().fuel_steps(),
                    }),
                };
                edit::set_occupants(&mut store, &mut meshes, &tables, at, occupants)
                    .expect("Fixture fire placement");
                fixture.ignition = Some(round);
                status.ignitions += 1;
                info!("profile_fixture ignition round={round}");
            } else {
                panic!("Active fixture requires declared fuel");
            }
        }
        let at = fixture.plan[0].0;
        status.fire_burning = store
            .occupants(
                [
                    at[0] - 2,
                    at[1],
                    at[2] - fixture.ignition.unwrap_or(0) as i32 * 3,
                ],
                &tables.fluids,
            )
            .is_some_and(|here| here.burning.is_some());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fire_is_refreshed_inside_the_measurement_window() {
        assert_eq!(ignition_round(19.99), None);
        assert_eq!(ignition_round(20.0), Some(0));
        assert_eq!(ignition_round(25.99), Some(0));
        assert_eq!(ignition_round(26.0), Some(1));
        assert_eq!(ignition_round(31.99), Some(1));
        assert_eq!(ignition_round(32.0), None);
    }
}
