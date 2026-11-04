//! Declarations become table rows in load order, and the limits hold.

use tesserae_core::shape::examples;
use tesserae_core::{
    Arm, BiomeId, BiomeSet, BiomeTable, ConnectDef, Connections, GroundParams, MaterialError,
    OreTable, Orientation, ShapeDef, ShapeError, Tables, Voxel, WorldgenParams,
};

use crate::bindings::tesserae::mods::content::{self, Declaration};
use crate::bindings::tesserae::mods::types;
use crate::{
    Applied, Kind, MAX_ITEMS, MAX_ITEMS_PER_MOD, MAX_TOOLS_PER_MOD, ModError, Registry,
    RegistryMismatch, ToolDef, ToolKind, ViewParams, apply, resolve_biomes, resolve_items,
};

fn material(name: &str) -> content::MaterialDef {
    content::MaterialDef {
        name: name.into(),
        emission: 0,
        dimming: 15,
        occlusion: content::Occlusion::Opaque,
        render: content::RenderClass::Opaque,
        remove_time_s: None,
        solid: true,
    }
}

fn slab(name: &str) -> content::ShapeDef {
    content::ShapeDef {
        name: name.into(),
        boxes: vec![types::NodeBox {
            min: (0, 0, 0),
            max: (16, 8, 16),
        }],
        collision: None,
        connect: None,
    }
}

fn declaring(materials: &[&str], shapes: &[&str]) -> Declaration {
    Declaration {
        materials: materials.iter().map(|n| material(n)).collect(),
        shapes: shapes.iter().map(|n| slab(n)).collect(),
        appearance: Vec::new(),
        world: None,
        view: None,
        fluids: Vec::new(),
        reactions: Vec::new(),
        combustion: Vec::new(),
        tools: Vec::new(),
        items: Vec::new(),
        biomes: Vec::new(),
        ores: Vec::new(),
    }
}

struct Load {
    tables: Tables,
    registry: Registry,
}

impl Load {
    fn new() -> Self {
        Load {
            tables: Tables::builtin(),
            registry: Registry::builtin(),
        }
    }

    fn apply(&mut self, id: &str, d: Declaration) -> Result<Applied, ModError> {
        apply(d, id, &mut self.tables, &mut self.registry, false)
    }
}

fn names(registry: &Registry, kind: Kind) -> Vec<&str> {
    registry.names(kind).map(|(_, name)| name).collect()
}

#[test]
fn two_mods_register_in_load_order_after_the_built_ins() {
    let mut load = Load::new();
    load.apply("a", declaring(&["x", "y"], &["s", "t"]))
        .unwrap();
    load.apply("b", declaring(&["z"], &["u"])).unwrap();

    assert_eq!(load.registry.id_of(Kind::Material, "a:x"), Some(8));
    assert_eq!(load.registry.id_of(Kind::Material, "a:y"), Some(9));
    assert_eq!(load.registry.id_of(Kind::Material, "b:z"), Some(10));
    assert_eq!(load.registry.id_of(Kind::Shape, "a:s"), Some(4));
    assert_eq!(load.registry.id_of(Kind::Shape, "a:t"), Some(5));
    assert_eq!(load.registry.id_of(Kind::Shape, "b:u"), Some(6));
    assert_eq!(load.registry.id_of(Kind::Material, "core:leaves"), None);
    assert_eq!(load.registry.id_of(Kind::Material, "core:water"), None);
    assert_eq!(load.registry.id_of(Kind::Shape, "core:post"), Some(3));
    assert_eq!(load.tables.materials.len(), 11);
    assert_eq!(load.tables.shapes.len(), 7);
}

#[test]
fn swapping_load_order_swaps_ids_and_replay_restores_the_first() {
    let mut first = Load::new();
    first.apply("a", declaring(&["x"], &["s"])).unwrap();
    first.apply("b", declaring(&["z"], &["u"])).unwrap();
    let mut second = Load::new();
    second.apply("b", declaring(&["z"], &["u"])).unwrap();
    second.apply("a", declaring(&["x"], &["s"])).unwrap();

    assert_eq!(second.registry.id_of(Kind::Material, "b:z"), Some(8));
    assert_eq!(second.registry.id_of(Kind::Material, "a:x"), Some(9));
    assert_eq!(second.registry.id_of(Kind::Shape, "b:u"), Some(4));
    assert_ne!(first.registry, second.registry);

    let replayed = second.registry.replay(&first.registry).unwrap();
    assert_eq!(replayed, first.registry);
}

#[test]
fn replay_fails_for_a_name_no_loaded_mod_provides() {
    let mut saved = Load::new();
    saved.apply("a", declaring(&["x"], &["s"])).unwrap();
    saved.apply("gone", declaring(&["m"], &[])).unwrap();
    let mut now = Load::new();
    now.apply("a", declaring(&["x"], &["s"])).unwrap();

    assert_eq!(
        now.registry.replay(&saved.registry),
        Err(RegistryMismatch::Missing {
            kind: Kind::Material,
            name: "gone:m".into()
        })
    );
}

#[test]
fn replay_fails_when_a_built_in_is_not_where_it_belongs() {
    let mut saved = Registry::builtin();
    saved.shapes.swap(1, 2);
    assert_eq!(
        Registry::builtin().replay(&saved),
        Err(RegistryMismatch::BuiltinMoved {
            kind: Kind::Shape,
            id: 1
        })
    );
}

#[test]
fn the_built_ins_are_ids_0_to_6_and_7_is_never_listed() {
    let builtin = Registry::builtin();
    assert_eq!(
        builtin.names(Kind::Material).collect::<Vec<_>>(),
        [(0, "core:air"),]
    );
    assert_eq!(builtin.len(Kind::Material), 8);
    assert_eq!(builtin.id_of(Kind::Material, ""), None);

    let mut saved = Registry::builtin();
    saved.materials[7] = Some("a:x".into());
    assert_eq!(
        Registry::builtin().replay(&saved),
        Err(RegistryMismatch::BuiltinMoved {
            kind: Kind::Material,
            id: 7
        })
    );
}

