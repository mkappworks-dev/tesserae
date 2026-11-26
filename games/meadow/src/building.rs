//! Meadow's building Materials: birch, walnut, fieldstone, straw, moss and honey-coloured tiles.

use demo_kit::building::{Block, Pattern::*, block, make};

const LOG: &str = "flora:log";
const LEAVES: &str = "flora:leaves";
const STONE: &str = "strata:stone";
const SAND: &str = "strata:sand";
const CLAY: &str = "strata:clay";
const SOIL: &str = "strata:soil";
const WHEAT: &str = "crops:wheat";

pub const BLOCKS: [Block; 16] = [
    block(
        "whitewash",
        Plaster,
        0xf1ead6,
        0xf1ead6,
        make("oven", &[(SAND, 1), (STONE, 1)], 2),
    ),
    block(
        "birch_log",
        Log,
        0xe6e1d3,
        0xd8c28f,
        make("hand", &[(LOG, 1)], 1),
    ),
    block(
        "walnut_log",
        Log,
        0x5a3d28,
        0xc9a46a,
        make("hand", &[(LOG, 1)], 1),
    ),
    block(
        "fieldstone",
        Cobble,
        0x9c9a88,
        0x6e6c5d,
        make("hand", &[(STONE, 1)], 1),
    ),
    block(
        "river_stone",
        Ashlar,
        0xb0ad9a,
        0x7d7a6a,
        make("anvil", &[(STONE, 2)], 2),
    ),
    block(
        "straw",
        Thatch,
        0xd8b64f,
        0xd8b64f,
        make("hand", &[(WHEAT, 2)], 2),
    ),
    block(
        "rose_tiles",
        Tiles,
        0xc97a7a,
        0xc97a7a,
        make("oven", &[(CLAY, 2)], 4),
    ),
    block(
        "moss_shingles",
        Shingles,
        0x6f8a45,
        0x6f8a45,
        make("hand", &[("meadow:birch_planks", 2), (LEAVES, 1)], 4),
    ),
    block(
        "moss_roof",
        Moss,
        0x6f9a45,
        0x9cc06a,
        make("hand", &[(LEAVES, 2), (SOIL, 1)], 2),
    ),
    block(
        "honey_tiles",
        Tiles,
        0xd69a3c,
        0xd69a3c,
        make("oven", &[(CLAY, 1), (SAND, 1)], 4),
    ),
    block(
        "birch_planks",
        Planks,
        0xd8c28f,
        0xd8c28f,
        make("hand", &[(LOG, 1)], 4),
    ),
    block(
        "honey_planks",
        Planks,
        0xc98f3a,
        0xc98f3a,
        make("hand", &[(LOG, 1)], 4),
    ),
    block(
        "walnut_planks",
        Planks,
        0x6b4a30,
        0x6b4a30,
        make("hand", &[(LOG, 1)], 4),
    ),
    block(
        "sage_planks",
        Planks,
        0x7f9a6a,
        0x7f9a6a,
        make("hand", &[(LOG, 1)], 4),
    ),
    block(
        "clay_brick",
        Bricks,
        0xb9704f,
        0xe9dfc9,
        make("oven", &[(CLAY, 2)], 2),
    ),
    block(
        "bloom_cloth",
        Cloth,
        0xe8b84a,
        0xf1ead6,
        make("hand", &[(WHEAT, 2), (LEAVES, 2)], 2),
    ),
];

demo_kit::building!("meadow", BLOCKS);
