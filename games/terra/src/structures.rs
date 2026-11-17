//! The Settlements Terra lays out from its Templates, and the rows its Biomes list them and its
//! wilderness Templates in.

use crate::tesserae::mods::content::{
    BiomeSettlement, BiomeStructure, LootEntry, LootTable, SettlementDef, SettlementOutskirts,
    SettlementPlot,
};

/// The smallest and largest copy's disc of each kind. No Template is wider than the smallest, so
/// every one fits a Plot.
const HAMLET_RADIUS_M: (f32, f32) = (24.0, 32.0);
const VILLAGE_RADIUS_M: (f32, f32) = (40.0, 64.0);
/// Wide enough that the largest copies grow a second ring of streets.
const TOWN_RADIUS_M: (f32, f32) = (96.0, 150.0);
/// Room round a Settlement's disc for the paths that lead into it and two rows of fields beside
/// them.
const MARGIN_M: f32 = 44.0;
const TOWN_MARGIN_M: f32 = 52.0;
/// Each lattice is over twice its kind's largest radius and margin: few cells pass the ground
/// rules, so a World still shows a village every few hundred metres, hamlets between them and a
/// town rarely.
const HAMLET_SPACING_M: f32 = 160.0;
const VILLAGE_SPACING_M: f32 = 224.0;
const TOWN_SPACING_M: f32 = 416.0;
/// Thins the lattices' draws, so Settlements stand apart rather than in every cell.
const HAMLET_CHANCE: f32 = 0.8;
const VILLAGE_CHANCE: f32 = 0.7;
/// A packed earth road, laid by the ground itself.
const VILLAGE_PATH: &str = "terra:dirt";
/// Over twice the widest wilderness Template, so they stand apart.
const WILD_SPACING_M: f32 = 256.0;
const WILD_CHANCE: f32 = 0.5;
/// A bridge's cell searches itself for a gap and seldom finds one: a closer lattice, every cell
/// drawn.
const BRIDGE_SPACING_M: f32 = 96.0;

fn plot(name: &str, weight: f32, min: u8, max: u8) -> SettlementPlot {
    SettlementPlot {
        template: format!("terra:{name}"),
        weight,
        min,
        max,
        loot: None,
    }
}

/// `rolls` Stacks, each `(item, min, max, weight)`.
fn loot(rolls: (u8, u8), entries: &[(&str, u32, u32, f32)]) -> LootTable {
    LootTable {
        rolls_min: rolls.0,
        rolls_max: rolls.1,
        entries: entries
            .iter()
            .map(|&(item, min, max, weight)| LootEntry {
                item: item.into(),
                min,
                max,
                weight,
            })
            .collect(),
    }
}

fn stocked(plot: SettlementPlot, table: LootTable) -> SettlementPlot {
    SettlementPlot {
        loot: Some(table),
        ..plot
    }
}

/// A village or hamlet keeps a little food and seed.
fn village_loot() -> LootTable {
    loot(
        (1, 3),
        &[
            ("crops:bread", 1, 2, 3.0),
            ("crops:wheat", 2, 6, 3.0),
            ("crops:carrot", 1, 4, 2.0),
            ("crops:potato", 1, 4, 2.0),
            ("flora:apple", 1, 3, 2.0),
            ("crops:wheat_seeds", 2, 5, 2.0),
            ("terra:oak_planks", 2, 8, 1.0),
            ("terra:rock", 1, 4, 1.0),
        ],
    )
}

/// A town keeps more, and better: pies, iron, glass and lamps.
fn town_loot() -> LootTable {
    loot(
        (3, 6),
        &[
            ("crops:bread", 2, 4, 3.0),
            ("crops:berry_pie", 1, 2, 2.0),
            ("flora:cherries", 2, 5, 2.0),
            ("flora:orange", 2, 5, 2.0),
            ("terra:iron", 1, 4, 2.0),
            ("terra:glass", 2, 6, 1.5),
            ("terra:lamp", 1, 2, 1.0),
            ("terra:spruce_planks", 4, 12, 1.0),
            ("terra:brick", 4, 12, 1.0),
        ],
    )
}

fn smithy_loot() -> LootTable {
    loot(
        (2, 4),
        &[
            ("terra:iron", 1, 4, 3.0),
            ("terra:rock", 2, 8, 2.0),
            ("terra:cobblestone", 4, 12, 2.0),
            ("terra:wood", 2, 6, 1.0),
        ],
    )
}

fn bakery_loot() -> LootTable {
    loot(
        (2, 4),
        &[
            ("crops:bread", 2, 5, 4.0),
            ("crops:berry_pie", 1, 2, 2.0),
            ("crops:wheat", 4, 10, 2.0),
            ("crops:berries", 2, 6, 1.0),
        ],
    )
}

