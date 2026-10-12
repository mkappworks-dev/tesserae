//! Tells the Server where the Player's Eye is.
//!
//! The Server holds the Player and decides how much of this to believe
//! (`server/player.rs`), and says where the Player is when it did not take it
//! all; the Client only reports and obeys. It reads the camera's
//! `GlobalTransform`, as `edit::send_intents` does for an Edit's origin, so a
//! report and an intent sent in one frame carry one point.

use bevy::prelude::*;

use super::camera::FlyCamera;
use crate::server::player::{PoseCorrection, PoseReport};

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<PoseSeq>().add_systems(
        Update,
        // A correction first, so the report is of where the camera now is.
        (apply_correction, report_pose).chain(),
    );
}

/// The last `seq` sent. The Server drops a report that is not newer.
#[derive(Resource, Default)]
pub(super) struct PoseSeq(u32);

/// Puts the camera where the Server says the Player is. The camera is Client
/// state, so this is not a World write. `GlobalTransform` is set too: it is what
/// `report_pose` and `send_intents` read, and propagation does not run until
/// `PostUpdate`, so without it this frame would report the position just left.
fn apply_correction(
    mut corrections: MessageReader<PoseCorrection>,
    camera: Single<(&mut Transform, &mut GlobalTransform), With<FlyCamera>>,
    mut newest: Local<Option<u32>>,
) {
    // In order, and the last that is not older than one already applied is where
    // the Player is. A reordered channel can deliver an older one late.
    let mut latest = None;
    for correction in corrections.read() {
        let older = newest.is_some_and(|seen| (correction.seq.wrapping_sub(seen) as i32) < 0);
        if !older {
            *newest = Some(correction.seq);
            latest = Some(*correction);
        }
    }
    let Some(correction) = latest else {
        return;
    };
    let (mut transform, mut global) = camera.into_inner();
    transform.translation = correction.eye;
    *global = GlobalTransform::from(*transform);
}

/// Only when the Eye has moved: a Player standing still costs the wire nothing.
pub(super) fn report_pose(
    camera: Single<&GlobalTransform, With<FlyCamera>>,
    mut seq: ResMut<PoseSeq>,
    mut reports: MessageWriter<PoseReport>,
    mut last_sent: Local<Option<Vec3>>,
) {
    let eye = camera.translation();
    if *last_sent == Some(eye) {
        return;
    }
    *last_sent = Some(eye);
    seq.0 = seq.0.wrapping_add(1);
    reports.write(PoseReport { seq: seq.0, eye });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<PoseReport>()
            .add_message::<PoseCorrection>();
        app.add_plugins(plugin);
        let camera = app
            .world_mut()
            .spawn((
                FlyCamera::default(),
                Transform::from_xyz(1.0, 2.0, 3.0),
                GlobalTransform::from(Transform::from_xyz(1.0, 2.0, 3.0)),
            ))
            .id();
        (app, camera)
    }

    fn reports(app: &mut App) -> Vec<PoseReport> {
        app.update();
        app.world_mut()
            .resource_mut::<Messages<PoseReport>>()
            .drain()
            .collect()
    }

    #[test]
    fn the_first_frame_reports_the_camera_as_seq_1() {
        let (mut app, _) = app();
        let sent = reports(&mut app);
        assert_eq!(sent.len(), 1);
        assert_eq!((sent[0].seq, sent[0].eye), (1, Vec3::new(1.0, 2.0, 3.0)));
    }

    #[test]
    fn a_still_camera_sends_nothing_and_a_moved_one_sends_the_next_seq() {
        let (mut app, camera) = app();
        reports(&mut app);
        assert!(reports(&mut app).is_empty(), "standing still");

        *app.world_mut().get_mut::<GlobalTransform>(camera).unwrap() =
            GlobalTransform::from(Transform::from_xyz(4.0, 5.0, 6.0));
        let sent = reports(&mut app);
        assert_eq!((sent[0].seq, sent[0].eye), (2, Vec3::new(4.0, 5.0, 6.0)));
    }

    fn correct(app: &mut App, eye: Vec3) {
        app.world_mut()
            .write_message(PoseCorrection { seq: 1, eye });
    }

    #[test]
    fn a_correction_moves_the_camera_and_the_same_frames_report_is_the_corrected_point() {
        let (mut app, camera) = app();
        reports(&mut app);

        correct(&mut app, Vec3::new(7.0, 8.0, 9.0));
        let sent = reports(&mut app);

        let world = app.world();
        assert_eq!(
            world.get::<Transform>(camera).unwrap().translation,
            Vec3::new(7.0, 8.0, 9.0)
        );
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].eye, Vec3::new(7.0, 8.0, 9.0));
    }

    #[test]
    fn the_last_of_several_corrections_in_a_frame_wins() {
        let (mut app, camera) = app();
        reports(&mut app);
        correct(&mut app, Vec3::X);
        correct(&mut app, Vec3::Y);
        reports(&mut app);
        assert_eq!(
            app.world().get::<Transform>(camera).unwrap().translation,
            Vec3::Y
        );
    }

    #[test]
    fn a_correction_older_than_one_already_applied_is_ignored() {
        let (mut app, camera) = app();
        reports(&mut app);
        let at = |app: &App| app.world().get::<Transform>(camera).unwrap().translation;

        app.world_mut().write_message(PoseCorrection {
            seq: 9,
            eye: Vec3::X,
        });
        reports(&mut app);
        app.world_mut().write_message(PoseCorrection {
            seq: 4,
            eye: Vec3::Y,
        });
        reports(&mut app);
        assert_eq!(at(&app), Vec3::X, "a late straggler");

        // The same `seq` is a respawn after the clamp it answers, and is newer.
        app.world_mut().write_message(PoseCorrection {
            seq: 9,
            eye: Vec3::Z,
        });
        reports(&mut app);
        assert_eq!(at(&app), Vec3::Z);
    }

    #[test]
    fn no_correction_leaves_the_camera_where_it_is() {
        let (mut app, camera) = app();
        let before = app.world().get::<Transform>(camera).copied();
        reports(&mut app);
        assert_eq!(app.world().get::<Transform>(camera).copied(), before);
    }
}
