//! The Settlement Lumen lays out from its Templates, and the rows its Biomes list it and its
//! wilderness Templates in.

use demo_kit::structures::{Village, plot};

const VILLAGE: Village = Village {
    game: "lumen",
    name: "village",
    radius_m: 40.0,
    spacing_m: 320.0,
    chance: 1.0,
    centre: "moon_well",
    plots: 12,
    templates: &[
        plot("lantern_cottage", 4.0, 1, 4),
        plot("twilight_house", 3.0, 1, 3),
        plot("glow_farm", 2.0, 0, 2),
        plot("starlight_inn", 1.0, 0, 1),
        plot("crystal_vault", 1.0, 0, 1),
        plot("glassworks", 1.0, 0, 1),
        plot("lens_kiln", 1.0, 0, 1),
        plot("glass_stall", 2.0, 0, 3),
        plot("glass_hall", 1.0, 0, 2),
    ],
};

demo_kit::structures!(VILLAGE);
