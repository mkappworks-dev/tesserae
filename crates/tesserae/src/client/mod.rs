//! The Client: rendering, input and camera. Never mutates World state, and has
//! no access to it — see `docs/adr/0002-client-server-from-day-one.md`.

mod actions;
mod aim;
mod avatar;
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
mod hotbar_store;
mod input;
pub(crate) mod loading;
mod material_textures;
mod menu;
mod menu_ui;
#[cfg(feature = "dev")]
mod overlay;
mod pad;
mod palette;
mod pick;
mod pose;
pub(crate) mod presentation;
mod preview;
pub(crate) mod profile;
mod removal;
mod save;
mod settings;
#[cfg(feature = "dev")]
mod soak;
mod tile_preview;
mod tool;
mod touch;
mod underwater;
mod view;
pub mod worlds;

use bevy::prelude::*;

use crate::session::SessionApp;

pub fn plugin(app: &mut App) {
    #[cfg(any(feature = "dev", feature = "profiling"))]
    app.add_plugins(capture::plugin);
    app.add_plugins((
        save::plugin,
        profile::plugin,
        presentation::plugin,
        settings::plugin,
        menu_ui::plugin,
        actions::plugin,
        input::plugin,
        pad::plugin,
        touch::plugin,
    ));
    app.add_plugins((
        camera::plugin,
        chunk_view::plugin,
        edit::plugin,
        hotbar_store::plugin,
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
        avatar::plugin,
    ));
    app.add_plugins((
        view::plugin,
        loading::plugin,
        tool::plugin,
        underwater::plugin,
    ));
    app.add_session_systems(
        Update,
        (|mut commands: Commands| commands.insert_resource(loading::TerrainArrived))
            .run_if(chunk_view::terrain_arrived)
            .run_if(not(resource_exists::<loading::TerrainArrived>)),
    );
    #[cfg(feature = "dev")]
    app.add_plugins((
        bevy::diagnostic::FrameTimeDiagnosticsPlugin::default(),
        overlay::plugin,
        soak::plugin,
    ));
}
