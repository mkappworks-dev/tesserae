//! Loot: the Items a chest worldgen stands holds the first time a Container is needed for it,
//! drawn from a key alone so it is the same whenever and from wherever it is asked. ADR-0029.

use serde::{Deserialize, Serialize};

use crate::inventory::Stack;

pub const MAX_LOOT_ENTRIES: usize = 16;
pub const MAX_LOOT_ROLLS: u8 = 27;

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LootEntry {
    /// An Item's full name.
    pub item: String,
    pub min: u32,
    pub max: u32,
    pub weight: f32,
}

/// Each roll draws one entry by weight and a count in its range, a Stack in a slot of its own.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LootTable {
    pub rolls_min: u8,
    pub rolls_max: u8,
    pub entries: Vec<LootEntry>,
}

impl LootTable {
    pub fn validate(&self) -> Result<(), String> {
        if self.rolls_min > self.rolls_max || self.rolls_max > MAX_LOOT_ROLLS {
            return Err(format!(
                "rolls-min must be at most rolls-max, which is at most {MAX_LOOT_ROLLS}"
            ));
        }
        if self.entries.is_empty() || self.entries.len() > MAX_LOOT_ENTRIES {
            return Err(format!("holds 1 to {MAX_LOOT_ENTRIES} entries"));
        }
        for e in &self.entries {
            if !(e.weight.is_finite() && e.weight > 0.0) {
                return Err(format!("`{}`: its weight must be above 0", e.item));
            }
            if e.min == 0 || e.min > e.max {
                return Err(format!(
                    "`{}`: min must be at least 1 and at most max",
                    e.item
                ));
            }
        }
        Ok(())
    }

    /// A chest of `slots` filled by the draw `key` names. A roll past the last free slot is
    /// dropped.
    pub fn draw(&self, key: u64, slots: usize) -> Vec<Option<Stack>> {
        let mut held = vec![None; slots];
        let span = u64::from(self.rolls_max - self.rolls_min) + 1;
        let rolls = u64::from(self.rolls_min) + mix(key) % span;
        let total: f64 = self.entries.iter().map(|e| f64::from(e.weight)).sum();
        for roll in 0..rolls.min(slots as u64) {
            let h = mix(key ^ mix(roll + 1));
            let mut pick = unit(h) * total;
            let entry = self
                .entries
                .iter()
                .find(|e| {
                    pick -= f64::from(e.weight);
                    pick < 0.0
                })
                .unwrap_or(&self.entries[self.entries.len() - 1]);
            let count = entry.min + (mix(h ^ 1) % u64::from(entry.max - entry.min + 1)) as u32;
            let start = (mix(h ^ 2) % slots as u64) as usize;
            if let Some(free) = (0..slots)
                .map(|i| (start + i) % slots)
                .find(|&i| held[i].is_none())
            {
                held[free] = Some(Stack {
                    item: entry.item.clone(),
                    count,
                });
            }
        }
        held
    }
}

fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

fn unit(h: u64) -> f64 {
    (h >> 11) as f64 / (1u64 << 53) as f64
}
