//! Lumen's building Materials: silverwood, moonstone, amethyst, glowcap and dusk-coloured planks.

use demo_kit::building::{Block, Pattern::*, block, make};

const LOG: &str = "flora:log";
const LEAVES: &str = "flora:leaves";
const STONE: &str = "strata:stone";
const SAND: &str = "strata:sand";
const SOIL: &str = "strata:soil";
const AMETHYST: &str = "crystals:amethyst";

pub const BLOCKS: [Block; 16] = [
    block(
        "pale_plaster",
        Plaster,
        0xd9dcf0,
        0xd9dcf0,
        make("oven", &[(SAND, 1), (STONE, 1)], 2),
    ),
    block(
        "silverwood",
        Log,
        0xa9aec8,
        0xd9dcf0,
        make("hand", &[(LOG, 1)], 1),
    ),
    block(
        "amethyst",
        Crystal,
        0x8a6cc9,
        0x8a6cc9,
        make("hand", &[(AMETHYST, 1)], 1),
    ),
    block(
        "moon_cobble",
        Cobble,
        0x8c8fb3,
        0x5d5f85,
        make("hand", &[(STONE, 1)], 1),
    ),
    block(
        "moonstone",
        Ashlar,
        0x9b9ec2,
        0x6b6e94,
        make("anvil", &[(STONE, 2)], 2),
    ),
    block(
        "glowcap",
        Thatch,
        0x7fc7b5,
        0x7fc7b5,
        make("hand", &[(LEAVES, 4)], 2),
    ),
    block(
        "silver_tiles",
        Tiles,
        0xa8b3c9,
        0xa8b3c9,
        make("oven", &[(SAND, 2)], 4),
    ),
    block(
        "dusk_shingles",
        Shingles,
        0x4a4f7a,
        0x4a4f7a,
        make("hand", &[("lumen:dusk_planks", 2)], 4),
    ),
    block(
        "night_slate",
        Slate,
        0x2f3557,
        0x2f3557,
        make("anvil", &[(STONE, 1), (AMETHYST, 1)], 3),
    ),
    block(
        "prism_tiles",
        Tiles,
        0xb48fd6,
        0xb48fd6,
        make("oven", &[(SAND, 1), (AMETHYST, 1)], 4),
    ),
    block(
        "star_planks",
        Planks,
        0x5d5a8a,
        0x5d5a8a,
        make("hand", &[(LOG, 1)], 4),
    ),
    block(
        "dusk_planks",
        Planks,
        0x3f3d66,
        0x3f3d66,
        make("hand", &[(LOG, 1)], 4),
    ),
    block(
        "ink_planks",
        Planks,
        0x2a2944,
        0x2a2944,
        make("hand", &[(LOG, 1)], 4),
    ),
    block(
        "violet_planks",
        Planks,
        0x6a4a9a,
        0x6a4a9a,
        make("hand", &[(LOG, 1)], 4),
    ),
    block(
        "moon_brick",
        Bricks,
        0x8c8fb3,
        0x4a4c70,
        make("oven", &[(STONE, 1), (SOIL, 1)], 2),
    ),
    block(
        "lumen_cloth",
        Cloth,
        0x8fd3c8,
        0x3f3d66,
        make("hand", &[(LEAVES, 6)], 2),
    ),
];

demo_kit::building!("lumen", BLOCKS);
