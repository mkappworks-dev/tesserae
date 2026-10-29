//! Controller adapter: one active pad feeds the shared action frame, and menus
//! get fixed focus navigation instead of a virtual cursor.
use super::actions::Device;
use super::input::{Owner, Ownership};
use bevy::input::gamepad::{ButtonSettings, Gamepad, GamepadButton, GamepadSettings};
use bevy::input::mouse::{AccumulatedMouseMotion, MouseWheel};
use bevy::input_focus::{
    FocusCause, InputFocus, InputFocusVisible,
    directional_navigation::{AutoNavigationConfig, FocusableArea},
    navigator::find_best_candidate,
    tab_navigation::TabGroup,
};
use bevy::math::CompassOctant;
use bevy::prelude::*;
use bevy::ui::{ComputedNode, InteractionDisabled, UiGlobalTransform};
use bevy::ui_widgets::{
    Activate, Button, Slider, SliderRange, SliderStep, SliderValue, ValueChange,
};
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

/// A stick or analog trigger past this, after its deadzone, is deliberate.
const DELIBERATE: f32 = 0.5;
/// Below this much mouse travel in a frame, a resting hand is not a choice.
const MOUSE_DELIBERATE_PX: f32 = 4.0;
/// Separates a trigger press from a release so noise at one value cannot
/// chatter. Bevy's default press/release pair is 0.75/0.65.
const TRIGGER_PRESS: f32 = 0.6;
const TRIGGER_RELEASE: f32 = 0.4;
/// Mild, so small corrections stay fine without slowing full turns.
const LOOK_EXPONENT: f32 = 1.5;
const SONY_VENDOR: u16 = 0x054C;
const REPEAT_DELAY_S: f32 = 0.4;
const REPEAT_INTERVAL_S: f32 = 0.12;

/// Menus own these buttons, so they never drive gameplay actions there.
const NAVIGATION: [GamepadButton; 9] = [
    GamepadButton::South,
    GamepadButton::East,
    GamepadButton::Start,
    GamepadButton::LeftTrigger,
    GamepadButton::RightTrigger,
    GamepadButton::DPadUp,
    GamepadButton::DPadDown,
    GamepadButton::DPadLeft,
    GamepadButton::DPadRight,
];

/// The active pad's frame, in the same shape keyboard and mouse arrive in.
#[derive(Resource, Default)]
pub(super) struct PadInput {
    /// Buttons that may drive actions; menu navigation buttons are withheld
    /// while a menu owns input.
    pub buttons: ButtonInput<GamepadButton>,
    /// Every button of the active pad, for menu navigation.
    pub navigation: ButtonInput<GamepadButton>,
    pub movement: Vec2,
    pub look: Vec2,
    /// Back/close requested from a menu this frame, like Escape.
    pub cancel: bool,
    pub active: Option<Entity>,
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<PadInput>()
        .init_resource::<AutoNavigationConfig>()
        .init_resource::<InputFocusVisible>()
        .add_systems(
            PreUpdate,
            (trigger_hysteresis, read)
                .chain()
                .after(bevy::input::InputSystems),
        )
        .add_systems(
            Update,
            (resume_play, hide_cursor)
                .after(super::input::resolve)
                .run_if(crate::server::persistence::playing),
        )
        .add_systems(
            Update,
            navigate
                .after(super::input::resolve)
                .after(super::settings::render),
        )
        .add_systems(Update, reveal_focus.after(navigate));
}

fn trigger_hysteresis(mut pads: Query<&mut GamepadSettings, Added<Gamepad>>) {
    for mut settings in &mut pads {
        let triggers = ButtonSettings::new(TRIGGER_PRESS, TRIGGER_RELEASE).unwrap();
        for button in [GamepadButton::LeftTrigger2, GamepadButton::RightTrigger2] {
            settings.button_settings.insert(button, triggers.clone());
        }
    }
}

/// Scaled radial deadzone: drift inside it reads as rest, and the live range
/// still reaches full deflection.
pub(super) fn apply_deadzone(stick: Vec2, deadzone: f32, exponent: f32) -> Vec2 {
    let magnitude = stick.length();
    if !magnitude.is_finite() || magnitude <= deadzone {
        return Vec2::ZERO;
    }
    let scaled = ((magnitude - deadzone) / (1.0 - deadzone)).min(1.0);
    stick / magnitude * scaled.powf(exponent)
}

