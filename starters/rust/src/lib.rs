wit_bindgen::generate!({ path: "wit", world: "mod" });

use exports::tesserae::mods::guest::{
    Command, Declaration, Guest, Registered, TickContext, WorldInfo,
};
use tesserae::mods::content::{
    Appearance, GroundParams, MaterialDef, MovementPolicy, Occlusion, RenderClass,
};
use tesserae::mods::types::Srgba;

struct Starter;

fn solid(name: &str) -> MaterialDef {
    MaterialDef {
        name: name.into(),
        emission: 0,
        dimming: 15,
        occlusion: Occlusion::Opaque,
        render: RenderClass::Opaque,
        remove_time_s: Some(0.35),
        contact_damage_per_s: None,
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

impl Guest for Starter {
    fn declare() -> Declaration {
        let mut world = tesserae::mods::host::default_world_params();
        world.movement = Some(MovementPolicy {
            allow_flight: true,
            allow_physics: true,
            start_in_flight: false,
        });
        world.ground = Some(GroundParams {
            rock: "starter_rust:stone".into(),
            soil: "starter_rust:stone".into(),
            surface: "starter_rust:turf".into(),
        });
        Declaration {
            materials: vec![solid("stone"), solid("turf")],
            shapes: Vec::new(),
            appearance: vec![
                appearance("stone", [110, 115, 125, 255]),
                appearance("turf", [90, 160, 60, 255]),
            ],
            world: Some(world),
            view: None,
            fluids: Vec::new(),
            reactions: Vec::new(),
            combustion: Vec::new(),
            tools: Vec::new(),
            items: Vec::new(),
            biomes: Vec::new(),
            survival: None,
            sounds: Vec::new(),
            sound_sets: Vec::new(),
            material_sounds: Vec::new(),
            ores: Vec::new(),
        }
    }

    fn start(_world: WorldInfo, _ids: Registered) {}

    fn tick(_ctx: TickContext) -> Vec<Command> {
        Vec::new()
    }
}

export!(Starter);
