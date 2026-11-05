//! The Game's food: a glowberry, lit from within like the lanterns, found in the garden and on
//! the frost court's snow.

use demo_kit::{Food, noise, tint};

const FOOD: Food = Food {
    game: "lumen",
    name: "glowberry",
    color: [214, 226, 120, 255],
    emission: 6,
    food: 5.0,
    // About one in every 20 by 20 m of the ground it grows on: findable, not a carpet.
    spacing_m: 20.0,
};

demo_kit::food!(FOOD, pixel);

fn pixel(face: usize, x: usize, y: usize) -> [u8; 3] {
    let n = noise(x, y, face + 11);
    let (dx, dy) = ((x % 4) as i16 - 2, (y % 4) as i16 - 2);
    if dx * dx + dy * dy <= 2 {
        tint(FOOD.rgb(), 16 + (n - 128) / 16)
    } else {
        tint([60, 96, 52], (n - 128) / 12)
    }
}
