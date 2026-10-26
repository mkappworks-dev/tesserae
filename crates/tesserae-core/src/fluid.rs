//! Fluid behaviour, as table data keyed by the Material that flows.
//!
//! A fluid is a registered Material (`docs/adr/0009-materials-are-table-data.md`)
//! with a row here saying how it moves. Lengths are metres and rates metres per
//! simulation second, so a World of any Voxel size runs the same water:
//! `docs/adr/0003-voxel-size-is-per-world.md`.

use crate::combustion::Heat;
use crate::voxel::MaterialId;

/// The farthest a fluid spreads from where it last fell.
pub const MAX_REACH_M: f32 = 64.0;
/// The fastest a fluid moves, which no Server tick could show anyway.
pub const MAX_RATE_M_PER_S: f32 = 256.0;
pub const MAX_RESET_DROP_M: f32 = 16.0;

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
    /// Whether it stands in the voids of a partial Shape. A fluid that does
    /// not is displaced by any solid and flows around them.
    pub waterlogs: bool,
    /// Whether it puts out burning fuel it touches, in the fuel's voids or
    /// across a face against the solid, and keeps that fuel from igniting.
    pub extinguishes: bool,
    /// Exposure it gives off; `None` for a cold fluid.
    pub heat: Option<Heat>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FluidError {
    /// A reach, rate or drop that is not a finite positive number.
    NotPositive,
    /// Past [`MAX_REACH_M`], [`MAX_RATE_M_PER_S`] or [`MAX_RESET_DROP_M`].
    OutOfRange,
    /// Heat outside [`Heat`]'s ranges.
    BadHeat,
    /// Hot and extinguishing at once: it would light what it puts out.
    HotExtinguisher,
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
            FluidError::OutOfRange => {
                "a fluid's reach is at most 64 m, its rates 256 m/s and its reset drop 16 m"
            }
            FluidError::BadHeat => {
                "heat strength is from 1 to 15 and its reach finite, positive and at most 16 m"
            }
            FluidError::HotExtinguisher => "a hot fluid cannot also extinguish",
            FluidError::AlreadyAFluid => "the Material already has a fluid definition",
            FluidError::NotAMaterial => "no Material is registered under that id",
            FluidError::Solid => "a solid Material cannot flow",
        })
    }
}

impl std::error::Error for FluidError {}

/// Every Material that flows, by id, with the order it was registered in.
#[derive(Clone, Debug)]
pub struct FluidTable {
    defs: Vec<Option<(FluidDef, u16)>>,
    registered: u16,
}

impl Default for FluidTable {
    fn default() -> Self {
        Self::builtin()
    }
}

impl FluidTable {
    /// Empty: every fluid is a Game's, declared and validated like any other.
    pub fn builtin() -> Self {
        FluidTable {
            defs: Vec::new(),
            registered: 0,
        }
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
        let rates = [
            def.sideways_m_per_s,
            def.downward_m_per_s,
            def.recession_m_per_s,
        ];
        if def.reach_m > MAX_REACH_M
            || def.reset_drop_m > MAX_RESET_DROP_M
            || rates.into_iter().any(|r| r > MAX_RATE_M_PER_S)
        {
            return Err(FluidError::OutOfRange);
        }
        if let Some(heat) = def.heat {
            if !heat.is_valid() {
                return Err(FluidError::BadHeat);
            }
            if def.extinguishes {
                return Err(FluidError::HotExtinguisher);
            }
        }
        if self.contains(material) {
            return Err(FluidError::AlreadyAFluid);
        }
        let i = material as usize;
        if self.defs.len() <= i {
            self.defs.resize(i + 1, None);
        }
        self.defs[i] = Some((def, self.registered));
        self.registered += 1;
        Ok(())
    }

    pub fn contains(&self, material: MaterialId) -> bool {
        self.get(material).is_some()
    }

    pub fn get(&self, material: MaterialId) -> Option<&FluidDef> {
        self.defs
            .get(material as usize)
            .and_then(Option::as_ref)
            .map(|(def, _)| def)
    }

