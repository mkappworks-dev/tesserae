//! Copy a Server-resolved Voxel into the hand without changing the World.
use super::{
    camera::FlyCamera,
    edit::{Holding, Hotbar},
    menu::Menu,
};
use crate::server::edit::{AimItem, PickIntent, PickedItem};
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
    app.init_resource::<Pending>()
        .add_message::<PickIntent>()
        .add_message::<PickedItem>()
        .add_systems(
            Update,
            (send.after(super::edit::send_intents), receive).chain(),
        );
}
fn send(
    (mouse, keys): (Res<ButtonInput<MouseButton>>, Res<ButtonInput<KeyCode>>),
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
    if !grabbed
        || keys.just_pressed(KeyCode::KeyX)
        || super::edit::SLOT_KEYS
            .iter()
            .any(|key| keys.just_pressed(*key))
    {
        pending.request = None;
    }
    if ready
        && !keys.just_pressed(KeyCode::KeyX)
        && (mouse.just_pressed(MouseButton::Middle) || keys.just_pressed(KeyCode::KeyP))
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
