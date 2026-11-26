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
            ("sounds/crack.ogg", super::sounds::MONO.to_vec()),
            ("sounds/wind.ogg", super::sounds::STEREO.to_vec()),
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
        contact_damage_per_s: None,
        tags: Vec::new(),
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

fn light(colour: (f32, f32, f32), strength: f32) -> content::CurveLight {
    content::CurveLight { colour, strength }
}

fn tint(colour: (f32, f32, f32), amount: f32) -> content::CurveTint {
    content::CurveTint { colour, amount }
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
            content::MaterialDef {
                contact_damage_per_s: Some(4.0),
                tags: Vec::new(),
                ..material("lava", 2, O::Never, R::Blend, false, 12, None)
            },
            material("wood", 15, O::SameMaterial, R::Cutout, true, 0, None),
            material("ash", 15, O::Opaque, R::Opaque, true, 0, None),
        ],
        shapes: vec![
            content::ShapeDef {
                form: Default::default(),
                name: "slab".into(),
                boxes: vec![node((0, 0, 0), (16, 8, 16))],
                collision: Some(vec![types::CollisionBox {
                    min: (0, 0, 0),
                    max: (16, 12, 16),
                }]),
                connect: None,
            },
            content::ShapeDef {
                form: Default::default(),
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
                variation: content::TileVariation::TurnedAndPatched,
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
                variation: content::TileVariation::Fixed,
            },
        ],
        world: Some(content::WorldParams {
            movement: Some(content::MovementPolicy {
                allow_flight: true,
                allow_physics: true,
                start_in_flight: false,
            }),
            map_policy: Some(content::MapPolicy { enabled: false }),
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
            lava: None,
            cave_biomes: None,
            sky: Some(content::SkyParams {
                day_length_s: 1200.0,
                start_hour: 7.0,
                day_share: 0.7,
                tilt_deg: 25.0,
                day_curve: Some(vec![
                    content::DayKey {
                        hour: 6.0,
                        sun: light((1.0, 0.55, 0.28), 1.0),
                        moon: light((0.62, 0.72, 1.0), 0.15),
                        ambient: light((0.95, 0.75, 0.65), 0.55),
                        sky_tint: tint((0.25, 0.18, 0.3), 0.4),
                        fog_tint: tint((1.0, 0.45, 0.18), 0.55),
                        stars: 0.2,
                    },
                    content::DayKey {
                        hour: 12.0,
                        sun: light((1.0, 1.0, 1.0), 1.0),
                        moon: light((0.62, 0.72, 1.0), 0.15),
                        ambient: light((1.0, 1.0, 1.0), 1.0),
                        sky_tint: tint((0.0, 0.0, 0.0), 0.0),
                        fog_tint: tint((0.0, 0.0, 0.0), 0.0),
                        stars: 0.0,
                    },
                ]),
                clouds: Some(content::CloudLayer {
                    altitude_m: 260.0,
                    thickness_m: 30.0,
                    cover: 0.4,
                    wind_direction_deg: 90.0,
                    wind_speed_m_per_s: 4.0,
                }),
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
            climate: Some(content::ClimateParams {
                temperature_size_m: 400.0,
                humidity_size_m: 640.0,
            }),
        }),
        view: Some(content::ViewParams {
            view_distance_m: 160.0,
            lod_distance_m: 3000.0,
            low_lod_distance_m: Some(2048.0),
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
        tools: vec![
            content::ToolDef {
                name: "ruler".into(),
                kind: content::ToolKind::Measure,
            },
            content::ToolDef {
                name: "builder".into(),
                kind: content::ToolKind::Place,
            },
        ],
        items: vec![
            content::ItemDef {
                name: "log".into(),
                size_m: 0.25,
                color: types::Srgba {
                    r: 120,
                    g: 80,
                    b: 40,
                    a: 255,
                },
                material: Some("full:wood".into()),
                max_stack: Some(16),
                food: None,
                texture: None,
            },
            content::ItemDef {
                name: "pebble".into(),
                size_m: 0.1,
                color: types::Srgba {
                    r: 150,
                    g: 150,
                    b: 150,
                    a: 255,
                },
                material: None,
                max_stack: None,
                food: Some(2.5),
                texture: None,
            },
        ],
        biomes: vec![
            content::BiomeDef {
                soundscape: None,
                selectors: None,
                strata: None,
                name: "dry".into(),
                temperature: 0.8,
                humidity: 0.15,
                ground: None,
                sky_fog: Some(content::SkyFog {
                    sky: (176, 160, 128),
                    fog: (196, 178, 142),
                }),
                cave_scale: Some(0.0),
                cloud_cover: Some(0.5),
                surface_features: Vec::new(),
                crop_fields: Vec::new(),
                structures: Vec::new(),
                settlements: Vec::new(),
                loot: None,
            },
            content::BiomeDef {
                soundscape: None,
                selectors: None,
                strata: None,
                name: "wet".into(),
                temperature: 0.35,
                humidity: 0.9,
                ground: Some(content::GroundParams {
                    rock: "full:rock".into(),
                    soil: "full:ash".into(),
                    surface: "full:wood".into(),
                }),
                sky_fog: None,
                cave_scale: Some(1.5),
                cloud_cover: None,
                surface_features: vec![content::SurfaceFeature {
                    material: Some("full:wood".into()),
                    plant: None,
                    shape: None,
                    spacing_m: 24.0,
                    clump: Some(content::Clump {
                        scale_m: 48.0,
                        strength: 0.5,
                    }),
                    field: Some(content::FieldDef {
                        group: "beds".into(),
                        scale_m: 24.0,
                        strays: 0.1,
                    }),
                }],
                crop_fields: Vec::new(),
                structures: Vec::new(),
                settlements: Vec::new(),
                loot: None,
            },
        ],
        ores: vec![
            content::OreDef {
                material: "full:ash".into(),
                min_depth_m: 4.0,
                max_depth_m: 40.0,
                vein_size_m: 3.0,
                rarity: 0.05,
                biomes: vec!["full:dry".into()],
            },
            content::OreDef {
                material: "full:wood".into(),
                min_depth_m: 0.0,
                max_depth_m: 12.0,
                vein_size_m: 1.5,
                rarity: 1.0,
                biomes: vec![],
            },
        ],
        survival: Some(content::SurvivalParams {
            max_health: Some(30.0),
            max_hunger: None,
            max_air_s: None,
            safe_fall_m: Some(4.5),
            fall_damage_per_m: None,
            drowning_damage_per_s: None,
            hunger_per_min: None,
            sprinting_hunger_per_min: None,
            regeneration_hunger: None,
            regeneration_per_s: Some(0.5),
            starvation_damage_per_s: None,
            keep_inventory: Some(true),
        }),
        sounds: vec![
            content::SoundDef {
                name: "crack".into(),
                recordings: [3, 1]
                    .map(|weight| content::Recording {
                        bytes: super::sounds::MONO.to_vec(),
                        weight,
                    })
                    .into(),
                volume: content::SoundRange { min: 0.8, max: 1.0 },
                pitch: content::SoundRange { min: 0.9, max: 1.1 },
                category: content::SoundCategory::Effects,
                distance: Some(content::SoundDistance {
                    reference_m: 2.0,
                    max_m: 24.0,
                }),
            },
            content::SoundDef {
                name: "wind".into(),
                recordings: vec![content::Recording {
                    bytes: super::sounds::STEREO.to_vec(),
                    weight: 1,
                }],
                volume: content::SoundRange { min: 1.0, max: 1.0 },
                pitch: content::SoundRange { min: 1.0, max: 1.0 },
                category: content::SoundCategory::Ambient,
                distance: None,
            },
            content::SoundDef {
                name: "tune".into(),
                recordings: vec![content::Recording {
                    bytes: super::sounds::STEREO.to_vec(),
                    weight: 1,
                }],
                volume: content::SoundRange { min: 1.0, max: 1.0 },
                pitch: content::SoundRange { min: 1.0, max: 1.0 },
                category: content::SoundCategory::Music,
                distance: None,
            },
        ],
        sound_sets: vec![content::SoundSetDef {
            name: "rock".into(),
            break_: Some("full:crack".into()),
            place: None,
            hit: None,
            step: Some("full:crack".into()),
            land: None,
        }],
        material_sounds: vec![content::MaterialSound {
            material: "full:rock".into(),
            sound_set: "full:rock".into(),
        }],
        soundscapes: vec![
            content::SoundscapeDef {
                name: "surface".into(),
                context: Some(content::SoundscapeContext::Default),
                beds: vec![content::AmbientBed {
                    sound: "full:wind".into(),
                    volume: 0.6,
                }],
                music: vec!["full:tune".into()],
                silence_s: None,
            },
            content::SoundscapeDef {
                name: "cave".into(),
                context: Some(content::SoundscapeContext::Underground),
                beds: Vec::new(),
                music: vec!["full:tune".into()],
                silence_s: Some(content::SoundRange {
                    min: 20.0,
                    max: 40.0,
                }),
            },
        ],
        cause_sounds: [content::SoundCause::Hurt, content::SoundCause::UseMeasured]
            .map(|cause| content::CauseSound {
                cause,
                sound: "full:crack".into(),
            })
            .into(),
        recipes: vec![content::RecipeDef {
            inputs: vec![
                content::RecipeStack {
                    item: "full:log".into(),
                    count: 2,
                },
                content::RecipeStack {
                    item: "full:pebble".into(),
                    count: 1,
                },
            ],
            output: content::RecipeStack {
                item: "full:log".into(),
                count: 16,
            },
            station: "hand".into(),
        }],
        plants: Vec::new(),
        marker_kinds: Vec::new(),
        death_marker_kind: None,
        templates: vec![content::TemplateDef {
            name: "cairn".into(),
            cell_m: 0.5,
            palette: vec![
                content::TemplatePaletteEntry {
                    symbol: '.',
                    cell: content::TemplateCell::Air,
                },
                content::TemplatePaletteEntry {
                    symbol: '_',
                    cell: content::TemplateCell::Keep,
                },
                content::TemplatePaletteEntry {
                    symbol: 'r',
                    cell: content::TemplateCell::Voxel(content::TemplateVoxel {
                        material: "full:rock".into(),
                        shape: Some("full:slab".into()),
                        orientation: Some(5),
                    }),
                },
            ],
            layers: vec![vec!["r.".into(), "_r".into()]],
            ground_layer: Some(0),
            foundation: Some("full:rock".into()),
            level_tolerance_m: Some(2.0),
            spans: false,
        }],
        settlements: vec![content::SettlementDef {
            name: "hamlet".into(),
            radius_m: 24.0,
            radius_min_m: Some(16.0),
            margin_m: Some(8.0),
            centre: Some("full:cairn".into()),
            plots: 4,
            plots_min: Some(2),
            templates: vec![content::SettlementPlot {
                template: "full:cairn".into(),
                weight: 2.0,
                min: 1,
                max: 3,
                loot: None,
            }],
            minimum: 2,
            path: "full:rock".into(),
            path_shape: Some("full:slab".into()),
            loot: None,
            outskirts: None,
        }],
        voxel_uses: vec![
            content::VoxelUse {
                material: "full:wood".into(),
                kind: content::VoxelUseKind::Door(content::DoorDef {
                    open: "full:ash".into(),
                    width_m: Some(1.5),
                    height_m: None,
                }),
                shape: None,
            },
            content::VoxelUse {
                material: "full:rock".into(),
                kind: content::VoxelUseKind::Station("bench".into()),
                shape: Some("core:slab".into()),
            },
        ],
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
    assert_eq!(data.map, native.map);
    assert_eq!(data.view, native.view);
    assert_eq!(data.survival, native.survival);
    assert_eq!(data.contact_damage, native.contact_damage);
    assert_eq!(data.world, native.world);
    assert_eq!(
        data.world.unwrap().climate,
        tesserae_core::ClimateParams {
            temperature_size_m: 400.0,
            humidity_size_m: 640.0
        }
    );
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
    assert!(d.items.is_empty() && d.survival.is_none() && d.recipes.is_empty());
    assert!(world.climate.is_none() && d.biomes.is_empty() && d.ores.is_empty());
    assert_eq!(d.materials[0].remove_time_s, None);
    assert_eq!(d.materials[0].contact_damage_per_s, None);
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
        ("size-m = 0.25", "size-m = 0.25\nweight = 3", "weight"),
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
        ("[world.climate]", "[world.weather]", "weather"),
        ("stars = 0.2", "stars = 0.2\nglow = 1.0", "glow"),
        ("cover = 0.4", "cover = 0.4\nrain = 1", "rain"),
        (
            "temperature-size-m = 400.0",
            "temperature-size-m = 400.0\nwind = 1",
            "wind",
        ),
        ("name = \"dry\"", "name = \"dry\"\nextra = 1", "extra"),
        ("humidity = 0.9", "humidity = 0.9\nrain = 3", "rain"),
        (
            "soil = \"full:ash\"",
            "soil = \"full:ash\", peat = 1",
            "peat",
        ),
        ("soil = \"full:ash\", ", "", "soil"),
        ("fog = [196, 178, 142]", "fog = [196, 178, 142, 9]", "fog"),
        ("fog = [196, 178, 142]", "fog = [196, 178, 300]", "fog"),
        ("sky = [176, 160, 128], ", "", "sky"),
        (
            "fog = [196, 178, 142]",
            "fog = [196, 178, 142], haze = 1",
            "haze",
        ),
        ("rarity = 0.05", "rarity = 0.05\nluck = 1", "luck"),
        ("max-health = 30", "max-health = 30\nmana = 5", "mana"),
        (
            "keep-inventory = true",
            "keep-inventory = 1",
            "keep-inventory",
        ),
        ("max-stack = 16", "max-stack = -1", "max-stack"),
        ("food = 2.5", "food = \"bread\"", "food"),
        ("rarity = 0.05", "rarity = \"rare\"", "rarity"),
        (
            "biomes = [\"full:dry\"]",
            "biomes = [\"full:dry\", 3]",
            "biomes",
        ),
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
            "temperature-size-m = 400.0",
            "temperature-size-m = 0.0",
            "climate",
        ),
        ("humidity = 0.15", "humidity = 1.5", "biome"),
        ("temperature = 0.8", "temperature = -0.1", "biome"),
        ("name = \"wet\"", "name = \"dry\"", "biome"),
        ("name = \"wet\"", "name = \"Wet\"", "biome"),
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
        ("max-stack = 16", "max-stack = 65", "max-stack"),
        ("food = 2.5", "food = 0", "food"),
        (
            "contact-damage-per-s = 4.0",
            "contact-damage-per-s = -1",
            "contact-damage-per-s",
        ),
        ("max-health = 30", "max-health = 0", "max-health"),
        ("safe-fall-m = 4.5", "safe-fall-m = 1001", "safe-fall-m"),
        ("stars = 0.2", "stars = 1.5", "sky.day_curve[0].stars"),
        ("hour = 12.0", "hour = 5.0", "sky.day_curve[1].hour"),
        (
            "altitude-m = 260.0",
            "altitude-m = 200.0",
            "sky.clouds.altitude_m",
        ),
        (
            "thickness-m = 30.0",
            "thickness-m = 0.5",
            "sky.clouds.thickness_m",
        ),
        ("cover = 0.4", "cover = 1.2", "sky.clouds.cover"),
        (
            "wind-speed-m-per-s = 4.0",
            "wind-speed-m-per-s = 60.0",
            "sky.clouds.wind_speed_m_per_s",
        ),
        (
            "cloud-cover = 0.5",
            "cloud-cover = 5.0",
            "biomes.cloud_cover",
        ),
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
        ("[survival]\nkeep-inventory = true", "survival"),
        (
            "[world.movement]\nallow-flight = true\nallow-physics = true\nstart-in-flight = false",
            "world.movement",
        ),
        ("[world.map]\nenabled = false", "world.map"),
        ("[world]\nrelief-m = 10.0", "world.relief-m"),
        (
            "[world.climate]\ntemperature-size-m = 100\nhumidity-size-m = 100",
            "world.climate",
        ),
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
        (0..129)
            .map(|_| format!("[[{table}]]\n{row}\n"))
            .collect::<String>()
    };
    for (table, row, what) in [
        ("tools", "name = 'a'\nkind = 'measure'", "tool"),
        (
            "biomes",
            "name = 'a'\ntemperature = 0.5\nhumidity = 0.5",
            "biome",
        ),
        (
            "ores",
            "material = 'full:x'\nmin-depth-m = 0\nmax-depth-m = 1\nvein-size-m = 1\nrarity = 1",
            "ore",
        ),
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

#[test]
fn both_formats_resolve_the_same_sounds() {
    let (_, data_registry, data) = load(parse(&full()).unwrap());
    let (_, registry, native) = load(component());
    let data = crate::resolve_sounds(&[data.behaviour], &data_registry).unwrap();
    let native = crate::resolve_sounds(&[native.behaviour], &registry).unwrap();
    assert_eq!(data, native);
    assert_eq!(data.sounds.len(), 3);
    assert_eq!(data.sounds[0].def.weights, [3, 1]);
    assert_eq!(data.material_sounds.len(), 1);
    assert_eq!(data.soundscapes.len(), 2);
    assert_eq!(data.soundscapes[1].def.silence_s.max, 40.0);
    assert_eq!(
        data.cause_sounds,
        [
            (
                tesserae_core::audio::Cause::Hurt,
                tesserae_core::audio::SoundId(0)
            ),
            (
                tesserae_core::audio::Cause::UseMeasured,
                tesserae_core::audio::SoundId(0)
            ),
        ]
    );
}

#[test]
fn data_biome_can_name_a_soundscape() {
    let declaration =
        parse("[[biomes]]\nname='dry'\ntemperature=0.8\nhumidity=0.2\nsoundscape='my-mod:wind'\n")
            .unwrap();
    assert_eq!(
        declaration.biomes[0].soundscape.as_deref(),
        Some("my-mod:wind")
    );
}

#[test]
fn data_biome_can_declare_crop_fields_and_a_field_group() {
    let declaration = parse(
        "[[biomes]]\nname='dry'\ntemperature=0.8\nhumidity=0.2\n\
         surface-features=[{ plant='a:b', spacing-m=4.0, field={ group='g', scale-m=20.0, strays=0.1 } }]\n\
         crop-fields=[{ crops=[['a:wheat', 6.0], ['a:rye', 1.0]], plot-m=[8.0, 14.0], spacing-m=64.0, \
         path='a:sand', ditch='a:water', near-water-m=32.0 }, { crops=[['a:wheat', 1.0]], \
         plot-m=[6.0, 6.0], spacing-m=32.0 }]\n",
    )
    .unwrap();
    let biome = &declaration.biomes[0];
    let field = biome.surface_features[0].field.as_ref().unwrap();
    assert_eq!(
        (field.group.as_str(), field.scale_m, field.strays),
        ("g", 20.0, 0.1)
    );
    let [first, second] = &biome.crop_fields[..] else {
        panic!("two crop fields");
    };
    assert_eq!(
        first.crops,
        [("a:wheat".to_owned(), 6.0), ("a:rye".to_owned(), 1.0)]
    );
    assert_eq!((first.plot_m, first.spacing_m), ((8.0, 14.0), 64.0));
    assert_eq!(first.path.as_deref(), Some("a:sand"));
    assert_eq!(first.ditch.as_deref(), Some("a:water"));
    assert_eq!(first.near_water_m, Some(32.0));
    assert_eq!(
        (&second.path, &second.ditch, second.near_water_m),
        (&None, &None, None)
    );
}

#[test]
fn water_motion_causes_parse_from_data() {
    let declaration = parse("[[cause-sounds]]\ncause='swimming'\nsound='full:crack'\n[[cause-sounds]]\ncause='floating'\nsound='full:crack'\n").unwrap();
    assert_eq!(
        declaration.cause_sounds[0].cause,
        content::SoundCause::Swimming
    );
    assert_eq!(
        declaration.cause_sounds[1].cause,
        content::SoundCause::Floating
    );
}

#[test]
fn use_and_craft_causes_parse_from_data() {
    use content::SoundCause as C;
    let names = [
        ("door-open", C::DoorOpen),
        ("door-close", C::DoorClose),
        ("container-open", C::ContainerOpen),
        ("container-close", C::ContainerClose),
        ("craft", C::Craft),
    ];
    let rows: String = names
        .iter()
        .map(|(name, _)| format!("[[cause-sounds]]\ncause='{name}'\nsound='full:thud'\n"))
        .collect();
    let declaration = parse(&rows).unwrap();
    let parsed: Vec<_> = declaration.cause_sounds.iter().map(|c| c.cause).collect();
    assert_eq!(parsed, names.map(|(_, cause)| cause));
}

#[test]
fn burn_cause_parses_from_data() {
    let declaration = parse("[[cause-sounds]]\ncause='burn'\nsound='full:fire'\n").unwrap();
    assert_eq!(declaration.cause_sounds[0].cause, content::SoundCause::Burn);
}

#[test]
fn a_container_row_parses_from_data_with_or_without_slots() {
    let declaration = parse(
        "[[voxel-uses]]\nkind='container'\nmaterial='full:chest'\n\
         [[voxel-uses]]\nkind='container'\nmaterial='full:vault'\nslots=54\n",
    )
    .unwrap();
    let kinds: Vec<_> = declaration.voxel_uses.iter().map(|u| &u.kind).collect();
    assert_eq!(
        kinds,
        [
            &content::VoxelUseKind::Container(None),
            &content::VoxelUseKind::Container(Some(54))
        ]
    );
}
