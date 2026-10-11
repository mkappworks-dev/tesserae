//! The Client: rendering, input and camera. Never mutates World state, and has
//! no access to it — see `docs/adr/0002-client-server-from-day-one.md`.

pub mod camera;
#[cfg(feature = "dev")]
mod capture;
pub mod chunk_mesh;
pub mod chunk_view;
pub mod edit;
#[cfg(feature = "dev")]
mod overlay;

use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.add_plugins((camera::plugin, chunk_view::plugin, edit::plugin));
    #[cfg(feature = "dev")]
    app.add_plugins((
        bevy::diagnostic::FrameTimeDiagnosticsPlugin::default(),
        overlay::plugin,
        capture::plugin,
    ));
}
