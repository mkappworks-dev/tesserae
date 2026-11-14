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
    /// The hotbar slot in hand.
    #[serde(default, skip_serializing_if = "is_first")]
    selected: usize,
}

fn is_first(at: &usize) -> bool {
    *at == 0
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Slot {
    Hotbar(usize),
    Backpack(usize),
}

impl Slot {
    fn at(self) -> At {
        match self {
            Slot::Hotbar(index) => At {
                side: Side::First,
                index,
            },
            Slot::Backpack(index) => At {
                side: Side::Second,
                index,
            },
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Side {
    First,
    Second,
}

impl Side {
    fn other(self) -> Self {
        match self {
            Side::First => Side::Second,
            Side::Second => Side::First,
        }
    }
}

/// A slot of a [`SlotSets`], counted across its side's runs in order.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct At {
    pub side: Side,
    pub index: usize,
}

/// Why a change was refused. A refused change leaves the Inventory as it was.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Refusal {
    NoSuchSlot,
    Empty,
    /// A merge into a stack already at its limit.
    StackFull,
    TooFewToSplit,
    /// A split needs an empty slot in the set it splits into.
    NoEmptySlot,
    /// The other slot set cannot take the whole stack.
    NoRoom,
    /// A take, or the stack it displaces, does not fit in the backpack whole.
    BackpackFull,
    /// A take of none, or of more than one stack holds.
    BadCount,
    /// A spend of more Items than the Inventory holds.
    TooFew,
}

/// Where a take lands.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Destination {
    /// Into matching stacks, then empty slots.
    Backpack,
    Slot(Slot),
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

/// How many of `item` `runs` can take, topping up matching stacks and filling
/// empty slots.
fn room(runs: &[&mut [Option<Stack>]], item: &str, max_stack: u32) -> u32 {
    runs.iter()
        .flat_map(|run| run.iter())
        .map(|slot| match slot {
            None => max_stack,
            Some(stack) if stack.item == item => max_stack.saturating_sub(stack.count),
            Some(_) => 0,
        })
        .fold(0, u32::saturating_add)
}

/// Two slot sets the slot rules move Stacks between, each one or more runs of
/// slots filled in order: an Inventory's hotbar and its backpack, or a whole
/// Inventory and a Container.
pub struct SlotSets<'a> {
    first: Vec<&'a mut [Option<Stack>]>,
    second: Vec<&'a mut [Option<Stack>]>,
}

impl<'a> SlotSets<'a> {
    fn set_mut(&mut self, side: Side) -> &mut [&'a mut [Option<Stack>]] {
        match side {
            Side::First => &mut self.first,
            Side::Second => &mut self.second,
        }
    }

    fn slot_mut(&mut self, at: At) -> Result<&mut Option<Stack>, Refusal> {
        self.set_mut(at.side)
            .iter_mut()
            .flat_map(|run| run.iter_mut())
            .nth(at.index)
            .ok_or(Refusal::NoSuchSlot)
    }

    /// Moves the stack in `from` onto `to`: into an empty slot whole, into the
    /// same Item up to its limit with the rest left behind, and otherwise
    /// swapping the two.
    pub fn move_stack(
        &mut self,
        from: At,
        to: At,
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

    /// Moves the smaller half of a stack into the first empty slot of `into`.
    pub fn split(&mut self, from: At, into: Side) -> Result<(), Refusal> {
        let count = self.slot_mut(from)?.as_ref().ok_or(Refusal::Empty)?.count;
        if count < 2 {
            return Err(Refusal::TooFewToSplit);
        }
        let empty = self
            .set_mut(into)
            .iter()
            .flat_map(|run| run.iter())
            .position(Option::is_none)
            .ok_or(Refusal::NoEmptySlot)?;
        let source = self.slot_mut(from)?.as_mut().ok_or(Refusal::Empty)?;
        let half = Stack {
            item: source.item.clone(),
            count: count / 2,
        };
        source.count -= half.count;
        *self.slot_mut(At {
            side: into,
            index: empty,
        })? = Some(half);
        Ok(())
    }

    /// Moves a whole stack into the other set, into matching stacks first and
    /// then empty slots, or not at all.
    pub fn quick_transfer(&mut self, from: At, limit: impl Fn(&str) -> u32) -> Result<(), Refusal> {
        let moving = self.slot_mut(from)?.clone().ok_or(Refusal::Empty)?;
        let max = limit(&moving.item);
        let into = self.set_mut(from.side.other());
        if room(into, &moving.item, max) < moving.count {
            return Err(Refusal::NoRoom);
        }
        stash(into, &moving.item, moving.count, max);
        *self.slot_mut(from)? = None;
        Ok(())
    }

    /// Merges the stacks of each Item in `side` and orders them by Item name
    /// from its first slot.
    pub fn sort(&mut self, side: Side, limit: impl Fn(&str) -> u32) -> Result<(), Refusal> {
        let set = self.set_mut(side);
        let mut stacks: Vec<Stack> = set
            .iter()
            .flat_map(|run| run.iter())
            .flatten()
            .cloned()
            .collect();
        stacks.sort_by(|a, b| a.item.cmp(&b.item));
        let mut sorted = vec![None; set.iter().map(|run| run.len()).sum()];
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
        for (slot, stack) in set.iter_mut().flat_map(|run| run.iter_mut()).zip(sorted) {
            *slot = stack;
        }
        Ok(())
    }

    /// Moves as much of each stack in `from` as the other set takes, into
    /// matching stacks first and then empty slots; what does not fit stays.
    /// Refused only when nothing moves.
    pub fn take_all(&mut self, from: Side, limit: impl Fn(&str) -> u32) -> Result<(), Refusal> {
        let len = self.set_mut(from).iter().map(|run| run.len()).sum();
        let (mut held, mut moved) = (false, false);
        for index in 0..len {
            let at = At { side: from, index };
            let Some(stack) = self.slot_mut(at)?.clone() else {
                continue;
            };
            held = true;
            let left = stash(
                self.set_mut(from.other()),
                &stack.item,
                stack.count,
                limit(&stack.item),
            );
            moved |= left < stack.count;
            *self.slot_mut(at)? = (left > 0).then_some(Stack {
                count: left,
                ..stack
            });
        }
        match (held, moved) {
            (false, _) => Err(Refusal::Empty),
            (true, false) => Err(Refusal::NoRoom),
            (true, true) => Ok(()),
        }
    }
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

    fn sets(&mut self) -> SlotSets<'_> {
        SlotSets {
            first: vec![&mut self.hotbar[..]],
            second: vec![&mut self.backpack[..]],
        }
    }

    /// This Inventory, hotbar then backpack, as the first set and `other` as
    /// the second.
    pub fn beside<'a>(&'a mut self, other: &'a mut [Option<Stack>]) -> SlotSets<'a> {
        SlotSets {
            first: vec![&mut self.hotbar[..], &mut self.backpack[..]],
            second: vec![other],
        }
    }

    pub fn move_stack(
        &mut self,
        from: Slot,
        to: Slot,
        limit: impl Fn(&str) -> u32,
    ) -> Result<(), Refusal> {
        self.sets().move_stack(from.at(), to.at(), limit)
    }

    /// Moves the smaller half of a stack into the first empty backpack slot.
    pub fn split(&mut self, from: Slot) -> Result<(), Refusal> {
        self.sets().split(from.at(), Side::Second)
    }

    /// Moves a whole stack between the backpack and the hotbar.
    pub fn quick_transfer(
        &mut self,
        from: Slot,
        limit: impl Fn(&str) -> u32,
    ) -> Result<(), Refusal> {
        self.sets().quick_transfer(from.at(), limit)
    }

    /// Sorts the backpack and leaves the hotbar.
    pub fn sort(&mut self, limit: impl Fn(&str) -> u32) -> Result<(), Refusal> {
        self.sets().sort(Side::Second, limit)
    }

    /// Adds a new stack of 1 to the limit of `item`, whole or not at all. Onto
    /// a slot it tops up the same Item to the limit, or sends the stack there
    /// to the backpack.
    pub fn take(
        &mut self,
        item: &str,
        count: u32,
        into: Destination,
        limit: impl Fn(&str) -> u32,
    ) -> Result<(), Refusal> {
        let max_stack = limit(item);
        if count == 0 || count > max_stack {
            return Err(Refusal::BadCount);
        }
        let mut next = self.clone();
        let displaced = match into {
            Destination::Backpack => Some(Stack {
                item: item.into(),
                count,
            }),
            Destination::Slot(slot) => {
                let target = next.slot_mut(slot)?;
                match target.take() {
                    Some(onto) if onto.item == item => {
                        if onto.count >= max_stack {
                            return Err(Refusal::StackFull);
                        }
                        *target = Some(Stack {
                            count: (onto.count + count).min(max_stack),
                            ..onto
                        });
                        None
                    }
                    other => {
                        *target = Some(Stack {
                            item: item.into(),
                            count,
                        });
                        other
                    }
                }
            }
        };
        if let Some(stack) = displaced {
            let max = limit(&stack.item);
            if stash(&mut [&mut next.backpack], &stack.item, stack.count, max) > 0 {
                return Err(Refusal::BackpackFull);
            }
        }
        *self = next;
        Ok(())
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn select(&mut self, at: usize) -> Result<(), Refusal> {
        if at >= HOTBAR_SLOTS {
            return Err(Refusal::NoSuchSlot);
        }
        self.selected = at;
        Ok(())
    }

    /// The stack in the selected hotbar slot.
    pub fn held(&self) -> Option<&Stack> {
        self.get(Slot::Hotbar(self.selected))
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

    /// How many of `item` every slot holds together.
    pub fn count(&self, item: &str) -> u32 {
        self.slots()
            .flatten()
            .filter(|s| s.item == item)
            .map(|s| s.count)
            .sum()
    }

    /// Takes `count` of the Stack in `from` out of the Inventory, for a drop:
    /// an empty slot, none, or more than the Stack holds is refused and changes
    /// nothing.
    pub fn take_from(&mut self, from: Slot, count: u32) -> Result<Stack, Refusal> {
        let slot = self.slot_mut(from)?;
        let held = slot.as_ref().ok_or(Refusal::Empty)?.count;
        if count == 0 || count > held {
            return Err(Refusal::BadCount);
        }
        if count == held {
            return Ok(slot.take().expect("counted above"));
        }
        let stack = slot.as_mut().expect("counted above");
        stack.count -= count;
        Ok(Stack {
            item: stack.item.clone(),
            count,
        })
    }

    /// Removes `count` of `item`, all of them or none: the held stack first,
    /// then the others hotbar before backpack.
    pub fn spend(&mut self, item: &str, count: u32) -> Result<(), Refusal> {
        if self.count(item) < count {
            return Err(Refusal::TooFew);
        }
        let held = self.selected;
        let order = std::iter::once(Slot::Hotbar(held))
            .chain((0..HOTBAR_SLOTS).filter(|&at| at != held).map(Slot::Hotbar))
            .chain((0..BACKPACK_SLOTS).map(Slot::Backpack));
        let mut left = count;
        for slot in order {
            if left == 0 {
                break;
            }
            let target = self.slot_mut(slot)?;
            let Some(stack) = target.as_mut().filter(|s| s.item == item) else {
                continue;
            };
            let spent = stack.count.min(left);
            stack.count -= spent;
            left -= spent;
            if stack.count == 0 {
                *target = None;
            }
        }
        Ok(())
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

    /// Empties every slot, keeping the selection, and returns the stacks,
    /// the hotbar first.
    pub fn take_all(&mut self) -> Vec<Stack> {
        self.hotbar
            .iter_mut()
            .chain(&mut self.backpack)
            .filter_map(Option::take)
            .collect()
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
    fn taking_from_a_slot_removes_that_many_and_refuses_the_impossible() {
        let mut inventory = Inventory::default();
        inventory.receive("m:rock", 40, 64);
        assert_eq!(
            inventory.take_from(Slot::Hotbar(0), 1),
            Ok(Stack {
                item: "m:rock".into(),
                count: 1
            })
        );
        assert_eq!(inventory.count("m:rock"), 39);
        assert_eq!(
            inventory.take_from(Slot::Hotbar(0), 40),
            Err(Refusal::BadCount)
        );
        assert_eq!(
            inventory.take_from(Slot::Hotbar(0), 0),
            Err(Refusal::BadCount)
        );
        assert_eq!(inventory.take_from(Slot::Hotbar(3), 1), Err(Refusal::Empty));
        assert_eq!(
            inventory.take_from(Slot::Hotbar(99), 1),
            Err(Refusal::NoSuchSlot)
        );
        assert_eq!(inventory.count("m:rock"), 39, "refusals change nothing");
        assert_eq!(
            inventory.take_from(Slot::Hotbar(0), 39),
            Ok(Stack {
                item: "m:rock".into(),
                count: 39
            })
        );
        assert_eq!(
            inventory.held(),
            None,
            "the emptied slot is empty, not a stack of none"
        );
    }

    #[test]
    fn the_held_stack_is_the_selected_hotbar_slot_s() {
        let mut inventory = Inventory::default();
        inventory.receive("m:rock", 5, 64);
        inventory.receive("m:sand", 64, 64);
        assert_eq!(inventory.held().cloned(), stack("m:rock", 5));
        inventory.select(1).unwrap();
        assert_eq!(inventory.selected(), 1);
        assert_eq!(inventory.held().cloned(), stack("m:sand", 64));
        inventory.select(8).unwrap();
        assert_eq!(inventory.held(), None);
        assert_eq!(inventory.select(9), Err(Refusal::NoSuchSlot));
        assert_eq!(inventory.selected(), 8);
    }

    #[test]
    fn taking_everything_empties_every_slot_hotbar_first_and_keeps_the_selection() {
        let mut inventory = Inventory::default();
        inventory.receive("m:rock", 64 * 9 + 3, 64);
        inventory.select(4).unwrap();
        let taken = inventory.take_all();
        assert_eq!(taken.len(), 10);
        assert_eq!(Some(taken[0].clone()), stack("m:rock", 64));
        assert_eq!(Some(taken[9].clone()), stack("m:rock", 3));
        assert!(inventory.is_empty());
        assert_eq!(inventory.selected(), 4);
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
    fn a_take_into_the_backpack_fills_matching_stacks_then_empty_slots() {
        let mut inventory = Inventory::default();
        inventory.hotbar[0] = stack("t:rock", 1);
        inventory.backpack[0] = stack("t:dust", 3);
        inventory.backpack[1] = stack("t:rock", 60);
        assert_eq!(
            inventory.take("t:rock", 10, Destination::Backpack, limit),
            Ok(())
        );
        assert_eq!(inventory.backpack[1], stack("t:rock", 64));
        assert_eq!(inventory.backpack[2], stack("t:rock", 6));
        assert_eq!(inventory.hotbar[0], stack("t:rock", 1), "not the hotbar");
    }

    #[test]
    fn a_take_into_a_full_backpack_is_refused_and_changes_nothing() {
        let mut inventory = Inventory::default();
        for slot in &mut inventory.backpack {
            *slot = stack("t:dust", 64);
        }
        inventory.backpack[26] = stack("t:rock", 60);
        let before = inventory.clone();
        assert_eq!(
            inventory.take("t:rock", 5, Destination::Backpack, limit),
            Err(Refusal::BackpackFull),
            "four fit, so the take of five is refused whole"
        );
        assert_eq!(inventory, before);
    }

    #[test]
    fn a_take_onto_a_slot_fills_it_merges_or_sends_the_stack_there_to_the_backpack() {
        let mut inventory = Inventory::default();
        let onto = |slot| Destination::Slot(slot);
        assert_eq!(
            inventory.take("t:rock", 16, onto(Slot::Hotbar(2)), limit),
            Ok(())
        );
        assert_eq!(inventory.hotbar[2], stack("t:rock", 16));
        assert_eq!(
            inventory.take("t:rock", 64, onto(Slot::Hotbar(2)), limit),
            Ok(()),
            "the same Item tops up to its limit"
        );
        assert_eq!(inventory.hotbar[2], stack("t:rock", 64));
        assert_eq!(inventory.backpack(), &Inventory::default().backpack);
        inventory.backpack[0] = stack("t:rock", 10);
        assert_eq!(
            inventory.take("t:dust", 1, onto(Slot::Hotbar(2)), limit),
            Ok(())
        );
        assert_eq!(inventory.hotbar[2], stack("t:dust", 1));
        assert_eq!(inventory.backpack[0], stack("t:rock", 64));
        assert_eq!(inventory.backpack[1], stack("t:rock", 10));
        assert_eq!(
            inventory.take("t:zinc", 2, onto(Slot::Backpack(1)), limit),
            Ok(())
        );
        assert_eq!(inventory.backpack[1], stack("t:zinc", 2));
        assert_eq!(
            inventory.backpack[2],
            stack("t:rock", 10),
            "the displaced stack never lands back in its own slot"
        );
    }

    #[test]
    fn a_take_that_cannot_land_whole_is_refused_and_changes_nothing() {
        let mut inventory = Inventory::default();
        for slot in &mut inventory.backpack {
            *slot = stack("t:dust", 64);
        }
        inventory.hotbar[0] = stack("t:rock", 5);
        inventory.hotbar[1] = stack("t:zinc", 64);
        let before = inventory.clone();
        for (count, slot, refusal) in [
            (3, Slot::Hotbar(0), Refusal::BackpackFull),
            (3, Slot::Backpack(4), Refusal::BackpackFull),
            (1, Slot::Hotbar(1), Refusal::StackFull),
            (1, Slot::Hotbar(9), Refusal::NoSuchSlot),
            (0, Slot::Hotbar(5), Refusal::BadCount),
            (65, Slot::Hotbar(5), Refusal::BadCount),
        ] {
            let item = if slot == Slot::Hotbar(1) {
                "t:zinc"
            } else {
                "t:clay"
            };
            assert_eq!(
                inventory.take(item, count, Destination::Slot(slot), limit),
                Err(refusal),
                "{count} onto {slot:?}"
            );
        }
        assert_eq!(
            inventory.take("t:clay", 0, Destination::Backpack, limit),
            Err(Refusal::BadCount)
        );
        assert_eq!(inventory, before);
    }

    #[test]
    fn spending_takes_from_the_held_stack_first_then_the_others_in_slot_order() {
        let mut inventory = Inventory::default();
        inventory.hotbar[1] = stack("t:rock", 2);
        inventory.hotbar[3] = stack("t:rock", 3);
        inventory.backpack[0] = stack("t:rock", 10);
        inventory.backpack[1] = stack("t:dust", 4);
        inventory.select(3).unwrap();
        assert_eq!(inventory.count("t:rock"), 15);
        assert_eq!(inventory.spend("t:rock", 1), Ok(()));
        assert_eq!(inventory.hotbar[3], stack("t:rock", 2));
        assert_eq!(inventory.spend("t:rock", 5), Ok(()));
        assert_eq!(inventory.hotbar[3], None, "an emptied slot is empty");
        assert_eq!(inventory.hotbar[1], None);
        assert_eq!(inventory.backpack[0], stack("t:rock", 9));
        assert_eq!(inventory.count("t:rock"), 9);
    }

    #[test]
    fn spending_more_than_is_held_spends_nothing() {
        let mut inventory = Inventory::default();
        inventory.hotbar[0] = stack("t:rock", 2);
        inventory.backpack[5] = stack("t:rock", 1);
        let before = inventory.clone();
        assert_eq!(inventory.spend("t:rock", 4), Err(Refusal::TooFew));
        assert_eq!(inventory.spend("t:sand", 1), Err(Refusal::TooFew));
        assert_eq!(inventory, before);
        assert_eq!(inventory.spend("t:rock", 0), Ok(()));
        assert_eq!(inventory, before);
    }

    fn mine(index: usize) -> At {
        At {
            side: Side::First,
            index,
        }
    }

    fn theirs(index: usize) -> At {
        At {
            side: Side::Second,
            index,
        }
    }

    #[test]
    fn stacks_move_merge_and_quick_transfer_between_an_inventory_and_a_shorter_slot_set() {
        let mut inventory = Inventory::default();
        inventory.hotbar[0] = stack("t:rock", 10);
        inventory.backpack[0] = stack("t:dust", 5);
        let mut chest = vec![None; 5];
        chest[1] = stack("t:rock", 60);
        let mut sets = inventory.beside(&mut chest);
        assert_eq!(
            sets.move_stack(mine(HOTBAR_SLOTS), theirs(0), limit),
            Ok(()),
            "the Inventory's slots run hotbar then backpack"
        );
        assert_eq!(sets.move_stack(mine(0), theirs(1), limit), Ok(()));
        assert_eq!(sets.move_stack(theirs(0), mine(0), limit), Ok(()));
        assert_eq!(
            sets.move_stack(mine(0), theirs(5), limit),
            Err(Refusal::NoSuchSlot)
        );
        assert_eq!(sets.quick_transfer(theirs(1), limit), Ok(()));
        assert_eq!(sets.quick_transfer(mine(1), limit), Ok(()));
        drop(sets);
        assert_eq!(inventory.hotbar[0], stack("t:dust", 5), "swapped");
        assert_eq!(inventory.hotbar[1], None);
        assert_eq!(inventory.backpack[0], None);
        assert_eq!(
            chest,
            [stack("t:rock", 64), stack("t:rock", 6), None, None, None],
            "the rest of the merge swapped back, then the quick transfers \
             topped it up and spilled into the first empty slot"
        );
    }

    #[test]
    fn quick_transfer_into_a_full_slot_set_is_refused_and_split_and_sort_stay_in_their_set() {
        let mut inventory = Inventory::default();
        inventory.hotbar[0] = stack("t:rock", 8);
        let mut chest = vec![stack("t:dust", 60), stack("t:zinc", 3), stack("t:dust", 4)];
        let mut sets = inventory.beside(&mut chest);
        assert_eq!(sets.quick_transfer(mine(0), limit), Err(Refusal::NoRoom));
        assert_eq!(
            sets.split(theirs(1), Side::Second),
            Err(Refusal::NoEmptySlot)
        );
        assert_eq!(sets.sort(Side::Second, limit), Ok(()));
        assert_eq!(sets.split(theirs(1), Side::Second), Ok(()));
        assert_eq!(sets.split(mine(0), Side::First), Ok(()));
        drop(sets);
        assert_eq!(inventory.hotbar[0], stack("t:rock", 4));
        assert_eq!(inventory.hotbar[1], stack("t:rock", 4));
        assert_eq!(
            chest,
            [stack("t:dust", 64), stack("t:zinc", 2), stack("t:zinc", 1)]
        );
    }

    #[test]
    fn take_all_moves_what_fits_and_leaves_the_rest() {
        let mut inventory = Inventory::default();
        for slot in inventory.backpack.iter_mut() {
            *slot = stack("t:dust", 64);
        }
        inventory.hotbar[0] = stack("t:rock", 60);
        for slot in &mut inventory.hotbar[1..] {
            *slot = stack("t:dust", 64);
        }
        let mut chest = vec![stack("t:rock", 10), None, stack("t:zinc", 2)];
        let mut sets = inventory.beside(&mut chest);
        assert_eq!(sets.take_all(Side::Second, limit), Ok(()));
        assert_eq!(
            sets.take_all(Side::Second, limit),
            Err(Refusal::NoRoom),
            "nothing more fits"
        );
        drop(sets);
        assert_eq!(inventory.hotbar[0], stack("t:rock", 64));
        assert_eq!(chest, [stack("t:rock", 6), None, stack("t:zinc", 2)]);
        let mut empty = vec![None; 2];
        assert_eq!(
            inventory.beside(&mut empty).take_all(Side::Second, limit),
            Err(Refusal::Empty)
        );
    }

    #[test]
    fn receiving_nothing_changes_nothing() {
        let mut inventory = Inventory::default();
        assert_eq!(inventory.receive("t:rock", 0, 64), 0);
        assert_eq!(inventory, Inventory::default());
    }
}
