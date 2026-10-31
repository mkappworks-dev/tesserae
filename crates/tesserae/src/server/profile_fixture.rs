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
    if PERSISTENT.contains(&kind.as_str()) {
        assert!(
            std::env::var_os("TESSERAE_EPHEMERAL").is_none()
                && std::env::var_os("TESSERAE_WORLD_DIR").is_some_and(|dir| std::fs::read_dir(dir)
                    .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)),
            "The {kind} fixture needs a TESSERAE_WORLD_DIR that does not exist yet"
        );
    }
    // App-lifetime, so the move count survives the restart fixture's reopen.
    app.init_resource::<ProfileFixtureStatus>()
        .on_session_start(install)
        .add_session_systems(FixedPostUpdate, build.run_if(resource_exists::<Fixture>));
    match kind.as_str() {
        "teleport" => {
            app.add_session_systems(FixedUpdate, teleport.before(super::streaming::Streaming));
        }
        "revisit" => {
            app.add_session_systems(FixedUpdate, revisit.before(super::streaming::Streaming));
        }
        "restart" => {
            app.add_session_systems(Update, restart).add_systems(
                Update,
                reopen.run_if(in_state(crate::session::AppState::List)),
            );
        }
        _ => {}
    }
}

const BUILT: [&str; 2] = ["transparency", "active"];
const MOVED: [&str; 8] = [
    "stationary",
    "flight",
    "sprint-flight",
    "sprint-walk",
    "dive",
    "teleport",
    "restart",
    "revisit",
];
/// Fixtures that need saves, so their eviction and reopening are a Persistent
/// World's.
const PERSISTENT: [&str; 2] = ["restart", "revisit"];

fn install(world: &mut World) {
    let Ok(kind) = std::env::var("TESSERAE_PROFILE_FIXTURE") else {
        return;
    };
    let moves = world
        .get_resource::<ProfileFixtureStatus>()
        .map_or(0, |status| status.moves);
    world.insert_resource(ProfileFixtureStatus {
        ready: MOVED.contains(&kind.as_str()),
        moves,
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
    /// Jumps or reopens the fixture has made, so a row tells which leg it is on.
    pub moves: u64,
}

/// When the teleport fixture jumps, at the start of the measured phase.
const TELEPORT_AT_S: f64 = 22.0;
/// How far it jumps: past every level-0 and level-1 ring, so the view refills.
const TELEPORT_M: f32 = 2000.0;

/// Moves the Player once.
fn teleport(
    start: Res<crate::profiling::ProcessStart>,
    generator: Res<super::WorldGenerator>,
    (mut player, mut status): (
        Single<&mut super::player::Player>,
        ResMut<ProfileFixtureStatus>,
    ),
    mut done: Local<bool>,
) {
    if *done || start.0.elapsed().as_secs_f64() < TELEPORT_AT_S {
        return;
    }
    *done = true;
    jump(&generator, &mut player, &mut status, TELEPORT_M);
}

/// Out as `teleport` does, then back to where it started.
fn revisit(
    start: Res<crate::profiling::ProcessStart>,
    (generator, stats): (
        Res<super::WorldGenerator>,
        Res<super::streaming::stats::StreamingStats>,
    ),
    (mut player, mut status): (
        Single<&mut super::player::Player>,
        ResMut<ProfileFixtureStatus>,
    ),
    mut trip: Local<Trip>,
) {
    match trip.step(start.0.elapsed().as_secs_f64(), stats.settled) {
        Some(Leg::Out) => jump(&generator, &mut player, &mut status, TELEPORT_M),
        Some(Leg::Back) => jump(&generator, &mut player, &mut status, -TELEPORT_M),
        None => {}
    }
}

/// Moves the Player along x, keeping the Eye's height above the generated ground.
fn jump(
    generator: &super::WorldGenerator,
    player: &mut super::player::Player,
    status: &mut ProfileFixtureStatus,
    x_m: f32,
) {
    let eye = player.eye();
    let above_m = eye.y - generator.0.surface_height_m(eye.x, eye.z);
    let to = eye + Vec3::X * x_m;
    let to = to.with_y(generator.0.surface_height_m(to.x, to.z) + above_m);
    player.set_eye(to);
    status.moves += 1;
    info!("profile_fixture teleport from={eye} to={to}");
}

/// Saves and leaves the World at the measured phase, through the same Save
/// and quit a player uses; [`reopen`] opens it again.
fn restart(
    start: Res<crate::profiling::ProcessStart>,
    mut switch: ResMut<crate::launch::Switch>,
    mut save: MessageWriter<super::persistence::SaveAction>,
    mut done: Local<bool>,
) {
    if *done || start.0.elapsed().as_secs_f64() < TELEPORT_AT_S {
        return;
    }
    *done = true;
    switch.0 = Some(crate::launch::Target::List);
    save.write(super::persistence::SaveAction::SaveQuit);
    info!("profile_fixture restart: saving and leaving");
}

/// Opens the saved World the way a launch does, from the environment, so
/// the reopen reads the save rather than the list's folder.
fn reopen(
    mut commands: Commands,
    mut status: ResMut<ProfileFixtureStatus>,
    mut next: ResMut<NextState<crate::session::AppState>>,
) {
    if status.moves > 0 {
        return;
    }
    status.moves += 1;
    commands.insert_resource(crate::open::OpenRequest {
        what: crate::open::What::Direct,
        name: None,
        from_list: false,
    });
    next.set(crate::session::AppState::Loading);
    info!("profile_fixture restart: reopening");
}

#[derive(Debug, PartialEq, Eq)]
enum Leg {
    Out,
    Back,
}

/// The revisit fixture's two jumps: out at the measured phase, back once the
/// far side has settled.
#[derive(Default)]
struct Trip {
    out: bool,
    /// Stats published before the jump still read settled, so the far side
    /// counts as settled only after it has been seen unsettled.
    unsettled: bool,
    back: bool,
}

impl Trip {
    fn step(&mut self, seconds: f64, settled: bool) -> Option<Leg> {
        if !self.out {
            self.out = seconds >= TELEPORT_AT_S;
            return self.out.then_some(Leg::Out);
        }
        if self.back {
            return None;
        }
        self.unsettled |= !settled;
        self.back = self.unsettled && settled;
        self.back.then_some(Leg::Back)
    }
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

    #[test]
    fn revisit_leaves_at_the_window_and_returns_once_the_far_side_settles() {
        let mut trip = Trip::default();
        assert_eq!(trip.step(21.99, true), None);
        assert_eq!(trip.step(22.0, true), Some(Leg::Out));
        // Stats published before the jump still read settled.
        assert_eq!(trip.step(22.02, true), None);
        assert_eq!(trip.step(22.04, false), None);
        assert_eq!(trip.step(23.1, true), Some(Leg::Back));
        assert_eq!(trip.step(23.2, false), None);
        assert_eq!(trip.step(24.5, true), None);
        assert_eq!(trip.step(40.0, true), None);
    }
}
