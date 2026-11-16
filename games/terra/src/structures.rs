//! The Settlements Terra lays out from its Templates, and the rows its Biomes list them and its
//! wilderness Templates in.

use crate::tesserae::mods::content::{
    BiomeSettlement, BiomeStructure, SettlementDef, SettlementPlot,
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
                plot("blacksmith", 1.0, 0, 1),
                plot("bakery", 1.0, 0, 1),
                plot("market_stall", 2.0, 0, 3),
                plot("barn", 1.0, 0, 2),
            ],
            minimum: 3,
            path: VILLAGE_PATH.into(),
            path_shape: None,
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
                plot("blacksmith", 1.0, 1, 1),
                plot("bakery", 1.0, 1, 2),
                plot("market_stall", 2.0, 1, 4),
                plot("barn", 1.0, 0, 3),
            ],
            minimum: 8,
            path: "terra:cobblestone".into(),
            path_shape: None,
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
