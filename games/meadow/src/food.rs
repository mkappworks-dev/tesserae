//! The Game's food: a pumpkin, found on the meadow's turf and the heath's soil.

use demo_kit::{Food, noise, tint};

const FOOD: Food = Food {
    game: "meadow",
    name: "pumpkin",
    color: [214, 118, 32, 255],
    emission: 0,
    food: 8.0,
    // About one in every 20 by 20 m of the ground it grows on: findable, not a carpet.
    spacing_m: 20.0,
};

demo_kit::food!(FOOD, pixel);

fn pixel(face: usize, x: usize, y: usize) -> [u8; 3] {
    let n = noise(x, y, face + 5);
    let base = FOOD.rgb();
    match face {
        0 if (7..=8).contains(&x) && (7..=8).contains(&y) => [92, 70, 34],
        1 if x % 4 == 0 => tint(base, -40),
        _ => tint(base, (n - 128) / 12),
    }
}
