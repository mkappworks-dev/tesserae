//! What two fluids do on contact, as table data.
//!
//! A rule names a pair of fluid Materials, which of them is consumed, and an
//! optional solid left where it was. Rules are stored in one canonical
//! orientation, so a pair declared either way round is the same rule, and at
//! most one rule can match any contact.
//!
//! **Receiving.** At a contact, fluid `b` arrives at fluid `a` when it is
//! where flow from it into `a`'s void would come from: above it, or beside it
//! on a floor no lower. Never from below. Only gravity enters, not the
//! horizontal direction or the order a rule names its fluids in, and on a
//! level side each arrives at the other; so two rules naming opposite
//! receiving sides can match one contact, and conflict like any other pair.

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
    /// When set, the rule matches only a contact where the other participant
    /// arrives at this one (see the module docs); unset, any contact.
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

    /// Whether some contact could match both rules, both canonical. Receiving
    /// sides never separate two rules: a level contact receives both ways.
    fn overlaps(&self, other: &ReactionDef) -> bool {
        self.first.fluid == other.first.fluid
            && self.second.fluid == other.second.fluid
            && self.first.phase.overlaps(other.first.phase)
            && self.second.phase.overlaps(other.second.phase)
    }
}

/// One side of a contact as a rule sees it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Participant {
    pub fluid: MaterialId,
    pub source: bool,
    /// Whether the other participant arrives at this one there.
    pub receives: bool,
}

/// What the one rule a contact matches does to it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Outcome {
    /// Whether the participant named first in [`ReactionTable::matching`] is
    /// the one consumed; otherwise the second is.
    pub consumes_first: bool,
    pub product: Option<MaterialId>,
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

    /// Whether any rule names `fluid`.
    pub fn involves(&self, fluid: MaterialId) -> bool {
        self.rules
            .iter()
            .any(|r| r.first.fluid == fluid || r.second.fluid == fluid)
    }

    /// The rule a contact between `a` and `b` matches, whichever is named
    /// first; at most one can.
    pub fn matching(&self, a: Participant, b: Participant) -> Option<Outcome> {
        self.between(a.fluid, b.fluid).find_map(|r| {
            let a_first = r.first.fluid == a.fluid;
            let (first, second) = if a_first { (a, b) } else { (b, a) };
            let fits = r.first.phase.matches(first.source)
                && r.second.phase.matches(second.source)
                && match r.receiving {
                    None => true,
                    Some(Side::First) => first.receives,
                    Some(Side::Second) => second.receives,
                };
            fits.then_some(Outcome {
                consumes_first: (r.consumed == Side::First) == a_first,
                product: r.product,
            })
        })
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

    /// Disjoint states are separate rules; opposite receiving sides are not,
    /// because a level contact receives both ways and would match both.
    #[test]
    fn disjoint_states_are_separate_rules_and_receiving_sides_are_not() {
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
        assert_eq!(table.between(WATER, LAVA).count(), 2);
        assert_eq!(
            table.insert(ReactionDef {
                first: state(Phase::Source),
                receiving: Some(Side::Second),
                product: None,
                ..cooling()
            }),
            Err(ReactionError::Conflict)
        );
        assert_eq!(
            table.insert(ReactionDef {
                first: any(WATER),
                ..cooling()
            }),
            Err(ReactionError::SameFluid)
        );
    }

    fn at(fluid: MaterialId, source: bool, receives: bool) -> Participant {
        Participant {
            fluid,
            source,
            receives,
        }
    }

    /// The match is the same whichever participant is asked about first, and
    /// says which of them goes.
    #[test]
    fn a_contact_matches_the_same_rule_either_way_round() {
        let mut table = ReactionTable::default();
        table.insert(cooling()).unwrap();
        let (lava, water) = (at(LAVA, false, true), at(WATER, true, true));
        let one = table.matching(lava, water).unwrap();
        let other = table.matching(water, lava).unwrap();
        assert!(one.consumes_first && !other.consumes_first);
        assert_eq!((one.product, other.product), (Some(1), Some(1)));
        assert_eq!(table.matching(lava, at(8, false, true)), None);
        assert!(table.involves(LAVA) && table.involves(WATER) && !table.involves(8));
    }

    /// States and a receiving side are data the match reads.
    #[test]
    fn states_and_the_receiving_side_narrow_a_match() {
        let mut table = ReactionTable::default();
        table
            .insert(ReactionDef {
                first: Reactant {
                    fluid: LAVA,
                    phase: Phase::Source,
                },
                receiving: Some(Side::First),
                ..cooling()
            })
            .unwrap();
        let water = at(WATER, false, false);
        assert!(table.matching(at(LAVA, true, true), water).is_some());
        assert_eq!(
            table.matching(at(LAVA, false, true), water),
            None,
            "flowing"
        );
        assert_eq!(
            table.matching(at(LAVA, true, false), water),
            None,
            "water did not arrive at it"
        );
    }
}
