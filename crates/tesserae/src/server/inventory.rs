//! The Player's Inventory on the Server: the Client's requests to arrange it,
//! what the Client is told of it, how a save holds it, and the retired Carried
//! totals packed into it. The Client only asks; see
//! `docs/adr/0002-client-server-from-day-one.md`.

use std::collections::BTreeMap;

use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use tesserae_core::MaterialId;
use tesserae_core::entity::DropSpec;
use tesserae_core::inventory::{self, Destination, Refusal, Slot};
use tesserae_core::save::{CarriedV1, InventoryV1};

use super::ContentTables;
use super::entities::EntityStore;
use super::items::{Items, PlacingItems};
use super::persistence::extensions::PlayerState;
use super::player::Player;
use super::world_state::ChunkStore;
use crate::session::SessionApp;
use crate::world::WorldSettings;

/// Every Item's stack limit until Items declare their own.
pub const MAX_STACK: u32 = 64;

#[derive(Component, Default, Clone, PartialEq, Debug, Deref, DerefMut)]
pub struct Inventory(pub inventory::Inventory);

/// The Player's whole Inventory, sent whenever it changes.
#[derive(Message, Clone, PartialEq, Debug)]
pub struct InventoryChanged(pub inventory::Inventory);

/// A Client's request to arrange the Inventory.
#[derive(Message, Clone, Copy, PartialEq, Eq, Debug)]
pub enum InventoryIntent {
    /// Into an empty slot, onto the same Item, or swapping with another.
    Move {
        from: Slot,
        to: Slot,
    },
    /// Between the backpack and the hotbar.
    QuickTransfer(Slot),
    Split(Slot),
    /// Merges and orders the backpack.
    Sort,
    /// Puts this hotbar slot in hand.
    Select(usize),
    /// From the Catalogue: a new stack of the Item that places `material`.
    Take {
        material: MaterialId,
        count: u32,
        into: Destination,
    },
}

/// Why the Server declined an intent.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reason {
    Rule(Refusal),
    /// A take of a Material no Item places.
    NotInCatalogue,
}

/// An intent the Server declined; the Inventory is as it was.
#[derive(Message, Clone, Copy, PartialEq, Eq, Debug)]
pub struct InventoryRefused(pub Reason);

/// For the Client to put on screen.
pub fn message(reason: Reason) -> &'static str {
    let Reason::Rule(refusal) = reason else {
        return "That Material is not in the Catalogue.";
    };
    match refusal {
        Refusal::NoSuchSlot => "That slot does not exist.",
        Refusal::Empty => "That slot is empty.",
        Refusal::StackFull => "That stack is full. Choose another slot.",
        Refusal::TooFewToSplit => "A single Item cannot be split.",
        Refusal::NoEmptySlot => "An empty backpack slot is needed to split this stack.",
        Refusal::NoRoom => "Not enough room for this stack.",
        Refusal::BackpackFull => "The backpack is full. Make room first.",
        Refusal::BadCount => "Choose a count from 1 to the stack limit.",
    }
}

pub(crate) fn plugin(app: &mut App) {
    app.add_session_message::<InventoryChanged>()
        .add_session_message::<InventoryIntent>()
        .add_session_message::<InventoryRefused>()
        .add_session_systems(
            FixedPreUpdate,
            (
                arrange.run_if(super::persistence::playing),
                announce_inventory.after(super::entities::pick_up),
            )
                .chain(),
        );
}

fn arrange(
    mut intents: MessageReader<InventoryIntent>,
    mut inventory: Single<&mut Inventory>,
    placing: Option<Res<PlacingItems>>,
    mut refused: MessageWriter<InventoryRefused>,
) {
    for intent in intents.read() {
        let mut next = inventory.0.clone();
        let limit = |_: &str| MAX_STACK;
        let result = match *intent {
            InventoryIntent::Move { from, to } => next.move_stack(from, to, limit),
            InventoryIntent::QuickTransfer(from) => next.quick_transfer(from, limit),
            InventoryIntent::Split(from) => next.split(from),
            InventoryIntent::Sort => next.sort(limit),
            InventoryIntent::Select(at) => next.select(at),
            InventoryIntent::Take {
                material,
                count,
                into,
            } => {
                // TODO(game mode): refuse a take unless the Player is in Creative.
                let Some(item) = placing.as_ref().and_then(|p| p.item(material)) else {
                    refused.write(InventoryRefused(Reason::NotInCatalogue));
                    continue;
                };
                next.take(item, count, into, limit)
            }
        };
        match result {
            Ok(()) => {
                inventory.set_if_neq(Inventory(next));
            }
            Err(refusal) => {
                refused.write(InventoryRefused(Reason::Rule(refusal)));
            }
        }
    }
}

