//! Recipes resolve against every Mod's Items once all have declared, and a bad
//! row is a load error naming the Recipe.

use tesserae_core::craft::{Recipe, Station};
use tesserae_core::inventory::Stack;
use tesserae_core::{Tables, inventory};

use super::declare::declaring;
use crate::bindings::tesserae::mods::content;
use crate::{
    DeclaredRecipes, ItemDef, MAX_RECIPES, MAX_RECIPES_PER_MOD, ModError, Registry, apply,
};

fn item(name: &str, max_stack: u32) -> ItemDef {
    ItemDef {
        name: name.into(),
        size_m: 0.25,
        color: [0, 0, 0, 255],
        material: None,
        max_stack,
        food: None,
    }
}

fn items() -> Vec<ItemDef> {
    vec![
        item("t:wood", 64),
        item("t:workbench", 16),
        item("u:plank", 64),
    ]
}

fn row(inputs: &[(&str, u32)], output: (&str, u32), station: &str) -> content::RecipeDef {
    let stack = |(item, count): (&str, u32)| content::RecipeStack {
        item: item.into(),
        count,
    };
    content::RecipeDef {
        inputs: inputs.iter().copied().map(stack).collect(),
        output: stack(output),
        station: station.into(),
    }
}

fn workbench() -> content::RecipeDef {
    row(&[("t:wood", 4)], ("t:workbench", 1), "hand")
}

fn applied(mods: &[(&str, Vec<content::RecipeDef>)]) -> Result<DeclaredRecipes, ModError> {
    let (mut tables, mut registry) = (Tables::builtin(), Registry::builtin());
    let mut behaviours = Vec::new();
    for (id, recipes) in mods {
        let mut d = declaring(&[], &[]);
        d.recipes = recipes.clone();
        behaviours.push(apply(d, id, &mut tables, &mut registry, false)?.behaviour);
    }
    Ok(DeclaredRecipes::take(&mut behaviours))
}

fn resolved(mods: &[(&str, Vec<content::RecipeDef>)]) -> Result<Vec<Recipe>, ModError> {
    applied(mods)?.resolve(&items(), &[])
}

fn stack(item: &str, count: u32) -> Stack {
    inventory::Stack {
        item: item.into(),
        count,
    }
}

#[test]
fn recipes_resolve_in_load_order_and_may_name_another_mods_items() {
    let recipes = resolved(&[
        ("t", vec![workbench()]),
        ("u", vec![row(&[("t:wood", 1)], ("u:plank", 4), "hand")]),
    ])
    .unwrap();
    assert_eq!(
        recipes,
        [
            Recipe {
                inputs: vec![stack("t:wood", 4)],
                output: stack("t:workbench", 1),
                station: Station::Hand,
            },
            Recipe {
                inputs: vec![stack("t:wood", 1)],
                output: stack("u:plank", 4),
                station: Station::Hand,
            },
        ]
    );
}

#[test]
fn a_station_recipe_names_a_declared_station_tag() {
    let declared = applied(&[(
        "t",
        vec![row(&[("t:wood", 8)], ("t:workbench", 1), "workbench")],
    )])
    .unwrap();
    assert_eq!(
        declared.resolve(&items(), &["workbench"]).unwrap()[0].station,
        Station::Tag("workbench".into())
    );
}

#[test]
fn a_bad_recipe_is_a_load_error_naming_the_mod_the_row_and_its_output() {
    let nine: Vec<_> = (0..9).map(|_| ("t:wood", 1)).collect();
    let ten = [nine.as_slice(), &[("u:plank", 1)]].concat();
    for (bad, says) in [
        (row(&[("t:nope", 4)], ("t:workbench", 1), "hand"), "t:nope"),
        (row(&[("t:wood", 4)], ("t:nope", 1), "hand"), "t:nope"),
        (row(&[], ("t:workbench", 1), "hand"), "inputs"),
        (row(&ten, ("t:workbench", 1), "hand"), "inputs"),
        (
            row(&[("t:wood", 1), ("t:wood", 1)], ("t:workbench", 1), "hand"),
            "inputs",
        ),
        (row(&[("t:wood", 0)], ("t:workbench", 1), "hand"), "inputs"),
        (row(&[("t:wood", 4)], ("t:workbench", 0), "hand"), "output"),
        (
            row(&[("t:wood", 4)], ("t:workbench", 17), "hand"),
            "max-stack",
        ),
        (row(&[("t:wood", 4)], ("t:workbench", 1), "anvil"), "anvil"),
        (row(&[("t:wood", 4)], ("t:workbench", 1), ""), "station"),
    ] {
        let error = resolved(&[("m", vec![workbench(), bad])]).unwrap_err();
        let text = error.to_string();
        assert!(matches!(error, ModError::Recipe { row: 1, .. }), "{text}");
        assert!(text.contains("`m`") && text.contains("recipe 1"), "{text}");
        assert!(text.contains(says), "{text}");
    }
}

#[test]
fn recipes_are_bounded_per_mod_and_per_world() {
    let many = |n| vec![workbench(); n];
    let error = applied(&[("m", many(MAX_RECIPES_PER_MOD + 1))]).unwrap_err();
    assert!(
        matches!(
            error,
            ModError::TooManyRows {
                what: "recipe",
                max: MAX_RECIPES_PER_MOD,
                ..
            }
        ),
        "{error}"
    );
    let mods: Vec<_> = ["a", "b", "c", "d", "e"]
        .iter()
        .map(|id| (*id, many(MAX_RECIPES_PER_MOD)))
        .collect();
    let error = resolved(&mods).unwrap_err();
    assert!(
        matches!(
            error,
            ModError::TooManyRows {
                what: "recipe",
                max: MAX_RECIPES,
                ..
            }
        ),
        "{error}"
    );
}
