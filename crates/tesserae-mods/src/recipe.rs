//! Every Mod's `recipes`, resolved against the World's Items.

use tesserae_core::craft::{HAND, MAX_INPUTS, Recipe, Station};
use tesserae_core::inventory::Stack;

use crate::bindings::tesserae::mods::content;
use crate::declare::{Behaviour, ItemDef};
use crate::error::ModError;

pub const MAX_RECIPES_PER_MOD: usize = 256;
pub const MAX_RECIPES: usize = 1024;

/// Every Mod's Recipe rows in load order, taken from its Behaviour before
/// [`crate::resolve`] consumes it: a Recipe may name an implicit Item, and
/// those are known only once every table is complete.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct DeclaredRecipes(Vec<(String, Vec<content::RecipeDef>)>);

impl DeclaredRecipes {
    pub fn take(behaviours: &mut [Behaviour]) -> Self {
        Self(
            behaviours
                .iter_mut()
                .map(|b| (b.mod_id.clone(), std::mem::take(&mut b.recipes)))
                .collect(),
        )
    }

    /// `items` are every Item in the World, declared and implicit; `stations`
    /// every declared Station tag.
    pub fn resolve(&self, items: &[ItemDef], stations: &[&str]) -> Result<Vec<Recipe>, ModError> {
        let mut recipes = Vec::new();
        for (mod_id, rows) in &self.0 {
            for (row, def) in rows.iter().enumerate() {
                if recipes.len() == MAX_RECIPES {
                    return Err(ModError::TooManyRows {
                        id: mod_id.clone(),
                        what: "recipe",
                        count: recipes.len() + 1,
                        max: MAX_RECIPES,
                    });
                }
                recipes.push(
                    recipe(def, items, stations).map_err(|reason| ModError::Recipe {
                        id: mod_id.clone(),
                        row,
                        output: def.output.item.clone(),
                        reason,
                    })?,
                );
            }
        }
        Ok(recipes)
    }
}

fn recipe(
    def: &content::RecipeDef,
    items: &[ItemDef],
    stations: &[&str],
) -> Result<Recipe, String> {
    let known = |name: &str| {
        items
            .iter()
            .find(|item| item.name == name)
            .ok_or_else(|| format!("names Item `{name}`, which no loaded Mod declares"))
    };
    if !(1..=MAX_INPUTS).contains(&def.inputs.len()) {
        return Err(format!("inputs must hold 1 to {MAX_INPUTS} rows"));
    }
    for (at, input) in def.inputs.iter().enumerate() {
        known(&input.item)?;
        if input.count == 0 {
            return Err("inputs: a count must be at least 1".into());
        }
        if def.inputs[..at].iter().any(|i| i.item == input.item) {
            return Err(format!("inputs name `{}` twice", input.item));
        }
    }
    let output = known(&def.output.item)?;
    if def.output.count == 0 {
        return Err("output: a count must be at least 1".into());
    }
    if def.output.count > output.max_stack {
        return Err(format!(
            "output: {} is more than its Item's max-stack of {}",
            def.output.count, output.max_stack
        ));
    }
    let station = if def.station == HAND {
        Station::Hand
    } else if stations.contains(&def.station.as_str()) {
        Station::Tag(def.station.clone())
    } else {
        return Err(format!(
            "station `{}` is neither `{HAND}` nor a declared Station's tag",
            def.station
        ));
    };
    let stack = |s: &content::RecipeStack| Stack {
        item: s.item.clone(),
        count: s.count,
    };
    Ok(Recipe {
        inputs: def.inputs.iter().map(stack).collect(),
        output: stack(&def.output),
        station,
    })
}