pub(super) fn announce_inventory(
    inventory: Single<Ref<Inventory>>,
    mut changed: MessageWriter<InventoryChanged>,
) {
    if inventory.is_changed() {
        changed.write(InventoryChanged(inventory.0.clone()));
    }
}

impl PlayerState for InventoryV1 {
    fn capture(player: EntityRef) -> Option<Self> {
        player
            .get::<Inventory>()
            .filter(|i| !i.is_empty())
            .map(|i| InventoryV1(i.0.clone()))
    }

    fn restore(self, player: &mut EntityCommands) {
        player.insert(Inventory(self.0));
    }
}

/// Read once from a save made before the Inventory, never written again.
impl PlayerState for CarriedV1 {
    fn capture(_: EntityRef) -> Option<Self> {
        None
    }

    fn restore(self, player: &mut EntityCommands) {
        let id = player.id();
        player
            .commands()
            .queue(move |world: &mut World| pack_carried(world, id, self.0));
    }
}

fn pack_carried(world: &mut World, player: Entity, carried: BTreeMap<String, u32>) {
    let Some(mut inventory) = world.get_mut::<Inventory>(player) else {
        return;
    };
    let overflow: Vec<_> = carried
        .into_iter()
        .filter_map(|(item, count)| {
            let left = inventory.receive(&item, count, MAX_STACK);
            (left > 0).then_some((item, left))
        })
        .collect();
    drop_beside(world, player, overflow);
}

/// Drops stacks around the Player's body, by Item name. An Item the Game no
/// longer declares has no drop to become, so it is lost.
pub(super) fn drop_beside(
    world: &mut World,
    player: Entity,
    stacks: impl IntoIterator<Item = (String, u32)>,
) {
    let Some(body) = world.get::<Player>(player).map(Player::body) else {
        return;
    };
    let items = world.resource::<Items>();
    let specs: Vec<_> = stacks
        .into_iter()
        .filter_map(|(name, count)| {
            let at = items.named(&name)?;
            Some(DropSpec {
                item: at as u32,
                count,
                size_m: items.0[at].size_m,
            })
        })
        .collect();
    if specs.is_empty() {
        return;
    }
    let centre = [0, 1, 2].map(|a| (body.min[a] + body.max[a]) / 2.0);
    world.resource_scope(|world, mut entities: Mut<EntityStore>| {
        entities.spawn_drops_around(
            world.resource::<ChunkStore>(),
            &world.resource::<ContentTables>().0,
            world.resource::<WorldSettings>().voxel_size,
            centre,
            specs,
        );
    });
}

#[cfg(test)]
mod tests {
    use bevy::ecs::message::Messages;
    use tesserae_core::MaterialId;
    use tesserae_core::inventory::{Destination, Refusal, Slot, Stack};

    use super::*;

    const ROCK: MaterialId = 3;
    const SAND: MaterialId = 4;

