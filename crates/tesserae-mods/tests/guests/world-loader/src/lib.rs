wit_bindgen::generate!({ path: "../../../wit/0.1", world: "mod" });

use exports::tesserae::mods::guest::{
    Command, Declaration, Guest, Registered, TickContext, WorldInfo,
};
use tesserae::mods::content::{
    Appearance, MaterialDef, Occlusion, RenderClass, ShapeDef, ViewParams,
};
use tesserae::mods::types::{NodeBox, Srgba};

struct WorldLoader;

impl Guest for WorldLoader {
    fn declare() -> Declaration {
        let mut world = tesserae::mods::host::default_world_params();
        world.caves = None;
        world.relief_m = 12.0;
        Declaration {
            materials: vec![MaterialDef {
                name: "paint".into(),
                emission: 0,
                dimming: 0,
                occlusion: Occlusion::Never,
                render: RenderClass::Opaque,
                solid: true,
            }],
            shapes: vec![ShapeDef {
                name: "half".into(),
                boxes: vec![NodeBox {
                    min: (0, 0, 0),
                    max: (16, 8, 16),
                }],
                collision: None,
                connect: None,
            }],
            appearance: vec![Appearance {
                material: "paint".into(),
                color: Srgba {
                    r: 64,
                    g: 128,
                    b: 192,
                    a: 128,
                },
            }],
            world: Some(world),
            view: Some(ViewParams {
                view_distance_m: 160.0,
                lod_distance_m: 3000.0,
            }),
        }
    }

    fn start(world: WorldInfo, ids: Registered) {
        assert_eq!(world.voxel_size_m, 0.5);
        assert_eq!(world.seed, 123);
        for (name, id) in [("core:air", 0), ("world:paint", 8), ("zextra:m0", 9)] {
            assert!(
                ids.materials
                    .iter()
                    .any(|row| row.name == name && row.id == id)
            );
        }
        for (name, id) in [("core:cube", 0), ("world:half", 4), ("zextra:s0", 5)] {
            assert!(
                ids.shapes
                    .iter()
                    .any(|row| row.name == name && row.id == id)
            );
        }
    }

    fn tick(_: TickContext) -> Vec<Command> {
        panic!("loading must not call tick")
    }
}

export!(WorldLoader);
