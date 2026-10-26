//! A 0.2 Game that asks for standing water, which names no fluid on its line.

wit_bindgen::generate!({ path: "../../../wit/0.2", world: "mod" });

use exports::tesserae::mods::guest::{
    Command, Declaration, Guest, Registered, TickContext, WorldInfo,
};
use tesserae::mods::content::WaterParams;

struct Pond;

impl Guest for Pond {
    fn declare() -> Declaration {
        let mut world = tesserae::mods::host::default_world_params();
        world.water = Some(WaterParams {
            level_m: world.sea_level_m + world.relief_m / 2.0,
        });
        Declaration {
            materials: vec![],
            shapes: vec![],
            appearance: vec![],
            world: Some(world),
            view: None,
            fluids: vec![],
            reactions: vec![],
            combustion: vec![],
        }
    }

    fn start(_: WorldInfo, _: Registered) {}

    fn tick(_: TickContext) -> Vec<Command> {
        vec![]
    }
}

export!(Pond);
