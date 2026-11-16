use serde::Deserialize;

use crate::bindings::tesserae::mods::content;

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(super) struct Settlement {
    name: String,
    radius_m: f32,
    margin_m: Option<f32>,
    centre: Option<String>,
    plots: u8,
    templates: Vec<SettlementPlot>,
    #[serde(default)]
    minimum: u8,
    path: String,
    path_shape: Option<String>,
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
            margin_m: s.margin_m,
            centre: s.centre,
            plots: s.plots,
            templates: s
                .templates
                .into_iter()
                .map(|p| content::SettlementPlot {
                    template: p.template,
                    weight: p.weight,
                    min: p.min,
                    max: p.max,
                })
                .collect(),
            minimum: s.minimum,
            path: s.path,
            path_shape: s.path_shape,
        }
    }
}
