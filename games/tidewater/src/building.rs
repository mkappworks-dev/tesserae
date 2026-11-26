//! Tidewater's building Materials: driftwood, palm, sandstone, coral and sailcloth.

use demo_kit::building::{Block, Pattern::*, block, make};

const LOG: &str = "flora:log";
const LEAVES: &str = "flora:leaves";
const STONE: &str = "strata:stone";
const SAND: &str = "strata:sand";
const CLAY: &str = "strata:clay";

pub const BLOCKS: [Block; 16] = [
    block(
        "shell_plaster",
        Plaster,
        0xefe4cf,
        0xefe4cf,
        make("oven", &[(SAND, 1), (STONE, 1)], 2),
    ),
    block(
        "palm_log",
        Log,
        0x8a7253,
        0xc9b38a,
        make("hand", &[(LOG, 1)], 1),
    ),
    block(
        "driftwood_log",
        Log,
        0x8f8676,
        0xc4baa4,
        make("hand", &[(LOG, 1)], 1),
    ),
    block(
        "sandstone",
        Ashlar,
        0xd8c48f,
        0xb39e68,
        make("hand", &[(SAND, 2)], 1),
    ),
    block(
        "coral_block",
        Cobble,
        0xd98b74,
        0xa8604f,
        make("anvil", &[(STONE, 1), (CLAY, 1)], 2),
    ),
    block(
        "reed_thatch",
        Thatch,
        0xb7a76a,
        0xb7a76a,
        make("hand", &[(LEAVES, 4)], 2),
    ),
    block(
        "shell_tiles",
        Tiles,
        0xe7d6c3,
        0xe7d6c3,
        make("oven", &[(SAND, 2)], 4),
    ),
    block(
        "sea_shingles",
        Shingles,
        0x5b7480,
        0x5b7480,
        make("hand", &[("tidewater:driftwood", 2)], 4),
    ),
    block(
        "sea_slate",
        Slate,
        0x3d5c63,
        0x3d5c63,
        make("anvil", &[(STONE, 2)], 3),
    ),
    block(
        "coral_tiles",
        Tiles,
        0xd07a62,
        0xd07a62,
        make("oven", &[(CLAY, 2)], 4),
    ),
    block(
        "driftwood",
        Planks,
        0xb9ae98,
        0xb9ae98,
        make("hand", &[(LOG, 1)], 4),
    ),
    block(
        "blue_planks",
        Planks,
        0x4f7ea8,
        0x4f7ea8,
        make("hand", &[(LOG, 1)], 4),
    ),
    block(
        "teak_planks",
        Planks,
        0x8a5a36,
        0x8a5a36,
        make("hand", &[(LOG, 1)], 4),
    ),
    block(
        "teal_planks",
        Planks,
        0x3f8a86,
        0x3f8a86,
        make("hand", &[(LOG, 1)], 4),
    ),
    block(
        "coral_brick",
        Bricks,
        0xd98b74,
        0xefe0c8,
        make("oven", &[(CLAY, 2)], 2),
    ),
    block(
        "sailcloth",
        Cloth,
        0xe9e4d4,
        0x3d7ea6,
        make("hand", &[(LEAVES, 6)], 2),
    ),
];

demo_kit::building!("tidewater", BLOCKS);
