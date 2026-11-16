//! A Mod declares Settlements, and a Biome lists them, resolved by name once every Mod has
//! declared.

use tesserae_core::settlement::{Building, Settlement};
use tesserae_core::{BiomeSettlement, BiomeTable, Tables};

use crate::bindings::tesserae::mods::content::Declaration;
use crate::{
    Behaviour, DeclaredSettlements, DeclaredTemplates, ModError, Registry, apply, biome_rows,
    resolve, resolve_biomes,
};

fn data(toml: &str) -> Result<Declaration, ModError> {
    let payload =
        crate::trust::CheckedPayload::for_test("content.toml", [("content.toml", toml.into())]);
    crate::data::parse("t", &payload)
}

/// One Mod's Settlements and Biomes, resolved as a World's load does.
fn load(toml: &str) -> Result<(Vec<Settlement>, BiomeTable), ModError> {
    let (mut tables, mut registry) = (Tables::builtin(), Registry::builtin());
    let mut behaviours: Vec<Behaviour> =
        vec![apply(data(toml)?, "t", &mut tables, &mut registry, true)?.behaviour];
    let rows = biome_rows(&behaviours);
    let templates = DeclaredTemplates::take(&mut behaviours);
    let settlements = DeclaredSettlements::take(&mut behaviours);
    resolve(behaviours, &mut tables, &registry)?;
    tables.templates = templates.resolve(&registry, &tables)?;
    tables.settlements = settlements.resolve(&registry, &tables)?;
    let biomes = resolve_biomes(&rows, &tables, &registry, None)?.0;
    Ok((tables.settlements, biomes))
}

/// A 2 m well with no door and a 4 m by 3 m house whose door is on its +X edge, a layer above
/// its floor.
const CONTENT: &str = r#"
[[materials]]
name = "planks"
emission = 0
dimming = 15
occlusion = "opaque"
render = "opaque"
solid = true

[[materials]]
name = "door"
emission = 0
dimming = 15
occlusion = "opaque"
render = "opaque"
solid = true

[[materials]]
name = "door_open"
emission = 0
dimming = 15
occlusion = "opaque"
render = "opaque"
solid = true

[[voxel-uses]]
kind = "door"
material = "t:door"
open = "t:door_open"

[[templates]]
name = "well"
cell-m = 1.0
ground-layer = 0
layers = [["pp", "pp"]]

[templates.palette]
"p" = { material = "t:planks" }

[[templates]]
name = "house"
cell-m = 1.0
ground-layer = 0
layers = [["pppp", "pppp", "pppp"], ["pppp", "...d", "pppp"]]

[templates.palette]
"p" = { material = "t:planks" }
"d" = { material = "t:door" }
"." = "air"
"#;

fn settlement(fields: &str) -> String {
    format!("[[settlements]]\nname = \"village\"\npath = \"t:planks\"\nminimum = 1\n{fields}\n")
}

const VILLAGE: &str = "radius-m = 32.0\ncentre = \"t:well\"\nplots = 6\n\
                       templates = [{ template = \"t:house\", weight = 2.0, min = 1, max = 4 }]";

fn world(settlements: &str, rows: &str) -> String {
    format!(
        "{CONTENT}\n{settlements}\n[[biomes]]\nname = \"plain\"\ntemperature = 0.5\n\
         humidity = 0.5\nsettlements = [{rows}]\n"
    )
}

#[test]
fn a_biome_lists_a_settlement_by_name_and_each_door_is_found() {
    let (settlements, biomes) = load(&world(
        &settlement(VILLAGE),
        "{ settlement = \"t:village\", spacing-m = 120.0 }",
    ))
    .unwrap();
    let village = &settlements[0];
    assert_eq!(village.name, "t:village");
    assert_eq!(village.margin_m, 16.0, "the margin is 16 m unless it says");
    assert_eq!(
        village.centre,
        Some(Building {
            template: 0,
            door: None
        })
    );
    assert_eq!(
        village.rules[0].building,
        Building {
            template: 1,
            door: Some([3, 1])
        }
    );
    assert_eq!(
        biomes.as_slice()[0].surface_features.settlements(),
        [BiomeSettlement {
            settlement: 0,
            spacing_m: 120.0
        }]
    );
}

fn refusal(settlements: &str, rows: &str) -> String {
    load(&world(settlements, rows)).unwrap_err().to_string()
}

#[test]
fn a_third_row_a_wide_radius_too_many_plots_or_an_unknown_template_names_the_settlement() {
    let row = "{ settlement = \"t:village\", spacing-m = 120.0 }";
    let third = refusal(&settlement(VILLAGE), &[row; 3].join(", "));
    assert!(
        third.contains("t:village") && third.contains("at most 2"),
        "{third}"
    );
    let wide = refusal(&settlement(&VILLAGE.replace("32.0", "64.5")), "");
    assert!(
        wide.contains("`village`") && wide.contains("radius"),
        "{wide}"
    );
    let many = refusal(&settlement(&VILLAGE.replace("plots = 6", "plots = 25")), "");
    assert!(many.contains("`village`") && many.contains("24"), "{many}");
    let unknown = refusal(&settlement(&VILLAGE.replace("t:house", "t:castle")), "");
    assert!(
        unknown.contains("`village`") && unknown.contains("t:castle"),
        "{unknown}"
    );
    let missing = refusal(
        &settlement(VILLAGE),
        "{ settlement = \"t:town\", spacing-m = 120.0 }",
    );
    assert!(missing.contains("t:town"), "{missing}");
    let close = refusal(
        &settlement(VILLAGE),
        "{ settlement = \"t:village\", spacing-m = 90.0 }",
    );
    assert!(
        close.contains("t:village") && close.contains("twice"),
        "{close}"
    );
}
