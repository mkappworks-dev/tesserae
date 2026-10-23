//! Heat and burning, as table data keyed by the Material that gives off heat
//! or burns.
//!
//! Heat is local exposure, not a temperature: a hot fluid or a burning solid
//! exposes reachable open space within `reach_m` to a bounded, dimensionless
//! strength that falls with distance. A Material burns only where a row here
//! says it does; colour or name never makes one flammable. Lengths are metres
//! and times simulation seconds (`docs/adr/0003-voxel-size-is-per-world.md`).

use crate::light::MAX_LIGHT;
use crate::voxel::MaterialId;

/// The strongest exposure there is, and the highest ignition threshold.
pub const MAX_HEAT: u8 = 15;
/// The farthest heat reaches, so one source's exposure stays a local cost.
pub const MAX_HEAT_REACH_M: f32 = 16.0;
/// The longest an ignition may wait.
pub const MAX_IGNITION_DELAY_S: f32 = 600.0;
/// The longest finite burn: its fuel in logical steps fits a
/// [`crate::Burning`]'s `u16`.
pub const MAX_BURN_S: f32 = 1000.0;

/// Exposure given off: `strength` at the source, falling to nothing over
/// `reach_m` of open path.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Heat {
    /// `1..=MAX_HEAT`.
    pub strength: u8,
    pub reach_m: f32,
}

impl Heat {
    fn check(&self) -> Result<(), CombustionError> {
        if !(1..=MAX_HEAT).contains(&self.strength) {
            return Err(CombustionError::HeatOutOfRange);
        }
        if !(self.reach_m.is_finite() && self.reach_m > 0.0 && self.reach_m <= MAX_HEAT_REACH_M) {
            return Err(CombustionError::HeatOutOfRange);
        }
        Ok(())
    }

    /// For [`crate::FluidDef`], which reports its own error kind.
    pub fn is_valid(&self) -> bool {
        self.check().is_ok()
    }
}

/// How a fire ends.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Burn {
    /// After `seconds` of fuel, the solid's Material becomes `residue`,
    /// keeping its Shape and Orientation; [`crate::AIR`] removes the solid.
    For { seconds: f32, residue: MaterialId },
    /// Never runs out: a deliberate, declared permanent fire.
    Permanent,
}

/// How one solid Material catches fire and burns.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CombustionDef {
    /// Exposure at or above which it starts to ignite, `1..=MAX_HEAT`.
    pub threshold: u8,
    /// Continuous exposure at the threshold before it catches.
    pub ignition_delay_s: f32,
    /// Block light while it burns, `0..=MAX_LIGHT`.
    pub emission: u8,
    /// Exposure it gives off while it burns; `None` burns without spreading.
    pub heat: Option<Heat>,
    pub burn: Burn,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CombustionError {
    /// No Material is registered under the fuel or residue id.
    NotAMaterial,
    /// Fuel and residue are solids: a fluid is not burnt, and a fire leaves
    /// no fluid behind.
    NotSolid,
    ThresholdOutOfRange,
    /// A delay that is not finite, negative, or past [`MAX_IGNITION_DELAY_S`].
    DelayOutOfRange,
    /// A burn that is not finite and positive, or past [`MAX_BURN_S`].
    DurationOutOfRange,
    EmissionOutOfRange,
    /// A strength outside `1..=MAX_HEAT` or a reach that is not finite,
    /// positive and within [`MAX_HEAT_REACH_M`].
    HeatOutOfRange,
    /// A second definition for one fuel.
    AlreadyCombustible,
    /// Residues that lead back to a fuel they came from would burn for ever
    /// without saying so: a permanent fire is [`Burn::Permanent`].
    Cycle,
}

