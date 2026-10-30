//! Fixed-step Server movement. Client messages contain intent, never a position.
use super::{
    ContentTables,
    world_state::{ChunkStore, SpawnPoint},
};
use crate::world::WorldSettings;
use bevy::prelude::*;
use tesserae_core::collide::{self, Aabb};
use tesserae_core::immersion::{FluidSurface, fluid_at};

pub const FLY_SPEED_CEILING_MPS: f32 = 64.0;
pub const EYE_HEIGHT: f32 = 1.62;
pub const CROUCH_EYE_HEIGHT: f32 = 1.02;
pub const BODY_HEIGHT_M: f32 = 1.8;
pub const CROUCH_BODY_HEIGHT_M: f32 = 1.2;
/// How far below the eye a fluid still counts, so a third-person boom that
/// dips under a surface the eye is above still reads the water.
pub const IMMERSION_REACH_M: f32 = 2.0;
/// Metres, not Voxels: a respawn lifts the same distance at any Voxel size.
const RESPAWN_LIFT_STEP_M: f32 = 0.25;
const RESPAWN_LIFT_MAX_M: f32 = 16.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MovementMode {
    #[default]
    Flight,
    Physics,
}

#[derive(Resource)]
pub struct MovementPolicy {
    pub flight: bool,
    pub physics: bool,
    pub initial: Option<MovementMode>,
}
impl Default for MovementPolicy {
    fn default() -> Self {
        Self {
            flight: true,
            physics: true,
            initial: None,
        }
    }
}

#[derive(Message, Clone, Copy, Debug, Default)]
pub struct MovementIntent {
    pub cancelled: bool,
    pub direction: Vec3,
    pub speed_mps: f32,
    pub sprint: bool,
    pub crouch: bool,
    pub jump: bool,
    pub yaw: f32,
    pub pitch: f32,
    pub switch_mode: bool,
}
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub enum MovementFeedback {
    Switched(MovementMode),
    Forbidden,
    Obstructed,
    RespawnBlocked,
}
impl MovementFeedback {
    pub fn message(self) -> &'static str {
        match self {
            Self::Switched(MovementMode::Physics) => "Physics mode",
            Self::Switched(MovementMode::Flight) => "Free flight mode",
            Self::Forbidden => "Movement mode forbidden by Server policy",
            Self::Obstructed => "Physics requires a clear body in loaded terrain",
            Self::RespawnBlocked => "Respawn blocked: no clear space over the spawn",
        }
    }
    pub fn refused(self) -> bool {
        !matches!(self, Self::Switched(_))
    }
}
#[derive(Message, Clone, Copy, Debug)]
pub struct PoseCorrection {
    pub mode: MovementMode,
    pub seq: u32,
    pub eye: Vec3,
    pub body: BodyState,
}
/// What the body did over the last step, for the Client to pose a figure by.
/// Presentation only: nothing here feeds back into movement.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BodyState {
    pub grounded: bool,
    pub horizontal_mps: f32,
    pub vertical_mps: f32,
    pub crouching: bool,
    pub sprinting: bool,
    pub immersion: Option<FluidSurface>,
}
#[derive(Message, Clone, Copy, Debug)]
pub struct RespawnIntent;
#[derive(Message)]
pub struct RestoreView(pub f32, pub f32);

