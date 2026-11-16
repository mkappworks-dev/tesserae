use std::collections::BTreeMap;

use serde::Deserialize;

use crate::bindings::tesserae::mods::content;

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(super) struct Template {
    name: String,
    cell_m: f32,
    palette: BTreeMap<char, Entry>,
    layers: Vec<Vec<String>>,
    ground_layer: Option<u32>,
    foundation: Option<String>,
    level_tolerance_m: Option<f32>,
}

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

#[derive(Deserialize)]
#[serde(untagged)]
enum Entry {
    Word(Word),
    Voxel(Voxel),
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum Word {
    Keep,
    Air,
    Ground,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Voxel {
    material: String,
    shape: Option<String>,
    orientation: Option<u8>,
}

impl From<Template> for content::TemplateDef {
    fn from(t: Template) -> Self {
        Self {
            name: t.name,
            cell_m: t.cell_m,
            palette: t
                .palette
                .into_iter()
                .map(|(symbol, entry)| content::TemplatePaletteEntry {
                    symbol,
                    cell: match entry {
                        Entry::Word(Word::Keep) => content::TemplateCell::Keep,
                        Entry::Word(Word::Air) => content::TemplateCell::Air,
                        Entry::Word(Word::Ground) => content::TemplateCell::Ground,
                        Entry::Voxel(v) => content::TemplateCell::Voxel(content::TemplateVoxel {
                            material: v.material,
                            shape: v.shape,
                            orientation: v.orientation,
                        }),
                    },
                })
                .collect(),
            layers: t.layers,
            ground_layer: t.ground_layer,
            foundation: t.foundation,
            level_tolerance_m: t.level_tolerance_m,
        }
    }
}
