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
    let soil = tint([109, 81, 60], spread(x, y, face, 2, 43, 5));
    if fleck(x, y, 47 + face, 216) {
        scale(soil, FLECK)
    } else {
        soil
    }
}

fn pixel(name: &str, face: usize, x: usize, y: usize, base: [u8; 3]) -> [u8; 3] {
    let n = cells(x, y, 1, face + 3);
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
                tint(base, (patch - 128) / 5 + (n - 128) / 16)
            }
        }
        "leaves" => {
            if cells(x, y, 2, 37) < 90 {
                tint([28, 70, 30], spread(x, y, face, 2, 39, 14))
            } else {
                // Leaves 4 texels across, every other row of them shifted by half a leaf.
                let lx = (x + 2 * (y / 4 % 2)) % 4;
                let ly = y % 4;
                let between = (lx == 0 || lx == 3) && (ly == 0 || ly == 3);
                tint(
                    base,
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
        _ => tint(base, spread(x, y, face, 4, 83, 12)),
    }
}

pub fn texture(name: &str, [r, g, b, opacity]: [u8; 4]) -> Vec<u8> {
    let mut pixels = Vec::with_capacity(EDGE * EDGE * 3 * 4);
    for y in 0..EDGE {
        for face in 0..3 {
            for x in 0..EDGE {
                let colour = pixel(name, face, x, y, [r, g, b]);
                let [r, g, b] = if name == "lava" {
                    colour
                } else {
                    albedo(colour)
                };
                pixels.extend_from_slice(&[r, g, b, opacity]);
            }
        }
    }
    pixels
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The faces of each pattern that repeat; a log's end grain and a melon's stem are drawn once
    /// per tile and are left out.
    const PERIOD_BOUND: [(&str, &[usize]); 14] = [
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
