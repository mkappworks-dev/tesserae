//! The first-person viewmodel: the Player's arm at the lower right of the view,
//! with the held Item in its hand, as Minecraft draws it. It is a child of the
//! camera on its own render layer, drawn by the hand camera after the World with
//! a cleared depth, so it never clips into a wall. The hand animation
//! (`hand_anim.rs`) poses it: a swing for a Place, a repeating one while a
//! removal runs, a raise to the mouth with bites for eating, a lower-and-raise
//! on a slot change. Walking adds a small bob; reduced motion holds it still.

use super::{
    hand_anim::{HandAnim, Pose, still},
    menu::Covers,
    pose::PublishedBody,
    preview::{HAND_LAYER, HandModel, Model},
    profile::Preferences,
    theme,
};
use crate::session::SessionApp;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

use super::camera::FlyCamera;

/// The fist's place in the camera's frame, at rest.
const FIST: Vec3 = Vec3::new(0.25, -0.19, -0.55);
/// From the fist back along the forearm, toward the shoulder below the view.
const FOREARM: Vec3 = Vec3::new(0.45, -0.55, 1.0);
const ARM_LENGTH_M: f32 = 0.42;
const ARM_WIDTH_M: f32 = 0.1;
/// The edge of the held cube.
const ITEM_M: f32 = 0.09;
/// Where the held Item sits against the fist, in the root's frame.
const ITEM_AT: Vec3 = Vec3::new(-0.01, 0.06, -0.03);
const BOB_X_M: f32 = 0.010;
const BOB_Y_M: f32 = 0.014;
/// Bob phase per metre walked, and how fast the bob fades in and out.
const BOB_PER_M: f32 = 2.2;
const BOB_EASE: f32 = 8.0;
/// A walk at this speed bobs fully.
const BOB_FULL_MPS: f32 = 4.5;

#[derive(Component)]
struct Viewmodel;
#[derive(Component)]
struct Arm;
#[derive(Component)]
struct Held;

pub(super) fn plugin(app: &mut App) {
    app.add_session_systems(
        Update,
        (build, hold, animate)
            .chain()
            .after(super::preview::model)
            .after(super::hand_anim::animate),
    );
}

/// The arm's mesh: a box whose far end is the fist at the origin, running back
/// along [`FOREARM`], shaded by face so it reads as a solid under no light.
fn arm_mesh() -> Mesh {
    let toward = FOREARM.normalize();
    let mut mesh = Mesh::from(Cuboid::new(ARM_WIDTH_M, ARM_WIDTH_M, ARM_LENGTH_M))
        .translated_by(Vec3::new(0.0, 0.0, ARM_LENGTH_M / 2.0))
        .rotated_by(Quat::from_rotation_arc(Vec3::Z, toward));
    let shading: Vec<[f32; 4]> = mesh
        .attribute(Mesh::ATTRIBUTE_NORMAL)
        .and_then(|normals| normals.as_float3())
        .map(|normals| {
            normals
                .iter()
                .map(|n| {
                    let shade = 0.62 + 0.38 * (n[0] * 0.3 + n[1] * 0.8 + n[2] * 0.5).max(0.0);
                    [shade, shade, shade, 1.0]
                })
                .collect()
        })
        .unwrap_or_default();
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, shading);
    mesh
}

fn build(
    mut commands: Commands,
    camera: Query<Entity, With<FlyCamera>>,
    existing: Query<(), With<Viewmodel>>,
    (mut meshes, mut materials): (ResMut<Assets<Mesh>>, ResMut<Assets<StandardMaterial>>),
) {
    let Ok(parent) = camera.single() else {
        return;
    };
    if !existing.is_empty() {
        return;
    }
    let root = commands
        .spawn((
            Viewmodel,
            Transform::from_translation(FIST),
            Visibility::Hidden,
            RenderLayers::layer(HAND_LAYER),
            ChildOf(parent),
        ))
        .id();
    commands.spawn((
        Arm,
        bevy::light::NotShadowCaster,
        Mesh3d(meshes.add(arm_mesh())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: theme::VIEWMODEL_ARM,
            unlit: true,
            ..default()
        })),
        Transform::default(),
        RenderLayers::layer(HAND_LAYER),
        ChildOf(root),
    ));
}