#[test]
fn a_bad_material_names_the_mod_and_the_row() {
    for (bad, expected) in [
        (
            content::MaterialDef {
                dimming: 14,
                ..material("glow")
            },
            MaterialError::OpaqueNotOpaque,
        ),
        (
            content::MaterialDef {
                emission: 16,
                ..material("glow")
            },
            MaterialError::LightOutOfRange,
        ),
    ] {
        let mut load = Load::new();
        let mut d = declaring(&["fine"], &[]);
        d.materials.push(bad);
        let err = load.apply("mymod", d).unwrap_err();
        match &err {
            ModError::Material {
                id,
                row,
                name,
                source,
            } => {
                assert_eq!(
                    (id.as_str(), *row, name.as_str()),
                    ("mymod", 1, "mymod:glow")
                );
                assert_eq!(*source, expected);
            }
            other => panic!("{other:?}"),
        }
        assert!(err.to_string().contains("mymod") && err.to_string().contains("Material 1"));
    }
}

#[test]
fn a_bad_shape_names_the_mod_and_the_row() {
    let no_boxes = content::ShapeDef {
        boxes: Vec::new(),
        ..slab("flat")
    };
    let no_arms = content::ShapeDef {
        connect: Some(types::ConnectDef {
            group: 1,
            connects_to: 1,
            to_sealed: false,
            arms: Vec::new(),
        }),
        ..slab("armless")
    };
    let seals = {
        let mut wit = shape_to_wit("sealing", examples::pipe());
        wit.boxes = vec![types::NodeBox {
            min: (0, 0, 0),
            max: (16, 16, 16),
        }];
        wit
    };
    for (bad, expected) in [
        (no_boxes, ShapeError::NoBoxes),
        (no_arms, ShapeError::BadArm),
        (seals, ShapeError::ConnectedSeals),
    ] {
        let mut load = Load::new();
        let mut d = declaring(&[], &["fine", "fine2"]);
        d.shapes.push(bad);
        match load.apply("mymod", d).unwrap_err() {
            ModError::Shape {
                id, row, source, ..
            } => {
                assert_eq!((id.as_str(), row), ("mymod", 2));
                assert_eq!(source, expected);
            }
            other => panic!("{other:?}"),
        }
    }
}

#[test]
fn the_palette_holds_56_materials_in_total_across_mods() {
    let many =
        |from: usize, to: usize| -> Vec<String> { (from..to).map(|i| format!("m{i}")).collect() };
    let decl =
        |names: &[String]| declaring(&names.iter().map(String::as_str).collect::<Vec<_>>(), &[]);

    let mut load = Load::new();
    load.apply("a", decl(&many(0, 40))).unwrap();
    load.apply("b", decl(&many(0, 16))).unwrap();
    assert_eq!(load.tables.materials.len(), 8 + 56);

    let mut load = Load::new();
    load.apply("a", decl(&many(0, 40))).unwrap();
    match load.apply("b", decl(&many(0, 17))).unwrap_err() {
        ModError::PaletteFull { id, count, max } => {
            assert_eq!((id.as_str(), count, max), ("b", 57, 56))
        }
        other => panic!("{other:?}"),
    }
    let mut load = Load::new();
    assert!(matches!(
        load.apply("solo", decl(&many(0, 57))).unwrap_err(),
        ModError::PaletteFull { .. }
    ));
}

#[test]
fn no_mod_brings_more_than_64_of_either_and_shapes_stop_at_252() {
    let shapes = |n: usize| -> Vec<String> { (0..n).map(|i| format!("s{i}")).collect() };
    let decl =
        |names: &[String]| declaring(&[], &names.iter().map(String::as_str).collect::<Vec<_>>());

    let mut load = Load::new();
    assert!(matches!(
        load.apply("big", decl(&shapes(65))).unwrap_err(),
        ModError::TooMany {
            kind: Kind::Shape,
            max: 64,
            ..
        }
    ));
    let mut load = Load::new();
    for id in ["a", "b", "c"] {
        load.apply(id, decl(&shapes(64))).unwrap();
    }
    load.apply("d", decl(&shapes(60))).unwrap();
    assert_eq!(load.tables.shapes.len(), 256);
    assert!(matches!(
        load.apply("e", decl(&shapes(1))).unwrap_err(),
        ModError::TooMany {
            kind: Kind::Shape,
            max: 252,
            ..
        }
    ));
}

#[test]
fn a_name_is_local_to_its_mod_and_may_not_repeat() {
    let mut load = Load::new();
    for bad in [
        "other:thing",
        "",
        "Upper",
        "9lives",
        "has-dash",
        &"x".repeat(33),
    ] {
        match load.apply("mymod", declaring(&[bad], &[])).unwrap_err() {
            ModError::BadName { id, row, .. } => assert_eq!((id.as_str(), row), ("mymod", 0)),
            other => panic!("{bad:?}: {other:?}"),
        }
    }
    assert!(
        load.apply("mymod", declaring(&[&"x".repeat(32), "a_1"], &[]))
            .is_ok()
    );

    let err = load
        .apply("mymod", declaring(&["fresh", "a_1"], &[]))
        .unwrap_err();
    assert!(
        matches!(&err, ModError::DuplicateName { row: 1, name, .. } if name == "mymod:a_1"),
        "{err:?}"
    );
    let err = load
        .apply("twice", declaring(&["n", "n"], &[]))
        .unwrap_err();
    assert!(
        matches!(&err, ModError::DuplicateName { row: 1, .. }),
        "{err:?}"
    );
    let err = load
        .apply("twice", declaring(&[], &["n", "n"]))
        .unwrap_err();
    assert!(
        matches!(
            &err,
            ModError::DuplicateName {
                kind: Kind::Shape,
                row: 1,
                ..
            }
        ),
        "{err:?}"
    );
}

#[test]
fn a_bad_row_after_a_good_one_registers_nothing_of_its_names() {
    let mut load = Load::new();
    load.apply("a", declaring(&["ok", "Bad"], &[])).unwrap_err();
    assert_eq!(load.tables.materials.len(), 8);
    assert_eq!(names(&load.registry, Kind::Material).len(), 1);
}

