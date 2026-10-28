//! Stable actions and a single keyboard/mouse resolution pass.
use bevy::input::{
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Trigger {
    Key(KeyCode),
    Mouse(MouseButton),
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Binding {
    pub trigger: Trigger,
    pub modifier: Modifier,
}

impl Binding {
    pub fn active(&self, keys: &ButtonInput<KeyCode>, mouse: &ButtonInput<MouseButton>) -> bool {
        let pressed = match self.trigger {
            Trigger::Key(key) => keys.pressed(key),
            Trigger::Mouse(button) => mouse.pressed(button),
        };
        pressed
            && self.modifier.matches(
                keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]),
                keys.any_pressed([KeyCode::SuperLeft, KeyCode::SuperRight]),
                keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]),
                keys.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]),
            )
    }
}

#[derive(Resource, Clone, PartialEq, Debug)]
pub(super) struct Bindings(pub HashMap<Action, [Option<Binding>; 2]>);
impl Default for Bindings {
    fn default() -> Self {
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
            bindings.insert(
                action,
                [
                    Some(Binding {
                        trigger: Trigger::Key(key),
                        modifier: Modifier::None,
                    }),
                    None,
                ],
            );
        }
        for (action, button) in [(Remove, MouseButton::Left), (Place, MouseButton::Right)] {
            bindings.insert(
                action,
                [
                    Some(Binding {
                        trigger: Trigger::Mouse(button),
                        modifier: Modifier::None,
                    }),
                    None,
                ],
            );
        }
        bindings.get_mut(&Sprint).unwrap()[1] = Some(Binding {
            trigger: Trigger::Key(KeyCode::ShiftRight),
            modifier: Modifier::None,
        });
        bindings.get_mut(&Sample).unwrap()[1] = Some(Binding {
            trigger: Trigger::Mouse(MouseButton::Middle),
            modifier: Modifier::None,
        });
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
        Self(bindings)
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub(super) struct State {
    pub started: bool,
    pub held: bool,
    pub released: bool,
    pub cancelled: bool,
}
pub(super) const PIXELS_PER_NOTCH: f32 = 100.0;
#[derive(Clone, Copy, Debug)]
pub(super) struct ScrollDelta {
    pub notches: f32,
    pub started: bool,
}
#[derive(Default, Debug)]
pub(super) struct Axes {
    pub look: Vec2,
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
    pub fn cancel(&mut self, action: Action) {
        let previously_held = self.prior_effective_held.contains(&action);
        let state = self.states.entry(action).or_default();
        *state = State {
            cancelled: previously_held || state.started || state.cancelled,
            ..default()
        };
    }
    fn resolve(
        &mut self,
        bindings: &Bindings,
        keys: &ButtonInput<KeyCode>,
        mouse: &ButtonInput<MouseButton>,
    ) {
        self.prior_effective_held = self
            .states
            .iter()
            .filter_map(|(&action, state)| state.held.then_some(action))
            .collect();
        self.reserved_cancel = keys.just_pressed(KeyCode::Escape);
        let matches = |binding: Binding| binding.active(keys, mouse);
        let consumed: HashSet<_> = bindings
            .0
            .values()
            .flatten()
            .flatten()
            .filter(|binding| binding.modifier != Modifier::None && matches(**binding))
            .map(|binding| binding.trigger)
            .collect();
        // A history chord owns its modifier too, preventing vertical movement.
        let history = [Undo, Redo].into_iter().any(|action| {
            bindings.0.get(&action).is_some_and(|alternatives| {
                alternatives
                    .iter()
                    .flatten()
                    .any(|binding| matches(*binding))
            })
        });
        for (&action, alternatives) in &bindings.0 {
            let held = alternatives.iter().flatten().any(|binding| {
                matches(*binding)
                    && (binding.modifier != Modifier::None || !consumed.contains(&binding.trigger))
            }) && !(action == Descend && history);
            let just_pressed = |trigger| match trigger {
                Trigger::Key(key) => keys.just_pressed(key),
                Trigger::Mouse(button) => mouse.just_pressed(button),
            };
            let continuing = alternatives.iter().flatten().any(|binding| {
                matches(*binding)
                    && self.raw_triggers.contains(&binding.trigger)
                    && !just_pressed(binding.trigger)
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
                                && alternatives.iter().flatten().any(|binding| {
                                    matches(*binding) && just_pressed(binding.trigger)
                                }))),
                    held,
                    released: previous && !held,
                    cancelled: false,
                },
            );
        }
        self.raw_triggers = keys
            .get_pressed()
            .copied()
            .map(Trigger::Key)
            .chain(mouse.get_pressed().copied().map(Trigger::Mouse))
            .collect();
    }
}