/// The held cube at `scale` of full size, tilted as if gripped, its cell centre
/// at the grip.
fn item_at(scale: f32) -> Transform {
    let tilt = Quat::from_euler(EulerRot::YXZ, -0.55, 0.3, 0.12);
    let edge = ITEM_M * scale;
    Transform::from_translation(ITEM_AT - tilt * Vec3::splat(edge / 2.0))
        .with_rotation(tilt)
        .with_scale(Vec3::splat(edge))
}

/// Draws the Item in hand, replacing it when the model changes: not before a
/// slot switch has lowered the old one.
fn hold(
    mut commands: Commands,
    model: Res<Model>,
    anim: Option<Res<HandAnim>>,
    root: Query<Entity, With<Viewmodel>>,
    items: Query<Entity, With<Held>>,
    fresh: Query<(), Added<Viewmodel>>,
    mut pending: Local<bool>,
) {
    let Ok(root) = root.single() else {
        return;
    };
    *pending |= model.is_changed() || !fresh.is_empty();
    if !*pending || !anim.is_none_or(|anim| anim.swapped()) {
        return;
    }
    *pending = false;
    for item in &items {
        commands.entity(item).despawn();
    }
    if matches!(model.held, None | Some(HandModel::Nothing)) {
        return;
    }
    commands.spawn((
        Held,
        bevy::light::NotShadowCaster,
        Mesh3d(model.mesh.clone()),
        MeshMaterial3d(model.hand.clone()),
        item_at(1.0),
        RenderLayers::layer(HAND_LAYER),
        ChildOf(root),
    ));
}

/// How far the view bobs at `phase` of a step, `amount` of full.
fn bob_offset(phase: f32, amount: f32) -> Vec3 {
    Vec3::new(
        phase.sin() * BOB_X_M,
        -(phase * 2.0).sin().abs() * BOB_Y_M,
        0.0,
    ) * amount
}

/// Where the viewmodel's root sits: at rest, moved by the hand's pose and the bob.
fn root_at(pose: Pose, bob: Vec3) -> Transform {
    Transform::from_translation(FIST + pose.offset + bob)
        .with_rotation(Quat::from_rotation_x(pose.pitch))
}

type Inputs<'w> = (
    Option<Res<'w, HandAnim>>,
    Option<Res<'w, Preferences>>,
    Option<Res<'w, PublishedBody>>,
);

