//! The Client: rendering, input and camera. Never mutates World state, and has
//! no access to it — see `docs/adr/0002-client-server-from-day-one.md`.

mod aim;
mod blend_order;
pub mod camera;
#[cfg(feature = "dev")]
mod capture;
pub mod chunk_mesh;
pub mod chunk_view;
pub mod drawn_mask;
pub mod edit;
mod menu;
#[cfg(feature = "dev")]
mod overlay;
mod palette;
mod pose;
mod preview;
mod preview_material;
mod tile_preview;

use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.add_plugins((
        camera::plugin,
        chunk_view::plugin,
        edit::plugin,
        menu::plugin,
        aim::plugin,
        preview_material::plugin,
        preview::plugin,
        tile_preview::plugin,
        pose::plugin,
    ));
    #[cfg(feature = "dev")]
    app.add_plugins((
        bevy::diagnostic::FrameTimeDiagnosticsPlugin::default(),
        overlay::plugin,
        capture::plugin,
    ));
}
