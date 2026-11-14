wit_bindgen::generate!({ path: "../../crates/tesserae-mods/wit/0.3", world: "mod" });

use exports::tesserae::mods::guest::{
    Command, Declaration, Guest, Registered, TickContext, WorldInfo,
};
mod audio;
mod character;
mod textures;

use std::cell::RefCell;
use std::collections::VecDeque;
use tesserae::mods::commands::{Fill, Placement, SourceFill};
use tesserae::mods::content::{
    Appearance, BiomeDef, BurnAfter, BurnEnd, ClimateParams, CombustionDef, FluidDef, FluidState,
    GroundParams, HeatDef, ItemDef, MaterialDef, Occlusion, OreDef, Reactant, ReactionDef,
    ReactionSide, RecipeDef, RecipeStack, RenderClass, ShapeDef, SkyFog, SurfaceFeature, ToolDef,
    ToolKind, ViewParams, VoxelUse, VoxelUseKind, WaterParams,
};
use tesserae::mods::types::{
    Arm, BoxM, CollisionBox, ConnectDef, LogLevel, NodeBox, Srgba, Vec3, Voxel,
};

#[derive(Default)]
struct State {
    size_m: f32,
    stone: u16,
    lamp: u16,
    lava: u16,
    water: u16,
    mud: u16,
    wood: u16,
    grass: u16,
    cube: u8,
    slab: u8,
    stair: u8,
    post: u8,
    fence: u8,
    stage: Stage,
    /// Build Commands not yet sent: the host caps each batch's Voxels.
    queued: VecDeque<Command>,
    /// Voxels the build's Commands cover, and how many of them have been
    /// reported applied or rejected, so the scenarios start on a finished build.
    issued: u64,
    applied: u64,
    rejected: u64,
    started_tick: u64,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Stage {
    #[default]
    Unbuilt,
    Building,
    Running,
    Doused,
    Dried,
}

/// Seconds after the scenarios start that water reaches the burning post, and
/// that the waterfall's source is taken away.
const DOUSE_S: f32 = 4.0;
const DRY_S: f32 = 24.0;

/// Ground spans `SEA_LEVEL_M` to `SEA_LEVEL_M + RELIEF_M`; lakes stand at 5/12
/// of the relief, the engine's default, so valleys hold water at any seed.
const SEA_LEVEL_M: f32 = 24.0;
const RELIEF_M: f32 = 24.0;
const LAKE_M: f32 = SEA_LEVEL_M + RELIEF_M * 5.0 / 12.0;

/// The stage site, levelled blind because a Mod cannot read terrain: ground
/// from below the band up to `SITE_TOP_M`, above the lake, and air from there
/// to above the band. The origin, where the Player spawns, stays outside it.
const SITE_X_M: (f32, f32) = (-28.0, 26.0);
const SITE_Z_M: (f32, f32) = (-12.0, -2.0);
const SITE_FLOOR_M: f32 = SEA_LEVEL_M - 4.0;
const SITE_TOP_M: f32 = LAKE_M + 2.0;
const SITE_CLEAR_M: f32 = SEA_LEVEL_M + RELIEF_M + 2.0;
/// How far the stage was raised from its old 28 m beams onto the site.
const LIFT_M: f32 = SITE_TOP_M - 28.0;

/// The host's caps on one `fill` and on one batch.
const MAX_FILL_VOXELS: u64 = 32_768;
const MAX_BATCH_VOXELS: u64 = 65_536;
const MAX_BATCH_COMMANDS: usize = 1_024;

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

struct Terra;

fn solid(name: &str) -> MaterialDef {
    MaterialDef {
        name: name.into(),
        emission: 0,
        dimming: 15,
        occlusion: Occlusion::Opaque,
        render: RenderClass::Opaque,
        remove_time_s: Some(0.35),
        contact_damage_per_s: None,
        solid: true,
    }
}

fn point(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3 { x, y, z }
}

fn region(min: (f32, f32, f32), max: (f32, f32, f32)) -> BoxM {
    BoxM {
        min: point(min.0, min.1, min.2),
        max: point(max.0, max.1, max.2),
    }
}

/// The Voxels a region covers at `size` m: those whose lower corner is inside.
fn voxels_in(region: &BoxM, size: f32) -> u64 {
    let n = |a: f32, b: f32| ((b / size).ceil() - (a / size).ceil()).max(0.0) as u64;
    n(region.min.x, region.max.x) * n(region.min.y, region.max.y) * n(region.min.z, region.max.z)
}

fn voxels(commands: &[Command], size: f32) -> u64 {
    commands
        .iter()
        .map(|c| match c {
            Command::Fill(f) => voxels_in(&f.region, size),
            Command::Place(p) => p.len() as u64,
            Command::PlaceSources(f) => voxels_in(&f.region, size),
            Command::RemoveSources(r) | Command::Ignite(r) | Command::Extinguish(r) => {
                voxels_in(r, size)
            }
        })
        .sum()
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

fn appearance(material: &str, [r, g, b, a]: [u8; 4]) -> Appearance {
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
                    solid: false,
                },
                solid("brick"),
                MaterialDef {
                    name: "water".into(),
                    emission: 0,
                    dimming: 2,
                    occlusion: Occlusion::SameMaterial,
                    render: RenderClass::Blend,
                    remove_time_s: None,
                    contact_damage_per_s: None,
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
                solid("melon"),
                solid("workbench"),
                solid("chest"),
                solid("door"),
                solid("door_open"),
                gate("gate"),
                gate("gate_open"),
            ],
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
                appearance("melon", [84, 150, 56, 255]),
                appearance("workbench", [214, 190, 140, 255]),
                appearance("chest", [128, 62, 40, 255]),
                appearance("door", [128, 88, 50, 255]),
                appearance("door_open", [128, 88, 50, 255]),
                appearance("gate", [140, 97, 55, 255]),
                appearance("gate_open", [140, 97, 55, 255]),
            ],
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
            tools: vec![ToolDef {
                name: "measure".into(),
                kind: ToolKind::Measure,
            }],
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
                },
            ],
            biomes: vec![
                BiomeDef {
                    soundscape: Some("terra:temperate".into()),
                    name: "temperate".into(),
                    temperature: 0.5,
                    humidity: 0.5,
                    ground: None,
                    sky_fog: Some(SkyFog {
                        sky: (126, 146, 168),
                        fog: (136, 160, 190),
                    }),
                    cave_scale: None,
                    // A melon in every 16 by 16 m of grass is found while exploring, not a
                    // carpet; Wood every 6 m gathers a Workbench, Chest and Door in a short walk.
                    surface_features: vec![
                        SurfaceFeature {
                            material: "terra:melon".into(),
                            spacing_m: 16.0,
                        },
                        SurfaceFeature {
                            material: "terra:wood".into(),
                            spacing_m: 6.0,
                        },
                    ],
                },
                BiomeDef {
                    soundscape: Some("terra:dry".into()),
                    name: "dry".into(),
                    temperature: 0.7,
                    humidity: 0.3,
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
                    surface_features: Vec::new(),
                },
                BiomeDef {
                    soundscape: Some("terra:cold".into()),
                    name: "cold".into(),
                    temperature: 0.3,
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
                    surface_features: Vec::new(),
                },
            ],
            survival: None,
            sounds: audio::sounds(),
            sound_sets: audio::sets(),
            material_sounds: audio::materials(),
            soundscapes: audio::soundscapes(),
            cause_sounds: audio::causes(),
            recipes: vec![
                RecipeDef {
                    inputs: vec![RecipeStack {
                        item: "terra:wood".into(),
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
                        item: "terra:leaves".into(),
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
                        item: "terra:wood".into(),
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
                        item: "terra:wood".into(),
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
                        item: "terra:wood".into(),
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
                        item: "terra:wood".into(),
                        count: 1,
                    }],
                    output: RecipeStack {
                        item: "terra:ash".into(),
                        count: 2,
                    },
                    station: "workbench".into(),
                },
            ],
            voxel_uses: vec![
                VoxelUse {
                    material: "terra:door".into(),
                    kind: VoxelUseKind::Door(tesserae::mods::content::DoorDef {
                        open: "terra:door_open".into(),
                        width_m: Some(1.0),
                        height_m: Some(2.0),
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
            ],
            ores: vec![OreDef {
                material: "terra:iron".into(),
                min_depth_m: 5.0,
                max_depth_m: 28.0,
                vein_size_m: 5.0,
                rarity: 0.1,
                biomes: Vec::new(),
            }],
        }
    }

    fn start(world: WorldInfo, ids: Registered) {
        let material = |name| {
            ids.materials
                .iter()
                .find(|row| row.name == name)
                .unwrap()
                .id as u16
        };
        let shape = |name| ids.shapes.iter().find(|row| row.name == name).unwrap().id as u8;
        STATE.replace(State {
            size_m: world.voxel_size_m,
            stage: if world.reopened {
                Stage::Dried
            } else {
                Stage::Unbuilt
            },
            stone: material("terra:stone"),
            lamp: material("terra:lamp"),
            lava: material("terra:lava"),
            water: material("terra:water"),
            mud: material("terra:mud"),
            wood: material("terra:wood"),
            grass: material("terra:grass"),
            cube: shape("core:cube"),
            slab: shape("core:slab"),
            stair: shape("core:stair"),
            post: shape("core:post"),
            fence: shape("terra:fence"),
            ..State::default()
        });
    }

    fn tick(ctx: TickContext) -> Vec<Command> {
        STATE.with_borrow_mut(|state| {
            state.applied += u64::from(ctx.last.applied);
            state.rejected += u64::from(ctx.last.rejected);
            let elapsed_s = (ctx.tick.saturating_sub(state.started_tick)) as f32 * ctx.dt_s;
            match state.stage {
                Stage::Unbuilt => {
                    log("terra first tick: site, wall, lantern post, lava and the scenarios in metres");
                    let mut all = site(state);
                    all.extend(lifted(build(state)));
                    state.issued = voxels(&all, state.size_m);
                    state.queued = all.into();
                    state.stage = Stage::Building;
                    next_batch(state)
                }
                Stage::Building if !state.queued.is_empty() => next_batch(state),
                Stage::Building if state.applied + state.rejected >= state.issued => {
                    log(&format!(
                        "terra build applied={} rejected={}",
                        state.applied, state.rejected
                    ));
                    log("terra scenario: sources placed, fuel lit");
                    state.stage = Stage::Running;
                    state.started_tick = ctx.tick;
                    lifted(start(state))
                }
                Stage::Running if elapsed_s >= DOUSE_S => {
                    log("terra scenario: water poured on the burning post");
                    state.stage = Stage::Doused;
                    vec![Command::PlaceSources(SourceFill {
                        region: cell(state, (-23.0, 30.0 + LIFT_M, -7.0)),
                        material: state.water,
                    })]
                }
                Stage::Doused if elapsed_s >= DRY_S => {
                    log("terra scenario: the waterfall's source taken away");
                    state.stage = Stage::Dried;
                    vec![Command::RemoveSources(cell(
                        state,
                        (22.0, 35.0 + LIFT_M, -7.0),
                    ))]
                }
                _ => vec![],
            }
        })
    }
}

