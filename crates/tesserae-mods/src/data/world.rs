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

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(super) struct Survival {
    max_health: Option<f32>,
    max_hunger: Option<f32>,
    max_air_s: Option<f32>,
    safe_fall_m: Option<f32>,
    fall_damage_per_m: Option<f32>,
    drowning_damage_per_s: Option<f32>,
    hunger_per_min: Option<f32>,
    sprinting_hunger_per_min: Option<f32>,
    regeneration_hunger: Option<f32>,
    regeneration_per_s: Option<f32>,
    starvation_damage_per_s: Option<f32>,
    keep_inventory: Option<bool>,
}

impl From<Survival> for content::SurvivalParams {
    fn from(s: Survival) -> Self {
        Self {
            max_health: s.max_health,
            max_hunger: s.max_hunger,
            max_air_s: s.max_air_s,
            safe_fall_m: s.safe_fall_m,
            fall_damage_per_m: s.fall_damage_per_m,
            drowning_damage_per_s: s.drowning_damage_per_s,
            hunger_per_min: s.hunger_per_min,
            sprinting_hunger_per_min: s.sprinting_hunger_per_min,
            regeneration_hunger: s.regeneration_hunger,
            regeneration_per_s: s.regeneration_per_s,
            starvation_damage_per_s: s.starvation_damage_per_s,
            keep_inventory: s.keep_inventory,
        }
    }
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