impl std::fmt::Display for CombustionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            CombustionError::NotAMaterial => "no Material is registered under that id",
            CombustionError::NotSolid => "fuel and residue must be solid Materials",
            CombustionError::ThresholdOutOfRange => "the ignition threshold is from 1 to 15",
            CombustionError::DelayOutOfRange => {
                "the ignition delay is finite, not negative and at most 600 s"
            }
            CombustionError::DurationOutOfRange => {
                "a finite burn is finite, positive and at most 1000 s"
            }
            CombustionError::EmissionOutOfRange => "burning emission is at most 15",
            CombustionError::HeatOutOfRange => {
                "heat strength is from 1 to 15 and its reach finite, positive and at most 16 m"
            }
            CombustionError::AlreadyCombustible => {
                "the Material already has a combustion definition"
            }
            CombustionError::Cycle => {
                "residues lead back to their own fuel; declare a permanent fire instead"
            }
        })
    }
}

impl std::error::Error for CombustionError {}

impl CombustionDef {
    /// The fuel a fresh fire has, in logical steps of [`crate::flow::STEP_S`].
    /// A permanent fire never spends it.
    pub fn fuel_steps(&self) -> u16 {
        match self.burn {
            Burn::For { seconds, .. } => (seconds as f64 / crate::flow::STEP_S)
                .round()
                .clamp(1.0, u16::MAX as f64) as u16,
            Burn::Permanent => u16::MAX,
        }
    }

    /// The numbers alone; [`crate::Tables::register_combustion`] checks the
    /// Materials.
    pub fn check(&self) -> Result<(), CombustionError> {
        if !(1..=MAX_HEAT).contains(&self.threshold) {
            return Err(CombustionError::ThresholdOutOfRange);
        }
        let delay = self.ignition_delay_s;
        if !(delay.is_finite() && (0.0..=MAX_IGNITION_DELAY_S).contains(&delay)) {
            return Err(CombustionError::DelayOutOfRange);
        }
        if self.emission > MAX_LIGHT {
            return Err(CombustionError::EmissionOutOfRange);
        }
        if let Some(heat) = &self.heat {
            heat.check()?;
        }
        if let Burn::For { seconds, .. } = self.burn
            && !(seconds.is_finite() && seconds > 0.0 && seconds <= MAX_BURN_S)
        {
            return Err(CombustionError::DurationOutOfRange);
        }
        Ok(())
    }
}

/// Every Material that burns, by id. Immutable once the World opens.
#[derive(Clone, Debug, Default)]
pub struct CombustionTable {
    defs: Vec<Option<CombustionDef>>,
}

impl CombustionTable {
    pub(crate) fn insert(
        &mut self,
        fuel: MaterialId,
        def: CombustionDef,
    ) -> Result<(), CombustionError> {
        def.check()?;
        if self.contains(fuel) {
            return Err(CombustionError::AlreadyCombustible);
        }
        let i = fuel as usize;
        if self.defs.len() <= i {
            self.defs.resize(i + 1, None);
        }
        self.defs[i] = Some(def);
        // A cycle can only close through the row just added.
        if self.leads_back(fuel) {
            self.defs[i] = None;
            return Err(CombustionError::Cycle);
        }
        Ok(())
    }

    fn leads_back(&self, fuel: MaterialId) -> bool {
        let mut at = fuel;
        // Each step visits a different fuel or the walk has looped.
        for _ in 0..=self.defs.len() {
            match self.get(at).map(|d| d.burn) {
                Some(Burn::For { residue, .. }) if residue == fuel => return true,
                Some(Burn::For { residue, .. }) => at = residue,
                _ => return false,
            }
        }
        false
    }

    pub fn contains(&self, material: MaterialId) -> bool {
        self.get(material).is_some()
    }

    pub fn get(&self, material: MaterialId) -> Option<&CombustionDef> {
        self.defs.get(material as usize).and_then(Option::as_ref)
    }

