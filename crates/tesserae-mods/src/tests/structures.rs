//! A Biome lists the Templates worldgen stands as Structures, resolved by name once every Mod
//! has declared.

use tesserae_core::{BiomeStructure, BiomeTable, Tables};

use crate::bindings::tesserae::mods::content::Declaration;
use crate::{
    Behaviour, DeclaredTemplates, ModError, Registry, apply, biome_rows, resolve, resolve_biomes,
};

fn data(toml: &str) -> Result<Declaration, ModError> {
    let payload =
        crate::trust::CheckedPayload::for_test("content.toml", [("content.toml", toml.into())]);
    crate::data::parse("t", &payload)
}

/// One Mod's Biomes, resolved against its Templates as a World's load does.
fn load(toml: &str) -> Result<BiomeTable, ModError> {
    let (mut tables, mut registry) = (Tables::builtin(), Registry::builtin());
    let mut behaviours: Vec<Behaviour> =
        vec![apply(data(toml)?, "t", &mut tables, &mut registry, true)?.behaviour];
    let rows = biome_rows(&behaviours);
    let declared = DeclaredTemplates::take(&mut behaviours);
    resolve(behaviours, &mut tables, &registry)?;
    tables.templates = declared.resolve(&registry, &tables)?;
    Ok(resolve_biomes(&rows, &tables, &registry, None)?.0)
}

/// A 4 by 3 m shed and a 2 m square well, so a shed's spacing is at least 8 m.
const TEMPLATES: &str = r#"
[[materials]]
name = "planks"
emission = 0
dimming = 15
occlusion = "opaque"
render = "opaque"
solid = true

[[templates]]
name = "well"
cell-m = 1.0
ground-layer = 0
layers = [["pp", "pp"]]

[templates.palette]
"p" = { material = "t:planks" }

[[templates]]
name = "shed"
cell-m = 1.0
ground-layer = 0
layers = [["pppp", "pppp", "pppp"]]

[templates.palette]
"p" = { material = "t:planks" }
"#;

fn biome(structures: &str) -> String {
    format!(
        "{TEMPLATES}\n[[biomes]]\nname = \"plain\"\ntemperature = 0.5\nhumidity = 0.5\n\
         structures = [{structures}]\n"
    )
}

#[test]
fn a_biome_lists_structures_by_template_name_with_a_spacing_and_a_chance() {
    let biomes = load(&biome(
        "{ template = \"t:shed\", spacing-m = 8.0, chance = 0.25 }, \
         { template = \"t:well\", spacing-m = 40.0, chance = 1.0 }",
    ))
    .unwrap();
    assert_eq!(
        biomes.as_slice()[0].surface_features.structures(),
        [
            BiomeStructure {
                template: 1,
                spacing_m: 8.0,
                chance: 0.25,
            },
            BiomeStructure {
                template: 0,
                spacing_m: 40.0,
                chance: 1.0,
            },
        ]
    );
}

fn refusal(structures: &str) -> String {
    load(&biome(structures)).unwrap_err().to_string()
}

#[test]
fn a_fifth_row_an_unknown_template_or_a_spacing_under_its_floor_names_the_biome() {
    let row = "{ template = \"t:well\", spacing-m = 40.0, chance = 1.0 }";
    let fifth = refusal(&[row; 5].join(", "));
    assert!(
        fifth.contains("biome `plain`") && fifth.contains("4"),
        "{fifth}"
    );
    let unknown = refusal("{ template = \"t:castle\", spacing-m = 40.0, chance = 1.0 }");
    assert!(
        unknown.contains("biome `plain`") && unknown.contains("t:castle"),
        "{unknown}"
    );
    let close = refusal("{ template = \"t:shed\", spacing-m = 7.5, chance = 1.0 }");
    assert!(
        close.contains("biome `plain`") && close.contains("twice"),
        "{close}"
    );
    let unlikely = refusal("{ template = \"t:shed\", spacing-m = 8.0, chance = 0.0 }");
    assert!(unlikely.contains("biome `plain`"), "{unlikely}");
}
