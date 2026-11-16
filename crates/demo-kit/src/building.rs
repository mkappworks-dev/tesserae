//! The building Materials a Game's Templates are made of, the Stations its buildings hold and the
//! door, chest, workbench, glass and lamp that furnish them, painted in the patterns the approved
//! Structures mockup gives them.

use crate::{noise, texture};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pattern {
    Planks,
    Log,
    Bricks,
    Ashlar,
    Cobble,
    Tiles,
    Shingles,
    Thatch,
    Plaster,
    Slate,
    Crystal,
    Metal,
    Cloth,
    Moss,
    Anvil,
    Forge,
    Oven,
    /// A Door, declared with its open state, `<name>_open`, and the Game's `panel` Shape.
    Door,
    /// A Container.
    Chest,
    /// The `workbench` Station, placed as the Game's `table` Shape.
    Workbench,
    Glass,
    Lamp,
}

/// A Recipe whose output is the [`Block`] it belongs to.
pub struct Make {
    pub station: &'static str,
    pub inputs: &'static [(&'static str, u32)],
    pub count: u32,
}

pub struct Block {
    pub name: &'static str,
    pub pattern: Pattern,
    /// `0xRRGGBB`.
    pub main: u32,
    pub second: u32,
    /// The tag of the Station a Use of it opens.
    pub station: Option<&'static str>,
    pub make: Make,
}

pub const fn make(
    station: &'static str,
    inputs: &'static [(&'static str, u32)],
    count: u32,
) -> Make {
    Make {
        station,
        inputs,
        count,
    }
}

pub const fn block(
    name: &'static str,
    pattern: Pattern,
    main: u32,
    second: u32,
    make: Make,
) -> Block {
    Block {
        name,
        pattern,
        main,
        second,
        station: None,
        make,
    }
}

/// A Station whose tag is its own name.
pub const fn station(
    name: &'static str,
    pattern: Pattern,
    main: u32,
    second: u32,
    make: Make,
) -> Block {
    Block {
        station: Some(name),
        ..block(name, pattern, main, second, make)
    }
}

/// A Door's size: whole Template cells, so a Template's opening fits it at every Voxel size, with
/// room round a Player's 0.6 m by 1.8 m body.
pub const DOOR_WIDTH_M: f32 = 2.0;
pub const DOOR_HEIGHT_M: f32 = 3.0;

/// A Forge's fire and an Oven's embers.
const GLOW: u32 = 0xe8742a;
const SOOT: u32 = 0x2a2220;

impl Block {
    pub fn rgba(&self) -> [u8; 4] {
        let [r, g, b] = rgb(self.main).map(|c| c as u8);
        [
            r,
            g,
            b,
            if self.pattern == Pattern::Glass {
                96
            } else {
                255
            },
        ]
    }

    pub fn emission(&self) -> u8 {
        match self.pattern {
            Pattern::Forge => 6,
            Pattern::Lamp => 15,
            _ => 0,
        }
    }

    pub fn remove_time_s(&self) -> f32 {
        use Pattern::*;
        match self.pattern {
            Planks | Log | Thatch | Cloth | Moss | Shingles | Door | Chest | Workbench => 0.4,
            Glass | Lamp => 0.3,
            _ => 0.6,
        }
    }

    /// Faces top, side and bottom; a log's end grain shows on the top and bottom.
    pub fn texture(&self) -> Vec<u8> {
        let salt = self
            .name
            .bytes()
            .fold(0usize, |h, b| h.wrapping_mul(31).wrapping_add(b as usize));
        texture(|face, x, y| {
            let n = noise(x, y, salt.wrapping_add(face)) as f32 / 255.0 * 0.08 - 0.04;
            let (colour, k) = texel(self.pattern, self.main, self.second, x, y, face != 1, n);
            let [r, g, b] = rgb(colour).map(|c| (c * k).clamp(0.0, 255.0) as u8);
            [r, g, b, alpha(self.pattern, x, y)]
        })
    }
}

