//! Input ownership is settled before any gameplay consumer runs.
use bevy::input::gamepad::GamepadButton;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use std::collections::HashSet;

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(super) enum Owner {
    Gameplay,
    PointerFree,
    Catalogue,
    Settings,
    BindingCapture,
    TextEntry,
    Modal,
    #[default]
    Unfocused,
}

#[derive(Resource, Default)]
pub(super) struct Ownership {
    pub owner: Owner,
    pub changed: bool,
    keys: HashSet<KeyCode>,
    buttons: HashSet<MouseButton>,
    pads: HashSet<GamepadButton>,
    device_epoch: u64,
}

impl Ownership {
    pub fn gameplay(&self) -> bool {
        self.owner == Owner::Gameplay && !self.changed
    }
    pub fn key(&self, key: KeyCode) -> bool {
        self.gameplay() && !self.keys.contains(&key)
    }
    pub fn button(&self, button: MouseButton) -> bool {
        self.gameplay() && !self.buttons.contains(&button)
    }
    pub fn pad(&self, button: GamepadButton) -> bool {
        self.gameplay() && !self.pads.contains(&button)
    }
    #[cfg(test)]
    pub fn playing() -> Self {
        Self {
            owner: Owner::Gameplay,
            ..default()
        }
    }
    fn resolve(&mut self, owner: Owner, inputs: super::actions::Inputs) {
        self.changed = self.owner != owner;
        self.owner = owner;
        self.keys.retain(|key| inputs.keys.pressed(*key));
        self.buttons.retain(|button| inputs.mouse.pressed(*button));
        self.pads.retain(|button| inputs.pad.pressed(*button));
        if self.changed || owner != Owner::Gameplay {
            self.own_pressed(inputs);
        }
    }
    fn own_pressed(&mut self, inputs: super::actions::Inputs) {
        self.keys.extend(inputs.keys.get_pressed().copied());
        self.buttons.extend(inputs.mouse.get_pressed().copied());
        self.pads.extend(inputs.pad.get_pressed().copied());
    }
}

#[derive(Resource, Default)]
pub(super) struct UiInput {
    pub text_entry: bool,
    pub device_epoch: u64,
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<UiInput>()
        .init_resource::<Ownership>()
        .add_systems(
            Update,
            resolve
                .after(super::menu::toggle)
                .after(super::camera::grab_cursor),
        );
}

type Contexts<'w> = (
    Res<'w, super::menu::Menu>,
    Option<Res<'w, super::settings::Settings>>,
    Option<Res<'w, UiInput>>,
    Option<Res<'w, crate::server::persistence::SaveStatus>>,
);

pub(super) fn resolve(
    (keys, mouse, pad): super::actions::Devices,
    (menu, settings, ui, save): Contexts,
    window: Single<(&Window, &CursorOptions), With<PrimaryWindow>>,
    mut ownership: ResMut<Ownership>,
    bindings: Res<super::actions::Bindings>,
    mut actions: ResMut<super::actions::Actions>,
) {
    let (window, cursor) = *window;
    let owner = if !window.focused {
        Owner::Unfocused
    } else if save.is_some_and(|status| status.modal()) {
        Owner::Modal
    } else if ui.as_ref().is_some_and(|ui| ui.text_entry) {
        Owner::TextEntry
    } else if settings
        .as_ref()
        .is_some_and(|settings| settings.capturing())
    {
        Owner::BindingCapture
    } else if settings.as_ref().is_some_and(|settings| settings.open) {
        Owner::Settings
    } else if menu.open {
        Owner::Catalogue
    } else if cursor.grab_mode == CursorGrabMode::Locked {
        Owner::Gameplay
    } else {
        Owner::PointerFree
    };
    let inputs = super::actions::Inputs {
        keys: &keys,
        mouse: &mouse,
        pad: &pad.buttons,
    };
    // Every pad button, including those a menu withholds from actions, so one
    // held while a menu closes is still owned until it is released.
    let held = super::actions::Inputs {
        pad: &pad.navigation,
        ..inputs
    };
    ownership.resolve(owner, held);
    if let Some(ui) = ui
        && ui.device_epoch != ownership.device_epoch
    {
        ownership.device_epoch = ui.device_epoch;
        ownership.changed = true;
        ownership.own_pressed(held);
    }

    if !ownership.gameplay() {
        actions.axes = default();
    }
    use super::actions::{Action, Trigger};
    for (&action, alternatives) in &bindings.keyboard {
        if matches!(
            action,
            Action::Catalogue | Action::ReleaseCursor | Action::Help | Action::Diagnostics
        ) {
            if ownership.changed
                || matches!(
                    owner,
                    Owner::Unfocused
                        | Owner::Settings
                        | Owner::BindingCapture
                        | Owner::TextEntry
                        | Owner::Modal
                )
            {
                actions.cancel(action);
            }
            continue;
        }
        // Catalogue shortcuts are keyboard ones; the pad navigates there.
        let catalogue_action = owner == Owner::Catalogue
            && !ownership.changed
            && (super::actions::SLOTS.contains(&action) || action == Action::EmptyHand)
            && alternatives
                .iter()
                .flatten()
                .any(|binding| binding.active(inputs));
        if !catalogue_action
            && (!ownership.gameplay()
                || bindings.alternatives(action).all(|binding| {
                    !binding.active(inputs)
                        || match binding.trigger {
                            Trigger::Key(key) => !ownership.key(key),
                            Trigger::Mouse(button) => !ownership.button(button),
                            Trigger::Pad(button) => !ownership.pad(button),
                        }
                }))
        {
            actions.cancel(action);
        }
    }
}