fn any_button_just_pressed(pad: &Gamepad) -> bool {
    pad.get_just_pressed()
        .any(|button| *button != GamepadButton::Mode)
}
fn stick_pushed(pad: &Gamepad, move_deadzone: f32, look_deadzone: f32) -> bool {
    apply_deadzone(pad.left_stick(), move_deadzone, 1.0).length() > DELIBERATE
        || apply_deadzone(pad.right_stick(), look_deadzone, 1.0).length() > DELIBERATE
}
fn deliberate(pad: &Gamepad, move_deadzone: f32, look_deadzone: f32) -> bool {
    any_button_just_pressed(pad) || stick_pushed(pad, move_deadzone, look_deadzone)
}

fn ui_context(owner: Owner) -> bool {
    matches!(
        owner,
        Owner::Catalogue | Owner::Settings | Owner::BindingCapture | Owner::Modal
    )
}

type Outputs<'w> = (
    ResMut<'w, PadInput>,
    ResMut<'w, super::actions::Labels>,
    Option<ResMut<'w, super::input::UiInput>>,
);
type KeyboardMouse<'w, 's> = (
    Res<'w, ButtonInput<KeyCode>>,
    Res<'w, ButtonInput<MouseButton>>,
    Option<Res<'w, AccumulatedMouseMotion>>,
    MessageReader<'w, 's, MouseWheel>,
);

pub(super) fn read(
    pads: Query<(Entity, &Gamepad)>,
    windows: Query<&Window, With<PrimaryWindow>>,
    (preferences, ownership): (Res<super::profile::Preferences>, Option<Res<Ownership>>),
    (keys, mouse, motion, mut wheel): KeyboardMouse,
    (mut input, mut labels, mut ui): Outputs,
    mut was_pushed: Local<bool>,
) {
    let tuning = &preferences.pad;
    let mut epoch = false;
    if input.active.is_some_and(|active| !pads.contains(active)) {
        input.active = None;
        epoch = true;
    }
    let focused = windows.iter().next().is_none_or(|window| window.focused);
    if focused
        && let Some((entity, _)) = pads.iter().find(|(entity, pad)| {
            Some(*entity) != input.active
                && deliberate(pad, tuning.move_deadzone, tuning.look_deadzone)
        })
    {
        epoch |= input.active.is_some();
        input.active = Some(entity);
    }
    if epoch && let Some(ui) = ui.as_mut() {
        ui.device_epoch += 1;
    }
    let pad = input.active.and_then(|active| pads.get(active).ok());
    let owner = ownership.map_or(Owner::Gameplay, |ownership| ownership.owner);
    input.buttons.clear();
    input.navigation.clear();
    input.cancel = false;
    let Some((_, pad)) = pad.filter(|_| focused) else {
        input.buttons.release_all();
        input.navigation.release_all();
        input.movement = Vec2::ZERO;
        input.look = Vec2::ZERO;
        return;
    };
    let menu = ui_context(owner);
    let PadInput {
        buttons,
        navigation,
        ..
    } = &mut *input;
    for button in GamepadButton::all() {
        let pressed = button != GamepadButton::Mode && pad.pressed(button);
        let withheld = menu && NAVIGATION.contains(&button);
        for (set, allowed) in [(&mut *navigation, true), (&mut *buttons, !withheld)] {
            if pressed && allowed {
                set.press(button);
            } else {
                set.release(button);
            }
        }
    }
    let capture = owner == Owner::BindingCapture;
    input.cancel = menu
        && (input.navigation.just_pressed(GamepadButton::Start)
            || (!capture && input.navigation.just_pressed(GamepadButton::East)));
    input.movement = apply_deadzone(pad.left_stick(), tuning.move_deadzone, 1.0);
    let look = apply_deadzone(pad.right_stick(), tuning.look_deadzone, LOOK_EXPONENT);
    input.look = Vec2::new(look.x, -look.y);

    let keyboard = keys.get_just_pressed().next().is_some()
        || mouse.get_just_pressed().next().is_some()
        || wheel.read().count() > 0
        || motion.is_some_and(|motion| motion.delta.length() > MOUSE_DELIBERATE_PX);
    // A stick held over is one choice, not a claim renewed every frame, or a
    // key pressed while still steering could never take the prompts back.
    let push = stick_pushed(pad, tuning.move_deadzone, tuning.look_deadzone);
    let push_started = push && !*was_pushed;
    *was_pushed = push;
    let device = if any_button_just_pressed(pad) || push_started {
        Some(Device::Pad)
    } else {
        keyboard.then_some(Device::KeyboardMouse)
    };
    if let Some(device) = device
        && labels.device != device
    {
        labels.device = device;
    }
    let sony = pad.vendor_id() == Some(SONY_VENDOR);
    if labels.sony != sony {
        labels.sony = sony;
    }
}