#[test]
fn the_built_in_namespace_is_not_a_mods() {
    let mut load = Load::new();
    for local in ["air", "cube", "newthing"] {
        let err = load.apply("core", declaring(&[local], &[])).unwrap_err();
        assert!(
            matches!(err, ModError::ReservedId { .. }),
            "{local}: {err:?}"
        );
    }
    assert_eq!(load.registry, Registry::builtin());
}

fn world(caves: bool) -> content::WorldParams {
    let mut params = crate::convert::engine_world_params();
    if !caves {
        params.caves = None;
    }
    params
}

#[test]
fn only_the_game_sets_world_or_view() {
    let mut d = declaring(&[], &[]);
    d.world = Some(world(true));
    let mut load = Load::new();
    match load.apply("content", d).unwrap_err() {
        ModError::NotAGame { id, field } => {
            assert_eq!((id.as_str(), field), ("content", "world.caves"))
        }
        other => panic!("{other:?}"),
    }
    let mut d = declaring(&[], &[]);
    d.view = Some(content::ViewParams {
        view_distance_m: 160.0,
        lod_distance_m: 3000.0,
    });
    assert!(matches!(
        load.apply("content", d).unwrap_err(),
        ModError::NotAGame { field: "view", .. }
    ));
}

#[test]
fn a_game_with_no_caves_gets_no_caves_and_its_reach() {
    let mut d = declaring(&["lamp"], &[]);
    d.world = Some(world(false));
    d.view = Some(content::ViewParams {
        view_distance_m: 160.0,
        lod_distance_m: 3000.0,
    });
    let mut load = Load::new();
    let applied = apply(d, "game", &mut load.tables, &mut load.registry, true).unwrap();

    assert_eq!(
        applied.world,
        Some(WorldgenParams {
            caves: None,
            ..WorldgenParams::default()
        })
    );
    assert_eq!(
        applied.view,
        Some(ViewParams {
            view_distance_m: 160.0,
            lod_distance_m: 3000.0
        })
    );
    assert_eq!(load.registry.id_of(Kind::Material, "game:lamp"), Some(8));
}

#[test]
fn a_game_that_sets_nothing_keeps_the_engines_defaults() {
    let mut load = Load::new();
    let applied = apply(
        declaring(&[], &[]),
        "game",
        &mut load.tables,
        &mut load.registry,
        true,
    )
    .unwrap();
    assert_eq!((applied.world, applied.view), (None, None));
}

#[test]
fn invalid_world_parameters_are_a_load_error_naming_the_field() {
    type Edit = fn(&mut content::WorldParams);
    let cases: [(Edit, &str); 7] = [
        (|p| p.relief_m = f32::NAN, "relief_m"),
        (|p| p.feature_size_m = f32::INFINITY, "feature_size_m"),
        (|p| p.relief_m = -1.0, "relief_m"),
        (|p| p.octaves = 0, "octaves"),
        (|p| p.soil_depth_m = f32::NEG_INFINITY, "soil_depth_m"),
        (
            |p| {
                p.caves = Some(content::CaveParams {
                    feature_size_m: 64.0,
                    radius_m: 0.0,
                    floor_m: -24.0,
                    octaves: 2,
                })
            },
            "caves.radius_m",
        ),
        (
            |p| {
                p.water = Some(content::WaterParams {
                    level_m: f32::NAN,
                    material: "game:brine".into(),
                })
            },
            "water.level_m",
        ),
    ];
    for (edit, field) in cases {
        let mut p = world(true);
        edit(&mut p);
        let mut d = declaring(&[], &[]);
        d.world = Some(p);
        let mut load = Load::new();
        match apply(d, "game", &mut load.tables, &mut load.registry, true).unwrap_err() {
            ModError::Params { id, what, source } => {
                assert_eq!((id.as_str(), what, source.field), ("game", "world", field));
            }
            other => panic!("{field}: {other:?}"),
        }
    }
}

#[test]
fn the_reach_is_checked_in_metres() {
    let ok = |view, lod| {
        ViewParams {
            view_distance_m: view,
            lod_distance_m: lod,
        }
        .validate()
    };
    assert_eq!(ok(32.0, 32.0), Ok(()));
    assert_eq!(ok(512.0, 8192.0), Ok(()));
    assert_eq!(ok(192.0, 4096.0), Ok(()));
    for (view, lod, field) in [
        (31.9, 4096.0, "view_distance_m"),
        (512.1, 4096.0, "view_distance_m"),
        (f32::NAN, 4096.0, "view_distance_m"),
        (192.0, 191.0, "lod_distance_m"),
        (192.0, 8192.5, "lod_distance_m"),
        (192.0, f32::INFINITY, "lod_distance_m"),
        (192.0, f32::NAN, "lod_distance_m"),
    ] {
        assert_eq!(ok(view, lod).unwrap_err().field, field, "{view} {lod}");
    }
    let mut d = declaring(&[], &[]);
    d.view = Some(content::ViewParams {
        view_distance_m: 10.0,
        lod_distance_m: 20.0,
    });
    let mut load = Load::new();
    let err = apply(d, "game", &mut load.tables, &mut load.registry, true).unwrap_err();
    assert!(
        matches!(&err, ModError::Params { what: "view", .. }),
        "{err:?}"
    );
}

#[test]
fn the_engines_default_world_converts_and_validates() {
    let (back, lake) = crate::convert::world_params(crate::convert::engine_world_params());
    assert_eq!(
        (back, lake),
        (
            WorldgenParams {
                ground: None,
                ..WorldgenParams::default()
            },
            None
        )
    );
    assert_eq!(back.validate(), Ok(()));
}

fn arm_to_wit(arm: Arm) -> types::Arm {
    types::Arm {
        face: arm.face as u8,
        boxes: arm.boxes.into_iter().map(Into::into).collect(),
        collision: arm
            .collision
            .map(|c| c.into_iter().map(Into::into).collect()),
    }
}

