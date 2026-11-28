//! The view's shimmer and light tint under water: a full-screen pass on the Player's camera while
//! it is under a water surface, before tonemapping, so the hand and the HUD stay still.

use bevy::core_pipeline::fullscreen_material::{FullscreenMaterial, FullscreenMaterialPlugin};
use bevy::prelude::*;
use bevy::render::RenderApp;
use bevy::render::extract_component::ExtractComponent;
use bevy::render::render_resource::ShaderType;
use bevy::shader::ShaderRef;

#[derive(Component, ExtractComponent, Clone, Copy, ShaderType, Default, Debug, PartialEq)]
#[extract_app(RenderApp)]
pub struct Wobble {
    pub time_s: f32,
    /// How far a pixel is read off, as a share of the view's height.
    pub strength: f32,
    /// How much of `hue` is laid over the view.
    pub tint: f32,
    /// Linear.
    pub hue: Vec3,
}

impl FullscreenMaterial for Wobble {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(
            bevy::asset::AssetPath::from_path_buf(bevy::asset::embedded_path!("wobble.wesl"))
                .with_source("embedded"),
        )
    }
}

pub(super) fn plugin(app: &mut App) {
    bevy::asset::embedded_asset!(app, "wobble.wesl");
    app.add_plugins(FullscreenMaterialPlugin::<Wobble>::default());
}
