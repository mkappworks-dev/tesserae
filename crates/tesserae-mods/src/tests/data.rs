//! The TOML Declaration decodes to the same content a component declares, and
//! the shared validation refuses what it refuses there.

use tesserae_core::Tables;

use crate::bindings::tesserae::mods::content::{self, Declaration};
use crate::bindings::tesserae::mods::types;
use crate::{Kind, ModError, Registry, apply, resolve};

const FULL: &str = include_str!("../../tests/fixtures/data/full.toml");

fn skin_toml() -> String {
    format!(
        "[world.character]\nskin = \"skin.png\"\n{}",
        include_str!("../../tests/fixtures/data/character.toml")
    )
}

fn full() -> String {
    format!("{FULL}\n{}", skin_toml())
}

fn blank_skin() -> Vec<u8> {
    let edge = crate::SKIN_EDGE as u32;
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, edge, edge);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&vec![0; crate::SKIN_BYTES])
        .unwrap();
    out
}

fn parse(toml: &str) -> Result<Declaration, ModError> {
    let payload = crate::trust::CheckedPayload::for_test(
        "content.toml",
        [
            ("content.toml", toml.as_bytes().to_vec()),
            ("skin.png", blank_skin()),
        ],
    );
    crate::data::parse("full", &payload)
}

fn material(
    name: &str,
    dimming: u8,
    occlusion: content::Occlusion,
    render: content::RenderClass,
    solid: bool,
    emission: u8,
    remove: Option<f32>,
) -> content::MaterialDef {
    content::MaterialDef {
        name: name.into(),
        emission,
        dimming,
        occlusion,
        render,
        solid,
        remove_time_s: remove,
    }
}

fn node(min: (u8, u8, u8), max: (u8, u8, u8)) -> types::NodeBox {
    types::NodeBox { min, max }
}

fn motion(swing_deg: f32, rate_hz: f32, arm_raise_deg: f32) -> content::LimbMotion {
    content::LimbMotion {
        swing_deg,
        rate_hz,
        arm_raise_deg,
    }
}