fn connect_to_wit(c: ConnectDef) -> types::ConnectDef {
    types::ConnectDef {
        group: c.group,
        connects_to: c.connects_to,
        to_sealed: c.to_sealed,
        arms: c.arms.into_iter().map(arm_to_wit).collect(),
    }
}

fn shape_to_wit(name: &str, def: ShapeDef) -> content::ShapeDef {
    content::ShapeDef {
        name: name.into(),
        boxes: def.boxes.into_iter().map(Into::into).collect(),
        collision: def
            .collision
            .map(|c| c.into_iter().map(Into::into).collect()),
        connect: def.connect.map(connect_to_wit),
    }
}

/// Every answer the table gives for the Shape at `id`, over all 24
/// orientations and every set of arms.
fn derived(tables: &Tables, id: u8) -> Vec<String> {
    let t = &tables.shapes;
    let mut out = vec![format!("{} {}", t.len(), t.any_connected())];
    for orientation in Orientation::all() {
        let v = Voxel::shaped(1, id, orientation);
        out.push(format!(
            "{:?} {} {} {} {} {}",
            orientation.bits(),
            t.volume(v),
            t.light_mask(v),
            t.joint_id(v) != 0,
            t.cap_units(v),
            t.is_connected(v)
        ));
        for face in 0..6 {
            out.push(format!("{:?} {:?}", t.coverage(v, face), t.seals(v, face)));
        }
        for mask in 0..64u8 {
            let c = (0..6)
                .filter(|f| mask >> f & 1 != 0)
                .fold(Connections::NONE, |c, f| c.with(f));
            out.push(format!(
                "{:?} {:?} {:?}",
                t.quads(v, c),
                t.collision(v, c),
                t.selection(v, c)
            ));
        }
    }
    out
}

#[test]
fn the_terra_fence_and_pipe_declared_as_data_register_as_the_core_examples_do() {
    let mut load = Load::new();
    let d = Declaration {
        shapes: vec![
            shape_to_wit("fence", examples::fence()),
            shape_to_wit("pipe", examples::pipe()),
        ],
        ..declaring(&[], &[])
    };
    load.apply("terra", d).unwrap();

    let mut native = Tables::builtin();
    assert_eq!(
        native.shapes.register(examples::fence()),
        Ok(examples::FENCE_ID)
    );
    native.shapes.register(examples::pipe()).unwrap();

    assert_eq!(
        load.registry.id_of(Kind::Shape, "terra:fence"),
        Some(examples::FENCE_ID as usize)
    );
    for id in [examples::FENCE_ID, examples::FENCE_ID + 1] {
        assert_eq!(
            derived(&load.tables, id),
            derived(&native, id),
            "shape {id}"
        );
    }
}

#[test]
fn preview_texture_payloads_are_bounded_and_name_the_invalid_appearance_row() {
    let row = |texture| content::Appearance {
        material: "rock".into(),
        color: types::Srgba {
            r: 120,
            g: 100,
            b: 80,
            a: 255,
        },
        texture,
    };
    for len in [
        0,
        1024,
        crate::MATERIAL_TEXTURE_BYTES - 1,
        crate::MATERIAL_TEXTURE_BYTES + 1,
    ] {
        let mut d = declaring(&["rock"], &[]);
        d.appearance.push(row(Some(vec![0; len])));
        let error = Load::new().apply("skin", d).unwrap_err();
        assert!(matches!(error,ModError::MaterialTexture { row:0,bytes,.. } if bytes==len));
        let message = error.to_string();
        assert!(message.contains("appearance 0 `rock`"), "{message}");
        assert!(message.contains("3072"), "{message}");
    }
    for preview in [None, Some(vec![255; crate::MATERIAL_TEXTURE_BYTES])] {
        let mut d = declaring(&["rock"], &[]);
        d.appearance.push(row(preview));
        assert!(Load::new().apply("skin", d).is_ok());
    }
    let mut d = declaring(&["rock"], &[]);
    d.appearance = (0..65).map(|_| row(None)).collect();
    assert!(matches!(
        Load::new().apply("skin", d),
        Err(ModError::TooManyRows {
            what: "appearance",
            max: 64,
            ..
        })
    ));
}

#[test]
fn material_removal_time_is_validated_and_defaults_for_absent_metadata() {
    use std::time::Duration;
    for (seconds, expected) in [
        (None, Duration::from_millis(350)),
        (Some(0.05), Duration::from_secs_f32(0.05)),
        (Some(60.0), Duration::from_secs(60)),
    ] {
        let mut load = Load::new();
        let mut declaration = declaring(&["stone"], &[]);
        declaration.materials[0].remove_time_s = seconds;
        let applied = load.apply("game", declaration).unwrap();
        let id =
            load.registry.id_of(Kind::Material, "game:stone").unwrap() as tesserae_core::MaterialId;
        assert_eq!(applied.remove_times, vec![(id, expected)]);
    }
    for seconds in [0.0, -1.0, 0.049, 60.1, f32::NAN, f32::INFINITY] {
        let mut declaration = declaring(&["stone"], &[]);
        declaration.materials[0].remove_time_s = Some(seconds);
        let error = Load::new().apply("game", declaration).unwrap_err();
        assert!(matches!(error, ModError::RemoveTime { .. }));
        let text = error.to_string();
        assert!(text.contains("game:stone") && text.contains("0.05 to 60"));
    }
}

#[test]
fn game_movement_policy_requires_an_allowed_initial_mode() {
    for (flight, physics, start, valid) in [
        (true, true, false, true),
        (false, true, false, true),
        (true, false, true, true),
        (false, true, true, false),
        (true, false, false, false),
        (false, false, false, false),
    ] {
        let mut d = declaring(&[], &[]);
        let mut world = crate::convert::engine_world_params();
        world.movement = Some(content::MovementPolicy {
            allow_flight: flight,
            allow_physics: physics,
            start_in_flight: start,
        });
        d.world = Some(world);
        let mut load = Load::new();
        let result = apply(d, "game", &mut load.tables, &mut load.registry, true);
        assert_eq!(result.is_ok(), valid);
        if valid {
            assert_eq!(result.unwrap().movement.unwrap().allow_flight, flight);
        }
    }
}

