//! The Settlement Tidewater lays out from its Templates, and the rows its Biomes list it and its
//! wilderness Templates in.

use demo_kit::structures::{Loot, Village, plot};

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
        plot("net_loft", 1.0, 0, 1).stocked(&NET_LOFT_LOOT),
        plot("boatwright", 1.0, 0, 1).stocked(&BOATWRIGHT_LOOT),
        plot("smokehouse", 1.0, 0, 1).stocked(&SMOKEHOUSE_LOOT),
        plot("fish_market", 2.0, 0, 3),
        plot("boathouse", 1.0, 0, 2),
    ],
    loot: &HARBOUR_LOOT,
};

/// A harbour keeps the catch and the gear: kelp, coconuts, driftwood and sailcloth.
const HARBOUR_LOOT: Loot = Loot {
    rolls: (1, 3),
    entries: &[
        ("aquatic:kelp", 2, 6, 3.0),
        ("tidewater:coconut", 1, 3, 2.0),
        ("crops:bread", 1, 2, 1.0),
        ("tidewater:driftwood", 2, 6, 2.0),
        ("tidewater:sailcloth", 1, 4, 1.0),
    ],
};

/// A net loft keeps sailcloth, its rope, and what the nets bring in.
const NET_LOFT_LOOT: Loot = Loot {
    rolls: (2, 4),
    entries: &[
        ("tidewater:sailcloth", 2, 6, 3.0),
        ("aquatic:kelp", 2, 8, 2.0),
        ("aquatic:seagrass", 2, 6, 1.0),
    ],
};

const BOATWRIGHT_LOOT: Loot = Loot {
    rolls: (2, 4),
    entries: &[
        ("tidewater:teak_planks", 4, 12, 3.0),
        ("tidewater:driftwood_log", 1, 4, 2.0),
        ("tidewater:sailcloth", 1, 3, 1.0),
    ],
};

const SMOKEHOUSE_LOOT: Loot = Loot {
    rolls: (2, 3),
    entries: &[
        ("aquatic:kelp", 4, 10, 3.0),
        ("tidewater:coconut", 1, 4, 2.0),
        ("crops:bread", 1, 3, 1.0),
    ],
};

/// What a wilderness Structure's chest in each of the Game's Biomes holds, by Biome name.
pub fn biome_loot(biome: &str) -> &'static Loot {
    match biome {
        "salt_flats" => &Loot {
            rolls: (1, 3),
            entries: &[
                ("strata:sand", 2, 8, 2.0),
                ("tidewater:shell_tiles", 2, 6, 2.0),
                ("crystals:citrine", 1, 2, 1.0),
            ],
        },
        "cold_shallows" => &Loot {
            rolls: (1, 3),
            entries: &[
                ("aquatic:kelp", 2, 6, 2.0),
                ("tidewater:sea_slate", 2, 6, 2.0),
                ("crystals:aquamarine", 1, 2, 1.0),
            ],
        },
        "rainforest" => &Loot {
            rolls: (1, 3),
            entries: &[
                ("flora:jungle_sapling", 1, 2, 2.0),
                ("flora:vine", 2, 6, 2.0),
                ("tidewater:palm_log", 1, 4, 1.0),
            ],
        },
        "tidal_marsh" => &Loot {
            rolls: (1, 3),
            entries: &[
                ("aquatic:lily_pad", 1, 3, 2.0),
                ("flora:cane", 2, 6, 2.0),
                ("aquatic:seagrass", 2, 6, 1.0),
            ],
        },
        _ => &Loot {
            rolls: (1, 3),
            entries: &[
                ("tidewater:coconut", 1, 3, 2.0),
                ("tidewater:coral_block", 1, 4, 2.0),
                ("aquatic:red_weed", 2, 6, 1.0),
            ],
        },
    }
}

demo_kit::structures!(VILLAGE);
