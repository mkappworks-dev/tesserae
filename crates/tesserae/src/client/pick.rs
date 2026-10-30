//! Copy a Server-resolved Voxel into the hand without changing the World.
use super::{
    camera::FlyCamera,
    edit::{Holding, Hotbar},
    menu::Menu,
};
use crate::server::edit::{AimItem, PickIntent, PickedItem};
use crate::session::SessionApp;
use bevy::{
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};

#[derive(Resource, Default)]
struct Pending {
    seq: u32,
    request: Option<(u32, Hotbar)>,
}

pub(super) fn plugin(app: &mut App) {
    app.init_session_state::<Pending>()
        .add_session_message::<PickIntent>()
        .add_session_message::<PickedItem>()
        .add_session_systems(
            Update,
            (send.after(super::edit::send_intents), receive).chain(),
        );
}
fn send(
    (ownership, actions): (Res<super::input::Ownership>, Res<super::actions::Actions>),
    cursor: Single<&CursorOptions, With<PrimaryWindow>>,
    camera: Single<&GlobalTransform, With<FlyCamera>>,
    (menu, hotbar): (Res<Menu>, Res<Hotbar>),
    mut pending: ResMut<Pending>,
    mut messages: MessageWriter<PickIntent>,
    mut was_grabbed: Local<bool>,
) {
    let grabbed = cursor.grab_mode == CursorGrabMode::Locked && !menu.open;
    let ready = grabbed && std::mem::replace(&mut *was_grabbed, grabbed);
    *was_grabbed = grabbed;
    use super::actions::{Action, SLOTS};
    if !ownership.gameplay()
        || !grabbed
        || actions.started(Action::EmptyHand)
        || SLOTS.iter().any(|action| actions.started(*action))
    {
        pending.request = None;
    }
    if ready
        && ownership.gameplay()
        && !actions.started(Action::EmptyHand)
        && actions.started(Action::Sample)
    {
        pending.seq = pending.seq.wrapping_add(1);
        pending.request = Some((pending.seq, hotbar.clone()));
        messages.write(PickIntent {
            seq: pending.seq,
            origin: camera.translation(),
            direction: *camera.forward(),
        });
    }
}
fn receive(
    mut messages: MessageReader<PickedItem>,
    mut pending: ResMut<Pending>,
    mut hotbar: ResMut<Hotbar>,
) {
    for message in messages.read() {
        if pending
            .request
            .as_ref()
            .is_some_and(|(seq, _)| *seq == message.seq)
        {
            let (_, before) = pending.request.take().unwrap();
            if before != *hotbar {
                continue;
            }
            if let Some(item) = message.item {
                hotbar.outside = Some(match item {
                    AimItem::Solid(voxel) => Holding::Solid {
                        material: voxel.material(),
                        shape: voxel.shape(),
                        orientation: tesserae_core::Orientation::from_bits(voxel.orientation())
                            .unwrap_or(tesserae_core::Orientation::IDENTITY),
                    },
                    AimItem::Fluid(material) => Holding::Fluid(material),
                    AimItem::Fire => Holding::Fire,
                });
            }
            pending.request = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tesserae_core::{Orientation, SHAPE_STAIR, Voxel};
    #[test]
    fn menu_entry_discards_a_sample_reply_in_the_same_frame() {
        let mut app = App::new();
        app.add_plugins(super::super::actions::test_plugin)
            .insert_resource(super::super::input::Ownership::playing())
            .init_resource::<Menu>()
            .init_resource::<Hotbar>()
            .init_resource::<Pending>()
            .add_message::<PickIntent>()
            .add_message::<PickedItem>()
            .add_systems(
                Update,
                (super::super::input::resolve, send, receive).chain(),
            );
        app.world_mut().spawn((
            Window {
                focused: true,
                ..default()
            },
            PrimaryWindow,
            CursorOptions {
                grab_mode: CursorGrabMode::Locked,
                ..default()
            },
        ));
        app.world_mut()
            .spawn((FlyCamera::default(), GlobalTransform::IDENTITY));
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyP);
        app.update();
        let seq = app
            .world()
            .resource::<Pending>()
            .request
            .as_ref()
            .unwrap()
            .0;
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.world_mut().resource_mut::<Menu>().open = true;
        app.world_mut().write_message(PickedItem {
            seq,
            item: Some(AimItem::Fire),
        });
        app.update();
        assert_eq!(app.world().resource::<Hotbar>().held(), Holding::Empty);
        assert!(app.world().resource::<Pending>().request.is_none());
    }

    #[test]
    fn picking_preserves_shape_and_orientation_and_does_not_overwrite_a_later_selection() {
        let mut app = App::new();
        app.init_resource::<Pending>()
            .init_resource::<Hotbar>()
            .add_message::<PickedItem>()
            .add_systems(Update, receive);
        let before = app.world().resource::<Hotbar>().clone();
        app.world_mut().resource_mut::<Pending>().request = Some((1, before.clone()));
        let orientation = Orientation::IDENTITY.turned();
        let item = Some(AimItem::Solid(Voxel::shaped(23, SHAPE_STAIR, orientation)));
        app.world_mut().write_message(PickedItem { seq: 1, item });
        app.update();
        assert_eq!(
            app.world().resource::<Hotbar>().held(),
            Holding::Solid {
                material: 23,
                shape: SHAPE_STAIR,
                orientation
            }
        );
        assert_eq!(app.world().resource::<Hotbar>().slots, before.slots);
        let before = app.world().resource::<Hotbar>().clone();
        app.world_mut().resource_mut::<Pending>().request = Some((2, before));
        app.world_mut().resource_mut::<Hotbar>().select(0);
        app.world_mut().write_message(PickedItem { seq: 2, item });
        app.update();
        assert_eq!(app.world().resource::<Hotbar>().held(), Holding::Empty);
    }
}
