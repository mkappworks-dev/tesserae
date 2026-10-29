//! Aim intent and drawing from the Server's answer, without World access.
use super::{
    camera::FlyCamera,
    edit::{Holding, Hotbar},
    menu::Menu,
};
use crate::server::edit::{AimBounds, AimItem, AimReport, AimTarget};
use bevy::prelude::*;
use std::collections::VecDeque;

#[derive(Resource, Default)]
pub(super) struct Target(
    pub(super) AimTarget,
    pub(super) Option<AimView>,
    pub(super) f64,
);
type AimView = (Vec3, Vec3, Option<AimItem>);

impl Target {
    pub(super) fn current(&self, sent: &Sent, now: f64) -> bool {
        self.1.zip(sent.last).is_some_and(|(answer, current)| {
            now - self.2 <= 0.2
                && answer.2 == current.2
                && answer.0.distance_squared(current.0) <= 4.0
                && answer.1.dot(current.1) >= 0.94
        })
    }
}
#[derive(Resource, Default)]
pub(super) struct Sent {
    pub(super) seq: u32,
    pub(super) last: Option<AimView>,
    history: VecDeque<(u32, AimView)>,
}
#[derive(Component)]
pub(super) struct Outline(pub(super) AimBounds);

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Time>()
        .init_resource::<Target>()
        .init_resource::<Sent>()
        .add_message::<AimReport>()
        .add_message::<AimTarget>()
        .add_systems(
            Update,
            (
                report.after(super::edit::send_intents),
                receive,
                outline,
                draw,
                crosshair,
            )
                .chain(),
        );
}
fn crosshair(
    target: Res<Target>,
    sent: Res<Sent>,
    time: Res<Time>,
    mut crosshair: Query<&mut TextColor, With<super::edit::Crosshair>>,
) {
    let alpha = if target.0.blocked && target.current(&sent, time.elapsed_secs_f64()) {
        super::edit::BLOCKED_CROSSHAIR_ALPHA
    } else {
        super::edit::CROSSHAIR_ALPHA
    };
    for mut colour in &mut crosshair {
        colour.set_if_neq(TextColor(Color::srgba(1.0, 1.0, 1.0, alpha)));
    }
}
pub(super) fn report(
    camera: Single<&GlobalTransform, With<FlyCamera>>,
    hotbar: Res<Hotbar>,
    mut sent: ResMut<Sent>,
    mut reports: MessageWriter<AimReport>,
) {
    let held = match hotbar.held() {
        Holding::Solid {
            material,
            shape,
            orientation,
        } => Some(AimItem::Solid(tesserae_core::Voxel::shaped(
            material,
            shape,
            orientation,
        ))),
        Holding::Fluid(id) => Some(AimItem::Fluid(id)),
        Holding::Fire => Some(AimItem::Fire),
        Holding::Empty => None,
    };
    let current = (camera.translation(), *camera.forward(), held);
    if sent.last == Some(current) {
        return;
    }
    sent.seq = sent.seq.wrapping_add(1);
    sent.last = Some(current);
    let seq = sent.seq;
    sent.history.push_back((seq, current));
    if sent.history.len() > 64 {
        sent.history.pop_front();
    }
    reports.write(AimReport {
        seq: sent.seq,
        origin: current.0,
        direction: current.1,
        held,
    });
}
pub(super) fn receive(
    mut messages: MessageReader<AimTarget>,
    sent: Res<Sent>,
    time: Res<Time>,
    mut target: ResMut<Target>,
) {
    for message in messages.read() {
        let Some((_, view)) = sent.history.iter().find(|(seq, _)| *seq == message.seq) else {
            continue;
        };
        if target.1.is_none() || message.seq.wrapping_sub(target.0.seq) as i32 >= 0 {
            *target = Target(*message, Some(*view), time.elapsed_secs_f64());
        }
    }
}
fn outline(
    mut commands: Commands,
    target: Res<Target>,
    sent: Res<Sent>,
    time: Res<Time>,
    menu: Res<Menu>,
    outlines: Query<(Entity, &Outline)>,
    settings: Option<Res<super::settings::Settings>>,
) {
    let wanted = (!menu.open
        && !settings.is_some_and(|s| s.open)
        && target.current(&sent, time.elapsed_secs_f64()))
    .then_some(target.0.outline)
    .flatten();
    if outlines.iter().next().map(|(_, o)| o.0) == wanted {
        return;
    }
    for (entity, _) in &outlines {
        commands.entity(entity).despawn();
    }
    if let Some(bounds) = wanted {
        commands.spawn(Outline(bounds));
    }
}
fn draw(outlines: Query<&Outline>, mut gizmos: Gizmos) {
    for Outline(bounds) in &outlines {
        gizmos.primitive_3d(
            &Cuboid::from_size(bounds.max - bounds.min + Vec3::splat(0.004)),
            Isometry3d::from_translation((bounds.min + bounds.max) * 0.5),
            Color::WHITE,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delayed_answers_stay_visible_while_moving_but_old_or_incompatible_answers_do_not() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<Target>()
            .init_resource::<Sent>()
            .init_resource::<Menu>()
            .add_message::<AimTarget>()
            .add_systems(Update, (receive, outline).chain());
        let view = (Vec3::ZERO, Vec3::NEG_Z, None);
        {
            let mut sent = app.world_mut().resource_mut::<Sent>();
            sent.seq = 4;
            sent.last = Some((Vec3::X * 0.1, Vec3::NEG_Z, None));
            sent.history.extend([(2, view), (3, view), (4, view)]);
        }
        let bounds = AimBounds {
            min: Vec3::ZERO,
            max: Vec3::ONE,
        };
        let answer = AimTarget {
            removal: None,
            seq: 3,
            outline: Some(bounds),
            ghost: None,
            blocked: false,
        };
        app.world_mut().write_message(answer);
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&Outline>()
                .single(app.world())
                .unwrap()
                .0,
            bounds
        );
        app.world_mut().write_message(AimTarget {
            seq: 2,
            outline: None,
            ..answer
        });
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&Outline>()
                .iter(app.world())
                .count(),
            1
        );
        let moved = AimBounds {
            min: Vec3::X,
            max: Vec3::X + Vec3::ONE,
        };
        app.world_mut().write_message(AimTarget {
            seq: 4,
            outline: Some(moved),
            ..answer
        });
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&Outline>()
                .single(app.world())
                .unwrap()
                .0,
            moved
        );
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(250));
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&Outline>()
                .iter(app.world())
                .count(),
            0
        );
        app.world_mut().write_message(AimTarget {
            seq: 4,
            outline: Some(moved),
            ..answer
        });
        app.update();
        app.world_mut().resource_mut::<Menu>().open = true;
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&Outline>()
                .iter(app.world())
                .count(),
            0
        );
        app.world_mut().resource_mut::<Menu>().open = false;
        app.world_mut().resource_mut::<Sent>().last =
            Some((Vec3::ZERO, Vec3::NEG_Z, Some(AimItem::Fire)));
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<&Outline>()
                .iter(app.world())
                .count(),
            0
        );
    }
}
