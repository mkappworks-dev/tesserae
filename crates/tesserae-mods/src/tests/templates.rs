//! A Template declared as data or by a component loads through one validator,
//! and the World samples and turns it in metres.

use tesserae_core::Tables;
use tesserae_core::mesh::{FACE_STEPS, NEG_X, NEG_Y, NEG_Z, POS_X, POS_Y, POS_Z};
use tesserae_core::shape::Orientation;
use tesserae_core::template::{Cell, Template, Turn};

use super::behaviour::{empty, fluid, material};
use crate::bindings::tesserae::mods::content::{self, Declaration};
use crate::{Behaviour, DeclaredTemplates, Kind, ModError, Registry, apply, resolve};

/// Declares each `(mod id, declaration)` in order, the first as the Game, and
/// resolves their Templates.
fn load(mods: Vec<(&str, Declaration)>) -> Result<(Registry, Vec<Template>), ModError> {
    let (mut tables, mut registry) = (Tables::builtin(), Registry::builtin());
    let mut behaviours: Vec<Behaviour> = Vec::new();
    for (at, (id, d)) in mods.into_iter().enumerate() {
        behaviours.push(apply(d, id, &mut tables, &mut registry, at == 0)?.behaviour);
    }
    let declared = DeclaredTemplates::take(&mut behaviours);
    resolve(behaviours, &mut tables, &registry)?;
    let templates = declared.resolve(&registry, &tables)?;
    Ok((registry, templates))
}

fn data(toml: &str) -> Result<Declaration, ModError> {
    let payload =
        crate::trust::CheckedPayload::for_test("content.toml", [("content.toml", toml.into())]);
    crate::data::parse("t", &payload)
}

const MATERIALS: &str = r#"
[[materials]]
name = "stone"
emission = 0
dimming = 15
occlusion = "opaque"
render = "opaque"
solid = true

[[materials]]
name = "planks"
emission = 0
dimming = 15
occlusion = "opaque"
render = "opaque"
solid = true

[[materials]]
name = "water"
emission = 0
dimming = 2
occlusion = "same-material"
render = "blend"
solid = false

[[fluids]]
material = "t:water"
reach-m = 7.0
sideways-m-per-s = 4.0
downward-m-per-s = 8.0
recession-m-per-s = 4.0
reset-drop-m = 0.5
waterlogs = true
extinguishes = true
"#;

/// A 4 by 3 m hut of 1 m cells, and a 3 by 1 m wall of 0.5 m cells.
const HUT: &str = r#"
[[templates]]
name = "hut"
cell-m = 1.0
ground-layer = 1
foundation = "t:stone"
level-tolerance-m = 0.5
layers = [
  ["wwww", "wwww", "wwww"],
  ["ws.w", "w__w", "wwww"],
]

[templates.palette]
"w" = { material = "t:planks" }
"s" = { material = "t:planks", shape = "core:stair", orientation = 1 }
"." = "air"
"_" = "keep"

[[templates]]
name = "wall"
cell-m = 0.5
ground-layer = 0
layers = [["xxxxxx", "xxxxxx"], ["xxxxxx", "xxxxxx"]]

[templates.palette]
"x" = { material = "t:stone" }
"#;

fn hut() -> Result<(Registry, Vec<Template>), ModError> {
    load(vec![("t", data(&format!("{MATERIALS}{HUT}"))?)])
}

#[test]
fn a_mod_with_a_template_loads_and_samples_the_same_metres_at_every_voxel_size() {
    let (registry, templates) = hut().unwrap();
    let [hut, wall] = &templates[..] else {
        panic!("two Templates");
    };
    assert_eq!(hut.name, "t:hut");
    let stone = registry.id_of(Kind::Material, "t:stone").unwrap() as u16;
    assert_eq!(hut.foundation(), Some(stone));
    assert_eq!(hut.level_tolerance_m(), 0.5);
    assert_eq!(wall.level_tolerance_m(), 1.0, "the default");
    for voxel_size in [1.0, 0.5, 0.25] {
        let metres = |t: &Template| {
            t.sample(voxel_size, Turn::NONE)
                .size()
                .map(|n| n as f32 * voxel_size)
        };
        assert_eq!(metres(hut), [4.0, 2.0, 3.0], "at {voxel_size} m");
        assert_eq!(metres(wall), [3.0, 1.0, 1.0], "at {voxel_size} m");
        let sampled = hut.sample(voxel_size, Turn::NONE);
        assert_eq!(
            sampled.ground() as f32 * voxel_size,
            1.0,
            "at {voxel_size} m"
        );
        // The Voxel starting 2.5 m east and 1.5 m up is in the air cell.
        let at = [2.5 / voxel_size, 1.5 / voxel_size, 0.0].map(|m| m as i32);
        assert_eq!(sampled.cell(at), Cell::Air, "at {voxel_size} m");
    }
}

