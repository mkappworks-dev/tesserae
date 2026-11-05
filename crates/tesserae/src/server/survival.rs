//! The Game's Survival rules and the Materials that hurt to touch.

use bevy::prelude::*;
use tesserae_core::{MaterialId, SurvivalRules};

#[derive(Resource, Clone, Default, Debug)]
pub struct Survival {
    pub rules: SurvivalRules,
    /// Health per second, by Material id.
    contact_damage: Vec<f32>,
}

impl Survival {
    pub fn new(rules: SurvivalRules, contact_damage: Vec<(MaterialId, f32)>) -> Self {
        let mut by_id = Vec::new();
        for (material, damage) in contact_damage {
            let at = usize::from(material);
            by_id.resize(by_id.len().max(at + 1), 0.0);
            by_id[at] = damage;
        }
        Self {
            rules,
            contact_damage: by_id,
        }
    }

    pub fn contact_damage_per_s(&self, material: MaterialId) -> f32 {
        self.contact_damage
            .get(usize::from(material))
            .copied()
            .unwrap_or(0.0)
    }

    pub fn any_contact_damage(&self) -> bool {
        self.contact_damage.iter().any(|&d| d > 0.0)
    }
}
