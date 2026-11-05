//! The Player state a commit holds beside the Eye, one registration per key.

use std::collections::BTreeMap;

use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use serde_json::Value;
use tesserae_core::save::{self, PlayerExtension};

/// Values by key, as a commit holds them.
pub(super) type Saved = BTreeMap<String, Value>;

/// Player state saved under [`PlayerExtension::KEY`].
pub(in crate::server) trait PlayerState: PlayerExtension {
    /// `None` leaves the key out of the save.
    fn capture(player: EntityRef) -> Option<Self>;
    /// Hands a reopened value to the Player once it exists.
    fn restore(self, player: &mut EntityCommands);
}

/// Every key this build saves and opens. A save holding any other is
/// refused, which is how an older build refuses a newer one's state instead of
/// dropping it on its next save.
const REGISTERED: &[Extension] = &[
    Extension::of::<save::MovementV1>(),
    Extension::of::<save::CarriedV1>(),
    Extension::of::<save::InventoryV1>(),
    Extension::of::<save::GameModeV1>(),
    Extension::of::<save::VitalsV1>(),
    #[cfg(test)]
    Extension::of::<super::tests::Probe>(),
];

struct Extension {
    key: &'static str,
    read: fn(&Value) -> Result<Value, save::Error>,
    capture: fn(EntityRef) -> Result<Option<Value>, save::Error>,
    restore: fn(Value, &mut EntityCommands),
}

impl Extension {
    const fn of<E: PlayerState>() -> Self {
        Self {
            key: E::KEY,
            read: read_one::<E>,
            capture: capture_one::<E>,
            restore: restore_one::<E>,
        }
    }
}

fn registered(key: &str) -> Option<&'static Extension> {
    REGISTERED.iter().find(|e| e.key == key)
}

fn to_value<E: PlayerExtension>(value: &E) -> Result<Value, save::Error> {
    serde_json::to_value(value).map_err(|e| save::Error::Corrupt(format!("{}: {e}", E::KEY)))
}

/// Written back through the type, so a saved value equals a fresh capture of
/// the same state however an older build spelled it.
fn read_one<E: PlayerState>(value: &Value) -> Result<Value, save::Error> {
    to_value(&E::read(value)?)
}

/// Checked as an opening would check it, so a save never holds what it
/// would refuse.
fn capture_one<E: PlayerState>(player: EntityRef) -> Result<Option<Value>, save::Error> {
    E::capture(player)
        .map(|state| {
            state.validate()?;
            to_value(&state)
        })
        .transpose()
}

fn restore_one<E: PlayerState>(value: Value, player: &mut EntityCommands) {
    if let Ok(state) = E::read(&value) {
        state.restore(player);
    }
}

/// A save's extensions as this build opens them.
pub(super) fn read(player: &save::Player) -> Result<Saved, save::Error> {
    player
        .extensions
        .iter()
        .map(|(key, value)| {
            let extension = registered(key).ok_or_else(|| {
                save::Error::Incompatible(format!(
                    "unknown Player extension {key} requires migration"
                ))
            })?;
            Ok((key.clone(), (extension.read)(value)?))
        })
        .collect()
}

/// The Player's extensions as a save would hold them now.
pub(super) fn capture(player: EntityRef) -> Result<Saved, save::Error> {
    let mut saved = Saved::new();
    for extension in REGISTERED {
        if let Some(value) = (extension.capture)(player)? {
            saved.insert(extension.key.into(), value);
        }
    }
    Ok(saved)
}

/// Values [`read`] accepted, handed to the Player.
pub(super) fn restore(saved: Saved, player: &mut EntityCommands) {
    for (key, value) in saved {
        if let Some(extension) = registered(&key) {
            (extension.restore)(value, player);
        }
    }
}
