//! Tells the Server where the Player's Eye is.
//!
//! The Server holds the Player and decides whether to believe this
//! (`server/player.rs`); the Client only reports. It reads the camera's
//! `GlobalTransform`, as `edit::send_intents` does for an Edit's origin, so a
//! report and an intent sent in one frame carry one point.

use bevy::prelude::*;

use super::camera::FlyCamera;
use crate::server::player::PoseReport;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<PoseSeq>()
        .add_systems(Update, report_pose);
}

/// The last `seq` sent. The Server drops a report that is not newer.
#[derive(Resource, Default)]
pub(super) struct PoseSeq(u32);

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
        app.add_plugins(MinimalPlugins).add_message::<PoseReport>();
        app.init_resource::<PoseSeq>()
            .add_systems(Update, report_pose);
        let camera = app
            .world_mut()
            .spawn((
                FlyCamera::default(),
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
}