#[derive(Resource, Default)]
pub(super) struct Labels(pub HashMap<KeyCode, String>);
impl Labels {
    pub fn binding(&self, binding: Binding) -> String {
        let trigger = match binding.trigger {
            Trigger::Key(key) => self.0.get(&key).cloned().unwrap_or_else(|| {
                let name = format!("{key:?}");
                name.strip_prefix("Key")
                    .or_else(|| name.strip_prefix("Digit"))
                    .unwrap_or(&name)
                    .to_owned()
            }),
            Trigger::Mouse(button) => format!("{button:?} mouse"),
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
}
impl Bindings {
    pub fn label(&self, action: Action, labels: &Labels) -> String {
        let alternatives: Vec<_> = self
            .0
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
            labels.0.insert(event.key_code, character.to_uppercase());
        }
    }
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Labels>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(PreUpdate, labels.after(bevy::input::InputSystems));
    app.init_resource::<Bindings>()
        .init_resource::<Actions>()
        .add_message::<MouseWheel>()
        .add_systems(PreUpdate, collect.after(bevy::input::InputSystems));
}
fn collect(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    bindings: Res<Bindings>,
    mut actions: ResMut<Actions>,
    motion: Option<Res<AccumulatedMouseMotion>>,
    mut wheel: MessageReader<MouseWheel>,
) {
    actions.resolve(&bindings, &keys, &mouse);
    actions.axes.look = motion.map_or(Vec2::ZERO, |motion| motion.delta);
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
}

#[cfg(test)]
pub(super) fn test_plugin(app: &mut App) {
    app.init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<ButtonInput<MouseButton>>();
    app.init_resource::<super::profile::Preferences>()
        .init_resource::<super::camera::Sprint>();
    plugin(app);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn both_sides_and_both_default_redo_gestures_resolve_once() {
        let bindings = Bindings::default();
        for modifier in [KeyCode::ControlLeft, KeyCode::ControlRight] {
            let mut keys = ButtonInput::default();
            keys.press(modifier);
            keys.press(KeyCode::KeyY);
            let mut actions = Actions::default();
            actions.resolve(&bindings, &keys, &ButtonInput::default());
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
                actions.resolve(&bindings, &keys, &ButtonInput::default());
                assert!(actions.started(Redo));
                assert!(!actions.started(Undo));
                assert!(!actions.held(Descend));
            }
        }
    }

    #[test]
    fn escape_remains_a_cancel_route_after_release_cursor_is_rebound() {
        let mut bindings = Bindings::default();
        bindings.0.insert(ReleaseCursor, [None, None]);
        let mut actions = Actions::default();
        let mut keys = ButtonInput::default();
        keys.press(KeyCode::Escape);
        actions.resolve(&bindings, &keys, &ButtonInput::default());
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
        actions.resolve(&bindings, &keys, &mouse);
        assert!(actions.started(Sprint));
        keys.clear();
        keys.press(KeyCode::ShiftRight);
        actions.resolve(&bindings, &keys, &mouse);
        assert!(!actions.started(Sprint));
        keys.clear();
        keys.release(KeyCode::ShiftLeft);
        actions.resolve(&bindings, &keys, &mouse);
        assert!(actions.held(Sprint));
        assert!(!actions.state(Sprint).released);
        keys.release(KeyCode::ShiftRight);
        actions.resolve(&bindings, &keys, &mouse);
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
            bindings.0[&Action::Forward][0].unwrap().trigger,
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
                actions.resolve(&bindings, &keys, &mouse);
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
