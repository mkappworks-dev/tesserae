//! Bounded player-edit inverses, applied through the normal World change path.
use super::{
    ContentTables,
    edit::{self, EditRejected, RejectReason},
    entities::{self, EntityStore},
    inventory::{self, Inventory},
    items::Items,
    mods::EditHistory,
    player::Player,
    streaming::MeshState,
    world_state::{ChunkStore, Lifetime, VoxelCoord},
};
use crate::world::WorldSettings;
use bevy::prelude::*;
use std::collections::{BTreeMap, VecDeque};
use tesserae_core::Occupants;
use tesserae_core::collide::Terrain;
use tesserae_core::entity::EntityId;
use tesserae_core::inventory::Stack;
use tesserae_mods::{EditEvent, EditKind};

const LIMIT: usize = 128;
const REQUEST_LIMIT: usize = 16;
/// Voxels the journal holds across all groups, so layouts cannot make 128
/// entries cost 128 times 256 records.
const RECORDED_LIMIT: usize = 1024;

#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryAction {
    Undo,
    Redo,
}
#[derive(Clone)]
pub(super) struct Entry {
    pub at: [i32; 3],
    pub lifetime: Lifetime,
    pub before: Occupants,
    pub after: Occupants,
    pub event: EditEvent,
    pub cost: Cost,
}

/// What an Entry's Edit did to the Player's Items, settled again by undo and
/// redo. Kept from when the Edit was made, so an Entry pays and refunds by the
/// rules it was made under whatever the mode is when it is undone.
#[derive(Clone, PartialEq, Debug)]
pub(super) enum Cost {
    Free,
    /// Placing spent one of this Item.
    Paid(String),
    /// Removing dropped one of this Item as this Dropped item.
    Dropped(String, EntityId),
}
/// The Voxels one Edit or one layout changed, undone and redone together.
type Group = Vec<Entry>;

#[derive(Resource, Default)]
pub(super) struct Journal {
    undo: VecDeque<Group>,
    redo: VecDeque<Group>,
    requests: VecDeque<HistoryAction>,
}
impl Journal {
    #[cfg(test)]
    pub(super) fn is_empty(&self) -> bool {
        self.undo.is_empty() && self.redo.is_empty()
    }

    pub(super) fn cancel_requests(&mut self) {
        self.requests.clear();
    }

    pub(super) fn clear(&mut self) {
        *self = Self::default();
    }

    pub(super) fn record(&mut self, entry: Entry) {
        self.record_group(vec![entry]);
    }

    pub(super) fn record_group(&mut self, mut group: Group) {
        group.retain(|entry| entry.before != entry.after);
        if group.is_empty() {
            return;
        }
        if self.undo.len() == LIMIT {
            self.undo.pop_front();
        }
        self.undo.push_back(group);
        while self.undo.iter().map(Vec::len).sum::<usize>() > RECORDED_LIMIT && self.undo.len() > 1
        {
            self.undo.pop_front();
        }
        self.redo.clear();
    }
}
pub(super) fn receive(
    mut messages: MessageReader<HistoryAction>,
    mut journal: ResMut<Journal>,
    mut rejected: MessageWriter<EditRejected>,
) {
    for &action in messages.read() {
        if journal.requests.len() == REQUEST_LIMIT {
            rejected.write(EditRejected {
                reason: RejectReason::Busy,
            });
        } else {
            journal.requests.push_back(action);
        }
    }
}
/// The Inventory and Dropped items an undo or redo settles with.
struct Settle<'a> {
    inventory: Mut<'a, Inventory>,
    body: tesserae_core::collide::Aabb,
    entities: &'a mut EntityStore,
    items: &'a Items,
    voxel_size: f32,
}