#[derive(Component, Debug)]
pub struct Player {
    eye: Vec3,
    mode: MovementMode,
    velocity_y: f32,
    grounded: bool,
    crouching: bool,
    intent: MovementIntent,
    seq: u32,
    view: [f32; 2],
    /// Measured from the Eye's travel, so a body held against a wall stands.
    travel_mps: Vec3,
    immersion: Option<FluidSurface>,
}
impl Player {
    fn new(eye: Vec3, mode: MovementMode) -> Self {
        Self {
            eye,
            mode,
            velocity_y: 0.0,
            grounded: false,
            crouching: false,
            intent: default(),
            seq: 0,
            view: [0.0; 2],
            travel_mps: Vec3::ZERO,
            immersion: None,
        }
    }
    pub fn eye(&self) -> Vec3 {
        self.eye
    }
    pub fn saved_movement(&self) -> tesserae_core::save::MovementV1 {
        tesserae_core::save::MovementV1 {
            flight: self.mode == MovementMode::Flight,
            crouching: self.crouching,
            yaw: self.view[0],
            pitch: self.view[1],
        }
    }
    #[cfg(test)]
    pub(in crate::server) fn at(eye: Vec3) -> Self {
        Self::new(eye, MovementMode::Flight)
    }
    #[cfg(test)]
    pub(in crate::server) fn set_eye(&mut self, eye: Vec3) {
        self.eye = eye;
    }
    pub(super) fn from_saved(eye: Vec3, saved: tesserae_core::save::MovementV1) -> Self {
        let mut p = Self::new(
            eye,
            if saved.flight {
                MovementMode::Flight
            } else {
                MovementMode::Physics
            },
        );
        p.view = [saved.yaw, saved.pitch];
        p.crouching = saved.crouching;
        p
    }
    pub fn body_state(&self) -> BodyState {
        BodyState {
            grounded: self.mode == MovementMode::Physics && self.grounded,
            horizontal_mps: self.travel_mps.with_y(0.0).length(),
            vertical_mps: self.travel_mps.y,
            crouching: self.crouching,
            sprinting: self.intent.sprint && !self.crouching,
            immersion: self.immersion,
        }
    }
    fn eye_height(&self) -> f32 {
        if self.crouching {
            CROUCH_EYE_HEIGHT
        } else {
            EYE_HEIGHT
        }
    }
    fn body(&self) -> Aabb {
        let eye_height = self.eye_height();
        let height = if self.crouching {
            CROUCH_BODY_HEIGHT_M
        } else {
            BODY_HEIGHT_M
        };
        Aabb::new(
            (self.eye - Vec3::new(0.3, eye_height, 0.3)).to_array(),
            (self.eye + Vec3::new(0.3, height - eye_height, 0.3)).to_array(),
        )
    }
    fn update_stance(
        &mut self,
        store: &impl collide::VoxelVolume,
        tables: &tesserae_core::Tables,
        size: f32,
    ) {
        if self.intent.crouch == self.crouching {
            return;
        }
        let feet = self.eye - Vec3::Y * self.eye_height();
        let standing = Aabb::new(
            (feet - Vec3::new(0.3, 0.0, 0.3)).to_array(),
            (feet + Vec3::new(0.3, BODY_HEIGHT_M, 0.3)).to_array(),
        );
        if !self.intent.crouch && collide::overlaps(store, tables, size, standing) {
            return;
        }
        self.crouching = self.intent.crouch;
        self.eye = feet + Vec3::Y * self.eye_height();
    }
    fn advance(
        &mut self,
        store: &impl collide::VoxelVolume,
        tables: &tesserae_core::Tables,
        size: f32,
        dt: f32,
    ) {
        let before = self.eye;
        self.step(store, tables, size, dt);
        if dt > 0.0 {
            self.travel_mps = (self.eye - before) / dt;
        }
    }
    fn step(
        &mut self,
        store: &impl collide::VoxelVolume,
        tables: &tesserae_core::Tables,
        size: f32,
        dt: f32,
    ) {
        let mut direction = self.intent.direction;
        // Below unit length is a partly deflected stick; the Client sends whole
        // keyboard directions at unit length, so they keep full speed.
        let throttle = direction.length().min(1.0);
        if self.mode == MovementMode::Flight {
            self.eye += direction.clamp_length_max(1.0)
                * (self.intent.speed_mps.clamp(0.5, FLY_SPEED_CEILING_MPS)
                    * if self.intent.sprint { 5.0 } else { 1.0 })
                .min(FLY_SPEED_CEILING_MPS)
                * dt;
            self.intent.jump = false;
            return;
        }
        self.update_stance(store, tables, size);
        // Unknown terrain and newly placed solids freeze an embedded body; no depenetration teleport.
        if collide::overlaps(store, tables, size, self.body()) {
            self.velocity_y = 0.0;
            self.intent.jump = false;
            return;
        }
        direction.y = 0.0;
        if self.intent.jump && self.grounded {
            self.velocity_y = 8.0;
        }
        self.intent.jump = false;
        self.velocity_y = (self.velocity_y - 24.0 * dt).max(-50.0);
        let speed = if self.crouching {
            2.25
        } else if self.intent.sprint {
            8.0
        } else {
            4.5
        };
        let horizontal = direction.normalize_or_zero() * throttle * speed * dt;
        let body = self.body();
        let mut moved = collide::sweep(
            store,
            tables,
            size,
            body,
            [horizontal.x, self.velocity_y * dt, horizontal.z],
        );
        // Step only from ground, with a clear head and a supporting surface.
        if self.grounded && self.velocity_y <= 0.0 && (moved.blocked[0] || moved.blocked[2]) {
            let up = collide::sweep(store, tables, size, body, [0.0, 0.55, 0.0]);
            let across = collide::sweep(
                store,
                tables,
                size,
                up.aabb,
                [horizontal.x, 0.0, horizontal.z],
            );
            let down = collide::sweep(store, tables, size, across.aabb, [0.0, -0.55 - 0.01, 0.0]);
            let distance =
                |b: Aabb| (b.min[0] - body.min[0]).powi(2) + (b.min[2] - body.min[2]).powi(2);
            if !up.blocked[1] && down.blocked[1] && distance(down.aabb) > distance(moved.aabb) {
                moved.aabb = down.aabb;
                moved.blocked[1] = true;
            }
        }
        self.grounded = moved.blocked[1] && self.velocity_y <= 0.0;
        if moved.blocked[1] {
            self.velocity_y = 0.0;
        }
        self.eye = Vec3::from_array(moved.aabb.min) + Vec3::new(0.3, self.eye_height(), 0.3);
    }
}
#[derive(SystemSet, Hash, PartialEq, Eq, Clone, Debug)]
pub(super) struct PlayerInput;
pub(super) fn plugin(app: &mut App) {
    app.init_resource::<MovementPolicy>()
        .add_message::<RestoreView>()
        .add_message::<MovementIntent>()
        .add_message::<MovementFeedback>()
        .add_message::<PoseCorrection>()
        .add_message::<RespawnIntent>()
        .add_systems(PostStartup, spawn_player)
        .add_systems(
            RunFixedMainLoop,
            receive
                .in_set(PlayerInput)
                .in_set(RunFixedMainLoopSystems::BeforeFixedMainLoop),
        )
        .add_systems(FixedUpdate, simulate.before(super::streaming::Streaming))
        .add_systems(
            RunFixedMainLoop,
            publish.in_set(RunFixedMainLoopSystems::AfterFixedMainLoop),
        );
}
fn fixture_mode() -> Option<MovementMode> {
    if cfg!(any(feature = "dev", feature = "profiling"))
        && (std::env::var_os("TESSERAE_SCREENSHOT").is_some()
            || std::env::var_os("TESSERAE_PROFILE_FIXTURE").is_some())
    {
        Some(
            match std::env::var("TESSERAE_FIXTURE_MOVEMENT").as_deref() {
                Ok("physics") => MovementMode::Physics,
                Ok("flight") | Err(_) => MovementMode::Flight,
                Ok(other) => panic!("Unknown fixture movement mode: {other}"),
            },
        )
    } else {
        None
    }
}
fn spawn_player(
    mut commands: Commands,
    spawn: Res<SpawnPoint>,
    generator: Option<Res<super::WorldGenerator>>,
    policy: Res<MovementPolicy>,
    persistence: Option<Res<super::persistence::Persistence>>,
) {
    let mode = persistence
        .as_ref()
        .and_then(|p| p.reopened_mode)
        .or_else(fixture_mode)
        .or(policy.initial)
        .unwrap_or_else(|| {
            if std::env::var_os("TESSERAE_SCREENSHOT").is_none()
                && std::env::var_os("TESSERAE_PROFILE_FIXTURE").is_none()
                && generator.is_some_and(|g| g.0.params().ground.is_some())
            {
                MovementMode::Physics
            } else {
                MovementMode::Flight
            }
        });
    let mut player = Player::new(
        spawn.entry_eye_at_height(if mode == MovementMode::Physics {
            EYE_HEIGHT
        } else {
            super::world_state::SPAWN_EYE_ALTITUDE_M
        }),
        mode,
    );
    if let Some(view) = persistence.as_ref().and_then(|p| p.reopened_view) {
        player = Player::from_saved(spawn.entry_eye(), view);
        commands.write_message(RestoreView(view.yaw, view.pitch));
    }
    info!(
        "Player spawned: movement={:?}, eye={:?}, view=first_person, fixed_hz=64",
        player.mode, player.eye
    );
    commands.spawn(player);
}
/// First standing position over `spawn` whose body is clear, so a Voxel placed
/// at the spawn lifts the respawn rather than embedding it. `None` when the
/// column is solid or unloaded up to the cap.
fn clear_respawn_eye(
    store: &impl collide::VoxelVolume,
    tables: &tesserae_core::Tables,
    size: f32,
    spawn: Vec3,
) -> Option<Vec3> {
    (0..=(RESPAWN_LIFT_MAX_M / RESPAWN_LIFT_STEP_M) as u32).find_map(|step| {
        let eye = spawn + Vec3::Y * (step as f32 * RESPAWN_LIFT_STEP_M + EYE_HEIGHT);
        let feet = eye - Vec3::Y * EYE_HEIGHT;
        let body = Aabb::new(
            (feet - Vec3::new(0.3, 0.0, 0.3)).to_array(),
            (feet + Vec3::new(0.3, BODY_HEIGHT_M, 0.3)).to_array(),
        );
        (!collide::overlaps(store, tables, size, body)).then_some(eye)
    })
}
pub(super) fn receive(
    mut intents: MessageReader<MovementIntent>,
    mut respawn: MessageReader<RespawnIntent>,
    mut player: Single<&mut Player>,
    policy: Res<MovementPolicy>,
    spawn: Res<SpawnPoint>,
    (store, tables, world): (Res<ChunkStore>, Res<ContentTables>, Res<WorldSettings>),
    mut feedback: MessageWriter<MovementFeedback>,
) {
    for intent in intents.read() {
        if !intent.direction.is_finite()
            || !intent.speed_mps.is_finite()
            || !intent.yaw.is_finite()
            || !intent.pitch.is_finite()
        {
            player.intent = default();
            continue;
        }
        player.view = [
            intent.yaw,
            intent
                .pitch
                .clamp(-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2),
        ];
        if intent.cancelled {
            player.intent = default();
            continue;
        }
        let jump = player.intent.jump || intent.jump;
        player.intent = *intent;
        player.intent.jump = jump;
        if intent.switch_mode {
            let next = if player.mode == MovementMode::Flight {
                MovementMode::Physics
            } else {
                MovementMode::Flight
            };
            let allowed = if next == MovementMode::Flight {
                policy.flight
            } else {
                policy.physics
            };
            if !allowed {
                feedback.write(MovementFeedback::Forbidden);
            } else if next == MovementMode::Physics
                && collide::overlaps(&*store, &tables, world.voxel_size, player.body())
            {
                feedback.write(MovementFeedback::Obstructed);
            } else {
                player.mode = next;
                player.crouching = false;
                player.velocity_y = 0.0;
                player.grounded = false;
                feedback.write(MovementFeedback::Switched(next));
            }
        }
    }
    if respawn.read().next().is_some() {
        respawn.clear();
        if player.mode == MovementMode::Physics {
            let Some(eye) = clear_respawn_eye(&*store, &tables, world.voxel_size, spawn.metres())
            else {
                feedback.write(MovementFeedback::RespawnBlocked);
                return;
            };
            player.eye = eye;
        } else {
            player.eye = spawn.eye();
        }
        player.velocity_y = 0.0;
        player.intent = default();
        player.grounded = false;
        player.crouching = false;
    }
}
fn publish(mut player: Single<&mut Player>, mut poses: MessageWriter<PoseCorrection>) {
    player.seq = player.seq.wrapping_add(1);
    poses.write(PoseCorrection {
        mode: player.mode,
        seq: player.seq,
        eye: player.eye,
        body: player.body_state(),
    });
}
fn simulate(
    status: Option<Res<super::persistence::SaveStatus>>,
    policy: Res<MovementPolicy>,
    mut player: Single<&mut Player>,
    store: Res<ChunkStore>,
    tables: Res<ContentTables>,
    world: Res<WorldSettings>,
    time: Res<Time<Fixed>>,
) {
    player.travel_mps = Vec3::ZERO;
    if status.is_some_and(|s| s.modal()) {
        player.intent = default();
    } else if (player.mode == MovementMode::Flight && !policy.flight)
        || (player.mode == MovementMode::Physics && !policy.physics)
    {
        player.intent = default();
        player.velocity_y = 0.0;
    } else {
        player.advance(&*store, &tables, world.voxel_size, time.delta_secs());
    }
    // On every path: a paused or refused Player still stands in a fluid.
    let eye = player.eye.to_array();
    player.immersion = fluid_at(&*store, &tables, world.voxel_size, eye, IMMERSION_REACH_M);
}

