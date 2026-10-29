//! Presents the Server-owned Eye; the Client never reports a position.
use super::camera::FlyCamera;
use crate::server::player::{MovementFeedback, PoseCorrection};
use bevy::prelude::*;
pub(super) fn plugin(app: &mut App) {
    app.add_systems(Update, (restore_view, apply_correction, feedback));
}
fn feedback(mut messages: MessageReader<MovementFeedback>, mut commands: Commands) {
    if let Some(message) = messages.read().last() {
        let tone = if message.refused() {
            super::menu_ui::Tone::Alert
        } else {
            super::menu_ui::Tone::Change
        };
        super::menu_ui::notify(&mut commands, message.message(), tone);
    }
}
pub(super) fn apply_correction(
    mut corrections: MessageReader<PoseCorrection>,
    camera: Single<(&mut Transform, &mut GlobalTransform, &mut FlyCamera)>,
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
    let (mut transform, mut global, mut fly) = camera.into_inner();
    fly.movement_mode = correction.mode;
    transform.translation = correction.eye;
    *global = GlobalTransform::from(*transform);
}

fn restore_view(
    mut messages: MessageReader<crate::server::player::RestoreView>,
    camera: Single<(&mut Transform, &mut FlyCamera)>,
) {
    let (mut transform, mut fly) = camera.into_inner();
    for view in messages.read() {
        fly.yaw = view.0;
        fly.pitch = view.1;
        transform.rotation = Quat::from_euler(EulerRot::YXZ, view.0, view.1, 0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::player::{MovementMode, RestoreView};
    #[test]
    fn movement_feedback_uses_the_shared_notification() {
        let mut app = App::new();
        app.add_message::<MovementFeedback>()
            .add_systems(Update, feedback);
        app.world_mut()
            .write_message(MovementFeedback::Switched(MovementMode::Physics));
        app.update();
        let messages: Vec<_> = app
            .world_mut()
            .query::<&Text>()
            .iter(app.world())
            .map(|text| text.0.as_str())
            .collect();
        assert_eq!(messages, ["Physics mode"]);
    }

    #[test]
    fn published_body_and_restored_view_drive_presentation_and_old_snapshots_are_ignored() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<PoseCorrection>()
            .add_message::<RestoreView>()
            .add_message::<MovementFeedback>()
            .init_resource::<super::super::edit::Notice>()
            .add_plugins(plugin);
        let camera = app
            .world_mut()
            .spawn((
                Transform::default(),
                GlobalTransform::default(),
                FlyCamera::default(),
            ))
            .id();
        app.world_mut().write_message(RestoreView(1.0, -0.25));
        app.world_mut().write_message(PoseCorrection {
            seq: 9,
            eye: Vec3::splat(4.0),
            mode: MovementMode::Physics,
        });
        app.update();
        assert_eq!(
            app.world().get::<Transform>(camera).unwrap().translation,
            Vec3::splat(4.0)
        );
        let fly = app.world().get::<FlyCamera>(camera).unwrap();
        assert_eq!((fly.yaw, fly.pitch), (1.0, -0.25));
        assert_eq!(fly.movement_mode, MovementMode::Physics);
        app.world_mut().write_message(PoseCorrection {
            seq: 8,
            eye: Vec3::ZERO,
            mode: MovementMode::Flight,
        });
        app.update();
        assert_eq!(
            app.world()
                .get::<GlobalTransform>(camera)
                .unwrap()
                .translation(),
            Vec3::splat(4.0)
        );
    }
}