/// A pad player has no use for a free cursor, so leaving a menu or pressing
/// South there returns to play. The press that resumes is owned until release.
fn resume_play(
    input: Res<PadInput>,
    ownership: Res<Ownership>,
    labels: Res<super::actions::Labels>,
    mut previous: Local<Owner>,
    cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let left_menu = ui_context(*previous) && ownership.owner == Owner::PointerFree;
    *previous = ownership.owner;
    if ownership.owner != Owner::PointerFree || labels.device != Device::Pad {
        return;
    }
    if left_menu || input.navigation.just_pressed(GamepadButton::South) {
        let mut cursor = cursor.into_inner();
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
}

fn hide_cursor(
    ownership: Res<Ownership>,
    labels: Res<super::actions::Labels>,
    cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if !ui_context(ownership.owner) {
        return;
    }
    let visible = labels.device != Device::Pad;
    let mut cursor = cursor.into_inner();
    if cursor.grab_mode == CursorGrabMode::None && cursor.visible != visible {
        cursor.visible = visible;
    }
}

#[derive(Default)]
struct Repeat {
    direction: Option<CompassOctant>,
    held_s: f32,
    next_s: f32,
}

type Focusables<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static ComputedNode,
        &'static UiGlobalTransform,
        &'static InheritedVisibility,
        Has<super::menu::CatalogueItem>,
        Option<&'static super::menu::TileState>,
    ),
    (
        Or<(With<Button>, With<Slider>)>,
        Without<super::menu_ui::SearchField>,
    ),
>;
type Sliders<'w, 's> = Query<
    'w,
    's,
    (
        &'static SliderValue,
        &'static SliderRange,
        &'static SliderStep,
    ),
>;

