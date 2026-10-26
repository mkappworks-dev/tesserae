//! Declaration types to `tesserae-core`'s, and back where the contract hands
//! the engine's own data to a Mod. Field for field, no clamping: validation is
//! the registration's, so a bad row is reported by the table that owns the rule.

use tesserae_core::{
    Arm, CaveParams, CollisionBox, ConnectDef, MaterialDef, NodeBox, Occlusion, RenderClass,
    ShapeDef, SmoothingParams, Voxel, WorldgenParams,
};

use crate::bindings::exports::tesserae::mods::guest;
use crate::bindings::tesserae::mods::{commands, content, types};
use crate::command::{Command, Placement, Region};
use crate::context::{EditEvent, EditKind, TickContext};

fn arr_u8((x, y, z): (u8, u8, u8)) -> [u8; 3] {
    [x, y, z]
}

fn arr_i8((x, y, z): (i8, i8, i8)) -> [i8; 3] {
    [x, y, z]
}

fn tup_u8([x, y, z]: [u8; 3]) -> (u8, u8, u8) {
    (x, y, z)
}

fn tup_i8([x, y, z]: [i8; 3]) -> (i8, i8, i8) {
    (x, y, z)
}

impl From<types::NodeBox> for NodeBox {
    fn from(b: types::NodeBox) -> Self {
        NodeBox::new(arr_u8(b.min), arr_u8(b.max))
    }
}

impl From<NodeBox> for types::NodeBox {
    fn from(b: NodeBox) -> Self {
        types::NodeBox {
            min: tup_u8(b.min),
            max: tup_u8(b.max),
        }
    }
}

impl From<types::CollisionBox> for CollisionBox {
    fn from(b: types::CollisionBox) -> Self {
        CollisionBox::new(arr_i8(b.min), arr_i8(b.max))
    }
}

impl From<CollisionBox> for types::CollisionBox {
    fn from(b: CollisionBox) -> Self {
        types::CollisionBox {
            min: tup_i8(b.min),
            max: tup_i8(b.max),
        }
    }
}

impl From<types::Arm> for Arm {
    fn from(a: types::Arm) -> Self {
        Arm {
            face: a.face.into(),
            boxes: a.boxes.into_iter().map(Into::into).collect(),
            collision: a.collision.map(|c| c.into_iter().map(Into::into).collect()),
        }
    }
}

