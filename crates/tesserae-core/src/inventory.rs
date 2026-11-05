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

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Slot {
    Hotbar(usize),
    Backpack(usize),
}

/// Why a change was refused. A refused change leaves the Inventory as it was.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Refusal {
    NoSuchSlot,
    Empty,
    /// A merge into a stack already at its limit.
    StackFull,
    TooFewToSplit,
    /// A split needs an empty backpack slot.
    NoEmptySlot,
    /// The other area cannot take the whole stack.
    NoRoom,
}

/// Adds `count` of `item` into `areas`, matching stacks first and then empty
/// slots, each pass in the order given. Returns how many did not fit.
fn stash(areas: &mut [&mut [Option<Stack>]], item: &str, count: u32, max_stack: u32) -> u32 {
    let mut left = count;
    for stack in areas.iter_mut().flat_map(|a| a.iter_mut()).flatten() {
        if left == 0 {
            return 0;
        }
        if stack.item == item {
            let added = max_stack.saturating_sub(stack.count).min(left);
            stack.count += added;
            left -= added;
        }
    }
    for slot in areas
        .iter_mut()
        .flat_map(|a| a.iter_mut())
        .filter(|s| s.is_none())
    {
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

impl Inventory {
    pub fn get(&self, slot: Slot) -> Option<&Stack> {
        match slot {
            Slot::Hotbar(at) => self.hotbar.get(at)?.as_ref(),
            Slot::Backpack(at) => self.backpack.get(at)?.as_ref(),
        }
    }

    fn slot_mut(&mut self, slot: Slot) -> Result<&mut Option<Stack>, Refusal> {
        match slot {
            Slot::Hotbar(at) => self.hotbar.get_mut(at),
            Slot::Backpack(at) => self.backpack.get_mut(at),
        }
        .ok_or(Refusal::NoSuchSlot)
    }

    /// Moves the stack in `from` onto `to`: into an empty slot whole, into the
    /// same Item up to its limit with the rest left behind, and otherwise
    /// swapping the two.
    pub fn move_stack(
        &mut self,
        from: Slot,
        to: Slot,
        limit: impl Fn(&str) -> u32,
    ) -> Result<(), Refusal> {
        let target = self.slot_mut(to)?.clone();
        let moving = self.slot_mut(from)?.clone().ok_or(Refusal::Empty)?;
        if from == to {
            return Ok(());
        }
        let (source, target) = match target {
            Some(onto) if onto.item == moving.item => {
                let added = limit(&onto.item)
                    .saturating_sub(onto.count)
                    .min(moving.count);
                if added == 0 {
                    return Err(Refusal::StackFull);
                }
                let rest = moving.count - added;
                (
                    (rest > 0).then_some(Stack {
                        count: rest,
                        ..moving
                    }),
                    Stack {
                        count: onto.count + added,
                        ..onto
                    },
                )
            }
            other => (other, moving),
        };
        *self.slot_mut(from)? = source;
        *self.slot_mut(to)? = Some(target);
        Ok(())
    }

    /// Moves the smaller half of a stack into the first empty backpack slot.
    pub fn split(&mut self, from: Slot) -> Result<(), Refusal> {
        let count = self.slot_mut(from)?.as_ref().ok_or(Refusal::Empty)?.count;
        if count < 2 {
            return Err(Refusal::TooFewToSplit);
        }
        let empty = self
            .backpack
            .iter()
            .position(Option::is_none)
            .ok_or(Refusal::NoEmptySlot)?;
        let source = self.slot_mut(from)?.as_mut().ok_or(Refusal::Empty)?;
        let half = Stack {
            item: source.item.clone(),
            count: count / 2,
        };
        source.count -= half.count;
        self.backpack[empty] = Some(half);
        Ok(())
    }

    /// Moves a whole stack between the backpack and the hotbar, into matching
    /// stacks first and then empty slots, or not at all.
    pub fn quick_transfer(
        &mut self,
        from: Slot,
        limit: impl Fn(&str) -> u32,
    ) -> Result<(), Refusal> {
        let moving = self.slot_mut(from)?.clone().ok_or(Refusal::Empty)?;
        let max = limit(&moving.item);
        let left = match from {
            Slot::Hotbar(_) => {
                let mut backpack = self.backpack.clone();
                let left = stash(&mut [&mut backpack], &moving.item, moving.count, max);
                if left == 0 {
                    self.backpack = backpack;
                }
                left
            }
            Slot::Backpack(_) => {
                let mut hotbar = self.hotbar.clone();
                let left = stash(&mut [&mut hotbar], &moving.item, moving.count, max);
                if left == 0 {
                    self.hotbar = hotbar;
                }
                left
            }
        };
        if left > 0 {
            return Err(Refusal::NoRoom);
        }
        *self.slot_mut(from)? = None;
        Ok(())
    }

    /// Merges the backpack's stacks of each Item and orders them by Item name
    /// from the first slot.
    pub fn sort(&mut self, limit: impl Fn(&str) -> u32) -> Result<(), Refusal> {
        let mut stacks: Vec<_> = self.backpack.iter().flatten().collect();
        stacks.sort_by(|a, b| a.item.cmp(&b.item));
        let mut sorted: [Option<Stack>; BACKPACK_SLOTS] = Default::default();
        for stack in stacks {
            if stash(
                &mut [&mut sorted],
                &stack.item,
                stack.count,
                limit(&stack.item),
            ) > 0
            {
                return Err(Refusal::NoRoom);
            }
        }
        self.backpack = sorted;
        Ok(())
    }

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

    /// Adds `count` of `item` up to `max_stack` per slot: into matching stacks,
    /// then empty slots, the hotbar before the backpack each time. Returns how
    /// many did not fit.
    pub fn receive(&mut self, item: &str, count: u32, max_stack: u32) -> u32 {
        stash(
            &mut [&mut self.hotbar, &mut self.backpack],
            item,
            count,
            max_stack,
        )
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

    fn limit(_: &str) -> u32 {
        64
    }

    #[test]
    fn a_stack_moves_into_an_empty_slot_and_swaps_with_another_item() {
        let mut inventory = Inventory::default();
        inventory.backpack[0] = stack("t:rock", 5);
        inventory.hotbar[2] = stack("t:dust", 3);
        assert_eq!(
            inventory.move_stack(Slot::Backpack(0), Slot::Backpack(4), limit),
            Ok(())
        );
        assert_eq!(inventory.backpack[0], None);
        assert_eq!(inventory.backpack[4], stack("t:rock", 5));
        assert_eq!(
            inventory.move_stack(Slot::Backpack(4), Slot::Hotbar(2), limit),
            Ok(())
        );
        assert_eq!(inventory.hotbar[2], stack("t:rock", 5));
        assert_eq!(inventory.backpack[4], stack("t:dust", 3));
    }

    #[test]
    fn a_stack_merges_into_the_same_item_up_to_the_limit_and_keeps_the_rest() {
        let mut inventory = Inventory::default();
        inventory.backpack[0] = stack("t:rock", 10);
        inventory.backpack[1] = stack("t:rock", 60);
        assert_eq!(
            inventory.move_stack(Slot::Backpack(0), Slot::Backpack(1), limit),
            Ok(())
        );
        assert_eq!(inventory.backpack[1], stack("t:rock", 64));
        assert_eq!(inventory.backpack[0], stack("t:rock", 6));
        inventory.hotbar[0] = stack("t:rock", 6);
        inventory.backpack[0] = None;
        assert_eq!(
            inventory.move_stack(Slot::Hotbar(0), Slot::Backpack(2), limit),
            Ok(())
        );
        assert_eq!(
            inventory.move_stack(Slot::Backpack(2), Slot::Backpack(3), |_| 4),
            Ok(()),
            "an empty slot takes the whole stack whatever the limit"
        );
        assert_eq!(inventory.backpack[3], stack("t:rock", 6));
    }

    #[test]
    fn a_merge_into_a_full_stack_and_a_move_of_nothing_are_refused_and_change_nothing() {
        let mut inventory = Inventory::default();
        inventory.backpack[0] = stack("t:rock", 10);
        inventory.backpack[1] = stack("t:rock", 64);
        let before = inventory.clone();
        assert_eq!(
            inventory.move_stack(Slot::Backpack(0), Slot::Backpack(1), limit),
            Err(Refusal::StackFull)
        );
        assert_eq!(
            inventory.move_stack(Slot::Backpack(5), Slot::Backpack(1), limit),
            Err(Refusal::Empty)
        );
        assert_eq!(
            inventory.move_stack(Slot::Backpack(0), Slot::Backpack(27), limit),
            Err(Refusal::NoSuchSlot)
        );
        assert_eq!(
            inventory.move_stack(Slot::Hotbar(9), Slot::Backpack(1), limit),
            Err(Refusal::NoSuchSlot)
        );
        assert_eq!(
            inventory.move_stack(Slot::Backpack(0), Slot::Backpack(0), limit),
            Ok(())
        );
        assert_eq!(inventory, before);
    }

    #[test]
    fn split_moves_the_smaller_half_into_the_first_empty_backpack_slot() {
        let mut inventory = Inventory::default();
        inventory.backpack[0] = stack("t:dust", 1);
        inventory.hotbar[3] = stack("t:rock", 7);
        assert_eq!(inventory.split(Slot::Hotbar(3)), Ok(()));
        assert_eq!(inventory.hotbar[3], stack("t:rock", 4));
        assert_eq!(inventory.backpack[1], stack("t:rock", 3));
    }

    #[test]
    fn split_is_refused_for_one_item_nothing_or_no_empty_backpack_slot() {
        let mut inventory = Inventory::default();
        inventory.backpack[0] = stack("t:dust", 1);
        assert_eq!(
            inventory.split(Slot::Backpack(0)),
            Err(Refusal::TooFewToSplit)
        );
        assert_eq!(inventory.split(Slot::Backpack(1)), Err(Refusal::Empty));
        for slot in &mut inventory.backpack {
            *slot = stack("t:dust", 2);
        }
        inventory.hotbar[0] = stack("t:rock", 8);
        let before = inventory.clone();
        assert_eq!(
            inventory.split(Slot::Hotbar(0)),
            Err(Refusal::NoEmptySlot),
            "an empty hotbar slot does not count"
        );
        assert_eq!(inventory, before);
    }

    #[test]
    fn quick_transfer_stashes_a_stack_in_the_other_area_matching_stacks_first() {
        let mut inventory = Inventory::default();
        inventory.backpack[5] = stack("t:rock", 70);
        inventory.hotbar[4] = stack("t:rock", 60);
        assert_eq!(inventory.quick_transfer(Slot::Backpack(5), |_| 100), Ok(()));
        assert_eq!(inventory.hotbar[4], stack("t:rock", 100));
        assert_eq!(inventory.hotbar[0], stack("t:rock", 30));
        assert_eq!(inventory.backpack[5], None);
        assert_eq!(inventory.quick_transfer(Slot::Hotbar(0), limit), Ok(()));
        assert_eq!(inventory.backpack[0], stack("t:rock", 30));
        assert_eq!(inventory.hotbar[0], None);
    }

    #[test]
    fn quick_transfer_without_room_for_the_whole_stack_is_refused_and_changes_nothing() {
        let mut inventory = Inventory::default();
        for slot in &mut inventory.hotbar {
            *slot = stack("t:dust", 64);
        }
        inventory.hotbar[8] = stack("t:rock", 63);
        inventory.backpack[0] = stack("t:rock", 2);
        let before = inventory.clone();
        assert_eq!(
            inventory.quick_transfer(Slot::Backpack(0), limit),
            Err(Refusal::NoRoom)
        );
        assert_eq!(inventory, before);
        assert_eq!(
            inventory.quick_transfer(Slot::Backpack(1), limit),
            Err(Refusal::Empty)
        );
    }

    #[test]
    fn sort_merges_and_orders_the_backpack_and_leaves_the_hotbar() {
        let mut inventory = Inventory::default();
        inventory.hotbar[0] = stack("t:zinc", 1);
        inventory.backpack[3] = stack("t:rock", 40);
        inventory.backpack[7] = stack("t:dust", 5);
        inventory.backpack[9] = stack("t:rock", 40);
        inventory.backpack[20] = stack("t:dust", 6);
        assert_eq!(inventory.sort(limit), Ok(()));
        assert_eq!(
            inventory.backpack[..4],
            [
                stack("t:dust", 11),
                stack("t:rock", 64),
                stack("t:rock", 16),
                None
            ]
        );
        assert!(inventory.backpack[3..].iter().all(Option::is_none));
        assert_eq!(inventory.hotbar[0], stack("t:zinc", 1));
    }

    #[test]
    fn receiving_nothing_changes_nothing() {
        let mut inventory = Inventory::default();
        assert_eq!(inventory.receive("t:rock", 0, 64), 0);
        assert_eq!(inventory, Inventory::default());
    }
}
