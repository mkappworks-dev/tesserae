wit_bindgen::generate!({ path: "../../../wit/0.1", world: "mod" });

use exports::tesserae::mods::guest::{
    Command, Declaration, Guest, Registered, TickContext, WorldInfo,
};
use tesserae::mods::commands::Placement;
use tesserae::mods::types::{Vec3, Voxel};

struct Echo;

impl Guest for Echo {
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

    // The edit count goes back as the x of one placement, so a test sees what the guest was given.
    fn tick(ctx: TickContext) -> Vec<Command> {
        vec![Command::Place(vec![Placement {
            at: Vec3 {
                x: ctx.edits.len() as f32,
                y: 0.0,
                z: 0.0,
            },
            voxel: Voxel {
                material: 1,
                shape: 0,
                orientation: 0,
            },
        }])]
    }
}

export!(Echo);