/// Glass is clear inside its frame; everything else is opaque.
fn alpha(pattern: Pattern, x: usize, y: usize) -> u8 {
    let frame = x == 0 || y == 0 || x == 15 || y == 15;
    if pattern == Pattern::Glass && !frame {
        80
    } else {
        255
    }
}

fn rgb(hex: u32) -> [f32; 3] {
    [(hex >> 16) & 255, (hex >> 8) & 255, hex & 255].map(|c| c as f32)
}

/// A value in 0..1 that is the same for the same arguments, to vary one plank or stone from the next.
fn hash(values: &[i32]) -> f32 {
    let h = values.iter().fold(2_166_136_261u32, |h, &v| {
        (h ^ (v as u32 & 0xffff)).wrapping_mul(16_777_619)
    });
    (h % 1000) as f32 / 1000.0
}

const COBBLES: [(f32, f32); 9] = [
    (2.0, 3.0),
    (9.0, 1.0),
    (14.0, 5.0),
    (5.0, 9.0),
    (11.0, 10.0),
    (1.0, 14.0),
    (8.0, 15.0),
    (15.0, 13.0),
    (4.0, 5.0),
];

/// One texel: which colour, and how light, for `pattern` at (`x`, `y`) of a face.
fn texel(pattern: Pattern, c1: u32, c2: u32, x: usize, y: usize, top: bool, n: f32) -> (u32, f32) {
    let (xi, yi) = (x as i32, y as i32);
    match pattern {
        Pattern::Planks => {
            let row = yi >> 2;
            if y % 4 == 3 {
                (c1, 0.62)
            } else if xi == (row * 5 + 3) % 16 {
                (c1, 0.7)
            } else {
                (c1, 0.9 + hash(&[row, 7]) * 0.16 + n)
            }
        }
        Pattern::Log => {
            if top {
                let d = (x as f32 - 7.5).hypot(y as f32 - 7.5);
                if d > 6.6 {
                    (c1, 0.8 + n)
                } else {
                    (c2, 0.85 + (d * 1.7).cos() * 0.1 + n)
                }
            } else if (x + (y >> 2)) % 4 == 0 {
                (c1, 0.68 + n)
            } else {
                (c1, 0.92 + hash(&[xi, 3]) * 0.14 + n)
            }
        }
        Pattern::Bricks => {
            let row = yi >> 2;
            let shifted = xi + (row % 2) * 4;
            if y % 4 == 3 || shifted % 8 == 7 {
                (c2, 0.95 + n)
            } else {
                (c1, 0.84 + hash(&[shifted >> 3, row]) * 0.22 + n)
            }
        }
        Pattern::Ashlar => {
            let row = yi >> 3;
            let shifted = xi + (row % 2) * 8;
            if y % 8 == 7 || shifted % 16 == 15 {
                (c2, 0.9 + n)
            } else {
                (c1, 0.9 + hash(&[shifted >> 4, row]) * 0.12 + n)
            }
        }
        Pattern::Cobble => {
            let mut ds: Vec<(f32, i32)> = COBBLES
                .iter()
                .enumerate()
                .map(|(i, &(px, py))| ((x as f32 - px).hypot(y as f32 - py), i as i32))
                .collect();
            ds.sort_by(|a, b| a.0.total_cmp(&b.0));
            if ds[1].0 - ds[0].0 < 1.1 {
                (c2, 0.85)
            } else {
                (c1, 0.82 + hash(&[ds[0].1, 11]) * 0.3 + n)
            }
        }
        Pattern::Tiles => {
            let row = yi >> 2;
            let tx = (xi + (row % 2) * 4) % 8;
            if y % 4 == 3 {
                (c1, 0.58)
            } else if tx == 0 {
                (c1, 0.7)
            } else {
                (c1, 0.84 + (1.0 - (tx - 4).abs() as f32 / 4.0) * 0.2 + n)
            }
        }
        Pattern::Shingles => {
            let row = yi >> 2;
            let shifted = xi + (row % 2) * 2;
            if y % 4 == 3 {
                (c1, 0.55)
            } else if shifted % 4 == 0 {
                (c1, 0.72)
            } else {
                (c1, 0.85 + hash(&[shifted >> 2, row]) * 0.25 + n)
            }
        }
        Pattern::Thatch => {
            if !top && y % 6 == 5 {
                (c1, 0.68)
            } else {
                (
                    c1,
                    0.78 + hash(&[xi, if top { yi >> 1 } else { 1 }]) * 0.3 + n * 2.0,
                )
            }
        }
        Pattern::Plaster => (c1, 0.94 + n),
        Pattern::Slate => {
            let row = yi / 3;
            let shifted = xi + (row % 2) * 3;
            if y % 3 == 2 {
                (c1, 0.6)
            } else if shifted % 6 == 0 {
                (c1, 0.75)
            } else {
                let lip = if y % 3 == 0 { 0.08 } else { 0.0 };
                (c1, 0.88 + hash(&[shifted / 6, row]) * 0.2 + lip + n)
            }
        }
        Pattern::Crystal => {
            if (x + y) % 6 == 0 {
                (c1, 1.35)
            } else if (xi - yi + 16) % 5 == 0 {
                (c1, 0.65)
            } else {
                (c1, 0.92 + n * 2.0)
            }
        }
        Pattern::Metal => {
            if y % 8 == 0 {
                (c1, 0.68)
            } else if x % 8 == 3 && y % 8 == 3 {
                (c1, 1.35)
            } else {
                (c1, 0.92 + (y % 8) as f32 * 0.012 + n * 0.5)
            }
        }
        Pattern::Cloth => (
            if (x >> 2) % 2 == 1 { c2 } else { c1 },
            0.92 + ((x + y) % 2) as f32 * 0.05 + n * 0.5,
        ),
        Pattern::Moss => (
            if hash(&[xi, yi]) > 0.82 { c2 } else { c1 },
            0.78 + hash(&[yi, xi]) * 0.3 + n,
        ),
        // An iron anvil on a wooden stump: the face, the waist, then the stump's grain.
        Pattern::Anvil => {
            let edge = x == 0 || y == 0 || x == 15 || y == 15;
            if top {
                if edge {
                    (c1, 0.7)
                } else if (5..=10).contains(&x) && (5..=10).contains(&y) {
                    (c1, 1.15 + n)
                } else {
                    (c1, 0.95 + n)
                }
            } else if y == 0 {
                (c1, 1.2)
            } else if y < 5 {
                (c1, 0.95 + n)
            } else if y < 9 {
                if (4..=11).contains(&x) {
                    (c1, 0.8 + n)
                } else {
                    (c2, 0.5)
                }
            } else if y == 9 {
                (c2, 0.62)
            } else {
                (c2, 0.85 + hash(&[xi, 5]) * 0.15 + n)
            }
        }
        // Fieldstone round an open fire on the sides, a grate over it on top.
        Pattern::Forge => {
            if !top && (4..=11).contains(&x) && (7..=13).contains(&y) {
                if y == 7 {
                    (c2, 0.5)
                } else {
                    (GLOW, 0.8 + (y - 8) as f32 * 0.06 + n * 2.0)
                }
            } else if top && (5..=10).contains(&x) && (5..=10).contains(&y) {
                if (x + y) % 2 == 0 {
                    (SOOT, 0.8)
                } else {
                    (GLOW, 0.9 + n)
                }
            } else {
                texel(Pattern::Cobble, c1, c2, x, y, top, n)
            }
        }
        // Brick with an arched mouth, embers at its floor.
        Pattern::Oven => {
            let dx = (x as f32 - 7.5).abs();
            let mouth = (4..=11).contains(&x) && (y >= 10 || (y >= 8 && dx <= 2.5));
            if !top && mouth {
                if y >= 14 {
                    (GLOW, 0.85 + n * 2.0)
                } else {
                    (SOOT, 0.9 + n)
                }
            } else {
                texel(Pattern::Bricks, c1, c2, x, y, top, n)
            }
        }
        // Upright boards in a frame, an iron handle at the latch side.
        Pattern::Door => {
            if x == 0 || x == 15 || y == 0 || y == 15 {
                (c1, 0.66)
            } else if (11..=12).contains(&x) && (7..=9).contains(&y) {
                (c2, 1.0 + n)
            } else if x % 5 == 0 {
                (c1, 0.72)
            } else {
                (c1, 0.9 + hash(&[xi / 5, 9]) * 0.14 + n)
            }
        }
        // Planks bound by an iron band, a latch at its middle; the lid on top.
        Pattern::Chest => {
            if top {
                if x == 0 || y == 0 || x == 15 || y == 15 {
                    (c2, 0.8)
                } else {
                    texel(Pattern::Planks, c1, c2, x, y, top, n)
                }
            } else if (7..=8).contains(&x) && (5..=9).contains(&y) {
                (c2, 1.25 + n)
            } else if y == 6 || y == 7 || x == 0 || x == 15 {
                (c2, 0.85 + n)
            } else {
                texel(Pattern::Planks, c1, c2, x, y, top, n)
            }
        }
        // A worn top with a square of iron tools; legs under a rail on the sides.
        Pattern::Workbench => {
            if top {
                if (3..=12).contains(&x) && (3..=12).contains(&y) && (x % 3 == 0 || y % 3 == 0) {
                    (c2, 0.9 + n)
                } else {
                    texel(Pattern::Planks, c1, c2, x, y, top, n)
                }
            } else if y < 4 {
                (c1, 1.05 + n)
            } else if x < 3 || x > 12 {
                (c1, 0.75 + n)
            } else {
                (c1, 0.35)
            }
        }
        // A frame round a pane with one glint across it.
        Pattern::Glass => {
            if x == 0 || y == 0 || x == 15 || y == 15 {
                (c2, 0.9 + n)
            } else if x + y == 10 || x + y == 11 {
                (c1, 1.4)
            } else {
                (c1, 1.0 + n * 0.5)
            }
        }
        // A bright lantern between a frame and two cross bars.
        Pattern::Lamp => {
            if x <= 1 || y <= 1 || x >= 14 || y >= 14 || x == 7 || x == 8 || y == 7 || y == 8 {
                (c2, 0.85 + n)
            } else {
                let d = (x as f32 - 7.5).hypot(y as f32 - 7.5);
                (c1, 1.25 - d * 0.03 + n)
            }
        }
    }
}