fn log(message: &str) {
    tesserae::mods::host::log(LogLevel::Info, message);
}

/// The stage below was laid out on 28 m beams; it now stands on the site.
fn lifted(commands: Vec<Command>) -> Vec<Command> {
    let up = |p: Vec3| point(p.x, p.y + LIFT_M, p.z);
    let raise = |r: BoxM| BoxM {
        min: up(r.min),
        max: up(r.max),
    };
    commands
        .into_iter()
        .map(|command| match command {
            Command::Fill(f) => Command::Fill(Fill {
                region: raise(f.region),
                voxel: f.voxel,
            }),
            Command::Place(placements) => Command::Place(
                placements
                    .into_iter()
                    .map(|p| Placement {
                        at: up(p.at),
                        voxel: p.voxel,
                    })
                    .collect(),
            ),
            Command::PlaceSources(f) => Command::PlaceSources(SourceFill {
                region: raise(f.region),
                material: f.material,
            }),
            Command::RemoveSources(r) => Command::RemoveSources(raise(r)),
            Command::Ignite(r) => Command::Ignite(raise(r)),
            Command::Extinguish(r) => Command::Extinguish(raise(r)),
        })
        .collect()
}

/// Queued build Commands up to the host's batch caps, in order.
fn next_batch(state: &mut State) -> Vec<Command> {
    let mut batch = Vec::new();
    let mut total = 0;
    while let Some(next) = state.queued.front() {
        let n = voxels(std::slice::from_ref(next), state.size_m);
        if !batch.is_empty() && (total + n > MAX_BATCH_VOXELS || batch.len() == MAX_BATCH_COMMANDS)
        {
            break;
        }
        total += n;
        batch.extend(state.queued.pop_front());
    }
    batch
}

