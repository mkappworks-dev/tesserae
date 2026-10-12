//! The Server's Player: what it holds for each person in the World, and so the
//! one place that says where an Edit may be made from.
//!
//! The Client reports where it is ([`PoseReport`]) and the Server decides
//! whether to believe it. Today that is only "finite and newer than the last",
//! because the movement rules that would say more (a speed, gravity, a step
//! height) do not exist yet. So a modified Client can report any pose and edit
//! from it: `edit::receive` bounds the inconsistency between a pose and an
//! origin, not the pose. `.scratch/2026-10-12-server-edit-origin/spec.md`,
//! decision 6.
//!
//! See `docs/adr/0002-client-server-from-day-one.md`.

use bevy::prelude::*;

use super::world_state::SpawnPoint;

/// One person in the World. Both fields are private so the Client has no route
/// to move a Player but a [`PoseReport`].
#[derive(Component, Debug)]
pub struct Player {
    /// Metres.
    eye: Vec3,
    seq: u32,
}

impl Player {
    /// Where an Edit's ray may start near, in metres.
    pub fn eye(&self) -> Vec3 {
        self.eye
    }

    /// Whether the report was believed. A repeated or older `seq` is dropped, so
    /// a report is idempotent and survives reordering on an unreliable channel.
    fn accept(&mut self, report: PoseReport) -> bool {
        // Wrapping, so the counter can overflow without freezing the Player.
        let newer = report.seq.wrapping_sub(self.seq) as i32 > 0;
        if !newer || !report.eye.is_finite() {
            return false;
        }
        self.eye = report.eye;
        self.seq = report.seq;
        true
    }

    #[cfg(test)]
    pub(in crate::server) fn at(eye: Vec3) -> Self {
        Player { eye, seq: 0 }
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

pub(super) fn plugin(app: &mut App) {
    app.add_message::<PoseReport>()
        // After `Startup`, where `open_world` queues the spawn point.
        .add_systems(PostStartup, spawn_player)
        .add_systems(
            RunFixedMainLoop,
            receive_pose.in_set(RunFixedMainLoopSystems::BeforeFixedMainLoop),
        );
}

fn spawn_player(mut commands: Commands, spawn: Res<SpawnPoint>) {
    commands.spawn(Player {
        eye: spawn.metres(),
        seq: 0,
    });
}

/// Every frame, for the reason `edit::receive` is.
pub(super) fn receive_pose(
    mut reports: MessageReader<PoseReport>,
    mut player: Single<&mut Player>,
) {
    for report in reports.read() {
        if !player.accept(*report) {
            debug!("pose report dropped: {report:?}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(SpawnPoint::new(Vec3::new(1.0, 2.0, 3.0)))
            .add_plugins(plugin);
        app.update();
        app
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

    #[test]
    fn a_player_starts_at_the_spawn_point_and_the_first_report_moves_it() {
        let mut app = app();
        assert_eq!(eye(&mut app), Vec3::new(1.0, 2.0, 3.0));
        report(&mut app, 1, Vec3::new(10.0, 20.0, 30.0));
        assert_eq!(eye(&mut app), Vec3::new(10.0, 20.0, 30.0));
    }

    #[test]
    fn a_repeated_or_stale_report_changes_nothing() {
        let mut app = app();
        report(&mut app, 5, Vec3::X);
        report(&mut app, 5, Vec3::Y);
        assert_eq!(eye(&mut app), Vec3::X, "repeated");
        report(&mut app, 4, Vec3::Z);
        assert_eq!(eye(&mut app), Vec3::X, "stale");
        report(&mut app, 6, Vec3::Y);
        assert_eq!(eye(&mut app), Vec3::Y);
    }

    #[test]
    fn a_non_finite_report_changes_nothing_and_does_not_spend_its_seq() {
        let mut app = app();
        report(&mut app, 1, Vec3::X);
        report(&mut app, 2, Vec3::new(f32::NAN, 0.0, 0.0));
        report(&mut app, 3, Vec3::new(0.0, f32::INFINITY, 0.0));
        assert_eq!(eye(&mut app), Vec3::X);
        report(&mut app, 2, Vec3::Y);
        assert_eq!(eye(&mut app), Vec3::Y, "seq 2 was never accepted");
    }

    #[test]
    fn the_counter_wrapping_is_newer_not_stale() {
        let mut app = app();
        // A jump of more than half the range reads as going backwards, so the
        // counter has to be walked up to the wrap.
        for seq in [i32::MAX as u32, u32::MAX - 1, u32::MAX] {
            report(&mut app, seq, Vec3::X);
        }
        assert_eq!(eye(&mut app), Vec3::X);
        report(&mut app, 0, Vec3::Y);
        assert_eq!(eye(&mut app), Vec3::Y);
        report(&mut app, u32::MAX, Vec3::Z);
        assert_eq!(
            eye(&mut app),
            Vec3::Y,
            "the old end of the range is stale now"
        );
    }

    #[test]
    fn a_report_is_applied_in_the_frame_it_is_written() {
        // `receive_pose` is in `BeforeFixedMainLoop`: a frame that runs no fixed
        // tick must not lose it.
        let mut app = app();
        report(&mut app, 1, Vec3::X);
        assert_eq!(eye(&mut app), Vec3::X);
    }
}
