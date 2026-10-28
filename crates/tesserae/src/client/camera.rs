//! A fly camera: WASD to move, mouse to look, shift to sprint, `-` and `=` to
//! change speed, R to return to the spawn point, Esc to release the cursor.
//!
//! Hand-rolled rather than a dependency so the Client's one piece of input
//! handling stays small enough to read, and so it is already expressed in
//! metres per second when the player controller replaces it.

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use std::f32::consts::FRAC_PI_2;

use super::actions::{Action, Actions};
use crate::server::player::{FLY_SPEED_CEILING_MPS, RespawnIntent};

/// Just inside ±90°, where the yaw/pitch basis degenerates.
const PITCH_LIMIT: f32 = FRAC_PI_2 - 1e-3;

/// Metres per second. The lower bound is a slow walk for inspecting a face; the
/// upper is a Cell every half second, past which Chunks cannot be meshed fast
/// enough to keep up once streaming exists.
const SPEED_RANGE: std::ops::RangeInclusive<f32> = 0.5..=64.0;

/// Multiplier per press, so the range above crosses in a dozen presses
/// regardless of where in it you start.
const SPEED_PER_PRESS: f32 = 1.5;

/// The Player's camera. `With<Camera3d>` also matches the coarse-level cameras,
/// so anything that means "the camera" filters by this.
#[derive(Component)]
pub struct MainCamera;

#[derive(Component)]
pub struct FlyCamera {
    /// Metres per second, not Voxels per second. A speed in Voxels would be
    /// secretly calibrated to one Voxel size.
    /// See `docs/adr/0003-voxel-size-is-per-world.md`.
    pub speed: f32,
    pub sprint_multiplier: f32,
    /// Radians of rotation per pixel of mouse travel.
    pub sensitivity: f32,
    pub yaw: f32,
    pub pitch: f32,
}

impl Default for FlyCamera {
    fn default() -> Self {
        FlyCamera {
            speed: 12.0,
            sprint_multiplier: 5.0,
            sensitivity: 0.003,
            yaw: 0.0,
            pitch: 0.0,
        }
    }
}

impl FlyCamera {
    /// Derives yaw and pitch from a transform so the camera can be spawned with
    /// `looking_at` and not snap on the first frame.
    pub fn looking_along(direction: Vec3) -> Self {
        let d = direction.normalize_or_zero();
        FlyCamera {
            yaw: (-d.x).atan2(-d.z),
            pitch: d.y.asin().clamp(-PITCH_LIMIT, PITCH_LIMIT),
            ..default()
        }
    }
}

/// How [`reset`] turns the camera back. Only the orientation: where the camera
/// goes is the Server's spawn eye, which it sends back as a correction.
#[derive(Component, Clone, Copy)]
pub struct CameraHome(pub Transform);

/// Shown whenever the cursor is free. Mouse look is gated on a click and nothing
/// else on screen says so, which reads as the mouse not working at all.
#[derive(Component)]
struct CursorHint;
#[derive(Component)]
struct SprintHint;
#[derive(Resource, Default)]
pub(super) struct Sprint(pub bool);

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Sprint>();
    app.add_systems(
        Update,
        grab_cursor
            .run_if(super::menu::closed)
            .after(super::menu::toggle),
    );
    let input = (
        reset,
        look.run_if(super::input::gameplay),
        adjust_speed,
        fly,
    )
        .chain()
        .after(super::input::resolve);
    #[cfg(feature = "dev")]
    let input = input.run_if(|| std::env::var_os("TESSERAE_SCREENSHOT").is_none());
    app.add_systems(Startup, spawn_hint)
        .add_systems(Update, input.run_if(crate::server::persistence::playing))
        .add_systems(Update, (show_hint.after(super::input::resolve), sprint_hint.after(fly)));

}

fn spawn_hint(mut commands: Commands) {
    commands.spawn((
        SprintHint,
        Visibility::Hidden,
        Pickable::IGNORE,
        Text::new("Sprint enabled"),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(16.0),
            left: Val::Percent(45.0),
            ..default()
        },
        TextFont {
            font_size: bevy::text::FontSize::Px(14.0),
            ..default()
        },
    ));
    commands.spawn((
        CursorHint,
        // ASCII only: Bevy's embedded default font is a FiraMono subset and
        // renders anything outside it, a middle dot included, as tofu.
        Text::new("Click to look around\nEsc to release the cursor"),
        TextFont {
            font_size: bevy::text::FontSize::Px(14.0),
            ..default()
        },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.7)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(16.0),
            bottom: Val::Px(16.0),
            ..default()
        },
    ));
}

