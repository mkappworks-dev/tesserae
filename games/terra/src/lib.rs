wit_bindgen::generate!({ path: "../../crates/tesserae-mods/wit/0.3", world: "mod" });

use exports::tesserae::mods::guest::{
    Command, Declaration, Guest, Registered, TickContext, WorldInfo,
};
mod audio;
mod building;
mod character;
mod landforms;
mod plants;
mod structures;
mod templates;
mod textures;

use tesserae::mods::content::{
    Appearance, BiomeDef, BiomeSelectors, BiomeStratum, BurnAfter, BurnEnd, ClimateParams,
    CombustionDef, FluidDef, FluidState, GroundParams, HeatDef, ItemDef, MarkerIcon, MarkerKindDef,
    MaterialDef, Occlusion, OreDef, Reactant, ReactionDef, ReactionSide, RecipeDef, RecipeStack,
    RenderClass, SelectorRange, ShapeDef, SkyFog, SurfaceFeature, ToolDef, ToolKind, ViewParams,
    VoxelUse, VoxelUseKind, WaterParams,
};
use tesserae::mods::types::{Arm, CollisionBox, ConnectDef, NodeBox, Srgba};

/// The door's size: whole Template cells, so a Template's opening fits it at every Voxel size, with
/// room round a Player's 0.6 m by 1.8 m body.
const DOOR_WIDTH_M: f32 = 2.0;
const DOOR_HEIGHT_M: f32 = 3.0;

/// Ground spans `SEA_LEVEL_M` to `SEA_LEVEL_M + RELIEF_M`; lakes stand at 5/12
/// of the relief, the engine's default, so valleys hold water at any seed.
const SEA_LEVEL_M: f32 = 24.0;
const RELIEF_M: f32 = 24.0;
const LAKE_M: f32 = SEA_LEVEL_M + RELIEF_M * 5.0 / 12.0;

struct Terra;

pub(crate) fn solid(name: &str) -> MaterialDef {
    MaterialDef {
        name: name.into(),
        emission: 0,
        dimming: 15,
        occlusion: Occlusion::Opaque,
        render: RenderClass::Opaque,
        remove_time_s: Some(0.35),
        contact_damage_per_s: None,
        tags: tags_of(name),
        solid: true,
    }
}

/// What the shared Plants name instead of Terra's own ground and water.
fn tags_of(name: &str) -> Vec<String> {
    let tags: &[&str] = match name {
        "dirt" | "grass" | "snow" => &["soil"],
        "sand" => &["sand"],
        "stone" => &["rock"],
        _ => &[],
    };
    tags.iter().map(|t| (*t).to_owned()).collect()
}

fn fence() -> ShapeDef {
    let arm = |face, min, max, collision_min, collision_max| Arm {
        face,
        boxes: vec![NodeBox { min, max }],
        collision: Some(vec![CollisionBox {
            min: collision_min,
            max: collision_max,
        }]),
    };
    ShapeDef {
        form: Default::default(),
        name: "fence".into(),
        boxes: vec![NodeBox {
            min: (6, 0, 6),
            max: (10, 16, 10),
        }],
        collision: Some(vec![CollisionBox {
            min: (6, 0, 6),
            max: (10, 24, 10),
        }]),
        connect: Some(ConnectDef {
            group: 1,
            connects_to: 1,
            to_sealed: true,
            arms: vec![
                arm(0, (0, 7, 7), (6, 13, 9), (0, 0, 7), (6, 24, 9)),
                arm(1, (10, 7, 7), (16, 13, 9), (10, 0, 7), (16, 24, 9)),
                arm(4, (7, 7, 0), (9, 13, 6), (7, 0, 0), (9, 24, 6)),
                arm(5, (7, 7, 10), (9, 13, 16), (7, 0, 10), (9, 24, 16)),
            ],
        }),
    }
}

