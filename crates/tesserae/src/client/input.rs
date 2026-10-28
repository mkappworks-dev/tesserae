//! Input ownership is settled before any gameplay consumer runs.
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use std::collections::HashSet;

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(super) enum Owner {
    Gameplay,
    Catalogue,
    Settings,
    #[default]
    Unfocused,
}

#[derive(Resource, Default)]
pub(super) struct Ownership {
    pub owner: Owner,
    pub changed: bool,
    keys: HashSet<KeyCode>,
    buttons: HashSet<MouseButton>,
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
    #[cfg(test)]
    pub fn playing() -> Self {
        Self {
            owner: Owner::Gameplay,
            ..default()
        }
    }
    fn resolve(
        &mut self,
        owner: Owner,
        keys: &ButtonInput<KeyCode>,
        mouse: &ButtonInput<MouseButton>,
    ) {
        self.changed = self.owner != owner;
        self.owner = owner;
        self.keys.retain(|key| keys.pressed(*key));
        self.buttons.retain(|button| mouse.pressed(*button));
        if self.changed || owner != Owner::Gameplay {
            self.keys.extend(keys.get_pressed().copied());
            self.buttons.extend(mouse.get_pressed().copied());
        }
    }
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Ownership>().add_systems(
        Update,
        resolve
            .after(super::menu::toggle)
            .after(super::camera::grab_cursor),
    );
}

pub(super) fn resolve(
    (keys, mouse): (Res<ButtonInput<KeyCode>>, Res<ButtonInput<MouseButton>>),
    menu: Res<super::menu::Menu>,
    settings: Option<Res<super::settings::Settings>>,
    window: Single<(&Window, &CursorOptions), With<PrimaryWindow>>,
    mut ownership: ResMut<Ownership>,
    bindings: Res<super::actions::Bindings>,
    mut actions: ResMut<super::actions::Actions>,
) {
    let (window, cursor) = *window;
    let owner = if !window.focused {
        Owner::Unfocused
    } else if settings.is_some_and(|settings| settings.open) {
        Owner::Settings
    } else if menu.open {
        Owner::Catalogue
    } else if cursor.grab_mode == CursorGrabMode::Locked {
        Owner::Gameplay
    } else {
        Owner::Unfocused
    };
    ownership.resolve(owner, &keys, &mouse);
    use super::actions::{Action, Trigger};
    for (&action, alternatives) in &bindings.0 {
        if matches!(
            action,
            Action::Catalogue | Action::ReleaseCursor | Action::Help | Action::Diagnostics
        ) {
            if matches!(owner, Owner::Unfocused | Owner::Settings)
                || alternatives
                    .iter()
                    .flatten()
                    .all(|binding| match binding.trigger {
                        Trigger::Key(key) => ownership.keys.contains(&key),
                        Trigger::Mouse(button) => ownership.buttons.contains(&button),
                    })
            {
                actions.cancel(action);
            }
            continue;
        }
        let catalogue_slot = owner == Owner::Catalogue && super::actions::SLOTS.contains(&action);
        if !catalogue_slot
            && (!ownership.gameplay()
                || alternatives
                    .iter()
                    .flatten()
                    .all(|binding| match binding.trigger {
                        Trigger::Key(key) => !ownership.key(key),
                        Trigger::Mouse(button) => !ownership.button(button),
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
    fn held_inputs_require_release_after_every_ownership_transition() {
        let mut ownership = Ownership::playing();
        let mut keys = ButtonInput::default();
        let mut mouse = ButtonInput::default();
        keys.press(KeyCode::KeyR);
        mouse.press(MouseButton::Left);
        ownership.resolve(Owner::Catalogue, &keys, &mouse);
        ownership.resolve(Owner::Gameplay, &keys, &mouse);
        ownership.resolve(Owner::Gameplay, &keys, &mouse);
        assert!(!ownership.key(KeyCode::KeyR));
        assert!(!ownership.button(MouseButton::Left));
        keys.release(KeyCode::KeyR);
        mouse.release(MouseButton::Left);
        ownership.resolve(Owner::Gameplay, &keys, &mouse);
        keys.press(KeyCode::KeyR);
        mouse.press(MouseButton::Left);
        ownership.resolve(Owner::Gameplay, &keys, &mouse);
        assert!(ownership.key(KeyCode::KeyR));
        assert!(ownership.button(MouseButton::Left));
    }
}
