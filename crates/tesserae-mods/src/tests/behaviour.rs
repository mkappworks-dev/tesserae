//! Fluid, reaction and fire rows resolve by full name once every Mod has
//! declared, and anything unknown, out of range or contradictory fails the
//! load before a World could open.

use futures_lite::future::block_on;
use tesserae_core::{
    Burn, CombustionError, FluidDef, FluidError, Heat, Phase, ReactionError, Side, Tables,
};

use super::sandbox::Rig;
use crate::bindings::tesserae::mods::content::{
    self, BurnAfter, BurnEnd, CombustionDef, Declaration, FluidState, HeatDef, MaterialDef,
    Occlusion, Reactant, ReactionDef, ReactionSide, RenderClass,
};
use crate::{
    Behaviour, Kind, MAX_BEHAVIOURS_PER_MOD, ModError, ModLimits, Registry, apply, resolve,
};

fn material(name: &str, solid: bool) -> MaterialDef {
    MaterialDef {
        name: name.into(),
        emission: 0,
        dimming: 2,
        occlusion: Occlusion::Never,
        render: RenderClass::Blend,
        remove_time_s: None,
        solid,
    }
}

fn fluid(material: &str) -> content::FluidDef {
    content::FluidDef {
        material: material.into(),
        reach_m: 3.0,
        sideways_m_per_s: 0.75,
        downward_m_per_s: 1.5,
        recession_m_per_s: 0.75,
        reset_drop_m: 0.5,
        waterlogs: false,
        extinguishes: false,
        heat: Some(HeatDef {
            strength: 12,
            reach_m: 2.0,
        }),
    }
}

fn reaction(first: &str, second: &str, product: Option<&str>) -> ReactionDef {
    let any = |fluid: &str| Reactant {
        fluid: fluid.into(),
        state: FluidState::Any,
    };
    ReactionDef {
        first: any(first),
        second: any(second),
        consumed: ReactionSide::First,
        product: product.map(Into::into),
        receiving: None,
    }
}

fn fire(fuel: &str, residue: Option<&str>) -> CombustionDef {
    CombustionDef {
        fuel: fuel.into(),
        ignition_threshold: 6,
        ignition_delay_s: 2.0,
        emission: 12,
        heat: Some(HeatDef {
            strength: 10,
            reach_m: 2.0,
        }),
        end: BurnEnd::After(BurnAfter {
            seconds: 8.0,
            residue: residue.map(Into::into),
        }),
    }
}

fn empty() -> Declaration {
    Declaration {
        materials: vec![],
        shapes: vec![],
        appearance: vec![],
        world: None,
        view: None,
        fluids: vec![],
        reactions: vec![],
        combustion: vec![],
    }
}

/// Declares each `(mod id, declaration)` in order, then resolves them all.
fn load(mods: Vec<(&str, Declaration)>) -> Result<(Tables, Registry), ModError> {
    let mut tables = Tables::builtin();
    let mut registry = Registry::builtin();
    let mut behaviours: Vec<Behaviour> = Vec::new();
    for (id, declaration) in mods {
        let applied = apply(declaration, id, &mut tables, &mut registry, false)?;
        behaviours.push(applied.behaviour);
    }
    resolve(behaviours, &mut tables, &registry)?;
    Ok((tables, registry))
}

/// Water as a Game declares it, the reaction partner lava used to find built in.
fn water_mod() -> Declaration {
    Declaration {
        materials: vec![material("water", false)],
        fluids: vec![content::FluidDef {
            waterlogs: true,
            extinguishes: true,
            heat: None,
            ..fluid("water:water")
        }],
        ..empty()
    }
}

fn lava_mod() -> Declaration {
    Declaration {
        materials: vec![material("lava", false), material("basalt", true)],
        fluids: vec![fluid("lava:lava")],
        ..empty()
    }
}

