//! Keyboard intents for the Server-owned edit history.
use crate::server::undo::HistoryAction;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

pub(super) fn plugin(app: &mut App) {
    app.add_message::<HistoryAction>().add_systems(
        Update,
        send.run_if(crate::server::persistence::playing)
            .before(super::removal::hold),
    );
}
fn action(keys: &ButtonInput<KeyCode>) -> Option<HistoryAction> {
    let ctrl = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    let command = keys.any_pressed([KeyCode::SuperLeft, KeyCode::SuperRight]);
    if keys.just_pressed(KeyCode::KeyZ) && (ctrl || command) {
        Some(
            if keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
                HistoryAction::Redo
            } else {
                HistoryAction::Undo
            },
        )
    } else if ctrl && keys.just_pressed(KeyCode::KeyY) {
        Some(HistoryAction::Redo)
    } else {
        None
    }
}
fn send(
    keys: Res<ButtonInput<KeyCode>>,
    cursor: Single<&CursorOptions, With<PrimaryWindow>>,
    menu: Res<super::menu::Menu>,
    mut intents: MessageWriter<HistoryAction>,
) {
    if !menu.open
        && cursor.grab_mode == CursorGrabMode::Locked
        && let Some(action) = action(&keys)
    {
        intents.write(action);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn supports_control_and_command_undo_and_redo_without_plain_letter_actions() {
        for modifier in [KeyCode::ControlLeft, KeyCode::SuperRight] {
            let mut keys = ButtonInput::default();
            keys.press(KeyCode::KeyZ);
            assert_eq!(action(&keys), None);
            keys.press(modifier);
            assert_eq!(action(&keys), Some(HistoryAction::Undo));
            keys.press(KeyCode::ShiftLeft);
            assert_eq!(action(&keys), Some(HistoryAction::Redo));
            keys.clear();
            assert_eq!(action(&keys), None);
        }
        let mut keys = ButtonInput::default();
        keys.press(KeyCode::ControlRight);
        keys.press(KeyCode::KeyY);
        assert_eq!(action(&keys), Some(HistoryAction::Redo));
    }
}
