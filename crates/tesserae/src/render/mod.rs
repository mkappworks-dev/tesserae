//! Materials. Step 9 adds the look-pack API that supplies their WESL.

pub mod chunk_material;
pub mod fog;
pub(crate) mod texture_atlas;
pub(crate) mod tile_icon;
mod view_caches;

use bevy::asset::embedded_asset;
use bevy::pbr::MaterialPlugin;
use bevy::prelude::*;
use bevy::shader::load_shader_library;

use chunk_material::ChunkMaterial;

pub fn plugin(app: &mut App) {
    // Embedded rather than loaded from `assets/`: the engine's own shaders ship
    // in the binary, as `bevy_pbr`'s do. A look pack's WESL is an asset because
    // it is content, which is the opposite case.
    embedded_asset!(app, "chunk_material.wesl");
    embedded_asset!(app, "chunk_prepass.wesl");
    embedded_asset!(app, "tile_icon.wesl");
    // A module the other two import, so it must be loaded to be found: an
    // embedded file nothing holds a handle to is never registered.
    load_shader_library!(app, "leaf_hole.wesl");
    load_shader_library!(app, "chunk_vertex.wesl");
    load_shader_library!(app, "material_texture.wesl");
    app.add_plugins((
        MaterialPlugin::<ChunkMaterial>::default(),
        bevy::ui_render::UiMaterialPlugin::<tile_icon::TileIcon>::default(),
        fog::plugin,
        view_caches::plugin,
    ));
}