/// Writes `materials`, `shapes`, `appearance`, `recipes` and `voxel_uses` for the Game `$game`'s
/// building [`Block`]s `$blocks`. Expanded inside the Game, where `crate::tesserae::mods` is its
/// WIT binding.
#[macro_export]
macro_rules! building {
    ($game:expr, $blocks:path) => {
        /// Each Block's name, and after a Door its open state's.
        fn names() -> impl Iterator<Item = (&'static $crate::building::Block, String)> {
            $blocks.iter().flat_map(|b| {
                let open = (b.pattern == $crate::building::Pattern::Door)
                    .then(|| (b, format!("{}_open", b.name)));
                [(b, b.name.to_owned())].into_iter().chain(open)
            })
        }

        pub fn materials() -> Vec<crate::tesserae::mods::content::MaterialDef> {
            use crate::tesserae::mods::content::{MaterialDef, Occlusion, RenderClass};
            names()
                .map(|(b, name)| {
                    let glass = b.pattern == $crate::building::Pattern::Glass;
                    MaterialDef {
                        name,
                        emission: b.emission(),
                        dimming: if glass { 0 } else { 15 },
                        occlusion: if glass {
                            Occlusion::SameMaterial
                        } else {
                            Occlusion::Opaque
                        },
                        render: if glass {
                            RenderClass::Blend
                        } else {
                            RenderClass::Opaque
                        },
                        remove_time_s: Some(b.remove_time_s()),
                        contact_damage_per_s: None,
                        tags: Vec::new(),
                        solid: true,
                    }
                })
                .collect()
        }

        /// A Door's panel, 3/16 thick against its own +Z, and a workbench's table: four legs under
        /// a thick top that a body collides with whole.
        pub fn shapes() -> Vec<crate::tesserae::mods::content::ShapeDef> {
            use crate::tesserae::mods::content::{CollisionBox, NodeBox, ShapeDef};
            let leg = |x, z| NodeBox {
                min: (x, 0, z),
                max: (x + 3, 12, z + 3),
            };
            vec![
                ShapeDef {
                    form: Default::default(),
                    name: "panel".into(),
                    boxes: vec![NodeBox {
                        min: (0, 0, 13),
                        max: (16, 16, 16),
                    }],
                    collision: None,
                    connect: None,
                },
                ShapeDef {
                    form: Default::default(),
                    name: "table".into(),
                    boxes: vec![
                        leg(1, 1),
                        leg(12, 1),
                        leg(1, 12),
                        leg(12, 12),
                        NodeBox {
                            min: (0, 12, 0),
                            max: (16, 16, 16),
                        },
                    ],
                    collision: Some(vec![CollisionBox {
                        min: (0, 0, 0),
                        max: (16, 16, 16),
                    }]),
                    connect: None,
                },
            ]
        }

        pub fn appearance() -> Vec<crate::tesserae::mods::content::Appearance> {
            names()
                .map(|(b, name)| {
                    let [r, g, b_, a] = b.rgba();
                    crate::tesserae::mods::content::Appearance {
                        material: name,
                        color: crate::tesserae::mods::types::Srgba { r, g, b: b_, a },
                        texture: Some(b.texture()),
                    }
                })
                .collect()
        }

        pub fn recipes() -> Vec<crate::tesserae::mods::content::RecipeDef> {
            let stack = |item: &str, count| crate::tesserae::mods::content::RecipeStack {
                item: item.into(),
                count,
            };
            $blocks
                .iter()
                .map(|b| crate::tesserae::mods::content::RecipeDef {
                    inputs: b.make.inputs.iter().map(|&(i, n)| stack(i, n)).collect(),
                    output: stack(&format!("{}:{}", $game, b.name), b.make.count),
                    station: b.make.station.into(),
                })
                .collect()
        }

        pub fn voxel_uses() -> Vec<crate::tesserae::mods::content::VoxelUse> {
            use crate::tesserae::mods::content::{DoorDef, VoxelUse, VoxelUseKind};
            use $crate::building::Pattern;
            let full = |name: &str| format!("{}:{}", $game, name);
            $blocks
                .iter()
                .filter_map(|b| {
                    let (kind, shape) = match (b.pattern, b.station) {
                        (Pattern::Door, _) => (
                            VoxelUseKind::Door(DoorDef {
                                open: full(&format!("{}_open", b.name)),
                                width_m: Some($crate::building::DOOR_WIDTH_M),
                                height_m: Some($crate::building::DOOR_HEIGHT_M),
                            }),
                            Some(full("panel")),
                        ),
                        (Pattern::Chest, _) => (VoxelUseKind::Container(None), None),
                        (Pattern::Workbench, Some(tag)) => {
                            (VoxelUseKind::Station(tag.into()), Some(full("table")))
                        }
                        (_, Some(tag)) => (VoxelUseKind::Station(tag.into()), None),
                        (_, None) => return None,
                    };
                    Some(VoxelUse {
                        material: full(b.name),
                        kind,
                        shape,
                    })
                })
                .collect()
        }
    };
}
