//! Material face artwork: a 2D atlas for item models and tile icons, and a
//! mipmapped array, one layer per face, for terrain.
use super::chunk_material::PALETTE_LEN;
use tesserae_mods::{MATERIAL_TEXTURE_EDGE, TileVariation};

pub const EDGE: usize = MATERIAL_TEXTURE_EDGE;
pub const WIDTH: usize = EDGE * FACES;
pub const ROWS: usize = PALETTE_LEN + 1;
pub const HEIGHT: usize = EDGE * ROWS;
/// Top, side and bottom.
pub const FACES: usize = 3;

/// Bits of one face's layer in a slot's face word.
pub const FACE_LAYER_BITS: u32 = 10;
/// A textured slot's top two bits, which `material_texture.wesl` mirrors: an
/// untextured slot's word is zero, and a fixed tile keeps the word it had
/// before tiles varied, `TEXTURED` alone. Bit 30 set is a turned tile.
pub const TEXTURED: u32 = 0b10 << 30;
const TURNED: u32 = 0b11 << 30;
const TURNED_AND_PATCHED: u32 = 0b01 << 30;
const _: () = assert!(FACES as u32 * FACE_LAYER_BITS <= 30);
const _: () = assert!(ROWS * FACES <= 1 << FACE_LAYER_BITS);

/// A slot's top, side and bottom layers in the face array and how its tile
/// varies, as the shader reads them.
pub const fn face_word(layers: [u32; FACES], variation: TileVariation) -> u32 {
    let look = match variation {
        TileVariation::Fixed => TEXTURED,
        TileVariation::Turned => TURNED,
        TileVariation::TurnedAndPatched => TURNED_AND_PATCHED,
    };
    look | layers[0] | layers[1] << FACE_LAYER_BITS | layers[2] << (2 * FACE_LAYER_BITS)
}

pub fn uv(slot: usize, face: usize, [u, v]: [f32; 2]) -> [f32; 2] {
    // Half-texel insets prevent neighbouring faces or Materials from bleeding.
    [
        (face as f32 * EDGE as f32 + 0.5 + u * (EDGE - 1) as f32) / WIDTH as f32,
        (slot as f32 * EDGE as f32 + 0.5 + v * (EDGE - 1) as f32) / HEIGHT as f32,
    ]
}
