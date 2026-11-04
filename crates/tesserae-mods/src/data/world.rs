use serde::Deserialize;

use crate::bindings::tesserae::mods::content;

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(super) struct Caves {
    feature_size_m: f32,
    radius_m: f32,
    floor_m: f32,
    octaves: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(super) struct Climate {
    temperature_size_m: f32,
    humidity_size_m: f32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(super) struct Smoothing {
    min_rise_m: f32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(super) struct Water {
    level_m: f32,
    material: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(super) struct Movement {
    allow_flight: bool,
    allow_physics: bool,
    start_in_flight: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(super) struct View {
    view_distance_m: f32,
    lod_distance_m: f32,
}

impl From<Caves> for content::CaveParams {
    fn from(c: Caves) -> Self {
        Self {
            feature_size_m: c.feature_size_m,
            radius_m: c.radius_m,
            floor_m: c.floor_m,
            octaves: c.octaves,
        }
    }
}

impl From<Climate> for content::ClimateParams {
    fn from(c: Climate) -> Self {
        Self {
            temperature_size_m: c.temperature_size_m,
            humidity_size_m: c.humidity_size_m,
        }
    }
}

impl From<Smoothing> for content::SmoothingParams {
    fn from(s: Smoothing) -> Self {
        Self {
            min_rise_m: s.min_rise_m,
        }
    }
}

impl From<Water> for content::WaterParams {
    fn from(w: Water) -> Self {
        Self {
            level_m: w.level_m,
            material: w.material,
        }
    }
}

impl From<Movement> for content::MovementPolicy {
    fn from(m: Movement) -> Self {
        Self {
            allow_flight: m.allow_flight,
            allow_physics: m.allow_physics,
            start_in_flight: m.start_in_flight,
        }
    }
}

impl From<View> for content::ViewParams {
    fn from(v: View) -> Self {
        Self {
            view_distance_m: v.view_distance_m,
            lod_distance_m: v.lod_distance_m,
        }
    }
}
