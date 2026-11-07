//! Queued Edits landing on the tick: the change, its payment or drop, its
//! journal entry and the relight and remesh it needs.

use super::*;
use tesserae_core::audio::Cause;

pub(in crate::server) fn apply_pending(
    mut pending: ResMut<PendingEdits>,
    (mut history, mut journal): (
        ResMut<crate::server::mods::EditHistory>,
        ResMut<crate::server::undo::Journal>,
    ),
    (mut store, mut meshes): (ResMut<ChunkStore>, ResMut<MeshState>),
    (mut rejected, mut applied_edits, mut heard): (
        MessageWriter<EditRejected>,
        MessageWriter<EditApplied>,
        MessageWriter<crate::server::sound::SoundEvent>,
    ),
    (tables, world, sounds): (
        Res<ContentTables>,
        Res<WorldSettings>,
        Res<crate::server::sound::SoundTable>,
    ),
    (mut entities, items): (
        ResMut<crate::server::entities::EntityStore>,
        Res<crate::server::items::Items>,
    ),
    mut players: Query<(&Player, &mut crate::server::inventory::Inventory)>,
) {
    let Ok((player, mut inventory)) = players.single_mut() else {
        return;
    };
    let body = player.solid_body();
    let mut applied = 0;
    let mut waiting = VecDeque::new();
    while applied < EDITS_PER_TICK {
        let Some(mut p) = pending.0.pop_front() else {
            break;
        };
        let outcome = resolve(&store, &tables, &world, &p.intent, p.reach)
            .and_then(|landing| {
                // Checked before the Voxel changes, so nothing lands unpaid.
                match &p.pay {
                    Some(item) if inventory.count(item) == 0 => Err(RejectReason::NothingHeld),
                    _ => Ok(landing),
                }
            })
            .and_then(|(at, change)| {
                land(
                    &mut store,
                    &mut meshes,
                    &tables,
                    &world,
                    &mut history,
                    body,
                    at,
                    change,
                    p.intent.action,
                )
                .map(|mut entry| {
                    if let Some(item) = &p.pay {
                        inventory
                            .spend(item, 1)
                            .expect("counted before the Voxel changed");
                        entry.cost = Cost::Paid(item.clone());
                    }
                    if let Some(material) = removal(p.intent.action, &entry)
                        && matches!(entry.event.kind, EditKind::Solid { .. })
                    {
                        heard.write_batch(sounds.voxel(
                            Cause::Break,
                            material,
                            entry.at,
                            world.voxel_size,
                            player.eye(),
                        ));
                    }
                    // Spawned here rather than from a message so the Entry
                    // holds the Dropped item an undo takes back.
                    if let Some(material) = removal(p.intent.action, &entry)
                        && let Some(item) = items.dropped_by(material)
                    {
                        let id = crate::server::entities::drop_one(
                            &mut entities,
                            &tables.terrain(&store, world.voxel_size),
                            &items,
                            entry.at,
                            item,
                        );
                        entry.cost = Cost::Dropped(items.0[item].name.clone(), id);
                    }
                    if let Some(kind) = applied_kind(p.intent.action, &entry) {
                        applied_edits.write(EditApplied { at: entry.at, kind });
                    }
                    journal.record(entry)
                })
            });
        match outcome {
            Ok(()) => applied += 1,
            Err(RejectReason::Busy) if p.waited < MAX_DEFERRED_TICKS => {
                p.waited += 1;
                waiting.push_back(p);
            }
            Err(reason) => {
                debug!("edit rejected: {}", reason.message());
                rejected.write(EditRejected { reason });
            }
        }
    }
    // What waited goes ahead of what was not looked at, so order is kept.
    waiting.extend(pending.0.drain(..));
    pending.0 = waiting;
    // Single Edits go first, so a large layout never starves them.
    let PendingEdits(_, layouts) = &mut *pending;
    layout::drain(
        layouts,
        EDITS_PER_TICK - applied,
        &mut history,
        &mut journal,
        &mut store,
        &mut meshes,
        &mut rejected,
        &tables,
        &world,
        body,
        &mut inventory,
    );
}

