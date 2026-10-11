//! The Tesserae engine's Bevy application.

mod client;
mod render;
mod server;
mod world;

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Tesserae".into(),
                ..default()
            }),
            ..default()
        }))
        .init_resource::<world::WorldSettings>()
        .add_plugins((render::plugin, server::plugin, client::plugin))
        .run();
}
