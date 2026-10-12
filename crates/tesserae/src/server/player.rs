//! The Server's Player: what it holds for each person in the World, and so the
//! one place that says where an Edit may be made from.
//!
//! The Client reports where it is ([`PoseReport`]) and the Server decides how
//! much of that to believe. A pose may not move faster than
//! [`PlayerLimits::max_speed_mps`]: a report that asks for more is clamped and
//! the Client is told where it really is ([`PoseCorrection`]). That is the only
//! movement rule there is, because flying is the only way to move; gravity,
//! collision and a walking speed are the player controller's, so a modified
//! Client can still fly at the cap and edit within reach of where it flies.
//! `.scratch/2026-10-12-server-edit-origin/spec.md`, decisions 6 and 8.
//!
//! See `docs/adr/0002-client-server-from-day-one.md`.

use bevy::prelude::*;

use super::world_state::SpawnPoint;

/// The camera's own stated ceiling, in metres per second: a Cell every half
/// second, past which Chunks cannot be meshed fast enough. The Client holds its
/// flight to this, so it never meets the cap it is held to.
pub const FLY_SPEED_CEILING_MPS: f32 = 64.0;

/// Seconds of full-speed travel a Player may bank, and so the jump a Client that
/// was silent can make. It is what absorbs jitter: over a wire a report's time is
/// when it arrives, not when it was sent. Not a slack added to every report,
/// which would be free speed per report.
const POSE_BURST_S: f32 = 0.25;

/// Float noise in comparing a distance with a budget; a Client flying exactly at
/// the cap must not be corrected for rounding.
const POSE_EPSILON_M: f32 = 1e-3;

/// What a Player may do. A resource so the controller replaces the figure.
#[derive(Resource, Clone, Copy, Debug)]
pub struct PlayerLimits {
    /// Metres per second, not Voxels per second.
    /// `docs/adr/0003-voxel-size-is-per-world.md`.
    pub max_speed_mps: f32,
}

impl Default for PlayerLimits {
    fn default() -> Self {
        PlayerLimits {
            max_speed_mps: FLY_SPEED_CEILING_MPS,
        }
    }
}

impl PlayerLimits {
    fn burst_m(self) -> f32 {
        self.max_speed_mps * POSE_BURST_S
    }
}

/// One person in the World. The fields are private so the Client has no route to
/// move a Player but a [`PoseReport`] or a [`RespawnIntent`].
#[derive(Component, Debug)]
pub struct Player {
    /// Metres.
    eye: Vec3,
    seq: u32,
    /// Metres of movement the Player may still make now.
    budget_m: f32,
    /// Server seconds at which `budget_m` was last brought up to date.
    refilled_at: f64,
}

/// What became of a report.
#[derive(Debug, PartialEq)]
enum Verdict {
    Dropped,
    Taken,
    /// Taken as far as the budget allowed, to this point.
    Clamped(Vec3),
}

impl Player {
    fn new(eye: Vec3, limits: PlayerLimits, now: f64) -> Self {
        Player {
            eye,
            seq: 0,
            budget_m: limits.burst_m(),
            refilled_at: now,
        }
    }

    /// Where an Edit's ray may start near, in metres.
    pub fn eye(&self) -> Vec3 {
        self.eye
    }

    fn refill(&mut self, limits: PlayerLimits, now: f64) {
        let elapsed = (now - self.refilled_at).max(0.0) as f32;
        self.budget_m = (self.budget_m + limits.max_speed_mps * elapsed).min(limits.burst_m());
        self.refilled_at = now;
    }

    /// A repeated or older `seq` is dropped, so a report is idempotent and
    /// survives reordering on an unreliable channel.
    fn accept(&mut self, report: PoseReport, limits: PlayerLimits, now: f64) -> Verdict {
        // Wrapping, so the counter can overflow without freezing the Player.
        let newer = report.seq.wrapping_sub(self.seq) as i32 > 0;
        if !newer || !report.eye.is_finite() {
            return Verdict::Dropped;
        }
        self.refill(limits, now);
        self.seq = report.seq;

        let delta = report.eye - self.eye;
        let distance = delta.length();
        if distance <= self.budget_m + POSE_EPSILON_M {
            self.budget_m = (self.budget_m - distance).max(0.0);
            self.eye = report.eye;
            return Verdict::Taken;
        }
        self.eye += delta / distance * self.budget_m;
        self.budget_m = 0.0;
        Verdict::Clamped(self.eye)
    }

