//! The Player's Inventory on the Server: what the Client is told of it, how a
//! save holds it, and the retired Carried totals packed into it.

use std::collections::BTreeMap;

use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use tesserae_core::entity::DropSpec;
use tesserae_core::inventory;
use tesserae_core::save::{CarriedV1, InventoryV1};

use super::ContentTables;
use super::entities::EntityStore;
use super::items::Items;
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

pub(super) fn plugin(app: &mut App) {
    app.add_session_message::<InventoryChanged>()
        .add_session_systems(
            FixedPreUpdate,
            announce_inventory.after(super::entities::pick_up),
        );
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
