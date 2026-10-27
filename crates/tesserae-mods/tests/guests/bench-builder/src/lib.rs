wit_bindgen::generate!({ path: "../../../wit/0.1", world: "mod" });

use exports::tesserae::mods::guest::{
    Command, Declaration, Guest, Registered, TickContext, WorldInfo,
};
use tesserae::mods::commands::Placement;
use tesserae::mods::types::{Vec3, Voxel};

use std::cell::Cell;
use tesserae::mods::content::{MaterialDef, Occlusion, RenderClass};
thread_local! {static ROCK:Cell<u16>=const {Cell::new(0)};}
struct Builder;

impl Guest for Builder {
    fn declare() -> Declaration {
        Declaration {
            materials: vec![MaterialDef {
                name: "stone".into(),
                emission: 0,
                dimming: 15,
                occlusion: Occlusion::Opaque,
                render: RenderClass::Opaque,
                solid: true,
            }],
            shapes: Vec::new(),
            appearance: Vec::new(),
            world: None,
            view: None,
        }
    }

    fn start(_: WorldInfo, ids: Registered) {
        ROCK.set(
            ids.materials
                .iter()
                .find(|m| m.name == "bench-builder:stone")
                .unwrap()
                .id as u16,
        );
    }

    // The tick number is how many Voxels to place, so one guest serves every size.
    fn tick(ctx: TickContext) -> Vec<Command> {
        let material = ROCK.get();
        let placements = (0..ctx.tick)
            .map(|i| Placement {
                at: Vec3 {
                    x: i as f32,
                    y: 0.0,
                    z: 0.0,
                },
                voxel: Voxel {
                    material,
                    shape: 0,
                    orientation: 0,
                },
            })
            .collect();
        vec![Command::Place(placements)]
    }
}

export!(Builder);
