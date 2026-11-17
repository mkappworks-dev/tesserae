//! A Mod declares Settlements, and a Biome lists them, resolved by name once every Mod has
//! declared.

use tesserae_core::loot::{LootEntry, LootTable};
use tesserae_core::settlement::{Building, Settlement};
use tesserae_core::{BiomeSettlement, BiomeTable, Tables};

use crate::bindings::tesserae::mods::content::Declaration;
use crate::{
    Behaviour, DeclaredSettlements, DeclaredTemplates, ModError, Registry, apply, biome_loot,
    biome_rows, resolve, resolve_biomes, resolve_items,
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
            spacing_m: 120.0,
            chance: 1.0,
            near_spawn: false,
        }],
        "every lattice cell draws a Settlement unless the row says"
    );
}

#[test]
fn a_biome_settlement_row_may_say_near_spawn() {
    let (_, biomes) = load(&world(
        &settlement(VILLAGE),
        "{ settlement = \"t:village\", spacing-m = 120.0, near-spawn = true }",
    ))
    .unwrap();
    assert!(biomes.as_slice()[0].surface_features.settlements()[0].near_spawn);
}

#[test]
fn a_settlement_declares_its_plots_and_radius_as_ranges_and_a_single_value_is_a_fixed_size() {
    let town = "radius-min-m = 40.0\nradius-max-m = 160.0\ncentre = \"t:well\"\n\
                plots-min = 30\nplots-max = 96\n\
                templates = [{ template = \"t:house\", weight = 2.0, min = 1, max = 96 }]";
    let (settlements, _) = load(&world(
        &format!(
            "{}{}",
            settlement(VILLAGE),
            settlement(town).replace("village", "town")
        ),
        "{ settlement = \"t:town\", spacing-m = 352.0 }",
    ))
    .unwrap();
    let [village, town] = &settlements[..] else {
        panic!("{settlements:?}");
    };
    assert_eq!(
        (
            village.plots_min,
            village.plots,
            village.radius_min_m,
            village.radius_m
        ),
        (6, 6, 32.0, 32.0),
        "a single value is a fixed size"
    );
    assert_eq!(
        (town.plots_min, town.plots, town.radius_min_m, town.radius_m),
        (30, 96, 40.0, 160.0)
    );
}

#[test]
fn a_range_upside_down_or_too_small_for_its_minimums_or_buildings_is_refused() {
    let ranged = |fields: &str| {
        settlement(&format!(
            "centre = \"t:well\"\n{fields}\n\
             templates = [{{ template = \"t:house\", weight = 2.0, min = 1, max = 4 }}]"
        ))
    };
    for (fields, says) in [
        ("radius-m = 32.0\nplots-min = 7\nplots = 6", "plots-min"),
        (
            "radius-min-m = 40.0\nradius-m = 32.0\nplots = 6",
            "radius-min-m",
        ),
        (
            "radius-min-m = 3.0\nradius-m = 32.0\nplots = 6",
            "smallest radius",
        ),
        ("radius-m = 32.0\nplots-min = 0\nplots = 6", "smallest copy"),
        ("radius-m = 32.0\nplots = 6\nplots-max = 6", "duplicate"),
    ] {
        let refused = refusal(&ranged(fields), "");
        assert!(refused.contains(says), "{fields}: {refused}");
    }
}

#[test]
fn the_spacing_rule_takes_the_largest_radius() {
    let town = "radius-min-m = 20.0\nradius-max-m = 100.0\ncentre = \"t:well\"\n\
                plots-min = 3\nplots-max = 40\n\
                templates = [{ template = \"t:house\", weight = 2.0, min = 1, max = 40 }]";
    let close = refusal(
        &settlement(town),
        "{ settlement = \"t:village\", spacing-m = 200.0 }",
    );
    assert!(close.contains("twice"), "{close}");
    load(&world(
        &settlement(town),
        "{ settlement = \"t:village\", spacing-m = 232.0 }",
    ))
    .unwrap();
}

#[test]
fn a_biome_row_carries_its_chance() {
    let (_, biomes) = load(&world(
        &settlement(VILLAGE),
        "{ settlement = \"t:village\", spacing-m = 120.0, chance = 0.4 }",
    ))
    .unwrap();
    assert_eq!(
        biomes.as_slice()[0].surface_features.settlements()[0].chance,
        0.4
    );
}