fn figure() -> content::CharacterDef {
    let still = content::LimbMotion {
        swing_deg: 0.0,
        rate_hz: 0.0,
        arm_raise_deg: 0.0,
    };
    let cube = |origin, size, uv| content::CharacterCube { origin, size, uv };
    content::CharacterDef {
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
        pixel_m: 1.8 / 32.0,
        skin: vec![0; crate::SKIN_BYTES],
        idle: still,
        walk: still,
        sprint: still,
        airborne: still,
        crouch: still,
    }
}

fn declare_figure(figure: content::CharacterDef, is_root: bool) -> Result<Applied, ModError> {
    let mut d = declaring(&[], &[]);
    let mut world = crate::convert::engine_world_params();
    world.character = Some(figure);
    d.world = Some(world);
    let mut load = Load::new();
    apply(d, "game", &mut load.tables, &mut load.registry, is_root)
}

#[test]
fn a_game_character_is_validated_into_bones_that_name_earlier_parents() {
    let character = declare_figure(figure(), true).unwrap().character.unwrap();
    assert_eq!(character.bones.len(), 2);
    assert_eq!(character.bones[0].parent, None);
    assert_eq!(character.bones[1].parent, Some(0));
    assert_eq!(character.bones[1].role, crate::Role::Head);
    assert_eq!(character.bones[1].cubes[0].size, [8, 8, 8]);
    assert!(
        declare_figure(figure(), false).is_err(),
        "only the Game declares World parameters"
    );
    let mut d = declaring(&[], &[]);
    d.world = Some(crate::convert::engine_world_params());
    let mut load = Load::new();
    let none = apply(d, "game", &mut load.tables, &mut load.registry, true).unwrap();
    assert_eq!(none.character, None);
}

#[test]
fn character_bounds_are_refused_at_load() {
    type Break = fn(&mut content::CharacterDef);
    let cases: [(&str, Break); 14] = [
        ("no bones", |f| f.bones.clear()),
        ("seventeen bones", |f| {
            for n in 0..15 {
                let mut bone = f.bones[1].clone();
                bone.name = format!("extra{n}");
                bone.cubes.clear();
                f.bones.push(bone);
            }
        }),
        ("seventeen cubes", |f| {
            let cube = f.bones[0].cubes[0];
            f.bones[0].cubes.extend(std::iter::repeat_n(cube, 15));
        }),
        ("short skin", |f| f.skin.truncate(16383)),
        ("tiny pixels", |f| f.pixel_m = 0.0),
        ("nan pixels", |f| f.pixel_m = f32::NAN),
        ("bad name", |f| f.bones[1].name = "Head".into()),
        ("duplicate name", |f| f.bones[1].name = "body".into()),
        ("unknown parent", |f| {
            f.bones[1].parent = Some("neck".into())
        }),
        ("two roots", |f| f.bones[1].parent = None),
        ("empty cube", |f| f.bones[0].cubes[0].size.1 = 0),
        ("uv past the skin", |f| f.bones[0].cubes[0].uv = (41, 16)),
        ("far pivot", |f| f.bones[0].pivot.0 = 65.0),
        ("wild swing", |f| f.walk.swing_deg = 91.0),
    ];
    for (what, break_it) in cases {
        let mut f = figure();
        break_it(&mut f);
        let error = declare_figure(f, true).unwrap_err();
        assert!(
            matches!(
                error,
                ModError::Params {
                    what: "character",
                    ..
                }
            ),
            "{what}: {error}"
        );
    }
}

fn measure(name: &str) -> content::ToolDef {
    content::ToolDef {
        name: name.into(),
        kind: content::ToolKind::Measure,
    }
}

fn as_the_game(id: &str, d: Declaration) -> Result<Applied, ModError> {
    apply(
        d,
        id,
        &mut Tables::builtin(),
        &mut Registry::builtin(),
        true,
    )
}

#[test]
fn the_game_declares_tools_named_by_its_mod() {
    let mut d = declaring(&[], &[]);
    d.tools = vec![measure("ruler"), measure("tape")];
    let applied = as_the_game("terra", d).unwrap();
    assert_eq!(
        applied.tools,
        vec![
            ToolDef {
                name: "terra:ruler".into(),
                kind: ToolKind::Measure
            },
            ToolDef {
                name: "terra:tape".into(),
                kind: ToolKind::Measure
            },
        ]
    );
}

#[test]
fn a_mod_that_is_not_the_game_declares_no_tools() {
    let mut d = declaring(&[], &[]);
    d.tools = vec![measure("ruler")];
    let mut load = Load::new();
    assert!(matches!(
        load.apply("content", d).unwrap_err(),
        ModError::NotAGame { field: "tools", .. }
    ));
}

#[test]
fn tool_names_are_local_unique_and_bounded() {
    for bad in ["", "Ruler", "terra:ruler", "9lives", &"x".repeat(33)] {
        let mut d = declaring(&[], &[]);
        d.tools = vec![measure(bad)];
        assert!(
            matches!(
                as_the_game("terra", d).unwrap_err(),
                ModError::Params { what: "tool", .. }
            ),
            "{bad:?}"
        );
    }
    let mut d = declaring(&[], &[]);
    d.tools = vec![measure("ruler"), measure("ruler")];
    assert!(matches!(
        as_the_game("terra", d).unwrap_err(),
        ModError::Params { what: "tool", .. }
    ));
    let mut d = declaring(&[], &[]);
    d.tools = (0..=MAX_TOOLS_PER_MOD)
        .map(|i| measure(&format!("t{i}")))
        .collect();
    assert!(matches!(
        as_the_game("terra", d).unwrap_err(),
        ModError::TooManyRows {
            what: "tool",
            count: 9,
            max: 8,
            ..
        }
    ));
}

fn item(name: &str, material: Option<&str>) -> content::ItemDef {
    content::ItemDef {
        name: name.into(),
        size_m: 0.25,
        color: types::Srgba {
            r: 1,
            g: 2,
            b: 3,
            a: 255,
        },
        material: material.map(Into::into),
    }
}

