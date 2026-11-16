//! The Settlement Ember lays out from its Templates, and the rows its Biomes list it and its
//! wilderness Templates in.

use demo_kit::structures::{Village, plot};

const VILLAGE: Village = Village {
    game: "ember",
    name: "hamlet",
    radius_m: 40.0,
    spacing_m: 192.0,
    chance: 0.8,
    centre: "cistern",
    plots: 12,
    templates: &[
        plot("ash_hut", 4.0, 1, 4),
        plot("forge_hall", 3.0, 1, 3),
        plot("cinder_farm", 2.0, 0, 2),
        plot("ember_inn", 1.0, 0, 1),
        plot("ore_store", 1.0, 0, 1),
        plot("smelter", 1.0, 0, 1),
        plot("charcoal_kiln", 1.0, 0, 1),
        plot("ore_stall", 2.0, 0, 3),
        plot("basalt_hall", 1.0, 0, 2),
    ],
};

demo_kit::structures!(VILLAGE);