#[test]
fn a_fluid_declared_by_name_runs_its_row_at_the_registered_id() {
    let (tables, registry) = load(vec![("lava", lava_mod())]).unwrap();
    let id = registry.id_of(Kind::Material, "lava:lava").unwrap() as u16;
    assert_eq!(id, 8, "after the built-ins and the retired 7");
    assert_eq!(
        tables.fluids.get(id),
        Some(&FluidDef {
            reach_m: 3.0,
            sideways_m_per_s: 0.75,
            downward_m_per_s: 1.5,
            recession_m_per_s: 0.75,
            reset_drop_m: 0.5,
            waterlogs: false,
            extinguishes: false,
            heat: Some(Heat {
                strength: 12,
                reach_m: 2.0
            }),
        })
    );
    assert_eq!(tables.fluids.rank(id), Some(0));
    assert_eq!(tables.fluids.iter().count(), 1, "no fluid is built in");
}

#[test]
fn core_water_is_a_name_like_any_unknown_one() {
    let (tables, registry) = load(vec![("lava", lava_mod())]).unwrap();
    assert_eq!(registry.id_of(Kind::Material, "core:water"), None);
    assert!(registry.names(Kind::Material).all(|(id, _)| id != 7));
    assert!(!tables.materials.contains(7) && !tables.fluids.contains(7));
}

/// Names resolve after every Declaration: a row may name a Material of a
/// Mod that loads after it, and fluids are in before reactions need them.
#[test]
fn rows_name_any_loaded_mods_materials_whatever_the_load_order() {
    let early = Declaration {
        reactions: vec![reaction("lava:lava", "water:water", Some("lava:basalt"))],
        combustion: vec![fire("late:wood", Some("lava:basalt"))],
        ..empty()
    };
    let late = Declaration {
        materials: vec![material("wood", true)],
        ..empty()
    };
    let (tables, registry) = load(vec![
        ("early", early),
        ("water", water_mod()),
        ("lava", lava_mod()),
        ("late", late),
    ])
    .unwrap();
    let id = |name| registry.id_of(Kind::Material, name).unwrap() as u16;
    let rule = tables.reactions.rules()[0];
    assert_eq!(
        (rule.first.fluid, rule.second.fluid),
        (id("water:water"), id("lava:lava"))
    );
    assert_eq!(rule.consumed, Side::Second, "the lava, canonical");
    assert_eq!(rule.product, Some(id("lava:basalt")));
    assert_eq!(rule.first.phase, Phase::Any);
    let wood = tables.combustion.get(id("late:wood")).unwrap();
    assert_eq!(
        wood.burn,
        Burn::For {
            seconds: 8.0,
            residue: id("lava:basalt")
        }
    );
    assert_eq!(wood.threshold, 6);
}

/// Every refusal names the Mod and the row, and happens at resolution, which
/// the loader runs before any Mod starts and the World opens.
#[test]
fn unknown_names_bad_values_and_contradictions_fail_the_load() {
    type Change = fn(&mut Declaration);
    let with = |change: Change| {
        let mut d = lava_mod();
        change(&mut d);
        load(vec![("water", water_mod()), ("lava", d)])
            .expect_err("refused")
            .to_string()
    };
    let cases: [(Change, &str); 13] = [
        (
            |d| d.fluids[0].material = "lava:missing".into(),
            "fluid 0 names Material `lava:missing`",
        ),
        (
            |d| d.fluids[0].material = "lava".into(),
            "fluid 0 names Material `lava`",
        ),
        (
            |d| d.fluids[0].reach_m = f32::NAN,
            "fluid 0 `lava:lava`: a fluid's reach",
        ),
        (
            |d| d.fluids[0].reset_drop_m = -0.5,
            "fluid 0 `lava:lava`: a fluid's reach",
        ),
        (|d| d.fluids[0].sideways_m_per_s = 1e6, "its rates 256 m/s"),
        (
            |d| d.fluids[0].material = "lava:basalt".into(),
            "a solid Material cannot flow",
        ),
        (
            |d| d.fluids.push(fluid("lava:lava")),
            "fluid 1 `lava:lava`: the Material already",
        ),
        (
            |d| d.fluids.push(fluid("water:water")),
            "fluid 1 `water:water`: the Material already",
        ),
        (
            |d| d.fluids.push(fluid("core:water")),
            "fluid 1 names Material `core:water`",
        ),
        (
            |d| d.fluids[0].extinguishes = true,
            "a hot fluid cannot also extinguish",
        ),
        (
            |d| {
                d.reactions = vec![
                    reaction("lava:lava", "water:water", None),
                    reaction("water:water", "lava:lava", None),
                ]
            },
            "reaction 1: another reaction",
        ),
        (
            |d| d.reactions = vec![reaction("lava:lava", "water:water", Some("water:water"))],
            "reaction 0: the product must be",
        ),
        (
            |d| d.combustion = vec![fire("lava:basalt", Some("lava:basalt"))],
            "combustion 0 `lava:basalt`: residues lead back",
        ),
    ];
    for (change, expected) in cases {
        let message = with(change);
        assert!(message.contains("mod `lava`"), "{message}");
        assert!(message.contains(expected), "{expected}: {message}");
    }
}

