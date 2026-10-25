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
        // `last.applied` 1 swaps the loop for hashing and allocation, which costs more time per fuel.
        if ctx.last.applied == 1 {
            return churn(ctx.tick);
        }
        // Loads, stores and arithmetic that the optimiser cannot fold away, so a unit of fuel
        // costs about what it does in a real Mod.
        let mut buf = [0u32; 4096];
        let mut n = 0u32;
        // A tick number above zero is an iteration count, so a test can calibrate fuel against it.
        while ctx.tick == 0 || u64::from(n) < ctx.tick {
            let i = (n as usize).wrapping_mul(7) & 4095;
            let v = buf[i] ^ n.wrapping_mul(2_654_435_761);
            buf[(n as usize) & 4095] = black_box(v);
            n = n.wrapping_add(1);
        }
        black_box(buf);
        Vec::new()
    }
}

fn churn(iterations: u64) -> Vec<Command> {
    // Zero runs until the host cuts it off.
    let iterations = if iterations == 0 { u64::from(u32::MAX) } else { iterations };
    let mut map = std::collections::HashMap::new();
    let mut names: Vec<String> = Vec::new();
    for i in 0..iterations as u32 {
        map.insert(i % 512, black_box(i));
        if i % 16 == 0 {
            names.push(format!("m{i}"));
            if names.len() > 64 {
                names.clear();
            }
        }
        black_box(map.get(&(i.wrapping_mul(7) % 512)));
    }
    black_box((map, names));
    Vec::new()
}

export!(Fixture);