/// What a wilderness Structure's chest in each of Terra's Biomes holds, by Biome name.
pub fn biome_loot(biome: &str) -> LootTable {
    match biome {
        "dry" => loot(
            (1, 3),
            &[
                ("strata:sand", 2, 8, 2.0),
                ("flora:cactus", 1, 3, 2.0),
                ("terra:terracotta_tiles", 2, 6, 1.0),
                ("terra:iron", 1, 2, 1.0),
            ],
        ),
        "cold" => loot(
            (1, 3),
            &[
                ("terra:snow", 2, 8, 2.0),
                ("flora:pine_sapling", 1, 2, 2.0),
                ("terra:spruce_planks", 2, 6, 2.0),
                ("terra:iron", 1, 2, 1.0),
            ],
        ),
        "jungle" => loot(
            (1, 3),
            &[
                ("flora:coconut", 1, 3, 2.0),
                ("flora:jungle_sapling", 1, 2, 2.0),
                ("flora:vine", 2, 6, 2.0),
                ("terra:lamp", 1, 1, 1.0),
            ],
        ),
        "swamp" => loot(
            (1, 3),
            &[
                ("terra:mud", 2, 8, 2.0),
                ("flora:mushroom", 1, 4, 2.0),
                ("aquatic:lily_pad", 1, 3, 2.0),
            ],
        ),
        "savanna" => loot(
            (1, 3),
            &[
                ("flora:acacia_sapling", 1, 2, 2.0),
                ("flora:dry_grass", 2, 6, 2.0),
                ("terra:clay_tiles", 2, 6, 1.0),
                ("terra:iron", 1, 2, 1.0),
            ],
        ),
        _ => loot(
            (1, 3),
            &[
                ("flora:apple", 1, 3, 2.0),
                ("flora:oak_sapling", 1, 2, 2.0),
                ("terra:rock", 2, 6, 2.0),
                ("terra:iron", 1, 2, 1.0),
            ],
        ),
    }
}

/// A Settlement of Terra's: its disc and most Plots run from its smallest copy to its largest.
#[allow(clippy::too_many_arguments)]
fn kind(
    name: &str,
    (radius_min_m, radius_m): (f32, f32),
    (plots_min, plots): (u8, u8),
    templates: Vec<SettlementPlot>,
    minimum: u8,
    path: &str,
    loot: LootTable,
    (margin_m, outskirts): (f32, SettlementOutskirts),
) -> SettlementDef {
    SettlementDef {
        name: name.into(),
        radius_m,
        radius_min_m: Some(radius_min_m),
        margin_m: Some(margin_m),
        centre: Some("terra:well".into()),
        plots,
        plots_min: Some(plots_min),
        templates,
        minimum,
        path: path.into(),
        path_shape: None,
        loot: Some(loot),
        outskirts: Some(vec![outskirts]),
    }
}

/// Fields, orchards, paddocks and sheds beside the approach paths of `chance` of a kind's copies,
/// from `count.0` to `count.1` of them; a copy without room for `count.0` has none.
fn farms(chance: f32, count: (u8, u8), templates: Vec<SettlementPlot>) -> SettlementOutskirts {
    SettlementOutskirts {
        templates,
        count_min: count.0,
        count_max: count.1,
        chance: Some(chance),
    }
}

