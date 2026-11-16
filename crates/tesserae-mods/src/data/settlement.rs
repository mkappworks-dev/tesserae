use serde::Deserialize;

use crate::bindings::tesserae::mods::content;

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(super) struct Settlement {
    name: String,
    #[serde(alias = "radius-max-m")]
    radius_m: f32,
    radius_min_m: Option<f32>,
    margin_m: Option<f32>,
    centre: Option<String>,
    #[serde(alias = "plots-max")]
    plots: u8,
    plots_min: Option<u8>,
    templates: Vec<SettlementPlot>,
    #[serde(default)]
    minimum: u8,
    path: String,
    path_shape: Option<String>,
    loot: Option<Loot>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SettlementPlot {
    template: String,
    #[serde(default = "one")]
    weight: f32,
    #[serde(default)]
    min: u8,
    #[serde(default = "every")]
    max: u8,
    loot: Option<Loot>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(super) struct Loot {
    #[serde(default = "one_roll")]
    rolls_min: u8,
    rolls_max: Option<u8>,
    entries: Vec<LootEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LootEntry {
    item: String,
    #[serde(default = "one_item")]
    min: u32,
    /// Absent is `min`.
    max: Option<u32>,
    #[serde(default = "one")]
    weight: f32,
}

fn one_roll() -> u8 {
    1
}

fn one_item() -> u32 {
    1
}

impl From<Loot> for content::LootTable {
    fn from(l: Loot) -> Self {
        Self {
            rolls_min: l.rolls_min,
            rolls_max: l.rolls_max.unwrap_or(l.rolls_min),
            entries: l
                .entries
                .into_iter()
                .map(|e| content::LootEntry {
                    item: e.item,
                    min: e.min,
                    max: e.max.unwrap_or(e.min),
                    weight: e.weight,
                })
                .collect(),
        }
    }
}

fn one() -> f32 {
    1.0
}

fn every() -> u8 {
    tesserae_core::settlement::MAX_PLOTS
}

impl From<Settlement> for content::SettlementDef {
    fn from(s: Settlement) -> Self {
        Self {
            name: s.name,
            radius_m: s.radius_m,
            radius_min_m: s.radius_min_m,
            margin_m: s.margin_m,
            centre: s.centre,
            plots: s.plots,
            plots_min: s.plots_min,
            templates: s
                .templates
                .into_iter()
                .map(|p| content::SettlementPlot {
                    template: p.template,
                    weight: p.weight,
                    min: p.min,
                    max: p.max,
                    loot: p.loot.map(Into::into),
                })
                .collect(),
            minimum: s.minimum,
            path: s.path,
            path_shape: s.path_shape,
            loot: s.loot.map(Into::into),
        }
    }
}