    /// Every fuel, in Material id order.
    pub fn iter(&self) -> impl Iterator<Item = (MaterialId, &CombustionDef)> {
        self.defs
            .iter()
            .enumerate()
            .filter_map(|(i, def)| Some((i as MaterialId, def.as_ref()?)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WOOD: CombustionDef = CombustionDef {
        threshold: 6,
        ignition_delay_s: 2.0,
        emission: 12,
        heat: Some(Heat {
            strength: 10,
            reach_m: 2.0,
        }),
        burn: Burn::For {
            seconds: 8.0,
            residue: 0,
        },
    };

    #[test]
    fn numbers_outside_their_ranges_are_refused() {
        let heat = |strength, reach_m| Some(Heat { strength, reach_m });
        let burn = |seconds| Burn::For {
            seconds,
            residue: 0,
        };
        for (def, error) in [
            (
                CombustionDef {
                    threshold: 0,
                    ..WOOD
                },
                CombustionError::ThresholdOutOfRange,
            ),
            (
                CombustionDef {
                    threshold: 16,
                    ..WOOD
                },
                CombustionError::ThresholdOutOfRange,
            ),
            (
                CombustionDef {
                    ignition_delay_s: -1.0,
                    ..WOOD
                },
                CombustionError::DelayOutOfRange,
            ),
            (
                CombustionDef {
                    ignition_delay_s: f32::NAN,
                    ..WOOD
                },
                CombustionError::DelayOutOfRange,
            ),
            (
                CombustionDef {
                    emission: 16,
                    ..WOOD
                },
                CombustionError::EmissionOutOfRange,
            ),
            (
                CombustionDef {
                    heat: heat(0, 2.0),
                    ..WOOD
                },
                CombustionError::HeatOutOfRange,
            ),
            (
                CombustionDef {
                    heat: heat(5, f32::INFINITY),
                    ..WOOD
                },
                CombustionError::HeatOutOfRange,
            ),
            (
                CombustionDef {
                    heat: heat(5, 17.0),
                    ..WOOD
                },
                CombustionError::HeatOutOfRange,
            ),
            (
                CombustionDef {
                    burn: burn(0.0),
                    ..WOOD
                },
                CombustionError::DurationOutOfRange,
            ),
            (
                CombustionDef {
                    burn: burn(1001.0),
                    ..WOOD
                },
                CombustionError::DurationOutOfRange,
            ),
        ] {
            assert_eq!(def.check(), Err(error), "{def:?}");
        }
        assert_eq!(WOOD.check(), Ok(()));
        assert_eq!(WOOD.fuel_steps(), 512, "8 s of 64 Hz steps");
        let longest = CombustionDef {
            burn: burn(MAX_BURN_S),
            ..WOOD
        };
        assert_eq!(longest.fuel_steps(), 64_000);
        let instant = CombustionDef {
            ignition_delay_s: 0.0,
            heat: None,
            burn: Burn::Permanent,
            ..WOOD
        };
        assert_eq!(instant.check(), Ok(()));
    }

    #[test]
    fn a_second_row_or_a_residue_cycle_is_refused_and_leaves_no_row() {
        let into = |residue| CombustionDef {
            burn: Burn::For {
                seconds: 8.0,
                residue,
            },
            ..WOOD
        };
        let mut table = CombustionTable::default();
        assert_eq!(table.insert(10, into(11)), Ok(()));
        assert_eq!(
            table.insert(10, into(0)),
            Err(CombustionError::AlreadyCombustible)
        );
        assert_eq!(table.insert(11, into(12)), Ok(()));
        assert_eq!(table.insert(12, into(10)), Err(CombustionError::Cycle));
        assert!(!table.contains(12));
        assert_eq!(table.insert(13, into(13)), Err(CombustionError::Cycle));
        // Charcoal that burns to ash is a chain, not a cycle.
        assert_eq!(table.insert(12, into(0)), Ok(()));
        assert_eq!(
            table.iter().map(|(m, _)| m).collect::<Vec<_>>(),
            [10, 11, 12]
        );
    }
}