/// What a component Game would return for [`FULL`] and the character.
fn component() -> Declaration {
    use content::{Occlusion as O, RenderClass as R};
    let still = motion(0.0, 0.0, 0.0);
    let cube = |origin, size, uv| content::CharacterCube { origin, size, uv };
    let heat = |strength, reach_m| Some(content::HeatDef { strength, reach_m });
    let any = |fluid: &str, state| content::Reactant {
        fluid: fluid.into(),
        state,
    };
    Declaration {
        materials: vec![
            material("rock", 15, O::Opaque, R::Opaque, true, 0, None),
            material("water", 2, O::SameMaterial, R::Blend, false, 0, Some(0.5)),
            material("lava", 2, O::Never, R::Blend, false, 12, None),
            material("wood", 15, O::SameMaterial, R::Cutout, true, 0, None),
            material("ash", 15, O::Opaque, R::Opaque, true, 0, None),
        ],
        shapes: vec![
            content::ShapeDef {
                name: "slab".into(),
                boxes: vec![node((0, 0, 0), (16, 8, 16))],
                collision: Some(vec![types::CollisionBox {
                    min: (0, 0, 0),
                    max: (16, 12, 16),
                }]),
                connect: None,
            },
            content::ShapeDef {
                name: "fence".into(),
                boxes: vec![node((6, 0, 6), (10, 16, 10))],
                collision: None,
                connect: Some(types::ConnectDef {
                    group: 1,
                    connects_to: 3,
                    to_sealed: true,
                    arms: vec![
                        types::Arm {
                            face: 1,
                            boxes: vec![node((10, 4, 7), (16, 14, 9))],
                            collision: Some(vec![types::CollisionBox {
                                min: (10, -4, 6),
                                max: (32, 24, 10),
                            }]),
                        },
                        types::Arm {
                            face: 0,
                            boxes: vec![node((0, 4, 7), (6, 14, 9))],
                            collision: None,
                        },
                    ],
                }),
            },
        ],
        appearance: vec![
            content::Appearance {
                material: "rock".into(),
                color: types::Srgba {
                    r: 80,
                    g: 90,
                    b: 100,
                    a: 255,
                },
                texture: None,
            },
            content::Appearance {
                material: "water".into(),
                color: types::Srgba {
                    r: 30,
                    g: 80,
                    b: 200,
                    a: 128,
                },
                texture: None,
            },
        ],
        world: Some(content::WorldParams {
            movement: Some(content::MovementPolicy {
                allow_flight: true,
                allow_physics: true,
                start_in_flight: false,
            }),
            character: Some(content::CharacterDef {
                bones: vec![
                    content::CharacterBone {
                        name: "body".into(),
                        parent: None,
                        role: content::BoneRole::Body,
                        pivot: (0.0, 12.0, 0.0),
                        cubes: vec![cube((-4.0, 12.0, -2.0), (8, 12, 4), (16, 16))],
                    },
                    content::CharacterBone {
                        name: "head".into(),
                        parent: Some("body".into()),
                        role: content::BoneRole::Head,
                        pivot: (0.0, 24.0, 0.0),
                        cubes: vec![cube((-4.0, 24.0, -4.0), (8, 8, 8), (0, 0))],
                    },
                ],
                pixel_m: 0.05625,
                skin: vec![0; crate::SKIN_BYTES],
                idle: still,
                walk: motion(45.0, 2.0, 0.0),
                sprint: motion(60.0, 3.0, 10.0),
                airborne: motion(10.0, 0.0, -20.0),
                crouch: motion(20.0, 1.0, 0.0),
            }),
            sea_level_m: 20.0,
            relief_m: 12.0,
            feature_size_m: 48.0,
            soil_depth_m: 3.0,
            octaves: 4,
            caves: Some(content::CaveParams {
                feature_size_m: 24.0,
                radius_m: 3.0,
                floor_m: 4.0,
                octaves: 2,
            }),
            smoothing: Some(content::SmoothingParams { min_rise_m: 0.4 }),
            water: Some(content::WaterParams {
                level_m: 24.0,
                material: "full:water".into(),
            }),
            ground: Some(content::GroundParams {
                rock: "full:rock".into(),
                soil: "full:rock".into(),
                surface: "full:rock".into(),
            }),
        }),
        view: Some(content::ViewParams {
            view_distance_m: 160.0,
            lod_distance_m: 3000.0,
        }),
        fluids: vec![
            content::FluidDef {
                material: "full:water".into(),
                reach_m: 7.0,
                sideways_m_per_s: 4.0,
                downward_m_per_s: 8.0,
                recession_m_per_s: 4.0,
                reset_drop_m: 0.5,
                waterlogs: true,
                extinguishes: true,
                heat: None,
            },
            content::FluidDef {
                material: "full:lava".into(),
                reach_m: 3.0,
                sideways_m_per_s: 0.75,
                downward_m_per_s: 1.5,
                recession_m_per_s: 0.75,
                reset_drop_m: 0.5,
                waterlogs: false,
                extinguishes: false,
                heat: heat(12, 2.0),
            },
        ],
        reactions: vec![content::ReactionDef {
            first: any("full:water", content::FluidState::Any),
            second: any("full:lava", content::FluidState::Source),
            consumed: content::ReactionSide::Second,
            product: Some("full:rock".into()),
            receiving: Some(content::ReactionSide::First),
        }],
        combustion: vec![
            content::CombustionDef {
                fuel: "full:wood".into(),
                ignition_threshold: 6,
                ignition_delay_s: 2.0,
                emission: 12,
                heat: heat(10, 2.0),
                end: content::BurnEnd::After(content::BurnAfter {
                    seconds: 8.0,
                    residue: Some("full:ash".into()),
                }),
            },
            content::CombustionDef {
                fuel: "full:ash".into(),
                ignition_threshold: 15,
                ignition_delay_s: 600.0,
                emission: 0,
                heat: None,
                end: content::BurnEnd::Permanent,
            },
        ],
        tools: vec![content::ToolDef {
            name: "ruler".into(),
            kind: content::ToolKind::Measure,
        }],
    }
}

fn load(declaration: Declaration) -> (Tables, Registry, crate::Applied) {
    let mut tables = Tables::builtin();
    let mut registry = Registry::builtin();
    let applied = apply(declaration, "full", &mut tables, &mut registry, true).unwrap();
    resolve(vec![applied.behaviour.clone()], &mut tables, &registry).unwrap();
    (tables, registry, applied)
}

#[test]
fn data_decodes_to_the_declaration_a_component_returns() {
    assert_eq!(parse(&full()).unwrap(), component());
}

