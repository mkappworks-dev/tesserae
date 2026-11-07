//! Each Player's game mode. Only the Server sets it, and the Client is told;
//! see `docs/adr/0002-client-server-from-day-one.md`.

use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
pub use tesserae_core::save::GameMode;
use tesserae_core::save::GameModeV1;

use super::persistence::extensions::PlayerState;
use crate::session::SessionApp;

#[derive(Component, Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct PlayerMode(pub GameMode);

impl PlayerMode {
    pub fn survival(self) -> bool {
        self.0 == GameMode::Survival
    }
}

/// The Player's mode, sent when they arrive and whenever it changes.
#[derive(Message, Clone, Copy, PartialEq, Eq, Debug)]
pub struct GameModeChanged(pub GameMode);

pub(crate) fn plugin(app: &mut App) {
    app.add_session_message::<GameModeChanged>()
        .add_session_systems(FixedPreUpdate, announce);
}

fn announce(mode: Single<Ref<PlayerMode>>, mut changed: MessageWriter<GameModeChanged>) {
    if mode.is_changed() {
        changed.write(GameModeChanged(mode.0));
    }
}

impl PlayerState for GameModeV1 {
    fn capture(player: EntityRef) -> Option<Self> {
        player.get::<PlayerMode>().map(|m| GameModeV1(m.0))
    }

    /// Applied by `spawn_player` instead: the mode decides whether the Player
    /// may spawn flying.
    fn restore(self, _: &mut EntityCommands) {}
}

#[cfg(test)]
mod tests {
    use bevy::ecs::message::Messages;

    use super::*;

    #[test]
    fn the_mode_is_sent_when_the_player_arrives_and_when_it_changes() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_plugins(plugin);
        let player = app.world_mut().spawn(PlayerMode::default()).id();
        let sent = |app: &mut App| {
            app.world_mut().run_schedule(FixedPreUpdate);
            app.world_mut()
                .resource_mut::<Messages<GameModeChanged>>()
                .drain()
                .collect::<Vec<_>>()
        };
        assert_eq!(sent(&mut app), [GameModeChanged(GameMode::Creative)]);
        assert!(sent(&mut app).is_empty());
        app.world_mut()
            .entity_mut(player)
            .insert(PlayerMode(GameMode::Survival));
        assert_eq!(sent(&mut app), [GameModeChanged(GameMode::Survival)]);
    }
}
