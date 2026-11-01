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
    if kind == "cave" {
        app.add_session_systems(FixedUpdate, into_cave.before(super::streaming::Streaming));
    }
    if kind == "hidden-hill" {
        app.add_session_systems(FixedUpdate, into_sky.before(super::streaming::Streaming));
    }
}

const BUILT: [&str; 3] = ["transparency", "active", "hidden-hill"];
const MOVED: [&str; 9] = [
    "stationary",
    "flight",
    "sprint-flight",
    "sprint-walk",
    "dive",
    "teleport",
    "cave",
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
    if BUILT.contains(&kind.as_str()) {
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

/// How far from spawn the cave fixture looks for a cave.
const CAVE_SEARCH_M: f32 = 128.0;
/// Ground over the Eye, so it is in a cave and not a cave's mouth.
const CAVE_COVER_M: f32 = 8.0;
/// Rock farther than this along an axis counts as this far: past a tunnel's
/// radius, more room says nothing more.
const CAVE_ROOM_M: f32 = 4.0;

/// Moves the Player into a cave near spawn on the first tick, before anything
/// streams around spawn.
fn into_cave(
    generator: Res<super::WorldGenerator>,
    mut player: Single<&mut super::player::Player>,
    mut done: Local<bool>,
) {
    if std::mem::replace(&mut *done, true) {
        return;
    }
    let from = player.eye();
    let to = cave_eye(&generator.0, from).expect("Cave fixture requires a cave near spawn");
    player.set_eye(to);
    info!("profile_fixture cave from={from} to={to}");
}

/// The middle of a cave near `near_m`: among open Voxels with
/// [`CAVE_COVER_M`] of ground over them, in the Chunk that holds the most of
/// them, the one farthest from rock along the axes. Deterministic for a seed.
fn cave_eye(g: &tesserae_core::Worldgen, near_m: Vec3) -> Option<Vec3> {
    use tesserae_core::CHUNK_SIZE;
    let size = CHUNK_SIZE as i32;
    let voxel_m = g.voxel_size();
    let cell_m = CHUNK_SIZE as f32 * voxel_m;
    let floor_m = g.params().caves?.floor_m;
    let reach = (CAVE_SEARCH_M / cell_m).ceil() as i32;
    let (cx0, cz0) = (
        (near_m.x / cell_m).floor() as i32,
        (near_m.z / cell_m).floor() as i32,
    );
    let (_, highest_m) = g.surface_bounds_m();
    let (lowest_cy, highest_cy) = (
        (floor_m / cell_m).floor() as i32,
        ((highest_m - CAVE_COVER_M) / cell_m).floor() as i32,
    );
    let index = |c: i32, i: usize| (c * size + i as i32) as f32 * voxel_m;
    let mut best: Option<(usize, [i32; 3], [usize; 3])> = None;
    for cz in cz0 - reach..=cz0 + reach {
        for cx in cx0 - reach..=cx0 + reach {
            let ground: Vec<f32> = (0..CHUNK_SIZE * CHUNK_SIZE)
                .map(|i| {
                    let (x, z) = (i % CHUNK_SIZE, i / CHUNK_SIZE);
                    g.surface_height_m(index(cx, x) + voxel_m / 2.0, index(cz, z) + voxel_m / 2.0)
                })
                .collect();
            for cy in lowest_cy..=highest_cy {
                let chunk = g.chunk(cx, cy, cz);
                if chunk.uniform().is_some() {
                    continue;
                }
                let mut covered = Vec::new();
                for z in 0..CHUNK_SIZE {
                    for y in 0..CHUNK_SIZE {
                        for x in 0..CHUNK_SIZE {
                            let below_m = ground[z * CHUNK_SIZE + x] - index(cy, y);
                            if below_m >= CAVE_COVER_M && chunk.get(x, y, z).is_air() {
                                covered.push([x, y, z]);
                            }
                        }
                    }
                }
                if best.as_ref().is_none_or(|b| covered.len() > b.0) {
                    let room = |[x, y, z]: [usize; 3]| {
                        let most = (CAVE_ROOM_M / voxel_m) as i32;
                        (0..6)
                            .map(|face| {
                                let d = tesserae_core::mesh::FACE_STEPS[face];
                                (1..=most)
                                    .take_while(|k| {
                                        let at = [
                                            x as i32 + d[0] * k,
                                            y as i32 + d[1] * k,
                                            z as i32 + d[2] * k,
                                        ];
                                        at.iter().all(|&a| (0..size).contains(&a))
                                            && chunk
                                                .get(at[0] as usize, at[1] as usize, at[2] as usize)
                                                .is_air()
                                    })
                                    .count()
                            })
                            .min()
                            .unwrap_or(0)
                    };
                    if let Some(roomiest) = covered.iter().copied().max_by_key(|&v| room(v)) {
                        best = Some((covered.len(), [cx, cy, cz], roomiest));
                    }
                }
            }
        }
    }
    let (_, [cx, cy, cz], [x, y, z]) = best?;
    Some(Vec3::new(index(cx, x), index(cy, y), index(cz, z)) + voxel_m / 2.0)
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

/// Where the hidden-hill fixture stands the Eye in its base Chunk, in metres
/// from the Chunk's corner: farther than the sight walk's margin from every
/// face, so the walk starts from this Chunk alone.
const HILL_EYE_M: Vec3 = Vec3::new(16.0, 6.0, 16.0);

/// The hidden-hill fixture's base Chunk, in Voxels: over spawn, two Cells
/// above the highest ground, so nothing but its own geometry shadows it.
fn sky_base(world: &WorldSettings, generator: &tesserae_core::Worldgen, spawn_m: Vec3) -> IVec3 {
    let cell_m = world.cell_size();
    let (_, highest_m) = generator.surface_bounds_m();
    let chunk = IVec3::new(
        (spawn_m.x / cell_m).floor() as i32,
        (highest_m / cell_m).ceil() as i32 + 2,
        (spawn_m.z / cell_m).floor() as i32,
    );
    chunk * tesserae_core::CHUNK_SIZE as i32
}

/// Moves the Player to the hidden-hill fixture's Eye on the first tick.
fn into_sky(
    generator: Res<super::WorldGenerator>,
    world: Res<WorldSettings>,
    spawn: Res<SpawnPoint>,
    mut player: Single<&mut super::player::Player>,
    mut done: Local<bool>,
) {
    if std::mem::replace(&mut *done, true) {
        return;
    }
    let base = sky_base(&world, &generator.0, spawn.metres());
    let to = base.as_vec3() * world.voxel_size + HILL_EYE_M;
    player.set_eye(to);
    info!("profile_fixture hidden-hill eye={to}");
}

/// A ridge, a hill behind it and a floor in view, in Voxels from the base
/// Chunk `b` the Eye stands in, at 1 m Voxels:
///
/// - the ridge fills the plane x = 62 of the Chunk east of the Eye's, so no
///   walk from the Eye enters the Chunk east of that one, which holds the
///   hill (x 66-74, y 4-12, z 8-16) and is culled;
/// - the floor (y = -7) lies under the hill's Chunk, reached by walking down
///   and east, and seen from the Eye under the ridge's lower edge;
/// - the sun, from +x, +y, +z, puts the hill's shadow on the floor at x 64-70.
fn hidden_hill_plan(b: IVec3, stone: Voxel) -> Vec<([i32; 3], Voxel)> {
    let mut plan = Vec::new();
    let mut put = |x: i32, y: i32, z: i32| plan.push(((b + IVec3::new(x, y, z)).to_array(), stone));
    for y in 0..32 {
        for z in 0..32 {
            put(62, y, z);
        }
    }
    for x in 64..96 {
        for z in 0..32 {
            put(x, -7, z);
        }
    }
    for x in 66..74 {
        for y in 4..12 {
            for z in 8..16 {
                put(x, y, z);
            }
        }
    }
    plan
}

fn ignition_round(seconds: f64) -> Option<u64> {
    (20.0..32.0)
        .contains(&seconds)
        .then(|| ((seconds - 20.0) / 6.0) as u64)
}

fn build(
    (mut fixture, mut status): (ResMut<Fixture>, ResMut<ProfileFixtureStatus>),
    start: Res<crate::profiling::ProcessStart>,
    (world, generator): (Res<WorldSettings>, Res<super::WorldGenerator>),
    spawn: Res<SpawnPoint>,
    tables: Res<ContentTables>,
    mut store: ResMut<ChunkStore>,
    mut meshes: ResMut<MeshState>,
) {
    if fixture.plan.is_empty() && fixture.kind == "hidden-hill" {
        let stone = (8..tables.materials.len())
            .map(|id| Voxel::cube(id as u16))
            .find(|&v| tables.materials.solid(v) && tables.materials.is_opaque(v))
            .expect("Fixture requires a declared opaque solid");
        let base = sky_base(&world, &generator.0, spawn.metres());
        fixture.plan = hidden_hill_plan(base, stone);
    }
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
    fn the_hidden_hill_is_culled_and_shadows_floor_the_eye_sees() {
        use tesserae_core::visibility::{Bounds, connectivity, sight};
        let tables = crate::server::fixture_tables();
        let stone = Voxel::cube(tesserae_core::testgen::fixtures().ground.rock);
        let plan = hidden_hill_plan(IVec3::ZERO, stone);
        let size = tesserae_core::CHUNK_SIZE as i32;
        let mut chunks: std::collections::HashMap<IVec3, tesserae_core::Chunk> = default();
        for &(at, voxel) in &plan {
            let at = IVec3::from_array(at);
            let i = at.rem_euclid(IVec3::splat(size)).as_uvec3();
            chunks
                .entry(at.div_euclid(IVec3::splat(size)))
                .or_insert_with(tesserae_core::Chunk::air)
                .set(i.x as usize, i.y as usize, i.z as usize, voxel);
        }
        let of = |c: [i32; 3]| {
            chunks
                .get(&IVec3::from_array(c))
                .map_or(tesserae_core::visibility::Connectivity::ALL, |chunk| {
                    connectivity(chunk, &tables.materials)
                })
        };
        let walked = sight([0; 3], Bounds::around([0; 3], [4, 4, 4]), of);
        assert!(!walked.contains([2, 0, 0]), "the hill's Chunk is culled");
        assert!(walked.contains([2, -1, 0]), "the floor's Chunk is seen");

        // A floor point the Eye sees under the ridge, whose sun ray meets the
        // hill and not the ridge.
        let solid: std::collections::HashSet<[i32; 3]> = plan.iter().map(|&(at, _)| at).collect();
        let hits = |from: Vec3, to: Vec3, skip: &dyn Fn([i32; 3]) -> bool| {
            (1..2000).any(|k| {
                let p = from + (to - from) * (k as f32 / 2000.0);
                let v = p.floor().as_ivec3().to_array();
                solid.contains(&v) && !skip(v)
            })
        };
        let sun = Vec3::new(1.0, 2.2, 0.8).normalize();
        let hill = |v: [i32; 3]| (66..74).contains(&v[0]) && (4..12).contains(&v[1]);
        let shadowed_in_view = (64..96)
            .flat_map(|x| (0..32).map(move |z| (x, z)))
            .any(|(x, z)| {
                let floor = Vec3::new(x as f32 + 0.5, -6.0, z as f32 + 0.5);
                let seen = !hits(HILL_EYE_M, floor + Vec3::Y * 0.01, &|v| v[1] == -7);
                let towards_sun = floor + sun * 40.0;
                let by_hill = hits(floor + Vec3::Y * 0.01, towards_sun, &|v| !hill(v));
                let by_other = hits(floor + Vec3::Y * 0.01, towards_sun, &hill);
                seen && by_hill && !by_other
            });
        assert!(shadowed_in_view);
    }

    #[test]
    fn the_cave_eye_is_in_open_air_under_cover_with_room_to_turn() {
        let g = tesserae_core::Worldgen::new(7, 1.0, tesserae_core::testgen::world_params());
        let eye = cave_eye(&g, Vec3::ZERO).expect("a cave near the origin");
        let size = tesserae_core::CHUNK_SIZE as i32;
        let open = |p: Vec3| {
            let v = p.floor().as_ivec3();
            let c = v.div_euclid(IVec3::splat(size));
            let i = v.rem_euclid(IVec3::splat(size)).as_uvec3();
            g.chunk(c.x, c.y, c.z)
                .get(i.x as usize, i.y as usize, i.z as usize)
                .is_air()
        };
        assert!(open(eye));
        assert!(g.surface_height_m(eye.x, eye.z) - eye.y >= CAVE_COVER_M);
        for d in [
            Vec3::X,
            Vec3::NEG_X,
            Vec3::Y,
            Vec3::NEG_Y,
            Vec3::Z,
            Vec3::NEG_Z,
        ] {
            assert!(open(eye + d * 2.0), "rock within 2 m along {d}");
        }
        assert!(eye.xz().length() <= CAVE_SEARCH_M + 32.0 * 1.5);
    }
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
