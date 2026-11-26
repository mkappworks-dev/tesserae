//! Material face artwork: a 2D atlas for item models and tile icons, and a
//! mipmapped array, one layer per face, for terrain.
use super::chunk_material::PALETTE_LEN;
use bevy::shader::ShaderDefVal;
use tesserae_mods::{MATERIAL_TEXTURE_EDGE, TileVariation};

pub const EDGE: usize = MATERIAL_TEXTURE_EDGE;
pub const WIDTH: usize = EDGE * FACES;
pub const ROWS: usize = PALETTE_LEN + 1;
pub const HEIGHT: usize = EDGE * ROWS;
/// Top, side and bottom.
pub const FACES: usize = 3;

/// Bits of one face's layer in a slot's face word.
pub const FACE_LAYER_BITS: u32 = 10;
/// Where a face word's look starts: its top two bits, over the three layers. An untextured slot's
/// look is zero; `material_texture.wesl` reads these as `constants::CHUNK_LOOK_*`.
pub const LOOK_SHIFT: u32 = 30;
pub const LOOK_FIXED: u32 = 0b10;
pub const LOOK_TURNED: u32 = 0b11;
pub const LOOK_TURNED_AND_PATCHED: u32 = 0b01;
/// Set in both turned looks.
pub const LOOK_TURN_BIT: u32 = 0b01;
const _: () = assert!(FACES as u32 * FACE_LAYER_BITS <= LOOK_SHIFT);
const _: () = assert!(ROWS * FACES <= 1 << FACE_LAYER_BITS);
const _: () = assert!(LOOK_FIXED & LOOK_TURN_BIT == 0);
const _: () = assert!(LOOK_TURNED & LOOK_TURNED_AND_PATCHED == LOOK_TURN_BIT);

/// A slot's top, side and bottom layers in the face array and how its tile
/// varies, as the shader reads them.
pub const fn face_word(layers: [u32; FACES], variation: TileVariation) -> u32 {
    let look = match variation {
        TileVariation::Fixed => LOOK_FIXED,
        TileVariation::Turned => LOOK_TURNED,
        TileVariation::TurnedAndPatched => LOOK_TURNED_AND_PATCHED,
    };
    look << LOOK_SHIFT
        | layers[0]
        | layers[1] << FACE_LAYER_BITS
        | layers[2] << (2 * FACE_LAYER_BITS)
}

/// The face word's layout as the chunk shader's `constants::CHUNK_*`.
pub fn face_word_shader_defs() -> Vec<ShaderDefVal> {
    let uint = |name: &str, value: u32| ShaderDefVal::UInt(format!("CHUNK_{name}").into(), value);
    vec![
        uint("FACE_LAYER_BITS", FACE_LAYER_BITS),
        uint("LOOK_SHIFT", LOOK_SHIFT),
        uint("LOOK_FIXED", LOOK_FIXED),
        uint("LOOK_TURNED", LOOK_TURNED),
        uint("LOOK_TURNED_AND_PATCHED", LOOK_TURNED_AND_PATCHED),
        uint("LOOK_TURN_BIT", LOOK_TURN_BIT),
    ]
}

pub fn uv(slot: usize, face: usize, [u, v]: [f32; 2]) -> [f32; 2] {
    // Half-texel insets prevent neighbouring faces or Materials from bleeding.
    [
        (face as f32 * EDGE as f32 + 0.5 + u * (EDGE - 1) as f32) / WIDTH as f32,
        (slot as f32 * EDGE as f32 + 0.5 + v * (EDGE - 1) as f32) / HEIGHT as f32,
    ]
}
