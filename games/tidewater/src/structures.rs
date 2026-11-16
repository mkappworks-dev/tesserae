//! The Settlement Tidewater lays out from its Templates, and the rows its Biomes list it and its
//! wilderness Templates in.

use demo_kit::structures::{Village, plot};

const VILLAGE: Village = Village {
    game: "tidewater",
    name: "harbour",
    // Small, since islands are.
    radius_m: 28.0,
    spacing_m: 104.0,
    chance: 0.6,
    centre: "tide_well",
    plots: 8,
    templates: &[
        plot("fisher_cottage", 4.0, 1, 4),
        plot("harbour_house", 3.0, 1, 3),
        plot("shore_farm", 2.0, 0, 2),
        plot("harbour_inn", 1.0, 0, 1),
        plot("net_loft", 1.0, 0, 1),
        plot("boatwright", 1.0, 0, 1),
        plot("smokehouse", 1.0, 0, 1),
        plot("fish_market", 2.0, 0, 3),
        plot("boathouse", 1.0, 0, 2),
    ],
};

demo_kit::structures!(VILLAGE);
