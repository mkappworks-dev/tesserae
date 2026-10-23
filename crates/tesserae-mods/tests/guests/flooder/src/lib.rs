wit_bindgen::generate!({ path: "../../../wit/0.1", world: "mod" });

use exports::tesserae::mods::guest::{
    Command, Declaration, Guest, Registered, TickContext, WorldInfo,
};

struct Flooder;

impl Guest for Flooder {
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

    // The tick number is how many commands to return, so one guest serves every size.
    fn tick(ctx: TickContext) -> Vec<Command> {
        vec![Command::Place(Vec::new()); ctx.tick as usize]
    }
}

export!(Flooder);
