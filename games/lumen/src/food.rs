//! The Game's food: a glowberry, lit from within like the lanterns, found in the garden and on
//! the frost court's snow.

use crate::tesserae::mods::content::{
    Appearance, ItemDef, MaterialDef, Occlusion, RenderClass, SurfaceFeature,
};
use crate::tesserae::mods::types::Srgba;

const NAME: &str = "glowberry";
const COLOR: [u8; 4] = [214, 226, 120, 255];
const FOOD: f32 = 5.0;
/// About one in every 20 by 20 m of the ground it grows on: findable, not a carpet.
const SPACING_M: f32 = 20.0;

pub fn material() -> MaterialDef {
    MaterialDef {
        name: NAME.into(),
        emission: 6,
        dimming: 15,
        occlusion: Occlusion::Opaque,
        render: RenderClass::Opaque,
        remove_time_s: Some(0.35),
        contact_damage_per_s: None,
        solid: true,
    }
}

pub fn appearance() -> Appearance {
    let [r, g, b, a] = COLOR;
    Appearance {
        material: NAME.into(),
        color: Srgba { r, g, b, a },
        texture: Some(texture()),
    }
}

pub fn item() -> ItemDef {
    let [r, g, b, a] = COLOR;
    ItemDef {
        name: NAME.into(),
        size_m: 0.25,
        color: Srgba { r, g, b, a },
        material: Some(format!("lumen:{NAME}")),
        max_stack: None,
        food: Some(FOOD),
    }
}

pub fn feature() -> SurfaceFeature {
    SurfaceFeature {
        material: format!("lumen:{NAME}"),
        spacing_m: SPACING_M,
    }
}

fn noise(x: usize, y: usize, salt: usize) -> i16 {
    let mut n = (x as u32).wrapping_mul(374_761_393)
        ^ (y as u32).wrapping_mul(668_265_263)
        ^ (salt as u32).wrapping_mul(1_274_126_177);
    n = (n ^ (n >> 13)).wrapping_mul(1_274_126_177);
    ((n ^ (n >> 16)) & 255) as i16
}

fn tint(base: [u8; 3], amount: i16) -> [u8; 3] {
    base.map(|c| (c as i16 + amount).clamp(0, 255) as u8)
}

/// Faces top, side and bottom, 16 by 16 each, side by side.
fn texture() -> Vec<u8> {
    let mut pixels = Vec::with_capacity(16 * 48 * 4);
    for y in 0..16 {
        for face in 0..3 {
            for x in 0..16 {
                let [r, g, b] = pixel(face, x, y);
                pixels.extend_from_slice(&[r, g, b, COLOR[3]]);
            }
        }
    }
    pixels
}

fn pixel(face: usize, x: usize, y: usize) -> [u8; 3] {
    let n = noise(x, y, face + 11);
    let (dx, dy) = ((x % 4) as i16 - 2, (y % 4) as i16 - 2);
    if dx * dx + dy * dy <= 2 {
        tint([COLOR[0], COLOR[1], COLOR[2]], 16 + (n - 128) / 16)
    } else {
        tint([60, 96, 52], (n - 128) / 12)
    }
}
