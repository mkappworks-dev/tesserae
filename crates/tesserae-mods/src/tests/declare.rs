//! Declarations become table rows in load order, and the limits hold.

use tesserae_core::shape::examples;
use tesserae_core::{
    Arm, ConnectDef, Connections, MaterialError, Orientation, ShapeDef, ShapeError, Tables, Voxel,
    WorldgenParams,
};

use crate::bindings::tesserae::mods::content::{self, Declaration};
use crate::bindings::tesserae::mods::types;
use crate::{Applied, Kind, ModError, Registry, RegistryMismatch, ViewParams, apply};

fn material(name: &str) -> content::MaterialDef {
    content::MaterialDef {
        name: name.into(),
        emission: 0,
        dimming: 15,
        occlusion: content::Occlusion::Opaque,
        render: content::RenderClass::Opaque,
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
        ModError::NotAGame { id, field } => assert_eq!((id.as_str(), field), ("content", "world")),
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
fn the_demo_fence_and_pipe_declared_as_data_register_as_the_core_examples_do() {
    let mut load = Load::new();
    let d = Declaration {
        shapes: vec![
            shape_to_wit("fence", examples::fence()),
            shape_to_wit("pipe", examples::pipe()),
        ],
        ..declaring(&[], &[])
    };
    load.apply("demo", d).unwrap();

    let mut native = Tables::builtin();
    assert_eq!(
        native.shapes.register(examples::fence()),
        Ok(examples::FENCE_ID)
    );
    native.shapes.register(examples::pipe()).unwrap();

    assert_eq!(
        load.registry.id_of(Kind::Shape, "demo:fence"),
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
