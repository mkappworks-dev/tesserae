//! The 0.1 line carried to 0.2 and back.
//!
//! A 0.1 Declaration becomes a 0.2 one that declares no fluid, reaction or
//! fire. Its Commands become 0.2 ones in which a fluid Voxel placed is a
//! whole-Voxel source, as it always was. It is shown the packed-Voxel
//! projection of player Edits, which no later line's Edit kinds carry, so that
//! comes from the host's own history; it never sees what the simulation does.

use crate::bindings::v01::exports::tesserae::mods::guest as old_guest;
use crate::bindings::v01::tesserae::mods::{commands as old_commands, content as old, types as ot};
use crate::bindings::v02::exports::tesserae::mods::guest;
use crate::bindings::v02::tesserae::mods::{commands, content, types};
use crate::context::TickContext;

fn node_box(b: ot::NodeBox) -> types::NodeBox {
    types::NodeBox {
        min: b.min,
        max: b.max,
    }
}

fn collision_box(b: ot::CollisionBox) -> types::CollisionBox {
    types::CollisionBox {
        min: b.min,
        max: b.max,
    }
}

fn collisions(c: Option<Vec<ot::CollisionBox>>) -> Option<Vec<types::CollisionBox>> {
    c.map(|c| c.into_iter().map(collision_box).collect())
}

fn shape(s: old::ShapeDef) -> content::ShapeDef {
    content::ShapeDef {
        name: s.name,
        boxes: s.boxes.into_iter().map(node_box).collect(),
        collision: collisions(s.collision),
        connect: s.connect.map(|c| types::ConnectDef {
            group: c.group,
            connects_to: c.connects_to,
            to_sealed: c.to_sealed,
            arms: c
                .arms
                .into_iter()
                .map(|a| types::Arm {
                    face: a.face,
                    boxes: a.boxes.into_iter().map(node_box).collect(),
                    collision: collisions(a.collision),
                })
                .collect(),
        }),
    }
}

fn material(m: old::MaterialDef) -> content::MaterialDef {
    content::MaterialDef {
        name: m.name,
        emission: m.emission,
        dimming: m.dimming,
        occlusion: match m.occlusion {
            old::Occlusion::Opaque => content::Occlusion::Opaque,
            old::Occlusion::SameMaterial => content::Occlusion::SameMaterial,
            old::Occlusion::Never => content::Occlusion::Never,
        },
        render: match m.render {
            old::RenderClass::Opaque => content::RenderClass::Opaque,
            old::RenderClass::Blend => content::RenderClass::Blend,
            old::RenderClass::Cutout => content::RenderClass::Cutout,
        },
        solid: m.solid,
    }
}

fn world(p: old::WorldParams) -> content::WorldParams {
    content::WorldParams {
        sea_level_m: p.sea_level_m,
        relief_m: p.relief_m,
        feature_size_m: p.feature_size_m,
        soil_depth_m: p.soil_depth_m,
        octaves: p.octaves,
        caves: p.caves.map(|c| content::CaveParams {
            feature_size_m: c.feature_size_m,
            radius_m: c.radius_m,
            floor_m: c.floor_m,
            octaves: c.octaves,
        }),
        smoothing: p.smoothing.map(|s| content::SmoothingParams {
            min_rise_m: s.min_rise_m,
        }),
        water: p.water.map(|w| content::WaterParams { level_m: w.level_m }),
    }
}

impl From<old::Declaration> for content::Declaration {
    fn from(d: old::Declaration) -> Self {
        content::Declaration {
            materials: d.materials.into_iter().map(material).collect(),
            shapes: d.shapes.into_iter().map(shape).collect(),
            appearance: d
                .appearance
                .into_iter()
                .map(|a| content::Appearance {
                    material: a.material,
                    color: types::Srgba {
                        r: a.color.r,
                        g: a.color.g,
                        b: a.color.b,
                        a: a.color.a,
                    },
                })
                .collect(),
            world: d.world.map(world),
            view: d.view.map(|v| content::ViewParams {
                view_distance_m: v.view_distance_m,
                lod_distance_m: v.lod_distance_m,
            }),
            fluids: Vec::new(),
            reactions: Vec::new(),
            combustion: Vec::new(),
        }
    }
}

