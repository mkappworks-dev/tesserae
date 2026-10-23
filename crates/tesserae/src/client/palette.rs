//! The Server's appearance rows become the Client's linear-colour uniform.

use crate::content::MaterialPalette;
use crate::render::chunk_material::{ChunkMaterialExtension, PALETTE_LEN};
use bevy::prelude::*;

impl ChunkMaterialExtension {
    pub fn apply_palette(&mut self, message: &MaterialPalette) {
        for &(id, [r, g, b, a]) in &message.0 {
            if let Some(color) = self.palette[..PALETTE_LEN].get_mut(id as usize) {
                *color = Color::srgba(
                    r as f32 / 255.0,
                    g as f32 / 255.0,
                    b as f32 / 255.0,
                    a as f32 / 255.0,
                )
                .to_linear()
                .to_vec4();
            }
        }
    }
}
