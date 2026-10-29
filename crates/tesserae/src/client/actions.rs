//! Stable actions and a single keyboard/mouse/controller resolution pass.
use bevy::input::{
    gamepad::GamepadButton,
    mouse::{AccumulatedMouseMotion, MouseScrollUnit, MouseWheel},
    touch::TouchPhase,
};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Action {
    Forward,
    Backward,
    Left,
    Right,
    Ascend,
    Descend,
    Sprint,
    Respawn,
    MovementMode,
    View,
    ZoomIn,
    ZoomOut,
    Faster,
    Slower,
    Remove,
    Place,
    Sample,
    EmptyHand,
    Shape,
    Turn,
    Flip,
    Slot1,
    Slot2,
    Slot3,
    Slot4,
    Slot5,
    Slot6,
    Slot7,
    Slot8,
    Slot9,
    PreviousSlot,
    NextSlot,
    Undo,
    Redo,
    Catalogue,
    ReleaseCursor,
    Help,
    Diagnostics,
}
use Action::*;

pub(super) const SLOTS: [Action; 9] = [
    Slot1, Slot2, Slot3, Slot4, Slot5, Slot6, Slot7, Slot8, Slot9,
];

/// Which binding set an input belongs to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(super) enum Device {
    #[default]
    KeyboardMouse,
    Pad,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Trigger {
    Key(KeyCode),
    Mouse(MouseButton),
    Pad(GamepadButton),
    Wheel(Wheel),
}
impl Trigger {
    pub fn device(self) -> Device {
        match self {
            Self::Key(_) | Self::Mouse(_) | Self::Wheel(_) => Device::KeyboardMouse,
            Self::Pad(_) => Device::Pad,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Wheel {
    Up,
    Down,
}
/// Which way the wheel turned a whole notch this frame. A wheel trigger is
/// pressed for that frame only.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct WheelPress {
    pub up: bool,
    pub down: bool,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Modifier {
    #[default]
    None,
    Control,
    Primary,
    PrimaryShift,
    Shift,
    Alt,
    AltShift,
    PrimaryAlt,
    PrimaryAltShift,
}
impl Modifier {
    fn matches(self, ctrl: bool, command: bool, shift: bool, alt: bool) -> bool {
        match self {
            Self::None => true,
            Self::Control => ctrl,
            Self::Primary => (ctrl || command) && !shift && !alt,
            Self::PrimaryShift => (ctrl || command) && shift && !alt,
            Self::Shift => shift && !alt,
            Self::Alt => alt && !shift,
            Self::AltShift => alt && shift,
            Self::PrimaryAlt => (ctrl || command) && alt && !shift,
            Self::PrimaryAltShift => (ctrl || command) && alt && shift,
        }
    }
    pub fn overlaps(self, other: Self) -> bool {
        (0..16).any(|mask| {
            self.matches(mask & 1 != 0, mask & 2 != 0, mask & 4 != 0, mask & 8 != 0)
                && other.matches(mask & 1 != 0, mask & 2 != 0, mask & 4 != 0, mask & 8 != 0)
        })
    }
}

/// The physical input resources, for systems that read every device.
pub(super) type Devices<'w> = (
    Res<'w, ButtonInput<KeyCode>>,
    Res<'w, ButtonInput<MouseButton>>,
    Res<'w, super::pad::PadInput>,
);

/// Every physical input the resolver reads in one frame.
#[derive(Clone, Copy)]
pub(super) struct Inputs<'a> {
    pub keys: &'a ButtonInput<KeyCode>,
    pub mouse: &'a ButtonInput<MouseButton>,
    pub pad: &'a ButtonInput<GamepadButton>,
    pub wheel: WheelPress,
}
impl Inputs<'_> {
    pub fn pressed(&self, trigger: Trigger) -> bool {
        match trigger {
            Trigger::Key(key) => self.keys.pressed(key),
            Trigger::Mouse(button) => self.mouse.pressed(button),
            Trigger::Pad(button) => self.pad.pressed(button),
            Trigger::Wheel(Wheel::Up) => self.wheel.up,
            Trigger::Wheel(Wheel::Down) => self.wheel.down,
        }
    }
    pub fn just_pressed(&self, trigger: Trigger) -> bool {
        match trigger {
            Trigger::Key(key) => self.keys.just_pressed(key),
            Trigger::Mouse(button) => self.mouse.just_pressed(button),
            Trigger::Pad(button) => self.pad.just_pressed(button),
            // Each notch is a fresh press, even in consecutive frames.
            Trigger::Wheel(_) => self.pressed(trigger),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Binding {
    pub trigger: Trigger,
    pub modifier: Modifier,
}

impl Binding {
    pub fn active(&self, inputs: Inputs) -> bool {
        inputs.pressed(self.trigger) && self.modifier_held(inputs.keys)
    }
    fn modifier_held(&self, keys: &ButtonInput<KeyCode>) -> bool {
        self.modifier.matches(
            keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]),
            keys.any_pressed([KeyCode::SuperLeft, KeyCode::SuperRight]),
            keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]),
            keys.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]),
        )
    }
}