#[test]
fn errors_carry_the_core_reason() {
    let mirrored = Declaration {
        reactions: vec![
            reaction("lava:lava", "water:water", None),
            ReactionDef {
                consumed: ReactionSide::Second,
                ..reaction("water:water", "lava:lava", None)
            },
        ],
        ..lava_mod()
    };
    assert!(matches!(
        load(vec![("water", water_mod()), ("lava", mirrored)]),
        Err(ModError::Reaction {
            row: 1,
            source: ReactionError::Duplicate,
            ..
        })
    ));
    let hot_fuel = Declaration {
        combustion: vec![CombustionDef {
            ignition_threshold: 0,
            ..fire("lava:basalt", None)
        }],
        ..lava_mod()
    };
    assert!(matches!(
        load(vec![("lava", hot_fuel)]),
        Err(ModError::Combustion {
            source: CombustionError::ThresholdOutOfRange,
            ..
        })
    ));
    let gushing = Declaration {
        fluids: vec![content::FluidDef {
            reach_m: 100.0,
            ..fluid("lava:lava")
        }],
        ..lava_mod()
    };
    assert!(matches!(
        load(vec![("lava", gushing)]),
        Err(ModError::Fluid {
            source: FluidError::OutOfRange,
            ..
        })
    ));
}

#[test]
fn a_mod_declares_a_bounded_number_of_rows() {
    let d = Declaration {
        combustion: (0..=MAX_BEHAVIOURS_PER_MOD)
            .map(|_| fire("core:leaves", None))
            .collect(),
        ..empty()
    };
    let error = load(vec![("many", d)]).err().unwrap();
    assert!(matches!(error, ModError::TooManyRows { count: 65, .. }));
    let mut tables = Tables::builtin();
    let d = Declaration {
        combustion: (0..=MAX_BEHAVIOURS_PER_MOD)
            .map(|_| fire("core:leaves", None))
            .collect(),
        materials: vec![material("x", true)],
        ..empty()
    };
    let _ = apply(d, "many", &mut tables, &mut Registry::builtin(), false);
    assert_eq!(tables.materials.len(), 8, "refused before any row");
}

/// The real 0.2 guest: its Declaration crosses the WASI boundary and resolves
/// as a native one does.
#[test]
fn the_fluids_guest_declares_a_fluid_a_reaction_and_a_fire() {
    let rig = Rig::new();
    let instance = rig.instance("fluids", ModLimits::HOST).unwrap();
    assert_eq!(instance.line(), crate::Line::V02);
    let (_, declaration) = block_on(instance.declare()).unwrap();
    let (tables, registry) = load(vec![("fluids", declaration)]).unwrap();
    let id = |name| registry.id_of(Kind::Material, name).unwrap() as u16;
    let slime = tables.fluids.get(id("fluids:slime")).unwrap();
    assert_eq!(
        (slime.reach_m, slime.sideways_m_per_s, slime.reset_drop_m),
        (2.0, 2.0, 1.0)
    );
    assert!(slime.waterlogs && slime.extinguishes && slime.heat.is_none());
    let brine = tables.fluids.get(id("fluids:brine")).unwrap();
    assert_eq!((brine.reach_m, brine.sideways_m_per_s), (7.0, 4.0));
    let rule = tables.reactions.rules()[0];
    assert_eq!(
        (rule.first.fluid, rule.second.fluid),
        (id("fluids:slime"), id("fluids:brine"))
    );
    assert_eq!(rule.first.phase, Phase::Flowing);
    assert_eq!(rule.product, Some(id("fluids:ash")));
    assert!(tables.combustion.contains(id("fluids:wood")));
    assert!(!tables.combustion.contains(id("fluids:ash")));
}

