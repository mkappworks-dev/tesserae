wit_bindgen::generate!({ path: "../../../wit/0.1", world: "mod" });

use exports::tesserae::mods::guest::{
    Command, Declaration, Guest, Registered, TickContext, WorldInfo,
};
use tesserae::mods::content::{MaterialDef, Occlusion, RenderClass};

struct Declarer;

impl Guest for Declarer {
    fn declare() -> Declaration {
        let materials = (0..10_000)
            .map(|i| MaterialDef {
                name: format!("m{i}"),
                emission: 0,
                dimming: 0,
                occlusion: Occlusion::Never,
                render: RenderClass::Opaque,
                solid: true,
            })
            .collect();
        Declaration {
            materials,
            shapes: Vec::new(),
            appearance: Vec::new(),
            world: None,
            view: None,
        }
    }

    fn start(_: WorldInfo, _: Registered) {}

    fn tick(_: TickContext) -> Vec<Command> {
        Vec::new()
    }
}

export!(Declarer);
