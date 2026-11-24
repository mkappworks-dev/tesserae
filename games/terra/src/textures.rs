use std::f32::consts::TAU;
use std::ops::Range;

const EDGE: usize = 16;

pub(crate) fn noise(x: usize, y: usize, salt: usize) -> i16 {
    let mut n = (x as u32).wrapping_mul(374_761_393)
        ^ (y as u32).wrapping_mul(668_265_263)
        ^ (salt as u32).wrapping_mul(1_274_126_177);
    n = (n ^ (n >> 13)).wrapping_mul(1_274_126_177);
    ((n ^ (n >> 16)) & 255) as i16
}

/// Noise per `size`-texel cell, wrapped so it repeats every tile.
pub(crate) fn cells(x: usize, y: usize, size: usize, salt: usize) -> i16 {
    let per_tile = EDGE / size;
    noise(x / size % per_tile, y / size % per_tile, salt)
}

/// Noise of one level in `k`: two thirds in `size`-texel cells, which outlast the first mips, and
/// a third per texel.
fn spread(x: usize, y: usize, face: usize, size: usize, salt: usize, k: i16) -> i16 {
    (cells(x, y, size, salt) - 128) / k + (cells(x, y, 1, face + 3) - 128) / (2 * k)
}

/// A pebble or fleck: as bright as this over its ground, in linear light, and 2x2 so a mip keeps it.
const FLECK: f32 = 1.3;

fn fleck(x: usize, y: usize, salt: usize, above: i16) -> bool {
    cells(x, y, 2, salt) > above
}

/// The phase of `v` on a wave that repeats every `period` texels.
fn phase(v: usize, period: usize) -> f32 {
    (v % period) as f32 * TAU / period as f32
}

