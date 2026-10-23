wit_bindgen::generate!({ path: "../../../wit/0.2", world: "mod" });

use exports::tesserae::mods::guest::{
    Command, Declaration, Guest, Registered, TickContext, WorldInfo,
};
use tesserae::mods::content::FluidDef;

struct BadFluid;

impl Guest for BadFluid {
    /// A fluid row naming a Material nobody registers.
    fn declare() -> Declaration {
        Declaration {
            materials: vec![],
            shapes: vec![],
            appearance: vec![],
            world: None,
            view: None,
            fluids: vec![FluidDef {
                material: "nobody:quicksilver".into(),
                reach_m: 4.0,
                sideways_m_per_s: 4.0,
                downward_m_per_s: 8.0,
                recession_m_per_s: 4.0,
                reset_drop_m: 0.5,
                waterlogs: true,
                extinguishes: false,
                heat: None,
            }],
            reactions: vec![],
            combustion: vec![],
        }
    }

    fn start(_: WorldInfo, _: Registered) {
        unreachable!("a load that fails never starts a Mod");
    }

    fn tick(_: TickContext) -> Vec<Command> {
        Vec::new()
    }
}

export!(BadFluid);
