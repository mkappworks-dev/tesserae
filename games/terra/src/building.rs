//! Terra's building Materials and the Stations its buildings hold, painted in the patterns the
//! approved Structures mockup gives them. The Games share this painter through demo-kit; Terra
//! keeps its own copy because it builds against the WIT contract alone.

use crate::tesserae::mods::content::{
    Appearance, MaterialDef, Occlusion, RecipeDef, RecipeStack, RenderClass, VoxelUse, VoxelUseKind,
};
use crate::tesserae::mods::types::Srgba;
use crate::textures::noise;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Pattern {
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
    Cloth,
    Anvil,
    Forge,
    Oven,
}
use Pattern::*;

struct Block {
    name: &'static str,
    pattern: Pattern,
    /// `0xRRGGBB`.
    main: u32,
    second: u32,
    station: Option<&'static str>,
    make: (&'static str, &'static [(&'static str, u32)], u32),
}

const fn block(
    name: &'static str,
    pattern: Pattern,
    main: u32,
    second: u32,
    make: (&'static str, &'static [(&'static str, u32)], u32),
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

const fn station(
    name: &'static str,
    pattern: Pattern,
    main: u32,
    second: u32,
    make: (&'static str, &'static [(&'static str, u32)], u32),
) -> Block {
    Block {
        station: Some(name),
        ..block(name, pattern, main, second, make)
    }
}

const LOG: &str = "flora:log";
const LEAVES: &str = "flora:leaves";
const ROCK: &str = "terra:rock";
const DIRT: &str = "terra:dirt";
const SAND: &str = "terra:sand";
const ASH: &str = "terra:ash";

const BLOCKS: [Block; 18] = [
    block(
        "oak_planks",
        Planks,
        0xb88a54,
        0xb88a54,
        ("hand", &[(LOG, 1)], 4),
    ),
    block(
        "spruce_planks",
        Planks,
        0x7a5634,
        0x7a5634,
        ("hand", &[(LOG, 1)], 4),
    ),
    block(
        "dark_oak_planks",
        Planks,
        0x4f3620,
        0x4f3620,
        ("hand", &[(LOG, 1)], 4),
    ),
    block(
        "red_planks",
        Planks,
        0x8e3b2c,
        0x8e3b2c,
        ("workbench", &[(LOG, 1)], 4),
    ),
    block("oak_log", Log, 0x6b4a2b, 0xb88a54, ("hand", &[(LOG, 1)], 1)),
    block(
        "dark_log",
        Log,
        0x3e2a19,
        0x7a5634,
        ("hand", &[(LOG, 1), (ASH, 1)], 1),
    ),
    block(
        "whitewash",
        Plaster,
        0xe9e3d3,
        0xe9e3d3,
        ("oven", &[(SAND, 1), (ASH, 1)], 2),
    ),
    block(
        "cobblestone",
        Cobble,
        0x8a8a85,
        0x5e5e5a,
        ("hand", &[(ROCK, 1)], 1),
    ),
    block(
        "ashlar",
        Ashlar,
        0xa9a59a,
        0x77746c,
        ("anvil", &[(ROCK, 2)], 2),
    ),
    block(
        "clay_tiles",
        Tiles,
        0xa8492f,
        0xa8492f,
        ("oven", &[(DIRT, 2)], 4),
    ),
    block(
        "terracotta_tiles",
        Tiles,
        0xc87a45,
        0xc87a45,
        ("oven", &[(DIRT, 1), (SAND, 1)], 4),
    ),
    block(
        "thatch",
        Thatch,
        0xc9a752,
        0xc9a752,
        ("hand", &[(LEAVES, 4)], 2),
    ),
    block(
        "slate",
        Slate,
        0x4c5560,
        0x4c5560,
        ("anvil", &[(ROCK, 2)], 3),
    ),
    block(
        "shingles",
        Shingles,
        0x6d5a45,
        0x6d5a45,
        ("workbench", &[("terra:oak_planks", 2)], 4),
    ),
    block(
        "awning",
        Cloth,
        0xc94f4f,
        0xefe6d6,
        ("workbench", &[(LEAVES, 6)], 2),
    ),
    station(
        "anvil",
        Anvil,
        0x5b6168,
        0x6b4a2b,
        ("forge", &[("terra:iron", 4)], 1),
    ),
    station(
        "forge",
        Forge,
        0x8a8a85,
        0x5e5e5a,
        ("workbench", &[(ROCK, 8)], 1),
    ),
    station(
        "oven",
        Oven,
        0xa8492f,
        0x67534a,
        ("workbench", &[("terra:brick", 6)], 1),
    ),
];

pub(crate) fn materials() -> Vec<MaterialDef> {
    BLOCKS
        .iter()
        .map(|b| MaterialDef {
            name: b.name.into(),
            emission: if b.pattern == Forge { 6 } else { 0 },
            dimming: 15,
            occlusion: Occlusion::Opaque,
            render: RenderClass::Opaque,
            remove_time_s: Some(match b.pattern {
                Planks | Log | Thatch | Cloth | Shingles => 0.4,
                _ => 0.6,
            }),
            contact_damage_per_s: None,
            tags: Vec::new(),
            solid: true,
        })
        .collect()
}

pub(crate) fn appearance() -> Vec<Appearance> {
    BLOCKS
        .iter()
        .map(|b| {
            let [r, g, bl] = rgb(b.main).map(|c| c as u8);
            Appearance {
                material: b.name.into(),
                color: Srgba {
                    r,
                    g,
                    b: bl,
                    a: 255,
                },
                texture: Some(texture(b)),
            }
        })
        .collect()
}

pub(crate) fn recipes() -> Vec<RecipeDef> {
    let stack = |item: &str, count| RecipeStack {
        item: item.into(),
        count,
    };
    BLOCKS
        .iter()
        .map(|b| {
            let (station, inputs, count) = b.make;
            RecipeDef {
                inputs: inputs.iter().map(|&(i, n)| stack(i, n)).collect(),
                output: stack(&format!("terra:{}", b.name), count),
                station: station.into(),
            }
        })
        .collect()
}

pub(crate) fn voxel_uses() -> Vec<VoxelUse> {
    BLOCKS
        .iter()
        .filter_map(|b| {
            Some(VoxelUse {
                material: format!("terra:{}", b.name),
                kind: VoxelUseKind::Station(b.station?.into()),
                shape: None,
            })
        })
        .collect()
}

/// Faces top, side and bottom; a log's end grain shows on the top and bottom.
fn texture(b: &Block) -> Vec<u8> {
    const EDGE: usize = 16;
    let salt = b
        .name
        .bytes()
        .fold(0usize, |h, c| h.wrapping_mul(31).wrapping_add(c as usize));
    let mut pixels = Vec::with_capacity(EDGE * EDGE * 3 * 4);
    for y in 0..EDGE {
        for face in 0..3 {
            for x in 0..EDGE {
                let n = noise(x, y, salt.wrapping_add(face)) as f32 / 255.0 * 0.08 - 0.04;
                let (colour, k) = texel(b.pattern, b.main, b.second, x, y, face != 1, n);
                let [r, g, bl] = rgb(colour).map(|c| (c * k).clamp(0.0, 255.0) as u8);
                pixels.extend_from_slice(&[r, g, bl, 255]);
            }
        }
    }
    pixels
}

/// A Forge's fire and an Oven's embers.
const GLOW: u32 = 0xe8742a;
const SOOT: u32 = 0x2a2220;

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
        Planks => {
            let row = yi >> 2;
            if y % 4 == 3 {
                (c1, 0.62)
            } else if xi == (row * 5 + 3) % 16 {
                (c1, 0.7)
            } else {
                (c1, 0.9 + hash(&[row, 7]) * 0.16 + n)
            }
        }
        Log => {
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
        Bricks => {
            let row = yi >> 2;
            let shifted = xi + (row % 2) * 4;
            if y % 4 == 3 || shifted % 8 == 7 {
                (c2, 0.95 + n)
            } else {
                (c1, 0.84 + hash(&[shifted >> 3, row]) * 0.22 + n)
            }
        }
        Ashlar => {
            let row = yi >> 3;
            let shifted = xi + (row % 2) * 8;
            if y % 8 == 7 || shifted % 16 == 15 {
                (c2, 0.9 + n)
            } else {
                (c1, 0.9 + hash(&[shifted >> 4, row]) * 0.12 + n)
            }
        }
        Cobble => {
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
        Tiles => {
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
        Shingles => {
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
        Thatch => {
            if !top && y % 6 == 5 {
                (c1, 0.68)
            } else {
                (
                    c1,
                    0.78 + hash(&[xi, if top { yi >> 1 } else { 1 }]) * 0.3 + n * 2.0,
                )
            }
        }
        Plaster => (c1, 0.94 + n),
        Slate => {
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
        Cloth => (
            if (x >> 2) % 2 == 1 { c2 } else { c1 },
            0.92 + ((x + y) % 2) as f32 * 0.05 + n * 0.5,
        ),
        // An iron anvil on a wooden stump: the face, the waist, then the stump's grain.
        Anvil => {
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
        Forge => {
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
                texel(Cobble, c1, c2, x, y, top, n)
            }
        }
        // Brick with an arched mouth, embers at its floor.
        Oven => {
            let dx = (x as f32 - 7.5).abs();
            let mouth = (4..=11).contains(&x) && (y >= 10 || (y >= 8 && dx <= 2.5));
            if !top && mouth {
                if y >= 14 {
                    (GLOW, 0.85 + n * 2.0)
                } else {
                    (SOOT, 0.9 + n)
                }
            } else {
                texel(Bricks, c1, c2, x, y, top, n)
            }
        }
    }
}
