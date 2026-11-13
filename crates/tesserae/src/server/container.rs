//! Containers on the Server: each Container Voxel's Stacks, held by position
//! outside the Chunk, and the sessions Players have open on them. Anything but
//! the Player's own Remove or history that takes a Container away spills its
//! Stacks as Dropped items
//! (`docs/adr/0026-usable-voxels-are-declared-kinds-the-server-runs.md`).

use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;
use tesserae_core::audio::Cause;
use tesserae_core::collide::VoxelVolume;
use tesserae_core::entity::DropSpec;
use tesserae_core::inventory::Stack;
use tesserae_core::save::containers::SavedContainer;
use tesserae_core::{MaterialId, Voxel};

use super::ContentTables;
use super::edit::EDIT_REACH_M;
use super::entities::EntityStore;
use super::items::Items;
use super::player::Player;
use super::sound::Emitter;
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
    revision: u64,
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Closed {
    OutOfReach,
    /// The Voxel changed or was removed.
    Changed,
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

    /// The slots of the Container `viewer` has open, to change; a change is
    /// sent to its viewers and saved.
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

    pub fn restore(saved: Vec<SavedContainer>) -> Self {
        Self {
            held: saved.into_iter().map(|c| (c.at, c.slots)).collect(),
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
    (store, tables, world): (Res<ChunkStore>, Res<ContentTables>, Res<WorldSettings>),
    players: Query<&Player>,
    (mut entities, items): (ResMut<EntityStore>, Res<Items>),
    (mut closed, mut audio): (MessageWriter<ContainerClosed>, Emitter),
) {
    let containers = &mut *containers;
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

/// Whether the Voxel `at` is within Edit reach of `eye`, measured to its
/// nearest point.
fn in_reach(eye: Vec3, at: [i32; 3], voxel_size: f32) -> bool {
    let min = Vec3::from_array(at.map(|i| i as f32 * voxel_size));
    eye.distance(eye.clamp(min, min + Vec3::splat(voxel_size))) <= EDIT_REACH_M
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