/// What a Player's own single Place or Remove did, for the Client to show.
fn applied_kind(action: EditAction, entry: &crate::server::undo::Entry) -> Option<AppliedKind> {
    if let Some(material) = removal(action, entry) {
        return Some(AppliedKind::Removed(material));
    }
    match (action, entry.event.kind) {
        (
            EditAction::Place(_) | EditAction::PlaceHeld { .. },
            EditKind::Solid { before, after },
        ) if before.is_air() && !after.is_air() => Some(AppliedKind::Placed(after.material())),
        (EditAction::Place(_) | EditAction::PlaceHeld { .. }, EditKind::SourcePlaced(material)) => {
            Some(AppliedKind::Placed(material))
        }
        _ => None,
    }
}

/// Applies one resolved change and reports it to Mods, returning the history
/// entry for the caller to journal: alone for a single Edit, grouped for a
/// layout.
#[allow(clippy::too_many_arguments)]
pub(super) fn land(
    store: &mut ChunkStore,
    meshes: &mut MeshState,
    tables: &Tables,
    world: &WorldSettings,
    history: &mut crate::server::mods::EditHistory,
    body: Option<Aabb>,
    at: [i32; 3],
    change: Change,
    action: EditAction,
) -> Result<crate::server::undo::Entry, RejectReason> {
    // The incremental relight and the bulk solve must not both write one
    // Chunk, so an edit waits for its Chunk's light to settle.
    if meshes.is_unlit(VoxelCoord::of(at).chunk) {
        return Err(RejectReason::Busy);
    }
    if let (Change::Solid(voxel), Some(body)) = (change, body)
        && !voxel.is_air()
        && overlaps(&PlacedOnly { at, voxel }, tables, world.voxel_size, body)
    {
        return Err(RejectReason::InsidePlayer);
    }
    let before = store.voxel(at).ok_or(RejectReason::NotLoaded)?;
    let was = store
        .occupants(at, &tables.fluids)
        .ok_or(RejectReason::NotLoaded)?;
    match change {
        Change::Solid(voxel) => set_voxel(store, meshes, tables, at, voxel)?,
        Change::Occupants(occupants) => set_occupants(store, meshes, tables, at, occupants)?,
    }
    let after = store.voxel(at).expect("the edited Chunk is held");
    let now = store
        .occupants(at, &tables.fluids)
        .expect("the edited Chunk is held");
    let kind = match (action, change, intent_fluid(&was, &now)) {
        (EditAction::Ignite, ..) => EditKind::Ignited(now.solid.material()),
        (EditAction::Remove | EditAction::RemoveAt(_), ..) if was.burning.is_some() => {
            EditKind::PutOut(now.solid.material())
        }
        (_, Change::Occupants(_), Some(kind)) => kind,
        _ => EditKind::Solid {
            before: was.solid,
            after: now.solid,
        },
    };
    let event = tesserae_mods::EditEvent {
        at: at.map(|i| i as f32 * world.voxel_size),
        kind,
        before,
        after,
    };
    history.record(event);
    Ok(crate::server::undo::Entry {
        at,
        lifetime: store
            .lifetime(VoxelCoord::of(at).chunk)
            .expect("the edited Chunk is held"),
        before: was,
        after: now,
        event,
        cost: Cost::Free,
    })
}

/// The solid or fluid source a Player's single deliberate Remove took out of
/// the World, which is what drops an Item: flowing fluid never does, as no
/// Remove aims at it, and undo, Layouts, the fluid simulation, fire and Mods'
/// Commands do not.
pub(super) fn removal(
    action: EditAction,
    entry: &crate::server::undo::Entry,
) -> Option<MaterialId> {
    match (action, entry.event.kind) {
        (EditAction::Remove | EditAction::RemoveAt(_), EditKind::Solid { before, after })
            if after.is_air() && !before.is_air() =>
        {
            Some(before.material())
        }
        (EditAction::Remove | EditAction::RemoveAt(_), EditKind::SourceRemoved(material)) => {
            Some(material)
        }
        _ => None,
    }
}

