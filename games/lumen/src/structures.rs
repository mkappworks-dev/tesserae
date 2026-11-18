//! The Settlements Lumen lays out from its Templates, and the rows its Biomes list them and its
//! wilderness Templates in.

use demo_kit::structures::{Loot, Village, plot};

const HAMLET: Village = Village {
    game: "lumen",
    name: "hamlet",
    radius_m: (24.0, 32.0),
    spacing_m: 128.0,
    chance: 0.8,
    near_spawn: false,
    centre: "moon_well",
    plots: (3, 6),
    minimum: 2,
    templates: &[
        plot("lantern_cottage", 4.0, 1, 4),
        plot("twilight_house", 3.0, 0, 2),
        plot("glow_farm", 2.0, 0, 2),
        plot("lens_kiln", 1.0, 0, 1).stocked(&KILN_LOOT),
    ],
    loot: &VILLAGE_LOOT,
    water: (0.7, (1, 2)),
};

const VILLAGE: Village = Village {
    game: "lumen",
    name: "village",
    radius_m: (40.0, 64.0),
    spacing_m: 256.0,
    chance: 1.0,
    near_spawn: true,
    centre: "moon_well",
    plots: (8, 20),
    minimum: 5,
    templates: &[
        plot("lantern_cottage", 4.0, 1, 6),
        plot("twilight_house", 3.0, 1, 4),
        plot("glow_farm", 2.0, 0, 3),
        plot("starlight_inn", 1.0, 0, 1),
        plot("crystal_vault", 1.0, 0, 2).stocked(&VAULT_LOOT),
        plot("glassworks", 1.0, 0, 1).stocked(&GLASSWORKS_LOOT),
        plot("lens_kiln", 1.0, 0, 1).stocked(&KILN_LOOT),
        plot("glass_stall", 2.0, 0, 3),
        plot("glass_hall", 1.0, 0, 2),
    ],
    loot: &VILLAGE_LOOT,
    water: (0.9, (1, 3)),
};

/// A village keeps glowberries, glass and the odd crystal.
const VILLAGE_LOOT: Loot = Loot {
    rolls: (1, 3),
    entries: &[
        ("lumen:glowberry", 1, 4, 3.0),
        ("crops:bread", 1, 2, 2.0),
        ("glassworks:glass", 2, 6, 2.0),
        ("glassworks:lantern", 1, 2, 1.0),
        ("crystals:amethyst", 1, 2, 1.0),
    ],
};

/// A crystal vault keeps crystals.
const VAULT_LOOT: Loot = Loot {
    rolls: (1, 3),
    entries: &[
        ("crystals:amethyst", 1, 3, 2.0),
        ("crystals:citrine", 1, 3, 2.0),
        ("crystals:aquamarine", 1, 3, 2.0),
        ("lumen:moonstone", 2, 6, 1.0),
    ],
};

const GLASSWORKS_LOOT: Loot = Loot {
    rolls: (2, 4),
    entries: &[
        ("glassworks:glass", 4, 12, 3.0),
        ("glassworks:frosted_glass", 2, 8, 2.0),
        ("glassworks:lantern", 1, 3, 1.0),
    ],
};

const KILN_LOOT: Loot = Loot {
    rolls: (2, 3),
    entries: &[
        ("glassworks:glass", 2, 8, 3.0),
        ("crystals:citrine", 1, 2, 1.0),
        ("strata:sand", 4, 12, 2.0),
    ],
};

/// What a wilderness Structure's chest in each of the Game's Biomes holds, by Biome name.
pub fn biome_loot(biome: &str) -> &'static Loot {
    match biome {
        "arid_court" => &Loot {
            rolls: (1, 3),
            entries: &[
                ("crystals:citrine", 1, 2, 2.0),
                ("strata:sand", 2, 8, 2.0),
                ("lumen:moon_cobble", 2, 6, 1.0),
            ],
        },
        "frost_court" => &Loot {
            rolls: (1, 3),
            entries: &[
                ("frost:ice", 2, 6, 2.0),
                ("frost:snow", 2, 8, 2.0),
                ("crystals:aquamarine", 1, 2, 1.0),
            ],
        },
        "conservatory" => &Loot {
            rolls: (1, 3),
            entries: &[
                ("flora:orchid", 1, 3, 2.0),
                ("lumen:glowberry", 1, 4, 2.0),
                ("glassworks:lantern", 1, 2, 1.0),
            ],
        },
        "lily_pond" => &Loot {
            rolls: (1, 3),
            entries: &[
                ("flora:kelp", 1, 4, 2.0),
                ("lumen:glowcap", 1, 4, 2.0),
                ("crystals:amethyst", 1, 2, 1.0),
            ],
        },
        _ => &Loot {
            rolls: (1, 3),
            entries: &[
                ("lumen:glowberry", 1, 4, 2.0),
                ("lumen:glowcap", 1, 3, 2.0),
                ("flora:poppy", 1, 3, 1.0),
            ],
        },
    }
}

demo_kit::structures!(HAMLET, VILLAGE);
