//! First-person look and semantic movement intents for the Server controller.

use bevy::prelude::*;

use crate::session::SessionApp;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use std::f32::consts::FRAC_PI_2;

use super::actions::{Action, Actions};
use crate::server::player::{MovementIntent, RespawnIntent};

/// Just inside ±90°, where the yaw/pitch basis degenerates.
const PITCH_LIMIT: f32 = FRAC_PI_2 - 1e-3;

/// Metres per second. The lower bound is a slow walk for inspecting a face; the
/// upper is a Cell every half second, past which Chunks cannot be meshed fast
/// enough to keep up once streaming exists.
const SPEED_RANGE: std::ops::RangeInclusive<f32> = 0.5..=64.0;

/// Multiplier per press, so the range above crosses in a dozen presses
/// regardless of where in it you start.
const SPEED_PER_PRESS: f32 = 1.5;

/// Radians per second at full right-stick deflection and sensitivity 1.
const PAD_TURN_RATE: f32 = 3.0;

/// The Player's camera. `With<Camera3d>` also matches the coarse-level cameras,
/// so anything that means "the camera" filters by this.
#[derive(Component)]
pub struct MainCamera;

#[derive(Component)]
pub struct FlyCamera {
    pub movement_mode: crate::server::player::MovementMode,
    /// Where the Server has the Player's Eye. The camera sits here in
    /// first-person and behind it in third.
    pub eye: Vec3,
    /// Metres per second, not Voxels per second. A speed in Voxels would be
    /// secretly calibrated to one Voxel size.
    /// See `docs/adr/0003-voxel-size-is-per-world.md`.
    pub speed: f32,
    /// Radians of rotation per pixel of mouse travel.
    pub sensitivity: f32,
    pub yaw: f32,
    pub pitch: f32,
}