fn linear(c: u8) -> f32 {
    let c = c as f32 / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn srgb(l: f32) -> u8 {
    let l = l.clamp(0.0, 1.0);
    let c = if l <= 0.003_130_8 {
        l * 12.92
    } else {
        1.055 * l.powf(1.0 / 2.4) - 0.055
    };
    (c * 255.0).round() as u8
}

fn luminance([r, g, b]: [f32; 3]) -> f32 {
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// `colour` times `factor` in linear light, which keeps its hue and saturation. Capped where the
/// brightest channel would clip, which would shift the hue.
fn scale(colour: [u8; 3], factor: f32) -> [u8; 3] {
    let lin = colour.map(linear);
    let top = lin.iter().fold(0.0f32, |a, &c| a.max(c));
    let factor = if top > 0.0 {
        factor.min(1.0 / top)
    } else {
        factor
    };
    lin.map(|c| srgb(c * factor))
}

/// Lighter or darker by `amount` levels on the brightest channel, as one factor in linear light: an
/// offset added to each channel would saturate every darkened texel.
pub(crate) fn tint(base: [u8; 3], amount: i16) -> [u8; 3] {
    let top = *base.iter().max().unwrap();
    if top == 0 {
        return base;
    }
    let to = (top as i16 + amount).clamp(0, 255) as u8;
    scale(base, linear(to) / linear(top))
}

/// Below 50 sRGB a texel is a painted shadow that stacks with the sky light's; above 243, a light.
const ALBEDO: Range<u8> = 50..243;

/// `colour` scaled into the range a lit, non-emissive surface keeps, by its luminance.
pub(crate) fn albedo(colour: [u8; 3]) -> [u8; 3] {
    let y = luminance(colour.map(linear));
    let (low, high) = (linear(ALBEDO.start), linear(ALBEDO.end));
    if y > 0.0 && y < low {
        scale(colour, low / y)
    } else if y > high {
        scale(colour, high / y)
    } else {
        colour
    }
}

/// What a tile reads as from afar: the linear mean of the drawn texels on `faces` (0 top, 1 side,
/// 2 bottom).
pub(crate) fn mean(pixels: &[u8], faces: Range<usize>) -> [u8; 3] {
    let drawn: Vec<&[u8]> = pixels
        .chunks_exact(4)
        .enumerate()
        .filter(|(i, p)| faces.contains(&(i / EDGE % 3)) && p[3] > 0)
        .map(|(_, p)| p)
        .collect();
    [0, 1, 2].map(|c| srgb(drawn.iter().map(|p| linear(p[c])).sum::<f32>() / drawn.len() as f32))
}

fn dirt(x: usize, y: usize, face: usize) -> [u8; 3] {
    let soil = tint([107, 74, 46], spread(x, y, face, 2, 43, 7));
    if fleck(x, y, 47 + face, 236) {
        scale(soil, FLECK)
    } else {
        soil
    }
}

/// Noise of a step of one in `k`, rounded to nearest.
fn grain(base: [u8; 3], x: usize, y: usize, salt: usize, k: i16) -> [u8; 3] {
    tint(base, (2 * (noise(x, y, salt) - 128) + k).div_euclid(2 * k))
}

const INK: [u8; 3] = [62, 46, 31];
const WALNUT: [u8; 3] = [92, 62, 38];

/// A saw laid diagonally across a Workbench's top and a hammer above it.
fn saw_and_hammer(x: usize, y: usize) -> bool {
    let blade = |x: usize| 12 - (x - 3) * 3 / 5;
    ((3..=11).contains(&x) && y == blade(x))
        || ((4..=10).contains(&x) && x % 2 == 0 && y == blade(x) + 1)
        || (x == 12 && (3..=10).contains(&y))
        || (y == 3 && (10..=14).contains(&x))
}

const GATE_STILE: [usize; 2] = [2, 13];

/// A Gate's side is two stiles with three rails across them, the upper two at
/// the edges of a fence's rail; the rest is open. Its top and bottom show only
/// the stiles' ends, in a corner whichever edge of the cell the panel is on.
fn gate_gap(face: usize, x: usize, y: usize) -> bool {
    let stile = |c: usize| c <= GATE_STILE[0] || c >= GATE_STILE[1];
    if face == 1 {
        !stile(x) && !matches!(y, 3 | 4 | 7 | 8 | 11 | 12)
    } else {
        !(stile(x) && stile(y))
    }
}

/// Flat plants, drawn on all three tiles so a cross reads from any side; `None` is clear.
const SPRITES: [&str; 15] = [
    "wheat_sprout",
    "wheat_young",
    "wheat_green",
    "wheat_ripe",
    "berry_sprout",
    "berry_bush",
    "berry_ripe",
    "cane",
    "mushroom",
    "vine",
    "kelp",
    "oak_sapling",
    "apple_sapling",
    "apple_unripe",
    "apple_ripe",
];

fn sprite(name: &str, x: usize, y: usize) -> Option<[u8; 3]> {
    let n = noise(x, y, 7);
    let (xi, yi) = (x as i16, y as i16);
    match name {
        "wheat_sprout" | "wheat_young" | "wheat_green" | "wheat_ripe" => {
            let stage = SPRITES.iter().position(|s| *s == name).unwrap();
            let height = [3, 6, 10, 13][stage];
            let stalk = [3, 6, 9, 12]
                .iter()
                .any(|&c| (xi - c - noise(c as usize, 0, 9) % 2).abs() == 0);
            let top = 16 - height;
            if !stalk || yi < top {
                return None;
            }
            if stage == 3 {
                let ear = yi < top + 4;
                return Some(if ear {
                    tint([214, 178, 70], (n - 128) / 14)
                } else {
                    tint([160, 150, 60], (n - 128) / 14)
                });
            }
            Some(tint(
                [92 + 18 * stage as u8, 158, 52],
                (n - 128) / 14 + (16 - yi),
            ))
        }
        "berry_sprout" | "berry_bush" | "berry_ripe" => {
            let radius = if name == "berry_sprout" { 3 } else { 6 };
            let (dx, dy) = (xi * 2 - 15, (yi - 10) * 2);
            if dx * dx + dy * dy * 2 > radius * radius * 4 || yi < 4 {
                return None;
            }
            if name == "berry_ripe" && n > 205 {
                return Some([186, 36, 62]);
            }
            Some(tint(
                [52, 112, 44],
                (n - 128) / 10 - if dy < 0 { 0 } else { 14 },
            ))
        }
        "cane" => {
            let reed = matches!(xi, 4..=5 | 8..=9 | 12);
            if !reed || (xi == 12 && yi < 5) {
                return None;
            }
            let node = yi % 5 == 0;
            Some(tint(
                [140, 190, 90],
                if node { -34 } else { (n - 128) / 12 } + (xi % 2) * 8,
            ))
        }
        "mushroom" => {
            let cap = (7..=10).contains(&yi) && (xi - 7).abs() * 2 <= 2 * (yi - 6) + 1;
            let stem = (11..=15).contains(&yi) && (7..=8).contains(&xi);
            if cap {
                Some(if (xi + yi) % 4 == 0 {
                    [236, 226, 206]
                } else {
                    tint([138, 74, 52], (n - 128) / 14)
                })
            } else if stem {
                Some(tint([222, 210, 186], (n - 128) / 18))
            } else {
                None
            }
        }
        "vine" => {
            let strand = (7..=8).contains(&(xi - yi / 3 % 2));
            let leaf = yi % 4 == 1 && ((4..=11).contains(&xi)) && n > 80;
            (strand || leaf).then(|| tint([56, 118, 44], (n - 128) / 12))
        }
        "kelp" => {
            let sway = 2 * (yi / 3 % 2) - 1;
            if xi == 7 + sway || xi == 8 + sway {
                Some(tint([44, 110, 70], (n - 128) / 12))
            } else if yi % 4 == 1 && (xi == 5 + sway || xi == 10 + sway) {
                Some(tint([60, 140, 84], (n - 128) / 14))
            } else {
                None
            }
        }
        "oak_sapling" | "apple_sapling" => {
            if (7..=8).contains(&xi) && yi >= 9 {
                return Some(tint([110, 78, 44], (n - 128) / 14));
            }
            let (dx, dy) = ((xi * 2 - 15).abs(), ((yi - 5) * 2 - 1).abs());
            if dx + dy * 6 / 5 < 10 && n > 60 {
                if name == "apple_sapling" && n > 235 {
                    return Some([196, 44, 40]);
                }
                return Some(tint(
                    [62, 138, 44],
                    (n - 128) / 10 - if yi > 6 { 14 } else { 0 },
                ));
            }
            None
        }
        "apple_unripe" | "apple_ripe" => {
            let ripe = name == "apple_ripe";
            let (radius, top) = if ripe { (5, 4) } else { (3, 7) };
            let (dx, dy) = (xi * 2 - 15, (yi - top - radius) * 2);
            if (6..=7).contains(&xi) && yi == top - 1 {
                return Some([88, 60, 34]);
            }
            if xi == 9 && yi == top - 1 || xi == 10 && yi == top - 2 {
                return Some([70, 140, 48]);
            }
            if dx * dx + dy * dy > radius * radius * 4 {
                return None;
            }
            let (body, under) = if ripe {
                ([196, 44, 40], [140, 28, 30])
            } else {
                ([128, 168, 58], [92, 130, 44])
            };
            Some(tint(if dy > radius { under } else { body }, (n - 128) / 18))
        }
        _ => None,
    }
}

fn pixel(name: &str, face: usize, x: usize, y: usize, base: [u8; 3]) -> [u8; 3] {
    let n = cells(x, y, 1, face + 3);
    let name = match name {
        "oak_log" | "apple_log" => "wood",
        "oak_leaves" | "apple_leaves" => "leaves",
        other => other,
    };
    match name {
        "stone" => {
            if (x + 2 * y) % EDGE == 0 && cells(x, y, 4, 19) > 145 {
                [86, 87, 96]
            } else {
                let rock = tint(base, spread(x, y, face, 4, 23, 9));
                if fleck(x, y, 31 + face, 240) {
                    scale(rock, FLECK)
                } else {
                    rock
                }
            }
        }
        "dirt" => dirt(x, y, face),
        "crystal" => {
            if cells(x, y, 2, 61) > 150 {
                tint([196, 160, 255], (n - 128) / 8)
            } else {
                pixel("stone", face, x, y, [133, 133, 140])
            }
        }
        "iron" => {
            if cells(x, y, 2, 53) > 176 {
                tint([190, 128, 86], (n - 128) / 10)
            } else {
                pixel("stone", face, x, y, [133, 133, 140])
            }
        }
        "grass" => {
            if face == 2 || (face == 1 && y >= 3 + cells(x, 0, 2, 41) as usize % 3) {
                dirt(x, y, face)
            } else {
                let patch = cells(x, y, 2, 71);
                tint([80, 144, 49], (patch - 128) / 5 + (n - 128) / 16)
            }
        }
        "leaves" => {
            if cells(x, y, 2, 37) < 90 {
                tint([25, 75, 29], spread(x, y, face, 2, 39, 14))
            } else {
                // Leaves 4 texels across, every other row of them shifted by half a leaf.
                let lx = (x + 2 * (y / 4 % 2)) % 4;
                let ly = y % 4;
                let between = (lx == 0 || lx == 3) && (ly == 0 || ly == 3);
                tint(
                    [60, 136, 44],
                    spread(x, y, face, 2, 41, 12) - if between { 14 } else { 0 },
                )
            }
        }
        "wood" => {
            if face != 1 {
                let ring = (x as i16 - 7).abs().max((y as i16 - 7).abs());
                tint(base, if ring % 3 == 0 { -37 } else { 16 } + (n - 128) / 18)
            } else {
                let bend = (y / 4 + x / 8) % 2;
                let stripe = (x + bend) % 8;
                let knot = ((x % EDGE) as i16 - 5).abs() + ((y % EDGE) as i16 - 10).abs() <= 2;
                tint(
                    base,
                    if stripe <= 1 || knot {
                        -44
                    } else if stripe == 2 {
                        18
                    } else {
                        -5
                    } + (n - 128) / 17,
                )
            }
        }
        "chest" => {
            let frame = x < 2 || x > 13 || y < 2 || y > 13;
            let rope = face == 1
                && ((y == 8 && (6..=9).contains(&x))
                    || ((x == 5 || x == 10) && (8..=10).contains(&y))
                    || (y == 10 && (5..=10).contains(&x)));
            if rope {
                grain([196, 168, 112], x, y, 75, 16)
            } else if frame {
                grain(tint(base, -48), x, y, 71 + 2 * face, 18)
            } else if face == 1 && y == 6 {
                tint(base, -46)
            } else {
                grain(
                    tint(base, if y % 4 == 0 { -18 } else { 0 }),
                    x,
                    y,
                    72 + 2 * face,
                    14,
                )
            }
        }
        "door" | "door_open" => {
            let frame = x <= 1 || x >= 14 || y <= 1 || y >= 14;
            let seam = x % 4 == 1;
            tint(
                base,
                if frame {
                    -34
                } else if seam {
                    -22
                } else {
                    (n - 128) / 14
                },
            )
        }
        "gate" | "gate_open" => {
            let stile = x <= GATE_STILE[0] || x >= GATE_STILE[1];
            let shadow = face == 1 && if stile { x == 0 || x == 15 } else { y % 4 == 0 };
            let grain = if stile || face != 1 { x % 3 } else { y % 3 } == 1;
            tint(
                base,
                if shadow {
                    -36
                } else if grain {
                    -14
                } else {
                    (n - 128) / 14
                },
            )
        }
        "lamp" => {
            if x < 2 || y < 2 || x > 13 || y > 13 {
                tint([116, 86, 44], (n - 128) / 15)
            } else if x == 2 || y == 2 || x == 13 || y == 13 {
                [203, 163, 85]
            } else {
                let center = (x as i16 - 7).abs().max((y as i16 - 7).abs());
                tint([255, 248, 204], -center * 3 + (n - 128) / 28)
            }
        }
        "water" => {
            let wave = (phase(y, 8) + phase(x, EDGE).sin() * 1.2).sin();
            let base = if face == 0 {
                [48, 140, 196]
            } else {
                [30, 115, 181]
            };
            if wave > 0.93 && (x + y / 2) % 8 < 4 {
                tint(base, 31)
            } else {
                tint(base, (wave * 6.0) as i16 + (n - 128) / 55)
            }
        }
        "lava" => {
            let drift = phase(y, EDGE).sin() * 0.75;
            let flow = (phase(x, 8) + drift).sin();
            if flow > 0.75 {
                tint([255, 178, 36], (n - 128) / 65)
            } else if flow > 0.0 {
                tint([255, 134, 19], (n - 128) / 55)
            } else {
                tint([239, 99, 13], (n - 128) / 48)
            }
        }
        "mud" => {
            if y % 8 == 2 && x % 8 < 3 && n > 110 {
                scale(tint(base, (n - 128) / 22), FLECK)
            } else if cells(x, y, 2, 93) < 130 {
                tint([66, 53, 37], spread(x, y, face, 4, 95, 20))
            } else {
                tint(base, spread(x, y, face, 4, 97, 11))
            }
        }
        "ash" => {
            let ash = tint(base, spread(x, y, face, 2, 57, 7));
            if fleck(x, y, 59 + face, 241) {
                scale(ash, FLECK)
            } else {
                ash
            }
        }
        "brick" => {
            let row = y / 4;
            let seam = y % 4 == 0 || (x + (row % 2) * 4) % 8 == 0;
            if seam {
                [103, 83, 68]
            } else {
                tint(
                    base,
                    spread(x, y, face, 2, 67, 12) + if y % 4 == 1 { 15 } else { 0 },
                )
            }
        }
        "melon" => {
            let stripe = if face == 1 { x } else { (x + y) / 2 };
            if face == 0 && (6..=9).contains(&x) && (6..=9).contains(&y) {
                [104, 84, 40]
            } else if stripe % 4 == 0 {
                tint([44, 98, 30], (n - 128) / 20)
            } else {
                tint(base, (n - 128) / 14)
            }
        }
        "workbench" => {
            if face != 1 {
                if x == 0 || y == 0 || x == 15 || y == 15 {
                    WALNUT
                } else if face == 0 && saw_and_hammer(x, y) {
                    INK
                } else {
                    grain(tint(base, if y % 5 == 0 { -16 } else { 0 }), x, y, 11, 14)
                }
            } else if y < 3 {
                grain(base, x, y, 12, 14)
            } else if (1..=3).contains(&x) || (12..=14).contains(&x) {
                grain(tint(base, -18), x, y, 13, 14)
            } else if (6..=9).contains(&y) && (x + y) % 3 == 0 {
                [176, 150, 108]
            } else {
                grain(WALNUT, x, y, 14, 14)
            }
        }
        "glass" => {
            if x == 0 || y == 0 || x == 15 || y == 15 {
                [181, 216, 255]
            } else if (8..=9).contains(&(x + y)) {
                [224, 242, 255]
            } else {
                [151, 201, 246]
            }
        }
        _ => tint(base, spread(x, y, face, 4, 83, 12)),
    }
}

pub fn texture(name: &str, [r, g, b, opacity]: [u8; 4]) -> Vec<u8> {
    let mut pixels = Vec::with_capacity(EDGE * EDGE * 3 * 4);
    for y in 0..EDGE {
        for face in 0..3 {
            for x in 0..EDGE {
                let drawn = SPRITES.contains(&name).then(|| sprite(name, x, y));
                let colour = drawn
                    .flatten()
                    .unwrap_or_else(|| pixel(name, face, x, y, [r, g, b]));
                let [r, g, b] = if matches!(name, "glass" | "lava" | "lamp") {
                    colour
                } else {
                    albedo(colour)
                };
                let alpha = if drawn == Some(None) {
                    0
                } else if name.starts_with("gate") {
                    if gate_gap(face, x, y) { 0 } else { opacity }
                } else if name != "glass" {
                    opacity
                } else if x == 0 || y == 0 || x == 15 || y == 15 {
                    100
                } else if (8..=9).contains(&(x + y)) {
                    48
                } else {
                    8
                };
                pixels.extend_from_slice(&[r, g, b, alpha]);
            }
        }
    }
    pixels
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The faces of each pattern that repeat; a log's end grain, a melon's stem, a frame or a
    /// lamp is drawn once per tile and is left out.
    const PERIOD_BOUND: [(&str, &[usize]); 15] = [
        ("stone", &[0, 1, 2]),
        ("dirt", &[0, 1, 2]),
        ("crystal", &[0, 1, 2]),
        ("iron", &[0, 1, 2]),
        ("grass", &[0, 2]),
        ("leaves", &[0, 1, 2]),
        ("wood", &[1]),
        ("water", &[0, 1, 2]),
        ("lava", &[0, 1, 2]),
        ("mud", &[0, 1, 2]),
        ("ash", &[0, 1, 2]),
        ("brick", &[0, 1, 2]),
        ("melon", &[1, 2]),
        ("sand", &[0, 1, 2]),
        ("snow", &[0, 1, 2]),
    ];

    /// Drawing past the tile's edge continues the pattern into the next Voxel's tile, which must
    /// match that tile's own start.
    #[test]
    fn every_period_bound_pattern_tiles_across_a_voxel_edge() {
        let base = [133, 133, 140];
        for (name, faces) in PERIOD_BOUND {
            for &face in faces {
                for y in 0..EDGE {
                    for x in 0..EDGE {
                        let at = |x, y| pixel(name, face, x, y, base);
                        assert_eq!(
                            at(x + EDGE, y),
                            at(x, y),
                            "{name} face {face} at x {x}, y {y}"
                        );
                        assert_eq!(
                            at(x, y + EDGE),
                            at(x, y),
                            "{name} face {face} at x {x}, y {y}"
                        );
                    }
                }
            }
        }
        // A grass side's lip is at its top edge, so it repeats only sideways.
        for y in 0..EDGE {
            for x in 0..EDGE {
                assert_eq!(
                    pixel("grass", 1, x + EDGE, y, base),
                    pixel("grass", 1, x, y, base),
                    "grass side at x {x}, y {y}"
                );
            }
        }
    }
}
