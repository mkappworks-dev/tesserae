//! Keyboard intents for the Server-owned edit history.
use crate::server::undo::HistoryAction;
use bevy::prelude::*;

use crate::session::SessionApp;

pub(super) fn plugin(app: &mut App) {
    app.add_session_message::<HistoryAction>()
        .add_session_systems(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::edit::{Holding, Hotbar};
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn undo_and_redo_stay_live_with_a_tool_in_hand() {
        let mut app = App::new();
        app.add_plugins(crate::client::actions::test_plugin)
            .insert_resource(crate::client::input::Ownership::playing())
            .init_resource::<Hotbar>()
            .add_message::<HistoryAction>();
        app.world_mut().resource_mut::<Hotbar>().outside =
            Some(Holding::Tool(crate::server::tool::ToolId(0)));
        for (extra, expected) in [
            (None, HistoryAction::Undo),
            (Some(KeyCode::ShiftLeft), HistoryAction::Redo),
        ] {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.release_all();
            keys.clear();
            keys.press(KeyCode::ControlLeft);
            keys.press(KeyCode::KeyZ);
            if let Some(extra) = extra {
                keys.press(extra);
            }
            app.world_mut().run_schedule(PreUpdate);
            app.world_mut().run_system_once(send).unwrap();
            let sent: Vec<_> = app
                .world_mut()
                .resource_mut::<Messages<HistoryAction>>()
                .drain()
                .collect();
            assert_eq!(sent, vec![expected]);
        }
    }
}
