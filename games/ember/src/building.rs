//! Ember's building Materials: charred wood, basalt, obsidian, iron sheet and rust, and the
//! Stations its buildings hold.

use demo_kit::building::{Block, Pattern::*, block, make, station};

const LOG: &str = "flora:log";
const LEAVES: &str = "flora:leaves";
const STONE: &str = "strata:stone";
const BASALT: &str = "strata:basalt";
const CLAY: &str = "strata:clay";
const ASH: &str = "hearth:ash";
const OBSIDIAN: &str = "waters:obsidian";

pub const BLOCKS: [Block; 19] = [
    block(
        "ash_plaster",
        Plaster,
        0x8c847d,
        0x8c847d,
        make("oven", &[(ASH, 1), (STONE, 1)], 2),
    ),
    block(
        "charred_log",
        Log,
        0x2b2220,
        0x5a3a2a,
        make("hand", &[(LOG, 1), (ASH, 1)], 1),
    ),
    block(
        "obsidian",
        Crystal,
        0x2a2333,
        0x2a2333,
        make("hand", &[(OBSIDIAN, 1)], 1),
    ),
    block(
        "basalt_cobble",
        Cobble,
        0x3b3836,
        0x1f1d1c,
        make("hand", &[(BASALT, 1)], 1),
    ),
    block(
        "basalt",
        Ashlar,
        0x45403d,
        0x2a2624,
        make("anvil", &[(BASALT, 2)], 2),
    ),
    block(
        "iron_sheet",
        Metal,
        0x5b6168,
        0x5b6168,
        make("forge", &[(BASALT, 2), (OBSIDIAN, 1)], 4),
    ),
    block(
        "rust_tiles",
        Tiles,
        0x8a4a2c,
        0x8a4a2c,
        make("oven", &[(CLAY, 2)], 4),
    ),
    block(
        "soot_shingles",
        Shingles,
        0x3a302c,
        0x3a302c,
        make("hand", &[("ember:soot_planks", 2)], 4),
    ),
    block(
        "ember_slate",
        Slate,
        0x5a2f28,
        0x5a2f28,
        make("anvil", &[(BASALT, 1), (STONE, 1)], 3),
    ),
    block(
        "magma_tiles",
        Tiles,
        0xb8452c,
        0xb8452c,
        make("oven", &[(CLAY, 1), (ASH, 1)], 4),
    ),
    block(
        "soot_planks",
        Planks,
        0x3e3029,
        0x3e3029,
        make("hand", &[(LOG, 1)], 4),
    ),
    block(
        "ash_planks",
        Planks,
        0x5d4a40,
        0x5d4a40,
        make("hand", &[(LOG, 1)], 4),
    ),
    block(
        "char_planks",
        Planks,
        0x2a211d,
        0x2a211d,
        make("hand", &[(LOG, 1)], 4),
    ),
    block(
        "rust_planks",
        Planks,
        0x7a3a22,
        0x7a3a22,
        make("hand", &[(LOG, 1)], 4),
    ),
    block(
        "ash_brick",
        Bricks,
        0x6b4b3e,
        0x2a2220,
        make("oven", &[(CLAY, 1), (ASH, 1)], 2),
    ),
    block(
        "ember_cloth",
        Cloth,
        0xd9822b,
        0x3b2a24,
        make("hand", &[(LEAVES, 6)], 2),
    ),
    station(
        "anvil",
        Anvil,
        0x5b6168,
        0x3e3029,
        make("forge", &[(BASALT, 4), (OBSIDIAN, 1)], 1),
    ),
    station(
        "forge",
        Forge,
        0x3b3836,
        0x1f1d1c,
        make("hand", &[(BASALT, 8)], 1),
    ),
    station(
        "oven",
        Oven,
        0x6b4b3e,
        0x2a2220,
        make("hand", &[(CLAY, 6)], 1),
    ),
];

demo_kit::building!("ember", BLOCKS);