fn with_items(materials: &[&str], items: Vec<content::ItemDef>) -> Declaration {
    let mut d = declaring(materials, &[]);
    d.items = items;
    d
}

fn resolved(
    load: &mut Load,
    mods: &[(&str, Declaration)],
) -> Result<Vec<crate::ItemDef>, ModError> {
    let behaviours: Vec<_> = mods
        .iter()
        .map(|(id, d)| load.apply(id, d.clone()).unwrap().behaviour)
        .collect();
    resolve_items(&behaviours, &load.tables, &load.registry)
}

#[test]
fn items_register_in_load_order_with_full_names_and_resolved_materials() {
    let mut load = Load::new();
    let items = resolved(
        &mut load,
        &[
            (
                "a",
                with_items(
                    &["rock"],
                    vec![item("chunk", Some("a:rock")), item("dust", None)],
                ),
            ),
            ("b", with_items(&[], vec![item("seed", None)])),
        ],
    )
    .unwrap();
    let names: Vec<_> = items.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, ["a:chunk", "a:dust", "b:seed"]);
    assert_eq!(
        items[0].material,
        load.registry
            .id_of(Kind::Material, "a:rock")
            .map(|i| i as u16)
    );
    assert_eq!(items[1].material, None);
    assert_eq!(items[0].color, [1, 2, 3, 255]);
}

#[test]
fn an_item_may_come_from_a_mod_that_is_not_the_game() {
    let mut load = Load::new();
    assert!(
        load.apply("content", with_items(&[], vec![item("dust", None)]))
            .is_ok()
    );
}

#[test]
fn a_bad_item_names_the_mod_and_the_field() {
    for (bad, field) in [
        (item("", None), "name"),
        (item("Dust", None), "name"),
        (item("a:dust", None), "name"),
        (
            content::ItemDef {
                size_m: 0.04,
                ..item("dust", None)
            },
            "size-m",
        ),
        (
            content::ItemDef {
                size_m: 0.51,
                ..item("dust", None)
            },
            "size-m",
        ),
        (
            content::ItemDef {
                size_m: f32::NAN,
                ..item("dust", None)
            },
            "size-m",
        ),
    ] {
        let error = Load::new()
            .apply("m", with_items(&[], vec![bad]))
            .unwrap_err();
        assert!(
            matches!(&error, ModError::Params { what: "item", source, .. } if source.field == field),
            "{error}"
        );
        assert!(error.to_string().contains("`m`"), "{error}");
    }
    let error = Load::new()
        .apply(
            "m",
            with_items(&[], vec![item("dust", None), item("dust", None)]),
        )
        .unwrap_err();
    assert!(matches!(error, ModError::Params { what: "item", .. }));
}

#[test]
fn an_item_material_must_exist_be_solid_and_belong_to_one_item() {
    let unknown = resolved(
        &mut Load::new(),
        &[("m", with_items(&[], vec![item("a", Some("m:nope"))]))],
    );
    assert!(matches!(
        unknown,
        Err(ModError::UnknownName { what: "item", .. })
    ));

    let mut d = with_items(&["glass"], vec![item("shard", Some("m:glass"))]);
    d.materials[0].solid = false;
    let not_solid = resolved(&mut Load::new(), &[("m", d)]);
    assert!(
        matches!(not_solid, Err(ModError::Params { what: "item", ref source, .. }) if source.field == "material"),
        "{not_solid:?}"
    );

    let twice = resolved(
        &mut Load::new(),
        &[
            ("m", with_items(&["rock"], vec![item("a", Some("m:rock"))])),
            ("n", with_items(&[], vec![item("b", Some("m:rock"))])),
        ],
    );
    assert!(
        matches!(twice, Err(ModError::Params { what: "item", ref source, .. }) if source.field == "material"),
        "{twice:?}"
    );
}

#[test]
fn items_are_bounded_per_mod_and_per_world() {
    let many = |prefix: &str, n: usize| {
        (0..n)
            .map(|i| item(&format!("{prefix}{i}"), None))
            .collect::<Vec<_>>()
    };
    assert!(matches!(
        Load::new()
            .apply("m", with_items(&[], many("i", MAX_ITEMS_PER_MOD + 1)))
            .unwrap_err(),
        ModError::TooManyRows {
            what: "item",
            count: 65,
            max: 64,
            ..
        }
    ));
    let mods: Vec<_> = ["a", "b", "c", "d", "e"]
        .iter()
        .map(|id| (*id, with_items(&[], many("i", MAX_ITEMS_PER_MOD))))
        .collect();
    assert!(matches!(
        resolved(&mut Load::new(), &mods),
        Err(ModError::TooManyRows {
            what: "item",
            max: MAX_ITEMS,
            ..
        })
    ));
}

fn biome(name: &str, temperature: f32, humidity: f32) -> content::BiomeDef {
    content::BiomeDef {
        name: name.into(),
        temperature,
        humidity,
        ground: None,
    }
}

fn ground(rock: &str, soil: &str, surface: &str) -> Option<content::GroundParams> {
    Some(content::GroundParams {
        rock: rock.into(),
        soil: soil.into(),
        surface: surface.into(),
    })
}

fn ore(material: &str, biomes: &[&str]) -> content::OreDef {
    content::OreDef {
        material: material.into(),
        min_depth_m: 4.0,
        max_depth_m: 30.0,
        vein_size_m: 2.0,
        rarity: 0.1,
        biomes: biomes.iter().map(|b| (*b).into()).collect(),
    }
}

fn with_biomes(
    materials: &[&str],
    biomes: Vec<content::BiomeDef>,
    ores: Vec<content::OreDef>,
) -> Declaration {
    let mut d = declaring(materials, &[]);
    d.biomes = biomes;
    d.ores = ores;
    d
}

fn resolved_biomes(
    load: &mut Load,
    mods: &[(&str, Declaration)],
) -> Result<(BiomeTable, OreTable), ModError> {
    let behaviours: Vec<_> = mods
        .iter()
        .map(|(id, d)| load.apply(id, d.clone()).unwrap().behaviour)
        .collect();
    resolve_biomes(&behaviours, &load.tables, &load.registry)
}