/// What a fluid-tool Edit meant, read from what it changed.
pub(super) fn intent_fluid(was: &Occupants, now: &Occupants) -> Option<EditKind> {
    match (was.fluid, now.fluid) {
        (_, Some(f)) if f.source && was.fluid != now.fluid => {
            Some(EditKind::SourcePlaced(f.material))
        }
        (Some(f), None) if f.source => Some(EditKind::SourceRemoved(f.material)),
        _ => None,
    }
}

/// One Voxel of a Mod's Command, relit at once: [`Batch::apply_intent`] in a
/// batch of its own.
#[cfg(test)]
pub(in crate::server) fn apply_intent(
    store: &mut ChunkStore,
    meshes: &mut MeshState,
    tables: &Tables,
    at: [i32; 3],
    intent: Intent,
) -> Result<(), RejectReason> {
    let mut batch = Batch::default();
    let written = batch.apply_intent(store, meshes, tables, at, intent);
    batch.finish(store, meshes, tables);
    written.map(drop)
}

/// Whether a write changed the World, and how far its relight may reach.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::server) enum Written {
    Changed,
    /// Changed the block light the Voxel gives off: its relight floods as far
    /// as that light reaches, a hundred times an ordinary change's.
    Emitter,
    /// It held exactly this already: nothing was journalled, woken or marked.
    Unchanged,
}

/// The block light a Voxel gives off: its solid's and fluid's emission, or a
/// fire's, by the larger.
pub(in crate::server) fn emission(tables: &Tables, o: &Occupants) -> u8 {
    let lit = tables.wet_emission(o.solid, o.fluid.map(|f| f.material));
    if o.burning.is_some() {
        lit.max(tables.burning_emission(o.solid))
    } else {
        lit
    }
}

/// The one World change, shared by the intents above, the Mod drain, the fluid
/// simulation and the `dev` builders, split so a run of writes floods light
/// once: each write lands in storage at once, under every rule an Edit keeps,
/// and only the relight waits for [`Batch::finish`].
///
/// Nothing may read light between a batch's first write and its `finish`; the
/// rules a write is checked by read Occupants, and neither flow nor heat reads
/// light (`docs/adr/0013-fluid-and-fire-live-beside-the-voxel.md`).
#[must_use = "light is stale until the batch is finished"]
#[derive(Default)]
pub(in crate::server) struct Batch {
    /// Every write that moved light, with the light its position held before
    /// the batch.
    relit: Vec<([i32; 3], Light)>,
    /// Each Chunk's shaped units once a write here has counted them, kept
    /// current by every later write: counting means reading the whole Chunk,
    /// and a build of stairs would do it once per stair.
    shaped: HashMap<ChunkCoord, usize>,
}

impl Batch {
    /// One Voxel of a Mod's Command, through the same rules and the same World
    /// change as a player's Edit.
    pub(in crate::server) fn apply_intent(
        &mut self,
        store: &mut ChunkStore,
        meshes: &mut MeshState,
        tables: &Tables,
        at: [i32; 3],
        intent: Intent,
    ) -> Result<Written, RejectReason> {
        if let Intent::Voxel(voxel) = intent {
            return self.set_voxel(store, meshes, tables, at, voxel);
        }
        let here = store
            .occupants(at, &tables.fluids)
            .ok_or(RejectReason::NotLoaded)?;
        let next = match intent {
            Intent::Voxel(_) => unreachable!("placed above"),
            Intent::Source(material) => with_source(here, material, tables)?,
            Intent::RemoveSource => without_source(here)?,
            Intent::Ignite => ignite(here, tesserae_core::fire::is_wet(store, tables, at), tables)?,
            Intent::Extinguish => extinguish(here)?,
        };
        self.set_occupants(store, meshes, tables, at, next)
    }

