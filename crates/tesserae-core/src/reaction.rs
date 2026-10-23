//! What two fluids do on contact, as table data.
//!
//! A rule names a pair of fluid Materials, which of them is consumed, and an
//! optional solid left where it was. Rules are stored in one canonical
//! orientation, so a pair declared either way round is the same rule, and at
//! most one rule can match any contact.

use crate::voxel::MaterialId;

/// Which states of a fluid a rule matches.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Phase {
    Any,
    Source,
    Flowing,
}

impl Phase {
    pub fn matches(self, source: bool) -> bool {
        match self {
            Phase::Any => true,
            Phase::Source => source,
            Phase::Flowing => !source,
        }
    }

    fn overlaps(self, other: Phase) -> bool {
        self == Phase::Any || other == Phase::Any || self == other
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Reactant {
    pub fluid: MaterialId,
    pub phase: Phase,
}

/// One participant of a rule, by its place in it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Side {
    First,
    Second,
}

impl Side {
    fn flip(self) -> Side {
        match self {
            Side::First => Side::Second,
            Side::Second => Side::First,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ReactionDef {
    pub first: Reactant,
    pub second: Reactant,
    /// The participant whose fluid the contact removes.
    pub consumed: Side,
    /// A solid Material placed, as a cube, where the consumed fluid was.
    pub product: Option<MaterialId>,
    /// When set, the rule matches only a contact made by the other
    /// participant arriving at this one; unset, either way round.
    pub receiving: Option<Side>,
}

impl ReactionDef {
    /// The same rule with the lower fluid id first.
    pub fn canonical(self) -> ReactionDef {
        if self.first.fluid <= self.second.fluid {
            return self;
        }
        ReactionDef {
            first: self.second,
            second: self.first,
            consumed: self.consumed.flip(),
            product: self.product,
            receiving: self.receiving.map(Side::flip),
        }
    }

    /// Whether some contact could match both rules, both canonical.
    fn overlaps(&self, other: &ReactionDef) -> bool {
        self.first.fluid == other.first.fluid
            && self.second.fluid == other.second.fluid
            && self.first.phase.overlaps(other.first.phase)
            && self.second.phase.overlaps(other.second.phase)
            && match (self.receiving, other.receiving) {
                (Some(a), Some(b)) => a == b,
                _ => true,
            }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ReactionError {
    /// A participant whose Material has no fluid definition.
    NotAFluid,
    /// Both participants are one fluid, which already admits its own flow.
    SameFluid,
    /// A product that is not a registered solid Material.
    ProductNotSolid,
    /// The same rule twice, declared either way round.
    Duplicate,
    /// Two different rules that could match one contact.
    Conflict,
}

impl std::fmt::Display for ReactionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ReactionError::NotAFluid => "both participants must be fluids",
            ReactionError::SameFluid => "a fluid does not react with itself",
            ReactionError::ProductNotSolid => "the product must be a registered solid Material",
            ReactionError::Duplicate => "the same reaction is already declared",
            ReactionError::Conflict => "another reaction already matches some of the same contacts",
        })
    }
}

impl std::error::Error for ReactionError {}

/// Every declared reaction, canonical, in registration order. Immutable once
/// the World opens.
#[derive(Clone, Debug, Default)]
pub struct ReactionTable {
    rules: Vec<ReactionDef>,
}

impl ReactionTable {
    /// [`crate::Tables::register_reaction`] checks the Materials first.
    pub(crate) fn insert(&mut self, def: ReactionDef) -> Result<(), ReactionError> {
        if def.first.fluid == def.second.fluid {
            return Err(ReactionError::SameFluid);
        }
        let def = def.canonical();
        if let Some(held) = self.rules.iter().find(|r| r.overlaps(&def)) {
            return Err(if *held == def {
                ReactionError::Duplicate
            } else {
                ReactionError::Conflict
            });
        }
        self.rules.push(def);
        Ok(())
    }

    pub fn rules(&self) -> &[ReactionDef] {
        &self.rules
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// The rules between two fluids, canonical, whichever is named first.
    pub fn between(&self, a: MaterialId, b: MaterialId) -> impl Iterator<Item = &ReactionDef> {
        let (low, high) = if a <= b { (a, b) } else { (b, a) };
        self.rules
            .iter()
            .filter(move |r| r.first.fluid == low && r.second.fluid == high)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WATER: MaterialId = 7;
    const LAVA: MaterialId = 9;

    fn any(fluid: MaterialId) -> Reactant {
        Reactant {
            fluid,
            phase: Phase::Any,
        }
    }

    /// Lava, written second, is consumed into stone and water is kept.
    fn cooling() -> ReactionDef {
        ReactionDef {
            first: any(LAVA),
            second: any(WATER),
            consumed: Side::First,
            product: Some(1),
            receiving: None,
        }
    }

    #[test]
    fn a_rule_is_stored_lower_fluid_first_with_its_sides_swapped() {
        let mut table = ReactionTable::default();
        table.insert(cooling()).unwrap();
        let held = table.rules()[0];
        assert_eq!(held.first.fluid, WATER);
        assert_eq!(held.second.fluid, LAVA);
        assert_eq!(held.consumed, Side::Second, "still the lava");
        assert_eq!(table.between(LAVA, WATER).count(), 1);
        assert_eq!(table.between(WATER, LAVA).count(), 1);
        assert_eq!(table.between(WATER, 8).count(), 0);
    }

    #[test]
    fn mirrored_duplicates_and_overlapping_rules_are_refused() {
        let mut table = ReactionTable::default();
        table.insert(cooling()).unwrap();
        let mirrored = ReactionDef {
            first: any(WATER),
            second: any(LAVA),
            consumed: Side::Second,
            ..cooling()
        };
        assert_eq!(table.insert(mirrored), Err(ReactionError::Duplicate));
        let sources_only = ReactionDef {
            first: Reactant {
                fluid: LAVA,
                phase: Phase::Source,
            },
            product: Some(2),
            ..cooling()
        };
        assert_eq!(table.insert(sources_only), Err(ReactionError::Conflict));
        let directed = ReactionDef {
            receiving: Some(Side::Second),
            ..cooling()
        };
        assert_eq!(table.insert(directed), Err(ReactionError::Conflict));
        assert_eq!(table.rules().len(), 1);
    }

    #[test]
    fn disjoint_states_and_receiving_sides_are_separate_rules() {
        let state = |phase| Reactant { fluid: LAVA, phase };
        let mut table = ReactionTable::default();
        for (phase, product) in [(Phase::Source, 1), (Phase::Flowing, 2)] {
            table
                .insert(ReactionDef {
                    first: state(phase),
                    product: Some(product),
                    receiving: Some(Side::First),
                    ..cooling()
                })
                .unwrap();
        }
        table
            .insert(ReactionDef {
                receiving: Some(Side::Second),
                product: None,
                ..cooling()
            })
            .unwrap();
        assert_eq!(table.between(WATER, LAVA).count(), 3);
        assert_eq!(
            table.insert(ReactionDef {
                first: any(WATER),
                ..cooling()
            }),
            Err(ReactionError::SameFluid)
        );
    }
}
