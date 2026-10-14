//! Materials. Step 9 adds the look-pack API that supplies their WESL.

pub mod chunk_material;
pub mod fog;

use bevy::asset::embedded_asset;
use bevy::pbr::MaterialPlugin;
use bevy::prelude::*;

use chunk_material::ChunkMaterial;

pub fn plugin(app: &mut App) {
    // Embedded rather than loaded from `assets/`: the engine's own shaders ship
    // in the binary, as `bevy_pbr`'s do. A look pack's WESL is an asset because
    // it is content, which is the opposite case.
    embedded_asset!(app, "chunk_material.wesl");
    app.add_plugins(MaterialPlugin::<ChunkMaterial>::default());
}
