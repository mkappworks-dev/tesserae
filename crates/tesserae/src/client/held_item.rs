//! The held Item in the avatar's right hand, for third person. It is the model
//! the first-person viewmodel draws (`preview::hand_model`), attached to the arm so it
//! moves with it, and it follows the same hand animation, food shrinking as it
//! is eaten. Hidden where the viewmodel is: an empty hand, death, and
//! the Menu, Settings or Inventory.

use super::{
    avatar::HandMount,
    hand_anim::{HandAnim, still},
    menu::Covers,
    preview::{HandModel, Model},
    profile::Preferences,
};
use crate::session::SessionApp;
use bevy::prelude::*;

/// A held cube's edge in metres, whatever the Voxel size: a prop in the hand,
/// not a Voxel.
const HELD_M: f32 = 0.26;
/// Where the cube's centre sits against the grip, in the arm's frame.
const GRIP: Vec3 = Vec3::new(0.0, -0.02, -0.1);

#[derive(Component)]
struct Held;

pub(super) fn plugin(app: &mut App) {
    app.add_session_systems(
        Update,
        hold.after(super::preview::model)
            .after(super::avatar::rebuild)
            .after(super::hand_anim::animate),
    );
}

/// The unit-cell model, `scale` of full size, centred on the grip.
fn placed(scale: f32) -> Transform {
    let edge = HELD_M * scale;
    Transform::from_translation(GRIP - Vec3::splat(edge / 2.0)).with_scale(Vec3::splat(edge))
}

fn visible(model: &Model, hidden: bool) -> Visibility {
    if hidden || matches!(model.held, None | Some(HandModel::Nothing)) {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    }
}

type Mounts<'w, 's> = Query<'w, 's, Entity, With<HandMount>>;

fn hold(
    mut commands: Commands,
    model: Res<Model>,
    (covers, dead): (Covers, Option<Res<super::death::Dead>>),
    (anim, preferences): (Option<Res<HandAnim>>, Option<Res<Preferences>>),
    (mounts, fresh): (Mounts, Query<(), Added<HandMount>>),
    mut items: Query<(Entity, &mut Transform, &mut Visibility), With<Held>>,
) {
    let hidden = super::menu::covered(&covers) || !super::death::alive(dead);
    let scale = anim.map_or(1.0, |anim| anim.pose(still(&preferences)).scale);
    if model.is_changed() || !fresh.is_empty() {
        for (item, ..) in &items {
            commands.entity(item).despawn();
        }
        if matches!(model.held, None | Some(HandModel::Nothing)) {
            return;
        }
        for mount in &mounts {
            commands.spawn((
                Held,
                bevy::light::NotShadowCaster,
                Mesh3d(model.mesh.clone()),
                MeshMaterial3d(model.hand.clone()),
                placed(scale),
                visible(&model, hidden),
                ChildOf(mount),
            ));
        }
        return;
    }
    for (_, mut transform, mut visibility) in &mut items {
        transform.set_if_neq(placed(scale));
        visibility.set_if_neq(visible(&model, hidden));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::{menu::Menu, preview::HandModel};

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<Model>()
            .init_resource::<Menu>()
            .init_resource::<Assets<Mesh>>()
            .add_systems(Update, hold);
        app.world_mut().resource_mut::<Model>().held = Some(HandModel::Item([9, 9, 9, 255]));
        app.world_mut().spawn((HandMount, Visibility::Inherited));
        app
    }

    fn held(app: &mut App) -> Vec<(Visibility, Vec3)> {
        app.world_mut()
            .query_filtered::<(&Visibility, &Transform), With<Held>>()
            .iter(app.world())
            .map(|(v, t)| (*v, t.scale))
            .collect()
    }

    #[test]
    fn the_item_hangs_on_the_right_hand_mount_with_the_first_person_models_mesh() {
        let mut app = app();
        let mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(Cuboid::default());
        app.world_mut().resource_mut::<Model>().mesh = mesh.clone();
        app.update();
        let (item, parent, shown) = app
            .world_mut()
            .query_filtered::<(&Mesh3d, &ChildOf, &Visibility), With<Held>>()
            .single(app.world())
            .map(|(m, p, v)| (m.0.clone(), p.parent(), *v))
            .unwrap();
        assert_eq!(item, mesh);
        assert!(app.world().get::<HandMount>(parent).is_some());
        assert_eq!(shown, Visibility::Inherited);
    }

    #[test]
    fn an_empty_hand_holds_nothing() {
        let mut app = app();
        app.world_mut().resource_mut::<Model>().held = Some(HandModel::Nothing);
        app.update();
        assert!(held(&mut app).is_empty());
    }

    #[test]
    fn the_item_hides_under_the_menu_and_when_dead_and_returns() {
        let mut app = app();
        app.update();
        assert_eq!(held(&mut app)[0].0, Visibility::Inherited);
        app.world_mut().resource_mut::<Menu>().open = true;
        app.update();
        assert_eq!(held(&mut app)[0].0, Visibility::Hidden, "Menu");
        app.world_mut().resource_mut::<Menu>().open = false;
        app.insert_resource(super::super::death::Dead::after_a_fall());
        app.update();
        assert_eq!(held(&mut app)[0].0, Visibility::Hidden, "dead");
        app.world_mut()
            .remove_resource::<super::super::death::Dead>();
        app.update();
        assert_eq!(held(&mut app)[0].0, Visibility::Inherited);
    }

    #[test]
    fn eating_shrinks_the_item_in_hand() {
        let mut app = app();
        app.init_resource::<HandAnim>();
        app.update();
        let full = held(&mut app)[0].1.x;
        app.world_mut().resource_mut::<HandAnim>().bite();
        for _ in 0..60 {
            app.world_mut().resource_mut::<HandAnim>().tick(0.01);
        }
        app.update();
        assert!(held(&mut app)[0].1.x < full * 0.95);
    }
}