#[test]
fn biomes_and_ores_append_in_load_order_with_ores_naming_any_mods_biome() {
    let mut load = Load::new();
    let (biomes, ores) = resolved_biomes(
        &mut load,
        &[
            (
                "a",
                with_biomes(
                    &["rock"],
                    vec![biome("dry", 0.8, 0.1), biome("wet", 0.2, 0.9)],
                    vec![ore("a:rock", &["b:cold"])],
                ),
            ),
            (
                "b",
                with_biomes(
                    &["gem"],
                    vec![biome("cold", 0.1, 0.4)],
                    vec![ore("b:gem", &[])],
                ),
            ),
        ],
    )
    .unwrap();
    let names: Vec<_> = biomes.as_slice().iter().map(|b| b.name.as_str()).collect();
    assert_eq!(names, ["a:dry", "a:wet", "b:cold"]);
    assert_eq!(
        (
            biomes.as_slice()[1].temperature,
            biomes.as_slice()[1].humidity
        ),
        (0.2, 0.9)
    );
    let rock = load.registry.id_of(Kind::Material, "a:rock").unwrap() as u16;
    let gem = load.registry.id_of(Kind::Material, "b:gem").unwrap() as u16;
    let ores = ores.as_slice();
    assert_eq!((ores[0].material, ores[1].material), (rock, gem));
    assert_eq!(ores[0].biomes, BiomeSet::new(&[BiomeId::new(2)]).unwrap());
    assert!(ores[1].biomes.is_unrestricted());
    assert_eq!((ores[0].min_depth_m, ores[0].max_depth_m), (4.0, 30.0));
    assert_eq!((ores[0].vein_size_m, ores[0].rarity), (2.0, 0.1));
}

#[test]
fn a_biomes_ground_resolves_to_material_ids_and_an_omitted_ground_stays_none() {
    let mut load = Load::new();
    let (biomes, _) = resolved_biomes(
        &mut load,
        &[
            (
                "a",
                with_biomes(
                    &["sand", "clay", "dust"],
                    vec![
                        content::BiomeDef {
                            ground: ground("a:sand", "a:clay", "a:dust"),
                            ..biome("dry", 0.8, 0.1)
                        },
                        biome("wet", 0.2, 0.9),
                    ],
                    vec![],
                ),
            ),
            (
                "b",
                // Another Mod's Materials, as an ore may name them.
                with_biomes(
                    &[],
                    vec![content::BiomeDef {
                        ground: ground("a:dust", "a:dust", "a:sand"),
                        ..biome("cold", 0.1, 0.4)
                    }],
                    vec![],
                ),
            ),
        ],
    )
    .unwrap();
    let id = |name| load.registry.id_of(Kind::Material, name).unwrap() as u16;
    let at = biomes.as_slice();
    assert_eq!(
        at[0].ground,
        Some(GroundParams {
            rock: id("a:sand"),
            soil: id("a:clay"),
            surface: id("a:dust")
        })
    );
    assert_eq!(at[1].ground, None);
    assert_eq!(
        at[2].ground,
        Some(GroundParams {
            rock: id("a:dust"),
            soil: id("a:dust"),
            surface: id("a:sand")
        })
    );
}

#[test]
fn a_bad_biome_ground_names_the_mod_and_the_field() {
    let refused = |ground| {
        let mut d = with_biomes(
            &["rock"],
            vec![content::BiomeDef {
                ground,
                ..biome("dry", 0.5, 0.5)
            }],
            vec![],
        );
        d.materials.push(content::MaterialDef {
            name: "glass".into(),
            solid: false,
            ..material("glass")
        });
        let mut load = Load::new();
        let applied = load.apply("m", d).unwrap();
        resolve_biomes(&[applied.behaviour], &load.tables, &load.registry).unwrap_err()
    };
    for (bad, field) in [
        (ground("m:rock", "m:rock", "m:glass"), "ground.surface"),
        (ground("core:air", "m:rock", "m:rock"), "ground.rock"),
        (ground("m:rock", "m:glass", "m:rock"), "ground.soil"),
    ] {
        match refused(bad) {
            ModError::Params { id, what, source } => {
                assert_eq!((id.as_str(), what), ("m", "biome"));
                assert_eq!(source.field, field);
            }
            other => panic!("{field}: {other:?}"),
        }
    }
    match refused(ground("m:rock", "m:rock", "m:nothing")) {
        ModError::UnknownName { id, what, name, .. } => assert_eq!(
            (id.as_str(), what, name.as_str()),
            ("m", "biome", "m:nothing")
        ),
        other => panic!("{other:?}"),
    }
}

#[test]
fn no_biomes_and_no_ores_resolve_to_empty_tables() {
    let mut load = Load::new();
    let (biomes, ores) = resolved_biomes(&mut load, &[("a", declaring(&["rock"], &[]))]).unwrap();
    assert!(biomes.is_empty() && ores.is_empty());
}

#[test]
fn a_mod_that_is_not_the_game_may_add_biomes_and_ores() {
    let mut load = Load::new();
    let d = with_biomes(
        &["rock"],
        vec![biome("dry", 0.5, 0.5)],
        vec![ore("content:rock", &[])],
    );
    assert!(load.apply("content", d).is_ok());
}

fn refused_biome_rows(d: Declaration, field: &str) {
    let mut load = Load::new();
    let error = match load.apply("m", d.clone()) {
        Err(e) => e,
        Ok(applied) => {
            resolve_biomes(&[applied.behaviour], &load.tables, &load.registry).unwrap_err()
        }
    };
    match error {
        ModError::Params { id, source, .. } => {
            assert_eq!(id, "m");
            assert!(source.field.contains(field), "{field}: {source}");
        }
        ModError::UnknownName { id, what, .. } => {
            assert_eq!((id.as_str(), what, field), ("m", "ore", "material"));
        }
        other => panic!("{field}: {other:?}"),
    }
}

