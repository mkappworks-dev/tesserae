wit_bindgen::generate!({ path: "../../../wit/0.2", world: "mod" });

use exports::tesserae::mods::guest::{
    Command, Declaration, Guest, Registered, TickContext, WorldInfo,
};
use std::cell::Cell;
use tesserae::mods::commands::{Fill, Placement, SourceFill};
use tesserae::mods::content::{
    Appearance, BurnAfter, BurnEnd, CombustionDef, FluidDef, FluidState, HeatDef, MaterialDef,
    Occlusion, Reactant, ReactionDef, ReactionSide, RenderClass,
};
use tesserae::mods::types::{BoxM, LogLevel, Srgba, Vec3, Voxel};

thread_local! {
    static MODE: Cell<u64> = const { Cell::new(0) };
    static CALLS: Cell<u64> = const { Cell::new(0) };
    static SIZE: Cell<f32> = const { Cell::new(1.0) };
    static SLIME: Cell<u16> = const { Cell::new(0) };
    static WOOD: Cell<u16> = const { Cell::new(0) };
}

struct Fluids;

fn material(name: &str, solid: bool) -> MaterialDef {
    MaterialDef {
        name: name.into(),
        emission: 0,
        dimming: if solid { 15 } else { 3 },
        occlusion: if solid {
            Occlusion::Opaque
        } else {
            Occlusion::SameMaterial
        },
        render: if solid {
            RenderClass::Opaque
        } else {
            RenderClass::Blend
        },
        solid,
    }
}

/// One Voxel's box at metre position `x, y, z`, at any Voxel size.
fn cell(x: f32, y: f32, z: f32) -> BoxM {
    let s = SIZE.get();
    BoxM {
        min: Vec3 { x, y, z },
        max: Vec3 {
            x: x + s,
            y: y + s,
            z: z + s,
        },
    }
}

fn reactant(fluid: &str, state: FluidState) -> Reactant {
    Reactant {
        fluid: fluid.into(),
        state,
    }
}

impl Guest for Fluids {
    fn declare() -> Declaration {
        Declaration {
            materials: vec![
                material("slime", false),
                material("wood", true),
                material("ash", true),
                material("brine", false),
            ],
            shapes: vec![],
            appearance: vec![Appearance {
                material: "slime".into(),
                color: Srgba {
                    r: 96,
                    g: 200,
                    b: 64,
                    a: 160,
                },
            }],
            world: None,
            view: None,
            fluids: vec![
                // Neither water nor lava: its own reach, rates and reset drop.
                FluidDef {
                    material: "fluids:slime".into(),
                    reach_m: 2.0,
                    sideways_m_per_s: 2.0,
                    downward_m_per_s: 3.0,
                    recession_m_per_s: 1.0,
                    reset_drop_m: 1.0,
                    waterlogs: true,
                    extinguishes: true,
                    heat: None,
                },
                // Water's numbers, so that slime meets it as it would water.
                FluidDef {
                    material: "fluids:brine".into(),
                    reach_m: 7.0,
                    sideways_m_per_s: 4.0,
                    downward_m_per_s: 8.0,
                    recession_m_per_s: 4.0,
                    reset_drop_m: 0.5,
                    waterlogs: true,
                    extinguishes: true,
                    heat: None,
                },
            ],
            reactions: vec![ReactionDef {
                first: reactant("fluids:slime", FluidState::Flowing),
                second: reactant("fluids:brine", FluidState::Any),
                consumed: ReactionSide::First,
                product: Some("fluids:ash".into()),
                receiving: None,
            }],
            combustion: vec![CombustionDef {
                fuel: "fluids:wood".into(),
                ignition_threshold: 6,
                ignition_delay_s: 2.0,
                emission: 12,
                heat: Some(HeatDef {
                    strength: 10,
                    reach_m: 2.0,
                }),
                end: BurnEnd::After(BurnAfter {
                    seconds: 8.0,
                    residue: Some("fluids:ash".into()),
                }),
            }],
        }
    }

    fn start(world: WorldInfo, ids: Registered) {
        MODE.set(world.seed);
        SIZE.set(world.voxel_size_m);
        let id = |name: &str| {
            ids.materials
                .iter()
                .find(|row| row.name == name)
                .expect("registered")
                .id as u16
        };
        SLIME.set(id("fluids:slime"));
        WOOD.set(id("fluids:wood"));
    }

    fn tick(ctx: TickContext) -> Vec<Command> {
        let call = CALLS.get() + 1;
        CALLS.set(call);
        tesserae::mods::host::log(
            LogLevel::Info,
            &format!(
                "call={call} applied={} rejected={} edits={:?}",
                ctx.last.applied, ctx.last.rejected, ctx.edits
            ),
        );
        let s = SIZE.get();
        let wood = Voxel {
            material: WOOD.get(),
            shape: 0,
            orientation: 0,
        };
        match (MODE.get(), call) {
            // Two sources, one of them on stone it cannot take, then both away.
            (1, 1) => vec![Command::PlaceSources(SourceFill {
                region: BoxM {
                    min: Vec3 {
                        x: 0.0,
                        y: s,
                        z: 0.0,
                    },
                    max: Vec3 {
                        x: 2.0 * s,
                        y: 2.0 * s,
                        z: s,
                    },
                },
                material: SLIME.get(),
            })],
            (1, 2) => vec![Command::RemoveSources(cell(0.0, s, 0.0))],
            // Fuel set alight, a Voxel of air beside it that cannot be, then
            // put out.
            (2, 1) => vec![
                Command::Fill(Fill {
                    region: cell(0.0, s, 0.0),
                    voxel: wood,
                }),
                Command::Ignite(BoxM {
                    min: Vec3 {
                        x: 0.0,
                        y: s,
                        z: 0.0,
                    },
                    max: Vec3 {
                        x: 2.0 * s,
                        y: 2.0 * s,
                        z: s,
                    },
                }),
            ],
            (2, 2) => vec![Command::Extinguish(cell(0.0, s, 0.0))],
            // A fluid placed as a solid is refused whole on this line.
            (3, 1) => vec![Command::Place(vec![Placement {
                at: Vec3 {
                    x: 0.0,
                    y: s,
                    z: 0.0,
                },
                voxel: Voxel {
                    material: SLIME.get(),
                    shape: 0,
                    orientation: 0,
                },
            }])],
            _ => vec![],
        }
    }
}

export!(Fluids);