impl Default for FlyCamera {
    fn default() -> Self {
        FlyCamera {
            movement_mode: default(),
            eye: Vec3::ZERO,
            speed: 12.0,
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
#[derive(Resource, Default)]
pub(super) struct Sprint(pub bool);
/// Controller latches: a thumb cannot hold a stick click while pushing that
/// stick, or hold B while steering look with the same thumb.
#[derive(Resource, Default)]
pub(super) struct PadLatch {
    pub sprint: bool,
    pub crouch: bool,
}

pub(super) fn plugin(app: &mut App) {
    app.init_session_state::<Sprint>()
        .init_session_state::<PadLatch>();
    app.add_session_systems(
        Update,
        grab_cursor
            .run_if(crate::server::persistence::playing)
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
    app.add_session_systems(Startup, spawn_hint)
        .add_session_systems(Update, input.run_if(crate::server::persistence::playing))
        .add_session_systems(Update, show_hint.after(super::input::resolve));
}

fn spawn_hint(mut commands: Commands) {
    commands.spawn((
        CursorHint,
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
    touch: Res<super::touch::TouchFrame>,
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
    text.set_if_neq(Text::new(if touch.active {
        "Touch the world to look around".to_owned()
    } else if labels.device == super::actions::Device::Pad {
        format!(
            "Press {} to play",
            labels.pad(bevy::input::gamepad::GamepadButton::South)
        )
    } else {
        format!(
            "Click to look around\n{} to release the cursor",
            bindings.label(Action::ReleaseCursor, &labels)
        )
    }));
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
    settings: Option<Res<super::settings::Settings>>,
    ui: Option<Res<super::input::UiInput>>,
    layout_menu: Option<Res<super::layout_menu::LayoutMenu>>,
) {
    let mut cursor = cursor.into_inner();
    if settings.is_some_and(|settings| settings.open)
        || ui.is_some_and(|ui| ui.text_entry)
        || layout_menu.is_some_and(|menu| menu.open)
    {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
        return;
    }
    if actions.cancel_requested() {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    } else if mouse.just_pressed(MouseButton::Left) {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
}

pub(super) fn look(
    preferences: Res<super::profile::Preferences>,
    ownership: Res<super::input::Ownership>,
    actions: Res<Actions>,
    time: Res<Time>,
    cursor: Single<&CursorOptions, With<PrimaryWindow>>,
    camera: Single<(&mut Transform, &mut FlyCamera)>,
) {
    let axes = &actions.axes;
    if !ownership.gameplay()
        || cursor.grab_mode == CursorGrabMode::None
        || (axes.look == Vec2::ZERO && axes.look_rate == Vec2::ZERO)
    {
        return;
    }

    let (mut transform, mut fly) = camera.into_inner();
    fly.sensitivity = preferences.sensitivity;
    let invert = |inverted| if inverted { -1.0 } else { 1.0 };
    // Mouse travel is already a distance; a stick is a rate, so elapsed time
    // makes it the same turn at any frame rate.
    let pad = axes.look_rate * PAD_TURN_RATE * preferences.pad.look_sensitivity * time.delta_secs();
    fly.yaw -= axes.look.x * fly.sensitivity + pad.x;
    fly.pitch = (fly.pitch
        - axes.look.y * fly.sensitivity * invert(preferences.invert_y)
        - pad.y * invert(preferences.pad.invert_y))
    .clamp(-PITCH_LIMIT, PITCH_LIMIT);
    transform.rotation = Quat::from_euler(EulerRot::YXZ, fly.yaw, fly.pitch, 0.0);
}

fn fly(
    mut commands: Commands,
    preferences: Res<super::profile::Preferences>,
    (mut sprint, mut latch): (ResMut<Sprint>, ResMut<PadLatch>),
    (ownership, body): (
        Res<super::input::Ownership>,
        Res<super::pose::PublishedBody>,
    ),
    actions: Res<Actions>,
    mut intents: MessageWriter<MovementIntent>,
    camera: Single<(&Transform, &FlyCamera)>,
) {
    let (transform, fly) = camera.into_inner();
    let playing = ownership.gameplay();
    let physics = fly.movement_mode == crate::server::player::MovementMode::Physics;
    let descend = actions.state(Action::Descend);
    let pad_crouch = preferences.pad.toggle_crouch && physics;
    // A toggled crouch is a pose, not a descent: the B press that started it
    // must not also count as holding Descend.
    let held_descend = descend.held && !(pad_crouch && descend.from_pad);
    let mut planar = Vec3::ZERO;
    for (key, axis) in [
        (Action::Forward, *transform.forward()),
        (Action::Backward, *transform.back()),
        (Action::Left, *transform.left()),
        (Action::Right, *transform.right()),
    ] {
        if playing && actions.held(key) {
            planar += axis;
        }
    }
    let mut vertical = Vec3::ZERO;
    if playing && actions.held(Action::Ascend) {
        vertical += Vec3::Y;
    }
    if playing && held_descend {
        vertical += Vec3::NEG_Y;
    }
    let stick = if playing {
        actions.axes.movement
    } else {
        Vec2::ZERO
    };
    let analog = *transform.right() * stick.x + *transform.forward() * stick.y;
    // Whole keyboard directions stay unit length so pitch and diagonals keep
    // full speed; only a stick sends less, which the Server reads as throttle.
    // On foot the Server ignores the vertical part, so a jump or crouch must
    // not turn a half-pushed stick into a full one.
    let direction = if planar != Vec3::ZERO || (vertical != Vec3::ZERO && !physics) {
        (planar + vertical + analog).normalize_or_zero()
    } else {
        analog
    };

    if !playing
        || actions.started(Action::Respawn)
        || actions.started(Action::MovementMode)
        || actions.started(Action::Ascend)
        || !pad_crouch
    {
        latch.crouch = false;
    }
    if playing && pad_crouch && descend.started && descend.from_pad {
        latch.crouch = !latch.crouch;
    }
    // Held Jump is a swim stroke, and the lift out of the water carries on until
    // the feet clear it; on land a hold must stay one hop per press.
    let stroking = physics && (body.0.swimming || (body.0.immersion.is_some() && !body.0.grounded));
    let sprint_state = actions.state(Action::Sprint);
    if playing && sprint_state.started && sprint_state.from_pad {
        latch.sprint = true;
    }
    let crouching = latch.crouch || (physics && held_descend);
    if !playing || actions.started(Action::Respawn) || stick == Vec2::ZERO || crouching {
        latch.sprint = false;
    }

    if !playing || actions.started(Action::Respawn) {
        sprint.0 = false;
    } else if preferences.toggle_sprint && sprint_state.started && !sprint_state.from_pad {
        sprint.0 = !sprint.0;
        super::menu_ui::notify(
            &mut commands,
            if sprint.0 {
                "Sprint enabled"
            } else {
                "Sprint disabled"
            },
            super::menu_ui::Tone::Change,
        );
    }
    if !preferences.toggle_sprint {
        sprint.0 = false;
    }
    let sprinting = latch.sprint
        || if preferences.toggle_sprint {
            sprint.0
        } else {
            actions.held(Action::Sprint)
        };
    intents.write(MovementIntent {
        cancelled: !playing || actions.started(Action::Respawn),
        direction,
        speed_mps: fly.speed,
        sprint: sprinting,
        crouch: playing && (held_descend || latch.crouch),
        yaw: fly.yaw,
        pitch: fly.pitch,
        jump: playing
            && (actions.started(Action::Ascend) || (stroking && actions.held(Action::Ascend))),
        switch_mode: playing && actions.started(Action::MovementMode),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::input::gamepad::GamepadButton;

    fn pad_app(mode: crate::server::player::MovementMode) -> App {
        let mut app = App::new();
        app.add_plugins(super::super::actions::test_plugin)
            .insert_resource(super::super::input::Ownership::playing())
            .add_message::<MovementIntent>()
            .init_resource::<super::super::pose::PublishedBody>()
            .add_systems(Update, fly);
        app.world_mut().spawn((
            Transform::default(),
            FlyCamera {
                movement_mode: mode,
                ..default()
            },
        ));
        app
    }
    /// One frame: buttons in `pressed` are down, everything else is up.
    fn frame(app: &mut App, pressed: &[GamepadButton], stick: Vec2) -> MovementIntent {
        let mut pad = app
            .world_mut()
            .resource_mut::<super::super::pad::PadInput>();
        pad.buttons.clear();
        for button in GamepadButton::all() {
            if pressed.contains(&button) {
                pad.buttons.press(button);
            } else {
                pad.buttons.release(button);
            }
        }
        pad.movement = stick;
        app.update();
        app.world_mut()
            .resource_mut::<Messages<MovementIntent>>()
            .drain()
            .last()
            .unwrap()
    }

    #[test]
    fn pad_crouch_toggles_on_foot_holds_in_flight_and_leaves_keyboard_hold_alone() {
        use crate::server::player::MovementMode;
        use GamepadButton::{East, South};
        let mut app = pad_app(MovementMode::Physics);
        assert!(frame(&mut app, &[East], Vec2::ZERO).crouch);
        let stood = frame(&mut app, &[], Vec2::ZERO);
        assert!(stood.crouch && stood.direction == Vec3::ZERO);
        assert!(frame(&mut app, &[], Vec2::ZERO).crouch);
        frame(&mut app, &[East], Vec2::ZERO);
        assert!(!frame(&mut app, &[], Vec2::ZERO).crouch);
        frame(&mut app, &[East], Vec2::ZERO);
        frame(&mut app, &[], Vec2::ZERO);
        let jump = frame(&mut app, &[South], Vec2::ZERO);
        assert!(jump.jump && !jump.crouch);

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ControlLeft);
        assert!(frame(&mut app, &[], Vec2::ZERO).crouch);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::ControlLeft);
        assert!(!frame(&mut app, &[], Vec2::ZERO).crouch);

        let mut app = pad_app(MovementMode::Flight);
        let held = frame(&mut app, &[East], Vec2::ZERO);
        assert!(held.crouch && held.direction == Vec3::NEG_Y);
        let released = frame(&mut app, &[], Vec2::ZERO);
        assert!(!released.crouch && released.direction == Vec3::ZERO);
    }

    #[test]
    fn jump_held_repeats_while_swimming_or_airborne_by_water_and_hops_once_per_press_on_land() {
        use crate::server::player::{BodyState, MovementMode};
        use GamepadButton::South;
        let mut app = pad_app(MovementMode::Physics);
        assert!(frame(&mut app, &[South], Vec2::ZERO).jump);
        assert!(!frame(&mut app, &[South], Vec2::ZERO).jump, "dry hold");
        frame(&mut app, &[], Vec2::ZERO);
        let fluid = Some(tesserae_core::immersion::FluidSurface {
            material: 1,
            surface_y_m: 0.0,
            dimming: 1,
        });
        for (body, repeats) in [
            (
                BodyState {
                    swimming: true,
                    ..default()
                },
                true,
            ),
            (
                BodyState {
                    immersion: fluid,
                    ..default()
                },
                true,
            ),
            (
                BodyState {
                    immersion: fluid,
                    grounded: true,
                    ..default()
                },
                false,
            ),
        ] {
            app.world_mut()
                .resource_mut::<super::super::pose::PublishedBody>()
                .0 = body;
            frame(&mut app, &[South], Vec2::ZERO);
            assert_eq!(
                frame(&mut app, &[South], Vec2::ZERO).jump,
                repeats,
                "{body:?}"
            );
            frame(&mut app, &[], Vec2::ZERO);
        }
    }

    #[test]
    fn a_stick_click_latches_sprint_until_the_stick_rests() {
        use crate::server::player::MovementMode;
        let mut app = pad_app(MovementMode::Physics);
        let forward = Vec2::new(0.0, 0.6);
        assert!(frame(&mut app, &[GamepadButton::LeftThumb], forward).sprint);
        let latched = frame(&mut app, &[], forward);
        assert!(latched.sprint);
        assert!((latched.direction.length() - 0.6).abs() < 1e-5);
        assert!(!frame(&mut app, &[], Vec2::ZERO).sprint);
        assert!(!frame(&mut app, &[], forward).sprint);

        let jump = frame(&mut app, &[GamepadButton::South], forward);
        assert!(jump.jump && (jump.direction.length() - 0.6).abs() < 1e-5);

        frame(&mut app, &[GamepadButton::LeftThumb], forward);
        app.world_mut()
            .resource_mut::<super::super::input::Ownership>()
            .owner = super::super::input::Owner::Catalogue;
        assert!(!frame(&mut app, &[], forward).sprint);
        app.world_mut()
            .resource_mut::<super::super::input::Ownership>()
            .owner = super::super::input::Owner::Gameplay;
        assert!(!frame(&mut app, &[], forward).sprint);
    }

    #[test]
    fn stick_look_turns_the_same_amount_at_every_frame_rate() {
        for hz in [60u32, 144, 1000] {
            let mut app = App::new();
            app.add_plugins((bevy::time::TimePlugin, super::super::actions::test_plugin))
                .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                    std::time::Duration::from_secs_f64(1.0 / f64::from(hz)),
                ))
                .insert_resource(super::super::input::Ownership::playing())
                .add_systems(Update, look);
            app.world_mut().spawn((
                PrimaryWindow,
                CursorOptions {
                    grab_mode: CursorGrabMode::Locked,
                    ..default()
                },
            ));
            let camera = app
                .world_mut()
                .spawn((Transform::default(), FlyCamera::default()))
                .id();
            app.update();
            app.world_mut()
                .resource_mut::<super::super::pad::PadInput>()
                .look = Vec2::new(1.0, 0.0);
            for _ in 0..hz {
                app.update();
            }
            let yaw = app.world().get::<FlyCamera>(camera).unwrap().yaw;
            assert!(
                (yaw + PAD_TURN_RATE).abs() <= PAD_TURN_RATE / 60.0,
                "{hz} Hz: {yaw}"
            );
        }
    }

    #[test]
    fn crouch_uses_the_remapped_descend_action_and_cancels_on_context_loss() {
        use super::super::actions::{Binding, Bindings, Modifier, Trigger};
        let mut app = App::new();
        app.add_plugins(super::super::actions::test_plugin)
            .insert_resource(super::super::input::Ownership::playing())
            .add_message::<MovementIntent>()
            .init_resource::<super::super::pose::PublishedBody>()
            .add_systems(Update, fly);
        app.world_mut()
            .spawn((Transform::default(), FlyCamera::default()));
        app.world_mut().resource_mut::<Bindings>().keyboard.insert(
            Action::Descend,
            [
                Some(Binding {
                    trigger: Trigger::Key(KeyCode::KeyC),
                    modifier: Modifier::None,
                }),
                None,
            ],
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyC);
        app.update();
        let intent = app
            .world_mut()
            .resource_mut::<Messages<MovementIntent>>()
            .drain()
            .last()
            .unwrap();
        assert!(intent.crouch);
        assert_eq!(intent.direction, Vec3::NEG_Y);
        app.world_mut()
            .resource_mut::<super::super::input::Ownership>()
            .owner = super::super::input::Owner::Catalogue;
        app.update();
        let intent = app
            .world_mut()
            .resource_mut::<Messages<MovementIntent>>()
            .drain()
            .last()
            .unwrap();
        assert!(intent.cancelled);
        assert!(!intent.crouch);
        assert_eq!(intent.direction, Vec3::ZERO);
    }

    #[test]
    fn toggle_sprint_survives_a_stop_and_clears_on_menu_and_respawn() {
        let mut app = App::new();
        app.add_plugins(super::super::actions::test_plugin)
            .insert_resource(super::super::input::Ownership::playing())
            .init_resource::<Time>()
            .add_message::<MovementIntent>()
            .init_resource::<super::super::pose::PublishedBody>()
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
        let toasts: Vec<_> = app
            .world_mut()
            .query::<&Text>()
            .iter(app.world())
            .map(|text| text.0.clone())
            .collect();
        assert_eq!(toasts, ["Sprint enabled"]);
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
            .init_resource::<super::super::pose::PublishedBody>()
            .add_message::<RespawnIntent>()
            .add_message::<MovementIntent>()
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
        assert_eq!(
            app.world_mut()
                .resource_mut::<Messages<MovementIntent>>()
                .drain()
                .last()
                .unwrap()
                .direction,
            Vec3::ZERO
        );
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
        assert_eq!(
            app.world_mut()
                .resource_mut::<Messages<MovementIntent>>()
                .drain()
                .last()
                .unwrap()
                .direction,
            Vec3::ZERO
        );
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
        let intent = app
            .world_mut()
            .resource_mut::<Messages<MovementIntent>>()
            .drain()
            .last()
            .unwrap();
        assert!(intent.direction.z < 0.0);
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
            .add_message::<MovementIntent>()
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
    fn the_reset_key_asks_the_server_and_turns_the_camera_but_does_not_move_it() {
        let mut app = App::new();
        app.add_plugins(super::super::actions::test_plugin);
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .insert_resource(super::super::input::Ownership::playing())
            .add_message::<RespawnIntent>()
            .add_message::<MovementIntent>()
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