#[test]
fn a_keep_cell_and_an_air_cell_are_different_cells() {
    let (_, templates) = hut().unwrap();
    let sampled = templates[0].sample(1.0, Turn::NONE);
    assert_eq!(sampled.cell([2, 1, 0]), Cell::Air);
    assert_eq!(sampled.cell([1, 1, 1]), Cell::Keep);
    assert_ne!(Cell::Air, Cell::Keep);
    assert!(matches!(sampled.cell([0, 0, 0]), Cell::Voxel(v) if !v.is_air()));
}

#[test]
fn a_ground_cell_loads_from_data_and_a_component() {
    let toml = format!(
        "{MATERIALS}
[[templates]]
name = \"plot\"
cell-m = 1.0
ground-layer = 1
layers = [[\"gg\"], [\"g#\"]]

[templates.palette]
\"g\" = \"ground\"
\"#\" = {{ material = \"t:stone\" }}
"
    );
    let (_, from_data) = load(vec![("t", data(&toml).unwrap())]).unwrap();
    let mut d = component();
    d.templates[0].palette.push(content::TemplatePaletteEntry {
        symbol: 'g',
        cell: content::TemplateCell::Ground,
    });
    d.templates[0].layers = vec![vec!["gg".into()], vec!["g#".into()]];
    d.templates[0].ground_layer = Some(1);
    let (_, from_component) = load(vec![("t", d)]).unwrap();
    for templates in [from_data, from_component] {
        let sampled = templates[0].sample(1.0, Turn::NONE);
        assert_eq!(sampled.cell([0, 0, 0]), Cell::Ground);
        assert_eq!(sampled.cell([0, 1, 0]), Cell::Ground);
        assert!(matches!(sampled.cell([1, 1, 0]), Cell::Voxel(_)));
    }
}

/// Where a face points after `quarters` quarter-turns about +Y, which takes +Z
/// to +X and +X to -Z.
fn yawed_face(face: usize, quarters: u8) -> usize {
    (0..quarters).fold(face, |f, _| match f {
        POS_Z => POS_X,
        POS_X => NEG_Z,
        NEG_Z => NEG_X,
        NEG_X => POS_Z,
        up_or_down => up_or_down,
    })
}

/// One column per built-in Shape, its 24 layers in the 24 orientations.
fn every_orientation() -> Declaration {
    let symbol = |o: u8| char::from(b'a' + o);
    let templates = ["core:cube", "core:slab", "core:stair", "core:post"]
        .iter()
        .enumerate()
        .map(|(n, shape)| content::TemplateDef {
            name: format!("shape{n}"),
            cell_m: 1.0,
            palette: Orientation::all()
                .map(|o| content::TemplatePaletteEntry {
                    symbol: symbol(o.bits()),
                    cell: content::TemplateCell::Voxel(content::TemplateVoxel {
                        material: "t:stone".into(),
                        shape: Some((*shape).into()),
                        orientation: Some(o.bits()),
                    }),
                })
                .collect(),
            layers: Orientation::all()
                .map(|o| vec![symbol(o.bits()).to_string()])
                .collect(),
            ground_layer: Some(0),
            foundation: None,
            level_tolerance_m: None,
        })
        .collect();
    Declaration {
        materials: vec![material("stone", true)],
        templates,
        ..empty()
    }
}

#[test]
fn every_turn_turns_every_shapes_orientation_with_the_structure() {
    let (_, templates) = load(vec![("t", every_orientation())]).unwrap();
    assert_eq!(templates.len(), 4);
    for template in &templates {
        for turn in Turn::all() {
            let sampled = template.sample(1.0, turn);
            for o in Orientation::all() {
                let Cell::Voxel(v) = sampled.cell([0, o.bits() as i32, 0]) else {
                    panic!("a Voxel");
                };
                let turned = Orientation::from_bits(v.orientation()).unwrap();
                for face in [POS_X, NEG_X, POS_Y, NEG_Y, POS_Z, NEG_Z] {
                    assert_eq!(
                        turned.face_of(face),
                        yawed_face(o.face_of(face), turn.quarters()),
                        "{} turned {} from {o:?}",
                        template.name,
                        turn.quarters()
                    );
                }
            }
        }
    }
}

#[test]
fn four_quarter_turns_are_the_identity() {
    assert_eq!(Turn::NONE.next().next().next().next(), Turn::NONE);
    assert_eq!(Turn::new(4), Turn::NONE);
    for o in Orientation::all() {
        assert_eq!(o.yawed(1).yawed(1).yawed(1).yawed(1), o);
    }
}

#[test]
fn a_turn_moves_cells_the_way_it_turns_their_faces() {
    // 3 wide and 2 deep, marked at the +X, -Z corner by a stair facing +X.
    let d = data(&format!(
        "{MATERIALS}{}",
        r#"
[[templates]]
name = "corner"
cell-m = 1.0
ground-layer = 0
layers = [["..s", "..."]]

[templates.palette]
"." = "air"
"s" = { material = "t:planks", shape = "core:stair" }
"#
    ))
    .unwrap();
    let (_, templates) = load(vec![("t", d)]).unwrap();
    // The corner the mark ends at, and the turned size, from +X and -Z turned by hand.
    for (quarters, size, corner) in [
        (0, [3, 1, 2], [2, 0, 0]),
        (1, [2, 1, 3], [0, 0, 0]),
        (2, [3, 1, 2], [0, 0, 1]),
        (3, [2, 1, 3], [1, 0, 2]),
    ] {
        let sampled = templates[0].sample(1.0, Turn::new(quarters));
        assert_eq!(sampled.size(), size, "turned {quarters}");
        let Cell::Voxel(v) = sampled.cell(corner) else {
            panic!("the mark at {corner:?} turned {quarters}");
        };
        let o = Orientation::from_bits(v.orientation()).unwrap();
        let faces = o.face_of(POS_X);
        assert_eq!(faces, yawed_face(POS_X, quarters));
        // The mark sits on the edge it faces.
        let step = FACE_STEPS[faces];
        for axis in [0, 2] {
            match step[axis] {
                1 => assert_eq!(corner[axis], size[axis] - 1),
                -1 => assert_eq!(corner[axis], 0),
                _ => {}
            }
        }
    }
}

/// A valid one-Template Declaration as a component returns it.
fn component() -> Declaration {
    Declaration {
        materials: vec![material("stone", true), material("water", false)],
        fluids: vec![fluid("t:water")],
        templates: vec![content::TemplateDef {
            name: "hut".into(),
            cell_m: 1.0,
            palette: vec![content::TemplatePaletteEntry {
                symbol: '#',
                cell: content::TemplateCell::Voxel(content::TemplateVoxel {
                    material: "t:stone".into(),
                    shape: None,
                    orientation: None,
                }),
            }],
            layers: vec![vec!["##".into(), "##".into()]],
            ground_layer: Some(0),
            foundation: None,
            level_tolerance_m: None,
        }],
        ..empty()
    }
}

/// The same Template as data, with `extra` lines in its row.
fn data_hut(layers: &str, extra: &str) -> String {
    format!(
        "{MATERIALS}
[[templates]]
name = \"hut\"
cell-m = 1.0
layers = {layers}
{extra}

[templates.palette]
\"#\" = {{ material = \"t:stone\" }}
"
    )
}

fn grid(rows: usize, width: usize, layers: usize) -> Vec<Vec<String>> {
    vec![vec!["#".repeat(width); rows]; layers]
}

fn toml_layers(layers: &[Vec<String>]) -> String {
    format!("{layers:?}")
}

/// Each case refuses the same way from a component and from data, naming the
/// Template and what is wrong.
#[test]
fn a_bad_template_fails_the_load_naming_it_from_data_and_a_component() {
    type Edit = fn(&mut content::TemplateDef);
    let cases: [(&str, Edit, String, &[&str]); 7] = [
        (
            "unknown Material",
            |t| t.palette[0].cell = voxel("t:marble", None),
            data_hut("[[\"#\"]]", "ground-layer = 0").replace("t:stone\" }", "t:marble\" }"),
            &["palette `#`", "t:marble"],
        ),
        (
            "unknown Shape",
            |t| t.palette[0].cell = voxel("t:stone", Some("t:arch")),
            data_hut("[[\"#\"]]", "ground-layer = 0")
                .replace("t:stone\" }", "t:stone\", shape = \"t:arch\" }"),
            &["palette `#`", "t:arch"],
        ),
        (
            "a fluid",
            |t| t.palette[0].cell = voxel("t:water", None),
            data_hut("[[\"#\"]]", "ground-layer = 0").replace("t:stone\" }", "t:water\" }"),
            &["palette `#`", "t:water"],
        ),
        (
            "a cell not in the palette",
            |t| t.layers = vec![vec!["#?".into()]],
            data_hut("[[\"#?\"]]", "ground-layer = 0"),
            &["layer 0, row 0, column 1", "`?`"],
        ),
        (
            "over 64 m a side",
            |t| t.layers = grid(1, 65, 1),
            data_hut(&toml_layers(&grid(1, 65, 1)), "ground-layer = 0"),
            &["65 m along x", "64"],
        ),
        (
            "over 65,536 cells",
            |t| t.layers = grid(41, 41, 41),
            data_hut(&toml_layers(&grid(41, 41, 41)), "ground-layer = 0"),
            &["68921 cells", "65536"],
        ),
        (
            "no Ground layer",
            |t| t.ground_layer = None,
            data_hut("[[\"#\"]]", ""),
            &["ground-layer"],
        ),
    ];
    for (what, edit, toml, expected) in cases {
        let mut native = component();
        edit(&mut native.templates[0]);
        for (source, declaration) in [("component", native), ("data", data(&toml).unwrap())] {
            let err = load(vec![("t", declaration)]).unwrap_err().to_string();
            assert!(
                err.contains("template 0 `hut`") && expected.iter().all(|e| err.contains(e)),
                "{what} from {source}: {err}"
            );
        }
    }
}

#[test]
fn a_ground_layer_above_the_top_fails_the_load_naming_the_template() {
    let mut native = component();
    native.templates[0].ground_layer = Some(1);
    let err = load(vec![("t", native)]).unwrap_err().to_string();
    assert!(
        err.contains("`hut`") && err.contains("ground-layer 1"),
        "{err}"
    );
}

fn voxel(material: &str, shape: Option<&str>) -> content::TemplateCell {
    content::TemplateCell::Voxel(content::TemplateVoxel {
        material: material.into(),
        shape: shape.map(Into::into),
        orientation: None,
    })
}

#[test]
fn a_65th_template_in_a_world_fails_the_load_naming_it() {
    let many = |prefix: &str, count: usize| {
        let mut d = component();
        let row = d.templates.remove(0);
        d.templates = (0..count)
            .map(|n| content::TemplateDef {
                name: format!("{prefix}{n}"),
                ..row.clone()
            })
            .collect();
        d
    };
    let (_, templates) = load(vec![("t", many("a", 64))]).unwrap();
    assert_eq!(templates.len(), 64);
    let mut other = many("b", 1);
    other.materials.clear();
    other.fluids.clear();
    let err = load(vec![("t", many("a", 64)), ("o", other)])
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("mod `o`") && err.contains("`b0`") && err.contains("64"),
        "{err}"
    );
}

#[test]
fn templates_are_named_by_mod_and_any_mod_may_declare_one() {
    let mut other = component();
    other.materials.clear();
    other.fluids.clear();
    other.templates[0].name = "well".into();
    let (_, templates) = load(vec![("t", component()), ("o", other)]).unwrap();
    let names: Vec<_> = templates.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ["t:hut", "o:well"]);
}

#[test]
fn a_declaration_with_no_template_loads_with_none() {
    let (_, templates) = load(vec![("t", empty())]).unwrap();
    assert!(templates.is_empty());
    assert!(Tables::builtin().templates.is_empty());
}

#[test]
fn the_0_1_and_0_2_adapters_give_no_templates() {
    use crate::bindings::{v01, v02};
    let old = v01::tesserae::mods::content::Declaration {
        materials: vec![],
        shapes: vec![],
        appearance: vec![],
        world: None,
        view: None,
    };
    let log = crate::log::LogBuffer::shared();
    let through_v01: v02::tesserae::mods::content::Declaration = old.into();
    let declaration = crate::adapt::v02::declaration(through_v01, &log);
    assert!(declaration.templates.is_empty());
    let old = v02::tesserae::mods::content::Declaration {
        materials: vec![],
        shapes: vec![],
        appearance: vec![],
        world: None,
        view: None,
        fluids: vec![],
        reactions: vec![],
        combustion: vec![],
    };
    assert!(
        crate::adapt::v02::declaration(old, &log)
            .templates
            .is_empty()
    );
}
