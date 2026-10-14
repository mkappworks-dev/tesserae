//! Declaration types to `tesserae-core`'s, and back where the contract hands
//! the engine's own data to a Mod. Field for field, no clamping: validation is
//! the registration's, so a bad row is reported by the table that owns the rule.

use tesserae_core::{
    Arm, CaveParams, CollisionBox, ConnectDef, MaterialDef, NodeBox, Occlusion, RenderClass,
    ShapeDef, SmoothingParams, WaterParams, WorldgenParams,
};

use crate::bindings::tesserae::mods::{content, types};

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

impl From<content::WorldParams> for WorldgenParams {
    fn from(p: content::WorldParams) -> Self {
        WorldgenParams {
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
            water: p.water.map(|w| WaterParams { level_m: w.level_m }),
        }
    }
}

impl From<WorldgenParams> for content::WorldParams {
    fn from(p: WorldgenParams) -> Self {
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
}
