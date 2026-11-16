//! A demo Game's one Settlement, and the rows its Biomes list it and its wilderness Templates in.

/// Room round a Settlement's disc for the paths that lead into it.
pub const MARGIN_M: f32 = 24.0;
/// A packed earth road, laid by the ground itself.
pub const PATH: &str = "strata:soil";
/// Below this many Plots besides its centre, a Settlement is dropped.
pub const MINIMUM: u8 = 3;
/// Over twice the widest wilderness Template, so they stand apart.
pub const WILD_SPACING_M: f32 = 256.0;
pub const WILD_CHANCE: f32 = 0.5;
/// A bridge's cell searches itself for a gap and seldom finds one, so its lattice is closer than
/// the other wilderness Templates' and every cell is drawn.
pub const BRIDGE_SPACING_M: f32 = 128.0;

/// A Template a Settlement draws its Plots from, by its unqualified name.
pub struct Plot {
    pub template: &'static str,
    pub weight: f32,
    pub min: u8,
    pub max: u8,
    /// What its chests hold, in place of the Settlement's.
    pub loot: Option<&'static Loot>,
}

pub const fn plot(template: &'static str, weight: f32, min: u8, max: u8) -> Plot {
    Plot {
        template,
        weight,
        min,
        max,
        loot: None,
    }
}

impl Plot {
    pub const fn stocked(self, loot: &'static Loot) -> Plot {
        Plot {
            loot: Some(loot),
            ..self
        }
    }
}

/// What a worldgen chest holds: `rolls` Stacks, each an entry `(item, min, max, weight)`, the Item
/// by its full name.
pub struct Loot {
    pub rolls: (u8, u8),
    pub entries: &'static [(&'static str, u32, u32, f32)],
}

pub struct Village {
    pub game: &'static str,
    pub name: &'static str,
    /// No Template is wider than this, so every one fits a Plot.
    pub radius_m: f32,
    /// At least twice the radius and [`MARGIN_M`]: few lattice cells pass the ground rules, so a
    /// World still shows one every few hundred metres.
    pub spacing_m: f32,
    /// Thins the lattice's draws, so villages stand apart rather than in every lattice cell.
    pub chance: f32,
    /// The centre Template, by its unqualified name.
    pub centre: &'static str,
    pub plots: u8,
    pub templates: &'static [Plot],
    /// What its buildings' chests hold, unless a Plot has its own.
    pub loot: &'static Loot,
}

impl Village {
    pub fn qualified(&self, name: &str) -> String {
        format!("{}:{name}", self.game)
    }
}

/// Writes `settlements`, `village`, `wild`, `bridge` and `loot` for the Game's [`Village`]
/// `$village`. Expanded
/// inside the Game, where `crate::tesserae::mods` is its WIT binding.
#[macro_export]
macro_rules! structures {
    ($village:path) => {
        pub fn settlements() -> Vec<crate::tesserae::mods::content::SettlementDef> {
            use crate::tesserae::mods::content::{SettlementDef, SettlementPlot};
            vec![SettlementDef {
                name: $village.name.into(),
                radius_m: $village.radius_m,
                margin_m: Some($crate::structures::MARGIN_M),
                centre: Some($village.qualified($village.centre)),
                plots: $village.plots,
                templates: $village
                    .templates
                    .iter()
                    .map(|p| SettlementPlot {
                        template: $village.qualified(p.template),
                        weight: p.weight,
                        min: p.min,
                        max: p.max,
                        loot: p.loot.map(loot),
                    })
                    .collect(),
                minimum: $crate::structures::MINIMUM,
                path: $crate::structures::PATH.into(),
                path_shape: None,
                loot: Some(loot($village.loot)),
            }]
        }

        /// A loot table, for the Settlement or a Biome's `loot`.
        pub fn loot(
            table: &$crate::structures::Loot,
        ) -> crate::tesserae::mods::content::LootTable {
            crate::tesserae::mods::content::LootTable {
                rolls_min: table.rolls.0,
                rolls_max: table.rolls.1,
                entries: table
                    .entries
                    .iter()
                    .map(
                        |&(item, min, max, weight)| crate::tesserae::mods::content::LootEntry {
                            item: item.into(),
                            min,
                            max,
                            weight,
                        },
                    )
                    .collect(),
            }
        }

        pub fn village() -> Vec<crate::tesserae::mods::content::BiomeSettlement> {
            vec![crate::tesserae::mods::content::BiomeSettlement {
                settlement: $village.qualified($village.name),
                spacing_m: $village.spacing_m,
                chance: $village.chance,
            }]
        }

        /// Each of `names`, a wilderness Template of the Game's.
        pub fn wild(names: &[&str]) -> Vec<crate::tesserae::mods::content::BiomeStructure> {
            names
                .iter()
                .map(|name| crate::tesserae::mods::content::BiomeStructure {
                    template: $village.qualified(name),
                    spacing_m: $crate::structures::WILD_SPACING_M,
                    chance: $crate::structures::WILD_CHANCE,
                })
                .collect()
        }

        /// The Game's bridge `name`, a Template that spans water.
        pub fn bridge(name: &str) -> Vec<crate::tesserae::mods::content::BiomeStructure> {
            vec![crate::tesserae::mods::content::BiomeStructure {
                template: $village.qualified(name),
                spacing_m: $crate::structures::BRIDGE_SPACING_M,
                chance: 1.0,
            }]
        }
    };
}
