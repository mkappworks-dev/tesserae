//! The Client: rendering, input and camera. Never mutates World state, and has
//! no access to it — see `docs/adr/0002-client-server-from-day-one.md`.

mod actions;
mod aim;
mod blend_order;
pub mod camera;
#[cfg(any(feature = "dev", feature = "profiling"))]
mod capture;
pub mod chunk_mesh;
pub mod chunk_view;
pub mod drawn_mask;
pub mod edit;
mod help;
mod history;
mod input;
mod material_textures;
mod menu;
#[cfg(feature = "dev")]
mod overlay;
mod palette;
mod pick;
mod pose;
pub(crate) mod presentation;
mod preview;
pub(crate) mod profile;
mod removal;
mod save;
mod settings;
mod tile_preview;

use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    #[cfg(any(feature = "dev", feature = "profiling"))]
    app.add_plugins(capture::plugin);
    app.add_plugins((
        save::plugin,
        profile::plugin,
        presentation::plugin,
        settings::plugin,
        actions::plugin,
        input::plugin,
    ));
    app.add_plugins((
        camera::plugin,
        chunk_view::plugin,
        edit::plugin,
        help::plugin,
        menu::plugin,
        aim::plugin,
        material_textures::plugin,
        preview::plugin,
        tile_preview::plugin,
        pose::plugin,
        pick::plugin,
        removal::plugin,
        history::plugin,
    ));
    #[cfg(feature = "dev")]
    app.add_plugins((
        bevy::diagnostic::FrameTimeDiagnosticsPlugin::default(),
        overlay::plugin,
    ));
}
