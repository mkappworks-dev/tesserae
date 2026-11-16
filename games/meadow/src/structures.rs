//! The Settlements Meadow lays out from its Templates, and the rows its Biomes list them and its
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
const VILLAGE_SPACING_M: f32 = 256.0;
/// Thins the lattice's draws, so villages stand apart rather than in every cell.
const VILLAGE_CHANCE: f32 = 0.9;
/// A packed earth road, laid by the ground itself.
const VILLAGE_PATH: &str = "strata:soil";
/// Over twice the widest wilderness Template, so they stand apart.
const WILD_SPACING_M: f32 = 256.0;
const WILD_CHANCE: f32 = 0.5;

fn plot(name: &str, weight: f32, min: u8, max: u8) -> SettlementPlot {
    SettlementPlot {
        template: format!("meadow:{name}"),
        weight,
        min,
        max,
    }
}

pub fn settlements() -> Vec<SettlementDef> {
    vec![SettlementDef {
        name: "village".into(),
        radius_m: VILLAGE_RADIUS_M,
        margin_m: Some(MARGIN_M),
        centre: Some("meadow:pond_well".into()),
        plots: 12,
        templates: vec![
            plot("cottage", 4.0, 1, 4),
            plot("family_house", 3.0, 1, 3),
            plot("farmhouse", 2.0, 0, 2),
            plot("tavern", 1.0, 0, 1),
            plot("granary", 1.0, 0, 1),
            plot("tinker", 1.0, 0, 1),
            plot("bakery", 1.0, 0, 1),
            plot("honey_stall", 2.0, 0, 3),
            plot("mill_barn", 1.0, 0, 2),
        ],
        minimum: 3,
        path: VILLAGE_PATH.into(),
        path_shape: None,
    }]
}

pub fn village() -> Vec<BiomeSettlement> {
    vec![BiomeSettlement {
        settlement: "meadow:village".into(),
        spacing_m: VILLAGE_SPACING_M,
        chance: VILLAGE_CHANCE,
    }]
}

/// Each of `names`, a wilderness Template of Meadow's.
pub fn wild(names: &[&str]) -> Vec<BiomeStructure> {
    names
        .iter()
        .map(|name| BiomeStructure {
            template: format!("meadow:{name}"),
            spacing_m: WILD_SPACING_M,
            chance: WILD_CHANCE,
        })
        .collect()
}