#[test]
fn the_bad_fluid_guest_is_refused_at_resolution() {
    let rig = Rig::new();
    let instance = rig.instance("bad-fluid", ModLimits::HOST).unwrap();
    let (_, declaration) = block_on(instance.declare()).unwrap();
    let message = load(vec![("bad", declaration)]).err().unwrap().to_string();
    assert!(
        message.contains("mod `bad`: fluid 0 names Material `nobody:quicksilver`"),
        "{message}"
    );
}

/// A 0.1 guest's Declaration is the 0.2 one it means: no fluid, reaction or
/// fire, and its Materials and Shapes as they were.
#[test]
fn a_0_1_declaration_comes_through_the_adapter_unchanged() {
    let rig = Rig::new();
    let instance = rig.instance("catalog", ModLimits::HOST).unwrap();
    assert_eq!(instance.line(), crate::Line::V01);
    let (_, declaration) = block_on(instance.declare()).unwrap();
    assert!(declaration.fluids.is_empty());
    assert!(declaration.reactions.is_empty() && declaration.combustion.is_empty());
    assert_eq!(declaration.materials.len(), 8);
    let (tables, _) = load(vec![("catalog", declaration)]).unwrap();
    assert_eq!(tables.fluids.iter().count(), 0, "no fluid row");
}

fn lake(level_m: f32, material: &str) -> content::WorldParams {
    content::WorldParams {
        movement: None,
        water: Some(content::WaterParams {
            level_m,
            material: material.into(),
        }),
        ground: Some(content::GroundParams {
            rock: "game:ground".into(),
            soil: "game:ground".into(),
            surface: "game:ground".into(),
        }),
        ..crate::convert::engine_world_params()
    }
}

/// The first `(mod id, declaration)` as the Game, then as [`load`], with the
/// lake `resolve` hands back.
fn load_game(
    mods: Vec<(&str, Declaration)>,
) -> Result<(Registry, Option<tesserae_core::WaterParams>), ModError> {
    let mut tables = Tables::builtin();
    let mut registry = Registry::builtin();
    let mut behaviours: Vec<Behaviour> = Vec::new();
    for (row, (id, mut declaration)) in mods.into_iter().enumerate() {
        if row == 0 && id == "game" {
            declaration.materials.push(material("ground", true));
        }
        let applied = apply(declaration, id, &mut tables, &mut registry, row == 0)?;
        assert!(applied.world.is_none_or(|w| w.water.is_none()));
        behaviours.push(applied.behaviour);
    }
    let (_, lake) = resolve(behaviours, &mut tables, &registry)?;
    Ok((registry, lake))
}

#[test]
fn a_games_lake_is_any_declared_fluid_whichever_mod_declares_it() {
    let game = Declaration {
        world: Some(lake(4.0, "lava:lava")),
        ..empty()
    };
    let (registry, lake) = load_game(vec![("game", game), ("lava", lava_mod())]).unwrap();
    assert_eq!(
        lake,
        Some(tesserae_core::WaterParams {
            level_m: 4.0,
            material: registry.id_of(Kind::Material, "lava:lava").unwrap() as u16,
        })
    );
}

#[test]
fn a_lake_that_is_unknown_solid_or_has_no_fluid_row_fails_the_load_naming_the_field() {
    for (name, rule) in [
        ("nobody:brine", "names no registered Material"),
        ("lava:basalt", "is solid, not a fluid"),
        ("game:still", "has no fluid row"),
    ] {
        let game = Declaration {
            materials: vec![material("still", false)],
            world: Some(lake(4.0, name)),
            ..empty()
        };
        let error = load_game(vec![("game", game), ("lava", lava_mod())])
            .err()
            .unwrap();
        assert!(
            matches!(&error, ModError::Lake { source, .. } if source.field == "water.material"),
            "{error:?}"
        );
        assert_eq!(
            error.to_string(),
            format!("mod `game`: world: water.material `{name}` {rule}")
        );
    }
}