pub(super) fn apply(
    mut journal: ResMut<Journal>,
    pending: Res<edit::PendingEdits>,
    (mut store, mut meshes, mut entities): (
        ResMut<ChunkStore>,
        ResMut<MeshState>,
        ResMut<EntityStore>,
    ),
    (tables, items, world): (Res<ContentTables>, Res<Items>, Res<WorldSettings>),
    mut history: ResMut<EditHistory>,
    mut players: Query<(&Player, &mut Inventory)>,
    mut rejected: MessageWriter<EditRejected>,
) {
    let Ok((player, inventory)) = players.single_mut() else {
        return;
    };
    let Some(action) = journal.requests.pop_front() else {
        return;
    };
    let group = match action {
        HistoryAction::Undo => journal.undo.back(),
        HistoryAction::Redo => journal.redo.back(),
    }
    .cloned();
    let Some(mut group) = group else {
        rejected.write(EditRejected {
            reason: if action == HistoryAction::Undo {
                RejectReason::NoUndo
            } else {
                RejectReason::NoRedo
            },
        });
        return;
    };
    let mut settle = Settle {
        inventory,
        body: player.body(),
        entities: &mut entities,
        items: &items,
        voxel_size: world.voxel_size,
    };
    let result = if !pending.is_empty() {
        Err(RejectReason::Busy)
    } else {
        restore(
            &mut store,
            &mut meshes,
            &tables,
            &mut settle,
            &mut group,
            action,
        )
    };
    match result {
        Ok(()) => {
            // The Mods hear the changes in the order they were made, or in
            // reverse when they are taken back.
            let mut events: Vec<EditEvent> = group.iter().map(|entry| entry.event).collect();
            if action == HistoryAction::Undo {
                events.reverse();
                for event in &mut events {
                    std::mem::swap(&mut event.before, &mut event.after);
                    event.kind = inverse(event.kind);
                }
                journal.undo.pop_back();
                journal.redo.push_back(group);
            } else {
                journal.redo.pop_back();
                journal.undo.push_back(group);
            }
            for event in events {
                history.record(event);
            }
        }
        Err(reason) => {
            rejected.write(EditRejected { reason });
        }
    }
}
/// Replays a whole group or none of it: every cell and every Item it needs is
/// checked before the first is changed, and a change that fails midway is
/// taken back.
fn restore(
    store: &mut ChunkStore,
    meshes: &mut MeshState,
    tables: &tesserae_core::Tables,
    settle: &mut Settle,
    group: &mut [Entry],
    action: HistoryAction,
) -> Result<(), RejectReason> {
    let undoing = action == HistoryAction::Undo;
    let step = |entry: &Entry| {
        if undoing {
            (entry.after, entry.before)
        } else {
            (entry.before, entry.after)
        }
    };
    for entry in group.iter() {
        let chunk = VoxelCoord::of(entry.at).chunk;
        let lifetime = store.lifetime(chunk).ok_or(RejectReason::NotLoaded)?;
        if lifetime != entry.lifetime {
            return Err(RejectReason::HistoryChanged);
        }
        if meshes.is_unlit(chunk) {
            return Err(RejectReason::Busy);
        }
        if store.occupants(entry.at, &tables.fluids) != Some(step(entry).0) {
            return Err(RejectReason::HistoryChanged);
        }
    }
    let owed = owed(group, action, settle.entities);
    if owed
        .iter()
        .any(|(item, &count)| settle.inventory.count(item) < count)
    {
        return Err(RejectReason::ItemNeeded);
    }
    let order: Vec<&Entry> = if undoing {
        group.iter().rev().collect()
    } else {
        group.iter().collect()
    };
    for (done, entry) in order.iter().enumerate() {
        if let Err(reason) = edit::set_occupants(store, meshes, tables, entry.at, step(entry).1) {
            for undone in order[..done].iter().rev() {
                let _ = edit::set_occupants(store, meshes, tables, undone.at, step(undone).0);
            }
            return Err(reason);
        }
    }
    let mut refused = Vec::new();
    for entry in group.iter_mut() {
        match (&entry.cost, undoing) {
            (Cost::Free, _) => {}
            (Cost::Paid(item), true) => {
                let limit = settle.items.stack_limit(item);
                if settle.inventory.receive(item, 1, limit) > 0 {
                    refused.push(Stack {
                        item: item.clone(),
                        count: 1,
                    });
                }
            }
            (Cost::Paid(item), false) => {
                settle.inventory.spend(item, 1).expect("owed and counted");
            }
            (Cost::Dropped(item, id), true) => {
                if settle.entities.get(*id).is_some() {
                    settle.entities.take(*id, 1);
                } else {
                    settle.inventory.spend(item, 1).expect("owed and counted");
                }
            }
            (Cost::Dropped(item, _), false) => {
                let item = item.clone();
                if let Some(at) = settle.items.named(&item) {
                    let id = entities::drop_one(
                        settle.entities,
                        &Terrain {
                            voxels: &*store,
                            tables,
                            voxel_size: settle.voxel_size,
                        },
                        settle.items,
                        entry.at,
                        at,
                    );
                    entry.cost = Cost::Dropped(item, id);
                }
            }
        }
    }
    inventory::drop_around(
        settle.entities,
        &Terrain {
            voxels: &*store,
            tables,
            voxel_size: settle.voxel_size,
        },
        settle.items,
        &settle.body,
        refused,
    );
    Ok(())
}
/// The Items restoring `group` takes from the Inventory: each placement a redo
/// lays again, and each removal an undo puts back whose Dropped item is gone.
fn owed<'a>(
    group: &'a [Entry],
    action: HistoryAction,
    entities: &EntityStore,
) -> BTreeMap<&'a str, u32> {
    let mut owed = BTreeMap::new();
    for entry in group {
        let item = match (&entry.cost, action) {
            (Cost::Paid(item), HistoryAction::Redo) => item,
            (Cost::Dropped(item, id), HistoryAction::Undo) if entities.get(*id).is_none() => item,
            _ => continue,
        };
        *owed.entry(item.as_str()).or_insert(0) += 1;
    }
    owed
}
fn inverse(kind: EditKind) -> EditKind {
    match kind {
        EditKind::Solid { before, after } => EditKind::Solid {
            before: after,
            after: before,
        },
        EditKind::SourcePlaced(id) => EditKind::SourceRemoved(id),
        EditKind::SourceRemoved(id) => EditKind::SourcePlaced(id),
        EditKind::Ignited(id) => EditKind::PutOut(id),
        EditKind::PutOut(id) => EditKind::Ignited(id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_undo_history_and_pending_edits_do_not_outlive_their_world() {
        use crate::session::testing::{app, enter, leave};
        let mut app = app();
        app.add_plugins(super::super::edit::plugin);
        enter(&mut app);
        app.world_mut()
            .resource_mut::<Journal>()
            .requests
            .push_back(HistoryAction::Undo);
        leave(&mut app);
        assert!(!app.world().contains_resource::<Journal>());
        assert!(
            !app.world()
                .contains_resource::<super::super::edit::PendingEdits>()
        );
        enter(&mut app);
        assert!(app.world().resource::<Journal>().requests.is_empty());
        assert!(
            app.world()
                .resource::<super::super::edit::PendingEdits>()
                .is_empty()
        );
    }
    #[test]
    fn entries_and_requests_are_bounded_and_fresh_edits_clear_redo() {
        use bevy::ecs::system::RunSystemOnce;
        use tesserae_core::{Chunk, Voxel};
        let mut store = ChunkStore::default();
        let coord = VoxelCoord::of([0, 0, 0]).chunk;
        store.insert(coord, Chunk::air());
        let entry = Entry {
            at: [0, 0, 0],
            lifetime: store.lifetime(coord).unwrap(),
            before: Occupants {
                solid: Voxel::AIR,
                fluid: None,
                burning: None,
            },
            after: Occupants {
                solid: Voxel::cube(8),
                fluid: None,
                burning: None,
            },
            event: EditEvent {
                at: [0.0; 3],
                before: Voxel::AIR,
                after: Voxel::cube(8),
                kind: EditKind::Solid {
                    before: Voxel::AIR,
                    after: Voxel::cube(8),
                },
            },
            cost: Cost::Free,
        };
        let mut journal = Journal::default();
        for _ in 0..LIMIT + 10 {
            journal.record(entry.clone());
        }
        assert_eq!(journal.undo.len(), LIMIT);
        journal.redo.push_back(vec![entry.clone()]);
        journal.record(entry);
        assert!(journal.redo.is_empty());
        let mut app = App::new();
        app.insert_resource(journal)
            .add_message::<HistoryAction>()
            .add_message::<EditRejected>();
        for _ in 0..REQUEST_LIMIT + 4 {
            app.world_mut().write_message(HistoryAction::Undo);
        }
        app.world_mut().run_system_once(receive).unwrap();
        assert_eq!(
            app.world().resource::<Journal>().requests.len(),
            REQUEST_LIMIT
        );
        assert_eq!(
            app.world_mut()
                .resource_mut::<Messages<EditRejected>>()
                .drain()
                .count(),
            4
        );
    }

    mod survival {
        use super::super::super::edit::tests::{
            down, held, place_held, rejections, replay, send, stocked, stone, survival, tick,
            voxel_at,
        };
        use super::super::super::edit::{EditAction, RejectReason};
        use super::super::super::entities::EntityStore;
        use super::super::super::inventory::Inventory;
        use super::super::super::items::Items;
        use super::super::HistoryAction::{Redo, Undo};
        use bevy::prelude::*;
        use tesserae_core::inventory::{BACKPACK_SLOTS, HOTBAR_SLOTS};
        use tesserae_core::{Orientation, SHAPE_FULL_CUBE};

        const ABOVE: [i32; 3] = [5, 8, 5];
        const GROUND: [i32; 3] = [5, 7, 5];

        /// Three `t:stone` in hand, the Item a stone Voxel drops.
        fn game(survival_mode: bool) -> App {
            let mut app = stocked(&[(4, "t:stone")], 4);
            app.insert_resource(Items(vec![tesserae_mods::ItemDef {
                name: "t:stone".into(),
                size_m: 0.25,
                color: [0; 4],
                material: Some(stone().material()),
                max_stack: 64,
                food: None,
            }]));
            if survival_mode {
                survival(&mut app);
            }
            app
        }

        fn place(app: &mut App) {
            send(app, place_held(SHAPE_FULL_CUBE, Orientation::IDENTITY));
            tick(app);
            assert!(rejections(app).is_empty());
        }

        fn remove(app: &mut App) {
            send(app, down(EditAction::Remove));
            tick(app);
            assert!(rejections(app).is_empty());
        }

        fn inventory(app: &mut App) -> Inventory {
            let mut query = app.world_mut().query::<&Inventory>();
            query.single(app.world()).unwrap().clone()
        }

        fn set_inventory(app: &mut App, to: Inventory) {
            let mut query = app.world_mut().query::<&mut Inventory>();
            *query.single_mut(app.world_mut()).unwrap() = to;
        }

        fn drops(app: &App) -> Vec<(u32, u32)> {
            app.world()
                .resource::<EntityStore>()
                .iter()
                .map(|e| (e.as_dropped().item, e.as_dropped().count))
                .collect()
        }

        #[test]
        fn undo_then_redo_of_a_survival_placement_returns_the_inventory_to_each_prior_state() {
            let mut app = game(true);
            let before = inventory(&mut app);
            place(&mut app);
            let placed = inventory(&mut app);
            assert_eq!(held(&mut app, "t:stone"), 2);
            replay(&mut app, Undo);
            assert!(rejections(&mut app).is_empty());
            assert!(voxel_at(&app, ABOVE).is_air());
            assert_eq!(inventory(&mut app), before);
            replay(&mut app, Redo);
            assert!(rejections(&mut app).is_empty());
            assert_eq!(voxel_at(&app, ABOVE), stone());
            assert_eq!(inventory(&mut app), placed);
        }

        #[test]
        fn a_redo_the_player_cannot_pay_for_is_refused_and_changes_nothing() {
            let mut app = game(true);
            place(&mut app);
            replay(&mut app, Undo);
            set_inventory(&mut app, Inventory::default());
            replay(&mut app, Redo);
            assert_eq!(rejections(&mut app), [RejectReason::ItemNeeded]);
            assert!(voxel_at(&app, ABOVE).is_air());
            assert_eq!(inventory(&mut app), Inventory::default());
            assert!(!RejectReason::ItemNeeded.message().is_empty());
        }

        #[test]
        fn a_refund_with_no_room_drops_at_the_player() {
            let mut app = game(true);
            place(&mut app);
            let mut full = Inventory::default();
            let slots = (HOTBAR_SLOTS + BACKPACK_SLOTS) as u32;
            assert_eq!(full.receive("t:dust", 64 * slots, 64), 0);
            set_inventory(&mut app, full.clone());
            replay(&mut app, Undo);
            assert!(rejections(&mut app).is_empty());
            assert!(voxel_at(&app, ABOVE).is_air());
            assert_eq!(inventory(&mut app), full);
            assert_eq!(drops(&app), [(0, 1)]);
        }

        #[test]
        fn undoing_a_removal_despawns_its_drop_and_spends_nothing_and_redo_drops_it_again() {
            let mut app = game(true);
            remove(&mut app);
            assert_eq!(drops(&app), [(0, 1)]);
            let before = inventory(&mut app);
            replay(&mut app, Undo);
            assert!(rejections(&mut app).is_empty());
            assert_eq!(voxel_at(&app, GROUND), stone());
            assert_eq!(drops(&app), []);
            assert_eq!(inventory(&mut app), before);
            replay(&mut app, Redo);
            assert!(rejections(&mut app).is_empty());
            assert!(voxel_at(&app, GROUND).is_air());
            assert_eq!(drops(&app), [(0, 1)]);
            assert_eq!(inventory(&mut app), before);
        }

        #[test]
        fn undoing_a_removal_whose_drop_is_gone_spends_the_item_or_is_refused_without_it() {
            let mut app = game(true);
            remove(&mut app);
            app.world_mut().resource_mut::<EntityStore>().0 = Default::default();
            set_inventory(&mut app, Inventory::default());
            replay(&mut app, Undo);
            assert_eq!(rejections(&mut app), [RejectReason::ItemNeeded]);
            assert!(voxel_at(&app, GROUND).is_air());
            assert_eq!(inventory(&mut app), Inventory::default());
            let mut one = Inventory::default();
            one.receive("t:stone", 1, 64);
            set_inventory(&mut app, one);
            replay(&mut app, Undo);
            assert!(rejections(&mut app).is_empty());
            assert_eq!(voxel_at(&app, GROUND), stone());
            assert_eq!(inventory(&mut app), Inventory::default());
            assert_eq!(drops(&app), []);
        }

        /// Soil in hand, placed and dropped by its implicit Item, Item 2.
        fn implicit() -> App {
            let mut app = stocked(&[(4, "t:soil")], 4);
            survival(&mut app);
            app
        }

        #[test]
        fn an_implicit_items_refund_with_no_room_drops_at_the_player() {
            let mut app = implicit();
            place(&mut app);
            let mut full = Inventory::default();
            let slots = (HOTBAR_SLOTS + BACKPACK_SLOTS) as u32;
            assert_eq!(full.receive("t:dust", 64 * slots, 64), 0);
            set_inventory(&mut app, full.clone());
            replay(&mut app, Undo);
            assert!(rejections(&mut app).is_empty());
            assert!(voxel_at(&app, ABOVE).is_air());
            assert_eq!(inventory(&mut app), full);
            assert_eq!(drops(&app), [(2, 1)]);
        }

        #[test]
        fn redoing_the_removal_of_an_implicit_items_voxel_drops_it_again() {
            let mut app = implicit();
            place(&mut app);
            send(&mut app, down(EditAction::Remove));
            tick(&mut app);
            assert!(rejections(&mut app).is_empty());
            assert_eq!(drops(&app), [(2, 1)]);
            replay(&mut app, Undo);
            assert!(rejections(&mut app).is_empty());
            assert_eq!(drops(&app), []);
            replay(&mut app, Redo);
            assert!(rejections(&mut app).is_empty());
            assert!(voxel_at(&app, ABOVE).is_air());
            assert_eq!(drops(&app), [(2, 1)]);
        }

        /// Water in hand, placed and dropped by its implicit Item, Item 3.
        fn watered() -> App {
            let mut app = stocked(&[(4, "t:water")], 4);
            survival(&mut app);
            app
        }

        fn source() -> tesserae_core::Voxel {
            tesserae_core::Voxel::cube(tesserae_core::testgen::fixtures().water)
        }

        #[test]
        fn undoing_a_source_removal_despawns_its_drop_and_spends_nothing_and_redo_drops_it_again() {
            let mut app = watered();
            place(&mut app);
            remove(&mut app);
            assert_eq!(drops(&app), [(3, 1)]);
            let before = inventory(&mut app);
            replay(&mut app, Undo);
            assert!(rejections(&mut app).is_empty());
            assert_eq!(voxel_at(&app, ABOVE), source());
            assert_eq!(drops(&app), []);
            assert_eq!(inventory(&mut app), before);
            replay(&mut app, Redo);
            assert!(rejections(&mut app).is_empty());
            assert!(voxel_at(&app, ABOVE).is_air());
            assert_eq!(drops(&app), [(3, 1)]);
            assert_eq!(inventory(&mut app), before);
        }

        #[test]
        fn undoing_a_source_removal_whose_drop_is_gone_spends_the_item_or_is_refused_without_it() {
            let mut app = watered();
            place(&mut app);
            remove(&mut app);
            app.world_mut().resource_mut::<EntityStore>().0 = Default::default();
            set_inventory(&mut app, Inventory::default());
            replay(&mut app, Undo);
            assert_eq!(rejections(&mut app), [RejectReason::ItemNeeded]);
            assert!(voxel_at(&app, ABOVE).is_air());
            let mut one = Inventory::default();
            one.receive("t:water", 1, 64);
            set_inventory(&mut app, one);
            replay(&mut app, Undo);
            assert!(rejections(&mut app).is_empty());
            assert_eq!(voxel_at(&app, ABOVE), source());
            assert_eq!(inventory(&mut app), Inventory::default());
        }

        #[test]
        fn undoing_a_creative_placement_refunds_its_item_and_redo_spends_it_again() {
            let mut app = game(false);
            let before = inventory(&mut app);
            place(&mut app);
            let placed = inventory(&mut app);
            assert_eq!(held(&mut app, "t:stone"), 2);
            replay(&mut app, Undo);
            assert!(rejections(&mut app).is_empty());
            assert!(voxel_at(&app, ABOVE).is_air());
            assert_eq!(inventory(&mut app), before);
            replay(&mut app, Redo);
            assert!(rejections(&mut app).is_empty());
            assert_eq!(voxel_at(&app, ABOVE), stone());
            assert_eq!(inventory(&mut app), placed);
        }

        #[test]
        fn a_creative_placement_keeps_its_cost_after_a_switch_to_survival() {
            let mut app = game(false);
            let before = inventory(&mut app);
            place(&mut app);
            survival(&mut app);
            replay(&mut app, Undo);
            assert!(rejections(&mut app).is_empty());
            assert_eq!(inventory(&mut app), before, "refunded as it was paid");
        }

        #[test]
        fn creative_entries_stay_free_after_a_switch_to_survival() {
            let mut app = game(false);
            remove(&mut app);
            place(&mut app);
            assert_eq!(drops(&app), []);
            survival(&mut app);
            set_inventory(&mut app, Inventory::default());
            for action in [Undo, Undo, Redo, Redo] {
                replay(&mut app, action);
                assert!(rejections(&mut app).is_empty(), "{action:?}");
            }
            assert_eq!(voxel_at(&app, GROUND), stone(), "the place filled it back");
            assert_eq!(inventory(&mut app), Inventory::default());
            assert_eq!(drops(&app), []);
        }
    }

    #[test]
    fn the_journal_bounds_recorded_voxels_across_groups_and_drops_empty_ones() {
        use tesserae_core::{Chunk, Voxel};
        let mut store = ChunkStore::default();
        let coord = VoxelCoord::of([0, 0, 0]).chunk;
        store.insert(coord, Chunk::air());
        let solid = |voxel| Occupants {
            solid: voxel,
            fluid: None,
            burning: None,
        };
        let entry = |before: Voxel, after: Voxel| Entry {
            at: [0, 0, 0],
            lifetime: store.lifetime(coord).unwrap(),
            before: solid(before),
            after: solid(after),
            event: EditEvent {
                at: [0.0; 3],
                before,
                after,
                kind: EditKind::Solid { before, after },
            },
            cost: Cost::Free,
        };
        let mut journal = Journal::default();
        journal.record_group(vec![entry(Voxel::AIR, Voxel::AIR); 4]);
        assert!(journal.undo.is_empty(), "a group that changed nothing");
        for _ in 0..RECORDED_LIMIT / 256 + 1 {
            journal.record_group(vec![entry(Voxel::AIR, Voxel::cube(8)); 256]);
        }
        assert_eq!(journal.undo.len(), RECORDED_LIMIT / 256);
    }
}
