//! The Game's food: a fire pepper, found on the ash flats and the highland grass.

use crate::tesserae::mods::content::{
    Appearance, ItemDef, MaterialDef, Occlusion, RenderClass, SurfaceFeature,
};
use crate::tesserae::mods::types::Srgba;

const NAME: &str = "pepper";
const COLOR: [u8; 4] = [176, 36, 28, 255];
const FOOD: f32 = 4.0;
/// About one in every 16 by 16 m of the ground it grows on: findable, not a carpet.
const SPACING_M: f32 = 16.0;

pub fn material() -> MaterialDef {
    MaterialDef {
        name: NAME.into(),
        emission: 0,
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
        material: Some(format!("ember:{NAME}")),
        max_stack: None,
        food: Some(FOOD),
    }
}

pub fn feature() -> SurfaceFeature {
    SurfaceFeature {
        material: format!("ember:{NAME}"),
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
    let n = noise(x, y, face + 7);
    let base = [COLOR[0], COLOR[1], COLOR[2]];
    match face {
        0 if (6..=9).contains(&x) && (6..=9).contains(&y) => tint([62, 112, 40], (n - 128) / 16),
        1 if x % 8 == 3 => tint(base, 48),
        _ => tint(base, (n - 128) / 10),
    }
}