    fn respawn(&mut self, eye: Vec3, limits: PlayerLimits, now: f64) {
        self.eye = eye;
        self.budget_m = limits.burst_m();
        self.refilled_at = now;
    }

    #[cfg(test)]
    pub(in crate::server) fn at(eye: Vec3) -> Self {
        Player::new(eye, PlayerLimits::default(), 0.0)
    }

    #[cfg(test)]
    pub(in crate::server) fn set_eye(&mut self, eye: Vec3) {
        self.eye = eye;
    }
}

/// Where the Client says its Player's Eye is, in metres. Plain data with no
/// `Entity`, so it can cross a wire unchanged. The first report is `seq` 1.
#[derive(Message, Clone, Copy, Debug)]
pub struct PoseReport {
    pub seq: u32,
    pub eye: Vec3,
}

/// Where the Server has the Player, sent when it is not where the Client said:
/// the report was clamped, or the Player was respawned. `seq` is the last report
/// the Server took, so a Client can tell a late correction from a current one.
#[derive(Message, Clone, Copy, Debug)]
pub struct PoseCorrection {
    pub seq: u32,
    pub eye: Vec3,
}

/// The Client asks to be put back at the spawn eye. The Client moving itself
/// there would be a teleport, which is what the speed cap refuses.
#[derive(Message, Clone, Copy, Debug)]
pub struct RespawnIntent;

/// What the Player's input does to it, before an Edit is judged against it.
#[derive(SystemSet, Hash, PartialEq, Eq, Clone, Debug)]
pub(super) struct PlayerInput;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<PlayerLimits>()
        .add_message::<PoseReport>()
        .add_message::<PoseCorrection>()
        .add_message::<RespawnIntent>()
        // After `Startup`, where `open_world` queues the spawn point.
        .add_systems(PostStartup, spawn_player)
        .add_systems(
            RunFixedMainLoop,
            (receive_pose, receive_respawn)
                .chain()
                .in_set(PlayerInput)
                .in_set(RunFixedMainLoopSystems::BeforeFixedMainLoop),
        );
}

fn spawn_player(
    mut commands: Commands,
    spawn: Res<SpawnPoint>,
    limits: Res<PlayerLimits>,
    time: Res<Time>,
) {
    commands.spawn(Player::new(spawn.eye(), *limits, time.elapsed_secs_f64()));
}

/// Every frame, for the reason `edit::receive` is.
pub(super) fn receive_pose(
    mut reports: MessageReader<PoseReport>,
    mut corrections: MessageWriter<PoseCorrection>,
    mut player: Single<&mut Player>,
    limits: Res<PlayerLimits>,
    time: Res<Time>,
) {
    let now = time.elapsed_secs_f64();
    for report in reports.read() {
        match player.accept(*report, *limits, now) {
            Verdict::Taken => {}
            Verdict::Clamped(eye) => {
                debug!("pose report clamped: {report:?}");
                corrections.write(PoseCorrection {
                    seq: report.seq,
                    eye,
                });
            }
            Verdict::Dropped => debug!("pose report dropped: {report:?}"),
        }
    }
}