/// `make` over `region` cut along x into pieces within the fill cap.
fn split(region: BoxM, size: f32, make: impl Fn(BoxM) -> Command) -> Vec<Command> {
    let across = |a: f32, b: f32| ((b / size).ceil() - (a / size).ceil()).max(1.0);
    let column = across(region.min.y, region.max.y) * across(region.min.z, region.max.z);
    let step = (MAX_FILL_VOXELS as f32 / column).floor().max(1.0) * size;
    let mut pieces = Vec::new();
    let mut x = region.min.x;
    while x < region.max.x {
        let end = (x + step).min(region.max.x);
        pieces.push(make(BoxM {
            min: point(x, region.min.y, region.min.z),
            max: point(end, region.max.y, region.max.z),
        }));
        x = end;
    }
    pieces
}

/// Levels the stage site: no lake water in it, ground to `SITE_TOP_M` with
/// grass on top, and nothing above.
fn site(state: &State) -> Vec<Command> {
    let s = state.size_m;
    let (x, z) = (SITE_X_M, SITE_Z_M);
    let band = |low: f32, high: f32| region((x.0, low, z.0), (x.1, high, z.1));
    let fill = |voxel: Voxel| move |region| Command::Fill(Fill { region, voxel });
    let solid = |material| Voxel {
        material,
        shape: state.cube,
        orientation: 0,
    };
    let air = Voxel {
        material: 0,
        shape: state.cube,
        orientation: 0,
    };
    let mut commands = split(band(SITE_FLOOR_M, SITE_TOP_M), s, Command::RemoveSources);
    commands.extend(split(
        band(SITE_FLOOR_M, SITE_TOP_M - 1.0),
        s,
        fill(solid(state.stone)),
    ));
    commands.extend(split(
        band(SITE_TOP_M - 1.0, SITE_TOP_M),
        s,
        fill(solid(state.grass)),
    ));
    commands.extend(split(band(SITE_TOP_M, SITE_CLEAR_M), s, fill(air)));
    commands
}