#[cfg(test)]
mod tests {
    use super::*;
    use tesserae_core::collide::VoxelVolume;
    use tesserae_core::{Orientation, SHAPE_SLAB, Tables, Voxel};
    struct Scene {
        size: f32,
        wall: bool,
        ceiling: bool,
        gap: bool,
        slab: bool,
    }
    impl VoxelVolume for Scene {
        fn voxel(&self, at: [i32; 3]) -> Option<Voxel> {
            let [x, y, _z] = at.map(|a| a as f32 * self.size);
            if self.gap && x >= 3.0 {
                return None;
            }
            if y < 0.0 || self.wall && x >= 3.0 || self.ceiling && y >= 2.0 {
                Some(Voxel::cube(8))
            } else if self.slab && (2.0..3.0).contains(&x) && y == 0.0 {
                Some(Voxel::shaped(8, SHAPE_SLAB, Orientation::IDENTITY))
            } else {
                Some(Voxel::AIR)
            }
        }
    }
    fn tables() -> Tables {
        crate::server::fixture_tables()
    }
    fn scene(size: f32) -> Scene {
        Scene {
            size,
            wall: false,
            ceiling: false,
            gap: false,
            slab: false,
        }
    }
    fn player() -> Player {
        Player::new(Vec3::new(0.0, EYE_HEIGHT, 0.0), MovementMode::Physics)
    }
    fn steps(p: &mut Player, s: &Scene, n: usize) {
        let t = tables();
        for _ in 0..n {
            p.step(s, &t, s.size, 1.0 / 64.0);
        }
    }
    #[test]
    fn floors_walls_unknown_and_ceiling_stop_the_authoritative_body_at_all_sizes() {
        for size in [1.0, 0.5, 0.25] {
            let mut s = scene(size);
            let mut p = player();
            p.eye.y += 4.0;
            steps(&mut p, &s, 128);
            assert!((p.eye.y - EYE_HEIGHT).abs() < 0.001);
            for gap in [false, true] {
                p = player();
                s.wall = !gap;
                s.gap = gap;
                p.intent.direction = Vec3::X;
                steps(&mut p, &s, 128);
                assert!((p.eye.x - 2.7).abs() < 0.001, "{size}: {:?}", p.eye);
            }
            s = scene(size);
            s.ceiling = true;
            p = player();
            steps(&mut p, &s, 1);
            p.intent.jump = true;
            steps(&mut p, &s, 4);
            assert!(p.body().max[1] <= 2.0001);
            steps(&mut p, &s, 64);
            assert!(p.grounded);
        }
    }
    #[test]
    fn a_partly_deflected_stick_scales_speed_and_whole_directions_keep_full_speed() {
        for size in [1.0, 0.5, 0.25] {
            let s = scene(size);
            let travel = |mode, direction: Vec3| {
                let mut p = Player::new(Vec3::new(0.0, EYE_HEIGHT, 0.0), mode);
                steps(&mut p, &s, 1);
                let start = p.eye;
                p.intent.direction = direction;
                p.intent.speed_mps = 12.0;
                steps(&mut p, &s, 32);
                (p.eye - start).with_y(0.0).length()
            };
            for mode in [MovementMode::Physics, MovementMode::Flight] {
                let full = travel(mode, Vec3::X);
                assert!(full > 0.0);
                let diagonal = travel(mode, Vec3::new(1.0, 0.0, 1.0));
                assert!((diagonal - full).abs() < 1e-4, "{mode:?} {size}");
                let half = travel(mode, Vec3::X * 0.5);
                assert!(
                    (half / full - 0.5).abs() < 1e-3,
                    "{mode:?} {size}: {half}/{full}"
                );
                assert!((travel(mode, Vec3::X * 7.0) - full).abs() < 1e-4);
            }
            // Pitch must not slow walking: the Client's whole forward is unit length.
            let pitched = Vec3::new(0.5, -0.866, 0.0);
            let walk = travel(MovementMode::Physics, pitched);
            assert!((walk - travel(MovementMode::Physics, Vec3::X)).abs() < 1e-4);
        }
    }
    #[test]
    fn the_published_body_reports_travel_not_intent() {
        let t = tables();
        for size in [1.0, 0.5, 0.25] {
            let mut s = scene(size);
            let mut p = player();
            p.advance(&s, &t, size, 1.0 / 64.0);
            p.intent.direction = Vec3::X;
            p.intent.sprint = true;
            p.advance(&s, &t, size, 1.0 / 64.0);
            let body = p.body_state();
            assert!(body.grounded && body.sprinting && !body.crouching);
            assert!((body.horizontal_mps - 8.0).abs() < 0.01, "{body:?}");
            s.wall = true;
            for _ in 0..128 {
                p.advance(&s, &t, size, 1.0 / 64.0);
            }
            assert!(p.body_state().horizontal_mps < 0.01, "held at the wall");
            p.intent.jump = true;
            p.advance(&s, &t, size, 1.0 / 64.0);
            let body = p.body_state();
            assert!(!body.grounded && body.vertical_mps > 0.0, "{body:?}");
            p.mode = MovementMode::Flight;
            assert!(!p.body_state().grounded, "flight is never grounded");
        }
    }
    #[test]
    fn slabs_step_up_without_passing_a_low_ceiling() {
        let mut s = scene(1.0);
        s.slab = true;
        let mut p = player();
        steps(&mut p, &s, 1);
        p.intent.direction = Vec3::X;
        steps(&mut p, &s, 35);
        assert!(p.eye.x > 2.0);
        assert!((p.eye.y - EYE_HEIGHT - 0.5).abs() < 0.001);
        s.ceiling = true;
        p = player();
        steps(&mut p, &s, 1);
        p.intent.direction = Vec3::X;
        steps(&mut p, &s, 35);
        assert!(p.eye.x <= 1.7001);
    }
    #[test]
    fn stairs_are_climbed_by_the_body_and_jump_does_not_repeat() {
        struct Stair;
        impl VoxelVolume for Stair {
            fn voxel(&self, at: [i32; 3]) -> Option<Voxel> {
                Some(if at[1] < 0 {
                    Voxel::cube(8)
                } else if at == [0, 0, 2] {
                    Voxel::shaped(8, tesserae_core::SHAPE_STAIR, Orientation::IDENTITY)
                } else {
                    Voxel::AIR
                })
            }
        }
        let mut p = player();
        p.eye.x = 0.5;
        let t = tables();
        p.step(&Stair, &t, 1.0, 1.0 / 64.0);
        p.intent.direction = Vec3::Z;
        for _ in 0..38 {
            p.step(&Stair, &t, 1.0, 1.0 / 64.0);
        }
        assert!(p.eye.z > 2.5 && p.eye.y > EYE_HEIGHT + 0.9, "{:?}", p.eye);
        let s = scene(1.0);
        p = player();
        steps(&mut p, &s, 1);
        p.intent.jump = true;
        steps(&mut p, &s, 10);
        assert!(p.eye.y > EYE_HEIGHT + 0.5);
        steps(&mut p, &s, 128);
        assert!((p.eye.y - EYE_HEIGHT).abs() < 0.001);
        assert!(p.grounded);
    }