/// The real 0.3 guest: its lake names the fluid it declares, and nothing is
/// warned of.
#[test]
fn the_lake_guest_fills_its_lakes_with_its_own_fluid() {
    let rig = Rig::new();
    let instance = rig.instance("lake", ModLimits::HOST).unwrap();
    assert_eq!(instance.line(), crate::Line::V03);
    let log = instance.log();
    let (_, declaration) = block_on(instance.declare()).unwrap();
    let (registry, lake) = load_game(vec![("lake", declaration)]).unwrap();
    let defaults = tesserae_core::WorldgenParams::default();
    assert_eq!(
        lake,
        Some(tesserae_core::WaterParams {
            level_m: defaults.sea_level_m + defaults.relief_m / 2.0,
            material: registry.id_of(Kind::Material, "lake:brine").unwrap() as u16,
        })
    );
    assert!(log.drain().lines.is_empty());
}

fn assert_warned_dry(log: &crate::LogHandle) {
    let lines = log.drain().lines;
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!(lines.iter().all(|line| line.level == crate::LogLevel::Warn));
    assert!(
        lines
            .iter()
            .any(|line| line.message.contains("no ground before contract line 0.3"))
    );
    assert!(lines.iter().any(|line| {
        line.message
            .contains("names no fluid before contract line 0.3")
            && line.message.contains("this World has no lakes")
    }));
}

/// A 0.2 Game's standing water names no fluid: it loads, says why its World
/// is dry, and has no lake.
#[test]
fn a_0_2_games_standing_water_is_dropped_with_a_warning() {
    let rig = Rig::new();
    let instance = rig.instance("pond", ModLimits::HOST).unwrap();
    assert_eq!(instance.line(), crate::Line::V02);
    let log = instance.log();
    let (_, declaration) = block_on(instance.declare()).unwrap();
    let world = declaration.world.clone().unwrap();
    assert!(world.water.is_none());
    assert_eq!(
        crate::convert::world_params(world).0,
        tesserae_core::WorldgenParams::default()
    );
    assert_eq!(load_game(vec![("pond", declaration)]).unwrap().1, None);
    assert_warned_dry(&log);
}

/// 0.1 goes up through the 0.2 adapter, so its standing water is dropped the
/// same way.
#[test]
fn a_0_1_games_standing_water_is_dropped_on_the_way_through_0_2() {
    use crate::bindings::{v01, v02};
    let mut world = v01::tesserae::mods::content::WorldParams::from(
        v02::tesserae::mods::content::WorldParams::from(crate::convert::engine_world_params()),
    );
    world.water = Some(v01::tesserae::mods::content::WaterParams { level_m: 4.0 });
    let old = v01::tesserae::mods::content::Declaration {
        materials: vec![],
        shapes: vec![],
        appearance: vec![],
        world: Some(world),
        view: None,
    };
    let log = crate::log::LogBuffer::shared();
    let declaration = crate::adapt::v02::declaration(old.into(), &log);
    assert!(declaration.world.unwrap().water.is_none());
    assert_warned_dry(&log);
}