    /// Where `material` came in the registration order, first registered first: what
    /// decides an otherwise equal claim between two fluids.
    pub fn rank(&self, material: MaterialId) -> Option<u16> {
        self.defs
            .get(material as usize)
            .and_then(Option::as_ref)
            .map(|&(_, rank)| rank)
    }

    /// Every fluid, in Material id order.
    pub fn iter(&self) -> impl Iterator<Item = (MaterialId, &FluidDef)> {
        self.defs
            .iter()
            .enumerate()
            .filter_map(|(i, def)| Some((i as MaterialId, &def.as_ref()?.0)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::RETIRED;
    use crate::testgen::WATER_FLUID;

    #[test]
    fn no_fluid_is_built_in() {
        let table = FluidTable::builtin();
        assert_eq!(table.iter().count(), 0);
        for material in 0..=RETIRED + 1 {
            assert!(!table.contains(material), "{material}");
        }
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
                    ..WATER_FLUID
                },
            ),
            (
                "infinite rate",
                FluidDef {
                    sideways_m_per_s: f32::INFINITY,
                    ..WATER_FLUID
                },
            ),
            (
                "zero downward",
                FluidDef {
                    downward_m_per_s: 0.0,
                    ..WATER_FLUID
                },
            ),
            (
                "negative recession",
                FluidDef {
                    recession_m_per_s: -1.0,
                    ..WATER_FLUID
                },
            ),
            (
                "zero drop",
                FluidDef {
                    reset_drop_m: 0.0,
                    ..WATER_FLUID
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
        assert_eq!(table.register(lava, WATER_FLUID), Ok(()));
        assert_eq!(
            table.register(lava, WATER_FLUID),
            Err(FluidError::AlreadyAFluid)
        );
    }

    #[test]
    fn bounds_heat_and_contradictions_are_refused() {
        use crate::combustion::Heat;
        let mut table = FluidTable::builtin();
        let hot = Some(Heat {
            strength: 12,
            reach_m: 2.0,
        });
        for (name, def, error) in [
            (
                "reach past the bound",
                FluidDef {
                    reach_m: 65.0,
                    ..WATER_FLUID
                },
                FluidError::OutOfRange,
            ),
            (
                "rate past the bound",
                FluidDef {
                    downward_m_per_s: 300.0,
                    ..WATER_FLUID
                },
                FluidError::OutOfRange,
            ),
            (
                "drop past the bound",
                FluidDef {
                    reset_drop_m: 17.0,
                    ..WATER_FLUID
                },
                FluidError::OutOfRange,
            ),
            (
                "heat strength zero",
                FluidDef {
                    heat: Some(Heat {
                        strength: 0,
                        reach_m: 2.0,
                    }),
                    extinguishes: false,
                    ..WATER_FLUID
                },
                FluidError::BadHeat,
            ),
            (
                "hot and extinguishing",
                FluidDef {
                    heat: hot,
                    ..WATER_FLUID
                },
                FluidError::HotExtinguisher,
            ),
        ] {
            assert_eq!(table.register(9, def), Err(error), "{name}");
        }
        let lava = FluidDef {
            heat: hot,
            extinguishes: false,
            ..WATER_FLUID
        };
        assert_eq!(table.register(9, lava), Ok(()));
    }

    /// Registration order, not Material id, ranks fluids.
    #[test]
    fn rank_is_the_registration_order() {
        let mut table = FluidTable::builtin();
        let before = table.iter().count() as u16;
        table.register(12, WATER_FLUID).unwrap();
        table.register(9, WATER_FLUID).unwrap();
        assert_eq!(table.rank(12), Some(before));
        assert_eq!(table.rank(9), Some(before + 1));
        assert_eq!(table.rank(10), None);
        assert_eq!(
            table
                .iter()
                .map(|(m, _)| m)
                .skip(before.into())
                .collect::<Vec<_>>(),
            [9, 12]
        );
    }
}
