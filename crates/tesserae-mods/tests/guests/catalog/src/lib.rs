wit_bindgen::generate!({ path: "../../../wit", world: "mod" });

use exports::tesserae::mods::guest::{
    Command, Declaration, Guest, Registered, TickContext, WorldInfo,
};
use tesserae::mods::content::{MaterialDef, Occlusion, RenderClass, ShapeDef};
use tesserae::mods::types::NodeBox;

struct Catalog;

impl Guest for Catalog {
    fn declare() -> Declaration {
        let materials = (0..8)
            .map(|i| MaterialDef {
                name: format!("m{i}"),
                emission: i,
                dimming: 0,
                occlusion: Occlusion::Never,
                render: RenderClass::Opaque,
                solid: true,
            })
            .collect();
        let shapes = (0..8)
            .map(|i| ShapeDef {
                name: format!("s{i}"),
                boxes: vec![NodeBox {
                    min: (0, 0, 0),
                    max: (16, 8 + i, 16),
                }],
                collision: None,
                connect: None,
            })
            .collect();
        Declaration {
            materials,
            shapes,
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

export!(Catalog);
