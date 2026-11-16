//! Containers on the Server: each Container Voxel's Stacks, held by position
//! outside the Chunk, and the sessions Players have open on them. Anything but
//! the Player's own Remove or history that takes a Container away spills its
//! Stacks as Dropped items
//! (`docs/adr/0026-usable-voxels-are-declared-kinds-the-server-runs.md`).
//!
//! A chest worldgen stood draws its loot the first time a Container is needed
//! for it: opened, or written over. Its cell is then looted for good, so a
//! Player's chest put back there starts empty.

use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;
use tesserae_core::audio::Cause;
use tesserae_core::collide::VoxelVolume;
use tesserae_core::entity::DropSpec;
use tesserae_core::inventory::Stack;
use tesserae_core::save::containers::SavedContainer;
use tesserae_core::{MaterialId, Voxel, Worldgen};

use super::ContentTables;
use super::entities::EntityStore;
use super::items::Items;
use super::player::Player;
use super::sound::Emitter;
use super::usable::{Closed, in_reach};
use super::world_state::ChunkStore;
use crate::session::SessionApp;
use crate::world::WorldSettings;

#[derive(Resource, Default, Debug)]
pub struct Containers {
    held: BTreeMap<[i32; 3], Vec<Option<Stack>>>,
    sessions: BTreeMap<Entity, Session>,
    /// Containers whose viewers have not been sent their Stacks since they
    /// changed or were opened.
    unsent: BTreeSet<[i32; 3]>,
    /// Cells whose worldgen chest has drawn its loot, or was written over first.
    looted: BTreeSet<[i32; 3]>,
    revision: u64,
}

/// What a worldgen chest's loot is drawn from, where the World has a generator.
#[derive(bevy::ecs::system::SystemParam)]
pub(super) struct LootSource<'w> {
    generator: Option<Res<'w, super::WorldGenerator>>,
    items: Option<Res<'w, Items>>,
}

impl LootSource<'_> {
    pub(super) fn get(&self) -> Option<(&Worldgen, &Items)> {
        Some((&self.generator.as_ref()?.0, self.items.as_ref()?))
    }
}

/// A Player's open Container, and the Voxel it was when opened: any change to
/// it closes the session.
#[derive(Clone, Copy, Debug)]
struct Session {
    at: [i32; 3],
    voxel: Voxel,
}

/// A Container's Stacks whole, sent to its viewers when it opens and whenever
/// they change.
#[derive(Message, Clone, PartialEq, Debug)]
pub struct ContainerShown {
    pub at: [i32; 3],
    pub material: MaterialId,
    pub slots: Vec<Option<Stack>>,
}

/// The Server closed a session; a Player's own close is not answered.
#[derive(Message, Clone, Copy, PartialEq, Eq, Debug)]
pub struct ContainerClosed {
    pub at: [i32; 3],
    pub why: Closed,
}