    #[test]
    fn crouch_preserves_feet_slows_sprint_and_waits_for_standing_clearance() {
        struct Tunnel(f32);
        impl VoxelVolume for Tunnel {
            fn voxel(&self, at: [i32; 3]) -> Option<Voxel> {
                let [x, y, _] = at.map(|a| a as f32 * self.0);
                Some(if y < 0.0 {
                    Voxel::cube(8)
                } else if (2.0..4.0).contains(&x) {
                    if self.0 == 1.0 && at[1] == 1 {
                        Voxel::shaped(
                            8,
                            tesserae_core::SHAPE_SLAB,
                            Orientation::new(tesserae_core::mesh::NEG_Y, 0),
                        )
                    } else if y >= 1.5 {
                        Voxel::cube(8)
                    } else {
                        Voxel::AIR
                    }
                } else {
                    Voxel::AIR
                })
            }
        }
        let tables = tables();
        for size in [1.0, 0.5, 0.25] {
            let terrain = Tunnel(size);
            let mut p = player();
            p.intent = MovementIntent {
                direction: Vec3::X,
                crouch: true,
                sprint: true,
                ..default()
            };
            for _ in 0..64 {
                p.step(&terrain, &tables, size, 1.0 / 64.0);
            }
            assert!((p.eye.x - 2.25).abs() < 0.001);
            assert!(p.crouching);
            assert!((p.body().min[1]).abs() < 0.001);
            assert!((p.eye.y - CROUCH_EYE_HEIGHT).abs() < 0.001);
            p = Player::from_saved(p.eye, p.saved_movement());
            p.step(&terrain, &tables, size, 1.0 / 64.0);
            assert!(p.crouching, "reopen under ceiling must retain stance");
            assert!((p.body().min[1]).abs() < 0.001);
            p.intent.direction = Vec3::X;
            for _ in 0..100 {
                p.step(&terrain, &tables, size, 1.0 / 64.0);
            }
            assert!(!p.crouching);
            assert!((p.eye.y - EYE_HEIGHT).abs() < 0.001);
        }
    }

