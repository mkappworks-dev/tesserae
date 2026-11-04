//! Dropped items in a commit, additive to the World header: a commit without
//! them opens with none, and nothing here touches the format or generator
//! version.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::Error;
use crate::entity::{
    DESPAWN_STEPS, DroppedItem, Entities, EntityId, MAX_ENTITIES, PICKUP_DELAY_STEPS,
};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedEntities {
    pub next_id: u64,
    pub dropped: Vec<SavedDrop>,
}

/// An Item is saved by name, like a Material, and its size is read from the
/// declaration again on load.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedDrop {
    pub id: u64,
    pub item: String,
    pub count: u32,
    pub pos_m: [f32; 3],
    pub vel_m_s: [f32; 3],
    pub resting: bool,
    pub steps_left: u32,
    pub pickup_in: u32,
}

impl SavedEntities {
    /// `name_of` maps an item index to its full name; an unknown index fails.
    pub fn capture(
        entities: &Entities,
        name_of: impl Fn(u32) -> Option<String>,
    ) -> Result<Self, Error> {
        let dropped = entities
            .iter()
            .map(|e| {
                let d = e.as_dropped();
                Ok(SavedDrop {
                    id: e.id.0,
                    item: name_of(d.item)
                        .ok_or_else(|| Error::Corrupt(format!("unknown Item index {}", d.item)))?,
                    count: d.count,
                    pos_m: d.pos_m,
                    vel_m_s: d.vel_m_s,
                    resting: d.resting,
                    steps_left: d.steps_left,
                    pickup_in: d.pickup_in,
                })
            })
            .collect::<Result<_, Error>>()?;
        Ok(Self {
            next_id: entities.next_id(),
            dropped,
        })
    }

    pub fn validate(&self) -> Result<(), Error> {
        let bad = |why: &str| Err(Error::Corrupt(format!("dropped items: {why}")));
        if self.dropped.len() > MAX_ENTITIES {
            return bad("more than the live-set cap");
        }
        let mut ids = BTreeSet::new();
        for d in &self.dropped {
            if !ids.insert(d.id) || d.id >= self.next_id {
                return bad("duplicate id, or an id the next one does not follow");
            }
            if d.item.is_empty() || d.count == 0 {
                return bad("an empty name or count");
            }
            if d.pos_m.iter().chain(&d.vel_m_s).any(|v| !v.is_finite()) {
                return bad("a non-finite position or velocity");
            }
            if d.steps_left == 0 || d.steps_left > DESPAWN_STEPS || d.pickup_in > PICKUP_DELAY_STEPS
            {
                return bad("a timer out of range");
            }
        }
        Ok(())
    }

    /// `resolve` maps a full Item name to its index and its size in metres.
    pub fn restore(
        &self,
        voxel_size: f32,
        resolve: impl Fn(&str) -> Option<(u32, f32)>,
    ) -> Result<Entities, Error> {
        self.validate()?;
        let drops = self
            .dropped
            .iter()
            .map(|d| {
                let (item, size_m) = resolve(&d.item)
                    .ok_or_else(|| Error::Corrupt(format!("unknown Item `{}`", d.item)))?;
                Ok((
                    EntityId(d.id),
                    DroppedItem {
                        item,
                        count: d.count,
                        size_m,
                        pos_m: d.pos_m,
                        vel_m_s: d.vel_m_s,
                        resting: d.resting,
                        steps_left: d.steps_left,
                        pickup_in: d.pickup_in,
                    },
                ))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(Entities::restore(self.next_id, drops, voxel_size))
    }
}
