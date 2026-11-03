//! A second, plain Game: rolling sand-and-turf ground, no water, no stage. It exists so two
//! Games can be installed side by side and told apart at a glance.

wit_bindgen::generate!({ path: "../../crates/tesserae-mods/wit/0.3", world: "mod" });

use exports::tesserae::mods::guest::{
    Command, Declaration, Guest, Registered, TickContext, WorldInfo,
};
use tesserae::mods::content::{
    Appearance, GroundParams, MaterialDef, MovementPolicy, Occlusion, RenderClass,
};
use tesserae::mods::types::Srgba;

struct Meadow;

fn solid(name: &str) -> MaterialDef {
    MaterialDef {
        name: name.into(),
        emission: 0,
        dimming: 15,
        occlusion: Occlusion::Opaque,
        render: RenderClass::Opaque,
        remove_time_s: Some(0.35),
        solid: true,
    }
}

fn appearance(material: &str, [r, g, b, a]: [u8; 4]) -> Appearance {
    Appearance {
        material: material.into(),
        color: Srgba { r, g, b, a },
        texture: None,
    }
}

impl Guest for Meadow {
    fn declare() -> Declaration {
        let mut world = tesserae::mods::host::default_world_params();
        world.movement = Some(MovementPolicy {
            allow_flight: true,
            allow_physics: true,
            start_in_flight: false,
        });
        world.relief_m = 14.0;
        world.ground = Some(GroundParams {
            rock: "meadow:sandstone".into(),
            soil: "meadow:clay".into(),
            surface: "meadow:turf".into(),
        });
        Declaration {
            materials: vec![solid("sandstone"), solid("clay"), solid("turf")],
            shapes: Vec::new(),
            appearance: vec![
                appearance("sandstone", [214, 190, 140, 255]),
                appearance("clay", [176, 112, 84, 255]),
                appearance("turf", [150, 190, 70, 255]),
            ],
            world: Some(world),
            view: None,
            fluids: Vec::new(),
            reactions: Vec::new(),
            combustion: Vec::new(),
            tools: Vec::new(),
        }
    }

    fn start(_world: WorldInfo, _ids: Registered) {}

    fn tick(_ctx: TickContext) -> Vec<Command> {
        Vec::new()
    }
}

export!(Meadow);
