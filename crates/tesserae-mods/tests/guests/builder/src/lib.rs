wit_bindgen::generate!({ path: "../../../wit", world: "mod" });

use exports::tesserae::mods::guest::{
    Command, Declaration, Guest, Registered, TickContext, WorldInfo,
};
use tesserae::mods::commands::Placement;
use tesserae::mods::types::{Vec3, Voxel};

struct Builder;

impl Guest for Builder {
    fn declare() -> Declaration {
        Declaration {
            materials: Vec::new(),
            shapes: Vec::new(),
            appearance: Vec::new(),
            world: None,
            view: None,
        }
    }

    fn start(_: WorldInfo, _: Registered) {}

    // The tick number is how many Voxels to place, so one guest serves every size.
    fn tick(ctx: TickContext) -> Vec<Command> {
        let placements = (0..ctx.tick)
            .map(|i| Placement {
                at: Vec3 {
                    x: i as f32,
                    y: 0.0,
                    z: 0.0,
                },
                voxel: Voxel {
                    material: 1,
                    shape: 0,
                    orientation: 0,
                },
            })
            .collect();
        vec![Command::Place(placements)]
    }
}

export!(Builder);