#[test]
fn a_bad_biome_names_the_mod_and_the_field() {
    for (bad, field) in [
        (biome("", 0.5, 0.5), "name"),
        (biome("Dry", 0.5, 0.5), "name"),
        (biome("m:dry", 0.5, 0.5), "name"),
        (biome("dry", -0.1, 0.5), "temperature"),
        (biome("dry", 0.5, 1.1), "humidity"),
        (biome("dry", f32::NAN, 0.5), "temperature"),
        (biome("dry", 0.5, f32::INFINITY), "humidity"),
    ] {
        refused_biome_rows(with_biomes(&[], vec![bad], vec![]), field);
    }
    refused_biome_rows(
        with_biomes(
            &[],
            vec![biome("dry", 0.1, 0.1), biome("dry", 0.9, 0.9)],
            vec![],
        ),
        "name",
    );
}

#[test]
fn a_biome_whose_full_name_outgrows_the_table_is_refused() {
    let long_mod = "m".repeat(20);
    let mut load = Load::new();
    let d = with_biomes(&[], vec![biome("a_rather_long_biome", 0.5, 0.5)], vec![]);
    let applied = load.apply(&long_mod, d).unwrap();
    let error = resolve_biomes(&[applied.behaviour], &load.tables, &load.registry).unwrap_err();
    assert!(error.to_string().contains(&long_mod), "{error}");
}

#[test]
fn a_bad_ore_names_the_mod_and_the_field() {
    let edit = |f: fn(&mut content::OreDef)| {
        let mut o = ore("m:rock", &[]);
        f(&mut o);
        o
    };
    for (bad, field) in [
        (ore("m:nothing", &[]), "material"),
        (ore("core:air", &[]), "material"),
        (ore("m:glass", &[]), "material"),
        (ore("m:rock", &["m:nowhere"]), "biomes"),
        (edit(|o| o.min_depth_m = -1.0), "depth"),
        (edit(|o| o.min_depth_m = 31.0), "depth"),
        (edit(|o| o.max_depth_m = f32::NAN), "depth"),
        (edit(|o| o.vein_size_m = 0.0), "vein_size_m"),
        (edit(|o| o.rarity = 0.0), "rarity"),
        (edit(|o| o.rarity = 1.5), "rarity"),
    ] {
        let mut d = with_biomes(&["rock"], vec![biome("dry", 0.5, 0.5)], vec![bad]);
        // A Material that is not solid.
        d.materials.push(content::MaterialDef {
            name: "glass".into(),
            solid: false,
            ..material("glass")
        });
        refused_biome_rows(d, field);
    }
}

#[test]
fn the_world_holds_at_most_the_tables_cap_across_mods() {
    let rows = |prefix: &str, n: usize| -> Vec<content::BiomeDef> {
        (0..n)
            .map(|i| biome(&format!("{prefix}{i}"), 0.5, 0.5))
            .collect()
    };
    let mut load = Load::new();
    let error = resolved_biomes(
        &mut load,
        &[
            ("a", with_biomes(&[], rows("x", 10), vec![])),
            ("b", with_biomes(&[], rows("y", 7), vec![])),
        ],
    )
    .unwrap_err();
    assert!(
        matches!(&error, ModError::Params { id, .. } if id == "b"),
        "{error:?}"
    );
    let mut load = Load::new();
    let (biomes, _) = resolved_biomes(
        &mut load,
        &[
            ("a", with_biomes(&[], rows("x", 10), vec![])),
            ("b", with_biomes(&[], rows("y", 6), vec![])),
        ],
    )
    .unwrap();
    assert_eq!(biomes.len(), 16);

    let ores: Vec<_> = (0..16).map(|_| ore("a:rock", &[])).collect();
    let mut load = Load::new();
    let (_, table) = resolved_biomes(
        &mut load,
        &[
            ("a", with_biomes(&["rock"], vec![], ores.clone())),
            ("b", with_biomes(&[], vec![], ores.clone())),
        ],
    )
    .unwrap();
    assert_eq!(table.len(), 32);
    let mut load = Load::new();
    assert!(
        resolved_biomes(
            &mut load,
            &[
                ("a", with_biomes(&["rock"], vec![], ores.clone())),
                ("b", with_biomes(&[], vec![], ores.clone())),
                ("c", with_biomes(&[], vec![], vec![ore("a:rock", &[])])),
            ],
        )
        .is_err()
    );
}

#[test]
fn a_climate_that_is_not_finite_and_positive_is_a_load_error() {
    for (t, h, field) in [
        (0.0, 100.0, "climate.temperature_size_m"),
        (100.0, f32::NAN, "climate.humidity_size_m"),
        (f32::INFINITY, 100.0, "climate.temperature_size_m"),
    ] {
        let mut p = world(true);
        p.climate = Some(content::ClimateParams {
            temperature_size_m: t,
            humidity_size_m: h,
        });
        let mut d = declaring(&[], &[]);
        d.world = Some(p);
        let mut load = Load::new();
        match apply(d, "game", &mut load.tables, &mut load.registry, true).unwrap_err() {
            ModError::Params { what, source, .. } => {
                assert_eq!((what, source.field), ("world", field));
            }
            other => panic!("{field}: {other:?}"),
        }
    }
}

#[test]
fn only_the_game_declares_climate() {
    let mut p = crate::convert::engine_world_params();
    p.caves = None;
    p.smoothing = None;
    p.climate = Some(content::ClimateParams {
        temperature_size_m: 100.0,
        humidity_size_m: 100.0,
    });
    let mut d = declaring(&[], &[]);
    d.world = Some(p);
    let mut load = Load::new();
    assert!(matches!(
        load.apply("content", d.clone()).unwrap_err(),
        ModError::NotAGame {
            field: "world.climate",
            ..
        }
    ));
    let applied = apply(d, "game", &mut load.tables, &mut load.registry, true).unwrap();
    assert_eq!(
        applied.world.unwrap().climate,
        tesserae_core::ClimateParams {
            temperature_size_m: 100.0,
            humidity_size_m: 100.0
        }
    );
}
