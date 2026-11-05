//! The Game's rules for Survival: how much a Player can take, and what costs
//! them health and hunger. Health and hunger are points, distances metres and
//! times seconds.

use crate::ParamError;

/// The most any one rate, maximum or food value may be.
pub const MAX_SURVIVAL_VALUE: f32 = 1000.0;
pub const MAX_AIR_S: f32 = 600.0;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SurvivalRules {
    pub max_health: f32,
    pub max_hunger: f32,
    pub max_air_s: f32,
    pub safe_fall_m: f32,
    /// Beyond `safe_fall_m`.
    pub fall_damage_per_m: f32,
    /// With no air left.
    pub drowning_damage_per_s: f32,
    pub hunger_per_min: f32,
    pub sprinting_hunger_per_min: f32,
    /// Health regenerates while hunger is at least this.
    pub regeneration_hunger: f32,
    /// Health per second while hunger is at least `regeneration_hunger`.
    pub regeneration_per_s: f32,
    /// At zero hunger.
    pub starvation_damage_per_s: f32,
    pub keep_inventory: bool,
}

impl Default for SurvivalRules {
    fn default() -> Self {
        Self {
            max_health: 20.0,
            max_hunger: 20.0,
            max_air_s: 10.0,
            safe_fall_m: 3.0,
            fall_damage_per_m: 1.0,
            drowning_damage_per_s: 2.0,
            hunger_per_min: 1.0,
            sprinting_hunger_per_min: 2.0,
            regeneration_hunger: 18.0,
            regeneration_per_s: 0.25,
            starvation_damage_per_s: 0.25,
            keep_inventory: false,
        }
    }
}

impl SurvivalRules {
    pub fn validate(&self) -> Result<(), ParamError> {
        const POINTS: &str = "must be finite and from 1 to 1000";
        const RATE: &str = "must be finite and from 0 to 1000";
        let max = MAX_SURVIVAL_VALUE;
        for (field, value, range, rule) in [
            ("max-health", self.max_health, 1.0..=max, POINTS),
            ("max-hunger", self.max_hunger, 1.0..=max, POINTS),
            (
                "max-air-s",
                self.max_air_s,
                1.0..=MAX_AIR_S,
                "must be finite and from 1 to 600 s",
            ),
            (
                "safe-fall-m",
                self.safe_fall_m,
                0.0..=max,
                "must be finite and from 0 to 1000 m",
            ),
            ("fall-damage-per-m", self.fall_damage_per_m, 0.0..=max, RATE),
            (
                "drowning-damage-per-s",
                self.drowning_damage_per_s,
                0.0..=max,
                RATE,
            ),
            ("hunger-per-min", self.hunger_per_min, 0.0..=max, RATE),
            (
                "sprinting-hunger-per-min",
                self.sprinting_hunger_per_min,
                0.0..=max,
                RATE,
            ),
            (
                "regeneration-hunger",
                self.regeneration_hunger,
                0.0..=self.max_hunger,
                "must be finite and from 0 to max-hunger",
            ),
            (
                "regeneration-per-s",
                self.regeneration_per_s,
                0.0..=max,
                RATE,
            ),
            (
                "starvation-damage-per-s",
                self.starvation_damage_per_s,
                0.0..=max,
                RATE,
            ),
        ] {
            if !range.contains(&value) {
                return Err(ParamError { field, rule });
            }
        }
        Ok(())
    }
}