fn single(trigger: Trigger) -> [Option<Binding>; 2] {
    [
        Some(Binding {
            trigger,
            modifier: Modifier::None,
        }),
        None,
    ]
}

/// Two alternatives per action for each device, so a controller binding never
/// displaces a keyboard one. Every action has an entry in both maps.
#[derive(Resource, Clone, PartialEq, Debug)]
pub(super) struct Bindings {
    pub keyboard: HashMap<Action, [Option<Binding>; 2]>,
    pub pad: HashMap<Action, [Option<Binding>; 2]>,
}
impl Bindings {
    pub fn map(&self, device: Device) -> &HashMap<Action, [Option<Binding>; 2]> {
        match device {
            Device::KeyboardMouse => &self.keyboard,
            Device::Pad => &self.pad,
        }
    }
    pub fn map_mut(&mut self, device: Device) -> &mut HashMap<Action, [Option<Binding>; 2]> {
        match device {
            Device::KeyboardMouse => &mut self.keyboard,
            Device::Pad => &mut self.pad,
        }
    }
    pub fn alternatives(&self, action: Action) -> impl Iterator<Item = Binding> + '_ {
        [&self.keyboard, &self.pad]
            .into_iter()
            .filter_map(move |map| map.get(&action))
            .flatten()
            .flatten()
            .copied()
    }
    pub fn pad_defaults() -> HashMap<Action, [Option<Binding>; 2]> {
        let keyboard = Self::keyboard_defaults();
        let mut pad: HashMap<_, _> = keyboard.keys().map(|&action| (action, [None; 2])).collect();
        for (action, button) in [
            (Ascend, GamepadButton::South),
            (Descend, GamepadButton::East),
            (Catalogue, GamepadButton::North),
            (Shape, GamepadButton::West),
            (Sprint, GamepadButton::LeftThumb),
            (Sample, GamepadButton::RightThumb),
            (Remove, GamepadButton::RightTrigger2),
            (Place, GamepadButton::LeftTrigger2),
            (PreviousSlot, GamepadButton::LeftTrigger),
            (NextSlot, GamepadButton::RightTrigger),
            (Turn, GamepadButton::DPadLeft),
            (Flip, GamepadButton::DPadRight),
            (EmptyHand, GamepadButton::DPadDown),
            (Help, GamepadButton::Start),
            (MovementMode, GamepadButton::Select),
        ] {
            pad.insert(action, single(Trigger::Pad(button)));
        }
        pad
    }
    fn keyboard_defaults() -> HashMap<Action, [Option<Binding>; 2]> {
        let mut bindings = HashMap::new();
        for (action, key) in [
            (Forward, KeyCode::KeyW),
            (Backward, KeyCode::KeyS),
            (Left, KeyCode::KeyA),
            (Right, KeyCode::KeyD),
            (Ascend, KeyCode::Space),
            (Descend, KeyCode::ControlLeft),
            (Sprint, KeyCode::ShiftLeft),
            (Respawn, KeyCode::KeyR),
            (MovementMode, KeyCode::KeyV),
            (View, KeyCode::F5),
            (Faster, KeyCode::Equal),
            (Slower, KeyCode::Minus),
            (Sample, KeyCode::KeyP),
            (EmptyHand, KeyCode::KeyX),
            (Shape, KeyCode::KeyF),
            (Turn, KeyCode::KeyQ),
            (Flip, KeyCode::KeyE),
            (Catalogue, KeyCode::Tab),
            (ReleaseCursor, KeyCode::Escape),
            (Help, KeyCode::KeyI),
            (Diagnostics, KeyCode::F3),
        ]
        .into_iter()
        .chain(SLOTS.into_iter().zip(super::edit::SLOT_KEYS))
        {
            bindings.insert(action, single(Trigger::Key(key)));
        }
        for (action, button) in [(Remove, MouseButton::Left), (Place, MouseButton::Right)] {
            bindings.insert(action, single(Trigger::Mouse(button)));
        }
        // The wheel already steps through slots; these exist for the controller.
        bindings.insert(PreviousSlot, [None; 2]);
        bindings.insert(NextSlot, [None; 2]);
        bindings.get_mut(&Sprint).unwrap()[1] = Some(Binding {
            trigger: Trigger::Key(KeyCode::ShiftRight),
            modifier: Modifier::None,
        });
        bindings.get_mut(&Sample).unwrap()[1] = Some(Binding {
            trigger: Trigger::Mouse(MouseButton::Middle),
            modifier: Modifier::None,
        });
        for (action, wheel) in [(ZoomIn, Wheel::Up), (ZoomOut, Wheel::Down)] {
            bindings.insert(
                action,
                [
                    Some(Binding {
                        trigger: Trigger::Wheel(wheel),
                        modifier: Modifier::Alt,
                    }),
                    None,
                ],
            );
        }
        bindings.insert(
            Undo,
            [
                Some(Binding {
                    trigger: Trigger::Key(KeyCode::KeyZ),
                    modifier: Modifier::Primary,
                }),
                None,
            ],
        );
        bindings.insert(
            Redo,
            [
                Some(Binding {
                    trigger: Trigger::Key(KeyCode::KeyZ),
                    modifier: Modifier::PrimaryShift,
                }),
                Some(Binding {
                    trigger: Trigger::Key(KeyCode::KeyY),
                    modifier: Modifier::Control,
                }),
            ],
        );
        bindings
    }
}
impl Default for Bindings {
    fn default() -> Self {
        Self {
            keyboard: Self::keyboard_defaults(),
            pad: Self::pad_defaults(),
        }
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub(super) struct State {
    pub started: bool,
    pub held: bool,
    pub released: bool,
    pub cancelled: bool,
    /// Held through a controller binding, for behaviour that differs by hand
    /// position: a pad latches what a keyboard holds.
    pub from_pad: bool,
}
pub(super) const PIXELS_PER_NOTCH: f32 = 100.0;
#[derive(Clone, Copy, Debug)]
pub(super) struct ScrollDelta {
    pub notches: f32,
    pub started: bool,
}
#[derive(Default, Debug)]
pub(super) struct Axes {
    /// Mouse travel this frame, in pixels.
    pub look: Vec2,
    /// Analog look deflection in [-1, 1], screen-oriented like `look` (+y is
    /// down). A rate, so the camera scales it by elapsed time.
    pub look_rate: Vec2,
    /// Analog movement in [-1, 1]: +x is right, +y is forward.
    pub movement: Vec2,
    pub scroll: Vec<ScrollDelta>,
}
#[derive(Resource, Default)]
pub(super) struct Actions {
    states: HashMap<Action, State>,
    raw_held: HashSet<Action>,
    raw_triggers: HashSet<Trigger>,
    reserved_cancel: bool,
    prior_effective_held: HashSet<Action>,
    pub axes: Axes,
    pub wheel: WheelPress,
    /// A trackpad's part of a notch, kept until it makes a whole one.
    wheel_carry: f32,
}
impl Actions {
    pub fn consume_cancel(&mut self) {
        self.reserved_cancel = false;
        self.cancel(Action::ReleaseCursor);
    }
    pub fn cancel_requested(&self) -> bool {
        self.reserved_cancel || self.started(Action::ReleaseCursor)
    }
    pub fn state(&self, action: Action) -> State {
        self.states.get(&action).copied().unwrap_or_default()
    }
    pub fn started(&self, action: Action) -> bool {
        self.state(action).started
    }
    pub fn held(&self, action: Action) -> bool {
        self.state(action).held
    }
    fn press_wheel(&mut self) -> WheelPress {
        for delta in &self.axes.scroll {
            if delta.started || delta.notches * self.wheel_carry < 0.0 {
                self.wheel_carry = 0.0;
            }
            self.wheel_carry += delta.notches;
        }
        let whole = self.wheel_carry.trunc();
        self.wheel_carry -= whole;
        WheelPress {
            up: whole > 0.0,
            down: whole < 0.0,
        }
    }
    pub fn cancel(&mut self, action: Action) {
        let previously_held = self.prior_effective_held.contains(&action);
        let state = self.states.entry(action).or_default();
        *state = State {
            cancelled: previously_held || state.started || state.cancelled,
            ..default()
        };
    }
    fn resolve(&mut self, bindings: &Bindings, inputs: Inputs, cancel: bool) {
        self.prior_effective_held = self
            .states
            .iter()
            .filter_map(|(&action, state)| state.held.then_some(action))
            .collect();
        self.reserved_cancel = inputs.keys.just_pressed(KeyCode::Escape) || cancel;
        let matches = |binding: Binding| binding.active(inputs);
        let consumed: HashSet<_> = bindings
            .keyboard
            .values()
            .flatten()
            .flatten()
            .filter(|binding| binding.modifier != Modifier::None && matches(**binding))
            .map(|binding| binding.trigger)
            .collect();
        // A history chord owns its modifier too, preventing vertical movement.
        let history = [Undo, Redo]
            .into_iter()
            .any(|action| bindings.alternatives(action).any(matches));
        let actions: HashSet<Action> = bindings
            .keyboard
            .keys()
            .chain(bindings.pad.keys())
            .copied()
            .collect();
        for action in actions {
            let alternatives: Vec<_> = bindings.alternatives(action).collect();
            let held = alternatives.iter().any(|binding| {
                matches(*binding)
                    && (binding.modifier != Modifier::None || !consumed.contains(&binding.trigger))
            }) && !(action == Descend && history);
            let continuing = alternatives.iter().any(|binding| {
                matches(*binding)
                    && self.raw_triggers.contains(&binding.trigger)
                    && !inputs.just_pressed(binding.trigger)
            });
            let previous = self.raw_held.contains(&action);
            if held {
                self.raw_held.insert(action);
            } else {
                self.raw_held.remove(&action);
            }
            self.states.insert(
                action,
                State {
                    started: held
                        && (!previous
                            || (!continuing
                                && alternatives.iter().any(|binding| {
                                    matches(*binding) && inputs.just_pressed(binding.trigger)
                                }))),
                    held,
                    released: previous && !held,
                    cancelled: false,
                    from_pad: held
                        && alternatives.iter().any(|binding| {
                            binding.trigger.device() == Device::Pad && matches(*binding)
                        }),
                },
            );
        }
        self.raw_triggers = inputs
            .keys
            .get_pressed()
            .copied()
            .map(Trigger::Key)
            .chain(inputs.mouse.get_pressed().copied().map(Trigger::Mouse))
            .chain(inputs.pad.get_pressed().copied().map(Trigger::Pad))
            .collect();
    }
}

/// How bindings read on screen: keyboard labels follow the active layout, and
/// controller labels follow the last deliberately used device and its maker.
#[derive(Resource, Default)]
pub(super) struct Labels {
    keys: HashMap<KeyCode, String>,
    pub device: Device,
    pub sony: bool,
}
impl Labels {
    pub fn binding(&self, binding: Binding) -> String {
        let trigger = match binding.trigger {
            Trigger::Key(key) => self.keys.get(&key).cloned().unwrap_or_else(|| {
                let name = format!("{key:?}");
                name.strip_prefix("Key")
                    .or_else(|| name.strip_prefix("Digit"))
                    .unwrap_or(&name)
                    .to_owned()
            }),
            Trigger::Mouse(button) => format!("{button:?} mouse"),
            Trigger::Pad(button) => self.pad(button),
            Trigger::Wheel(Wheel::Up) => "Wheel up".into(),
            Trigger::Wheel(Wheel::Down) => "Wheel down".into(),
        };
        let modifier = match binding.modifier {
            Modifier::None => "",
            Modifier::Control => "Ctrl+",
            Modifier::Primary => "Ctrl/Cmd+",
            Modifier::PrimaryShift => "Ctrl/Cmd+Shift+",
            Modifier::Shift => "Shift+",
            Modifier::Alt => "Alt+",
            Modifier::AltShift => "Alt+Shift+",
            Modifier::PrimaryAlt => "Ctrl/Cmd+Alt+",
            Modifier::PrimaryAltShift => "Ctrl/Cmd+Alt+Shift+",
        };
        format!("{modifier}{trigger}")
    }
    pub fn pad(&self, button: GamepadButton) -> String {
        use GamepadButton::*;
        let (xbox, playstation) = match button {
            South => ("A", "Cross"),
            East => ("B", "Circle"),
            West => ("X", "Square"),
            North => ("Y", "Triangle"),
            LeftTrigger => ("LB", "L1"),
            RightTrigger => ("RB", "R1"),
            LeftTrigger2 => ("LT", "L2"),
            RightTrigger2 => ("RT", "R2"),
            Select => ("View", "Create"),
            Start => ("Menu", "Options"),
            Mode => ("Guide", "PS"),
            LeftThumb => ("LS", "L3"),
            RightThumb => ("RS", "R3"),
            DPadUp => ("D-pad ↑", "D-pad ↑"),
            DPadDown => ("D-pad ↓", "D-pad ↓"),
            DPadLeft => ("D-pad ←", "D-pad ←"),
            DPadRight => ("D-pad →", "D-pad →"),
            C => ("C", "C"),
            Z => ("Z", "Z"),
            Other(code) => return format!("Button {code}"),
        };
        if self.sony { playstation } else { xbox }.to_owned()
    }
}
impl Bindings {
    /// Labels from the binding set of the last deliberately used device.
    pub fn label(&self, action: Action, labels: &Labels) -> String {
        self.label_for(labels.device, action, labels)
    }
    pub fn label_for(&self, device: Device, action: Action, labels: &Labels) -> String {
        let alternatives: Vec<_> = self
            .map(device)
            .get(&action)
            .into_iter()
            .flatten()
            .flatten()
            .map(|binding| labels.binding(*binding))
            .collect();
        if alternatives.is_empty() {
            "Unbound".into()
        } else {
            alternatives.join(" / ")
        }
    }
}
fn labels(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    mut labels: ResMut<Labels>,
) {
    for event in events.read() {
        if let bevy::input::keyboard::Key::Character(character) = &event.logical_key {
            let label = character.to_uppercase();
            if labels.keys.get(&event.key_code) != Some(&label) {
                labels.keys.insert(event.key_code, label);
            }
        }
    }
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Labels>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(PreUpdate, labels.after(bevy::input::InputSystems));
    app.init_resource::<Bindings>()
        .init_resource::<Actions>()
        .init_resource::<super::pad::PadInput>()
        .add_message::<MouseWheel>()
        .add_systems(
            PreUpdate,
            collect
                .after(bevy::input::InputSystems)
                .after(super::pad::read),
        );
}
fn collect(
    (keys, mouse, pad): (
        Res<ButtonInput<KeyCode>>,
        Res<ButtonInput<MouseButton>>,
        Res<super::pad::PadInput>,
    ),
    bindings: Res<Bindings>,
    mut actions: ResMut<Actions>,
    motion: Option<Res<AccumulatedMouseMotion>>,
    mut wheel: MessageReader<MouseWheel>,
) {
    actions.axes.look = motion.map_or(Vec2::ZERO, |motion| motion.delta);
    actions.axes.look_rate = pad.look;
    actions.axes.movement = pad.movement;
    actions.axes.scroll.clear();
    actions
        .axes
        .scroll
        .extend(wheel.read().map(|event| ScrollDelta {
            notches: match event.unit {
                MouseScrollUnit::Line => event.y,
                MouseScrollUnit::Pixel => event.y / PIXELS_PER_NOTCH,
            },
            started: event.phase == TouchPhase::Started,
        }));
    actions.wheel = actions.press_wheel();
    let inputs = Inputs {
        keys: &keys,
        mouse: &mouse,
        pad: &pad.buttons,
        wheel: actions.wheel,
    };
    actions.resolve(&bindings, inputs, pad.cancel);
    // While a wheel chord's modifier is held it owns the wheel, including a
    // trackpad's part notches between its presses, which would otherwise
    // step the quick slots as it zooms.
    let chorded = bindings
        .keyboard
        .values()
        .flatten()
        .flatten()
        .any(|binding| {
            matches!(binding.trigger, Trigger::Wheel(_)) && binding.modifier_held(&keys)
        });
    if chorded {
        actions.axes.scroll.clear();
    }
}

#[cfg(test)]
pub(super) fn test_plugin(app: &mut App) {
    app.init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<ButtonInput<MouseButton>>();
    app.init_resource::<super::profile::Preferences>()
        .init_resource::<super::camera::Sprint>()
        .init_resource::<super::camera::PadLatch>();
    plugin(app);
}

#[cfg(test)]
mod tests {
    use super::*;
    static NO_PAD: std::sync::LazyLock<ButtonInput<GamepadButton>> =
        std::sync::LazyLock::new(ButtonInput::default);
    fn inputs<'a>(
        keys: &'a ButtonInput<KeyCode>,
        mouse: &'a ButtonInput<MouseButton>,
    ) -> Inputs<'a> {
        Inputs {
            keys,
            mouse,
            pad: &NO_PAD,
            wheel: WheelPress::default(),
        }
    }

