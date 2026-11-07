//! What the demo Games share and the contract does not: the noise and tint
//! their textures are drawn with, a texture's layout, and their food.
//!
//! Each Game binds the WIT contract itself, so the records it declares are its
//! own types; [`food!`] writes the functions that build them inside the Game.

/// A texture face's edge in pixels.
pub const EDGE: usize = 16;

pub fn noise(x: usize, y: usize, salt: usize) -> i16 {
    let mut n = (x as u32).wrapping_mul(374_761_393)
        ^ (y as u32).wrapping_mul(668_265_263)
        ^ (salt as u32).wrapping_mul(1_274_126_177);
    n = (n ^ (n >> 13)).wrapping_mul(1_274_126_177);
    ((n ^ (n >> 16)) & 255) as i16
}

pub fn tint(base: [u8; 3], amount: i16) -> [u8; 3] {
    base.map(|c| (c as i16 + amount).clamp(0, 255) as u8)
}

/// Faces top, side and bottom, [`EDGE`] square each, side by side, from
/// `pixel(face, x, y)`.
pub fn texture(pixel: impl Fn(usize, usize, usize) -> [u8; 4]) -> Vec<u8> {
    let mut pixels = Vec::with_capacity(EDGE * EDGE * 3 * 4);
    for y in 0..EDGE {
        for face in 0..3 {
            for x in 0..EDGE {
                pixels.extend_from_slice(&pixel(face, x, y));
            }
        }
    }
    pixels
}

/// A Game's one food: a solid Material standing on its Biomes' surface, and
/// the Item it drops.
pub struct Food {
    pub game: &'static str,
    pub name: &'static str,
    pub color: [u8; 4],
    pub emission: u8,
    /// Hunger one restores.
    pub food: f32,
    /// About one in every `spacing_m` by `spacing_m` metres of the ground it
    /// grows on.
    pub spacing_m: f32,
}

impl Food {
    pub fn qualified(&self) -> String {
        format!("{}:{}", self.game, self.name)
    }

    pub fn rgb(&self) -> [u8; 3] {
        let [r, g, b, _] = self.color;
        [r, g, b]
    }

    /// Its texture, opaque as its colour, from `pixel(face, x, y)`.
    pub fn texture(&self, pixel: fn(usize, usize, usize) -> [u8; 3]) -> Vec<u8> {
        texture(|face, x, y| {
            let [r, g, b] = pixel(face, x, y);
            [r, g, b, self.color[3]]
        })
    }
}

/// Writes `material`, `appearance`, `item` and `feature` for the Game's
/// [`Food`] `$food`, textured by `$pixel(face, x, y)`. Expanded inside the
/// Game, where `crate::tesserae::mods` is its WIT binding.
#[macro_export]
macro_rules! food {
    ($food:path, $pixel:path) => {
        pub fn material() -> crate::tesserae::mods::content::MaterialDef {
            crate::tesserae::mods::content::MaterialDef {
                name: $food.name.into(),
                emission: $food.emission,
                dimming: 15,
                occlusion: crate::tesserae::mods::content::Occlusion::Opaque,
                render: crate::tesserae::mods::content::RenderClass::Opaque,
                remove_time_s: Some(0.35),
                contact_damage_per_s: None,
                solid: true,
            }
        }

        pub fn appearance() -> crate::tesserae::mods::content::Appearance {
            let [r, g, b, a] = $food.color;
            crate::tesserae::mods::content::Appearance {
                material: $food.name.into(),
                color: crate::tesserae::mods::types::Srgba { r, g, b, a },
                texture: Some($food.texture($pixel)),
            }
        }

        pub fn item() -> crate::tesserae::mods::content::ItemDef {
            let [r, g, b, a] = $food.color;
            crate::tesserae::mods::content::ItemDef {
                name: $food.name.into(),
                size_m: 0.25,
                color: crate::tesserae::mods::types::Srgba { r, g, b, a },
                material: Some($food.qualified()),
                max_stack: None,
                food: Some($food.food),
            }
        }

        pub fn feature() -> crate::tesserae::mods::content::SurfaceFeature {
            crate::tesserae::mods::content::SurfaceFeature {
                material: $food.qualified(),
                spacing_m: $food.spacing_m,
            }
        }
    };
}