#[test]
fn retired_core_material_names_and_ids_are_unknown_on_every_line() {
    use crate::bindings::v02::tesserae::mods::content as v02;
    let named = |declaration| {
        load(vec![("lava", lava_mod()), ("game", declaration)])
            .err()
            .unwrap()
            .to_string()
    };
    for name in [
        "stone", "dirt", "grass", "lantern", "glass", "leaves", "water",
    ] {
        let name = format!("core:{name}");
        let native = Declaration {
            reactions: vec![reaction("lava:lava", &name, None)],
            ..empty()
        };
        let old = v02::Declaration {
            materials: vec![],
            shapes: vec![],
            appearance: vec![],
            world: None,
            view: None,
            fluids: vec![v02::FluidDef {
                material: name.clone(),
                reach_m: 7.0,
                sideways_m_per_s: 4.0,
                downward_m_per_s: 8.0,
                recession_m_per_s: 4.0,
                reset_drop_m: 0.5,
                waterlogs: true,
                extinguishes: true,
                heat: None,
            }],
            reactions: vec![],
            combustion: vec![],
        };
        let log = crate::log::LogBuffer::shared();
        for (line, message, expected) in [
            (
                "0.3",
                named(native),
                format!("reaction 0 names Material `{name}`"),
            ),
            (
                "0.2",
                named(crate::adapt::v02::declaration(old, &log)),
                format!("fluid 0 names Material `{name}`"),
            ),
        ] {
            assert!(message.contains(&expected), "{line}: {message}");
        }
    }

    let tables = Tables::builtin();
    let at = |voxel| {
        vec![crate::Command::Place(vec![crate::Placement {
            at: [0.0; 3],
            voxel,
        }])]
    };
    for line in [crate::Line::V01, crate::Line::V02, crate::Line::V03] {
        for id in tesserae_core::material::RETIRED_IDS {
            assert_eq!(
                crate::validate(&at(tesserae_core::Voxel::cube(id)), line, &tables, 1.0),
                Err(crate::BatchError::Voxel {
                    command: 0,
                    fault: tesserae_core::PlaceFault::UnknownMaterial
                }),
                "{line:?} id {id}"
            );
        }
    }
}

#[test]
fn a_games_ground_resolves_after_all_declarations_and_generates_its_materials() {
    let mut game = empty();
    let mut world = crate::convert::engine_world_params();
    world.ground = Some(content::GroundParams {
        rock: "game:rock".into(),
        soil: "game:soil".into(),
        surface: "game:surface".into(),
    });
    game.world = Some(world);
    for name in ["rock", "soil", "surface"] {
        game.materials.push(content::MaterialDef {
            name: name.into(),
            emission: 0,
            dimming: 15,
            occlusion: content::Occlusion::Opaque,
            render: content::RenderClass::Opaque,
            remove_time_s: None,
            solid: true,
        });
    }
    let mut tables = Tables::builtin();
    let mut registry = Registry::builtin();
    let applied = apply(game, "game", &mut tables, &mut registry, true).unwrap();
    let (ground, water) = resolve(vec![applied.behaviour], &mut tables, &registry).unwrap();
    let ground = ground.unwrap();
    assert_eq!(
        [ground.rock, ground.soil, ground.surface],
        ["game:rock", "game:soil", "game:surface"]
            .map(|n| registry.id_of(Kind::Material, n).unwrap() as u16)
    );
    for size in [1.0, 0.5, 0.25] {
        let params = tesserae_core::WorldgenParams {
            ground: Some(ground),
            water,
            ..applied.world.unwrap()
        };
        let generator = tesserae_core::Worldgen::new(7, size, params);
        let y = (generator.surface_height_m(0.0, 0.0) / size / tesserae_core::CHUNK_SIZE as f32)
            .floor() as i32;
        let chunk = generator.chunk(0, y, 0);
        assert!(
            chunk.kinds().iter().all(|v| v.is_air()
                || [ground.rock, ground.soil, ground.surface].contains(&v.material()))
        );
        assert!(chunk.kinds().iter().any(|v| v.material() == ground.surface));
    }
}

#[test]
fn unknown_and_non_solid_ground_and_lakes_without_ground_fail_the_load() {
    for field in ["rock", "soil", "surface"] {
        for name in ["missing:ground", "core:air", "lava:lava"] {
            let mut params = lake(4.0, "lava:lava");
            let mut ground = params.ground.take().unwrap();
            match field {
                "rock" => ground.rock = name.into(),
                "soil" => ground.soil = name.into(),
                _ => ground.surface = name.into(),
            }
            params.ground = Some(ground);
            let game = Declaration {
                world: Some(params),
                ..empty()
            };
            let message = load_game(vec![("game", game), ("lava", lava_mod())])
                .unwrap_err()
                .to_string();
            assert!(message.contains(&format!("ground.{field}")), "{message}");
        }
    }
    let mut params = lake(4.0, "lava:lava");
    params.ground = None;
    let game = Declaration {
        world: Some(params),
        ..empty()
    };
    assert!(
        load_game(vec![("game", game), ("lava", lava_mod())])
            .unwrap_err()
            .to_string()
            .contains("requires ground")
    );
}
