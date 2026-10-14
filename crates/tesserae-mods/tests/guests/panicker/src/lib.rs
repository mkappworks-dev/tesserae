wit_bindgen::generate!({ path: "../../../wit", world: "mod" });

use exports::tesserae::mods::guest::{
    Command, Declaration, Guest, Registered, TickContext, WorldInfo,
};

struct Fixture;

impl Guest for Fixture {
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

    fn tick(_: TickContext) -> Vec<Command> {
        panic!("the panicker panicked on purpose");
    }
}

export!(Fixture);