fn vec3(v: ot::Vec3) -> types::Vec3 {
    types::Vec3 {
        x: v.x,
        y: v.y,
        z: v.z,
    }
}

fn voxel(v: ot::Voxel) -> types::Voxel {
    types::Voxel {
        material: v.material,
        shape: v.shape,
        orientation: v.orientation,
    }
}

impl From<old_commands::Command> for commands::Command {
    fn from(c: old_commands::Command) -> Self {
        match c {
            old_commands::Command::Fill(fill) => commands::Command::Fill(commands::Fill {
                region: types::BoxM {
                    min: vec3(fill.region.min),
                    max: vec3(fill.region.max),
                },
                voxel: voxel(fill.voxel),
            }),
            old_commands::Command::Place(placements) => commands::Command::Place(
                placements
                    .into_iter()
                    .map(|p| commands::Placement {
                        at: vec3(p.at),
                        voxel: voxel(p.voxel),
                    })
                    .collect(),
            ),
        }
    }
}

fn old_voxel(v: tesserae_core::Voxel) -> ot::Voxel {
    ot::Voxel {
        material: v.material(),
        shape: v.shape(),
        orientation: v.orientation(),
    }
}

/// Player Edits as packed Voxels, with fluid-tool Edits that left the packed
/// Voxel alone left out.
impl From<TickContext> for old_guest::TickContext {
    fn from(c: TickContext) -> Self {
        old_guest::TickContext {
            tick: c.tick,
            dt_s: c.dt_s,
            edits: c
                .edits
                .into_iter()
                .filter(|e| e.in_voxel_projection())
                .map(|e| old_guest::EditEvent {
                    at: ot::Vec3 {
                        x: e.at[0],
                        y: e.at[1],
                        z: e.at[2],
                    },
                    before: old_voxel(e.before),
                    after: old_voxel(e.after),
                })
                .collect(),
            last: old_guest::BatchReport {
                applied: c.last.applied,
                rejected: c.last.rejected,
            },
        }
    }
}

impl From<guest::WorldInfo> for old_guest::WorldInfo {
    fn from(w: guest::WorldInfo) -> Self {
        old_guest::WorldInfo {
            voxel_size_m: w.voxel_size_m,
            seed: w.seed,
        }
    }
}

impl From<&guest::Registered> for old_guest::Registered {
    fn from(r: &guest::Registered) -> Self {
        let ids = |rows: &[guest::RegisteredId]| {
            rows.iter()
                .map(|row| old_guest::RegisteredId {
                    name: row.name.clone(),
                    id: row.id,
                })
                .collect()
        };
        old_guest::Registered {
            materials: ids(&r.materials),
            shapes: ids(&r.shapes),
        }
    }
}

impl From<content::WorldParams> for old::WorldParams {
    fn from(p: content::WorldParams) -> Self {
        old::WorldParams {
            sea_level_m: p.sea_level_m,
            relief_m: p.relief_m,
            feature_size_m: p.feature_size_m,
            soil_depth_m: p.soil_depth_m,
            octaves: p.octaves,
            caves: p.caves.map(|c| old::CaveParams {
                feature_size_m: c.feature_size_m,
                radius_m: c.radius_m,
                floor_m: c.floor_m,
                octaves: c.octaves,
            }),
            smoothing: p.smoothing.map(|s| old::SmoothingParams {
                min_rise_m: s.min_rise_m,
            }),
            water: p.water.map(|w| old::WaterParams { level_m: w.level_m }),
        }
    }
}

impl From<ot::LogLevel> for types::LogLevel {
    fn from(level: ot::LogLevel) -> Self {
        match level {
            ot::LogLevel::Trace => types::LogLevel::Trace,
            ot::LogLevel::Debug => types::LogLevel::Debug,
            ot::LogLevel::Info => types::LogLevel::Info,
            ot::LogLevel::Warn => types::LogLevel::Warn,
            ot::LogLevel::Error => types::LogLevel::Error,
        }
    }
}