    fn wheel(app: &mut App, y: f32, unit: MouseScrollUnit, phase: TouchPhase) {
        app.world_mut().write_message(MouseWheel {
            unit,
            x: 0.0,
            y,
            window: Entity::PLACEHOLDER,
            phase,
        });
        app.update();
    }

    #[test]
    fn an_alt_wheel_zooms_once_per_notch_and_never_also_steps_the_slots() {
        let mut app = App::new();
        app.add_plugins(test_plugin);
        let line = |app: &mut App, y| wheel(app, y, MouseScrollUnit::Line, TouchPhase::Moved);
        line(&mut app, 1.0);
        let actions = app.world().resource::<Actions>();
        assert!(
            !actions.started(ZoomIn),
            "a plain wheel is the quick slots'"
        );
        assert_eq!(actions.axes.scroll.len(), 1);

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::AltLeft);
        for _ in 0..3 {
            line(&mut app, 1.0);
            let actions = app.world().resource::<Actions>();
            assert!(actions.started(ZoomIn), "each notch, even back to back");
            assert!(!actions.started(ZoomOut));
            assert!(actions.axes.scroll.is_empty(), "the chord owns the wheel");
        }
        line(&mut app, -1.0);
        assert!(app.world().resource::<Actions>().started(ZoomOut));
        app.update();
        assert!(
            !app.world().resource::<Actions>().held(ZoomOut),
            "a notch lasts a frame"
        );
    }

    #[test]
    fn a_trackpad_zooms_once_its_pixels_make_a_notch() {
        let mut app = App::new();
        app.add_plugins(test_plugin);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::AltRight);
        let pixels = |app: &mut App, y, phase| wheel(app, y, MouseScrollUnit::Pixel, phase);
        pixels(&mut app, 60.0, TouchPhase::Started);
        assert!(!app.world().resource::<Actions>().started(ZoomIn));
        assert!(
            app.world().resource::<Actions>().axes.scroll.is_empty(),
            "part of a notch is the chord's too, never the quick slots'"
        );
        pixels(&mut app, 60.0, TouchPhase::Moved);
        assert!(app.world().resource::<Actions>().started(ZoomIn));
        // A new gesture drops what the last one left over.
        pixels(&mut app, 30.0, TouchPhase::Moved);
        pixels(&mut app, 80.0, TouchPhase::Started);
        assert!(!app.world().resource::<Actions>().started(ZoomIn));
    }

    #[test]
    fn both_sides_and_both_default_redo_gestures_resolve_once() {
        let bindings = Bindings::default();
        for modifier in [KeyCode::ControlLeft, KeyCode::ControlRight] {
            let mut keys = ButtonInput::default();
            keys.press(modifier);
            keys.press(KeyCode::KeyY);
            let mut actions = Actions::default();
            actions.resolve(&bindings, inputs(&keys, &ButtonInput::default()), false);
            assert!(actions.started(Redo));
            assert!(!actions.started(Undo));
            assert!(!actions.held(Descend));
        }
        for modifier in [
            KeyCode::ControlLeft,
            KeyCode::ControlRight,
            KeyCode::SuperLeft,
            KeyCode::SuperRight,
        ] {
            for shift in [KeyCode::ShiftLeft, KeyCode::ShiftRight] {
                let mut keys = ButtonInput::default();
                for key in [modifier, shift, KeyCode::KeyZ] {
                    keys.press(key);
                }
                let mut actions = Actions::default();
                actions.resolve(&bindings, inputs(&keys, &ButtonInput::default()), false);
                assert!(actions.started(Redo));
                assert!(!actions.started(Undo));
                assert!(!actions.held(Descend));
            }
        }
    }

    #[test]
    fn escape_remains_a_cancel_route_after_release_cursor_is_rebound() {
        let mut bindings = Bindings::default();
        bindings.keyboard.insert(ReleaseCursor, [None, None]);
        let mut actions = Actions::default();
        let mut keys = ButtonInput::default();
        keys.press(KeyCode::Escape);
        actions.resolve(&bindings, inputs(&keys, &ButtonInput::default()), false);
        assert!(actions.cancel_requested());
        actions.consume_cancel();
        assert!(!actions.cancel_requested());
    }

    #[test]
    fn a_second_alternative_does_not_restart_an_already_held_action() {
        let bindings = Bindings::default();
        let mut actions = Actions::default();
        let mut keys = ButtonInput::default();
        let mouse = ButtonInput::default();
        keys.press(KeyCode::ShiftLeft);
        actions.resolve(&bindings, inputs(&keys, &mouse), false);
        assert!(actions.started(Sprint));
        keys.clear();
        keys.press(KeyCode::ShiftRight);
        actions.resolve(&bindings, inputs(&keys, &mouse), false);
        assert!(!actions.started(Sprint));
        keys.clear();
        keys.release(KeyCode::ShiftLeft);
        actions.resolve(&bindings, inputs(&keys, &mouse), false);
        assert!(actions.held(Sprint));
        assert!(!actions.state(Sprint).released);
        keys.release(KeyCode::ShiftRight);
        actions.resolve(&bindings, inputs(&keys, &mouse), false);
        assert!(actions.state(Sprint).released);
    }

    #[test]
    fn logical_labels_do_not_change_physical_binding_ids() {
        let mut app = App::new();
        app.add_plugins(test_plugin);
        app.world_mut()
            .write_message(bevy::input::keyboard::KeyboardInput {
                key_code: KeyCode::KeyW,
                logical_key: bevy::input::keyboard::Key::Character("z".into()),
                state: bevy::input::ButtonState::Pressed,
                text: Some("z".into()),
                repeat: false,
                window: Entity::PLACEHOLDER,
            });
        app.update();
        let bindings = app.world().resource::<Bindings>();
        assert_eq!(
            bindings.label(Action::Forward, app.world().resource::<Labels>()),
            "Z"
        );
        assert_eq!(
            bindings.keyboard[&Action::Forward][0].unwrap().trigger,
            Trigger::Key(KeyCode::KeyW)
        );
    }

    #[test]
    fn history_chords_consume_descent_and_have_identical_edges_at_all_update_rates() {
        for (hz, modifier) in [60, 144, 1000].into_iter().flat_map(|hz| {
            [
                KeyCode::ControlLeft,
                KeyCode::ControlRight,
                KeyCode::SuperLeft,
                KeyCode::SuperRight,
            ]
            .map(|modifier| (hz, modifier))
        }) {
            let mut actions = Actions::default();
            let bindings = Bindings::default();
            let mut keys = ButtonInput::default();
            let mouse = ButtonInput::default();
            let mut undo = 0;
            let mut redo = 0;
            for frame in 0..hz {
                if frame == 0 {
                    keys.press(modifier);
                    keys.press(KeyCode::KeyZ);
                }
                if frame == hz / 2 {
                    keys.press(KeyCode::ShiftRight);
                }
                actions.resolve(&bindings, inputs(&keys, &mouse), false);
                keys.clear();
                undo += usize::from(actions.started(Undo));
                redo += usize::from(actions.started(Redo));
                assert!(!actions.held(Descend));
            }
            assert_eq!((undo, redo), (1, 1));
            assert!(actions.state(Undo).released || !actions.held(Undo));
            actions.cancel(Redo);
            assert!(actions.state(Redo).cancelled);
        }
    }
}