fn refusal(settlements: &str, rows: &str) -> String {
    load(&world(settlements, rows)).unwrap_err().to_string()
}

#[test]
fn a_third_row_a_wide_radius_too_many_plots_an_unknown_or_spanning_template_names_the_settlement() {
    let row = "{ settlement = \"t:village\", spacing-m = 120.0 }";
    let third = refusal(&settlement(VILLAGE), &[row; 3].join(", "));
    assert!(
        third.contains("t:village") && third.contains("at most 2"),
        "{third}"
    );
    let wide = refusal(&settlement(&VILLAGE.replace("32.0", "160.5")), "");
    assert!(
        wide.contains("`village`") && wide.contains("radius"),
        "{wide}"
    );
    let many = refusal(&settlement(&VILLAGE.replace("plots = 6", "plots = 97")), "");
    assert!(many.contains("`village`") && many.contains("96"), "{many}");
    let unknown = refusal(&settlement(&VILLAGE.replace("t:house", "t:castle")), "");
    assert!(
        unknown.contains("`village`") && unknown.contains("t:castle"),
        "{unknown}"
    );
    let bridge = "[[templates]]\nname = \"bridge\"\ncell-m = 1.0\nground-layer = 0\n\
                  spans = true\nlayers = [[\"pppppp\"]]\n\n[templates.palette]\n\
                  \"p\" = { material = \"t:planks\" }\n";
    let spanning = refusal(
        &format!(
            "{bridge}{}",
            settlement(&VILLAGE.replace("t:house", "t:bridge"))
        ),
        "",
    );
    assert!(
        spanning.contains("`village`") && spanning.contains("spans a gap"),
        "{spanning}"
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
    for chance in ["0.0", "1.5"] {
        let odds = refusal(
            &settlement(VILLAGE),
            &format!("{{ settlement = \"t:village\", spacing-m = 120.0, chance = {chance} }}"),
        );
        assert!(
            odds.contains("t:village") && odds.contains("chance"),
            "{odds}"
        );
    }
}

/// Settlements and each Biome's loot, every Item checked as a World's load checks it once every
/// Item exists.
fn stocked(toml: &str) -> Result<(Vec<Settlement>, Vec<Option<LootTable>>), ModError> {
    let (mut tables, mut registry) = (Tables::builtin(), Registry::builtin());
    let mut behaviours: Vec<Behaviour> =
        vec![apply(data(toml)?, "t", &mut tables, &mut registry, true)?.behaviour];
    let items = resolve_items(&behaviours, &tables, &registry)?;
    let rows = biome_rows(&behaviours);
    let templates = DeclaredTemplates::take(&mut behaviours);
    let settlements = DeclaredSettlements::take(&mut behaviours);
    resolve(behaviours, &mut tables, &registry)?;
    tables.templates = templates.resolve(&registry, &tables)?;
    tables.settlements = settlements.resolve(&registry, &tables)?;
    settlements.check_loot(&items)?;
    Ok((tables.settlements, biome_loot(&rows, &items)?))
}

const ITEMS: &str = "[[items]]\nname = \"bread\"\nsize-m = 0.25\ncolor = [200, 160, 90, 255]\n\
                     max-stack = 8\n\n[[items]]\nname = \"nail\"\nsize-m = 0.1\n\
                     color = [90, 90, 90, 255]\n";

/// A Settlement whose house has its own loot, and a Biome with loot of its own.
fn stocked_world(settlement_loot: &str, house_loot: &str, biome_loot: &str) -> String {
    let village = VILLAGE.replace(
        "max = 4 }",
        &format!("max = 4, loot = {{ entries = [{house_loot}] }} }}"),
    );
    format!(
        "{CONTENT}\n{ITEMS}\n{}\n[settlements.loot]\nrolls-min = 2\nrolls-max = 5\n\
         entries = [{settlement_loot}]\n\n[[biomes]]\nname = \"plain\"\ntemperature = 0.5\n\
         humidity = 0.5\n\n[biomes.loot]\nentries = [{biome_loot}]\n",
        settlement(&village)
    )
}

#[test]
fn a_settlement_its_buildings_and_a_biome_carry_their_loot() {
    let (settlements, biomes) = stocked(&stocked_world(
        "{ item = \"t:bread\", min = 1, max = 3, weight = 2.0 }, { item = \"t:nail\" }",
        "{ item = \"t:nail\", min = 4, max = 9 }",
        "{ item = \"t:bread\" }",
    ))
    .unwrap();
    let entry = |item: &str, min, max, weight| LootEntry {
        item: item.into(),
        min,
        max,
        weight,
    };
    let village = &settlements[0];
    assert_eq!(
        village.loot,
        Some(LootTable {
            rolls_min: 2,
            rolls_max: 5,
            entries: vec![entry("t:bread", 1, 3, 2.0), entry("t:nail", 1, 1, 1.0)],
        })
    );
    assert_eq!(
        village.rules[0].loot,
        Some(LootTable {
            rolls_min: 1,
            rolls_max: 1,
            entries: vec![entry("t:nail", 4, 9, 1.0)],
        }),
        "one roll, and a count of min, unless it says"
    );
    assert_eq!(
        biomes,
        [Some(LootTable {
            rolls_min: 1,
            rolls_max: 1,
            entries: vec![entry("t:bread", 1, 1, 1.0)],
        })]
    );
}

#[test]
fn loot_naming_an_unknown_item_more_than_its_stack_or_out_of_range_is_refused() {
    let refused =
        |s: &str, h: &str, b: &str| stocked(&stocked_world(s, h, b)).unwrap_err().to_string();
    let ok = "{ item = \"t:nail\" }";
    let unknown = refused("{ item = \"t:cake\" }", ok, ok);
    assert!(
        unknown.contains("`village`") && unknown.contains("`t:cake` is no Item"),
        "{unknown}"
    );
    let house = refused(ok, "{ item = \"t:cake\" }", ok);
    assert!(
        house.contains("`village`") && house.contains("t:house") && house.contains("t:cake"),
        "{house}"
    );
    let biome = refused(ok, ok, "{ item = \"t:cake\" }");
    assert!(
        biome.contains("biome `plain` loot") && biome.contains("t:cake"),
        "{biome}"
    );
    let stack = refused("{ item = \"t:bread\", max = 9 }", ok, ok);
    assert!(stack.contains("stack of 8"), "{stack}");
    let empty = refused(ok, "{ item = \"t:nail\", min = 0 }", ok);
    assert!(empty.contains("min must be at least 1"), "{empty}");
}

#[test]
fn a_settlement_lists_outskirts_and_one_naming_a_missing_template_is_refused() {
    let farmed = format!(
        "{VILLAGE}\nmargin-m = 20.0\n\
         outskirts = {{ count-min = 2, count-max = 4, templates = [{{ template = \"t:well\" }}] }}"
    );
    let (settlements, _) = load(&world(
        &settlement(&farmed),
        "{ settlement = \"t:village\", spacing-m = 120.0 }",
    ))
    .unwrap();
    let outskirts = &settlements[0].outskirts;
    assert_eq!(outskirts.count, (2, 4));
    assert_eq!(outskirts.rules[0].building.template, 0);
    let (plain, _) = load(&world(
        &settlement(VILLAGE),
        "{ settlement = \"t:village\", spacing-m = 120.0 }",
    ))
    .unwrap();
    assert!(plain[0].outskirts.rules.is_empty(), "none unless it says");
    let missing = farmed.replace("t:well\" }]", "t:barn\" }]");
    let err = load(&world(
        &settlement(&missing),
        "{ settlement = \"t:village\", spacing-m = 120.0 }",
    ))
    .unwrap_err()
    .to_string();
    assert!(err.contains("village") && err.contains("t:barn"), "{err}");
}

#[test]
fn outskirts_read_a_chance_and_refuse_one_outside_zero_to_one() {
    let farmed = |chance: &str| {
        format!(
            "{VILLAGE}\nmargin-m = 20.0\n\
             outskirts = {{ count-min = 2, count-max = 4, chance = {chance}, templates = [{{ template = \"t:well\" }}] }}"
        )
    };
    let listed = "{ settlement = \"t:village\", spacing-m = 120.0 }";
    let (settlements, _) = load(&world(&settlement(&farmed("0.4")), listed)).unwrap();
    assert_eq!(settlements[0].outskirts.chance, 0.4);
    for wrong in ["0.0", "1.5"] {
        let err = load(&world(&settlement(&farmed(wrong)), listed))
            .unwrap_err()
            .to_string();
        assert!(err.contains("chance"), "{wrong}: {err}");
    }
}
