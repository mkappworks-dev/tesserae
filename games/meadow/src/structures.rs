//! The Settlement Meadow lays out from its Templates, and the rows its Biomes list it and its
//! wilderness Templates in.

use demo_kit::structures::{Loot, Village, plot};

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
        plot("granary", 1.0, 0, 1).stocked(&GRANARY_LOOT),
        plot("tinker", 1.0, 0, 1).stocked(&TINKER_LOOT),
        plot("bakery", 1.0, 0, 1).stocked(&BAKERY_LOOT),
        plot("honey_stall", 2.0, 0, 3),
        plot("mill_barn", 1.0, 0, 2),
    ],
    loot: &VILLAGE_LOOT,
};

/// A village keeps bread, grain and pumpkins.
const VILLAGE_LOOT: Loot = Loot {
    rolls: (1, 3),
    entries: &[
        ("crops:bread", 1, 3, 3.0),
        ("crops:wheat", 2, 6, 3.0),
        ("meadow:pumpkin", 1, 2, 2.0),
        ("flora:apple", 1, 3, 2.0),
        ("meadow:straw", 2, 6, 1.0),
    ],
};

const BAKERY_LOOT: Loot = Loot {
    rolls: (2, 4),
    entries: &[
        ("crops:bread", 2, 5, 4.0),
        ("crops:berry_pie", 1, 2, 2.0),
        ("crops:wheat", 4, 10, 2.0),
    ],
};

/// A tinker keeps glass, lanterns and stone.
const TINKER_LOOT: Loot = Loot {
    rolls: (2, 4),
    entries: &[
        ("glassworks:glass", 2, 6, 2.0),
        ("glassworks:lantern", 1, 2, 2.0),
        ("meadow:fieldstone", 2, 8, 1.0),
    ],
};

const GRANARY_LOOT: Loot = Loot {
    rolls: (1, 2),
    entries: &[
        ("crops:wheat", 4, 12, 3.0),
        ("crops:wheat_seeds", 2, 6, 2.0),
        ("crops:potato", 2, 6, 1.0),
    ],
};

/// What a wilderness Structure's chest in each of the Game's Biomes holds, by Biome name.
pub fn biome_loot(biome: &str) -> &'static Loot {
    match biome {
        "heath" => &Loot {
            rolls: (1, 3),
            entries: &[
                ("flora:fern", 1, 4, 2.0),
                ("meadow:straw", 2, 6, 2.0),
                ("crops:potato", 1, 3, 1.0),
            ],
        },
        "dunes" => &Loot {
            rolls: (1, 3),
            entries: &[
                ("strata:sand", 2, 8, 2.0),
                ("flora:dry_grass", 1, 4, 2.0),
                ("flora:cactus", 1, 2, 1.0),
            ],
        },
        "thicket" => &Loot {
            rolls: (1, 3),
            entries: &[
                ("flora:birch_sapling", 1, 2, 2.0),
                ("flora:mushroom_brown", 1, 3, 2.0),
                ("flora:apple", 1, 3, 1.0),
            ],
        },
        "fen" => &Loot {
            rolls: (1, 3),
            entries: &[
                ("flora:cane", 2, 6, 2.0),
                ("flora:mushroom", 1, 3, 2.0),
                ("meadow:river_stone", 2, 6, 1.0),
            ],
        },
        "prairie" => &Loot {
            rolls: (1, 3),
            entries: &[
                ("crops:wheat_seeds", 2, 5, 2.0),
                ("crops:pumpkin_seeds", 1, 3, 2.0),
                ("meadow:pumpkin", 1, 2, 1.0),
            ],
        },
        _ => &Loot {
            rolls: (1, 3),
            entries: &[
                ("flora:daisy", 1, 4, 2.0),
                ("flora:poppy", 1, 4, 2.0),
                ("crops:berries", 1, 4, 1.0),
            ],
        },
    }
}

demo_kit::structures!(VILLAGE);
