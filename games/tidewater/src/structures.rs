//! The Settlements Tidewater lays out from its Templates, and the rows its Biomes list them and its
//! wilderness Templates in.

use crate::tesserae::mods::content::{
    BiomeSettlement, BiomeStructure, SettlementDef, SettlementPlot,
};

/// No Template is wider than this, so every one fits a Plot; small, since islands are.
const VILLAGE_RADIUS_M: f32 = 28.0;
/// Room round a Settlement's disc for the paths that lead into it.
const MARGIN_M: f32 = 24.0;
/// Twice the village's radius and margin, the closest the lattice allows: few cells pass the
/// ground rules, so a World still shows one every few hundred metres.
const VILLAGE_SPACING_M: f32 = 104.0;
/// Thins the lattice's draws, so villages stand apart rather than in every cell.
const VILLAGE_CHANCE: f32 = 0.6;
/// A packed earth road, laid by the ground itself.
const VILLAGE_PATH: &str = "strata:soil";
/// Over twice the widest wilderness Template, so they stand apart.
const WILD_SPACING_M: f32 = 256.0;
const WILD_CHANCE: f32 = 0.5;

fn plot(name: &str, weight: f32, min: u8, max: u8) -> SettlementPlot {
    SettlementPlot {
        template: format!("tidewater:{name}"),
        weight,
        min,
        max,
    }
}

pub fn settlements() -> Vec<SettlementDef> {
    vec![SettlementDef {
        name: "harbour".into(),
        radius_m: VILLAGE_RADIUS_M,
        margin_m: Some(MARGIN_M),
        centre: Some("tidewater:tide_well".into()),
        plots: 8,
        templates: vec![
            plot("fisher_cottage", 4.0, 1, 4),
            plot("harbour_house", 3.0, 1, 3),
            plot("shore_farm", 2.0, 0, 2),
            plot("harbour_inn", 1.0, 0, 1),
            plot("net_loft", 1.0, 0, 1),
            plot("boatwright", 1.0, 0, 1),
            plot("smokehouse", 1.0, 0, 1),
            plot("fish_market", 2.0, 0, 3),
            plot("boathouse", 1.0, 0, 2),
        ],
        minimum: 3,
        path: VILLAGE_PATH.into(),
        path_shape: None,
    }]
}

pub fn village() -> Vec<BiomeSettlement> {
    vec![BiomeSettlement {
        settlement: "tidewater:harbour".into(),
        spacing_m: VILLAGE_SPACING_M,
        chance: VILLAGE_CHANCE,
    }]
}

/// Each of `names`, a wilderness Template of Tidewater's.
pub fn wild(names: &[&str]) -> Vec<BiomeStructure> {
    names
        .iter()
        .map(|name| BiomeStructure {
            template: format!("tidewater:{name}"),
            spacing_m: WILD_SPACING_M,
            chance: WILD_CHANCE,
        })
        .collect()
}
