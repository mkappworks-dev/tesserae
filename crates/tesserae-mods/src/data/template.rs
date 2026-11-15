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
