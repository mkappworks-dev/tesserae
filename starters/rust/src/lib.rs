wit_bindgen::generate!({ path: "wit", world: "mod" });

use exports::tesserae::mods::guest::{
    Command, Declaration, Guest, Registered, TickContext, WorldInfo,
};
use tesserae::mods::content::{
    Appearance, DoorDef, GroundParams, MaterialDef, MovementPolicy, Occlusion, RecipeDef, RecipeStack,
    RenderClass, VoxelUse, VoxelUseKind,
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

fn stack(item: &str, count: u32) -> RecipeStack {
    RecipeStack {
        item: item.into(),
        count,
    }
}

fn voxel_use(material: &str, kind: VoxelUseKind) -> VoxelUse {
    VoxelUse {
        material: material.into(),
        kind,
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
            materials: vec![
                solid("stone"),
                solid("turf"),
                solid("workbench"),
                solid("chest"),
                solid("door"),
                MaterialDef {
                    name: "door_open".into(),
                    emission: 0,
                    dimming: 0,
                    occlusion: Occlusion::Never,
                    render: RenderClass::Blend,
                    remove_time_s: Some(0.35),
                    contact_damage_per_s: None,
                    solid: true,
                },
            ],
            shapes: Vec::new(),
            appearance: vec![
                appearance("stone", [110, 115, 125, 255]),
                appearance("turf", [90, 160, 60, 255]),
                appearance("workbench", [167, 116, 63, 255]),
                appearance("chest", [107, 68, 35, 255]),
                appearance("door", [128, 88, 50, 255]),
                appearance("door_open", [128, 88, 50, 90]),
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
            soundscapes: Vec::new(),
            cause_sounds: Vec::new(),
            recipes: vec![
                RecipeDef {
                    inputs: vec![stack("starter_rust:stone", 4)],
                    output: stack("starter_rust:workbench", 1),
                    station: "hand".into(),
                },
                RecipeDef {
                    inputs: vec![stack("starter_rust:stone", 8)],
                    output: stack("starter_rust:chest", 1),
                    station: "workbench".into(),
                },
            ],
            voxel_uses: vec![
                voxel_use(
                    "starter_rust:workbench",
                    VoxelUseKind::Station("workbench".into()),
                ),
                voxel_use("starter_rust:chest", VoxelUseKind::Container(Some(27))),
                voxel_use(
                    "starter_rust:door",
                    VoxelUseKind::Door(DoorDef {
                        open: "starter_rust:door_open".into(),
                        width_m: None,
                        height_m: None,
                    }),
                ),
            ],
            ores: Vec::new(),
        }
    }

    fn start(_world: WorldInfo, _ids: Registered) {}

    fn tick(_ctx: TickContext) -> Vec<Command> {
        Vec::new()
    }
}

export!(Starter);
