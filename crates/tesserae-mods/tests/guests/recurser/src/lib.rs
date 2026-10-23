wit_bindgen::generate!({ path: "../../../wit/0.1", world: "mod" });

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

    fn tick(ctx: TickContext) -> Vec<Command> {
        black_box(recurse(ctx.tick));
        Vec::new()
    }
}

#[allow(unconditional_recursion)]
fn recurse(n: u64) -> u64 {
    let pad = [n as u8; 64];
    black_box(recurse(black_box(n + 1))) + u64::from(black_box(pad)[3])
}

export!(Fixture);