/// A Door's panel, 3/16 thick against its own +Z, which the engine turns to
/// face the Player and Use turns about its hinge.
fn panel() -> ShapeDef {
    ShapeDef {
        form: Default::default(),
        name: "panel".into(),
        boxes: vec![NodeBox {
            min: (0, 0, 13),
            max: (16, 16, 16),
        }],
        collision: None,
        connect: None,
    }
}

/// A Workbench's: four legs under a thick top. A body collides with the whole
/// cell, so nothing snags between the legs.
fn table() -> ShapeDef {
    let leg = |x, z| NodeBox {
        min: (x, 0, z),
        max: (x + 3, 12, z + 3),
    };
    ShapeDef {
        form: Default::default(),
        name: "table".into(),
        boxes: vec![
            leg(1, 1),
            leg(12, 1),
            leg(1, 12),
            leg(12, 12),
            NodeBox {
                min: (0, 12, 0),
                max: (16, 16, 16),
            },
        ],
        collision: Some(vec![CollisionBox {
            min: (0, 0, 0),
            max: (16, 16, 16),
        }]),
        connect: None,
    }
}

/// Cut out between its rails, so what stands behind a Gate shows through.
fn gate(name: &str) -> MaterialDef {
    MaterialDef {
        dimming: 0,
        occlusion: Occlusion::Never,
        render: RenderClass::Cutout,
        ..solid(name)
    }
}

fn marker(name: &str, icon: MarkerIcon, [r, g, b]: [u8; 3]) -> MarkerKindDef {
    MarkerKindDef {
        name: name.into(),
        icon,
        color: Srgba { r, g, b, a: 255 },
    }
}

pub(crate) fn appearance(material: &str, [r, g, b, a]: [u8; 4]) -> Appearance {
    Appearance {
        material: material.into(),
        color: Srgba { r, g, b, a },
        texture: Some(textures::texture(material, [r, g, b, a])),
    }
}