fn animate(
    time: Res<Time>,
    (anim, preferences, body): Inputs,
    (covers, dead): (Covers, Option<Res<super::death::Dead>>),
    mut root: Query<(&mut Transform, &mut Visibility), With<Viewmodel>>,
    mut item: Query<&mut Transform, (With<Held>, Without<Viewmodel>)>,
    mut walk: Local<(f32, f32)>,
) {
    let Ok((mut transform, mut visibility)) = root.single_mut() else {
        return;
    };
    visibility.set_if_neq(
        if super::menu::covered(&covers) || !super::death::alive(dead) {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        },
    );
    let still = still(&preferences);
    let pose = anim.map_or(Pose::REST, |anim| anim.pose(still));
    let dt = time.delta_secs();
    let (speed, grounded) = body.map_or((0.0, false), |b| (b.0.horizontal_mps, b.0.grounded));
    let target = if still || !grounded {
        0.0
    } else {
        (speed / BOB_FULL_MPS).clamp(0.0, 1.0)
    };
    let (phase, amount) = &mut *walk;
    *phase = (*phase + speed * BOB_PER_M * dt).rem_euclid(std::f32::consts::TAU);
    *amount += (target - *amount) * (1.0 - (-BOB_EASE * dt).exp());
    let next = root_at(pose, bob_offset(*phase, *amount));
    if *transform != next {
        *transform = next;
    }
    for mut held in &mut item {
        let next = item_at(pose.scale);
        if *held != next {
            *held = next;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::menu::Menu;
    use std::time::Duration;

    const BLOCK: Option<HandModel> = Some(HandModel::Item([200, 40, 40, 255]));

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                Duration::from_secs_f32(1.0 / 60.0),
            ))
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Model>()
            .init_resource::<HandAnim>()
            .add_systems(Update, (build, hold, animate).chain());
        app.world_mut().spawn(FlyCamera::default());
        app.world_mut().resource_mut::<Model>().held = BLOCK;
        app
    }

    fn root(app: &mut App) -> (Transform, Visibility) {
        let (t, v) = app
            .world_mut()
            .query_filtered::<(&Transform, &Visibility), With<Viewmodel>>()
            .single(app.world())
            .unwrap();
        (*t, *v)
    }

    fn held(app: &mut App) -> usize {
        app.world_mut()
            .query_filtered::<(), With<Held>>()
            .iter(app.world())
            .count()
    }

    #[test]
    fn the_arm_hangs_at_the_lower_right_on_its_own_layer_with_the_item_in_hand() {
        let mut app = app();
        app.update();
        app.update();
        let (transform, visibility) = root(&mut app);
        assert!(transform.translation.x > 0.0 && transform.translation.y < 0.0);
        assert_eq!(visibility, Visibility::Inherited);
        assert_eq!(held(&mut app), 1);
        let layers = app
            .world_mut()
            .query_filtered::<&RenderLayers, Or<(With<Viewmodel>, With<Arm>, With<Held>)>>()
            .iter(app.world())
            .count();
        assert_eq!(layers, 3, "root, arm and item are on the hand layer");

        app.world_mut().resource_mut::<Model>().held = Some(HandModel::Nothing);
        app.update();
        assert_eq!(held(&mut app), 0, "an empty hand is the bare arm");
        let arms = app
            .world_mut()
            .query_filtered::<(), With<Arm>>()
            .iter(app.world())
            .count();
        assert_eq!(arms, 1);
    }

    #[test]
    fn a_swing_moves_the_arm_forward_and_reduced_motion_holds_it_at_rest() {
        let mut app = app();
        app.update();
        let rest = root(&mut app).0.translation;
        app.world_mut().resource_mut::<HandAnim>().swing();
        app.world_mut().resource_mut::<HandAnim>().tick(0.1);
        app.update();
        assert!(root(&mut app).0.translation.z < rest.z - 0.05);

        app.insert_resource(Preferences {
            reduced_motion: true,
            ..default()
        });
        app.update();
        assert!(root(&mut app).0.translation.abs_diff_eq(rest, 1e-4));
    }

    #[test]
    fn eating_raises_the_food_toward_the_face_and_shrinks_it() {
        let mut app = app();
        app.update();
        let rest = root(&mut app).0.translation;
        app.world_mut().resource_mut::<HandAnim>().bite();
        for _ in 0..40 {
            app.world_mut().resource_mut::<HandAnim>().tick(0.01);
        }
        app.update();
        let raised = root(&mut app).0.translation;
        assert!(
            raised.y > rest.y + 0.2 && raised.x < rest.x,
            "{raised} from {rest}"
        );
        let edge = app
            .world_mut()
            .query_filtered::<&Transform, With<Held>>()
            .single(app.world())
            .unwrap()
            .scale
            .x;
        assert!(edge < ITEM_M * 0.95, "the food shrank: {edge}");
    }

    #[test]
    fn a_real_eat_the_server_confirms_raises_the_arm_through_the_whole_chain() {
        use crate::server::{tool::UseOutcome, vitals::EatIntent};
        use tesserae_core::inventory::Stack;
        let mut app = app();
        app.add_message::<EatIntent>()
            .add_message::<UseOutcome>()
            .add_message::<crate::server::edit::EditApplied>()
            .init_resource::<crate::client::edit::Hotbar>()
            .add_systems(
                Update,
                (
                    crate::client::hand_anim::confirm,
                    crate::client::hand_anim::animate,
                )
                    .chain()
                    .before(build),
            );
        app.world_mut()
            .resource_mut::<crate::client::edit::Hotbar>()
            .stacks[0] = Some(Stack {
            item: "m:melon".into(),
            count: 3,
        });
        app.update();
        let rest = root(&mut app).0.translation;
        app.world_mut().write_message(EatIntent);
        app.update();
        app.world_mut()
            .resource_mut::<crate::client::edit::Hotbar>()
            .stacks[0]
            .as_mut()
            .unwrap()
            .count = 2;
        let mut highest = rest.y;
        for _ in 0..40 {
            app.update();
            highest = highest.max(root(&mut app).0.translation.y);
        }
        assert!(
            highest > rest.y + 0.15,
            "the arm rose from {} to {highest}",
            rest.y
        );
    }

    #[test]
    fn a_slot_switch_lowers_the_item_and_swaps_it_at_the_bottom() {
        let mut app = app();
        app.update();
        let old = app.world().resource::<Model>().mesh.clone();
        let new = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(Cuboid::default());
        app.world_mut().resource_mut::<HandAnim>().switch();
        app.world_mut().resource_mut::<Model>().mesh = new.clone();
        app.update();
        let shown = |app: &mut App| {
            app.world_mut()
                .query_filtered::<&Mesh3d, With<Held>>()
                .single(app.world())
                .unwrap()
                .0
                .clone()
        };
        assert_eq!(
            shown(&mut app),
            old,
            "the old item is still on its way down"
        );
        for _ in 0..13 {
            app.world_mut().resource_mut::<HandAnim>().tick(0.01);
        }
        app.update();
        assert_eq!(shown(&mut app), new, "swapped once lowered");
    }

    #[test]
    fn the_viewmodel_hides_under_the_menu_settings_inventory_and_death() {
        let mut app = app();
        app.init_resource::<Menu>()
            .init_resource::<crate::client::settings::Settings>()
            .init_resource::<crate::client::inventory_window::InventoryWindow>();
        app.update();
        assert_eq!(root(&mut app).1, Visibility::Inherited);
        app.world_mut().resource_mut::<Menu>().open = true;
        app.update();
        assert_eq!(root(&mut app).1, Visibility::Hidden, "Menu");
        app.world_mut().resource_mut::<Menu>().open = false;
        app.world_mut()
            .resource_mut::<crate::client::settings::Settings>()
            .open = true;
        app.update();
        assert_eq!(root(&mut app).1, Visibility::Hidden, "Settings");
        app.world_mut()
            .resource_mut::<crate::client::settings::Settings>()
            .open = false;
        app.world_mut()
            .resource_mut::<crate::client::inventory_window::InventoryWindow>()
            .open = true;
        app.update();
        assert_eq!(root(&mut app).1, Visibility::Hidden, "Inventory");
        app.world_mut()
            .resource_mut::<crate::client::inventory_window::InventoryWindow>()
            .open = false;
        app.insert_resource(crate::client::death::Dead::after_a_fall());
        app.update();
        assert_eq!(root(&mut app).1, Visibility::Hidden, "dead");
    }

    #[test]
    fn walking_bobs_the_view_and_stillness_or_reduced_motion_does_not() {
        assert_eq!(bob_offset(1.3, 0.0), Vec3::ZERO);
        let widest = (0..64)
            .map(|n| bob_offset(n as f32 * 0.1, 1.0))
            .fold(Vec3::ZERO, |a, b| a.max(b.abs()));
        assert!(widest.x > 0.005 && widest.x <= BOB_X_M);
        assert!(widest.y > 0.005 && widest.y <= BOB_Y_M);

        let mut app = app();
        app.insert_resource(PublishedBody(crate::server::player::BodyState {
            grounded: true,
            horizontal_mps: 4.5,
            ..default()
        }));
        let mut moved = false;
        let rest = FIST;
        for _ in 0..90 {
            app.update();
            moved |= !root(&mut app).0.translation.abs_diff_eq(rest, 1e-4);
        }
        assert!(moved, "the walk bobs");

        app.insert_resource(Preferences {
            reduced_motion: true,
            ..default()
        });
        for _ in 0..60 {
            app.update();
        }
        assert!(
            root(&mut app).0.translation.abs_diff_eq(rest, 1e-3),
            "no bob when still"
        );
    }
}