    fn game() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_plugins(plugin);
        app.insert_resource(PlacingItems(vec![
            None,
            None,
            None,
            Some("t:rock".into()),
            Some("t:sand".into()),
        ]));
        app.world_mut().spawn(Player::at(Vec3::ZERO));
        tick(&mut app);
        app.world_mut()
            .resource_mut::<Messages<InventoryChanged>>()
            .clear();
        app
    }

    fn tick(app: &mut App) {
        app.world_mut().run_schedule(FixedPreUpdate);
    }

    fn holding(app: &mut App, stacks: &[(Slot, &str, u32)]) {
        let empty = |n| vec![serde_json::Value::Null; n];
        let mut slots = serde_json::json!({"hotbar": empty(9), "backpack": empty(27)});
        for &(slot, item, count) in stacks {
            let (area, at) = match slot {
                Slot::Hotbar(at) => ("hotbar", at),
                Slot::Backpack(at) => ("backpack", at),
            };
            slots[area][at] = serde_json::json!({"item": item, "count": count});
        }
        let inventory: inventory::Inventory = serde_json::from_value(slots).unwrap();
        let mut held = app.world_mut().query::<&mut Inventory>();
        held.single_mut(app.world_mut()).unwrap().0 = inventory;
        tick(app);
        app.world_mut()
            .resource_mut::<Messages<InventoryChanged>>()
            .clear();
    }

    /// The Server's answers to one intent: the Inventories sent and the refusals.
    fn ask(app: &mut App, intent: InventoryIntent) -> (Vec<inventory::Inventory>, Vec<Reason>) {
        app.world_mut().write_message(intent);
        tick(app);
        let changed = app
            .world_mut()
            .resource_mut::<Messages<InventoryChanged>>()
            .drain()
            .map(|m| m.0)
            .collect();
        let refused = app
            .world_mut()
            .resource_mut::<Messages<InventoryRefused>>()
            .drain()
            .map(|m| m.0)
            .collect();
        (changed, refused)
    }

    fn stack(item: &str, count: u32) -> Option<Stack> {
        Some(Stack {
            item: item.into(),
            count,
        })
    }

    #[test]
    fn a_move_merge_and_swap_are_applied_and_the_new_inventory_is_sent() {
        let mut app = game();
        holding(
            &mut app,
            &[
                (Slot::Backpack(0), "t:rock", 10),
                (Slot::Backpack(1), "t:rock", 60),
                (Slot::Hotbar(4), "t:dust", 3),
            ],
        );
        let (sent, refused) = ask(
            &mut app,
            InventoryIntent::Move {
                from: Slot::Backpack(0),
                to: Slot::Backpack(1),
            },
        );
        assert!(refused.is_empty());
        let [now] = &sent[..] else {
            panic!("one Inventory sent: {sent:?}");
        };
        assert_eq!(now.get(Slot::Backpack(1)).cloned(), stack("t:rock", 64));
        assert_eq!(now.get(Slot::Backpack(0)).cloned(), stack("t:rock", 6));
        let (sent, _) = ask(
            &mut app,
            InventoryIntent::Move {
                from: Slot::Hotbar(4),
                to: Slot::Backpack(0),
            },
        );
        assert_eq!(sent[0].get(Slot::Backpack(0)).cloned(), stack("t:dust", 3));
        assert_eq!(sent[0].get(Slot::Hotbar(4)).cloned(), stack("t:rock", 6));
    }

    #[test]
    fn split_sort_and_quick_transfer_are_applied() {
        let mut app = game();
        holding(
            &mut app,
            &[
                (Slot::Backpack(5), "t:rock", 9),
                (Slot::Backpack(2), "t:dust", 4),
            ],
        );
        let (sent, _) = ask(&mut app, InventoryIntent::Split(Slot::Backpack(5)));
        assert_eq!(sent[0].get(Slot::Backpack(5)).cloned(), stack("t:rock", 5));
        assert_eq!(sent[0].get(Slot::Backpack(0)).cloned(), stack("t:rock", 4));
        let (sent, _) = ask(&mut app, InventoryIntent::Sort);
        assert_eq!(
            [0, 1, 2].map(|at| sent[0].get(Slot::Backpack(at)).cloned()),
            [stack("t:dust", 4), stack("t:rock", 9), None]
        );
        let (sent, _) = ask(&mut app, InventoryIntent::QuickTransfer(Slot::Backpack(1)));
        assert_eq!(sent[0].get(Slot::Hotbar(0)).cloned(), stack("t:rock", 9));
        assert_eq!(sent[0].get(Slot::Backpack(1)).cloned(), None);
    }

    #[test]
    fn an_impossible_change_is_refused_with_a_message_and_changes_nothing() {
        let mut app = game();
        let full: Vec<_> = (0..27)
            .map(|at| (Slot::Backpack(at), "t:dust", 2))
            .chain([
                (Slot::Hotbar(0), "t:rock", 8),
                (Slot::Hotbar(1), "t:dust", 64),
            ])
            .collect();
        holding(&mut app, &full);
        let before = app
            .world_mut()
            .query::<&Inventory>()
            .single(app.world())
            .unwrap()
            .0
            .clone();
        for (intent, refusal) in [
            (
                InventoryIntent::Split(Slot::Hotbar(0)),
                Refusal::NoEmptySlot,
            ),
            (
                InventoryIntent::Move {
                    from: Slot::Backpack(0),
                    to: Slot::Hotbar(1),
                },
                Refusal::StackFull,
            ),
            (InventoryIntent::Split(Slot::Hotbar(7)), Refusal::Empty),
        ] {
            let (sent, refused) = ask(&mut app, intent);
            assert!(sent.is_empty(), "{intent:?} sent {sent:?}");
            assert_eq!(refused, [Reason::Rule(refusal)]);
            assert!(!message(Reason::Rule(refusal)).is_empty());
        }
        let after = app
            .world_mut()
            .query::<&Inventory>()
            .single(app.world())
            .unwrap()
            .0
            .clone();
        assert_eq!(after, before);
    }

    #[test]
    fn selecting_a_hotbar_slot_sends_the_inventory_and_a_slot_past_the_bar_is_refused() {
        let mut app = game();
        holding(&mut app, &[(Slot::Hotbar(4), "t:rock", 7)]);
        let (sent, refused) = ask(&mut app, InventoryIntent::Select(4));
        assert!(refused.is_empty());
        assert_eq!(sent[0].held().cloned(), stack("t:rock", 7));
        let (sent, _) = ask(&mut app, InventoryIntent::Select(4));
        assert!(sent.is_empty(), "the same slot again changes nothing");
        let (sent, refused) = ask(&mut app, InventoryIntent::Select(9));
        assert!(sent.is_empty());
        assert_eq!(refused, [Reason::Rule(Refusal::NoSuchSlot)]);
    }

    fn take(material: MaterialId, count: u32, into: Destination) -> InventoryIntent {
        InventoryIntent::Take {
            material,
            count,
            into,
        }
    }

    #[test]
    fn a_take_lands_the_item_that_places_the_material_with_its_count() {
        let mut app = game();
        let (sent, refused) = ask(&mut app, take(ROCK, 16, Destination::Slot(Slot::Hotbar(2))));
        assert!(refused.is_empty(), "{refused:?}");
        assert_eq!(sent[0].get(Slot::Hotbar(2)).cloned(), stack("t:rock", 16));
        let (sent, _) = ask(&mut app, take(SAND, 64, Destination::Backpack));
        assert_eq!(sent[0].get(Slot::Backpack(0)).cloned(), stack("t:sand", 64));
        let (sent, _) = ask(&mut app, take(SAND, 5, Destination::Slot(Slot::Hotbar(2))));
        assert_eq!(sent[0].get(Slot::Hotbar(2)).cloned(), stack("t:sand", 5));
        assert_eq!(
            sent[0].get(Slot::Backpack(1)).cloned(),
            stack("t:rock", 16),
            "the displaced stack moves to the backpack"
        );
    }

    #[test]
    fn a_take_into_a_full_backpack_or_of_what_nothing_places_is_refused() {
        let mut app = game();
        let full: Vec<_> = (0..27)
            .map(|at| (Slot::Backpack(at), "t:dust", 64))
            .chain([(Slot::Hotbar(0), "t:dust", 64)])
            .collect();
        holding(&mut app, &full);
        for (intent, reason) in [
            (
                take(ROCK, 1, Destination::Backpack),
                Reason::Rule(Refusal::BackpackFull),
            ),
            (
                take(ROCK, 1, Destination::Slot(Slot::Hotbar(0))),
                Reason::Rule(Refusal::BackpackFull),
            ),
            (
                take(ROCK, 65, Destination::Slot(Slot::Hotbar(1))),
                Reason::Rule(Refusal::BadCount),
            ),
            (take(0, 1, Destination::Backpack), Reason::NotInCatalogue),
            (take(99, 1, Destination::Backpack), Reason::NotInCatalogue),
        ] {
            let (sent, refused) = ask(&mut app, intent);
            assert!(sent.is_empty(), "{intent:?} sent {sent:?}");
            assert_eq!(refused, [reason], "{intent:?}");
            assert!(!message(reason).is_empty());
        }
    }
}
