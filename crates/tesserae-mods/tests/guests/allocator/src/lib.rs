wit_bindgen::generate!({ path: "../../../wit", world: "mod" });

use exports::tesserae::mods::guest::{
    Command, Declaration, Guest, Registered, TickContext, WorldInfo,
};
use std::hint::black_box;

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
        let big = vec![1u8; 1 << 30];
        black_box(&big);
        Vec::new()
    }
}

export!(Fixture);
