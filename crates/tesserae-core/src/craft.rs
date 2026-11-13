//! Recipes and the Craft rule: Items from the Inventory in, one Stack out.

use crate::inventory::{Inventory, Refusal, Stack};

/// The most inputs one Recipe spends.
pub const MAX_INPUTS: usize = 9;
/// The station of a Recipe craftable anywhere; no Station may take it as a tag.
pub const HAND: &str = "hand";

/// Where a Recipe is crafted.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Station {
    Hand,
    Tag(String),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Recipe {
    pub inputs: Vec<Stack>,
    pub output: Stack,
    pub station: Station,
}

/// How many times a Craft runs.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Count {
    One,
    /// As many as the Inventory affords, up to one output Stack.
    Affordable,
}

impl Recipe {
    /// How many times the Inventory affords this Recipe, bounded so the output
    /// is at most one Stack of an Item that holds `max_stack`.
    pub fn affordable(&self, inventory: &Inventory, max_stack: u32) -> u32 {
        let by_inputs = self
            .inputs
            .iter()
            .map(|input| inventory.count(&input.item) / input.count)
            .min()
            .unwrap_or(0);
        by_inputs.min(max_stack / self.output.count)
    }
}

/// Spends the inputs and gives the output in one step, or changes nothing.
/// Returns what did not fit.
pub fn craft(
    inventory: &mut Inventory,
    recipe: &Recipe,
    count: Count,
    limit: impl Fn(&str) -> u32,
) -> Result<Option<Stack>, Refusal> {
    let max_stack = limit(&recipe.output.item);
    let times = match (count, recipe.affordable(inventory, max_stack)) {
        (_, 0) => return Err(Refusal::TooFew),
        (Count::One, _) => 1,
        (Count::Affordable, all) => all,
    };
    let mut next = inventory.clone();
    for input in &recipe.inputs {
        next.spend(&input.item, input.count * times)?;
    }
    let made = recipe.output.count * times;
    let left = next.receive(&recipe.output.item, made, max_stack);
    *inventory = next;
    Ok((left > 0).then(|| Stack {
        item: recipe.output.item.clone(),
        count: left,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::{BACKPACK_SLOTS, HOTBAR_SLOTS, Slot};

    fn stack(item: &str, count: u32) -> Stack {
        Stack {
            item: item.into(),
            count,
        }
    }

    fn workbench() -> Recipe {
        Recipe {
            inputs: vec![stack("t:wood", 4)],
            output: stack("t:workbench", 1),
            station: Station::Hand,
        }
    }

    fn limit(item: &str) -> u32 {
        if item == "t:workbench" { 16 } else { 64 }
    }

    fn holding(stacks: &[(&str, u32)]) -> Inventory {
        let mut inventory = Inventory::default();
        for &(item, count) in stacks {
            assert_eq!(inventory.receive(item, count, 64), 0);
        }
        inventory
    }

    #[test]
    fn affordability_is_the_scarcest_input_bounded_by_one_output_stack() {
        let brick = Recipe {
            inputs: vec![stack("t:rock", 2), stack("t:dirt", 1)],
            output: stack("t:brick", 2),
            station: Station::Hand,
        };
        assert_eq!(brick.affordable(&holding(&[("t:rock", 7)]), 64), 0);
        assert_eq!(
            brick.affordable(&holding(&[("t:rock", 7), ("t:dirt", 9)]), 64),
            3
        );
        let plenty = holding(&[("t:rock", 64), ("t:rock", 64), ("t:dirt", 64)]);
        assert_eq!(brick.affordable(&plenty, 64), 32, "64 bricks is one Stack");
        assert_eq!(
            brick.affordable(&plenty, 5),
            2,
            "a Stack of 5 holds 2 crafts"
        );
    }

    #[test]
    fn crafting_one_spends_exactly_the_inputs_and_gives_the_output() {
        let mut inventory = holding(&[("t:wood", 9)]);
        assert_eq!(
            craft(&mut inventory, &workbench(), Count::One, limit),
            Ok(None)
        );
        assert_eq!(inventory.count("t:wood"), 5);
        assert_eq!(inventory.count("t:workbench"), 1);
    }

    #[test]
    fn crafting_the_affordable_count_makes_at_most_one_output_stack() {
        let mut inventory = holding(&[("t:wood", 64), ("t:wood", 64)]);
        assert_eq!(
            craft(&mut inventory, &workbench(), Count::Affordable, limit),
            Ok(None)
        );
        assert_eq!(inventory.count("t:workbench"), 16);
        assert_eq!(inventory.count("t:wood"), 128 - 64);
    }

    #[test]
    fn a_craft_with_too_few_inputs_is_refused_and_changes_nothing() {
        let mut inventory = holding(&[("t:wood", 3)]);
        let before = inventory.clone();
        for count in [Count::One, Count::Affordable] {
            assert_eq!(
                craft(&mut inventory, &workbench(), count, limit),
                Err(Refusal::TooFew)
            );
            assert_eq!(inventory, before);
        }
    }

    #[test]
    fn output_with_no_room_is_returned_after_the_inputs_are_spent() {
        let mut inventory = Inventory::default();
        for _ in 0..HOTBAR_SLOTS + BACKPACK_SLOTS {
            inventory.receive("t:rock", 64, 64);
        }
        inventory.take_from(Slot::Hotbar(0), 64).unwrap();
        inventory.receive("t:wood", 12, 64);
        assert_eq!(
            craft(&mut inventory, &workbench(), Count::One, limit),
            Ok(Some(stack("t:workbench", 1))),
            "the wood slot still holds wood, so nothing takes the output"
        );
        assert_eq!(inventory.count("t:wood"), 8);
        assert_eq!(inventory.count("t:workbench"), 0);
        assert_eq!(
            craft(&mut inventory, &workbench(), Count::Affordable, limit),
            Ok(None),
            "spending the last 8 frees the slot the 2 land in"
        );
        assert_eq!(inventory.count("t:workbench"), 2);
    }
}
