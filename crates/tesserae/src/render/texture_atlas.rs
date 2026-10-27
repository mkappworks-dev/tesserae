//! One face atlas shared by terrain and item models.
use super::chunk_material::PALETTE_LEN;
use tesserae_mods::MATERIAL_TEXTURE_EDGE;

pub const EDGE: usize = MATERIAL_TEXTURE_EDGE;
pub const WIDTH: usize = EDGE * 3;
pub const ROWS: usize = PALETTE_LEN + 1;
pub const HEIGHT: usize = EDGE * ROWS;

pub fn uv(slot: usize, face: usize, [u, v]: [f32; 2]) -> [f32; 2] {
    // Half-texel insets prevent neighbouring faces or Materials from bleeding.
    [
        (face as f32 * EDGE as f32 + 0.5 + u * (EDGE - 1) as f32) / WIDTH as f32,
        (slot as f32 * EDGE as f32 + 0.5 + v * (EDGE - 1) as f32) / HEIGHT as f32,
    ]
}
