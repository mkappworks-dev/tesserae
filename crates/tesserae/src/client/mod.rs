//! The Client: rendering, input and camera. Never mutates World state —
//! see `docs/adr/0002-client-server-from-day-one.md`.

pub mod camera;
pub mod chunk_mesh;

use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.add_plugins(camera::plugin);
}