pub(super) fn gameplay(ownership: Res<Ownership>) -> bool {
    ownership.gameplay()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn leaving_search_cancels_same_frame_catalogue_slot_input_until_a_fresh_press() {
        use super::super::actions::{Action, Actions};
        for (action, key) in [
            (Action::Slot5, KeyCode::Digit5),
            (Action::EmptyHand, KeyCode::KeyX),
        ] {
            let mut app = App::new();
            app.add_plugins(super::super::actions::test_plugin)
                .init_resource::<Ownership>()
                .init_resource::<super::super::menu::Menu>()
                .insert_resource(UiInput {
                    text_entry: true,
                    ..default()
                })
                .add_systems(Update, resolve);
            app.world_mut()
                .resource_mut::<super::super::menu::Menu>()
                .open = true;
            app.world_mut().spawn((
                PrimaryWindow,
                Window {
                    focused: true,
                    ..default()
                },
                CursorOptions::default(),
            ));
            app.update();
            app.world_mut().resource_mut::<UiInput>().text_entry = false;
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(key);
            app.update();
            assert_eq!(app.world().resource::<Ownership>().owner, Owner::Catalogue);
            assert!(!app.world().resource::<Actions>().started(action));
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .reset_all();
            app.update();
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(key);
            app.update();
            assert!(app.world().resource::<Actions>().started(action));
        }
    }
    #[test]
    fn save_modals_cancel_input_until_fresh_activation() {
        use super::super::actions::{Action, Actions};
        use crate::server::persistence::SaveStatus;
        let mut app = App::new();
        app.add_plugins(super::super::actions::test_plugin)
            .insert_resource(Ownership::playing())
            .init_resource::<super::super::menu::Menu>()
            .init_resource::<SaveStatus>()
            .add_systems(Update, resolve);
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
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyR);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Right);
        app.update();
        assert!(app.world().resource::<Actions>().started(Action::Respawn));
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .clear();
        app.world_mut().resource_mut::<SaveStatus>().quitting = true;
        app.update();
        assert_eq!(app.world().resource::<Ownership>().owner, Owner::Modal);
        assert!(!app.world().resource::<Actions>().held(Action::Place));
        app.world_mut().resource_mut::<SaveStatus>().quitting = false;
        app.update();
        app.update();
        assert!(!app.world().resource::<Actions>().held(Action::Place));
        assert!(!app.world().resource::<Actions>().held(Action::Respawn));
    }

    #[test]
    fn ownership_discards_look_and_scroll_in_the_shared_frame() {
        use bevy::input::mouse::{AccumulatedMouseMotion, MouseScrollUnit, MouseWheel};
        use bevy::input::touch::TouchPhase;
        let mut app = App::new();
        app.add_plugins(super::super::actions::test_plugin)
            .insert_resource(Ownership::playing())
            .init_resource::<super::super::menu::Menu>()
            .init_resource::<AccumulatedMouseMotion>()
            .add_systems(Update, resolve);
        let window = app
            .world_mut()
            .spawn((
                Window {
                    focused: true,
                    ..default()
                },
                PrimaryWindow,
                CursorOptions {
                    grab_mode: CursorGrabMode::Locked,
                    ..default()
                },
            ))
            .id();
        app.world_mut()
            .resource_mut::<super::super::menu::Menu>()
            .open = true;
        app.world_mut()
            .resource_mut::<AccumulatedMouseMotion>()
            .delta = Vec2::new(4.0, 8.0);
        app.world_mut().write_message(MouseWheel {
            unit: MouseScrollUnit::Line,
            x: 0.0,
            y: -1.0,
            window,
            phase: TouchPhase::Moved,
        });
        app.update();
        let actions = app.world().resource::<super::super::actions::Actions>();
        assert_eq!(actions.axes.look, Vec2::ZERO);
        assert!(actions.axes.scroll.is_empty());
        app.world_mut()
            .resource_mut::<AccumulatedMouseMotion>()
            .delta = Vec2::ZERO;
        app.world_mut()
            .resource_mut::<super::super::menu::Menu>()
            .open = false;
        app.update();
        assert!(
            app.world()
                .resource::<super::super::actions::Actions>()
                .axes
                .scroll
                .is_empty()
        );
        app.world_mut()
            .resource_mut::<AccumulatedMouseMotion>()
            .delta = Vec2::new(2.0, 3.0);
        app.update();
        assert_eq!(
            app.world()
                .resource::<super::super::actions::Actions>()
                .axes
                .look,
            Vec2::new(2.0, 3.0)
        );
    }

    #[test]
    fn device_cancellation_is_one_transition_at_every_update_rate() {
        use super::super::actions::{Action, Actions};
        for hz in [60, 144, 1000] {
            let mut app = App::new();
            app.add_plugins(super::super::actions::test_plugin)
                .insert_resource(Ownership::playing())
                .init_resource::<UiInput>()
                .init_resource::<super::super::menu::Menu>()
                .add_systems(Update, resolve);
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
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::KeyR);
            let mut starts = 0;
            let mut cancellations = 0;
            for frame in 0..hz {
                if frame == hz / 2 {
                    app.world_mut().resource_mut::<UiInput>().device_epoch += 1;
                }
                app.update();
                let state = app.world().resource::<Actions>().state(Action::Respawn);
                starts += usize::from(state.started);
                cancellations += usize::from(state.cancelled);
                if frame >= hz / 2 {
                    assert!(!state.held);
                }
                app.world_mut()
                    .resource_mut::<ButtonInput<KeyCode>>()
                    .clear();
            }
            assert_eq!((starts, cancellations), (1, 1));
        }
    }

    #[test]
    fn an_idle_alternative_cannot_resume_a_cancelled_held_binding() {
        use super::super::actions::{Action, Actions, Binding, Bindings, Modifier, Trigger};
        let mut app = App::new();
        app.add_plugins(super::super::actions::test_plugin)
            .insert_resource(Ownership::playing())
            .init_resource::<super::super::menu::Menu>()
            .add_systems(Update, resolve);
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
            .resource_mut::<Bindings>()
            .keyboard
            .get_mut(&Action::Forward)
            .unwrap()[1] = Some(Binding {
            trigger: Trigger::Key(KeyCode::KeyT),
            modifier: Modifier::None,
        });
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyW);
        app.world_mut()
            .resource_mut::<super::super::menu::Menu>()
            .open = true;
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.world_mut()
            .resource_mut::<super::super::menu::Menu>()
            .open = false;
        app.update();
        app.update();
        assert!(!app.world().resource::<Actions>().held(Action::Forward));
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::KeyW);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyW);
        app.update();
        assert!(app.world().resource::<Actions>().started(Action::Forward));
    }

    #[test]
    fn held_inputs_require_release_after_every_ownership_transition() {
        let mut ownership = Ownership::playing();
        let mut keys = ButtonInput::default();
        let mut mouse = ButtonInput::default();
        keys.press(KeyCode::KeyR);
        mouse.press(MouseButton::Left);
        let pad = ButtonInput::default();
        fn inputs<'a>(
            keys: &'a ButtonInput<KeyCode>,
            mouse: &'a ButtonInput<MouseButton>,
            pad: &'a ButtonInput<GamepadButton>,
        ) -> super::super::actions::Inputs<'a> {
            super::super::actions::Inputs { keys, mouse, pad }
        }
        ownership.resolve(Owner::Catalogue, inputs(&keys, &mouse, &pad));
        ownership.resolve(Owner::Gameplay, inputs(&keys, &mouse, &pad));
        ownership.resolve(Owner::Gameplay, inputs(&keys, &mouse, &pad));
        assert!(!ownership.key(KeyCode::KeyR));
        assert!(!ownership.button(MouseButton::Left));
        keys.release(KeyCode::KeyR);
        mouse.release(MouseButton::Left);
        ownership.resolve(Owner::Gameplay, inputs(&keys, &mouse, &pad));
        keys.press(KeyCode::KeyR);
        mouse.press(MouseButton::Left);
        ownership.resolve(Owner::Gameplay, inputs(&keys, &mouse, &pad));
        assert!(ownership.key(KeyCode::KeyR));
        assert!(ownership.button(MouseButton::Left));
    }
}