/// One Voxel's region at a point in metres, at any Voxel size.
fn cell(state: &State, at: (f32, f32, f32)) -> BoxM {
    let s = state.size_m;
    region(at, (at.0 + s, at.1 + s, at.2 + s))
}

/// Every solid, built before any fluid moves or fuel burns, laid out on beams
/// at 28 m; `lifted` stands it on the site.
fn build(state: &State) -> Vec<Command> {
    let s = state.size_m;
    let voxel = |material, shape, orientation| Voxel {
        material,
        shape,
        orientation,
    };
    let stone = voxel(state.stone, state.cube, 0);
    let fill = |min, max, voxel| {
        Command::Fill(Fill {
            region: region(min, max),
            voxel,
        })
    };
    let mut post = Vec::new();
    let mut y = (24.0 / s).ceil() * s;
    while y < 31.0 {
        post.push(Placement {
            at: point(3.0, y, -7.0),
            voxel: voxel(state.stone, state.fence, 0),
        });
        y += s;
    }
    post.push(Placement {
        at: point(3.0, 31.0, -7.0),
        voxel: voxel(state.lamp, state.cube, 0),
    });
    let wood = voxel(state.wood, state.cube, 0);
    let top_slab = 4;
    vec![
        fill((-4.0, 24.0, -7.0), (2.0, 29.0, -6.0), stone),
        Command::Place(post),
        // A lava source on the wall top, which spills over its sides.
        Command::PlaceSources(SourceFill {
            region: cell(state, (-1.0, 29.0, -7.0)),
            material: state.lava,
        }),
        // On a beam, mud from the west end meets lava and fires to brick, and
        // water from the east end meets it and cools it to stone; water runs
        // through a fence, a top slab and a post on its way, and falls off.
        fill((6.0, 28.0, -7.0), (20.0, 29.0, -6.0), stone),
        fill(
            (14.0, 29.0, -7.0),
            (15.0, 29.0 + s, -6.0),
            voxel(state.stone, state.fence, 0),
        ),
        fill(
            (15.0, 29.0, -7.0),
            (16.0, 29.0 + s, -6.0),
            voxel(state.stone, state.slab, top_slab),
        ),
        fill(
            (16.0, 29.0, -7.0),
            (17.0, 29.0 + s, -6.0),
            voxel(state.stone, state.post, 0),
        ),
        // The waterfall's pillar.
        fill((22.0, 24.0, -7.0), (23.0, 35.0, -6.0), stone),
        // Fire: lit wood at -14 catches the wood across a metre of open air
        // at -16, and its stair after; a stone wall keeps it from the wood at
        // -12 behind it.
        fill((-17.0, 28.0, -7.0), (-10.0, 29.0, -6.0), stone),
        fill((-13.0, 29.0, -9.0), (-12.0, 31.0, -4.0), stone),
        fill((-12.0, 29.0, -7.0), (-11.0, 30.0, -6.0), wood),
        fill((-14.0, 29.0, -7.0), (-13.0, 30.0, -6.0), wood),
        fill((-16.0, 29.0, -7.0), (-15.0, 30.0, -6.0), wood),
        fill(
            (-17.0, 29.0, -7.0),
            (-16.0, 30.0, -6.0),
            voxel(state.wood, state.stair, 1),
        ),
        // A burning post water is later poured on, on a beam of its own so
        // that water cannot reach the row above.
        fill((-25.0, 28.0, -7.0), (-20.0, 29.0, -6.0), stone),
        fill((-23.0, 29.0, -7.0), (-22.0, 30.0, -6.0), wood),
    ]
}

/// The fluids and fires the scenarios start with.
fn start(state: &State) -> Vec<Command> {
    let source = |at, material| {
        Command::PlaceSources(SourceFill {
            region: cell(state, at),
            material,
        })
    };
    let s = state.size_m;
    vec![
        source((6.0, 29.0, -7.0), state.mud),
        source((9.0, 29.0, -7.0), state.lava),
        source((19.0, 29.0, -7.0), state.water),
        source((22.0, 35.0, -7.0), state.water),
        // The top layer only: below it fire spreads by exposure, and wood
        // inside a block, which no water can touch, never catches.
        Command::Ignite(region((-14.0, 30.0 - s, -7.0), (-13.0, 30.0, -6.0))),
        Command::Ignite(region((-23.0, 30.0 - s, -7.0), (-22.0, 30.0, -6.0))),
    ]
}

export!(Terra);
