wit_bindgen::generate!({ path: "../../crates/tesserae-mods/wit/0.2", world: "mod" });

use exports::tesserae::mods::guest::{
    Command, Declaration, Guest, Registered, TickContext, WorldInfo,
};
use std::cell::RefCell;
use tesserae::mods::commands::{Fill, Placement, SourceFill};
use tesserae::mods::content::{
    Appearance, BurnAfter, BurnEnd, CombustionDef, FluidDef, FluidState, HeatDef, MaterialDef,
    Occlusion, Reactant, ReactionDef, ReactionSide, RenderClass, ShapeDef, ViewParams,
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
    cube: u8,
    fence: u8,
    built: bool,
}

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

impl Guest for Demo {
    fn declare() -> Declaration {
        let mut world = tesserae::mods::host::default_world_params();
        world.relief_m = 4.0;
        world.water = None;
        world.caves = None;
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
            ],
            shapes: vec![fence()],
            appearance: vec![
                Appearance {
                    material: "lamp".into(),
                    color: Srgba {
                        r: 255,
                        g: 144,
                        b: 48,
                        a: 255,
                    },
                },
                Appearance {
                    material: "lava".into(),
                    color: Srgba {
                        r: 230,
                        g: 72,
                        b: 16,
                        a: 255,
                    },
                },
                Appearance {
                    material: "wood".into(),
                    color: Srgba {
                        r: 150,
                        g: 104,
                        b: 60,
                        a: 255,
                    },
                },
                Appearance {
                    material: "ash".into(),
                    color: Srgba {
                        r: 70,
                        g: 66,
                        b: 62,
                        a: 255,
                    },
                },
            ],
            world: Some(world),
            view: Some(ViewParams {
                view_distance_m: 160.0,
                lod_distance_m: 3000.0,
            }),
            // Slower and shorter than water, and hot; it stays out of Shapes.
            fluids: vec![FluidDef {
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
            }],
            // Lava that touches water cools to stone; the water is kept.
            reactions: vec![ReactionDef {
                first: Reactant {
                    fluid: "demo:lava".into(),
                    state: FluidState::Any,
                },
                second: Reactant {
                    fluid: "core:water".into(),
                    state: FluidState::Any,
                },
                consumed: ReactionSide::First,
                product: Some("core:stone".into()),
                receiving: None,
            }],
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
            stone: material("core:stone"),
            lamp: material("demo:lamp"),
            lava: material("demo:lava"),
            cube: shape("core:cube"),
            fence: shape("demo:fence"),
            built: false,
        });
    }

    fn tick(ctx: TickContext) -> Vec<Command> {
        STATE.with_borrow_mut(|state| {
            if ctx.last.applied > 0 || ctx.last.rejected > 0 {
                tesserae::mods::host::log(
                    LogLevel::Info,
                    &format!(
                        "demo build applied={} rejected={}",
                        ctx.last.applied, ctx.last.rejected
                    ),
                );
            }
            if std::mem::replace(&mut state.built, true) {
                return vec![];
            }
            tesserae::mods::host::log(
                LogLevel::Info,
                "demo first tick: wall, lantern post and lava in metres",
            );
            let voxel = |material, shape| Voxel {
                material,
                shape,
                orientation: 0,
            };
            // The foundation starts below the 24..28 m terrain band, without a terrain read.
            let mut post = Vec::new();
            let mut y = (24.0 / state.size_m).ceil() * state.size_m;
            while y < 31.0 {
                post.push(Placement {
                    at: point(3.0, y, -7.0),
                    voxel: voxel(state.stone, state.fence),
                });
                y += state.size_m;
            }
            post.push(Placement {
                at: point(3.0, 31.0, -7.0),
                voxel: voxel(state.lamp, state.cube),
            });
            let size = state.size_m;
            vec![
                Command::Fill(Fill {
                    region: BoxM {
                        min: point(-4.0, 24.0, -7.0),
                        max: point(2.0, 29.0, -6.0),
                    },
                    voxel: voxel(state.stone, state.cube),
                }),
                Command::Place(post),
                // A lava source on the wall top, which spills over its sides.
                Command::PlaceSources(SourceFill {
                    region: BoxM {
                        min: point(-1.0, 29.0, -7.0),
                        max: point(-1.0 + size, 29.0 + size, -7.0 + size),
                    },
                    material: state.lava,
                }),
            ]
        })
    }
}

export!(Demo);
