//! The Settlement Meadow lays out from its Templates, and the rows its Biomes list it and its
//! wilderness Templates in.

use demo_kit::structures::{Village, plot};

const VILLAGE: Village = Village {
    game: "meadow",
    name: "village",
    radius_m: 40.0,
    spacing_m: 256.0,
    chance: 0.9,
    centre: "pond_well",
    plots: 12,
    templates: &[
        plot("cottage", 4.0, 1, 4),
        plot("family_house", 3.0, 1, 3),
        plot("farmhouse", 2.0, 0, 2),
        plot("tavern", 1.0, 0, 1),
        plot("granary", 1.0, 0, 1),
        plot("tinker", 1.0, 0, 1),
        plot("bakery", 1.0, 0, 1),
        plot("honey_stall", 2.0, 0, 3),
        plot("mill_barn", 1.0, 0, 2),
    ],
};

demo_kit::structures!(VILLAGE);
