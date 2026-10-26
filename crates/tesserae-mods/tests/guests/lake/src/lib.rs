//! A 0.3 Game whose lakes are a fluid it declares, and which logs every
//! player Edit it is shown.

wit_bindgen::generate!({ path: "../../../wit/0.3", world: "mod" });

use exports::tesserae::mods::guest::{
    Command, Declaration, Guest, Registered, TickContext, WorldInfo,
};
use tesserae::mods::content::{GroundParams, FluidDef, MaterialDef, Occlusion, RenderClass, WaterParams};
use tesserae::mods::types::LogLevel;

struct Lake;

impl Guest for Lake {
    fn declare() -> Declaration {
        let mut world = tesserae::mods::host::default_world_params();
        assert!(world.water.is_none(), "the engine's default is dry");
        world.ground = Some(GroundParams { rock: "lake:rock".into(), soil: "lake:rock".into(), surface: "lake:rock".into() });
        world.water = Some(WaterParams {
            level_m: world.sea_level_m + world.relief_m / 2.0,
            material: "lake:brine".into(),
        });
        Declaration {
            materials: vec![MaterialDef {
                name: "brine".into(),
                emission: 0,
                dimming: 2,
                occlusion: Occlusion::SameMaterial,
                render: RenderClass::Blend,
                solid: false,
            }, MaterialDef { name: "rock".into(), emission: 0, dimming: 15, occlusion: Occlusion::Opaque, render: RenderClass::Opaque, solid: true }],
            shapes: vec![],
            appearance: vec![],
            world: Some(world),
            view: None,
            fluids: vec![FluidDef {
                material: "lake:brine".into(),
                reach_m: 7.0,
                sideways_m_per_s: 4.0,
                downward_m_per_s: 8.0,
                recession_m_per_s: 4.0,
                reset_drop_m: 0.5,
                waterlogs: true,
                extinguishes: true,
                heat: None,
            }],
            reactions: vec![],
            combustion: vec![],
        }
    }

    fn start(_: WorldInfo, _: Registered) {}

    fn tick(ctx: TickContext) -> Vec<Command> {
        tesserae::mods::host::log(LogLevel::Info, &format!("edits={:?}", ctx.edits));
        vec![]
    }
}

export!(Lake);
