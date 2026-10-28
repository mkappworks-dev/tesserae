//! Stable actions and a single keyboard/mouse resolution pass.
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
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Binding {
    pub trigger: Trigger,
    pub modifier: Modifier,
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
#[derive(Resource, Default)]
pub(super) struct Actions(pub HashMap<Action, State>, HashSet<Action>);
impl Actions {
    pub fn state(&self, action: Action) -> State {
        self.0.get(&action).copied().unwrap_or_default()
    }
    pub fn started(&self, action: Action) -> bool {
        self.state(action).started
    }
    pub fn held(&self, action: Action) -> bool {
        self.state(action).held
    }
    pub fn cancel(&mut self, action: Action) {
        let state = self.0.entry(action).or_default();
        *state = State {
            cancelled: state.held,
            ..default()
        };
    }
    fn resolve(
        &mut self,
        bindings: &Bindings,
        keys: &ButtonInput<KeyCode>,
        mouse: &ButtonInput<MouseButton>,
    ) {
        let ctrl = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
        let primary = ctrl || keys.any_pressed([KeyCode::SuperLeft, KeyCode::SuperRight]);
        let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
        let matches = |binding: Binding| {
            let trigger = match binding.trigger {
                Trigger::Key(key) => keys.pressed(key),
                Trigger::Mouse(button) => mouse.pressed(button),
            };
            trigger
                && match binding.modifier {
                    Modifier::None => true,
                    Modifier::Control => ctrl,
                    Modifier::Primary => primary && !shift,
                    Modifier::PrimaryShift => primary && shift,
                }
        };
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
            let previous = self.1.contains(&action);
            if held {
                self.1.insert(action);
            } else {
                self.1.remove(&action);
            }
            self.0.insert(
                action,
                State {
                    started: held
                        && (!previous
                            || alternatives.iter().flatten().any(|binding| {
                                matches(*binding)
                                    && match binding.trigger {
                                        Trigger::Key(key) => keys.just_pressed(key),
                                        Trigger::Mouse(button) => mouse.just_pressed(button),
                                    }
                            })),
                    held,
                    released: previous && !held,
                    cancelled: false,
                },
            );
        }
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
        .add_systems(PreUpdate, collect.after(bevy::input::InputSystems));
}
fn collect(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    bindings: Res<Bindings>,
    mut actions: ResMut<Actions>,
) {
    actions.resolve(&bindings, &keys, &mouse);
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
    fn history_chords_consume_descent_and_have_identical_edges_at_all_update_rates() {
        for hz in [60, 144, 1000] {
            let mut actions = Actions::default();
            let bindings = Bindings::default();
            let mut keys = ButtonInput::default();
            let mouse = ButtonInput::default();
            let mut undo = 0;
            let mut redo = 0;
            for frame in 0..hz {
                if frame == 0 {
                    keys.press(KeyCode::ControlRight);
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
