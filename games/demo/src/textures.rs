const EDGE: usize = 16;

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

fn dirt(x: usize, y: usize, salt: usize) -> [u8; 3] {
    let n = noise(x, y, salt);
    if n > 236 {
        [151, 119, 79]
    } else {
        tint([107, 74, 46], (n - 128) / 7)
    }
}

fn pixel(name: &str, face: usize, x: usize, y: usize, base: [u8; 3]) -> [u8; 3] {
    let n = noise(x, y, face + 3);
    match name {
        "stone" => {
            if (x + 2 * y) % 13 == 0 && noise(x / 3, y / 3, 19) > 145 {
                [86, 87, 96]
            } else if n > 240 {
                [173, 173, 180]
            } else {
                tint(base, (n - 128) / 9)
            }
        }
        "dirt" => dirt(x, y, face),
        "grass" => {
            if face == 2 || (face == 1 && y >= 3 + noise(x / 2, 0, 41) as usize % 3) {
                dirt(x, y, face)
            } else {
                let patch = noise(x / 2, y / 2, 71);
                tint([80, 144, 49], (patch - 128) / 5 + (n - 128) / 16)
            }
        }
        "leaves" => {
            let dx = (x as i16 % 5 - 2).abs();
            let dy = (y as i16 % 5 - 2).abs();
            let leaf = dx + dy <= 2;
            if leaf {
                tint([60, 136, 44], (n - 128) / 12 - if dx == 0 { 19 } else { 0 })
            } else {
                tint([25, 75, 29], (n - 128) / 14)
            }
        }
        "wood" => {
            if face != 1 {
                let ring = (x as i16 - 7).abs().max((y as i16 - 7).abs());
                tint(base, if ring % 3 == 0 { -37 } else { 16 } + (n - 128) / 18)
            } else {
                let bend = (y / 4 + x / 7) % 3;
                let stripe = (x + bend) % 7;
                let knot = (x as i16 - 5).abs() + (y as i16 - 10).abs() <= 2;
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
            let wave = (y as f32 * 1.05 + (x as f32 * 0.35).sin() * 1.2).sin();
            let base = if face == 0 {
                [48, 140, 196]
            } else {
                [30, 115, 181]
            };
            if wave > 0.93 && (x + y / 3) % 11 < 6 {
                tint(base, 31)
            } else {
                tint(base, (wave * 6.0) as i16 + (n - 128) / 55)
            }
        }
        "lava" => {
            let drift = (y as f32 * 0.4).sin() * 0.75;
            let flow = (x as f32 * 0.75 + drift).sin();
            if flow > 0.75 {
                tint([255, 178, 36], (n - 128) / 65)
            } else if flow > 0.0 {
                tint([255, 134, 19], (n - 128) / 55)
            } else {
                tint([239, 99, 13], (n - 128) / 48)
            }
        }
        "mud" => {
            if y % 6 == 2 && x % 7 < 3 && n > 110 {
                [145, 128, 96]
            } else if noise(x / 3, y / 2, 93) < 130 {
                tint([66, 53, 37], (n - 128) / 20)
            } else {
                tint(base, (n - 128) / 11)
            }
        }
        "ash" => {
            if n > 241 {
                [183, 178, 166]
            } else {
                tint(base, (n - 128) / 7)
            }
        }
        "brick" => {
            let row = y / 5;
            let seam = y % 5 == 0 || (x + (row % 2) * 4) % 8 == 0;
            if seam {
                [103, 83, 68]
            } else {
                tint(base, (n - 128) / 12 + if y % 5 == 1 { 15 } else { 0 })
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
        _ => tint(base, (n - 128) / 12),
    }
}

pub fn texture(name: &str, [r, g, b, opacity]: [u8; 4]) -> Vec<u8> {
    let mut pixels = Vec::with_capacity(EDGE * EDGE * 3 * 4);
    for y in 0..EDGE {
        for face in 0..3 {
            for x in 0..EDGE {
                let [r, g, b] = pixel(name, face, x, y, [r, g, b]);
                let alpha = if name != "glass" {
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
