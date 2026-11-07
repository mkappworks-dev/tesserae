//! The Player's game mode as the Server last told it. Only what the Client
//! shows depends on it; the Server enforces the mode.

use bevy::prelude::*;

use crate::server::game_mode::{GameMode, GameModeChanged};
use crate::session::SessionApp;

/// `None` until the Server has said.
#[derive(Resource, Default)]
pub(super) struct Mode(pub(super) Option<GameMode>);

impl Mode {
    pub(super) fn survival(&self) -> bool {
        self.0 == Some(GameMode::Survival)
    }
}

pub(super) fn plugin(app: &mut App) {
    app.init_session_state::<Mode>()
        .add_session_message::<GameModeChanged>()
        .add_session_systems(Update, receive);
}

pub(super) fn receive(mut changes: MessageReader<GameModeChanged>, mut mode: ResMut<Mode>) {
    if let Some(GameModeChanged(now)) = changes.read().last() {
        mode.0 = Some(*now);
    }
}
