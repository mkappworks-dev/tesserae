//! What the demo Games share and the contract does not: the noise and tint
//! their textures are drawn with, a texture's layout, their food, their
//! building Materials and Stations, and their Settlement.
//!
//! Each Game binds the WIT contract itself, so the records it declares are its
//! own types; [`food!`] writes the functions that build them inside the Game.

pub mod building;
pub mod structures;

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
                tags: Vec::new(),
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
                texture: None,
            }
        }

        pub fn feature() -> crate::tesserae::mods::content::SurfaceFeature {
            crate::tesserae::mods::content::SurfaceFeature {
                material: Some($food.qualified()),
                plant: None,
                shape: None,
                spacing_m: $food.spacing_m,
                clump: None,
                field: None,
            }
        }
    };
}

/// Writes `plants(&[(plant, spacing_m, clump_scale_m, clump_strength)])`, the surface features that
/// scatter each named Plant of a shared Mod; a strength of 0 scatters it evenly. Also `grouped`,
/// which makes features compete by region so a patch is mostly one species, and `crop_field`, a
/// planted field. Expanded inside the Game, where `crate::tesserae::mods` is its WIT binding.
#[macro_export]
macro_rules! plants {
    () => {
        pub fn plants(
            list: &[(&str, f32, f32, f32)],
        ) -> Vec<crate::tesserae::mods::content::SurfaceFeature> {
            list.iter()
                .map(|&(plant, spacing_m, scale_m, strength)| {
                    crate::tesserae::mods::content::SurfaceFeature {
                        material: None,
                        plant: Some(plant.into()),
                        shape: None,
                        spacing_m,
                        clump: (strength > 0.0)
                            .then_some(crate::tesserae::mods::content::Clump { scale_m, strength }),
                        field: None,
                    }
                })
                .collect()
        }

        /// Puts `features` in one field `group`. A member stands only in the regions that hold it
        /// and among a share of strays, so its spacing closes by the root of that share to keep the
        /// group's cover where it was.
        pub fn grouped(
            group: &str,
            scale_m: f32,
            strays: f32,
            features: Vec<crate::tesserae::mods::content::SurfaceFeature>,
        ) -> Vec<crate::tesserae::mods::content::SurfaceFeature> {
            let members = features.len().max(1) as f32;
            let share = (1.0 + strays * (members - 1.0)) / members;
            features
                .into_iter()
                .map(|f| crate::tesserae::mods::content::SurfaceFeature {
                    spacing_m: f.spacing_m * share.sqrt(),
                    field: Some(crate::tesserae::mods::content::FieldDef {
                        group: group.into(),
                        scale_m,
                        strays,
                    }),
                    ..f
                })
                .collect()
        }

        /// A planted field of the named crops with their weights.
        pub fn crop_field(
            crops: &[(&str, f32)],
            plot_m: (f32, f32),
            spacing_m: f32,
            path: &str,
            ditch: Option<(&str, f32)>,
        ) -> crate::tesserae::mods::content::CropField {
            crate::tesserae::mods::content::CropField {
                crops: crops.iter().map(|&(c, w)| (c.to_owned(), w)).collect(),
                plot_m,
                spacing_m,
                path: Some(path.into()),
                ditch: ditch.map(|(fluid, _)| fluid.into()),
                near_water_m: ditch.map(|(_, near_m)| near_m),
            }
        }
    };
}

/// A Biome Generator version 3 chooses by the land's shape: it names a range, so version 3 prefers
/// it where the column fits and earlier versions never choose it.
pub struct Landform {
    pub name: &'static str,
    pub climate: (f32, f32),
    /// Metres above Sea level.
    pub altitude_m: (f32, f32),
    /// Metres a river's valley cuts the ground; `None` asks nothing of rivers.
    pub river_m: Option<(f32, f32)>,
    /// Rock, soil and surface.
    pub ground: [&'static str; 3],
    pub sky: [u8; 3],
    pub fog: [u8; 3],
    /// `false` keeps tunnels from under it, as under the sea.
    pub caves: bool,
}

/// The sea floor, the strand, the high ground, and a river's banks below the high ground.
pub const OCEAN_M: (f32, f32) = (-1000.0, -0.01);
pub const SHORE_M: (f32, f32) = (0.0, 3.0);
pub const ALPINE_M: (f32, f32) = (90.0, 1000.0);
pub const RIVER_M: (f32, f32) = (0.0, 90.0);
pub const RIVER_CUT_M: (f32, f32) = (1.5, 1000.0);

/// A region of the underground with its own rock, floor, features and fog.
pub struct Cave {
    pub name: &'static str,
    pub climate: (f32, f32),
    /// Metres above Sea level; `None` matches every height.
    pub altitude_m: Option<(f32, f32)>,
    pub rock: Option<&'static str>,
    pub floor: Option<&'static str>,
    pub sky: [u8; 3],
    pub fog: [u8; 3],
    /// Material, Shape (`None` a cube) and spacing in metres.
    pub features: &'static [(&'static str, Option<&'static str>, f32)],
}

/// Writes `landform(l, features)` and `cave(c)`, the Game's own records for a [`Landform`] and a
/// [`Cave`]. Expanded inside the Game, where `crate::tesserae::mods` is its WIT binding.
#[macro_export]
macro_rules! landforms {
    () => {
        pub fn landform(
            l: &$crate::Landform,
            surface_features: Vec<crate::tesserae::mods::content::SurfaceFeature>,
        ) -> crate::tesserae::mods::content::BiomeDef {
            use crate::tesserae::mods::content as c;
            let range = |(min, max): (f32, f32)| c::SelectorRange { min, max };
            let [rock, soil, surface] = l.ground;
            let [s0, s1, s2] = l.sky;
            let [f0, f1, f2] = l.fog;
            c::BiomeDef {
                soundscape: None,
                selectors: Some(c::BiomeSelectors {
                    altitude: Some(range(l.altitude_m)),
                    coast: None,
                    erosion: None,
                    river: l.river_m.map(range),
                    relief: None,
                }),
                strata: None,
                name: l.name.into(),
                temperature: l.climate.0,
                humidity: l.climate.1,
                ground: Some(c::GroundParams {
                    rock: rock.into(),
                    soil: soil.into(),
                    surface: surface.into(),
                }),
                sky_fog: Some(c::SkyFog {
                    sky: (s0, s1, s2),
                    fog: (f0, f1, f2),
                }),
                cave_scale: (!l.caves).then_some(0.0),
                crop_fields: Vec::new(),
                structures: Vec::new(),
                settlements: Vec::new(),
                surface_features,
                loot: None,
            }
        }

        pub fn cave(cave: &$crate::Cave) -> crate::tesserae::mods::content::CaveBiomeDef {
            use crate::tesserae::mods::content as c;
            let [s0, s1, s2] = cave.sky;
            let [f0, f1, f2] = cave.fog;
            c::CaveBiomeDef {
                name: cave.name.into(),
                temperature: cave.climate.0,
                humidity: cave.climate.1,
                altitude: cave
                    .altitude_m
                    .map(|(min, max)| c::SelectorRange { min, max }),
                rock: cave.rock.map(Into::into),
                floor: cave.floor.map(Into::into),
                sky_fog: Some(c::SkyFog {
                    sky: (s0, s1, s2),
                    fog: (f0, f1, f2),
                }),
                features: cave
                    .features
                    .iter()
                    .map(|&(material, shape, spacing_m)| c::CaveFeature {
                        material: material.into(),
                        shape: shape.map(Into::into),
                        spacing_m,
                    })
                    .collect(),
            }
        }
    };
}