impl From<types::ConnectDef> for ConnectDef {
    fn from(c: types::ConnectDef) -> Self {
        ConnectDef {
            group: c.group,
            connects_to: c.connects_to,
            to_sealed: c.to_sealed,
            arms: c.arms.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<content::ShapeDef> for ShapeDef {
    fn from(s: content::ShapeDef) -> Self {
        ShapeDef {
            connect: s.connect.map(Into::into),
            boxes: s.boxes.into_iter().map(Into::into).collect(),
            collision: s.collision.map(|c| c.into_iter().map(Into::into).collect()),
        }
    }
}

impl From<content::Occlusion> for Occlusion {
    fn from(o: content::Occlusion) -> Self {
        match o {
            content::Occlusion::Opaque => Occlusion::Opaque,
            content::Occlusion::SameMaterial => Occlusion::SameMaterial,
            content::Occlusion::Never => Occlusion::Never,
        }
    }
}

impl From<content::RenderClass> for RenderClass {
    fn from(r: content::RenderClass) -> Self {
        match r {
            content::RenderClass::Opaque => RenderClass::Opaque,
            content::RenderClass::Blend => RenderClass::Blend,
            content::RenderClass::Cutout => RenderClass::Cutout,
        }
    }
}

impl From<content::MaterialDef> for MaterialDef {
    fn from(m: content::MaterialDef) -> Self {
        MaterialDef {
            emission: m.emission,
            dimming: m.dimming,
            occlusion: m.occlusion.into(),
            render: m.render.into(),
            solid: m.solid,
        }
    }
}

/// A Game's terrain, and the lake it names apart: the name resolves only once
/// every Mod has declared.
pub(crate) fn world_params(
    p: content::WorldParams,
) -> (WorldgenParams, Option<content::WaterParams>) {
    let terrain = WorldgenParams {
        sea_level_m: p.sea_level_m,
        relief_m: p.relief_m,
        feature_size_m: p.feature_size_m,
        soil_depth_m: p.soil_depth_m,
        octaves: p.octaves,
        caves: p.caves.map(|c| CaveParams {
            feature_size_m: c.feature_size_m,
            radius_m: c.radius_m,
            floor_m: c.floor_m,
            octaves: c.octaves,
        }),
        smoothing: p.smoothing.map(|s| SmoothingParams {
            min_rise_m: s.min_rise_m,
        }),
        ground: None,
        water: None,
    };
    (terrain, p.water)
}

/// What `default-world-params` returns: the engine's own terrain parameters.
pub(crate) fn engine_world_params() -> content::WorldParams {
    let p = WorldgenParams::default();
    // A lake would need its fluid's name, and the engine declares no fluid.
    debug_assert!(p.water.is_none());
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
        ground: p.ground.map(|_| content::GroundParams {
            rock: "core:stone".into(), soil: "core:dirt".into(), surface: "core:grass".into(),
        }),
        water: None,
    }
}

fn array(v: types::Vec3) -> [f32; 3] {
    [v.x, v.y, v.z]
}

fn vec3([x, y, z]: [f32; 3]) -> types::Vec3 {
    types::Vec3 { x, y, z }
}

fn region(b: types::BoxM) -> Region {
    Region {
        min: array(b.min),
        max: array(b.max),
    }
}

impl From<types::Voxel> for Voxel {
    fn from(v: types::Voxel) -> Self {
        Voxel::new(v.material, v.shape, v.orientation)
    }
}

impl From<Voxel> for types::Voxel {
    fn from(v: Voxel) -> Self {
        types::Voxel {
            material: v.material(),
            shape: v.shape(),
            orientation: v.orientation(),
        }
    }
}

impl From<commands::Command> for Command {
    fn from(c: commands::Command) -> Self {
        match c {
            commands::Command::Fill(fill) => Command::Fill {
                region: region(fill.region),
                voxel: fill.voxel.into(),
            },
            commands::Command::PlaceSources(fill) => Command::PlaceSources {
                region: region(fill.region),
                material: fill.material,
            },
            commands::Command::RemoveSources(b) => Command::RemoveSources(region(b)),
            commands::Command::Ignite(b) => Command::Ignite(region(b)),
            commands::Command::Extinguish(b) => Command::Extinguish(region(b)),
            commands::Command::Place(placements) => Command::Place(
                placements
                    .into_iter()
                    .map(|p| Placement {
                        at: array(p.at),
                        voxel: p.voxel.into(),
                    })
                    .collect(),
            ),
        }
    }
}

impl From<TickContext> for guest::TickContext {
    fn from(c: TickContext) -> Self {
        guest::TickContext {
            tick: c.tick,
            dt_s: c.dt_s,
            edits: c.edits.into_iter().map(Into::into).collect(),
            last: guest::BatchReport {
                applied: c.last.applied,
                rejected: c.last.rejected,
            },
        }
    }
}

impl From<EditEvent> for guest::EditEvent {
    fn from(e: EditEvent) -> Self {
        guest::EditEvent {
            at: vec3(e.at),
            kind: match e.kind {
                EditKind::Solid { before, after } => guest::EditKind::Solid(guest::SolidEdit {
                    before: before.into(),
                    after: after.into(),
                }),
                EditKind::SourcePlaced(material) => guest::EditKind::SourcePlaced(material),
                EditKind::SourceRemoved(material) => guest::EditKind::SourceRemoved(material),
                EditKind::Ignited(fuel) => guest::EditKind::Ignited(fuel),
                EditKind::PutOut(fuel) => guest::EditKind::PutOut(fuel),
            },
        }
    }
}
