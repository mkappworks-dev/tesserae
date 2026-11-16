//! The Settlements Ember lays out from its Templates, and the rows its Biomes list them and its
//! wilderness Templates in.

use demo_kit::structures::{Loot, Village, plot};

const HAMLET: Village = Village {
    game: "ember",
    name: "hamlet",
    radius_m: (24.0, 32.0),
    spacing_m: 96.0,
    chance: 0.8,
    near_spawn: false,
    centre: "cistern",
    plots: (3, 6),
    minimum: 2,
    templates: &[
        plot("ash_hut", 4.0, 1, 4),
        plot("forge_hall", 3.0, 0, 2).stocked(&FORGE_LOOT),
        plot("cinder_farm", 2.0, 0, 2),
        plot("charcoal_kiln", 1.0, 0, 1).stocked(&KILN_LOOT),
    ],
    loot: &HAMLET_LOOT,
};

const VILLAGE: Village = Village {
    game: "ember",
    name: "village",
    radius_m: (40.0, 64.0),
    spacing_m: 160.0,
    chance: 0.8,
    near_spawn: true,
    centre: "cistern",
    plots: (8, 20),
    minimum: 5,
    templates: &[
        plot("ash_hut", 4.0, 1, 6),
        plot("forge_hall", 3.0, 1, 4).stocked(&FORGE_LOOT),
        plot("cinder_farm", 2.0, 0, 3),
        plot("ember_inn", 1.0, 0, 1),
        plot("ore_store", 1.0, 0, 2),
        plot("smelter", 1.0, 0, 1).stocked(&SMELTER_LOOT),
        plot("charcoal_kiln", 1.0, 0, 1).stocked(&KILN_LOOT),
        plot("ore_stall", 2.0, 0, 3),
        plot("basalt_hall", 1.0, 0, 2),
    ],
    loot: &HAMLET_LOOT,
};

/// A hamlet keeps food, fuel and stone.
const HAMLET_LOOT: Loot = Loot {
    rolls: (1, 3),
    entries: &[
        ("ember:pepper", 1, 4, 3.0),
        ("crops:potato", 1, 4, 2.0),
        ("crops:bread", 1, 2, 2.0),
        ("hearth:ash", 2, 6, 2.0),
        ("ember:charred_log", 1, 4, 1.0),
        ("ember:basalt_cobble", 2, 8, 1.0),
    ],
};

/// A forge hall keeps ore and metal: basalt, obsidian and iron sheet.
const FORGE_LOOT: Loot = Loot {
    rolls: (2, 4),
    entries: &[
        ("ember:iron_sheet", 1, 4, 3.0),
        ("strata:basalt", 2, 8, 2.0),
        ("ember:obsidian", 1, 3, 2.0),
        ("hearth:ash", 2, 6, 1.0),
    ],
};

const SMELTER_LOOT: Loot = Loot {
    rolls: (2, 4),
    entries: &[
        ("ember:iron_sheet", 2, 6, 3.0),
        ("ember:magma_tiles", 1, 4, 1.0),
        ("ember:obsidian", 1, 3, 1.0),
    ],
};

const KILN_LOOT: Loot = Loot {
    rolls: (2, 3),
    entries: &[("ember:charred_log", 2, 8, 3.0), ("hearth:ash", 4, 12, 2.0)],
};

/// What a wilderness Structure's chest in each of the Game's Biomes holds, by Biome name.
pub fn biome_loot(biome: &str) -> &'static Loot {
    match biome {
        "scorched" => &Loot {
            rolls: (1, 3),
            entries: &[
                ("ember:obsidian", 1, 3, 2.0),
                ("ember:magma_tiles", 1, 4, 2.0),
                ("ember:iron_sheet", 1, 2, 1.0),
            ],
        },
        "highland" => &Loot {
            rolls: (1, 3),
            entries: &[
                ("strata:basalt", 2, 6, 2.0),
                ("ember:basalt_cobble", 2, 6, 2.0),
                ("ember:iron_sheet", 1, 3, 1.0),
            ],
        },
        "cinder_bog" => &Loot {
            rolls: (1, 3),
            entries: &[
                ("flora:mushroom", 1, 4, 2.0),
                ("crops:potato", 1, 3, 2.0),
                ("ember:ash_brick", 2, 6, 1.0),
            ],
        },
        _ => &Loot {
            rolls: (1, 3),
            entries: &[
                ("hearth:ash", 2, 8, 2.0),
                ("ember:pepper", 1, 3, 2.0),
                ("ember:soot_planks", 2, 6, 1.0),
            ],
        },
    }
}

demo_kit::structures!(HAMLET, VILLAGE);