fn show_hint(
    cursor: Single<&CursorOptions, With<PrimaryWindow>>,
    hint: Single<(&mut Visibility, &mut Text), With<CursorHint>>,
    bindings: Res<super::actions::Bindings>,
    labels: Res<super::actions::Labels>,
) {
    let wanted = if cursor.grab_mode == CursorGrabMode::None {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    // Through change detection rather than unconditionally, so the UI is not
    // re-laid out every frame.
    let (mut visibility, mut text) = hint.into_inner();
    visibility.set_if_neq(wanted);
    text.set_if_neq(Text::new(format!(
        "Click to look around\n{} to release the cursor",
        bindings.label(Action::ReleaseCursor, &labels)
    )));
}

fn reset(
    ownership: Res<super::input::Ownership>,
    actions: Res<Actions>,
    camera: Single<(&mut Transform, &mut FlyCamera, &CameraHome)>,
    mut respawn: MessageWriter<RespawnIntent>,
) {
    if !ownership.gameplay() || !actions.started(Action::Respawn) {
        return;
    }
    // Asked, not done: moving the camera here would be a teleport the Server
    // clamps. Its reply puts the camera at the spawn eye.
    respawn.write(RespawnIntent);
    let (mut transform, mut fly, home) = camera.into_inner();
    transform.rotation = home.0.rotation;
    let restored = FlyCamera::looking_along(*home.0.forward());
    fly.yaw = restored.yaw;
    fly.pitch = restored.pitch;
}

/// Keys change how fast you fly rather than the field of view: in a fly camera
/// the distance to what you are looking at is the thing you actually want to
/// control, and W/S already do that. The scroll wheel picks a hotbar slot.
fn adjust_speed(
    ownership: Res<super::input::Ownership>,
    actions: Res<Actions>,
    cursor: Single<&CursorOptions, With<PrimaryWindow>>,
    mut fly: Single<&mut FlyCamera>,
) {
    if cursor.grab_mode == CursorGrabMode::None {
        return;
    }
    if !ownership.gameplay() {
        return;
    }
    let presses = actions.started(Action::Faster) as i32 - actions.started(Action::Slower) as i32;
    if presses != 0 {
        fly.speed = (fly.speed * SPEED_PER_PRESS.powi(presses))
            .clamp(*SPEED_RANGE.start(), *SPEED_RANGE.end());
    }
}

pub(super) fn grab_cursor(
    mouse: Res<ButtonInput<MouseButton>>,
    actions: Res<Actions>,
    cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
    panels: Option<Res<super::help::Panels>>,
    settings: Option<Res<super::settings::Settings>>,
    ui: Option<Res<super::input::UiInput>>,
) {
    let mut cursor = cursor.into_inner();
    if settings.is_some_and(|settings| settings.open) || ui.is_some_and(|ui| ui.text_entry) {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
        return;
    }
    if actions.cancel_requested() {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    } else if mouse.just_pressed(MouseButton::Left)
        && !panels.is_some_and(|panels| panels.pointer_over_info)
    {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
}

pub(super) fn look(
    preferences: Res<super::profile::Preferences>,
    ownership: Res<super::input::Ownership>,
    motion: Res<AccumulatedMouseMotion>,
    cursor: Single<&CursorOptions, With<PrimaryWindow>>,
    camera: Single<(&mut Transform, &mut FlyCamera)>,
) {
    if !ownership.gameplay()
        || cursor.grab_mode == CursorGrabMode::None
        || motion.delta == Vec2::ZERO
    {
        return;
    }

    let (mut transform, mut fly) = camera.into_inner();
    fly.sensitivity = preferences.sensitivity;
    fly.yaw -= motion.delta.x * fly.sensitivity;
    fly.pitch = (fly.pitch
        - motion.delta.y * fly.sensitivity * if preferences.invert_y { -1.0 } else { 1.0 })
    .clamp(-PITCH_LIMIT, PITCH_LIMIT);
    transform.rotation = Quat::from_euler(EulerRot::YXZ, fly.yaw, fly.pitch, 0.0);
}

/// Metres per second. Held to the Server's ceiling: it clamps a Player faster
/// than that, so flying past it only rubber-bands.
fn flight_speed(fly: &FlyCamera, sprinting: bool) -> f32 {
    let speed = if sprinting {
        fly.speed * fly.sprint_multiplier
    } else {
        fly.speed
    };
    speed.min(FLY_SPEED_CEILING_MPS)
}

fn fly(
    preferences: Res<super::profile::Preferences>,
    mut sprint: ResMut<Sprint>,
    ownership: Res<super::input::Ownership>,
    actions: Res<Actions>,
    time: Res<Time>,
    camera: Single<(&mut Transform, &FlyCamera)>,
) {
    let (mut transform, fly) = camera.into_inner();

    let mut direction = Vec3::ZERO;
    for (key, axis) in [
        (Action::Forward, *transform.forward()),
        (Action::Backward, *transform.back()),
        (Action::Left, *transform.left()),
        (Action::Right, *transform.right()),
        (Action::Ascend, Vec3::Y),
        (Action::Descend, Vec3::NEG_Y),
    ] {
        if ownership.gameplay() && actions.held(key) {
            direction += axis;
        }
    }

    if !ownership.gameplay() || actions.started(Action::Respawn) {
        sprint.0 = false;
    } else if preferences.toggle_sprint && actions.started(Action::Sprint) {
        sprint.0 = !sprint.0;
    }
    if !preferences.toggle_sprint {
        sprint.0 = false;
    }
    let sprinting = if preferences.toggle_sprint {
        sprint.0
    } else {
        actions.held(Action::Sprint)
    };
    let speed = flight_speed(fly, sprinting);
    transform.translation += direction.normalize_or_zero() * speed * time.delta_secs();
}

fn sprint_hint(
    sprint: Res<Sprint>,
    preferences: Res<super::profile::Preferences>,
    mut hints: Query<&mut Visibility, With<SprintHint>>,
) {
    for mut hint in &mut hints {
        hint.set_if_neq(if preferences.toggle_sprint && sprint.0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_sprint_survives_a_stop_and_clears_on_menu_and_respawn() {
        let mut app = App::new();
        app.add_plugins(super::super::actions::test_plugin)
            .insert_resource(super::super::input::Ownership::playing())
            .init_resource::<Time>()
            .add_systems(Update, fly);
        app.world_mut()
            .spawn((Transform::default(), FlyCamera::default()));
        app.world_mut()
            .resource_mut::<super::super::profile::Preferences>()
            .toggle_sprint = true;
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ShiftRight);
        app.update();
        assert!(app.world().resource::<Sprint>().0);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::ShiftRight);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.update();
        assert!(app.world().resource::<Sprint>().0);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyR);
        app.update();
        assert!(!app.world().resource::<Sprint>().0);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release_all();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ShiftLeft);
        app.update();
        assert!(app.world().resource::<Sprint>().0);
        app.world_mut()
            .resource_mut::<super::super::input::Ownership>()
            .owner = super::super::input::Owner::Catalogue;
        app.update();
        assert!(!app.world().resource::<Sprint>().0);
    }

    #[test]
    fn same_frame_menu_transitions_cancel_movement_and_respawn_until_release() {
        let mut app = App::new();
        app.add_plugins(super::super::actions::test_plugin)
            .init_resource::<super::super::menu::Menu>()
            .init_resource::<super::super::menu::PointerState>()
            .insert_resource(super::super::input::Ownership::playing())
            .init_resource::<Time>()
            .add_message::<RespawnIntent>()
            .add_systems(
                Update,
                (
                    super::super::menu::toggle,
                    super::super::input::resolve,
                    reset,
                    fly,
                )
                    .chain(),
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
        let camera = app
            .world_mut()
            .spawn((
                Transform::default(),
                FlyCamera::default(),
                CameraHome(Transform::default()),
            ))
            .id();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(1.0 / 60.0));
        for key in [KeyCode::KeyW, KeyCode::KeyR, KeyCode::Tab] {
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(key);
        }
        app.update();
        assert_eq!(
            app.world().get::<Transform>(camera).unwrap().translation,
            Vec3::ZERO
        );
        assert!(app.world().resource::<Messages<RespawnIntent>>().is_empty());
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::Tab);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Tab);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.update();
        assert_eq!(
            app.world().get::<Transform>(camera).unwrap().translation,
            Vec3::ZERO
        );
        assert!(app.world().resource::<Messages<RespawnIntent>>().is_empty());
        for key in [KeyCode::KeyW, KeyCode::KeyR] {
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .release(key);
        }
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.update();
        for key in [KeyCode::KeyW, KeyCode::KeyR] {
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(key);
        }
        app.update();
        assert!(app.world().get::<Transform>(camera).unwrap().translation.z < 0.0);
        assert_eq!(app.world().resource::<Messages<RespawnIntent>>().len(), 1);
    }

    #[test]
    fn focus_return_requires_a_fresh_respawn_activation() {
        let mut app = App::new();
        app.add_plugins(super::super::actions::test_plugin);
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<super::super::menu::Menu>()
            .insert_resource(super::super::input::Ownership::playing())
            .add_message::<RespawnIntent>()
            .add_systems(Update, (super::super::input::resolve, reset).chain());
        let window = app
            .world_mut()
            .spawn((
                Window {
                    focused: false,
                    ..default()
                },
                PrimaryWindow,
                CursorOptions {
                    grab_mode: CursorGrabMode::Locked,
                    ..default()
                },
            ))
            .id();
        app.world_mut().spawn((
            Transform::default(),
            FlyCamera::default(),
            CameraHome(Transform::default()),
        ));
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyR);
        app.update();
        app.world_mut()
            .entity_mut(window)
            .get_mut::<Window>()
            .unwrap()
            .focused = true;
        app.update();
        app.update();
        assert!(app.world().resource::<Messages<RespawnIntent>>().is_empty());
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::KeyR);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyR);
        app.update();
        assert_eq!(app.world().resource::<Messages<RespawnIntent>>().len(), 1);
    }

    #[test]
    fn looking_along_round_trips_through_the_rotation_it_implies() {
        for direction in [
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(-3.0, -2.0, -4.0),
            Vec3::new(2.0, 5.0, 1.0),
        ] {
            let fly = FlyCamera::looking_along(direction);
            let rotation = Quat::from_euler(EulerRot::YXZ, fly.yaw, fly.pitch, 0.0);
            let forward = rotation * Vec3::NEG_Z;
            assert!(
                forward.abs_diff_eq(direction.normalize(), 1e-5),
                "{direction} became {forward}"
            );
        }
    }

    #[test]
    fn flight_never_outruns_the_servers_ceiling() {
        let mut fly = FlyCamera::default();
        for speed in [*SPEED_RANGE.start(), 12.0, *SPEED_RANGE.end()] {
            fly.speed = speed;
            for sprinting in [false, true] {
                assert!(flight_speed(&fly, sprinting) <= FLY_SPEED_CEILING_MPS);
            }
        }
        fly.speed = 12.0;
        assert_eq!(flight_speed(&fly, false), 12.0, "slow flight is untouched");
        assert_eq!(
            flight_speed(&fly, true),
            60.0,
            "and so is a sprint under it"
        );
        fly.speed = 20.0;
        assert_eq!(flight_speed(&fly, true), FLY_SPEED_CEILING_MPS);
    }

    #[test]
    fn the_reset_key_asks_the_server_and_turns_the_camera_but_does_not_move_it() {
        let mut app = App::new();
        app.add_plugins(super::super::actions::test_plugin);
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .insert_resource(super::super::input::Ownership::playing())
            .add_message::<RespawnIntent>()
            .add_systems(Update, reset);
        let home = Transform::from_xyz(0.0, 50.0, 0.0).looking_to(Vec3::NEG_Z, Vec3::Y);
        let away =
            Transform::from_xyz(300.0, 9.0, -40.0).looking_to(Vec3::new(1.0, 0.0, 0.0), Vec3::Y);
        app.world_mut()
            .spawn((away, FlyCamera::default(), CameraHome(home)));

        app.update();
        assert_eq!(
            app.world().resource::<Messages<RespawnIntent>>().len(),
            0,
            "nothing pressed"
        );

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyR);
        app.update();

        assert_eq!(app.world().resource::<Messages<RespawnIntent>>().len(), 1);
        let transform = app
            .world_mut()
            .query_filtered::<&Transform, With<FlyCamera>>()
            .single(app.world())
            .unwrap();
        assert_eq!(
            transform.translation, away.translation,
            "the Server moves it"
        );
        assert!(transform.rotation.abs_diff_eq(home.rotation, 1e-6));
    }

    #[test]
    fn pitch_stays_inside_the_basis() {
        let straight_up = FlyCamera::looking_along(Vec3::Y);
        assert!(straight_up.pitch.abs() <= PITCH_LIMIT);
    }
}