    #[test]
    fn render_traces_do_not_buy_speed() {
        for physics in [false, true] {
            for size in [1.0, 0.5, 0.25] {
                let trace = |fps: u64| {
                    let mut app = App::new();
                    app.add_plugins(MinimalPlugins)
                        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                            std::time::Duration::from_secs_f64(1.0 / fps as f64),
                        ))
                        .insert_resource(WorldSettings {
                            voxel_size: size,
                            ..default()
                        })
                        .insert_resource(SpawnPoint::for_tests(Vec3::ZERO))
                        .init_resource::<ChunkStore>()
                        .insert_resource(ContentTables::default())
                        .add_plugins(plugin);
                    app.update();
                    let mut q = app.world_mut().query::<&mut Player>();
                    {
                        let mut p = q.single_mut(app.world_mut()).unwrap();
                        p.eye = Vec3::new(4.0, size + EYE_HEIGHT, 4.0);
                        p.mode = if physics {
                            MovementMode::Physics
                        } else {
                            MovementMode::Flight
                        };
                    }
                    let id = MovementIntent {
                        direction: Vec3::X,
                        speed_mps: 12.0,
                        ..default()
                    };
                    {
                        let mut store = app.world_mut().resource_mut::<ChunkStore>();
                        for x in 0..3 {
                            let mut chunk = tesserae_core::Chunk::air();
                            for z in 0..32 {
                                for xx in 0..32 {
                                    chunk.set(xx, 0, z, Voxel::cube(8));
                                }
                            }
                            store.insert(crate::world::ChunkCoord::new(x, 0, 0), chunk);
                        }
                    }
                    app.world_mut()
                        .insert_resource(ContentTables(std::sync::Arc::new(tables())));
                    app.world_mut().write_message(id);
                    for _ in 0..fps {
                        app.update();
                    }
                    q.single_mut(app.world_mut()).unwrap().eye
                };
                let expected = trace(60);
                assert!(
                    (expected.x - 4.0 - if physics { 4.5 } else { 12.0 }).abs() < 0.21,
                    "{expected:?}"
                );
                for fps in [144, 1000] {
                    assert!((trace(fps) - expected).length() < 0.21, "{physics} {fps}");
                }
            }
        }
    }
    #[test]
    fn embedded_entry_and_server_permissions_refuse_without_teleport() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(WorldSettings::default())
            .insert_resource(SpawnPoint::for_tests(Vec3::ZERO))
            .init_resource::<ChunkStore>()
            .insert_resource(ContentTables::default())
            .add_plugins(plugin);
        app.update();
        let eye = app
            .world_mut()
            .query::<&Player>()
            .single(app.world())
            .unwrap()
            .eye;
        app.world_mut().write_message(MovementIntent {
            switch_mode: true,
            ..default()
        });
        app.update();
        let p = app
            .world_mut()
            .query::<&Player>()
            .single(app.world())
            .unwrap();
        assert_eq!(p.mode, MovementMode::Flight);
        assert_eq!(p.eye, eye);
        assert_eq!(
            app.world_mut()
                .resource_mut::<Messages<MovementFeedback>>()
                .drain()
                .count(),
            1
        );
        app.world_mut().resource_mut::<MovementPolicy>().physics = false;
        app.world_mut().write_message(MovementIntent {
            switch_mode: true,
            ..default()
        });
        app.update();
        assert_eq!(
            app.world_mut()
                .resource_mut::<Messages<MovementFeedback>>()
                .drain()
                .next()
                .unwrap(),
            MovementFeedback::Forbidden
        );
    }
    #[test]
    fn noclip_speed_is_server_capped_and_mode_changes_never_move_the_eye() {
        let mut p = player();
        p.mode = MovementMode::Flight;
        p.intent = MovementIntent {
            direction: Vec3::X,
            speed_mps: f32::MAX,
            sprint: true,
            ..default()
        };
        steps(&mut p, &scene(1.0), 64);
        assert!((p.eye.x - 64.0).abs() < 0.001);
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(WorldSettings::default())
            .insert_resource(SpawnPoint::for_tests(Vec3::ZERO))
            .init_resource::<ChunkStore>()
            .insert_resource(ContentTables::default())
            .add_plugins(plugin);
        app.update();
        app.world_mut().resource_mut::<ChunkStore>().insert(
            crate::world::ChunkCoord::new(0, 0, 0),
            tesserae_core::Chunk::air(),
        );
        let eye = Vec3::splat(4.0);
        app.world_mut()
            .query::<&mut Player>()
            .single_mut(app.world_mut())
            .unwrap()
            .eye = eye;
        for mode in [MovementMode::Physics, MovementMode::Flight] {
            app.world_mut().write_message(MovementIntent {
                switch_mode: true,
                ..default()
            });
            app.update();
            let p = app
                .world_mut()
                .query::<&Player>()
                .single(app.world())
                .unwrap();
            assert_eq!(p.mode, mode);
            assert_eq!(p.eye, eye);
            let feedback: Vec<_> = app
                .world_mut()
                .resource_mut::<Messages<MovementFeedback>>()
                .drain()
                .map(|message| message.message())
                .collect();
            assert_eq!(
                feedback,
                [if mode == MovementMode::Physics {
                    "Physics mode"
                } else {
                    "Free flight mode"
                }]
            );
        }
    }

    const SPAWN_XZ_M: f32 = 2.0;
    const SPAWN_Y_M: f32 = 2.0;

    /// A Player far from the spawn, which sits at (2, 2, 2) m in a Chunk of air
    /// holding one solid slab per `slab` (min y, max y in metres) across it.
    fn respawn_app(size: f32, mode: MovementMode, slab: Option<(f32, f32)>) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(WorldSettings {
                voxel_size: size,
                ..default()
            })
            .insert_resource(SpawnPoint::for_tests(Vec3::new(
                SPAWN_XZ_M, SPAWN_Y_M, SPAWN_XZ_M,
            )))
            .init_resource::<ChunkStore>()
            .insert_resource(ContentTables(std::sync::Arc::new(tables())))
            .add_plugins(plugin);
        app.update();
        if let Some((low, high)) = slab {
            let mut chunk = tesserae_core::Chunk::air();
            let cells = |m: f32| (m / size).round() as usize;
            for y in cells(low)..cells(high) {
                for z in 0..32 {
                    for x in 0..32 {
                        chunk.set(x, y, z, Voxel::cube(8));
                    }
                }
            }
            app.world_mut()
                .resource_mut::<ChunkStore>()
                .insert(crate::world::ChunkCoord::new(0, 0, 0), chunk);
        }
        let mut query = app.world_mut().query::<&mut Player>();
        {
            let mut player = query.single_mut(app.world_mut()).unwrap();
            player.mode = mode;
            player.eye = Vec3::new(0.5, 0.5, 0.5);
        }
        app
    }
    fn respawn(app: &mut App) -> (Vec3, Vec<MovementFeedback>) {
        app.world_mut().write_message(RespawnIntent);
        app.update();
        let eye = app
            .world_mut()
            .query::<&Player>()
            .single(app.world())
            .unwrap()
            .eye;
        let feedback = app
            .world_mut()
            .resource_mut::<Messages<MovementFeedback>>()
            .drain()
            .collect();
        (eye, feedback)
    }
    fn body_is_clear(app: &mut App, eye: Vec3) -> bool {
        let mut player = Player::new(eye, MovementMode::Physics);
        player.crouching = false;
        let size = app.world().resource::<WorldSettings>().voxel_size;
        !collide::overlaps(
            app.world().resource::<ChunkStore>(),
            &tables(),
            size,
            player.body(),
        )
    }
    #[test]
    fn a_physics_respawn_into_placed_voxels_lifts_to_a_clear_body() {
        for size in [1.0, 0.5, 0.25] {
            let mut app = respawn_app(size, MovementMode::Physics, Some((2.0, 3.0)));
            let (eye, feedback) = respawn(&mut app);
            assert!(body_is_clear(&mut app, eye), "{size}: embedded at {eye:?}");
            assert!(eye.y >= 3.0 + EYE_HEIGHT - 0.001, "{size}: {eye:?}");
            assert!(eye.y < 3.0 + EYE_HEIGHT + 0.3, "{size}: {eye:?}");
            assert_eq!((eye.x, eye.z), (SPAWN_XZ_M, SPAWN_XZ_M), "{size}");
            assert!(feedback.is_empty(), "{size}: {feedback:?}");
        }
    }
    #[test]
    fn a_physics_respawn_with_no_clear_spot_is_refused_and_leaves_the_eye() {
        for size in [1.0, 0.5, 0.25] {
            let mut app = respawn_app(size, MovementMode::Physics, None);
            let (eye, feedback) = respawn(&mut app);
            assert_eq!(eye, Vec3::splat(0.5), "{size}: unloaded Chunk");
            assert_eq!(feedback, [MovementFeedback::RespawnBlocked], "{size}");
        }
    }
    #[test]
    fn respawn_into_open_air_and_flight_respawn_are_unchanged() {
        for size in [1.0, 0.5, 0.25] {
            let mut app = respawn_app(size, MovementMode::Physics, Some((0.0, 1.0)));
            let (eye, feedback) = respawn(&mut app);
            assert_eq!(
                eye,
                Vec3::new(SPAWN_XZ_M, SPAWN_Y_M + EYE_HEIGHT, SPAWN_XZ_M),
                "{size}"
            );
            assert!(feedback.is_empty());
            let mut app = respawn_app(size, MovementMode::Flight, Some((2.0, 3.0)));
            let (eye, feedback) = respawn(&mut app);
            let spawn = *app.world().resource::<SpawnPoint>();
            assert_eq!(eye, spawn.eye(), "{size}");
            assert!(feedback.is_empty());
        }
    }

    #[test]
    fn a_loaded_air_gap_is_fallen_into_instead_of_being_an_invisible_floor() {
        struct Gap;
        impl VoxelVolume for Gap {
            fn voxel(&self, at: [i32; 3]) -> Option<Voxel> {
                Some(if at[0] < 3 && at[1] < 0 {
                    Voxel::cube(8)
                } else {
                    Voxel::AIR
                })
            }
        }
        let mut p = player();
        p.intent.direction = Vec3::X;
        let t = tables();
        for _ in 0..128 {
            p.step(&Gap, &t, 1.0, 1.0 / 64.0);
        }
        assert!(p.eye.y < EYE_HEIGHT - 2.0);
        assert!(!p.grounded);
    }

    #[test]
    #[ignore = "manual wall-time cost sample; run without concurrent builds or benchmarks"]
    fn movement_cost_sample() {
        let t = tables();
        for size in [1.0, 0.5, 0.25] {
            let mut s = scene(size);
            s.wall = true;
            for _ in 0..3 {
                let mut p = player();
                p.intent.direction = Vec3::X;
                let start = std::time::Instant::now();
                for _ in 0..100_000 {
                    p.step(std::hint::black_box(&s), &t, size, 1.0 / 64.0);
                    std::hint::black_box(&p);
                }
                eprintln!(
                    "physics fixed64 shape-wall size={size} count=100000 wall_ns_per_step={:.1}",
                    start.elapsed().as_nanos() as f64 / 100_000.0
                );
            }
        }
    }

    #[test]
    fn cancellation_before_a_fixed_tick_discards_the_queued_jump() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::ZERO,
            ))
            .insert_resource(WorldSettings::default())
            .insert_resource(SpawnPoint::for_tests(Vec3::ZERO))
            .init_resource::<ChunkStore>()
            .insert_resource(ContentTables(std::sync::Arc::new(tables())))
            .add_plugins(plugin);
        app.update();
        let eye = Vec3::new(4.0, 1.0 + EYE_HEIGHT, 4.0);
        {
            let mut q = app.world_mut().query::<&mut Player>();
            let mut p = q.single_mut(app.world_mut()).unwrap();
            p.eye = eye;
            p.mode = MovementMode::Physics;
            p.grounded = true;
        }
        let mut chunk = tesserae_core::Chunk::air();
        for x in 0..32 {
            for z in 0..32 {
                chunk.set(x, 0, z, Voxel::cube(8));
            }
        }
        app.world_mut()
            .resource_mut::<ChunkStore>()
            .insert(crate::world::ChunkCoord::new(0, 0, 0), chunk);
        app.world_mut().write_message(MovementIntent {
            jump: true,
            ..default()
        });
        app.update();
        app.world_mut().write_message(MovementIntent {
            cancelled: true,
            ..default()
        });
        app.update();
        app.world_mut()
            .resource_mut::<Time<Fixed>>()
            .advance_by(std::time::Duration::from_secs_f64(1.0 / 64.0));
        app.world_mut().run_schedule(FixedUpdate);
        let p = app
            .world_mut()
            .query::<&Player>()
            .single(app.world())
            .unwrap();
        assert_eq!(p.eye, eye);
        assert!(!p.intent.jump);
        let status = super::super::persistence::SaveStatus {
            quitting: true,
            ..default()
        };
        app.world_mut().insert_resource(status);
        app.world_mut().write_message(MovementIntent {
            jump: true,
            ..default()
        });
        app.update();
        app.world_mut().run_schedule(FixedUpdate);
        app.world_mut()
            .resource_mut::<super::super::persistence::SaveStatus>()
            .quitting = false;
        app.world_mut().run_schedule(FixedUpdate);
        assert_eq!(
            app.world_mut()
                .query::<&Player>()
                .single(app.world())
                .unwrap()
                .eye,
            eye
        );
    }

    #[test]
    fn cancellation_stops_flight_and_physics() {
        for mode in [MovementMode::Flight, MovementMode::Physics] {
            let s = scene(1.0);
            let mut p = player();
            p.mode = mode;
            p.intent = MovementIntent {
                direction: Vec3::X,
                speed_mps: 12.0,
                sprint: true,
                ..default()
            };
            steps(&mut p, &s, 64);
            let x = p.eye.x;
            p.intent = default();
            steps(&mut p, &s, 64);
            assert_eq!(p.eye.x, x);
        }
    }

    #[test]
    fn the_published_body_reports_the_fluid_at_the_eye_in_both_modes() {
        let water = tesserae_core::testgen::fixtures().water;
        for mode in [MovementMode::Flight, MovementMode::Physics] {
            let mut app = App::new();
            app.add_plugins(MinimalPlugins)
                .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                    std::time::Duration::ZERO,
                ))
                .insert_resource(WorldSettings::default())
                .insert_resource(SpawnPoint::for_tests(Vec3::ZERO))
                .init_resource::<ChunkStore>()
                .insert_resource(ContentTables(std::sync::Arc::new(tables())))
                .add_plugins(plugin);
            app.update();
            let size = app.world().resource::<WorldSettings>().voxel_size;
            let mut chunk = tesserae_core::Chunk::air();
            for x in 0..32 {
                for z in 0..32 {
                    chunk.set(x, 0, z, Voxel::cube(8));
                    for y in 1..10 {
                        chunk.set(x, y, z, Voxel::cube(water));
                    }
                }
            }
            app.world_mut()
                .resource_mut::<ChunkStore>()
                .insert(crate::world::ChunkCoord::new(0, 0, 0), chunk);
            let eye = Vec3::new(4.0, 5.0 * size, 4.0);
            {
                let mut q = app.world_mut().query::<&mut Player>();
                let mut p = q.single_mut(app.world_mut()).unwrap();
                p.eye = eye;
                p.mode = mode;
            }
            app.world_mut()
                .resource_mut::<Time<Fixed>>()
                .advance_by(std::time::Duration::from_secs_f64(1.0 / 64.0));
            app.world_mut().run_schedule(FixedUpdate);
            let p = app
                .world_mut()
                .query::<&Player>()
                .single(app.world())
                .unwrap();
            let found = p.body_state().immersion.expect("the eye is in the lake");
            assert_eq!(found.material, water, "{mode:?}");
            assert_eq!(found.surface_y_m, 10.0 * size, "{mode:?}");
        }
    }
}
