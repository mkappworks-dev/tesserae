//! The Game's food: a coconut, washed up on the lagoon's island turf and the salt flats' sand.

use demo_kit::{Food, noise, tint};

const FOOD: Food = Food {
    game: "tidewater",
    name: "coconut",
    color: [112, 74, 42, 255],
    emission: 0,
    food: 6.0,
    // About one in every 16 by 16 m of the ground it grows on: findable, not a carpet.
    spacing_m: 16.0,
};

demo_kit::food!(FOOD, pixel);

fn pixel(face: usize, x: usize, y: usize) -> [u8; 3] {
    let n = noise(x, y, face + 13);
    let base = FOOD.rgb();
    if face == 0 && [(5, 5), (10, 5), (8, 10)].contains(&(x, y)) {
        [46, 30, 18]
    } else if noise(x, y / 3, face + 31) > 200 {
        tint(base, 30)
    } else {
        tint(base, (n - 128) / 10)
    }
}
