wit_bindgen::generate!({ path: "../../../wit/0.1", world: "mod" });

use exports::tesserae::mods::guest::{
    Command, Declaration, Guest, Registered, TickContext, WorldInfo,
};
use std::cell::Cell;
use tesserae::mods::commands::{Fill, Placement};
use tesserae::mods::content::{MaterialDef, Occlusion, RenderClass};
use tesserae::mods::types::{BoxM, LogLevel, Vec3, Voxel};

thread_local! {
    static MODE: Cell<u64> = const { Cell::new(0) };
    static ROCK: Cell<u16> = const { Cell::new(0) };
    static LAMP: Cell<u16> = const { Cell::new(0) };
    static CALLS: Cell<u64> = const { Cell::new(0) };
}
struct Queue;
fn point(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3 { x, y, z }
}
fn voxel(material: u16, shape: u8) -> Voxel {
    Voxel {
        material,
        shape,
        orientation: 0,
    }
}
fn place(at: Vec3, voxel: Voxel) -> Command {
    Command::Place(vec![Placement { at, voxel }])
}
fn fill(side: f32, material: u16) -> Command {
    Command::Fill(Fill {
        region: BoxM {
            min: point(0.0, 0.0, 0.0),
            max: point(side, side, side),
        },
        voxel: voxel(material, 0),
    })
}
impl Guest for Queue {
    fn declare() -> Declaration {
        Declaration {
            materials: vec![
                MaterialDef {
                    name: "stone".into(),
                    emission: 0,
                    dimming: 15,
                    occlusion: Occlusion::Opaque,
                    render: RenderClass::Opaque,
                    solid: true,
                },
                MaterialDef {
                    name: "lamp".into(),
                    emission: 15,
                    dimming: 15,
                    occlusion: Occlusion::Opaque,
                    render: RenderClass::Opaque,
                    solid: true,
                },
            ],
            shapes: vec![],
            appearance: vec![],
            world: None,
            view: None,
        }
    }
    fn start(world: WorldInfo, ids: Registered) {
        ROCK.set(
            ids.materials
                .iter()
                .find(|m| m.name.ends_with(":stone"))
                .unwrap()
                .id as u16,
        );
        LAMP.set(
            ids.materials
                .iter()
                .find(|m| m.name.ends_with(":lamp"))
                .unwrap()
                .id as u16,
        );
        MODE.set(world.seed);
    }
    fn tick(ctx: TickContext) -> Vec<Command> {
        let rock = ROCK.get();
        let lamp = LAMP.get();
        let call = CALLS.get() + 1;
        CALLS.set(call);
        tesserae::mods::host::log(
            LogLevel::Info,
            &format!(
                "call={call} tick={} dt={} applied={} rejected={} edits={} first={:?}",
                ctx.tick,
                ctx.dt_s,
                ctx.last.applied,
                ctx.last.rejected,
                ctx.edits.len(),
                ctx.edits.first()
            ),
        );
        if MODE.get() == 0x7e55_e2ae_0000_0001 && ctx.tick >= 128 {
            return vec![Command::Fill(Fill {
                region: BoxM {
                    min: point(16.0, 64.0, -64.0),
                    max: point(48.0, 96.0, -32.0),
                },
                voxel: voxel(rock, 0),
            })];
        }
        match (MODE.get(), call) {
            (1, 1) => vec![fill(4.0, 0), place(point(1.0, 1.0, 1.0), voxel(lamp, 0))],
            (2 | 5, 1) => vec![fill(32.0, rock)],
            (3, 1) => vec![place(point(4096.0, 0.0, 0.0), voxel(rock, 0))],
            (4, 1) => vec![place(point(0.0, 30.0, 0.0), voxel(rock, 2))],
            (5, 2) => panic!("fixture traps after queuing a fill"),
            (6, 1) => vec![
                place(point(0.0, 0.0, 0.0), voxel(rock, 0)),
                place(point(1.0, 0.0, 0.0), voxel(65535, 0)),
            ],
            (7, 1) => vec![Command::Place(vec![
                Placement {
                    at: point(0.0, 0.0, 0.0),
                    voxel: voxel(rock, 0)
                };
                65537
            ])],
            (8, _) => vec![fill(32.0, rock), fill(32.0, rock)],
            (10, 1) => {
                let at = ctx.edits[0].at;
                vec![Command::Fill(Fill {
                    region: BoxM {
                        min: at,
                        max: point(at.x + 32.0, at.y + 32.0, at.z + 32.0),
                    },
                    voxel: voxel(rock, 0),
                })]
            }
            (9, 1) => vec![place(point(31.0, 31.0, 31.0), voxel(rock, 0))],
            _ => vec![],
        }
    }
}
export!(Queue);