pub fn settlements() -> Vec<SettlementDef> {
    vec![
        kind(
            "village",
            VILLAGE_RADIUS_M,
            (8, 20),
            vec![
                plot("cottage", 4.0, 1, 6),
                plot("family_house", 3.0, 1, 4),
                plot("farmhouse", 2.0, 0, 3),
                plot("inn", 1.0, 0, 1),
                plot("warehouse", 1.0, 0, 2),
                stocked(plot("blacksmith", 1.0, 0, 1), smithy_loot()),
                stocked(plot("bakery", 1.0, 0, 1), bakery_loot()),
                plot("market_stall", 2.0, 0, 3),
                plot("barn", 1.0, 0, 2),
                plot("garden", 2.0, 0, 3),
            ],
            5,
            VILLAGE_PATH,
            village_loot(),
            (
                MARGIN_M,
                farms(
                    0.65,
                    (6, 10),
                    vec![
                        plot("wheat_field", 2.0, 0, 3),
                        plot("ripe_wheat_field", 1.5, 0, 2),
                        plot("mixed_field", 1.5, 0, 2),
                        plot("carrot_field", 1.0, 0, 2),
                        plot("potato_field", 1.0, 0, 2),
                        plot("berry_field", 1.0, 0, 2),
                        plot("watermelon_patch", 1.0, 0, 1),
                        plot("cane_field", 1.0, 0, 1),
                        plot("mushroom_shed", 0.5, 0, 1),
                        plot("orchard", 1.0, 0, 1),
                        plot("paddock", 1.0, 0, 2),
                    ],
                ),
            ),
        ),
        kind(
            "town",
            TOWN_RADIUS_M,
            (30, 40),
            vec![
                // One at least: two of the larger cottages crowd out the minimums after them.
                plot("cottage", 4.0, 1, 12),
                plot("family_house", 3.0, 1, 8),
                plot("farmhouse", 2.0, 1, 4),
                plot("inn", 1.0, 1, 2),
                plot("warehouse", 1.0, 0, 3),
                stocked(plot("blacksmith", 1.0, 1, 2), smithy_loot()),
                stocked(plot("bakery", 1.0, 1, 2), bakery_loot()),
                plot("market_stall", 2.0, 1, 6),
                plot("barn", 1.0, 0, 3),
                plot("garden", 2.0, 0, 6),
            ],
            20,
            "terra:cobblestone",
            town_loot(),
            (
                TOWN_MARGIN_M,
                farms(
                    0.85,
                    (10, 16),
                    vec![
                        plot("wheat_field", 2.5, 0, 5),
                        plot("ripe_wheat_field", 2.0, 0, 4),
                        plot("mixed_field", 1.5, 0, 3),
                        plot("carrot_field", 1.0, 0, 3),
                        plot("potato_field", 1.0, 0, 3),
                        plot("berry_field", 1.0, 0, 2),
                        plot("watermelon_patch", 1.0, 0, 2),
                        plot("cane_field", 1.0, 0, 2),
                        plot("mushroom_shed", 0.5, 0, 2),
                        plot("orchard", 1.5, 0, 3),
                        plot("paddock", 1.0, 0, 3),
                    ],
                ),
            ),
        ),
        kind(
            "hamlet",
            HAMLET_RADIUS_M,
            (3, 6),
            vec![
                plot("cottage", 4.0, 1, 4),
                plot("family_house", 3.0, 0, 2),
                plot("farmhouse", 2.0, 0, 2),
                stocked(plot("bakery", 1.0, 0, 1), bakery_loot()),
                plot("garden", 1.0, 0, 1),
            ],
            2,
            VILLAGE_PATH,
            village_loot(),
            (
                MARGIN_M,
                farms(
                    0.4,
                    (4, 6),
                    vec![
                        plot("carrot_field", 1.0, 0, 2),
                        plot("potato_field", 1.0, 0, 2),
                        plot("berry_field", 1.0, 0, 2),
                        plot("cane_field", 0.5, 0, 1),
                        plot("mushroom_shed", 0.5, 0, 1),
                        plot("paddock", 1.0, 0, 1),
                    ],
                ),
            ),
        ),
    ]
}

/// A Biome's row for the Settlement `name`; a village row stands one near spawn.
fn row(name: &str, spacing_m: f32, chance: f32) -> BiomeSettlement {
    BiomeSettlement {
        settlement: format!("terra:{name}"),
        spacing_m,
        chance,
        near_spawn: name == "village",
    }
}

/// A settled Biome's rows: the village first, so a hamlet the lattice puts where one stands gives
/// way.
pub fn village() -> Vec<BiomeSettlement> {
    vec![
        row("village", VILLAGE_SPACING_M, VILLAGE_CHANCE),
        row("hamlet", HAMLET_SPACING_M, HAMLET_CHANCE),
    ]
}

/// The town first, so a village the lattice puts where it stands gives way.
pub fn town_and_village() -> Vec<BiomeSettlement> {
    vec![
        row("town", TOWN_SPACING_M, 1.0),
        row("village", VILLAGE_SPACING_M, VILLAGE_CHANCE),
    ]
}

/// Each of `names`, a wilderness Template of Terra's.
pub fn wild(names: &[&str]) -> Vec<BiomeStructure> {
    names
        .iter()
        .map(|name| BiomeStructure {
            template: format!("terra:{name}"),
            spacing_m: WILD_SPACING_M,
            chance: WILD_CHANCE,
        })
        .collect()
}

/// Terra's bridge `name`, a Template that spans water.
pub fn bridge(name: &str) -> Vec<BiomeStructure> {
    vec![BiomeStructure {
        template: format!("terra:{name}"),
        spacing_m: BRIDGE_SPACING_M,
        chance: 1.0,
    }]
}
