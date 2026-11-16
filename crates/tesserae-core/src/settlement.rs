//! Settlements: rules world generation lays out as roads and Plots around a
//! centre, each Plot a Template. ADR-0029.

use crate::template::Template;
use crate::voxel::Voxel;

pub const MAX_SETTLEMENT_RADIUS_M: f32 = 64.0;
pub const MAX_PLOTS: u8 = 24;
/// Settlements in a World, from every Mod together.
pub const MAX_SETTLEMENTS: usize = 16;
/// How far past a Settlement's disc single Structures keep out, unless it says.
pub const DEFAULT_SETTLEMENT_MARGIN_M: f32 = 16.0;
pub const MAX_SETTLEMENT_MARGIN_M: f32 = 64.0;

/// A Template a Settlement stands, with the cell of its door if it has one.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Building {
    /// An index into the World's Templates.
    pub template: u16,
    /// The door's cell `[x, z]` in the unturned Template; the Template's edge
    /// nearest it is the side that faces the road.
    pub door: Option<[u16; 2]>,
}

/// One weighted entry a Settlement draws its Plots from.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct PlotRule {
    pub building: Building,
    pub weight: f32,
    pub min: u8,
    pub max: u8,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Settlement {
    /// `<mod id>:<name>`.
    pub name: String,
    pub radius_m: f32,
    /// Single Structures keep this far past the disc.
    pub margin_m: f32,
    pub centre: Option<Building>,
    /// How many Plots it lays at most, besides its centre.
    pub plots: u8,
    pub rules: Vec<PlotRule>,
    /// Below this many Plots, besides its centre, the Settlement is dropped.
    pub minimum: u8,
    /// What its roads are laid in on level ground.
    pub path: Voxel,
}

impl Settlement {
    /// Its disc and margin, in metres.
    pub fn reach_m(&self) -> f32 {
        self.radius_m + self.margin_m
    }

    /// Checked against the World's Templates, which every [`Building`] indexes.
    pub fn validate(&self, templates: &[Template]) -> Result<(), String> {
        if !(self.radius_m > 0.0 && self.radius_m <= MAX_SETTLEMENT_RADIUS_M) {
            return Err(format!(
                "radius-m must be above 0 and at most {MAX_SETTLEMENT_RADIUS_M} m"
            ));
        }
        if !(0.0..=MAX_SETTLEMENT_MARGIN_M).contains(&self.margin_m) {
            return Err(format!(
                "margin-m must be from 0 to {MAX_SETTLEMENT_MARGIN_M} m"
            ));
        }
        if self.plots > MAX_PLOTS {
            return Err(format!(
                "lays {} Plots; a Settlement lays at most {MAX_PLOTS}",
                self.plots
            ));
        }
        if self.rules.is_empty() || self.rules.len() > usize::from(MAX_PLOTS) {
            return Err(format!("draws its Plots from 1 to {MAX_PLOTS} Templates"));
        }
        if self.minimum > self.plots {
            return Err("its minimum is more Plots than it lays".into());
        }
        let mins: u32 = self.rules.iter().map(|r| u32::from(r.min)).sum();
        if mins > u32::from(self.plots) {
            return Err("its Templates' minimums add up to more Plots than it lays".into());
        }
        let buildings = self
            .centre
            .iter()
            .chain(self.rules.iter().map(|r| &r.building));
        for building in buildings {
            let template = templates
                .get(usize::from(building.template))
                .ok_or("names a missing Template")?;
            let [x, _, z] = template.size_m();
            if x.max(z) > self.radius_m {
                return Err(format!(
                    "`{}` is wider than the Settlement's radius",
                    template.name
                ));
            }
            let [w, _, d] = template.size();
            if building
                .door
                .is_some_and(|[dx, dz]| usize::from(dx) >= w || usize::from(dz) >= d)
            {
                return Err(format!("`{}`: its door is outside it", template.name));
            }
        }
        for rule in &self.rules {
            if !(rule.weight.is_finite() && rule.weight > 0.0) {
                return Err("a Template's weight must be above 0".into());
            }
            if rule.min > rule.max {
                return Err("a Template's min must be at most its max".into());
            }
        }
        Ok(())
    }
}