    /// Replaces what is at one position and marks every Chunk whose mesh that
    /// made stale but for the relight's, which [`Batch::finish`] marks.
    ///
    /// [`Batch::set_voxel`] is this with the solid-layer rule applied first. A
    /// change to the fluid or the fire alone is a change all the same, though
    /// the packed Voxel stays: the mesh, the cover and any Snapshot taken before
    /// it read what is now gone.
    ///
    /// Every change is journalled in the store, which is what wakes the fluid
    /// simulation around it on the next step.
    ///
    /// A mesh is stale in four kinds of Chunk: the edited one; any the relight
    /// wrote to; any across a plane of light the relight wrote to, whose faces
    /// against it that light shades; and the neighbour across a boundary plane
    /// the Voxel sits on, whose faces next to it change whether or not any
    /// light does. Removing a stone Voxel in the dark moves no light and still
    /// leaves the next Chunk a face to grow.
    pub(in crate::server) fn set_occupants(
        &mut self,
        store: &mut ChunkStore,
        meshes: &mut MeshState,
        tables: &Tables,
        at: [i32; 3],
        occupants: Occupants,
    ) -> Result<Written, RejectReason> {
        let place = VoxelCoord::of(at);
        let chunk = store.get(place.chunk).ok_or(RejectReason::NotLoaded)?;

        // Air is air whatever shape or orientation byte it carries.
        let solid = if occupants.solid.is_air() {
            Voxel::AIR
        } else {
            occupants.solid
        };
        let occupants = Occupants { solid, ..occupants };
        tables.vet_occupants(occupants)?;
        // A cube looks the same however it is turned, and a connected Shape
        // ignores its orientation, so the turn is dropped. Left in, two that
        // look identical would be two palette entries and unequal.
        let solid = if solid.shape() == SHAPE_FULL_CUBE || tables.shapes.is_connected(solid) {
            Voxel::new(solid.material(), solid.shape(), 0)
        } else {
            solid
        };
        let (voxel, overlay) = Occupants { solid, ..occupants }.encode(&tables.fluids);

        let before = chunk.get(place.x, place.y, place.z);
        let held = store
            .overlay(place.chunk)
            .expect("the Chunk is held")
            .get(place.x, place.y, place.z);
        if voxel == before && overlay == held {
            return Ok(Written::Unchanged);
        }
        let was = store
            .occupants(at, &tables.fluids)
            .expect("the Chunk is held");
        let (added, removed) = (
            tables.shapes.cap_units(voxel) as usize,
            tables.shapes.cap_units(before) as usize,
        );
        if added > removed {
            let units = self
                .shaped
                .entry(place.chunk)
                .or_insert_with(|| chunk.shaped_units(|v| tables.shapes.cap_units(v)));
            if *units + (added - removed) > MAX_SHAPED_PER_CHUNK {
                return Err(RejectReason::TooManyShaped);
            }
        }
        if let Some(units) = self.shaped.get_mut(&place.chunk) {
            *units = *units + added - removed;
        }

        let seen = (store.optical(at), store.wet(at), store.burning(at));
        store.set_overlay(place, overlay);
        if voxel != before {
            store.set_voxel(place, voxel);
        }
        // Light reads fluid as it reads a cube of its Material, or combined
        // with the solid it stands in, and a fire by its emission, so water
        // flowing in or receding and a fire catching or going out relight
        // though the packed Voxel stays; a surface or reach change alone moves
        // nothing. Nothing has relit since the batch began, so the light here
        // is still what it held before it.
        if (store.optical(at), store.wet(at), store.burning(at)) != seen {
            self.relit.push((at, store.light_at(at)));
        }
        let now = Occupants { solid, ..occupants };
        store.note_changed(
            at,
            voxel != before,
            tesserae_core::fire::alters_exposure(tables, &was, &now),
        );

        meshes.voxels_changed(place.chunk);
        meshes.edited(place.chunk);
        let local = [place.x, place.y, place.z];
        let last = tesserae_core::CHUNK_SIZE - 1;
        for (axis, step) in [
            (0, [NEG_X, POS_X]),
            (1, [NEG_Y, POS_Y]),
            (2, [NEG_Z, POS_Z]),
        ] {
            for (on_boundary, face) in [(local[axis] == 0, step[0]), (local[axis] == last, step[1])]
            {
                if !on_boundary {
                    continue;
                }
                let d = FACE_STEPS[face];
                let neighbour = ChunkCoord::new(
                    place.chunk.x + d[0],
                    place.chunk.y + d[1],
                    place.chunk.z + d[2],
                );
                if store.contains(neighbour) {
                    meshes.edited(neighbour);
                }
            }
        }
        Ok(if emission(tables, &was) != emission(tables, &now) {
            Written::Emitter
        } else {
            Written::Changed
        })
    }

