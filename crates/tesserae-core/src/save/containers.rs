//! Container contents in a commit, additive to the World header like dropped
//! items: a commit without them opens with none.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::Error;
use crate::inventory::Stack;
use crate::usable::MAX_CONTAINER_SLOTS;

/// One Container Voxel's slots, Stacks by Item name. Whether the Voxel is
/// still a Container is the Server's to judge when the World opens.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedContainer {
    pub at: [i32; 3],
    pub slots: Vec<Option<Stack>>,
}

pub fn validate(containers: &[SavedContainer]) -> Result<(), Error> {
    let bad = |why: &str| Err(Error::Corrupt(format!("containers: {why}")));
    let mut seen = BTreeSet::new();
    for c in containers {
        if !seen.insert(c.at) {
            return bad("two at one position");
        }
        if c.slots.is_empty() || c.slots.len() > usize::from(MAX_CONTAINER_SLOTS) {
            return bad("a slot count out of range");
        }
        if c.slots
            .iter()
            .flatten()
            .any(|s| s.item.is_empty() || s.count == 0)
        {
            return bad("an empty name or count");
        }
    }
    Ok(())
}
