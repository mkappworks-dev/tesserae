//! Fluid behaviour, as table data keyed by the Material that flows.
//!
//! A fluid is a registered Material (`docs/adr/0009-materials-are-table-data.md`)
//! with a row here saying how it moves. Lengths are metres and rates metres per
//! simulation second, so a World of any Voxel size runs the same water:
//! `docs/adr/0003-voxel-size-is-per-world.md`.

use crate::voxel::MaterialId;

/// How a fluid Material flows.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct FluidDef {
    /// How far a stream spreads sideways from where it last fell, in metres.
    pub reach_m: f32,
    pub sideways_m_per_s: f32,
    pub downward_m_per_s: f32,
    pub recession_m_per_s: f32,
    /// Gravity descent that renews `reach_m`, in metres. Crossing a Voxel
    /// boundary alone renews nothing.
    pub reset_drop_m: f32,
}

impl FluidDef {
    /// Initial tuning; the defaults are set from measurement before the phase
    /// closes, and no accepted value may hide a Voxel-size dependence.
    pub const WATER: FluidDef = FluidDef {
        reach_m: 7.0,
        sideways_m_per_s: 4.0,
        downward_m_per_s: 8.0,
        recession_m_per_s: 4.0,
        reset_drop_m: 0.5,
    };
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FluidError {
    /// A reach, rate or drop that is not a finite positive number.
    NotPositive,
    /// Two definitions for one Material contradict each other by existing.
    AlreadyAFluid,
    /// No Material is registered under the id.
    NotAMaterial,
    /// A solid Material cannot also flow.
    Solid,
}

impl std::fmt::Display for FluidError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            FluidError::NotPositive => {
                "a fluid's reach, rates and reset drop are finite and positive"
            }
            FluidError::AlreadyAFluid => "the Material already has a fluid definition",
            FluidError::NotAMaterial => "no Material is registered under that id",
            FluidError::Solid => "a solid Material cannot flow",
        })
    }
}

impl std::error::Error for FluidError {}

/// Every Material that flows, by id.
#[derive(Clone, Debug)]
pub struct FluidTable {
    defs: Vec<Option<FluidDef>>,
}

impl Default for FluidTable {
    fn default() -> Self {
        Self::builtin()
    }
}

impl FluidTable {
    pub fn builtin() -> Self {
        let mut table = FluidTable { defs: Vec::new() };
        table.set(crate::material::WATER, FluidDef::WATER);
        table
    }

    /// [`crate::Tables::register_fluid`] also checks the Material; this alone
    /// checks the numbers.
    pub fn register(&mut self, material: MaterialId, def: FluidDef) -> Result<(), FluidError> {
        let positive = |v: f32| v.is_finite() && v > 0.0;
        if ![
            def.reach_m,
            def.sideways_m_per_s,
            def.downward_m_per_s,
            def.recession_m_per_s,
            def.reset_drop_m,
        ]
        .into_iter()
        .all(positive)
        {
            return Err(FluidError::NotPositive);
        }
        if self.contains(material) {
            return Err(FluidError::AlreadyAFluid);
        }
        self.set(material, def);
        Ok(())
    }

    fn set(&mut self, material: MaterialId, def: FluidDef) {
        let i = material as usize;
        if self.defs.len() <= i {
            self.defs.resize(i + 1, None);
        }
        self.defs[i] = Some(def);
    }

    pub fn contains(&self, material: MaterialId) -> bool {
        self.get(material).is_some()
    }

    pub fn get(&self, material: MaterialId) -> Option<&FluidDef> {
        self.defs.get(material as usize).and_then(Option::as_ref)
    }

    /// Every fluid, in Material id order.
    pub fn iter(&self) -> impl Iterator<Item = (MaterialId, &FluidDef)> {
        self.defs
            .iter()
            .enumerate()
            .filter_map(|(i, def)| Some((i as MaterialId, def.as_ref()?)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::WATER;

    #[test]
    fn water_is_the_one_builtin_fluid() {
        let table = FluidTable::builtin();
        assert_eq!(table.get(WATER), Some(&FluidDef::WATER));
        for material in 0..WATER {
            assert!(!table.contains(material), "{material}");
        }
        assert!(!table.contains(WATER + 1));
    }

    /// Non-finite values, bad ranges and contradictory definitions are refused
    /// at registration, before a World opens.
    #[test]
    fn registration_refuses_what_cannot_flow() {
        let mut table = FluidTable::builtin();
        let lava = 8;
        for (name, def) in [
            (
                "nan reach",
                FluidDef {
                    reach_m: f32::NAN,
                    ..FluidDef::WATER
                },
            ),
            (
                "infinite rate",
                FluidDef {
                    sideways_m_per_s: f32::INFINITY,
                    ..FluidDef::WATER
                },
            ),
            (
                "zero downward",
                FluidDef {
                    downward_m_per_s: 0.0,
                    ..FluidDef::WATER
                },
            ),
            (
                "negative recession",
                FluidDef {
                    recession_m_per_s: -1.0,
                    ..FluidDef::WATER
                },
            ),
            (
                "zero drop",
                FluidDef {
                    reset_drop_m: 0.0,
                    ..FluidDef::WATER
                },
            ),
        ] {
            assert_eq!(
                table.register(lava, def),
                Err(FluidError::NotPositive),
                "{name}"
            );
        }
        assert!(!table.contains(lava), "a refused definition left a row");
        assert_eq!(table.register(lava, FluidDef::WATER), Ok(()));
        assert_eq!(
            table.register(lava, FluidDef::WATER),
            Err(FluidError::AlreadyAFluid)
        );
        assert_eq!(
            table.register(WATER, FluidDef::WATER),
            Err(FluidError::AlreadyAFluid)
        );
    }
}