type Context<'w> = (
    Res<'w, PadInput>,
    Res<'w, Ownership>,
    Res<'w, super::actions::Labels>,
    Res<'w, super::settings::Settings>,
);
type Structure<'w, 's> = (
    Query<'w, 's, (Entity, &'static TabGroup)>,
    Query<'w, 's, &'static ChildOf>,
    Query<'w, 's, (), With<InteractionDisabled>>,
    Sliders<'w, 's>,
    Query<'w, 's, (Entity, &'static super::settings::Operation)>,
);
struct Candidate {
    area: FocusableArea,
    catalogue: bool,
    selected: bool,
}

fn area(entity: Entity, node: &ComputedNode, transform: &UiGlobalTransform) -> FocusableArea {
    let (scale, _, translation) = transform.to_scale_angle_translation();
    FocusableArea {
        entity,
        position: translation * node.inverse_scale_factor(),
        size: node.size() * node.inverse_scale_factor() * scale,
    }
}

fn navigate(
    mut commands: Commands,
    (input, ownership, labels, settings): Context,
    (time, config): (Res<Time>, Res<AutoNavigationConfig>),
    (mut focus, mut visible): (ResMut<InputFocus>, ResMut<InputFocusVisible>),
    focusables: Focusables,
    (groups, parents, disabled, sliders, tabs): Structure,
    (mut repeat, mut last): (Local<Repeat>, Local<Option<Vec2>>),
) {
    let owner = ownership.owner;
    if !matches!(owner, Owner::Catalogue | Owner::Settings | Owner::Modal) {
        *repeat = default();
        // Capture and dialogs keep the place, so focus returns to the row.
        if !ui_context(owner) {
            *last = None;
        }
        return;
    }
    let modal: Vec<Entity> = groups
        .iter()
        .filter(|(_, group)| group.modal)
        .map(|(entity, _)| entity)
        .collect();
    let candidates: Vec<Candidate> = focusables
        .iter()
        .filter(|(entity, node, _, visibility, ..)| {
            !node.is_empty()
                && visibility.get()
                && (modal.is_empty()
                    || parents
                        .iter_ancestors(*entity)
                        .any(|ancestor| modal.contains(&ancestor)))
        })
        .map(|(entity, node, transform, _, catalogue, tile)| Candidate {
            area: area(entity, node, transform),
            catalogue,
            selected: tile.is_some_and(|tile| tile.selected),
        })
        .collect();
    let current = focus
        .get()
        .and_then(|entity| candidates.iter().find(|c| c.area.entity == entity));
    if let Some(current) = current {
        *last = Some(current.area.position);
    } else if labels.device == Device::Pad
        && let Some(entity) = initial(&candidates, *last)
    {
        focus.set(entity, FocusCause::Navigated);
        visible.0 = true;
        return;
    }
    let pad = &input.navigation;
    let dpad = [
        (GamepadButton::DPadUp, CompassOctant::North),
        (GamepadButton::DPadDown, CompassOctant::South),
        (GamepadButton::DPadLeft, CompassOctant::West),
        (GamepadButton::DPadRight, CompassOctant::East),
    ]
    .into_iter()
    .find(|(button, _)| pad.pressed(*button))
    .map(|(_, direction)| direction);
    let stick = input.movement;
    let direction = dpad.or_else(|| {
        (stick.length() > DELIBERATE).then(|| {
            if stick.x.abs() > stick.y.abs() {
                if stick.x > 0.0 {
                    CompassOctant::East
                } else {
                    CompassOctant::West
                }
            } else if stick.y > 0.0 {
                CompassOctant::North
            } else {
                CompassOctant::South
            }
        })
    });
    let step = match direction {
        None => {
            *repeat = default();
            false
        }
        Some(direction) if repeat.direction != Some(direction) => {
            *repeat = Repeat {
                direction: Some(direction),
                held_s: 0.0,
                next_s: REPEAT_DELAY_S,
            };
            true
        }
        Some(_) => {
            repeat.held_s += time.delta_secs();
            let due = repeat.held_s >= repeat.next_s;
            if due {
                repeat.next_s += REPEAT_INTERVAL_S;
            }
            due
        }
    };
    let Some(current) = current else {
        return;
    };
    let entity = current.area.entity;
    if step && let Some(direction) = direction {
        visible.0 = true;
        if matches!(direction, CompassOctant::East | CompassOctant::West)
            && let Ok((value, range, slider_step)) = sliders.get(entity)
        {
            let sign = if direction == CompassOctant::East {
                1.0
            } else {
                -1.0
            };
            commands.trigger(ValueChange {
                source: entity,
                value: range.clamp(value.0 + sign * slider_step.0),
                is_final: true,
            });
        } else {
            let areas: Vec<_> = candidates.iter().map(|c| c.area).collect();
            if let Some(next) = find_best_candidate(&current.area, direction, &areas, &config) {
                focus.set(next, FocusCause::Navigated);
            }
        }
    }
    if pad.just_pressed(GamepadButton::South) && !disabled.contains(entity) {
        visible.0 = true;
        commands.trigger(Activate { entity });
    }
    let shift = i32::from(pad.just_pressed(GamepadButton::RightTrigger))
        - i32::from(pad.just_pressed(GamepadButton::LeftTrigger));
    if shift != 0 && modal.is_empty() {
        use super::menu_ui::Tab;
        const ORDER: [Tab; 5] = Tab::ORDER;
        let tab = if settings.open {
            settings.tab
        } else {
            Tab::Catalogue
        };
        let index = ORDER.iter().position(|t| *t == tab).unwrap_or(0) as i32;
        let target = ORDER[(index + shift).rem_euclid(ORDER.len() as i32) as usize];
        if let Some((button, _)) = tabs.iter().find(|(_, operation)| {
            matches!(operation, super::settings::Operation::Navigate(t) if *t == target)
        }) {
            commands.trigger(Activate { entity: button });
        }
    }
}

/// Where focus lands when a menu opens or its focused entity is rebuilt: near
/// the last focused place, else the held Catalogue entry, else the first entry.
fn initial(candidates: &[Candidate], last: Option<Vec2>) -> Option<Entity> {
    let reading_order = |c: &&Candidate| {
        (
            (c.area.position.y * 10.0) as i64,
            (c.area.position.x * 10.0) as i64,
        )
    };
    if let Some(last) = last {
        return candidates
            .iter()
            .min_by(|a, b| {
                a.area
                    .position
                    .distance_squared(last)
                    .total_cmp(&b.area.position.distance_squared(last))
            })
            .map(|c| c.area.entity);
    }
    candidates
        .iter()
        .filter(|c| c.catalogue && c.selected)
        .min_by_key(reading_order)
        .or_else(|| {
            candidates
                .iter()
                .filter(|c| c.catalogue)
                .min_by_key(reading_order)
        })
        .or_else(|| candidates.iter().min_by_key(reading_order))
        .map(|c| c.area.entity)
}

fn reveal_focus(
    focus: Res<InputFocus>,
    labels: Res<super::actions::Labels>,
    nodes: Query<(&ComputedNode, &UiGlobalTransform)>,
    parents: Query<&ChildOf>,
    mut scrolls: Query<(&mut ScrollPosition, &Node)>,
    mut shown: Local<Option<Entity>>,
) {
    let target = focus.get();
    if *shown == target || labels.device != Device::Pad {
        *shown = target;
        return;
    }
    let Some(entity) = target else {
        *shown = None;
        return;
    };
    let Ok((node, transform)) = nodes.get(entity) else {
        return;
    };
    if node.is_empty() {
        return;
    }
    *shown = target;
    let item = area(entity, node, transform);
    for ancestor in parents.iter_ancestors(entity) {
        let Ok((mut scroll, style)) = scrolls.get_mut(ancestor) else {
            continue;
        };
        let Ok((container_node, container_transform)) = nodes.get(ancestor) else {
            continue;
        };
        let container = area(ancestor, container_node, container_transform);
        let (x, y) = (
            style.overflow.x == OverflowAxis::Scroll,
            style.overflow.y == OverflowAxis::Scroll,
        );
        if !x && !y {
            continue;
        }
        let low = (item.position - item.size / 2.0) - (container.position - container.size / 2.0);
        let high = (item.position + item.size / 2.0) - (container.position + container.size / 2.0);
        let mut offset = scroll.0;
        for (axis, scrolls) in [(0, x), (1, y)] {
            if !scrolls {
                continue;
            }
            if low[axis] < 0.0 {
                offset[axis] += low[axis];
            } else if high[axis] > 0.0 {
                offset[axis] += high[axis].min(low[axis]);
            }
        }
        let offset = offset.max(Vec2::ZERO);
        if scroll.0 != offset {
            scroll.0 = offset;
        }
        break;
    }
}

#[cfg(test)]
mod tests {
    use super::super::actions::{Action, Actions, Labels};
    use super::*;
    use bevy::input::gamepad::{
        GamepadAxis, RawGamepadAxisChangedEvent, RawGamepadButtonChangedEvent, RawGamepadEvent,
    };

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((bevy::time::TimePlugin, bevy::input::InputPlugin))
            .add_plugins(super::super::actions::test_plugin)
            .init_resource::<super::super::menu::Menu>()
            .init_resource::<super::super::input::UiInput>()
            .insert_resource(Ownership::playing())
            .add_systems(
                PreUpdate,
                (trigger_hysteresis, read)
                    .chain()
                    .after(bevy::input::InputSystems),
            )
            .add_systems(Update, super::super::input::resolve);
        app.world_mut().spawn((
            PrimaryWindow,
            Window {
                focused: true,
                ..default()
            },
            CursorOptions {
                grab_mode: CursorGrabMode::Locked,
                ..default()
            },
        ));
        app
    }
    fn connect(app: &mut App) -> Entity {
        let pad = app.world_mut().spawn(Gamepad::default()).id();
        app.update();
        pad
    }
    fn button(app: &mut App, pad: Entity, button: GamepadButton, value: f32) {
        app.world_mut()
            .write_message(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
                pad, button, value,
            )));
    }
    fn axis(app: &mut App, pad: Entity, axis: GamepadAxis, value: f32) {
        app.world_mut()
            .write_message(RawGamepadEvent::Axis(RawGamepadAxisChangedEvent::new(
                pad, axis, value,
            )));
    }
    fn state(app: &App, action: Action) -> super::super::actions::State {
        app.world().resource::<Actions>().state(action)
    }

    #[test]
    fn a_trigger_holds_remove_once_and_noise_between_thresholds_cannot_chatter() {
        let mut app = app();
        let pad = connect(&mut app);
        let mut starts = 0;
        for (value, held) in [
            (1.0, true),
            (0.5, true),
            (0.45, true),
            (0.55, true),
            (0.3, false),
            (0.55, false),
            (0.5, false),
            (0.7, true),
        ] {
            button(&mut app, pad, GamepadButton::RightTrigger2, value);
            app.update();
            let remove = state(&app, Action::Remove);
            starts += usize::from(remove.started);
            assert_eq!(remove.held, held, "trigger at {value}");
            assert!(remove.from_pad || !held);
        }
        assert_eq!(starts, 2);
    }

    #[test]
    fn drift_neither_moves_nor_claims_the_prompts() {
        let mut app = app();
        let pad = connect(&mut app);
        for (stick, value) in [
            (GamepadAxis::LeftStickX, 0.12),
            (GamepadAxis::LeftStickY, -0.08),
            (GamepadAxis::RightStickX, 0.14),
        ] {
            axis(&mut app, pad, stick, value);
        }
        for _ in 0..3 {
            app.update();
            let axes = &app.world().resource::<Actions>().axes;
            assert_eq!((axes.movement, axes.look_rate), (Vec2::ZERO, Vec2::ZERO));
            assert_eq!(
                app.world().resource::<Labels>().device,
                Device::KeyboardMouse
            );
        }
        axis(&mut app, pad, GamepadAxis::LeftStickY, 0.9);
        app.update();
        assert!(app.world().resource::<Actions>().axes.movement.y > 0.8);
        assert_eq!(app.world().resource::<Labels>().device, Device::Pad);
        let window = app
            .world_mut()
            .query_filtered::<Entity, With<PrimaryWindow>>()
            .single(app.world())
            .unwrap();
        app.world_mut()
            .write_message(bevy::input::keyboard::KeyboardInput {
                key_code: KeyCode::KeyW,
                logical_key: bevy::input::keyboard::Key::Character("w".into()),
                state: bevy::input::ButtonState::Pressed,
                text: None,
                repeat: false,
                window,
            });
        app.update();
        // The stick is still held over, but the key is the newer choice.
        assert_eq!(
            app.world().resource::<Labels>().device,
            Device::KeyboardMouse
        );
    }

    #[test]
    fn disconnecting_mid_hold_cancels_instead_of_completing() {
        let mut app = app();
        let pad = connect(&mut app);
        button(&mut app, pad, GamepadButton::RightTrigger2, 1.0);
        app.update();
        assert!(state(&app, Action::Remove).held);
        app.world_mut().entity_mut(pad).remove::<Gamepad>();
        app.update();
        let remove = state(&app, Action::Remove);
        assert!(remove.cancelled && !remove.released && !remove.held);
        app.update();
        assert!(!state(&app, Action::Remove).held);
    }

    #[test]
    fn a_second_pad_takes_over_only_on_a_deliberate_press_which_it_does_not_spend() {
        let mut app = app();
        let first = connect(&mut app);
        button(&mut app, first, GamepadButton::South, 1.0);
        app.update();
        assert!(state(&app, Action::Ascend).started);
        let second = connect(&mut app);
        axis(&mut app, second, GamepadAxis::LeftStickX, 0.1);
        app.update();
        assert_eq!(app.world().resource::<PadInput>().active, Some(first));
        button(&mut app, second, GamepadButton::RightTrigger2, 1.0);
        app.update();
        assert_eq!(app.world().resource::<PadInput>().active, Some(second));
        let ascend = state(&app, Action::Ascend);
        assert!(ascend.cancelled && !ascend.held);
        assert!(!state(&app, Action::Remove).held);
        button(&mut app, second, GamepadButton::RightTrigger2, 0.0);
        app.update();
        button(&mut app, second, GamepadButton::RightTrigger2, 1.0);
        app.update();
        assert!(state(&app, Action::Remove).started);
    }

    #[test]
    fn an_unfocused_window_ignores_the_pad_entirely() {
        let mut app = app();
        let pad = connect(&mut app);
        app.world_mut()
            .query::<&mut Window>()
            .single_mut(app.world_mut())
            .unwrap()
            .focused = false;
        button(&mut app, pad, GamepadButton::RightTrigger2, 1.0);
        axis(&mut app, pad, GamepadAxis::LeftStickY, 1.0);
        app.update();
        assert!(!state(&app, Action::Remove).held);
        assert_eq!(app.world().resource::<Actions>().axes.movement, Vec2::ZERO);
        assert_eq!(
            app.world().resource::<Labels>().device,
            Device::KeyboardMouse
        );
        assert_eq!(app.world().resource::<PadInput>().active, None);
    }

    #[test]
    fn menus_own_navigation_buttons_and_turn_back_into_cancel() {
        let mut app = app();
        let pad = connect(&mut app);
        app.world_mut()
            .resource_mut::<super::super::menu::Menu>()
            .open = true;
        app.update();
        assert_eq!(app.world().resource::<Ownership>().owner, Owner::Catalogue);
        button(&mut app, pad, GamepadButton::DPadDown, 1.0);
        button(&mut app, pad, GamepadButton::Start, 1.0);
        app.update();
        assert!(!state(&app, Action::EmptyHand).started);
        assert!(!state(&app, Action::Help).started);
        assert!(app.world().resource::<Actions>().cancel_requested());
        button(&mut app, pad, GamepadButton::North, 1.0);
        app.update();
        assert!(state(&app, Action::Catalogue).started);
        assert!(!app.world().resource::<Actions>().cancel_requested());
    }

    #[test]
    fn a_menu_or_lost_focus_mid_hold_cancels_and_never_completes() {
        for interruption in ["menu", "focus"] {
            let mut app = app();
            let pad = connect(&mut app);
            button(&mut app, pad, GamepadButton::RightTrigger2, 1.0);
            app.update();
            assert!(state(&app, Action::Remove).held);
            match interruption {
                "menu" => {
                    app.world_mut()
                        .resource_mut::<super::super::menu::Menu>()
                        .open = true
                }
                _ => {
                    app.world_mut()
                        .query::<&mut Window>()
                        .single_mut(app.world_mut())
                        .unwrap()
                        .focused = false
                }
            }
            app.update();
            let remove = state(&app, Action::Remove);
            assert!(
                remove.cancelled && !remove.released && !remove.held,
                "{interruption}"
            );
        }
    }

    #[test]
    fn a_navigation_button_held_as_a_menu_closes_waits_for_release() {
        let mut app = app();
        let pad = connect(&mut app);
        app.world_mut()
            .resource_mut::<super::super::menu::Menu>()
            .open = true;
        app.update();
        button(&mut app, pad, GamepadButton::DPadDown, 1.0);
        app.update();
        app.world_mut()
            .resource_mut::<super::super::menu::Menu>()
            .open = false;
        for _ in 0..3 {
            app.update();
            assert!(!state(&app, Action::EmptyHand).started);
        }
        button(&mut app, pad, GamepadButton::DPadDown, 0.0);
        app.update();
        button(&mut app, pad, GamepadButton::DPadDown, 1.0);
        app.update();
        assert!(state(&app, Action::EmptyHand).started);
    }

    #[test]
    fn focus_returns_near_where_it_was_or_to_the_held_entry() {
        let candidate = |index: u32, x: f32, y: f32, catalogue: bool, selected: bool| Candidate {
            area: FocusableArea {
                entity: Entity::from_raw_u32(index).unwrap(),
                position: Vec2::new(x, y),
                size: Vec2::splat(10.0),
            },
            catalogue,
            selected,
        };
        let candidates = [
            candidate(1, 0.0, 0.0, false, false),
            candidate(2, 50.0, 100.0, true, false),
            candidate(3, 10.0, 100.0, true, false),
            candidate(4, 90.0, 200.0, true, true),
        ];
        let entity = |index| Some(Entity::from_raw_u32(index).unwrap());
        assert_eq!(initial(&candidates, None), entity(4));
        assert_eq!(initial(&candidates[..3], None), entity(3));
        assert_eq!(initial(&candidates[..1], None), entity(1));
        assert_eq!(initial(&candidates, Some(Vec2::new(45.0, 95.0))), entity(2));
    }

    #[test]
    fn drift_inside_the_deadzone_reads_as_rest_and_full_deflection_survives() {
        assert_eq!(apply_deadzone(Vec2::new(0.1, 0.1), 0.15, 1.0), Vec2::ZERO);
        assert_eq!(apply_deadzone(Vec2::new(0.0, -0.15), 0.15, 1.5), Vec2::ZERO);
        let full = apply_deadzone(Vec2::new(0.0, 1.0), 0.15, 1.5);
        assert!((full.length() - 1.0).abs() < 1e-6);
        let half = apply_deadzone(Vec2::new(0.575, 0.0), 0.15, 1.0);
        assert!((half.x - 0.5).abs() < 1e-5, "{half:?}");
        assert_eq!(apply_deadzone(Vec2::NAN, 0.15, 1.0), Vec2::ZERO);
    }
}
