//! The Game's food: a fire pepper, found on the ash flats and the highland grass.

use demo_kit::{Food, noise, tint};

const FOOD: Food = Food {
    game: "ember",
    name: "pepper",
    color: [176, 36, 28, 255],
    emission: 0,
    food: 4.0,
    // About one in every 16 by 16 m of the ground it grows on: findable, not a carpet.
    spacing_m: 16.0,
};

demo_kit::food!(FOOD, pixel);

fn pixel(face: usize, x: usize, y: usize) -> [u8; 3] {
    let n = noise(x, y, face + 7);
    let base = FOOD.rgb();
    match face {
        0 if (6..=9).contains(&x) && (6..=9).contains(&y) => tint([62, 112, 40], (n - 128) / 16),
        1 if x % 8 == 3 => tint(base, 48),
        _ => tint(base, (n - 128) / 10),
    }
}