impl Containers {
    /// Whether the Container at `at` holds any Stack, which refuses the
    /// Player's Remove of it and history that would remove it.
    pub fn holds_stacks(&self, at: [i32; 3]) -> bool {
        self.held
            .get(&at)
            .is_some_and(|slots| slots.iter().any(|s| s.is_some()))
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Puts the loot of the chest worldgen stood at `at` in it, the first time
    /// it is asked while the cell still holds that chest as `voxel`; anything
    /// else there draws nothing.
    pub(super) fn draw_loot(
        &mut self,
        (generator, items): (&Worldgen, &Items),
        at: [i32; 3],
        voxel: Voxel,
        slots: usize,
    ) {
        if self.looted.contains(&at) {
            return;
        }
        let Some(stood) = generator.stood_at(at).filter(|s| s.voxel == voxel) else {
            return;
        };
        self.looted.insert(at);
        self.revision += 1;
        // A Container held from before loot existed keeps what it holds.
        let (Some(loot), false) = (stood.loot, self.held.contains_key(&at)) else {
            return;
        };
        let drawn = loot.draw(slots).into_iter().map(|slot| {
            let mut stack = slot?;
            let item = items.named(&stack.item)?;
            stack.count = stack.count.min(items.0[item].max_stack);
            Some(stack)
        });
        self.held.insert(at, drawn.collect());
    }

    /// Opens `viewer`'s session on the Container `voxel` at `at`, closing any
    /// other it had.
    pub(super) fn open(&mut self, viewer: Entity, at: [i32; 3], voxel: Voxel, slots: usize) {
        let held = self.held.entry(at).or_default();
        if held.len() < slots {
            held.resize(slots, None);
        }
        self.sessions.insert(viewer, Session { at, voxel });
        self.unsent.insert(at);
    }

    pub(super) fn viewing(&self, viewer: Entity) -> bool {
        self.sessions.contains_key(&viewer)
    }

    /// Ends `viewer`'s session, returning where its Container is.
    pub(super) fn close(&mut self, viewer: Entity) -> Option<[i32; 3]> {
        self.sessions.remove(&viewer).map(|s| s.at)
    }

    /// The slots of the Container `viewer` has open.
    pub(super) fn viewed(&self, viewer: Entity) -> Option<&[Option<Stack>]> {
        let at = self.sessions.get(&viewer)?.at;
        self.held.get(&at).map(Vec::as_slice)
    }

    /// The slots of the Container `viewer` has open, to change; taking them
    /// sends them to its viewers and saves them.
    pub(super) fn viewing_mut(&mut self, viewer: Entity) -> Option<&mut Vec<Option<Stack>>> {
        let at = self.sessions.get(&viewer)?.at;
        self.unsent.insert(at);
        self.revision += 1;
        self.held.get_mut(&at)
    }

    pub fn capture(&self) -> Vec<SavedContainer> {
        self.held
            .iter()
            .filter(|(_, slots)| slots.iter().any(|s| s.is_some()))
            .map(|(&at, slots)| SavedContainer {
                at,
                slots: slots.clone(),
            })
            .collect()
    }

    pub fn looted(&self) -> Vec<[i32; 3]> {
        self.looted.iter().copied().collect()
    }

    pub fn restore(saved: Vec<SavedContainer>, looted: Vec<[i32; 3]>) -> Self {
        Self {
            held: saved.into_iter().map(|c| (c.at, c.slots)).collect(),
            looted: looted.into_iter().collect(),
            ..default()
        }
    }
}

pub(super) fn plugin(app: &mut App) {
    app.init_session_resource::<Containers>()
        .add_session_message::<ContainerShown>()
        .add_session_message::<ContainerClosed>()
        .add_session_systems(
            FixedPreUpdate,
            (watch, announce)
                .chain()
                .after(super::edit::apply_pending)
                .after(super::undo::apply)
                .after(super::mods::Drain)
                .after(super::fluids::simulate)
                .after(super::inventory::arrange)
                .before(super::persistence::drive),
        );
}

/// Closes each session whose Voxel changed or whose Player left reach, and
/// spills what a Voxel that is no longer its Container held. Polled rather
/// than hooked into the one World write, so every path that removes a Voxel
/// is covered, at the cost of a lookup per held Container per tick.
#[allow(clippy::too_many_arguments)]
pub(super) fn watch(
    mut containers: ResMut<Containers>,
    (mut store, tables, world): (ResMut<ChunkStore>, Res<ContentTables>, Res<WorldSettings>),
    (players, generator): (Query<&Player>, Option<Res<super::WorldGenerator>>),
    (mut entities, items): (ResMut<EntityStore>, Res<Items>),
    (mut closed, mut audio): (MessageWriter<ContainerClosed>, Emitter),
) {
    let containers = &mut *containers;
    for (at, was) in store.take_left_containers() {
        let (Some(generator), Some(slots)) = (&generator, tables.uses.container(was.material()))
        else {
            continue;
        };
        containers.draw_loot((&generator.0, &items), at, was, slots);
    }
    containers.sessions.retain(|&viewer, session| {
        let why = if store.voxel(session.at) != Some(session.voxel) {
            Closed::Changed
        } else if players
            .get(viewer)
            .is_ok_and(|player| in_reach(player.eye(), session.at, world.voxel_size))
        {
            return true;
        } else {
            audio.emit_at(Cause::ContainerClose, session.at);
            Closed::OutOfReach
        };
        closed.write(ContainerClosed {
            at: session.at,
            why,
        });
        false
    });
    let mut spilled = Vec::new();
    containers.held.retain(|&at, slots| {
        let Some(voxel) = store.voxel(at) else {
            return true;
        };
        let room = tables.uses.container(voxel.material()).unwrap_or(0);
        if slots.len() > room {
            spilled.extend(slots.drain(room..).flatten().map(|stack| (at, stack)));
        }
        room > 0
    });
    let viewed: BTreeSet<_> = containers.sessions.values().map(|s| s.at).collect();
    containers
        .held
        .retain(|at, slots| viewed.contains(at) || slots.iter().any(|s| s.is_some()));
    if spilled.is_empty() {
        return;
    }
    containers.revision += 1;
    let terrain = tables.terrain(&store, world.voxel_size);
    for (at, stack) in spilled {
        let Some(item) = items.named(&stack.item) else {
            continue;
        };
        let centre = at.map(|i| (i as f32 + 0.5) * world.voxel_size);
        entities.spawn_drops_around(
            &terrain,
            centre,
            [DropSpec {
                item: item as u32,
                count: stack.count,
                size_m: items.0[item].size_m,
            }],
        );
    }
}

pub(super) fn announce(
    mut containers: ResMut<Containers>,
    mut shown: MessageWriter<ContainerShown>,
) {
    if containers.unsent.is_empty() {
        return;
    }
    let containers = &mut *containers;
    for at in std::mem::take(&mut containers.unsent) {
        let (Some(session), Some(slots)) = (
            containers.sessions.values().find(|s| s.at == at),
            containers.held.get(&at),
        ) else {
            continue;
        };
        shown.write(ContainerShown {
            at,
            material: session.voxel.material(),
            slots: slots.clone(),
        });
    }
}

#[cfg(test)]
mod tests;