    /// The whole-Voxel Edit a player's click, a Mod's Command and a `dev`
    /// builder make: `voxel` applied to the solid layer by
    /// [`Occupants::place_solid`], then [`Batch::set_occupants`].
    pub(in crate::server) fn set_voxel(
        &mut self,
        store: &mut ChunkStore,
        meshes: &mut MeshState,
        tables: &Tables,
        at: [i32; 3],
        voxel: Voxel,
    ) -> Result<Written, RejectReason> {
        let now = store
            .occupants(at, &tables.fluids)
            .ok_or(RejectReason::NotLoaded)?;
        self.set_occupants(store, meshes, tables, at, now.place_solid(voxel, tables))
    }

    /// Re-floods the light around every write that moved it, once for all of
    /// them, and marks each Chunk that light was written into or across.
    pub(in crate::server) fn finish(
        self,
        store: &mut ChunkStore,
        meshes: &mut MeshState,
        tables: &Tables,
    ) {
        if self.relit.is_empty() {
            return;
        }
        let mut lattice = LightLattice::new(store);
        relight_many(&mut lattice, tables, &self.relit);
        let mut lit = std::mem::take(&mut lattice.touched);
        let mut across = std::mem::take(&mut lattice.across);
        for list in [&mut lit, &mut across] {
            list.sort_unstable_by_key(|c| (c.x, c.y, c.z));
            list.dedup();
        }
        for coord in lit {
            meshes.edited(coord);
        }
        for coord in across {
            if store.contains(coord) {
                meshes.edited(coord);
            }
        }
    }
}

/// One write, relit at once: [`Batch::set_occupants`] in a batch of its own.
pub(in crate::server) fn set_occupants(
    store: &mut ChunkStore,
    meshes: &mut MeshState,
    tables: &Tables,
    at: [i32; 3],
    occupants: Occupants,
) -> Result<(), RejectReason> {
    let mut batch = Batch::default();
    let written = batch.set_occupants(store, meshes, tables, at, occupants);
    batch.finish(store, meshes, tables);
    written.map(drop)
}

pub(super) fn vet_capacity(
    store: &ChunkStore,
    tables: &Tables,
    at: [i32; 3],
    voxel: Voxel,
) -> Result<(), RejectReason> {
    let place = VoxelCoord::of(at);
    let chunk = store.get(place.chunk).ok_or(RejectReason::NotLoaded)?;
    let before = chunk.get(place.x, place.y, place.z);
    let added = tables.shapes.cap_units(voxel);
    let removed = tables.shapes.cap_units(before);
    if added > removed
        && chunk.shaped_units(|v| tables.shapes.cap_units(v)) + (added - removed) as usize
            > MAX_SHAPED_PER_CHUNK
    {
        return Err(RejectReason::TooManyShaped);
    }
    Ok(())
}

/// One whole-Voxel write, relit at once: [`Batch::set_voxel`] in a batch of
/// its own.
pub(in crate::server) fn set_voxel(
    store: &mut ChunkStore,
    meshes: &mut MeshState,
    tables: &Tables,
    at: [i32; 3],
    voxel: Voxel,
) -> Result<(), RejectReason> {
    let mut batch = Batch::default();
    let written = batch.set_voxel(store, meshes, tables, at, voxel);
    batch.finish(store, meshes, tables);
    written.map(drop)
}

/// Air everywhere but one placed Voxel, so an overlap with it alone is asked of
/// the body and terrain it already stands in does not count.
pub(super) struct PlacedOnly {
    pub(super) at: [i32; 3],
    pub(super) voxel: Voxel,
}
impl VoxelVolume for PlacedOnly {
    fn voxel(&self, at: [i32; 3]) -> Option<Voxel> {
        Some(if at == self.at {
            self.voxel
        } else {
            Voxel::AIR
        })
    }
}
