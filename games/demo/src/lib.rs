wit_bindgen::generate!({ path: "../../crates/tesserae-mods/wit/0.3", world: "mod" });

use exports::tesserae::mods::guest::{
    Command, Declaration, Guest, Registered, TickContext, WorldInfo,
};
mod preview;

use std::cell::RefCell;
use tesserae::mods::commands::{Fill, Placement, SourceFill};
use tesserae::mods::content::{
    Appearance, BurnAfter, BurnEnd, CombustionDef, FluidDef, FluidState, GroundParams, HeatDef,
    MaterialDef, Occlusion, Reactant, ReactionDef, ReactionSide, RenderClass, ShapeDef, ViewParams,
    WaterParams,
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
    cube: u8,
    slab: u8,
    stair: u8,
    post: u8,
    fence: u8,
    stage: Stage,
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

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

struct Demo;

fn solid(name: &str) -> MaterialDef {
    MaterialDef {
        name: name.into(),
        emission: 0,
        dimming: 15,
        occlusion: Occlusion::Opaque,
        render: RenderClass::Opaque,
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

fn appearance(material: &str, [r, g, b, a]: [u8; 4]) -> Appearance {
    Appearance {
        material: material.into(),
        color: Srgba { r, g, b, a },
        preview: Some(preview::texture(material, [r, g, b])),
    }
}

impl Guest for Demo {
    fn declare() -> Declaration {
        let mut world = tesserae::mods::host::default_world_params();
        world.relief_m = 4.0;
        // Under the lowest ground within 20 m of the stage at the engine's
        // default seed, 26 m, so lakes stand in valleys and the stage is dry.
        world.water = Some(WaterParams {
            level_m: 25.5,
            material: "demo:water".into(),
        });
        world.caves = None;
        world.ground = Some(GroundParams {
            rock: "demo:stone".into(),
            soil: "demo:dirt".into(),
            surface: "demo:grass".into(),
        });
        Declaration {
            materials: vec![
                MaterialDef {
                    name: "lamp".into(),
                    emission: 15,
                    dimming: 15,
                    occlusion: Occlusion::Opaque,
                    render: RenderClass::Opaque,
                    solid: true,
                },
                MaterialDef {
                    name: "lava".into(),
                    emission: 15,
                    dimming: 15,
                    occlusion: Occlusion::Opaque,
                    render: RenderClass::Opaque,
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
                    solid: false,
                },
                solid("brick"),
                MaterialDef {
                    name: "water".into(),
                    emission: 0,
                    dimming: 2,
                    occlusion: Occlusion::SameMaterial,
                    render: RenderClass::Blend,
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
            ],
            shapes: vec![fence()],
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
                appearance("glass", [179, 224, 235, 89]),
                appearance("leaves", [41, 107, 36, 255]),
            ],
            world: Some(world),
            view: Some(ViewParams {
                view_distance_m: 160.0,
                lod_distance_m: 3000.0,
            }),
            // Water first: registration order decides an otherwise equal claim.
            fluids: vec![
                FluidDef {
                    material: "demo:water".into(),
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
                    material: "demo:lava".into(),
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
                    material: "demo:mud".into(),
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
                        fluid: "demo:lava".into(),
                        state: FluidState::Any,
                    },
                    second: Reactant {
                        fluid: "demo:water".into(),
                        state: FluidState::Any,
                    },
                    consumed: ReactionSide::First,
                    product: Some("demo:stone".into()),
                    receiving: None,
                },
                // Mud that touches lava fires to brick; here the lava is kept.
                ReactionDef {
                    first: Reactant {
                        fluid: "demo:mud".into(),
                        state: FluidState::Any,
                    },
                    second: Reactant {
                        fluid: "demo:lava".into(),
                        state: FluidState::Any,
                    },
                    consumed: ReactionSide::First,
                    product: Some("demo:brick".into()),
                    receiving: None,
                },
            ],
            // Wood catches 2 s after heat reaches it, burns for 8 s, lighting
            // and heating 2 m round it, and leaves ash, which does not burn.
            combustion: vec![CombustionDef {
                fuel: "demo:wood".into(),
                ignition_threshold: 6,
                ignition_delay_s: 2.0,
                emission: 12,
                heat: Some(HeatDef {
                    strength: 12,
                    reach_m: 2.0,
                }),
                end: BurnEnd::After(BurnAfter {
                    seconds: 8.0,
                    residue: Some("demo:ash".into()),
                }),
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
            stone: material("demo:stone"),
            lamp: material("demo:lamp"),
            lava: material("demo:lava"),
            water: material("demo:water"),
            mud: material("demo:mud"),
            wood: material("demo:wood"),
            cube: shape("core:cube"),
            slab: shape("core:slab"),
            stair: shape("core:stair"),
            post: shape("core:post"),
            fence: shape("demo:fence"),
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
                    log("demo first tick: wall, lantern post, lava and the scenarios in metres");
                    let build = build(state);
                    state.issued = voxels(&build, state.size_m);
                    state.stage = Stage::Building;
                    build
                }
                Stage::Building if state.applied + state.rejected >= state.issued => {
                    log(&format!(
                        "demo build applied={} rejected={}",
                        state.applied, state.rejected
                    ));
                    log("demo scenario: sources placed, fuel lit");
                    state.stage = Stage::Running;
                    state.started_tick = ctx.tick;
                    start(state)
                }
                Stage::Running if elapsed_s >= DOUSE_S => {
                    log("demo scenario: water poured on the burning post");
                    state.stage = Stage::Doused;
                    vec![Command::PlaceSources(SourceFill {
                        region: cell(state, (-23.0, 30.0, -7.0)),
                        material: state.water,
                    })]
                }
                Stage::Doused if elapsed_s >= DRY_S => {
                    log("demo scenario: the waterfall's source taken away");
                    state.stage = Stage::Dried;
                    vec![Command::RemoveSources(cell(state, (22.0, 35.0, -7.0)))]
                }
                _ => vec![],
            }
        })
    }
}

fn log(message: &str) {
    tesserae::mods::host::log(LogLevel::Info, message);
}

/// One Voxel's region at a point in metres, at any Voxel size.
fn cell(state: &State, at: (f32, f32, f32)) -> BoxM {
    let s = state.size_m;
    region(at, (at.0 + s, at.1 + s, at.2 + s))
}

/// Every solid, built before any fluid moves or fuel burns. Without a terrain
/// read, foundations start below the 24..28 m terrain band and the scenarios
/// stand on beams at 28 m, clear of it.
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

export!(Demo);