#[test]
fn equivalent_declarations_register_equivalent_content_and_simulation() {
    let (data_tables, data_registry, data) = load(parse(&full()).unwrap());
    let (tables, registry, native) = load(component());
    assert_eq!(data_registry, registry);
    assert_eq!(data.tools, native.tools);
    assert_eq!(data.character, native.character);
    assert_eq!(data.movement, native.movement);
    assert_eq!(data.view, native.view);
    assert_eq!(data.world, native.world);
    let id = |name| registry.id_of(Kind::Material, name).unwrap() as u16;
    let (water, lava, wood, ash) = (
        id("full:water"),
        id("full:lava"),
        id("full:wood"),
        id("full:ash"),
    );
    let (a, b) = (&data_tables, &tables);
    assert_eq!(a.materials.len(), b.materials.len());
    assert_eq!(a.shapes.len(), b.shapes.len());
    assert_eq!(a.fluids.get(water), b.fluids.get(water));
    assert_eq!(a.fluids.get(lava), b.fluids.get(lava));
    assert_eq!(a.combustion.get(wood), b.combustion.get(wood));
    assert_eq!(a.combustion.get(ash), b.combustion.get(ash));
    assert!(tables.fluids.get(lava).unwrap().heat.is_some());
    assert!(tables.fluids.get(water).unwrap().extinguishes);
    assert!(tables.combustion.get(wood).is_some());
    assert_eq!(data_tables.reactions.rules(), tables.reactions.rules());
    let rule = &data_tables.reactions.rules()[0];
    assert_eq!(rule.product, Some(id("full:rock")));
}

#[test]
fn absent_optional_values_stay_absent_rather_than_defaulting() {
    let d = parse("[[materials]]\nname='a'\nemission=0\ndimming=0\nocclusion='never'\nrender='opaque'\nsolid=true\n[world]").unwrap();
    let world = d.world.unwrap();
    assert!(world.caves.is_none() && world.smoothing.is_none() && world.water.is_none());
    assert!(world.movement.is_none() && world.character.is_none() && world.ground.is_none());
    assert!(d.view.is_none() && d.fluids.is_empty() && d.tools.is_empty());
    assert_eq!(d.materials[0].remove_time_s, None);
}

fn refused(replace: &str, with: &str, field: &str) {
    let text = full();
    assert!(text.contains(replace), "{replace}");
    let error = match parse(&text.replacen(replace, with, 1)) {
        Err(e) => e,
        Ok(d) => {
            let mut tables = Tables::builtin();
            let mut registry = Registry::builtin();
            match apply(d, "full", &mut tables, &mut registry, true) {
                Err(e) => e,
                Ok(applied) => resolve(vec![applied.behaviour.clone()], &mut tables, &registry)
                    .expect_err(with),
            }
        }
    };
    let error = error.to_string().to_lowercase();
    assert!(
        error.contains("full") && error.contains(field),
        "{with}: {error}"
    );
}

#[test]
fn unknown_fields_variants_and_bad_optionals_name_the_mod_and_field() {
    for (replace, with, field) in [
        ("kind = \"measure\"", "kind = \"dig\"", "dig"),
        ("name = \"ruler\"", "name = \"ruler\"\nextra = 1", "extra"),
        ("state = \"any\"", "state = \"sideways\"", "sideways"),
        ("end = \"permanent\"", "end = \"never\"", "never"),
        (
            "end = \"permanent\"",
            "end = { after = { seconds = 1, soon = true } }",
            "soon",
        ),
        ("receiving = \"first\"", "receiving = \"third\"", "third"),
        (
            "remove-time-s = 0.5",
            "remove-time-s = \"slow\"",
            "remove-time-s",
        ),
        ("role = \"head\"", "role = \"tail\"", "tail"),
        (
            "heat = { strength = 12, reach-m = 2.0 }",
            "heat = { strength = 300, reach-m = 2.0 }",
            "heat",
        ),
        (
            "collision = [{ min = [0, 0, 0], max = [16, 12, 16] }]",
            "collision = [{ min = [0, 0, 0], max = [16, 12, 16], x = 1 }]",
            "x",
        ),
        ("[world.movement]", "[world.walking]", "walking"),
    ] {
        refused(replace, with, field);
    }
}