impl Guest for Terra {
    fn declare() -> Declaration {
        let mut world = tesserae::mods::host::default_world_params();
        world.movement = Some(tesserae::mods::content::MovementPolicy {
            allow_flight: true,
            allow_physics: true,
            start_in_flight: false,
        });
        world.character = Some(character::character());
        world.sea_level_m = SEA_LEVEL_M;
        world.relief_m = RELIEF_M;
        world.water = Some(WaterParams {
            level_m: LAKE_M,
            material: "terra:water".into(),
        });
        world.ground = Some(GroundParams {
            rock: "terra:stone".into(),
            soil: "terra:dirt".into(),
            surface: "terra:grass".into(),
        });
        world.lava = Some("terra:lava".into());
        world.cave_biomes = Some(landforms::caves());
        // Smaller than the engine's 512 m, so a walk across the stage meets a border.
        world.climate = Some(ClimateParams {
            temperature_size_m: 256.0,
            humidity_size_m: 256.0,
        });
        Declaration {
            materials: vec![
                MaterialDef {
                    name: "lamp".into(),
                    emission: 15,
                    dimming: 15,
                    occlusion: Occlusion::Opaque,
                    render: RenderClass::Opaque,
                    remove_time_s: None,
                    contact_damage_per_s: None,
                    tags: Vec::new(),
                    solid: true,
                },
                MaterialDef {
                    name: "lava".into(),
                    emission: 15,
                    dimming: 15,
                    occlusion: Occlusion::Opaque,
                    render: RenderClass::Opaque,
                    remove_time_s: None,
                    contact_damage_per_s: Some(4.0),
                    tags: Vec::new(),
                    solid: false,
                },
                solid("wood"),
                solid("ash"),
                // Murky rather than opaque: light is one value per Voxel, so a
                // fluid that stops it and gives none would draw its surfaces,
                // and the Shapes it soaks into, unlit.
                MaterialDef {
                    name: "mud".into(),
                    emission: 0,
                    dimming: 6,
                    occlusion: Occlusion::SameMaterial,
                    render: RenderClass::Blend,
                    remove_time_s: None,
                    contact_damage_per_s: None,
                    tags: Vec::new(),
                    solid: false,
                },
                solid("brick"),
                // Clear: sky light reaches the floor of a version 3 sea tens of metres down, and the
                // underwater fog, which follows dimming, reaches its farthest.
                MaterialDef {
                    name: "water".into(),
                    emission: 0,
                    dimming: 0,
                    occlusion: Occlusion::SameMaterial,
                    render: RenderClass::Blend,
                    remove_time_s: None,
                    contact_damage_per_s: None,
                    tags: vec!["water".into()],
                    solid: false,
                },
                solid("stone"),
                solid("dirt"),
                solid("grass"),
                MaterialDef {
                    dimming: 0,
                    occlusion: Occlusion::SameMaterial,
                    render: RenderClass::Blend,
                    ..solid("glass")
                },
                MaterialDef {
                    dimming: 1,
                    occlusion: Occlusion::Never,
                    render: RenderClass::Cutout,
                    ..solid("leaves")
                },
                solid("sand"),
                solid("snow"),
                solid("iron"),
                // Veins deep in the rock that a cavern or shaft lays open: the light down there.
                MaterialDef {
                    emission: 10,
                    ..solid("crystal")
                },
                solid("melon"),
                solid("workbench"),
                solid("chest"),
                solid("door"),
                solid("door_open"),
                gate("gate"),
                gate("gate_open"),
            ]
            .into_iter()
            .chain(building::materials())
            .collect(),
            shapes: vec![fence(), panel(), table()],
            appearance: vec![
                appearance("lamp", [245, 227, 184, 255]),
                appearance("lava", [230, 72, 16, 255]),
                appearance("wood", [150, 104, 60, 255]),
                appearance("ash", [70, 66, 62, 255]),
                appearance("mud", [92, 70, 44, 224]),
                appearance("brick", [168, 74, 52, 255]),
                appearance("water", [26, 82, 158, 166]),
                appearance("stone", [133, 133, 140, 255]),
                appearance("dirt", [107, 74, 46, 255]),
                appearance("grass", [84, 143, 61, 255]),
                appearance("glass", [164, 210, 250, 89]),
                appearance("leaves", [41, 107, 36, 255]),
                appearance("sand", [214, 196, 140, 255]),
                appearance("snow", [236, 240, 246, 255]),
                appearance("iron", [150, 120, 104, 255]),
                appearance("crystal", [150, 110, 230, 255]),
                appearance("melon", [84, 150, 56, 255]),
                appearance("workbench", [214, 190, 140, 255]),
                appearance("chest", [128, 62, 40, 255]),
                appearance("door", [128, 88, 50, 255]),
                appearance("door_open", [128, 88, 50, 255]),
                appearance("gate", [140, 97, 55, 255]),
                appearance("gate_open", [140, 97, 55, 255]),
            ]
            .into_iter()
            .chain(building::appearance())
            .collect(),
            world: Some(world),
            view: Some(ViewParams {
                view_distance_m: 160.0,
                lod_distance_m: 3000.0,
            }),
            // Water first: registration order decides an otherwise equal claim.
            fluids: vec![
                FluidDef {
                    material: "terra:water".into(),
                    reach_m: 7.0,
                    sideways_m_per_s: 4.0,
                    downward_m_per_s: 8.0,
                    recession_m_per_s: 4.0,
                    reset_drop_m: 0.5,
                    waterlogs: true,
                    extinguishes: true,
                    heat: None,
                },
                // Slower and shorter than water, and hot; it stays out of Shapes.
                FluidDef {
                    material: "terra:lava".into(),
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
                },
                // Faster than lava and slower than water; it soaks into Shapes
                // and smothers fire as water does, but goes only 2 m.
                FluidDef {
                    material: "terra:mud".into(),
                    reach_m: 2.0,
                    sideways_m_per_s: 1.0,
                    downward_m_per_s: 2.0,
                    recession_m_per_s: 1.0,
                    reset_drop_m: 0.5,
                    waterlogs: true,
                    extinguishes: true,
                    heat: None,
                },
            ],
            reactions: vec![
                // Lava that touches water cools to stone; the water is kept.
                ReactionDef {
                    first: Reactant {
                        fluid: "terra:lava".into(),
                        state: FluidState::Any,
                    },
                    second: Reactant {
                        fluid: "terra:water".into(),
                        state: FluidState::Any,
                    },
                    consumed: ReactionSide::First,
                    product: Some("terra:stone".into()),
                    receiving: None,
                },
                // Mud that touches lava fires to brick; here the lava is kept.
                ReactionDef {
                    first: Reactant {
                        fluid: "terra:mud".into(),
                        state: FluidState::Any,
                    },
                    second: Reactant {
                        fluid: "terra:lava".into(),
                        state: FluidState::Any,
                    },
                    consumed: ReactionSide::First,
                    product: Some("terra:brick".into()),
                    receiving: None,
                },
            ],
            // Wood catches 2 s after heat reaches it, burns for 8 s, lighting
            // and heating 2 m round it, and leaves ash, which does not burn.
            combustion: vec![CombustionDef {
                fuel: "terra:wood".into(),
                ignition_threshold: 6,
                ignition_delay_s: 2.0,
                emission: 12,
                heat: Some(HeatDef {
                    strength: 12,
                    reach_m: 2.0,
                }),
                end: BurnEnd::After(BurnAfter {
                    seconds: 8.0,
                    residue: Some("terra:ash".into()),
                }),
            }],
            tools: vec![
                ToolDef {
                    name: "measure".into(),
                    kind: ToolKind::Measure,
                },
                ToolDef {
                    name: "place".into(),
                    kind: ToolKind::Place,
                },
            ],
            items: vec![
                ItemDef {
                    name: "rock".into(),
                    size_m: 0.25,
                    color: Srgba {
                        r: 133,
                        g: 133,
                        b: 140,
                        a: 255,
                    },
                    material: Some("terra:stone".into()),
                    max_stack: None,
                    food: None,
                    texture: None,
                },
                ItemDef {
                    name: "dirt".into(),
                    size_m: 0.25,
                    color: Srgba {
                        r: 107,
                        g: 74,
                        b: 46,
                        a: 255,
                    },
                    material: Some("terra:dirt".into()),
                    max_stack: None,
                    food: None,
                    texture: None,
                },
                ItemDef {
                    name: "grass".into(),
                    size_m: 0.25,
                    color: Srgba {
                        r: 84,
                        g: 143,
                        b: 61,
                        a: 255,
                    },
                    material: Some("terra:grass".into()),
                    max_stack: None,
                    food: None,
                    texture: None,
                },
                ItemDef {
                    name: "melon".into(),
                    size_m: 0.25,
                    color: Srgba {
                        r: 84,
                        g: 150,
                        b: 56,
                        a: 255,
                    },
                    material: Some("terra:melon".into()),
                    max_stack: None,
                    food: Some(6.0),
                    texture: None,
                },
                ItemDef {
                    name: "workbench".into(),
                    size_m: 0.25,
                    color: Srgba {
                        r: 167,
                        g: 116,
                        b: 63,
                        a: 255,
                    },
                    material: Some("terra:workbench".into()),
                    max_stack: None,
                    food: None,
                    texture: None,
                },
                ItemDef {
                    name: "chest".into(),
                    size_m: 0.25,
                    color: Srgba {
                        r: 107,
                        g: 68,
                        b: 35,
                        a: 255,
                    },
                    material: Some("terra:chest".into()),
                    max_stack: None,
                    food: None,
                    texture: None,
                },
                ItemDef {
                    name: "door".into(),
                    size_m: 0.25,
                    color: Srgba {
                        r: 128,
                        g: 88,
                        b: 50,
                        a: 255,
                    },
                    material: Some("terra:door".into()),
                    max_stack: None,
                    food: None,
                    texture: None,
                },
                ItemDef {
                    name: "gate".into(),
                    size_m: 0.25,
                    color: Srgba {
                        r: 140,
                        g: 97,
                        b: 55,
                        a: 255,
                    },
                    material: Some("terra:gate".into()),
                    max_stack: None,
                    food: None,
                    texture: None,
                },
            ],
            biomes: vec![
                BiomeDef {
                    soundscape: Some("terra:temperate".into()),
                    selectors: None,
                    strata: bedding("temperate"),
                    name: "temperate".into(),
                    temperature: 0.38,
                    humidity: 0.59,
                    ground: None,
                    sky_fog: Some(SkyFog {
                        sky: (126, 146, 168),
                        fog: (136, 160, 190),
                    }),
                    cave_scale: None,
                    // A melon in every 16 by 16 m of grass is found while exploring, not a
                    // carpet; the oaks every 9 m give Wood for a Workbench, Chest and Door in a
                    // short walk, and no Wood lies about on its own.
                    crop_fields: plants::temperate_fields(),
                    structures: [
                        structures::wild(&["watchtower", "standing_stones", "waystone"]),
                        structures::bridge("stone_bridge"),
                    ]
                    .concat(),
                    settlements: structures::town_and_village(),
                    surface_features: [SurfaceFeature {
                        material: Some("terra:melon".into()),
                        plant: None,
                        shape: None,
                        spacing_m: 16.0,
                        clump: None,
                        field: None,
                    }]
                    .into_iter()
                    .chain(plants::temperate())
                    .collect(),
                    loot: Some(structures::biome_loot("temperate")),
                },
                BiomeDef {
                    soundscape: Some("terra:dry".into()),
                    selectors: None,
                    strata: bedding("dry"),
                    name: "dry".into(),
                    temperature: 0.54,
                    humidity: 0.42,
                    ground: Some(GroundParams {
                        rock: "terra:stone".into(),
                        soil: "terra:sand".into(),
                        surface: "terra:sand".into(),
                    }),
                    sky_fog: Some(SkyFog {
                        sky: (176, 160, 128),
                        fog: (196, 178, 142),
                    }),
                    cave_scale: Some(1.5),
                    crop_fields: Vec::new(),
                    structures: [
                        structures::wild(&["ruined_keep"]),
                        structures::bridge("stone_bridge"),
                    ]
                    .concat(),
                    settlements: structures::village(),
                    surface_features: plants::dry(),
                    loot: Some(structures::biome_loot("dry")),
                },
                BiomeDef {
                    soundscape: Some("terra:cold".into()),
                    selectors: None,
                    strata: bedding("cold"),
                    name: "cold".into(),
                    temperature: 0.29,
                    humidity: 0.55,
                    ground: Some(GroundParams {
                        rock: "terra:stone".into(),
                        soil: "terra:dirt".into(),
                        surface: "terra:snow".into(),
                    }),
                    sky_fog: Some(SkyFog {
                        sky: (150, 164, 184),
                        fog: (176, 190, 208),
                    }),
                    cave_scale: Some(0.0),
                    crop_fields: Vec::new(),
                    structures: [
                        structures::wild(&["watchtower", "ruined_keep"]),
                        structures::bridge("stone_bridge"),
                    ]
                    .concat(),
                    settlements: structures::village(),
                    surface_features: plants::cold(),
                    loot: Some(structures::biome_loot("cold")),
                },
                BiomeDef {
                    soundscape: None,
                    selectors: None,
                    strata: bedding("jungle"),
                    name: "jungle".into(),
                    temperature: 0.65,
                    humidity: 0.71,
                    ground: None,
                    sky_fog: Some(SkyFog {
                        sky: (112, 148, 128),
                        fog: (126, 166, 140),
                    }),
                    cave_scale: None,
                    crop_fields: Vec::new(),
                    structures: [
                        structures::wild(&["ruined_keep"]),
                        structures::bridge("stone_bridge"),
                    ]
                    .concat(),
                    settlements: Vec::new(),
                    surface_features: plants::jungle(),
                    loot: Some(structures::biome_loot("jungle")),
                },
                BiomeDef {
                    soundscape: None,
                    selectors: None,
                    strata: bedding("swamp"),
                    name: "swamp".into(),
                    temperature: 0.36,
                    humidity: 0.75,
                    ground: Some(GroundParams {
                        rock: "terra:stone".into(),
                        soil: "terra:dirt".into(),
                        surface: "terra:dirt".into(),
                    }),
                    sky_fog: Some(SkyFog {
                        sky: (108, 126, 114),
                        fog: (124, 142, 126),
                    }),
                    cave_scale: Some(0.5),
                    crop_fields: Vec::new(),
                    structures: structures::bridge("stone_bridge"),
                    settlements: Vec::new(),
                    surface_features: plants::swamp(),
                    loot: Some(structures::biome_loot("swamp")),
                },
                BiomeDef {
                    soundscape: Some("terra:dry".into()),
                    selectors: None,
                    strata: bedding("savanna"),
                    name: "savanna".into(),
                    temperature: 0.63,
                    humidity: 0.56,
                    ground: Some(GroundParams {
                        rock: "terra:stone".into(),
                        soil: "terra:dirt".into(),
                        surface: "terra:dirt".into(),
                    }),
                    sky_fog: Some(SkyFog {
                        sky: (170, 160, 124),
                        fog: (192, 178, 136),
                    }),
                    cave_scale: None,
                    crop_fields: Vec::new(),
                    structures: [
                        structures::wild(&["standing_stones", "waystone"]),
                        structures::bridge("stone_bridge"),
                    ]
                    .concat(),
                    settlements: structures::village(),
                    surface_features: plants::savanna(),
                    loot: Some(structures::biome_loot("savanna")),
                },
                // Version 3 only: the sea floor, the strand and the high ground, chosen by altitude.
                // They name a range, so version 3 prefers them where they fit and earlier versions never choose them.
                BiomeDef {
                    soundscape: None,
                    selectors: Some(BiomeSelectors {
                        altitude: Some(SelectorRange {
                            min: -1000.0,
                            max: -0.01,
                        }),
                        coast: None,
                        erosion: None,
                        river: None,
                        relief: None,
                    }),
                    strata: bedding("ocean"),
                    name: "ocean".into(),
                    temperature: 0.5,
                    humidity: 0.8,
                    ground: Some(GroundParams {
                        rock: "terra:stone".into(),
                        soil: "terra:sand".into(),
                        surface: "terra:sand".into(),
                    }),
                    sky_fog: Some(SkyFog {
                        sky: (118, 150, 178),
                        fog: (130, 164, 190),
                    }),
                    cave_scale: Some(0.0),
                    crop_fields: Vec::new(),
                    structures: Vec::new(),
                    settlements: Vec::new(),
                    surface_features: plants::ocean(),
                    loot: None,
                },
                BiomeDef {
                    soundscape: None,
                    selectors: Some(BiomeSelectors {
                        // A metre of strand: a wider one takes a coast's ground from the Climate Biomes.
                        altitude: Some(SelectorRange { min: 0.0, max: 1.0 }),
                        coast: None,
                        erosion: None,
                        river: None,
                        relief: None,
                    }),
                    strata: bedding("beach"),
                    name: "beach".into(),
                    temperature: 0.6,
                    humidity: 0.6,
                    ground: Some(GroundParams {
                        rock: "terra:stone".into(),
                        soil: "terra:sand".into(),
                        surface: "terra:sand".into(),
                    }),
                    sky_fog: Some(SkyFog {
                        sky: (140, 156, 176),
                        fog: (160, 176, 196),
                    }),
                    cave_scale: None,
                    crop_fields: Vec::new(),
                    structures: Vec::new(),
                    settlements: Vec::new(),
                    surface_features: Vec::new(),
                    loot: None,
                },
                BiomeDef {
                    soundscape: Some("terra:cold".into()),
                    selectors: Some(BiomeSelectors {
                        altitude: Some(SelectorRange {
                            min: 90.0,
                            max: 1000.0,
                        }),
                        coast: None,
                        erosion: None,
                        river: None,
                        relief: None,
                    }),
                    strata: bedding("alpine"),
                    name: "alpine".into(),
                    temperature: 0.25,
                    humidity: 0.4,
                    ground: Some(GroundParams {
                        rock: "terra:stone".into(),
                        soil: "terra:stone".into(),
                        surface: "terra:snow".into(),
                    }),
                    sky_fog: Some(SkyFog {
                        sky: (150, 168, 192),
                        fog: (180, 194, 214),
                    }),
                    cave_scale: Some(0.0),
                    crop_fields: Vec::new(),
                    structures: Vec::new(),
                    settlements: Vec::new(),
                    surface_features: Vec::new(),
                    loot: None,
                },
            ]
            .into_iter()
            .chain([landforms::river()])
            .collect(),
            survival: None,
            sounds: audio::sounds(),
            sound_sets: audio::sets(),
            material_sounds: audio::materials(),
            soundscapes: audio::soundscapes(),
            cause_sounds: audio::causes(),
            recipes: vec![
                RecipeDef {
                    inputs: vec![RecipeStack {
                        item: "flora:log".into(),
                        count: 4,
                    }],
                    output: RecipeStack {
                        item: "terra:workbench".into(),
                        count: 1,
                    },
                    station: "hand".into(),
                },
                RecipeDef {
                    inputs: vec![
                        RecipeStack {
                            item: "terra:rock".into(),
                            count: 2,
                        },
                        RecipeStack {
                            item: "terra:dirt".into(),
                            count: 1,
                        },
                    ],
                    output: RecipeStack {
                        item: "terra:brick".into(),
                        count: 2,
                    },
                    station: "hand".into(),
                },
                RecipeDef {
                    inputs: vec![RecipeStack {
                        item: "terra:rock".into(),
                        count: 1,
                    }],
                    output: RecipeStack {
                        item: "terra:sand".into(),
                        count: 2,
                    },
                    station: "hand".into(),
                },
                RecipeDef {
                    inputs: vec![RecipeStack {
                        item: "flora:leaves".into(),
                        count: 4,
                    }],
                    output: RecipeStack {
                        item: "terra:dirt".into(),
                        count: 1,
                    },
                    station: "hand".into(),
                },
                RecipeDef {
                    inputs: vec![RecipeStack {
                        item: "flora:log".into(),
                        count: 8,
                    }],
                    output: RecipeStack {
                        item: "terra:chest".into(),
                        count: 1,
                    },
                    station: "workbench".into(),
                },
                RecipeDef {
                    inputs: vec![RecipeStack {
                        item: "flora:log".into(),
                        count: 3,
                    }],
                    output: RecipeStack {
                        item: "terra:door".into(),
                        count: 1,
                    },
                    station: "workbench".into(),
                },
                RecipeDef {
                    inputs: vec![RecipeStack {
                        item: "flora:log".into(),
                        count: 2,
                    }],
                    output: RecipeStack {
                        item: "terra:gate".into(),
                        count: 1,
                    },
                    station: "workbench".into(),
                },
                RecipeDef {
                    inputs: vec![RecipeStack {
                        item: "terra:sand".into(),
                        count: 2,
                    }],
                    output: RecipeStack {
                        item: "terra:glass".into(),
                        count: 1,
                    },
                    station: "workbench".into(),
                },
                RecipeDef {
                    inputs: vec![
                        RecipeStack {
                            item: "terra:iron".into(),
                            count: 1,
                        },
                        RecipeStack {
                            item: "terra:glass".into(),
                            count: 1,
                        },
                    ],
                    output: RecipeStack {
                        item: "terra:lamp".into(),
                        count: 2,
                    },
                    station: "workbench".into(),
                },
                RecipeDef {
                    inputs: vec![RecipeStack {
                        item: "terra:sand".into(),
                        count: 4,
                    }],
                    output: RecipeStack {
                        item: "terra:rock".into(),
                        count: 1,
                    },
                    station: "workbench".into(),
                },
                RecipeDef {
                    inputs: vec![RecipeStack {
                        item: "flora:log".into(),
                        count: 1,
                    }],
                    output: RecipeStack {
                        item: "terra:ash".into(),
                        count: 2,
                    },
                    station: "workbench".into(),
                },
            ]
            .into_iter()
            .chain(building::recipes())
            .collect(),
            plants: Vec::new(),
            marker_kinds: vec![
                marker("home", MarkerIcon::Home, [224, 177, 90]),
                marker("camp", MarkerIcon::Camp, [217, 130, 74]),
                marker("mine", MarkerIcon::Mine, [157, 180, 204]),
                marker("farm", MarkerIcon::Farm, [147, 199, 101]),
                marker("landmark", MarkerIcon::Landmark, [207, 154, 216]),
                marker("death", MarkerIcon::Death, [217, 100, 92]),
            ],
            death_marker_kind: Some("terra:death".into()),
            templates: templates::templates(),
            settlements: structures::settlements(),
            voxel_uses: vec![
                VoxelUse {
                    material: "terra:door".into(),
                    kind: VoxelUseKind::Door(tesserae::mods::content::DoorDef {
                        open: "terra:door_open".into(),
                        width_m: Some(DOOR_WIDTH_M),
                        height_m: Some(DOOR_HEIGHT_M),
                    }),
                    shape: Some("terra:panel".into()),
                },
                VoxelUse {
                    material: "terra:gate".into(),
                    kind: VoxelUseKind::Door(tesserae::mods::content::DoorDef {
                        open: "terra:gate_open".into(),
                        width_m: Some(1.0),
                        height_m: Some(1.0),
                    }),
                    shape: Some("terra:panel".into()),
                },
                VoxelUse {
                    material: "terra:chest".into(),
                    kind: VoxelUseKind::Container(None),
                    shape: None,
                },
                VoxelUse {
                    material: "terra:workbench".into(),
                    kind: VoxelUseKind::Station("workbench".into()),
                    shape: Some("terra:table".into()),
                },
            ]
            .into_iter()
            .chain(building::voxel_uses())
            .collect(),
            ores: vec![
                OreDef {
                    material: "terra:iron".into(),
                    min_depth_m: 5.0,
                    max_depth_m: 28.0,
                    vein_size_m: 5.0,
                    rarity: 0.1,
                    biomes: Vec::new(),
                },
                OreDef {
                    material: "terra:crystal".into(),
                    min_depth_m: 24.0,
                    max_depth_m: 160.0,
                    vein_size_m: 4.0,
                    rarity: 0.15,
                    biomes: Vec::new(),
                },
            ],
        }
    }

