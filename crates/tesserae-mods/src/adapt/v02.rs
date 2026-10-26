//! The 0.2 line carried to 0.3 and back.
//!
//! Everything but two closed types is the same record under another package.
//! Going up, a 0.2 standing-water record names no fluid, so it is dropped with
//! a warning and that World is dry. Going down, the Edit history leaves out
//! ignitions and put-outs, cases 0.2's `edit-kind` does not have.

use crate::bindings::v02::exports::tesserae::mods::guest as old_guest;
use crate::bindings::v02::tesserae::mods::{commands as old_commands, content as old, types as ot};
use crate::bindings::v03::exports::tesserae::mods::guest;
use crate::bindings::v03::tesserae::mods::{commands, content, types};
use crate::log::{LogHandle, LogLevel};

fn vec3(v: ot::Vec3) -> types::Vec3 {
    types::Vec3 {
        x: v.x,
        y: v.y,
        z: v.z,
    }
}

fn box_m(b: ot::BoxM) -> types::BoxM {
    types::BoxM {
        min: vec3(b.min),
        max: vec3(b.max),
    }
}

fn voxel(v: ot::Voxel) -> types::Voxel {
    types::Voxel {
        material: v.material,
        shape: v.shape,
        orientation: v.orientation,
    }
}

fn node_box(b: ot::NodeBox) -> types::NodeBox {
    types::NodeBox {
        min: b.min,
        max: b.max,
    }
}

fn node_boxes(boxes: Vec<ot::NodeBox>) -> Vec<types::NodeBox> {
    boxes.into_iter().map(node_box).collect()
}

fn collisions(c: Option<Vec<ot::CollisionBox>>) -> Option<Vec<types::CollisionBox>> {
    c.map(|c| {
        c.into_iter()
            .map(|b| types::CollisionBox {
                min: b.min,
                max: b.max,
            })
            .collect()
    })
}

fn shape(s: old::ShapeDef) -> content::ShapeDef {
    content::ShapeDef {
        name: s.name,
        boxes: node_boxes(s.boxes),
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
                    boxes: node_boxes(a.boxes),
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

fn world(p: old::WorldParams, log: &LogHandle) -> content::WorldParams {
    log.push(LogLevel::Warn, "World parameters name no ground before contract line 0.3, so this World has no ground; name rock, soil and surface on 0.3");
    if let Some(water) = p.water {
        log.push(
            LogLevel::Warn,
            &format!(
                "standing water at {} m names no fluid before contract line 0.3, so this \
                 World has no lakes; name the lake's fluid on 0.3",
                water.level_m
            ),
        );
    }
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
        ground: None,
        water: None,
    }
}

fn heat(h: old::HeatDef) -> content::HeatDef {
    content::HeatDef {
        strength: h.strength,
        reach_m: h.reach_m,
    }
}

fn fluid(f: old::FluidDef) -> content::FluidDef {
    content::FluidDef {
        material: f.material,
        reach_m: f.reach_m,
        sideways_m_per_s: f.sideways_m_per_s,
        downward_m_per_s: f.downward_m_per_s,
        recession_m_per_s: f.recession_m_per_s,
        reset_drop_m: f.reset_drop_m,
        waterlogs: f.waterlogs,
        extinguishes: f.extinguishes,
        heat: f.heat.map(heat),
    }
}

fn side(s: old::ReactionSide) -> content::ReactionSide {
    match s {
        old::ReactionSide::First => content::ReactionSide::First,
        old::ReactionSide::Second => content::ReactionSide::Second,
    }
}

fn reactant(r: old::Reactant) -> content::Reactant {
    content::Reactant {
        fluid: r.fluid,
        state: match r.state {
            old::FluidState::Any => content::FluidState::Any,
            old::FluidState::Source => content::FluidState::Source,
            old::FluidState::Flowing => content::FluidState::Flowing,
        },
    }
}

fn reaction(r: old::ReactionDef) -> content::ReactionDef {
    content::ReactionDef {
        first: reactant(r.first),
        second: reactant(r.second),
        consumed: side(r.consumed),
        product: r.product,
        receiving: r.receiving.map(side),
    }
}

fn combustion(c: old::CombustionDef) -> content::CombustionDef {
    content::CombustionDef {
        fuel: c.fuel,
        ignition_threshold: c.ignition_threshold,
        ignition_delay_s: c.ignition_delay_s,
        emission: c.emission,
        heat: c.heat.map(heat),
        end: match c.end {
            old::BurnEnd::After(after) => content::BurnEnd::After(content::BurnAfter {
                seconds: after.seconds,
                residue: after.residue,
            }),
            old::BurnEnd::Permanent => content::BurnEnd::Permanent,
        },
    }
}

/// The 0.3 Declaration a 0.2 one means; a dropped lake is warned of in `log`.
pub(crate) fn declaration(d: old::Declaration, log: &LogHandle) -> content::Declaration {
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
        world: d.world.map(|w| world(w, log)),
        view: d.view.map(|v| content::ViewParams {
            view_distance_m: v.view_distance_m,
            lod_distance_m: v.lod_distance_m,
        }),
        fluids: d.fluids.into_iter().map(fluid).collect(),
        reactions: d.reactions.into_iter().map(reaction).collect(),
        combustion: d.combustion.into_iter().map(combustion).collect(),
    }
}

impl From<old_commands::Command> for commands::Command {
    fn from(c: old_commands::Command) -> Self {
        match c {
            old_commands::Command::Fill(fill) => commands::Command::Fill(commands::Fill {
                region: box_m(fill.region),
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
            old_commands::Command::PlaceSources(fill) => {
                commands::Command::PlaceSources(commands::SourceFill {
                    region: box_m(fill.region),
                    material: fill.material,
                })
            }
            old_commands::Command::RemoveSources(b) => commands::Command::RemoveSources(box_m(b)),
            old_commands::Command::Ignite(b) => commands::Command::Ignite(box_m(b)),
            old_commands::Command::Extinguish(b) => commands::Command::Extinguish(box_m(b)),
        }
    }
}

fn old_vec3(v: types::Vec3) -> ot::Vec3 {
    ot::Vec3 {
        x: v.x,
        y: v.y,
        z: v.z,
    }
}

fn old_voxel(v: types::Voxel) -> ot::Voxel {
    ot::Voxel {
        material: v.material,
        shape: v.shape,
        orientation: v.orientation,
    }
}

/// Every Edit 0.2 has a case for; ignitions and put-outs are left out.
impl From<guest::TickContext> for old_guest::TickContext {
    fn from(c: guest::TickContext) -> Self {
        old_guest::TickContext {
            tick: c.tick,
            dt_s: c.dt_s,
            edits: c
                .edits
                .into_iter()
                .filter_map(|e| {
                    let kind = match e.kind {
                        guest::EditKind::Solid(s) => {
                            old_guest::EditKind::Solid(old_guest::SolidEdit {
                                before: old_voxel(s.before),
                                after: old_voxel(s.after),
                            })
                        }
                        guest::EditKind::SourcePlaced(m) => old_guest::EditKind::SourcePlaced(m),
                        guest::EditKind::SourceRemoved(m) => old_guest::EditKind::SourceRemoved(m),
                        guest::EditKind::Ignited(_) | guest::EditKind::PutOut(_) => return None,
                    };
                    Some(old_guest::EditEvent {
                        at: old_vec3(e.at),
                        kind,
                    })
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

/// A lake keeps its level and loses its fluid's name, which 0.2 cannot hold.
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
