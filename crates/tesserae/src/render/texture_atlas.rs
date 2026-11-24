//! Material face artwork: a 2D atlas for item models and tile icons, and a
//! mipmapped array, one layer per face, for terrain.
use super::chunk_material::PALETTE_LEN;
use tesserae_mods::MATERIAL_TEXTURE_EDGE;

pub const EDGE: usize = MATERIAL_TEXTURE_EDGE;
pub const WIDTH: usize = EDGE * FACES;
pub const ROWS: usize = PALETTE_LEN + 1;
pub const HEIGHT: usize = EDGE * ROWS;
/// Top, side and bottom.
pub const FACES: usize = 3;

/// Bits of one face's layer in a slot's face word.
pub const FACE_LAYER_BITS: u32 = 10;
/// Marks a slot with a texture; an untextured slot's word is zero.
pub const TEXTURED: u32 = 1 << 31;
const _: () = assert!(ROWS * FACES <= 1 << FACE_LAYER_BITS);

/// A slot's top, side and bottom layers in the face array, as the shader reads them.
pub const fn face_word(layers: [u32; FACES]) -> u32 {
    TEXTURED | layers[0] | layers[1] << FACE_LAYER_BITS | layers[2] << (2 * FACE_LAYER_BITS)
}

pub fn uv(slot: usize, face: usize, [u, v]: [f32; 2]) -> [f32; 2] {
    // Half-texel insets prevent neighbouring faces or Materials from bleeding.
    [
        (face as f32 * EDGE as f32 + 0.5 + u * (EDGE - 1) as f32) / WIDTH as f32,
        (slot as f32 * EDGE as f32 + 0.5 + v * (EDGE - 1) as f32) / HEIGHT as f32,
    ]
}