    fn start(_world: WorldInfo, _ids: Registered) {}

    /// Terra builds nothing in a World: a new World's spawn stands beside a Settlement.
    fn tick(_ctx: TickContext) -> Vec<Command> {
        Vec::new()
    }
}

export!(Terra);

/// The rock a Biome beds under its soil, from the top down: a clay seam, a band of paler stone and
/// basalt deep down; sand over the clay where the ground is dry, and no clay up a mountain.
fn bedding(biome: &str) -> Option<Vec<BiomeStratum>> {
    let layer = |material: &str, depth_min_m: f32, depth_max_m: f32| BiomeStratum {
        material: material.into(),
        depth_min_m,
        depth_max_m,
    };
    let deep = layer("strata:basalt", 40.0, 96.0);
    Some(match biome {
        "dry" | "savanna" | "beach" => vec![
            layer("terra:sand", 3.0, 8.0),
            layer("strata:clay", 14.0, 17.0),
            layer("strata:stone", 22.0, 30.0),
            deep,
        ],
        "alpine" => vec![layer("strata:stone", 6.0, 28.0), deep],
        "ocean" => vec![layer("strata:clay", 2.0, 6.0), deep],
        _ => vec![
            layer("strata:clay", 4.0, 7.0),
            layer("strata:stone", 12.0, 20.0),
            deep,
        ],
    })
}
