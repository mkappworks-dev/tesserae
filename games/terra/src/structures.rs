//! The Settlements Terra lays out from its Templates, and the rows its Biomes list them and its
//! wilderness Templates in.

use crate::tesserae::mods::content::{
    BiomeSettlement, BiomeStructure, LootEntry, LootTable, SettlementDef, SettlementPlot,
};

/// No Template is wider than this, so every one fits a Plot.
const VILLAGE_RADIUS_M: f32 = 40.0;
/// Room round a Settlement's disc for the paths that lead into it.
const MARGIN_M: f32 = 24.0;
/// Twice the village's radius and margin, the closest the lattice allows: few cells pass the
/// ground rules, so a World still shows one every few hundred metres.
const VILLAGE_SPACING_M: f32 = 128.0;
/// Thins the lattice's draws, so villages stand apart rather than in every cell.
const VILLAGE_CHANCE: f32 = 0.7;
/// A packed earth road, laid by the ground itself.
const VILLAGE_PATH: &str = "terra:dirt";
/// A town's disc, and its lattice: wider apart than villages, so one is a rarer find.
const TOWN_RADIUS_M: f32 = 64.0;
const TOWN_SPACING_M: f32 = 384.0;
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

/// A village keeps a little food and seed.
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

pub fn settlements() -> Vec<SettlementDef> {
    vec![
        SettlementDef {
            name: "village".into(),
            radius_m: VILLAGE_RADIUS_M,
            margin_m: Some(MARGIN_M),
            centre: Some("terra:well".into()),
            plots: 12,
            templates: vec![
                plot("cottage", 4.0, 1, 4),
                plot("family_house", 3.0, 1, 3),
                plot("farmhouse", 2.0, 0, 2),
                plot("inn", 1.0, 0, 1),
                plot("warehouse", 1.0, 0, 1),
                stocked(plot("blacksmith", 1.0, 0, 1), smithy_loot()),
                stocked(plot("bakery", 1.0, 0, 1), bakery_loot()),
                plot("market_stall", 2.0, 0, 3),
                plot("barn", 1.0, 0, 2),
            ],
            minimum: 3,
            path: VILLAGE_PATH.into(),
            path_shape: None,
            loot: Some(village_loot()),
        },
        SettlementDef {
            name: "town".into(),
            radius_m: TOWN_RADIUS_M,
            margin_m: Some(MARGIN_M),
            centre: Some("terra:well".into()),
            plots: 22,
            templates: vec![
                // One at least: two of the larger cottages crowd out the minimums after them.
                plot("cottage", 4.0, 1, 6),
                plot("family_house", 3.0, 1, 4),
                plot("farmhouse", 2.0, 1, 3),
                plot("inn", 1.0, 1, 1),
                plot("warehouse", 1.0, 0, 2),
                stocked(plot("blacksmith", 1.0, 1, 1), smithy_loot()),
                stocked(plot("bakery", 1.0, 1, 2), bakery_loot()),
                plot("market_stall", 2.0, 1, 4),
                plot("barn", 1.0, 0, 3),
            ],
            minimum: 8,
            path: "terra:cobblestone".into(),
            path_shape: None,
            loot: Some(town_loot()),
        },
    ]
}

pub fn village() -> Vec<BiomeSettlement> {
    vec![BiomeSettlement {
        settlement: "terra:village".into(),
        spacing_m: VILLAGE_SPACING_M,
        chance: VILLAGE_CHANCE,
    }]
}

/// The town first, so a village the lattice puts where it stands gives way.
pub fn town_and_village() -> Vec<BiomeSettlement> {
    let mut rows = vec![BiomeSettlement {
        settlement: "terra:town".into(),
        spacing_m: TOWN_SPACING_M,
        chance: 1.0,
    }];
    rows.extend(village());
    rows
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