fn receive_respawn(
    mut intents: MessageReader<RespawnIntent>,
    mut corrections: MessageWriter<PoseCorrection>,
    mut player: Single<&mut Player>,
    spawn: Res<SpawnPoint>,
    limits: Res<PlayerLimits>,
    time: Res<Time>,
) {
    if intents.read().next().is_none() {
        return;
    }
    intents.clear();
    player.respawn(spawn.eye(), *limits, time.elapsed_secs_f64());
    corrections.write(PoseCorrection {
        seq: player.seq,
        eye: player.eye,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::time::TimeUpdateStrategy;
    use std::time::Duration;

    const SPAWN: Vec3 = Vec3::new(1.0, 2.0, 3.0);

    fn spawn_eye() -> Vec3 {
        SpawnPoint::for_tests(SPAWN).eye()
    }

    /// Each `update` is `step_ms` of Server time, so the budget is deterministic.
    fn app_with_step(step_ms: u64) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
                step_ms,
            )))
            .insert_resource(SpawnPoint::for_tests(SPAWN))
            .add_plugins(plugin);
        app.update();
        app
    }

    fn app() -> App {
        app_with_step(0)
    }

    fn eye(app: &mut App) -> Vec3 {
        app.world_mut()
            .query::<&Player>()
            .single(app.world())
            .unwrap()
            .eye()
    }

    fn report(app: &mut App, seq: u32, eye: Vec3) {
        app.world_mut().write_message(PoseReport { seq, eye });
        app.update();
    }

    fn corrections(app: &mut App) -> Vec<PoseCorrection> {
        app.world_mut()
            .resource_mut::<Messages<PoseCorrection>>()
            .drain()
            .collect()
    }

    /// Near the spawn eye, inside the first burst.
    fn near(offset: Vec3) -> Vec3 {
        spawn_eye() + offset
    }

    #[test]
    fn a_player_starts_at_the_spawn_eye_and_the_first_report_there_is_not_corrected() {
        let mut app = app();
        assert_eq!(eye(&mut app), spawn_eye());
        report(&mut app, 1, spawn_eye());
        assert_eq!(eye(&mut app), spawn_eye());
        assert!(corrections(&mut app).is_empty());
    }

    #[test]
    fn a_report_inside_the_budget_moves_the_eye_and_is_not_corrected() {
        let mut app = app();
        report(&mut app, 1, near(Vec3::new(5.0, 0.0, 0.0)));
        assert_eq!(eye(&mut app), near(Vec3::new(5.0, 0.0, 0.0)));
        assert!(corrections(&mut app).is_empty());
    }

    #[test]
    fn a_repeated_or_stale_report_changes_nothing() {
        let mut app = app();
        report(&mut app, 5, near(Vec3::X));
        report(&mut app, 5, near(Vec3::Y));
        assert_eq!(eye(&mut app), near(Vec3::X), "repeated");
        report(&mut app, 4, near(Vec3::Z));
        assert_eq!(eye(&mut app), near(Vec3::X), "stale");
        report(&mut app, 6, near(Vec3::Y));
        assert_eq!(eye(&mut app), near(Vec3::Y));
    }

    #[test]
    fn a_non_finite_report_changes_nothing_and_does_not_spend_its_seq() {
        let mut app = app();
        report(&mut app, 1, near(Vec3::X));
        report(&mut app, 2, Vec3::new(f32::NAN, 0.0, 0.0));
        report(&mut app, 3, Vec3::new(0.0, f32::INFINITY, 0.0));
        assert_eq!(eye(&mut app), near(Vec3::X));
        report(&mut app, 2, near(Vec3::Y));
        assert_eq!(eye(&mut app), near(Vec3::Y), "seq 2 was never accepted");
        assert!(corrections(&mut app).is_empty(), "dropped, not clamped");
    }

    #[test]
    fn the_counter_wrapping_is_newer_not_stale() {
        let mut app = app();
        // A jump of more than half the range reads as going backwards, so the
        // counter has to be walked up to the wrap.
        for seq in [i32::MAX as u32, u32::MAX - 1, u32::MAX] {
            report(&mut app, seq, near(Vec3::X));
        }
        assert_eq!(eye(&mut app), near(Vec3::X));
        report(&mut app, 0, near(Vec3::Y));
        assert_eq!(eye(&mut app), near(Vec3::Y));
        report(&mut app, u32::MAX, near(Vec3::Z));
        assert_eq!(
            eye(&mut app),
            near(Vec3::Y),
            "the old end of the range is stale now"
        );
    }

    #[test]
    fn a_report_is_applied_in_the_frame_it_is_written() {
        // `receive_pose` is in `BeforeFixedMainLoop`: a frame that runs no fixed
        // tick must not lose it.
        let mut app = app();
        report(&mut app, 1, near(Vec3::X));
        assert_eq!(eye(&mut app), near(Vec3::X));
    }

    #[test]
    fn a_teleport_is_clamped_to_the_budget_and_the_client_is_told_where_it_is() {
        let mut app = app();
        let far = near(Vec3::new(1000.0, 0.0, 0.0));
        report(&mut app, 7, far);

        let burst = FLY_SPEED_CEILING_MPS * POSE_BURST_S;
        let moved = eye(&mut app) - spawn_eye();
        assert!(
            (moved - Vec3::X * burst).length() < 1e-3,
            "moved {moved}, wanted {burst} m along the report"
        );
        let told = corrections(&mut app);
        assert_eq!(told.len(), 1);
        assert_eq!(told[0].seq, 7);
        assert_eq!(told[0].eye, eye(&mut app));
    }

    #[test]
    fn a_long_silence_buys_no_more_than_the_burst() {
        let mut app = app_with_step(60_000);
        let burst = FLY_SPEED_CEILING_MPS * POSE_BURST_S;
        // A minute of Server time has passed since the Player spawned.
        report(&mut app, 1, near(Vec3::new(burst * 2.0, 0.0, 0.0)));
        let moved = (eye(&mut app) - spawn_eye()).length();
        assert!((moved - burst).abs() < 1e-3, "moved {moved} m");
    }

    #[test]
    fn many_small_reports_buy_no_more_than_the_speed() {
        // 100 frames of 10 ms is one second, in which 64 m/s allows 64 m plus the
        // burst the Player began with. A Client asking for 1 m a frame is asking
        // for 100 m, so it ends well short of that.
        let mut app = app_with_step(10);
        let mut asked = spawn_eye();
        for seq in 1..=100 {
            asked.x += 1.0;
            report(&mut app, seq, asked);
        }
        let moved = (eye(&mut app) - spawn_eye()).length();
        let allowed = FLY_SPEED_CEILING_MPS * (1.0 + POSE_BURST_S);
        assert!(moved <= allowed + 0.1, "moved {moved} m, allowed {allowed}");
        assert!(moved > 60.0, "and not much less than it, {moved} m");
        assert!(!corrections(&mut app).is_empty());
    }

    #[test]
    fn flying_at_the_ceiling_is_never_corrected() {
        let mut app = app_with_step(16);
        let step = FLY_SPEED_CEILING_MPS * 0.016;
        let mut at = spawn_eye();
        for seq in 1..=200 {
            at.x += step;
            report(&mut app, seq, at);
        }
        assert!(corrections(&mut app).is_empty());
        assert!((eye(&mut app) - at).length() < 0.05);
    }

    #[test]
    fn a_clamped_player_can_continue_from_where_it_was_put() {
        let mut app = app();
        report(&mut app, 1, near(Vec3::new(1000.0, 0.0, 0.0)));
        let put = corrections(&mut app)[0].eye;
        report(&mut app, 2, put + Vec3::X * 0.0);
        assert_eq!(eye(&mut app), put);
        assert!(corrections(&mut app).is_empty());
    }

    #[test]
    fn a_respawn_puts_the_player_at_the_spawn_eye_from_anywhere_and_says_so() {
        let mut app = app();
        // Walk it a burst at a time so it is nowhere near the spawn.
        let mut at = spawn_eye();
        for seq in 1..=3 {
            at.x += 10.0;
            report(&mut app, seq, at);
        }
        assert_ne!(eye(&mut app), spawn_eye());
        corrections(&mut app);

        app.world_mut().write_message(RespawnIntent);
        app.update();

        assert_eq!(eye(&mut app), spawn_eye());
        let told = corrections(&mut app);
        assert_eq!(told.len(), 1);
        assert_eq!((told[0].seq, told[0].eye), (3, spawn_eye()));
    }

    #[test]
    fn a_report_after_a_respawn_from_the_corrected_point_is_accepted() {
        let mut app = app();
        report(&mut app, 1, near(Vec3::new(10.0, 0.0, 0.0)));
        app.world_mut().write_message(RespawnIntent);
        app.update();
        corrections(&mut app);

        report(&mut app, 2, near(Vec3::new(3.0, 0.0, 0.0)));
        assert_eq!(eye(&mut app), near(Vec3::new(3.0, 0.0, 0.0)));
        assert!(corrections(&mut app).is_empty());
    }
}
