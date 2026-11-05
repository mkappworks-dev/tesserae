//! A Player's Inventory: a hotbar and a backpack of Stacks, and the rules that
//! fill them.

use serde::{Deserialize, Serialize};

pub const HOTBAR_SLOTS: usize = 9;
pub const BACKPACK_SLOTS: usize = 27;

/// One Item and a count, never zero: an emptied slot is empty.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stack {
    /// The Item's declared name, so a slot survives a change in Mod load order.
    pub item: String,
    pub count: u32,
}

#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Inventory {
    hotbar: [Option<Stack>; HOTBAR_SLOTS],
    backpack: [Option<Stack>; BACKPACK_SLOTS],
}

impl Inventory {
    pub fn hotbar(&self) -> &[Option<Stack>; HOTBAR_SLOTS] {
        &self.hotbar
    }

    pub fn backpack(&self) -> &[Option<Stack>; BACKPACK_SLOTS] {
        &self.backpack
    }

    /// Every slot, the hotbar first.
    pub fn slots(&self) -> impl Iterator<Item = &Option<Stack>> {
        self.hotbar.iter().chain(&self.backpack)
    }

    pub fn is_empty(&self) -> bool {
        self.slots().all(Option::is_none)
    }

    fn slots_mut(&mut self) -> impl Iterator<Item = &mut Option<Stack>> {
        self.hotbar.iter_mut().chain(&mut self.backpack)
    }

    /// Adds `count` of `item` up to `max_stack` per slot: into matching stacks,
    /// then empty slots, the hotbar before the backpack each time. Returns how
    /// many did not fit.
    pub fn receive(&mut self, item: &str, count: u32, max_stack: u32) -> u32 {
        let mut left = count;
        for stack in self.slots_mut().flatten() {
            if left == 0 {
                return 0;
            }
            if stack.item == item {
                let added = max_stack.saturating_sub(stack.count).min(left);
                stack.count += added;
                left -= added;
            }
        }
        for slot in self.slots_mut().filter(|s| s.is_none()) {
            if left == 0 || max_stack == 0 {
                break;
            }
            let added = max_stack.min(left);
            *slot = Some(Stack {
                item: item.into(),
                count: added,
            });
            left -= added;
        }
        left
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stack(item: &str, count: u32) -> Option<Stack> {
        Some(Stack {
            item: item.into(),
            count,
        })
    }

    #[test]
    fn an_empty_inventory_takes_a_pickup_into_the_first_hotbar_slot() {
        let mut inventory = Inventory::default();
        assert_eq!(inventory.receive("t:rock", 5, 64), 0);
        assert_eq!(inventory.hotbar()[0], stack("t:rock", 5));
        assert!(inventory.hotbar()[1..].iter().all(Option::is_none));
        assert!(inventory.backpack().iter().all(Option::is_none));
    }

    #[test]
    fn matching_stacks_fill_before_empty_slots_hotbar_before_backpack() {
        let mut inventory = Inventory::default();
        inventory.hotbar[0] = stack("t:dust", 3);
        inventory.hotbar[4] = stack("t:rock", 60);
        inventory.backpack[2] = stack("t:rock", 10);
        assert_eq!(inventory.receive("t:rock", 4 + 54 + 20, 64), 0);
        assert_eq!(inventory.hotbar[4], stack("t:rock", 64));
        assert_eq!(inventory.backpack[2], stack("t:rock", 64));
        assert_eq!(inventory.hotbar[1], stack("t:rock", 20), "then empty");
        assert_eq!(inventory.hotbar[0], stack("t:dust", 3));
    }

    #[test]
    fn empty_slots_fill_at_the_limit_across_the_hotbar_then_the_backpack() {
        let mut inventory = Inventory::default();
        assert_eq!(inventory.receive("t:rock", 64 * 9 + 70, 64), 0);
        assert!(inventory.hotbar.iter().all(|s| *s == stack("t:rock", 64)));
        assert_eq!(inventory.backpack[0], stack("t:rock", 64));
        assert_eq!(inventory.backpack[1], stack("t:rock", 6));
    }

    #[test]
    fn a_full_inventory_returns_what_did_not_fit() {
        let mut inventory = Inventory::default();
        assert_eq!(inventory.receive("t:dust", 64 * 36 - 1, 64), 0);
        assert_eq!(inventory.receive("t:rock", 5, 64), 5);
        assert_eq!(inventory.receive("t:dust", 5, 64), 4);
        assert_eq!(inventory.backpack[26], stack("t:dust", 64));
        let full = inventory.clone();
        assert_eq!(inventory.receive("t:dust", 1, 64), 1);
        assert_eq!(inventory, full);
    }

    #[test]
    fn receiving_nothing_changes_nothing() {
        let mut inventory = Inventory::default();
        assert_eq!(inventory.receive("t:rock", 0, 64), 0);
        assert_eq!(inventory, Inventory::default());
    }
}
