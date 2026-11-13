use serde::Deserialize;

use crate::bindings::tesserae::mods::content;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Recipe {
    inputs: Vec<RecipeStack>,
    output: RecipeStack,
    station: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecipeStack {
    item: String,
    count: u32,
}

impl From<RecipeStack> for content::RecipeStack {
    fn from(s: RecipeStack) -> Self {
        Self {
            item: s.item,
            count: s.count,
        }
    }
}

impl From<Recipe> for content::RecipeDef {
    fn from(r: Recipe) -> Self {
        Self {
            inputs: r.inputs.into_iter().map(Into::into).collect(),
            output: r.output.into(),
            station: r.station,
        }
    }
}