#[test]
fn shared_validation_bounds_apply_to_data() {
    for (replace, with, field) in [
        (
            "max = [16, 8, 16] }]\ncollision",
            "max = [16, 8, 17] }]\ncollision",
            "shape",
        ),
        (
            "min = [6, 0, 6], max = [10, 16, 10]",
            "min = [6, 0, 6], max = [6, 16, 10]",
            "shape",
        ),
        ("face = 1", "face = 6", "shape"),
        (
            "emission = 12\ndimming = 2",
            "emission = 16\ndimming = 2",
            "emission",
        ),
        ("view-distance-m = 160.0", "view-distance-m = 1.0", "view"),
        ("relief-m = 12", "relief-m = -3", "world"),
        (
            "allow-flight = true\nallow-physics = true\nstart-in-flight = false",
            "allow-flight = false\nallow-physics = true\nstart-in-flight = true",
            "movement",
        ),
        ("pixel-m = 0.05625", "pixel-m = 0.0", "character"),
        ("size = [8, 12, 4]", "size = [8, 33, 4]", "character"),
        ("parent = \"body\"", "parent = \"neck\"", "character"),
        (
            "material = \"full:lava\"\nreach-m",
            "material = \"full:nothing\"\nreach-m",
            "nothing",
        ),
        (
            "material = \"full:water\"\nreach-m",
            "material = \"full:rock\"\nreach-m",
            "fluid",
        ),
        (
            "ignition-threshold = 6",
            "ignition-threshold = 0",
            "combustion",
        ),
        ("seconds = 8.0", "seconds = 5000.0", "combustion"),
        (
            "fuel = \"full:wood\"",
            "fuel = \"full:water\"",
            "combustion",
        ),
        ("fuel = \"full:ash\"", "fuel = \"full:wood\"", "combustion"),
    ] {
        refused(replace, with, field);
    }
}

#[test]
fn a_mod_that_is_not_the_game_may_not_declare_game_only_fields() {
    for (toml, field) in [
        (
            "[view]\nview-distance-m = 160\nlod-distance-m = 3000",
            "view",
        ),
        ("[[tools]]\nname = 'ruler'\nkind = 'measure'", "tools"),
        (
            "[world.movement]\nallow-flight = true\nallow-physics = true\nstart-in-flight = false",
            "world.movement",
        ),
        ("[world]\nrelief-m = 10.0", "world.relief-m"),
    ] {
        let d = parse(toml).unwrap();
        let mut tables = Tables::builtin();
        let mut registry = Registry::builtin();
        let error = apply(d, "full", &mut tables, &mut registry, false)
            .err()
            .unwrap();
        assert!(
            matches!(error, ModError::NotAGame { field: f, .. } if f == field),
            "{error}"
        );
    }
}

#[test]
fn per_mod_row_limits_apply_to_data() {
    let rows = |table: &str, row: &str| {
        (0..65)
            .map(|_| format!("[[{table}]]\n{row}\n"))
            .collect::<String>()
    };
    for (table, row, what) in [
        ("tools", "name = 'a'\nkind = 'measure'", "tool"),
        (
            "fluids",
            "material = 'full:x'\nreach-m = 1\nsideways-m-per-s = 1\ndownward-m-per-s = 1\nrecession-m-per-s = 1\nreset-drop-m = 1\nwaterlogs = false\nextinguishes = false",
            "fluid",
        ),
    ] {
        let d = parse(&rows(table, row)).unwrap();
        let mut tables = Tables::builtin();
        let mut registry = Registry::builtin();
        let error = apply(d, "full", &mut tables, &mut registry, true)
            .err()
            .unwrap();
        assert!(
            matches!(error, ModError::TooManyRows { what: w, .. } if w == what),
            "{error}"
        );
    }
}

#[test]
fn ambiguous_reactions_fail_the_load() {
    let mut text = full();
    text.push_str("\n[[reactions]]\nconsumed = 'first'\nfirst = { fluid = 'full:lava', state = 'any' }\nsecond = { fluid = 'full:water', state = 'any' }\n");
    let d = parse(&text).unwrap();
    let mut tables = Tables::builtin();
    let mut registry = Registry::builtin();
    let applied = apply(d, "full", &mut tables, &mut registry, true).unwrap();
    let error = resolve(vec![applied.behaviour.clone()], &mut tables, &registry).unwrap_err();
    assert!(error.to_string().contains("full"), "{error}");
}
