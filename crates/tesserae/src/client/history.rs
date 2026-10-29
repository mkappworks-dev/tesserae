//! Keyboard intents for the Server-owned edit history.
use crate::server::undo::HistoryAction;
use bevy::prelude::*;

pub(super) fn plugin(app: &mut App) {
    app.add_message::<HistoryAction>().add_systems(
        Update,
        send.run_if(crate::server::persistence::playing)
            .after(super::input::resolve)
            .before(super::removal::hold),
    );
}
fn send(
    ownership: Res<super::input::Ownership>,
    actions: Res<super::actions::Actions>,
    mut intents: MessageWriter<HistoryAction>,
) {
    if !ownership.gameplay() {
        return;
    }
    if actions.started(super::actions::Action::Redo) {
        intents.write(HistoryAction::Redo);
    } else if actions.started(super::actions::Action::Undo) {
        intents.write(HistoryAction::Undo);
    }
}
