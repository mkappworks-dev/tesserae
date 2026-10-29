//! The Server's Voxel-edit path: the only way World state changes after
//! generation.
//!
//! The Client has no read access to World state (`docs/adr/0002-client-server-
//! from-day-one.md`), so it cannot pick a Voxel and does not try: it sends the
//! ray through its crosshair, in metres, and the Server finds the Voxel. A
//! position the Client chose would be a claim to re-check; a ray is only an
//! intent.
//!
//! The ray's origin is a claim too, checked against the Server's [`Player`]
//! when the intent arrives (`docs/adr/0008-the-server-owns-the-players-pose.md`).
//! The pose is the Client's report, clamped to a speed the Server allows, so this
//! stops a stale or invented origin and not a Client that flies to where it wants
//! to edit; that is the player controller's.
//!
//! **Intents are queued, not read on the tick.** `server/mod.rs` explains why a
//! message the Client writes can be dropped by a frame that runs no fixed tick,
//! which at 120 fps against a 64 Hz tick is a large fraction of clicks. So
//! [`receive`] runs every frame, before the fixed loop, and moves them into a
//! queue only the Server can see; [`apply_pending`] drains that on the tick. The
//! queue is also the shape a network transport will fill in build-order step 10.

use std::collections::VecDeque;

use bevy::prelude::*;
use tesserae_core::collide::{RayHit, VoxelVolume, raycast_until};
use tesserae_core::mesh::{FACE_STEPS, NEG_X, NEG_Y, NEG_Z, POS_X, POS_Y, POS_Z};
use tesserae_core::{
    Burning, Fluid, MAX_SHAPED_PER_CHUNK, MaterialId, Occupants, PlaceFault, SHAPE_FULL_CUBE,
    Tables, Voxel, relight,
};
use tesserae_mods::{EditKind, Intent};

use super::ContentTables;
use super::player::{self, Player};
use super::streaming::MeshState;
use super::world_state::{ChunkStore, LightLattice, VoxelCoord};
use crate::world::{ChunkCoord, WorldSettings};

/// How far from the Player's Eye an Edit can reach, in **metres**: a reach in
/// Voxels would be calibrated to one Voxel size.
/// `docs/adr/0003-voxel-size-is-per-world.md`.
///
/// Only the part of the ray within this of the Eye is traced, wherever the
/// ray starts, so neither bound below can add to it.
pub const EDIT_REACH_M: f32 = 8.0;

/// How far a third-person camera sits from the Eye at most, in metres: the
/// Client's shoulder offset and longest boom, rounded up. The Server does not
/// know the view, so any ray may start this far off.
pub const CAMERA_OFFSET_MAX_M: f32 = 7.0;

/// How stale an origin may be, in metres, beyond [`CAMERA_OFFSET_MAX_M`]. An
/// honest Client in the same process adds 0; over a wire it is the distance a
/// Player moves while an intent travels, so a third of a second at a sprint. A
/// constant until the controller has a speed to derive it from.
pub const EDIT_ORIGIN_TOLERANCE_M: f32 = 2.0;

/// How far an Edit's ray may start from the Eye, in metres.
pub const EDIT_ORIGIN_BOUND_M: f32 = CAMERA_OFFSET_MAX_M + EDIT_ORIGIN_TOLERANCE_M;

/// Edits applied per tick. Each costs a relight over at most the 15 Voxels light
/// reaches, so this bounds a tick rather than protecting the budget; the mesh
/// each one dirties is paid for by `StreamingBudget::landed_per_tick`.
const EDITS_PER_TICK: usize = 16;

/// Ticks an edit waits for its Chunk's light to be solved before it is dropped.
const MAX_DEFERRED_TICKS: u32 = 64;

/// Intents queued at once. A Server that stalls must not grow without bound.
const MAX_PENDING: usize = 256;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum EditAction {
    /// Remove fire, then solid, then source at the first occupied Voxel.
    Remove,
    /// Complete a deliberate hold only if the ray still strikes this target.
    RemoveAt(AimRemoval),
    /// Put this Voxel in the empty space in front of the face the ray strikes.
    Place(Voxel),
    /// Put a source of this fluid in front of the face the ray strikes. The ray
    /// stops at sources as well as solids and goes through flowing fluid.
    PlaceSource(MaterialId),
    /// Set the solid the ray strikes alight, by the rule a Mod's ignition
    /// meets.
    Ignite,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AimItem {
    Solid(Voxel),
    Fluid(MaterialId),
    Fire,
}
impl AimItem {
    fn action(self) -> EditAction {
        match self {
            Self::Solid(voxel) => EditAction::Place(voxel),
            Self::Fluid(material) => EditAction::PlaceSource(material),
            Self::Fire => EditAction::Ignite,
        }
    }
}

#[derive(Message, Clone, Copy, Debug)]
pub struct AimReport {
    pub seq: u32,
    pub origin: Vec3,
    pub direction: Vec3,
    pub held: Option<AimItem>,
}
#[derive(Message, Clone, Copy, Debug)]
pub struct PickIntent {
    pub seq: u32,
    pub origin: Vec3,
    pub direction: Vec3,
}
#[derive(Message, Clone, Copy, Debug)]
pub struct PickedItem {
    pub seq: u32,
    pub item: Option<AimItem>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AimBounds {
    pub min: Vec3,
    pub max: Vec3,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AimRemoval {
    pub at: [i32; 3],
    pub bounds: AimBounds,
    pub material: MaterialId,
    pub burning: bool,
}
#[derive(Message, Clone, Copy, Debug, Default, PartialEq)]
pub struct AimTarget {
    pub seq: u32,
    pub outline: Option<AimBounds>,
    pub ghost: Option<Vec3>,
    pub removal: Option<AimRemoval>,
    /// The camera sees a target the Eye cannot: nothing above is offered.
    pub blocked: bool,
}
#[derive(Resource, Default)]
struct AimState {
    latest: Option<(AimReport, Reach)>,
}

/// The stretch of an Edit's ray the Server traces, judged against the Eye when
/// the intent arrives.
#[derive(Clone, Copy, Debug)]
struct Reach {
    /// Where the ray passes nearest the Eye, or its origin if it starts past
    /// that point: what lies between a camera and the Player is never aimed at.
    start: Vec3,
    /// How far along the ray from `start` it stays within [`EDIT_REACH_M`] of
    /// the Eye.
    length_m: f32,
    /// Set when `start` is off the Eye, where a camera may see past what the
    /// Eye can.
    eye: Option<Vec3>,
}

/// What the Client wants done: an action, and the ray it wants it done along, in
/// metres. Intent only - the Server decides whether it happens.
#[derive(Message, Clone, Copy, Debug)]
pub struct EditIntent {
    pub action: EditAction,
    pub origin: Vec3,
    pub direction: Vec3,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RejectReason {
    /// The ray struck nothing within [`EDIT_REACH_M`].
    OutOfReach,
    /// The ray starts more than [`EDIT_ORIGIN_BOUND_M`] from the Player.
    OriginTooFar,
    /// The camera sees the target, but something stands between it and the
    /// Eye.
    Blocked,
    NotLoaded,
    /// The place a Voxel would go already holds one.
    Occupied,
    /// A Material the table does not hold: the table is the registry.
    UnknownMaterial,
    UnknownShape,
    BadOrientation,
    /// A Place of air, which is a Break by another name.
    NoMaterial,
    /// The Chunk's shaped Voxels, in stairs, would pass [`MAX_SHAPED_PER_CHUNK`].
    TooManyShaped,
    /// A fluid Material where a solid goes.
    NotSolid,
    /// A fluid whose Material has no fluid definition.
    NotAFluid,
    /// Fluid in a Voxel a full cube leaves no space in.
    FluidExcluded,
    BadSurface,
    /// Burning with nothing solid to burn.
    NoFuel,
    /// A source removal that struck no source.
    NoSource,
    /// Ignition of a Material with no combustion declaration.
    NotFlammable,
    AlreadyBurning,
    /// Ignition of fuel an extinguishing fluid touches.
    Wet,
    /// Extinguishing where nothing burns.
    NotBurning,
    /// Its Chunk's light never settled, or too many edits were queued.
    Busy,
    NoUndo,
    NoRedo,
    HistoryChanged,
    TargetChanged,
}

impl RejectReason {
    /// For the Client to put on screen.
    pub fn message(self) -> &'static str {
        match self {
            RejectReason::OutOfReach => "nothing in reach",
            RejectReason::OriginTooFar => "too far from where the server has you",
            RejectReason::Blocked => "something is in the way",
            RejectReason::NotLoaded => "that part of the world is not loaded",
            RejectReason::Occupied => "something is already there",
            RejectReason::UnknownMaterial => "unknown material",
            RejectReason::UnknownShape => "unknown shape",
            RejectReason::BadOrientation => "invalid orientation",
            RejectReason::NoMaterial => "nothing to place",
            RejectReason::TooManyShaped => "too many shaped blocks in this chunk",
            RejectReason::NotSolid => "a fluid is not placed as a solid",
            RejectReason::NotAFluid => "not a fluid",
            RejectReason::FluidExcluded => "a full cube leaves no room for fluid",
            RejectReason::BadSurface => "invalid fluid surface",
            RejectReason::NoFuel => "nothing solid to burn",
            RejectReason::NoSource => "no source there",
            RejectReason::NotFlammable => "that does not burn",
            RejectReason::AlreadyBurning => "it is already burning",
            RejectReason::Wet => "it is too wet to burn",
            RejectReason::NotBurning => "nothing is burning there",
            RejectReason::Busy => "the server is busy, try again",
            RejectReason::NoUndo => "nothing to undo",
            RejectReason::NoRedo => "nothing to redo",
            RejectReason::HistoryChanged => "that cell changed; cannot undo or redo it",
            RejectReason::TargetChanged => "removal target changed; hold again",
        }
    }
}

impl From<PlaceFault> for RejectReason {
    fn from(fault: PlaceFault) -> Self {
        match fault {
            PlaceFault::Air => RejectReason::NoMaterial,
            PlaceFault::UnknownMaterial => RejectReason::UnknownMaterial,
            PlaceFault::UnknownShape => RejectReason::UnknownShape,
            PlaceFault::BadOrientation => RejectReason::BadOrientation,
            PlaceFault::NotSolid => RejectReason::NotSolid,
            PlaceFault::NotAFluid => RejectReason::NotAFluid,
            PlaceFault::FluidExcluded => RejectReason::FluidExcluded,
            PlaceFault::BadSurface => RejectReason::BadSurface,
            PlaceFault::NoFuel => RejectReason::NoFuel,
            PlaceFault::NotFuel => RejectReason::NotFlammable,
        }
    }
}

/// An intent the Server declined.
#[derive(Message, Clone, Copy, Debug)]
pub struct EditRejected {
    pub reason: RejectReason,
}

struct Pending {
    intent: EditIntent,
    waited: u32,
    reach: Reach,
}

/// Intents received and not yet applied. Server-private: nothing outside this
/// module can push to it, so the Client can only ask by message.
#[derive(Resource, Default)]
pub(in crate::server) struct PendingEdits(VecDeque<Pending>);

impl PendingEdits {
    pub(super) fn clear(&mut self) {
        self.0.clear();
    }
    pub(in crate::server) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<super::undo::Journal>()
        .add_message::<super::undo::HistoryAction>()
        .add_message::<super::undo::HistoryApplied>()
        .init_resource::<AimState>()
        .add_message::<AimReport>()
        .add_message::<AimTarget>()
        .add_message::<PickIntent>()
        .add_message::<PickedItem>()
        .init_resource::<PendingEdits>()
        .init_resource::<super::mods::EditHistory>()
        .add_message::<EditIntent>()
        .add_message::<EditRejected>()
        // Every frame, so a click is never lost to a frame that runs no tick.
        .add_systems(
            RunFixedMainLoop,
            (receive, receive_aim, pick, super::undo::receive)
                .in_set(RunFixedMainLoopSystems::BeforeFixedMainLoop)
                .after(player::PlayerInput),
        )
        // Before `FixedUpdate`, where `streaming::stream` meshes, so the same
        // tick's `publish` sees the edit.
        .add_systems(
            FixedPreUpdate,
            (
                apply_pending.run_if(super::persistence::playing),
                super::undo::apply
                    .run_if(super::persistence::playing)
                    .before(super::mods::Drain),
                answer_aim,
            )
                .chain(),
        );
}

fn pick(
    mut requests: MessageReader<PickIntent>,
    mut answers: MessageWriter<PickedItem>,
    store: Res<ChunkStore>,
    tables: Res<ContentTables>,
    world: Res<WorldSettings>,
    player: Single<&Player>,
) {
    if let Some(request) = requests.read().last() {
        answers.write(PickedItem {
            seq: request.seq,
            item: pick_item(&store, &tables, &world, *request, player.eye()),
        });
    }
}
fn pick_item(
    store: &ChunkStore,
    tables: &Tables,
    world: &WorldSettings,
    request: PickIntent,
    eye: Vec3,
) -> Option<AimItem> {
    let intent = EditIntent {
        origin: request.origin,
        direction: request.direction,
        action: EditAction::Remove,
    };
    let reach = reach_from(&intent, eye).ok()?;
    let (at, _) = resolve(store, tables, world, &intent, reach).ok()?;
    let here = store.occupants(at, &tables.fluids)?;
    if !here.solid.is_air() {
        Some(AimItem::Solid(here.solid))
    } else {
        here.fluid.map(|fluid| AimItem::Fluid(fluid.material))
    }
}

fn receive_aim(
    mut reports: MessageReader<AimReport>,
    mut state: ResMut<AimState>,
    player: Single<&Player>,
) {
    for report in reports.read() {
        if state
            .latest
            .is_some_and(|(last, _)| report.seq.wrapping_sub(last.seq) as i32 <= 0)
        {
            continue;
        }
        let intent = EditIntent {
            origin: report.origin,
            direction: report.direction,
            action: EditAction::Remove,
        };
        if let Ok(reach) = reach_from(&intent, player.eye()) {
            state.latest = Some((*report, reach));
        }
    }
}

fn answer_aim(
    state: Res<AimState>,
    store: Res<ChunkStore>,
    tables: Res<ContentTables>,
    world: Res<WorldSettings>,
    meshes: Res<MeshState>,
    mut targets: MessageWriter<AimTarget>,
) {
    if let Some((report, reach)) = state.latest {
        targets.write(aim_target(&store, &tables, &world, &meshes, report, reach));
    }
}

fn aim_target(
    store: &ChunkStore,
    tables: &Tables,
    world: &WorldSettings,
    meshes: &MeshState,
    report: AimReport,
    reach: Reach,
) -> AimTarget {
    let intent = |action| EditIntent {
        action,
        origin: report.origin,
        direction: report.direction,
    };
    let remove = resolve(store, tables, world, &intent(EditAction::Remove), reach);
    let place = report
        .held
        .map(|held| resolve(store, tables, world, &intent(held.action()), reach));
    let blocked = matches!(remove, Err(RejectReason::Blocked))
        || matches!(place, Some(Err(RejectReason::Blocked)));
    let remove = remove.ok();
    let place = place.and_then(Result::ok);
    let outline_at = if report.held == Some(AimItem::Fire) {
        place.map(|(at, _)| at)
    } else {
        remove.map(|(at, _)| at)
    };
    let removal = remove.and_then(|(at, _)| removal_target(store, tables, world, at));
    let outline = outline_at
        .and_then(|at| removal_target(store, tables, world, at))
        .map(|r| r.bounds);
    let ghost = place
        .filter(|_| report.held != Some(AimItem::Fire))
        .and_then(|(at, change)| {
            if meshes.is_unlit(VoxelCoord::of(at).chunk) {
                return None;
            }
            let here = store.occupants(at, &tables.fluids)?;
            let next = match change {
                Change::Solid(voxel) => here.place_solid(voxel, tables),
                Change::Occupants(next) => next,
            };
            tables.vet_occupants(next).ok()?;
            vet_capacity(store, tables, at, next.solid).ok()?;
            Some(Vec3::from_array(at.map(|i| i as f32 * world.voxel_size)))
        });
    AimTarget {
        seq: report.seq,
        removal,
        outline,
        ghost,
        blocked,
    }
}

fn removal_target(
    store: &ChunkStore,
    tables: &Tables,
    world: &WorldSettings,
    at: [i32; 3],
) -> Option<AimRemoval> {
    let here = store.occupants(at, &tables.fluids)?;
    let voxel = if here.solid.is_air() {
        Voxel::cube(here.fluid?.material)
    } else {
        here.solid
    };
    let around = FACE_STEPS.map(|d| {
        store
            .voxel([at[0] + d[0], at[1] + d[1], at[2] + d[2]])
            .unwrap_or(Voxel::AIR)
    });
    let connections = tables.shapes.connections(voxel, around);
    let boxes = tables.shapes.selection(voxel, connections);
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for b in boxes {
        min = min.min(Vec3::from_array(b.min.map(f32::from)));
        max = max.max(Vec3::from_array(b.max.map(f32::from)));
    }
    if boxes.is_empty() {
        return None;
    }
    let origin = Vec3::from_array(at.map(|i| i as f32 * world.voxel_size));
    Some(AimRemoval {
        at,
        material: voxel.material(),
        burning: here.burning.is_some(),
        bounds: AimBounds {
            min: origin + min * (world.voxel_size / 16.0),
            max: origin + max * (world.voxel_size / 16.0),
        },
    })
}

pub(in crate::server) fn receive(
    mut intents: MessageReader<EditIntent>,
    mut pending: ResMut<PendingEdits>,
    mut rejected: MessageWriter<EditRejected>,
    player: Single<&Player>,
) {
    for intent in intents.read() {
        // Here and not in `apply_pending`: an intent can wait a second there, by
        // when the Player has moved on from where the Client meant.
        let reach = match reach_from(intent, player.eye()) {
            Ok(reach) => reach,
            Err(reason) => {
                rejected.write(EditRejected { reason });
                continue;
            }
        };
        if pending.0.len() >= MAX_PENDING {
            rejected.write(EditRejected {
                reason: RejectReason::Busy,
            });
            continue;
        }
        pending.0.push_back(Pending {
            intent: *intent,
            waited: 0,
            reach,
        });
    }
}

/// The part of the intent's ray within [`EDIT_REACH_M`] of the Eye, if its
/// origin is near enough to be a camera.
fn reach_from(intent: &EditIntent, eye: Vec3) -> Result<Reach, RejectReason> {
    if !intent.origin.is_finite() || !intent.direction.is_finite() {
        return Err(RejectReason::OutOfReach);
    }
    let off = intent.origin - eye;
    if off.length() > EDIT_ORIGIN_BOUND_M {
        return Err(RejectReason::OriginTooFar);
    }
    let direction = intent.direction.normalize_or_zero();
    // The ray meets the sphere of reach about the Eye where
    // |off + t * direction| = EDIT_REACH_M.
    let b = off.dot(direction);
    let discriminant = b * b - (off.length_squared() - EDIT_REACH_M * EDIT_REACH_M);
    if direction == Vec3::ZERO || discriminant < 0.0 {
        return Err(RejectReason::OutOfReach);
    }
    let enter = (-b).max(0.0);
    let leave = -b + discriminant.sqrt();
    if leave <= enter {
        return Err(RejectReason::OutOfReach);
    }
    let start = intent.origin + direction * enter;
    Ok(Reach {
        start,
        length_m: leave - enter,
        eye: (start.distance(eye) > 1e-3).then_some(eye),
    })
}

/// Whether the Eye sees the point the ray struck, through nothing solid but
/// the Voxel struck.
fn seen_from_eye(
    store: &ChunkStore,
    tables: &Tables,
    world: &WorldSettings,
    eye: Vec3,
    struck: Vec3,
    hit: RayHit,
) -> bool {
    // Nudged inside the struck box, so a sight line grazing a neighbour's
    // edge on the way to a face does not count as that neighbour.
    let inside =
        struck - Vec3::from(FACE_STEPS[hit.face].map(|c| c as f32)) * world.voxel_size * 1e-3;
    let sight = inside - eye;
    raycast_until(
        store,
        tables,
        world.voxel_size,
        eye.to_array(),
        sight.to_array(),
        sight.length(),
        |v| tables.materials.solid(v),
    )
    .is_none_or(|seen| seen.at == hit.at)
}

pub(in crate::server) fn apply_pending(
    mut pending: ResMut<PendingEdits>,
    (mut history, mut journal): (
        ResMut<super::mods::EditHistory>,
        ResMut<super::undo::Journal>,
    ),
    mut store: ResMut<ChunkStore>,
    mut meshes: ResMut<MeshState>,
    mut rejected: MessageWriter<EditRejected>,
    (tables, world): (Res<ContentTables>, Res<WorldSettings>),
) {
    let mut applied = 0;
    let mut waiting = VecDeque::new();
    while applied < EDITS_PER_TICK {
        let Some(mut p) = pending.0.pop_front() else {
            break;
        };
        let outcome =
            resolve(&store, &tables, &world, &p.intent, p.reach).and_then(|(at, change)| {
                // The incremental relight and the bulk solve must not both write one
                // Chunk, so an edit waits for its Chunk's light to settle.
                if meshes.is_unlit(VoxelCoord::of(at).chunk) {
                    return Err(RejectReason::Busy);
                }
                let before = store.voxel(at).ok_or(RejectReason::NotLoaded)?;
                let was = store
                    .occupants(at, &tables.fluids)
                    .ok_or(RejectReason::NotLoaded)?;
                match change {
                    Change::Solid(voxel) => set_voxel(&mut store, &mut meshes, &tables, at, voxel)?,
                    Change::Occupants(occupants) => {
                        set_occupants(&mut store, &mut meshes, &tables, at, occupants)?
                    }
                }
                let after = store.voxel(at).expect("the edited Chunk is held");
                let now = store
                    .occupants(at, &tables.fluids)
                    .expect("the edited Chunk is held");
                let kind = match (p.intent.action, change, intent_fluid(&was, &now)) {
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
                journal.record(super::undo::Entry {
                    at,
                    lifetime: store
                        .lifetime(VoxelCoord::of(at).chunk)
                        .expect("the edited Chunk is held"),
                    before: was,
                    after: now,
                    event,
                });
                Ok(())
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
}

/// What a fluid-tool Edit meant, read from what it changed.
fn intent_fluid(was: &Occupants, now: &Occupants) -> Option<EditKind> {
    match (was.fluid, now.fluid) {
        (_, Some(f)) if f.source && was.fluid != now.fluid => {
            Some(EditKind::SourcePlaced(f.material))
        }
        (Some(f), None) if f.source => Some(EditKind::SourceRemoved(f.material)),
        _ => None,
    }
}

/// What an intent does where it lands.
#[derive(Clone, Copy)]
enum Change {
    /// A whole-Voxel placement, through the solid-layer rule.
    Solid(Voxel),
    /// Exactly these occupants: a source placed or taken away, or a fire lit
    /// or put out.
    Occupants(Occupants),
}

/// Where an intent lands, and what goes there.
fn resolve(
    store: &ChunkStore,
    tables: &Tables,
    world: &WorldSettings,
    intent: &EditIntent,
    reach: Reach,
) -> Result<([i32; 3], Change), RejectReason> {
    if !intent.origin.is_finite() || !intent.direction.is_finite() {
        return Err(RejectReason::OutOfReach);
    }
    let fluid_tool = matches!(
        intent.action,
        EditAction::PlaceSource(_) | EditAction::Remove | EditAction::RemoveAt(_)
    );
    if let EditAction::PlaceSource(material) = intent.action
        && !tables.fluids.contains(material)
    {
        return Err(RejectReason::NotAFluid);
    }
    // A packed fluid Voxel is a source; flowing fluid is packed air.
    let stops = |v: Voxel| {
        tables.materials.solid(v) || (fluid_tool && tables.fluids.contains(v.material()))
    };
    let hit = raycast_until(
        store,
        tables,
        world.voxel_size,
        reach.start.to_array(),
        intent.direction.to_array(),
        reach.length_m,
        stops,
    )
    .ok_or(RejectReason::OutOfReach)?;
    if let Some(eye) = reach.eye {
        let struck = reach.start + intent.direction.normalize() * hit.distance_m;
        if !seen_from_eye(store, tables, world, eye, struck, hit) {
            return Err(RejectReason::Blocked);
        }
    }
    if let EditAction::RemoveAt(expected) = intent.action
        && removal_target(store, tables, world, hit.at) != Some(expected)
    {
        return Err(RejectReason::TargetChanged);
    }
    let struck = || {
        store
            .occupants(hit.at, &tables.fluids)
            .ok_or(RejectReason::NotLoaded)
    };
    let step = FACE_STEPS[hit.face];
    let in_front = [
        hit.at[0] + step[0],
        hit.at[1] + step[1],
        hit.at[2] + step[2],
    ];

    match intent.action {
        EditAction::Remove | EditAction::RemoveAt(_) => {
            let here = struck()?;
            if here.burning.is_some() {
                Ok((hit.at, Change::Occupants(extinguish(here)?)))
            } else if !here.solid.is_air() {
                Ok((hit.at, Change::Solid(Voxel::AIR)))
            } else {
                Ok((hit.at, Change::Occupants(without_source(here)?)))
            }
        }
        EditAction::Place(voxel) => {
            tables.vet_placed(voxel)?;
            match store.voxel(in_front) {
                None => Err(RejectReason::NotLoaded),
                // Water is not solid, so a Voxel placed into it replaces it.
                Some(there) if tables.materials.solid(there) => Err(RejectReason::Occupied),
                Some(_) => Ok((in_front, Change::Solid(voxel))),
            }
        }
        EditAction::PlaceSource(material) => {
            // A Shape with room for fluid takes the source itself, keeping its
            // solid; anything else gets it in front of the face struck.
            let struck = struck()?;
            let into = if !struck.solid.is_air()
                && tables.shapes.void_slots(struck.solid) > 0
                && tables.holds(struck.solid, material)
            {
                hit.at
            } else {
                in_front
            };
            let there = store
                .occupants(into, &tables.fluids)
                .ok_or(RejectReason::NotLoaded)?;
            if into == in_front && !there.solid.is_air() {
                return Err(RejectReason::Occupied);
            }
            Ok((
                into,
                Change::Occupants(with_source(there, material, tables)?),
            ))
        }
        EditAction::Ignite => {
            let wet = tesserae_core::fire::is_wet(store, tables, hit.at);
            Ok((hit.at, Change::Occupants(ignite(struck()?, wet, tables)?)))
        }
    }
}

/// What a source of `material` placed here leaves, or why there is no room:
/// the rule the fluid tool and a Mod's source Commands share. Flow of the same
/// fluid becomes the source; another fluid, or any source, stays.
pub(in crate::server) fn with_source(
    here: Occupants,
    material: MaterialId,
    tables: &Tables,
) -> Result<Occupants, RejectReason> {
    if !tables.fluids.contains(material) {
        return Err(RejectReason::NotAFluid);
    }
    let room = here.solid.is_air()
        || (tables.shapes.void_slots(here.solid) > 0 && tables.holds(here.solid, material));
    if !room {
        return Err(RejectReason::FluidExcluded);
    }
    if here
        .fluid
        .is_some_and(|f| f.source || f.material != material)
    {
        return Err(RejectReason::Occupied);
    }
    Ok(Occupants {
        fluid: Some(Fluid::source(material)),
        ..here
    })
}

/// A source taken away, keeping the solid it shares the Voxel with.
pub(in crate::server) fn without_source(here: Occupants) -> Result<Occupants, RejectReason> {
    if !here.fluid.is_some_and(|f| f.source) {
        return Err(RejectReason::NoSource);
    }
    Ok(Occupants {
        fluid: None,
        ..here
    })
}

/// Declared fuel set alight with its whole fuel, unless it is `wet`: an
/// extinguishing fluid touches it (`tesserae_core::fire::is_wet`). From there
/// it burns as fire caught from heat does.
fn ignite(here: Occupants, wet: bool, tables: &Tables) -> Result<Occupants, RejectReason> {
    if here.solid.is_air() {
        return Err(RejectReason::NoFuel);
    }
    let fuel = tables
        .combustion
        .get(here.solid.material())
        .ok_or(RejectReason::NotFlammable)?;
    if here.burning.is_some() {
        return Err(RejectReason::AlreadyBurning);
    }
    if wet {
        return Err(RejectReason::Wet);
    }
    Ok(Occupants {
        burning: Some(Burning {
            fuel: fuel.fuel_steps(),
        }),
        ..here
    })
}

fn extinguish(here: Occupants) -> Result<Occupants, RejectReason> {
    if here.burning.is_none() {
        return Err(RejectReason::NotBurning);
    }
    Ok(Occupants {
        burning: None,
        ..here
    })
}

/// One Voxel of a Mod's Command, through the same rules and the same World
/// change as a player's Edit.
pub(in crate::server) fn apply_intent(
    store: &mut ChunkStore,
    meshes: &mut MeshState,
    tables: &Tables,
    at: [i32; 3],
    intent: Intent,
) -> Result<(), RejectReason> {
    if let Intent::Voxel(voxel) = intent {
        return set_voxel(store, meshes, tables, at, voxel);
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
    set_occupants(store, meshes, tables, at, next)
}

/// Replaces what is at one position, re-floods the light around it, and marks
/// every Chunk whose mesh that made stale.
///
/// The one World change, shared by the intents above, the Mod drain and the
/// `dev` builders: [`set_voxel`] is this with the solid-layer rule applied
/// first. A change to the fluid or the fire alone is a change all the same,
/// though the packed Voxel stays: the mesh, the cover and any Snapshot taken
/// before it read what is now gone.
///
/// Every change is journalled in the store, which is what wakes the fluid
/// simulation around it on the next step.
///
/// A mesh is stale in four kinds of Chunk: the edited one; any the relight
/// wrote to; any across a plane of light the relight wrote to, whose faces
/// against it that light shades; and the neighbour across a boundary plane the
/// Voxel sits on, whose faces next to it change whether or not any light does.
/// Removing a stone Voxel in the dark moves no light and still leaves the next
/// Chunk a face to grow.
pub(in crate::server) fn set_occupants(
    store: &mut ChunkStore,
    meshes: &mut MeshState,
    tables: &Tables,
    at: [i32; 3],
    occupants: Occupants,
) -> Result<(), RejectReason> {
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
    // A cube looks the same however it is turned, and a connected Shape ignores
    // its orientation, so the turn is dropped. Left in, two that look identical
    // would be two palette entries and unequal.
    let solid = if solid.shape() == SHAPE_FULL_CUBE || tables.shapes.is_connected(solid) {
        Voxel::new(solid.material(), solid.shape(), 0)
    } else {
        solid
    };
    let (voxel, overlay) = Occupants { solid, ..occupants }.encode(&tables.fluids);

    let before = chunk.get(place.x, place.y, place.z);
    let was = store
        .occupants(at, &tables.fluids)
        .expect("the Chunk is held");
    vet_capacity(store, tables, at, voxel)?;

    let seen = (store.optical(at), store.wet(at), store.burning(at));
    store.set_overlay(place, overlay);
    if voxel != before {
        store.set_voxel(place, voxel);
    }
    // Light reads fluid as it reads a cube of its Material, or combined with
    // the solid it stands in, and a fire by its emission, so water flowing in
    // or receding and a fire catching or going out relight though the packed
    // Voxel stays; a surface or reach change alone moves nothing.
    if (store.optical(at), store.wet(at), store.burning(at)) != seen {
        let was = store.light_at(at);
        let mut lattice = LightLattice::new(store);
        relight(&mut lattice, tables, at, was);
        let lit = std::mem::take(&mut lattice.touched);
        let across = std::mem::take(&mut lattice.across);
        for coord in lit {
            meshes.edited(coord);
        }
        for coord in across {
            if store.contains(coord) {
                meshes.edited(coord);
            }
        }
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
        for (on_boundary, face) in [(local[axis] == 0, step[0]), (local[axis] == last, step[1])] {
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
    Ok(())
}

fn vet_capacity(
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

/// The whole-Voxel Edit a player's click, a Mod's Command and a `dev` builder
/// make: `voxel` applied to the solid layer by [`Occupants::place_solid`], then
/// [`set_occupants`].
pub(in crate::server) fn set_voxel(
    store: &mut ChunkStore,
    meshes: &mut MeshState,
    tables: &Tables,
    at: [i32; 3],
    voxel: Voxel,
) -> Result<(), RejectReason> {
    let now = store
        .occupants(at, &tables.fluids)
        .ok_or(RejectReason::NotLoaded)?;
    set_occupants(store, meshes, tables, at, now.place_solid(voxel, tables))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::time::TimeUpdateStrategy;
    use std::time::Duration;
    use tesserae_core::mesh::POS_X;
    use tesserae_core::shape::examples;
    use tesserae_core::{
        Burning, CHUNK_SIZE, Chunk, ChunkLight, Fluid, LitChunk, Occupants, Orientation, Overlay,
        SHAPE_POST, SHAPE_STAIR, mesh_chunk,
    };

    use std::sync::{Arc, LazyLock};
    use tesserae_core::testgen::WATER_FLUID;

    use super::super::player::Player;
    use crate::server::world_state::SpawnPoint;

    /// The Server's default tables with the fixture water: those of `app_with`,
    /// and what a test reading water outside an App holds.
    static WATERED: LazyLock<(Tables, MaterialId)> = LazyLock::new(|| {
        let tables = crate::server::fixture_tables();
        let water = tesserae_core::testgen::fixtures().water;
        (tables, water)
    });

    fn watered() -> Tables {
        WATERED.0.clone()
    }

    fn water() -> MaterialId {
        WATERED.1
    }

    fn stone() -> Voxel {
        Voxel::cube(tesserae_core::testgen::fixtures().ground.rock)
    }

    fn stair() -> Voxel {
        Voxel::shaped(
            tesserae_core::testgen::fixtures().ground.rock,
            SHAPE_STAIR,
            Orientation::IDENTITY,
        )
    }

    /// Chunk (0, 0, 0) with stone filling y = 0..8, so the surface is at 8 m, and
    /// air above it.
    fn ground() -> Chunk {
        let mut chunk = Chunk::air();
        for y in 0..8 {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    chunk.set(x, y, z, stone());
                }
            }
        }
        chunk
    }

    fn app_with(chunks: &[(ChunkCoord, Chunk)]) -> App {
        app_with_tables(chunks, watered())
    }

    fn app_with_tables(chunks: &[(ChunkCoord, Chunk)], tables: Tables) -> App {
        let mut app = App::new();
        app.insert_resource(WorldSettings::default())
            .insert_resource(ContentTables(Arc::new(tables)))
            .init_resource::<ChunkStore>()
            .init_resource::<MeshState>()
            .add_plugins(plugin);
        app.world_mut().spawn(Player::at(DOWN_EYE));
        let mut store = app.world_mut().resource_mut::<ChunkStore>();
        for (coord, chunk) in chunks {
            store.insert(*coord, chunk.clone());
        }
        app
    }

    fn origin() -> ChunkCoord {
        ChunkCoord::new(0, 0, 0)
    }

    const DOWN_EYE: Vec3 = Vec3::new(5.5, 12.0, 5.5);

    /// A click straight down at the ground over (5, 5), from 4 m up.
    fn down(action: EditAction) -> EditIntent {
        EditIntent {
            action,
            origin: DOWN_EYE,
            direction: Vec3::NEG_Y,
        }
    }

    fn voxel_at(app: &App, at: [i32; 3]) -> Voxel {
        app.world().resource::<ChunkStore>().voxel(at).unwrap()
    }

    fn rejections(app: &mut App) -> Vec<RejectReason> {
        app.world_mut()
            .resource_mut::<Messages<EditRejected>>()
            .drain()
            .map(|m| m.reason)
            .collect()
    }

    /// Delivers one intent as if the Player stood at its origin, so the tests
    /// that exercise `resolve` with arbitrary origins are not also testing the
    /// origin check; `send_from_afar` is the one that does.
    fn send(app: &mut App, intent: EditIntent) {
        if intent.origin.is_finite() {
            set_eye(app, intent.origin);
        }
        send_from_afar(app, intent);
    }

    fn set_eye(app: &mut App, eye: Vec3) {
        let mut players = app.world_mut().query::<&mut Player>();
        players.single_mut(app.world_mut()).unwrap().set_eye(eye);
    }

    /// Delivers one intent, then empties the buffer. `run_system_once` builds a
    /// fresh reader every call, so without this each delivery would read every
    /// earlier message again; the real system keeps its reader between frames.
    fn send_from_afar(app: &mut App, intent: EditIntent) {
        app.world_mut().write_message(intent);
        app.world_mut().run_system_once(receive).unwrap();
        app.world_mut()
            .resource_mut::<Messages<EditIntent>>()
            .clear();
    }

    fn tick(app: &mut App) {
        app.world_mut().run_system_once(apply_pending).unwrap();
    }

    #[test]
    fn breaking_removes_the_voxel_the_ray_strikes_and_placing_fills_the_cell_in_front() {
        let mut app = app_with(&[(origin(), ground())]);
        assert_eq!(voxel_at(&app, [5, 7, 5]), stone());

        send(&mut app, down(EditAction::Remove));
        tick(&mut app);
        assert!(voxel_at(&app, [5, 7, 5]).is_air());

        // The surface is now at 7 m; placing puts the Voxel in the hole's air
        // just above the floor the ray strikes next.
        send(&mut app, down(EditAction::Place(stair())));
        tick(&mut app);
        assert_eq!(voxel_at(&app, [5, 7, 5]), stair());
        assert!(rejections(&mut app).is_empty());
    }

    /// `ground` with a layer of water on it at y = 8.
    fn pool() -> Chunk {
        let mut chunk = ground();
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                chunk.set(x, 8, z, Voxel::cube(water()));
            }
        }
        chunk
    }

    #[test]
    fn remove_takes_the_surface_source_and_place_replaces_water() {
        let mut app = app_with(&[(origin(), pool())]);
        send(&mut app, down(EditAction::Remove));
        tick(&mut app);
        assert_eq!(voxel_at(&app, [5, 7, 5]), stone(), "the bed stays");
        assert!(
            voxel_at(&app, [5, 8, 5]).is_air(),
            "the surface source goes"
        );

        let mut app = app_with(&[(origin(), pool())]);
        send(&mut app, down(EditAction::Place(stone())));
        tick(&mut app);
        assert_eq!(
            voxel_at(&app, [5, 8, 5]),
            stone(),
            "the water cell is filled"
        );
        assert!(rejections(&mut app).is_empty());
    }

    #[test]
    fn a_cube_is_stored_the_same_however_it_was_turned() {
        let mut store = ChunkStore::default();
        store.insert(origin(), Chunk::air());
        let mut meshes = MeshState::default();
        let tables = crate::server::fixture_tables();
        set_voxel(
            &mut store,
            &mut meshes,
            &tables,
            [0, 0, 0],
            Voxel::new(tesserae_core::testgen::fixtures().ground.rock, 0, 13),
        )
        .unwrap();
        assert_eq!(store.voxel([0, 0, 0]), Some(stone()));
        // The turn is dropped, not validated away: a bad one on a cube is still bad.
        assert_eq!(
            set_voxel(
                &mut store,
                &mut meshes,
                &tables,
                [1, 0, 0],
                Voxel::new(tesserae_core::testgen::fixtures().ground.rock, 0, 99)
            ),
            Err(RejectReason::BadOrientation)
        );
    }

    #[test]
    fn retired_and_unknown_materials_are_rejected_and_registered_materials_are_placed() {
        let mut store = ChunkStore::default();
        store.insert(origin(), Chunk::air());
        let mut meshes = MeshState::default();
        let tables = crate::server::fixture_tables();
        for unknown in std::iter::once(999).chain(tesserae_core::material::RETIRED_IDS) {
            assert_eq!(
                set_voxel(
                    &mut store,
                    &mut meshes,
                    &tables,
                    [0, 0, 0],
                    Voxel::cube(unknown)
                ),
                Err(RejectReason::UnknownMaterial)
            );
        }
        assert!(store.voxel([0, 0, 0]).unwrap().is_air(), "nothing changed");
        let known = (1..tables.materials.len() as u16).filter(|&m| tables.materials.contains(m));
        for (i, material) in known.enumerate() {
            let at = [i as i32, 1, 0];
            set_voxel(&mut store, &mut meshes, &tables, at, Voxel::cube(material))
                .unwrap_or_else(|r| panic!("Material {material}: {r:?}"));
            assert_eq!(store.voxel(at), Some(Voxel::cube(material)));
        }
    }

    #[test]
    fn an_edit_re_floods_the_light_around_it() {
        let mut app = app_with(&[(origin(), ground())]);
        send(
            &mut app,
            down(EditAction::Place(Voxel::cube(
                tesserae_core::testgen::fixtures().lantern,
            ))),
        );
        tick(&mut app);
        let store = app.world().resource::<ChunkStore>();
        assert_eq!(store.light_at([5, 8, 5]).block(), 15);
        assert_eq!(store.light_at([6, 8, 5]).block(), 14);
    }

    #[test]
    fn an_edit_on_a_chunk_boundary_re_meshes_the_neighbour_even_when_no_light_moves() {
        let solid = Chunk::filled(stone());
        let mut app = app_with(&[
            (ChunkCoord::new(0, 0, 0), solid.clone()),
            (ChunkCoord::new(1, 0, 0), solid.clone()),
            (ChunkCoord::new(0, 1, 0), solid.clone()),
        ]);
        let last = CHUNK_SIZE as i32 - 1;

        // Opening a cell in the dark: no light anywhere changes.
        let mut store = app.world_mut().resource_mut::<ChunkStore>();
        let mut meshes = MeshState::default();
        set_voxel(
            &mut store,
            &mut meshes,
            &crate::server::fixture_tables(),
            [last, 4, 4],
            Voxel::AIR,
        )
        .unwrap();

        assert!(meshes.is_dirty(ChunkCoord::new(0, 0, 0)));
        assert!(
            meshes.is_dirty(ChunkCoord::new(1, 0, 0)),
            "the face next door has to grow"
        );
        assert!(
            !meshes.is_dirty(ChunkCoord::new(0, 1, 0)),
            "a Chunk the Voxel does not touch is left alone"
        );
    }

    fn fence_table() -> Tables {
        crate::server::fixture_tables()
    }

    fn fence() -> Voxel {
        Voxel::new(
            tesserae_core::testgen::fixtures().ground.rock,
            examples::FENCE_ID,
            0,
        )
    }

    /// How many quads `coord` would mesh to, with its neighbours as the store holds
    /// them. Unlit: the count does not depend on light.
    fn quads_of(app: &App, table: &Tables, coord: ChunkCoord) -> usize {
        static DARK: ChunkLight = ChunkLight::dark();
        let store = app.world().resource::<ChunkStore>();
        let at = |c: ChunkCoord| store.get(c).map(|chunk| LitChunk::new(chunk, &DARK));
        let neighbors = [
            ChunkCoord::new(coord.x - 1, coord.y, coord.z),
            ChunkCoord::new(coord.x + 1, coord.y, coord.z),
            ChunkCoord::new(coord.x, coord.y - 1, coord.z),
            ChunkCoord::new(coord.x, coord.y + 1, coord.z),
            ChunkCoord::new(coord.x, coord.y, coord.z - 1),
            ChunkCoord::new(coord.x, coord.y, coord.z + 1),
        ]
        .map(at);
        mesh_chunk(at(coord).expect("loaded"), neighbors, table).quad_count()
    }

    fn edit_at(app: &mut App, table: &Tables, at: [i32; 3], voxel: Voxel) -> MeshState {
        let mut store = app.world_mut().resource_mut::<ChunkStore>();
        let mut meshes = MeshState::default();
        set_voxel(&mut store, &mut meshes, table, at, voxel).unwrap();
        meshes
    }

    #[test]
    fn a_fence_placed_across_a_chunk_boundary_grows_an_arm_in_the_chunk_next_door() {
        let table = fence_table();
        let (a, b) = (ChunkCoord::new(0, 0, 0), ChunkCoord::new(1, 0, 0));
        let mut app = app_with(&[(a, Chunk::air()), (b, Chunk::air())]);
        let last = CHUNK_SIZE as i32 - 1;

        edit_at(&mut app, &table, [last, 4, 4], fence());
        let alone = quads_of(&app, &table, a);

        // The new fence is in `b`, and the arm that grows is on `a`'s: `a` has to
        // be told, though nothing in it changed.
        let meshes = edit_at(&mut app, &table, [last + 1, 4, 4], fence());
        assert!(meshes.is_dirty(a), "the neighbour's arm has to grow");
        assert!(quads_of(&app, &table, a) > alone, "and it does");

        // And breaking it takes the arm away again.
        let meshes = edit_at(&mut app, &table, [last + 1, 4, 4], Voxel::AIR);
        assert!(meshes.is_dirty(a));
        assert_eq!(quads_of(&app, &table, a), alone);
    }

    #[test]
    fn a_fence_grows_an_arm_towards_a_cube_placed_beside_it() {
        let table = fence_table();
        let mut app = app_with(&[(origin(), Chunk::air())]);
        edit_at(&mut app, &table, [5, 4, 5], fence());
        let alone = quads_of(&app, &table, origin());
        edit_at(&mut app, &table, [6, 4, 5], stone());
        // The cube's own faces, and then the arm.
        assert!(quads_of(&app, &table, origin()) > alone + 5);
    }

    #[test]
    fn a_fence_counts_for_as_many_stairs_as_it_costs_to_mesh() {
        let table = fence_table();
        let units = table.shapes.cap_units(fence()) as usize;
        assert!(units > 1);
        let mut full = ground();
        let fences = MAX_SHAPED_PER_CHUNK / units;
        for i in 0..fences {
            full.set(i % CHUNK_SIZE, 8 + i / CHUNK_SIZE, 0, fence());
        }
        let mut store = ChunkStore::default();
        store.insert(origin(), full);
        let mut meshes = MeshState::default();
        let mut put = |at, v| set_voxel(&mut store, &mut meshes, &table, at, v);

        // No room for another fence, though far fewer than the cap's worth of
        // Voxels are shaped.
        assert_eq!(put([0, 20, 5], fence()), Err(RejectReason::TooManyShaped));
        // What is left over fits stairs, which cost one each, and no more.
        let left = MAX_SHAPED_PER_CHUNK - fences * units;
        for i in 0..left {
            assert_eq!(put([i as i32, 20, 6], stair()), Ok(()), "stair {i}");
        }
        assert_eq!(put([0, 20, 7], stair()), Err(RejectReason::TooManyShaped));
        // A fence for a fence is net zero, and breaking one makes room for one.
        assert_eq!(put([0, 8, 0], fence()), Ok(()));
        assert_eq!(put([0, 8, 0], Voxel::AIR), Ok(()));
        assert_eq!(put([0, 20, 5], fence()), Ok(()));
    }

    #[test]
    fn a_connected_shape_is_stored_without_the_orientation_it_ignores() {
        let table = fence_table();
        let mut app = app_with(&[(origin(), Chunk::air())]);
        let turned = Voxel::new(
            tesserae_core::testgen::fixtures().ground.rock,
            examples::FENCE_ID,
            Orientation::new(POS_X, 3).bits(),
        );
        edit_at(&mut app, &table, [5, 4, 5], turned);
        assert_eq!(voxel_at(&app, [5, 4, 5]), fence());
    }

    #[test]
    fn a_message_written_in_a_frame_with_no_tick_is_still_applied_by_a_later_one() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO))
            .insert_resource(WorldSettings::default())
            .insert_resource(ContentTables(Arc::new(crate::server::fixture_tables())))
            .init_resource::<ChunkStore>()
            .init_resource::<MeshState>()
            .add_plugins(plugin);
        app.world_mut()
            .resource_mut::<ChunkStore>()
            .insert(origin(), ground());
        app.world_mut().spawn(Player::at(DOWN_EYE));
        app.update();

        app.world_mut().write_message(down(EditAction::Remove));
        // Three frames in which no time passes and so no fixed tick runs. A
        // reader inside the fixed loop would have lost the message by the second.
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(app.world().resource::<PendingEdits>().0.len(), 1);
        assert_eq!(voxel_at(&app, [5, 7, 5]), stone(), "nothing ticked yet");

        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            100,
        )));
        app.update();
        assert!(voxel_at(&app, [5, 7, 5]).is_air());
        assert!(app.world().resource::<PendingEdits>().0.is_empty());
    }

    #[test]
    fn what_cannot_be_done_is_rejected_and_says_why() {
        let mut app = app_with(&[(origin(), ground())]);

        let mut cases: Vec<(EditIntent, RejectReason)> = vec![
            // Looking at the sky: nothing to strike.
            (
                EditIntent {
                    direction: Vec3::Y,
                    ..down(EditAction::Remove)
                },
                RejectReason::OutOfReach,
            ),
            // Too far: 4 m of air and then a ground 20 m below.
            (
                EditIntent {
                    origin: Vec3::new(5.5, 30.0, 5.5),
                    ..down(EditAction::Remove)
                },
                RejectReason::OutOfReach,
            ),
            (
                EditIntent {
                    origin: Vec3::NAN,
                    ..down(EditAction::Remove)
                },
                RejectReason::OutOfReach,
            ),
            (
                down(EditAction::Place(Voxel::AIR)),
                RejectReason::NoMaterial,
            ),
            (
                down(EditAction::Place(Voxel::new(
                    tesserae_core::testgen::fixtures().ground.rock,
                    200,
                    0,
                ))),
                RejectReason::UnknownShape,
            ),
            (
                down(EditAction::Place(Voxel::new(
                    tesserae_core::testgen::fixtures().ground.rock,
                    SHAPE_STAIR,
                    24,
                ))),
                RejectReason::BadOrientation,
            ),
        ];
        for (intent, expected) in cases.drain(..) {
            send(&mut app, intent);
            tick(&mut app);
            assert_eq!(rejections(&mut app), vec![expected], "{intent:?}");
        }
        assert_eq!(voxel_at(&app, [5, 7, 5]), stone(), "nothing changed");
    }

    /// A full reach from [`DOWN_EYE`], as an intent from the Eye itself gets.
    fn at_eye() -> Reach {
        reach_from(&down(EditAction::Remove), DOWN_EYE).unwrap()
    }

    fn from(eye: Vec3) -> EditIntent {
        EditIntent {
            origin: eye,
            ..down(EditAction::Remove)
        }
    }

    #[test]
    fn an_origin_far_from_the_players_eye_is_rejected_and_never_queued() {
        let mut app = app_with(&[(origin(), ground())]);

        // The ray would strike the ground, but it starts 40 m from the Player.
        send_from_afar(&mut app, from(DOWN_EYE + Vec3::new(40.0, 0.0, 0.0)));
        tick(&mut app);

        assert_eq!(rejections(&mut app), vec![RejectReason::OriginTooFar]);
        assert!(app.world().resource::<PendingEdits>().0.is_empty());
        assert_eq!(voxel_at(&app, [5, 7, 5]), stone(), "nothing changed");
    }

    #[test]
    fn the_tolerance_is_inclusive_and_a_hair_past_it_is_not() {
        let mut app = app_with(&[(origin(), ground())]);
        let below_eye = DOWN_EYE - Vec3::Y * 4.0;
        let toward = |at: Vec3| EditIntent {
            origin: at,
            direction: below_eye - at,
            ..down(EditAction::Remove)
        };
        let at = DOWN_EYE + Vec3::Z * EDIT_ORIGIN_BOUND_M;
        send_from_afar(&mut app, toward(at));
        assert_eq!(app.world().resource::<PendingEdits>().0.len(), 1);
        assert!(rejections(&mut app).is_empty());

        send_from_afar(&mut app, toward(at + Vec3::Z * 0.01));
        assert_eq!(rejections(&mut app), vec![RejectReason::OriginTooFar]);
    }

    #[test]
    fn the_origin_is_judged_when_the_intent_arrives_not_when_it_is_applied() {
        let mut app = app_with(&[(origin(), ground())]);
        send_from_afar(&mut app, down(EditAction::Remove));

        // The Player moves on while the intent waits for its Chunk's light.
        set_eye(&mut app, DOWN_EYE + Vec3::X * 100.0);
        tick(&mut app);

        assert!(voxel_at(&app, [5, 7, 5]).is_air());
        assert!(rejections(&mut app).is_empty());
    }

    /// Both systems touch the `Player`, so without a constraint Bevy picks an
    /// order, and the order it picked passed a behavioural version of this test
    /// with the constraint deleted. So this asks the schedule whether the pair is
    /// still ambiguous.
    #[test]
    fn movement_input_is_taken_before_edit_intents() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(WorldSettings::default())
            .insert_resource(SpawnPoint::new(Vec3::ZERO))
            .insert_resource(ContentTables(Arc::new(crate::server::fixture_tables())))
            .init_resource::<ChunkStore>()
            .init_resource::<MeshState>()
            .add_plugins((plugin, super::super::player::plugin));

        let ambiguous = app
            .world_mut()
            .schedule_scope(RunFixedMainLoop, |world, schedule| {
                schedule.initialize(world).unwrap();
                let key_of = |system: Box<dyn System<In = (), Out = ()>>| {
                    let id = system.system_type();
                    schedule
                        .systems()
                        .unwrap()
                        .find(|(_, s)| s.system_type() == id)
                        .map(|(key, _)| key)
                        .expect("scheduled")
                };
                let pose = key_of(Box::new(IntoSystem::into_system(player::receive)));
                let edit = key_of(Box::new(IntoSystem::into_system(receive)));
                schedule
                    .graph()
                    .conflicting_systems()
                    .iter()
                    .any(|(a, b, _)| (*a, *b) == (pose, edit) || (*a, *b) == (edit, pose))
            });
        assert!(!ambiguous, "nothing says which of the two runs first");
    }

    /// Straight down at the ground over (5, 5), from `origin_y`, with the Player's
    /// Eye at `eye_y`: whether the Voxel was broken.
    fn broken_from(eye_y: f32, origin_y: f32) -> bool {
        let mut app = app_with(&[(origin(), ground())]);
        set_eye(&mut app, Vec3::new(5.5, eye_y, 5.5));
        send_from_afar(
            &mut app,
            EditIntent {
                origin: Vec3::new(5.5, origin_y, 5.5),
                ..down(EditAction::Remove)
            },
        );
        tick(&mut app);
        voxel_at(&app, [5, 7, 5]).is_air()
    }

    #[test]
    fn reach_is_measured_from_the_eye_not_from_the_claimed_origin() {
        // The ground is 7 m below the origin, in reach of it. The Eye is 2 m
        // higher, so the Voxel is 9 m from the Player and not in reach of them.
        assert!(!broken_from(17.0, 15.0));
        // The same ray from an origin at the Eye is 7 m and is.
        assert!(broken_from(15.0, 15.0));
    }

    #[test]
    fn no_edit_lands_farther_from_the_eye_than_the_reach() {
        let ground_top = 8.0;
        for eye_y in [15.0, 16.0, 17.0] {
            for step in -4..=4 {
                let origin_y = eye_y + step as f32 * 0.5;
                if broken_from(eye_y, origin_y) {
                    assert!(
                        eye_y - ground_top <= EDIT_REACH_M,
                        "eye {eye_y}, origin {origin_y}: struck {} m from the Eye",
                        eye_y - ground_top
                    );
                }
            }
        }
    }

    #[test]
    fn a_place_into_an_unloaded_chunk_or_an_occupied_cell_is_rejected() {
        let mut app = app_with(&[(origin(), ground())]);
        // A post stands in the air a ray passes beside before striking the
        // ground, so the cell in front of the ground is taken without the ray
        // having noticed.
        app.world_mut().resource_mut::<ChunkStore>().set_voxel(
            VoxelCoord::of([5, 8, 5]),
            Voxel::new(
                tesserae_core::testgen::fixtures().ground.rock,
                SHAPE_POST,
                0,
            ),
        );
        send(
            &mut app,
            EditIntent {
                action: EditAction::Place(stone()),
                origin: Vec3::new(5.1, 12.0, 5.5),
                direction: Vec3::NEG_Y,
            },
        );
        tick(&mut app);
        assert_eq!(rejections(&mut app), vec![RejectReason::Occupied]);

        // A ray that starts inside the top layer of the loaded world strikes it at
        // once, and the cell in front of that is in a Chunk nobody has loaded.
        let mut app = app_with(&[(origin(), Chunk::filled(stone()))]);
        send(
            &mut app,
            EditIntent {
                action: EditAction::Place(stone()),
                origin: Vec3::new(5.5, 31.5, 5.5),
                direction: Vec3::NEG_Y,
            },
        );
        tick(&mut app);
        assert_eq!(rejections(&mut app), vec![RejectReason::NotLoaded]);

        // And a ray that never reaches anything held strikes nothing.
        let mut app = app_with(&[(origin(), ground())]);
        send(
            &mut app,
            EditIntent {
                action: EditAction::Place(stone()),
                origin: Vec3::new(20.5, 12.0, 5.5),
                direction: Vec3::X,
            },
        );
        tick(&mut app);
        assert_eq!(rejections(&mut app), vec![RejectReason::OutOfReach]);
    }

    #[test]
    fn a_chunk_at_the_shaped_cap_takes_no_more_but_lets_one_be_swapped() {
        let mut full = ground();
        let mut placed = 0;
        'fill: for y in 8..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    if placed == MAX_SHAPED_PER_CHUNK {
                        break 'fill;
                    }
                    full.set(x, y, z, stair());
                    placed += 1;
                }
            }
        }
        assert_eq!(full.shaped_count(), MAX_SHAPED_PER_CHUNK);
        let mut store = ChunkStore::default();
        store.insert(origin(), full);
        let mut meshes = MeshState::default();
        let tables = crate::server::fixture_tables();

        // Free space still exists above the stairs; another stair is refused...
        assert_eq!(
            set_voxel(&mut store, &mut meshes, &tables, [0, 30, 0], stair()),
            Err(RejectReason::TooManyShaped)
        );
        // ...a cube is not shaped and is fine...
        assert_eq!(
            set_voxel(&mut store, &mut meshes, &tables, [0, 31, 0], stone()),
            Ok(())
        );
        // ...and replacing a stair with another is net zero.
        let swapped = Voxel::shaped(
            tesserae_core::testgen::fixtures().ground.rock,
            SHAPE_STAIR,
            Orientation::new(POS_Y, 1),
        );
        assert_eq!(
            set_voxel(&mut store, &mut meshes, &tables, [0, 8, 0], swapped),
            Ok(())
        );
        assert_eq!(
            store.get(origin()).unwrap().shaped_count(),
            MAX_SHAPED_PER_CHUNK
        );
        // Breaking one makes room.
        assert_eq!(
            set_voxel(&mut store, &mut meshes, &tables, [1, 8, 0], Voxel::AIR),
            Ok(())
        );
        assert_eq!(
            set_voxel(&mut store, &mut meshes, &tables, [0, 30, 0], stair()),
            Ok(())
        );
    }

    #[test]
    fn an_edit_on_a_chunk_whose_light_has_not_settled_waits_and_is_dropped_after_a_while() {
        let mut app = app_with(&[(origin(), ground())]);
        app.world_mut()
            .resource_mut::<MeshState>()
            .mark_unlit(origin());

        send(&mut app, down(EditAction::Remove));
        for _ in 0..MAX_DEFERRED_TICKS {
            tick(&mut app);
            assert_eq!(voxel_at(&app, [5, 7, 5]), stone(), "applied too early");
        }
        assert!(rejections(&mut app).is_empty(), "still waiting");

        // Settled in time: applied.
        app.world_mut()
            .resource_mut::<MeshState>()
            .mark_lit(origin());
        tick(&mut app);
        assert!(voxel_at(&app, [5, 7, 5]).is_air());

        // Never settles: dropped, with a reason.
        let mut app = app_with(&[(origin(), ground())]);
        app.world_mut()
            .resource_mut::<MeshState>()
            .mark_unlit(origin());
        send(&mut app, down(EditAction::Remove));
        for _ in 0..=MAX_DEFERRED_TICKS + 1 {
            tick(&mut app);
        }
        assert_eq!(rejections(&mut app), vec![RejectReason::Busy]);
        assert!(app.world().resource::<PendingEdits>().0.is_empty());
    }

    #[test]
    fn a_flood_of_intents_is_bounded() {
        let mut app = app_with(&[(origin(), ground())]);
        for _ in 0..MAX_PENDING + 10 {
            app.world_mut().write_message(down(EditAction::Remove));
        }
        app.world_mut().run_system_once(receive).unwrap();
        assert_eq!(app.world().resource::<PendingEdits>().0.len(), MAX_PENDING);
        assert_eq!(rejections(&mut app), vec![RejectReason::Busy; 10]);
    }

    /// The watered tables with stone declared fuel, so a stone Voxel may hold
    /// a fire: only declared fuel does.
    fn stone_burns() -> Tables {
        let mut tables = watered();
        tables
            .register_combustion(
                tesserae_core::testgen::fixtures().ground.rock,
                tesserae_core::CombustionDef {
                    threshold: 6,
                    ignition_delay_s: 2.0,
                    emission: 0,
                    heat: None,
                    burn: tesserae_core::Burn::Permanent,
                },
            )
            .unwrap();
        tables
    }

    fn put_occupants(
        app: &mut App,
        at: [i32; 3],
        occupants: Occupants,
    ) -> (MeshState, Result<(), RejectReason>) {
        let tables = stone_burns();
        let mut store = app.world_mut().resource_mut::<ChunkStore>();
        let mut meshes = MeshState::default();
        let result = set_occupants(&mut store, &mut meshes, &tables, at, occupants);
        (meshes, result)
    }

    fn occupants_at(app: &App, at: [i32; 3]) -> Occupants {
        app.world()
            .resource::<ChunkStore>()
            .occupants(at, &app.world().resource::<ContentTables>().0.fluids)
            .expect("held")
    }

    /// Setting a stone cube on fire, stone declared fuel here, changes no
    /// Voxel. It is still an Edit: the
    /// mesh, the face next door and the level-1 cover all read it, and a
    /// Snapshot taken before it is no longer current.
    #[test]
    fn an_overlay_only_change_invalidates_what_a_snapshot_read() {
        let solid = Chunk::filled(stone());
        let mut app = app_with(&[
            (ChunkCoord::new(0, 0, 0), solid.clone()),
            (ChunkCoord::new(1, 0, 0), solid.clone()),
            (ChunkCoord::new(0, 1, 0), solid.clone()),
        ]);
        let last = CHUNK_SIZE as i32 - 1;
        let at = [last, 4, 4];
        let snapshot = app
            .world()
            .resource::<ChunkStore>()
            .shared(origin())
            .unwrap();

        let burning = Occupants {
            burning: Some(Burning { fuel: 9 }),
            ..occupants_at(&app, at)
        };
        let (meshes, result) = put_occupants(&mut app, at, burning);

        assert_eq!(result, Ok(()));
        assert_eq!(voxel_at(&app, at), stone(), "the Voxel changed");
        assert_eq!(occupants_at(&app, at), burning);
        assert!(meshes.is_dirty(origin()));
        assert!(meshes.is_edited(origin()), "the cover does not know");
        assert!(
            meshes.is_dirty(ChunkCoord::new(1, 0, 0)),
            "the face next door does not know"
        );
        assert!(!meshes.is_dirty(ChunkCoord::new(0, 1, 0)));
        let store = app.world().resource::<ChunkStore>();
        assert!(!store.unchanged_since(origin(), &snapshot));
        assert_eq!(snapshot.overlay.get(last as usize, 4, 4), Overlay::NONE);
    }

    /// A refused reading changes neither layer.
    #[test]
    fn what_the_tables_refuse_changes_nothing() {
        let mut app = app_with(&[(origin(), pool())]);
        let source = Fluid::source(water());
        let was = occupants_at(&app, [5, 8, 5]);
        let cases = [
            (
                Occupants {
                    solid: stone(),
                    fluid: Some(source),
                    burning: None,
                },
                RejectReason::FluidExcluded,
            ),
            (
                Occupants {
                    solid: Voxel::AIR,
                    fluid: Some(source),
                    burning: Some(Burning { fuel: 1 }),
                },
                RejectReason::NoFuel,
            ),
            (
                Occupants {
                    solid: Voxel::AIR,
                    fluid: Some(Fluid::source(
                        tesserae_core::testgen::fixtures().ground.rock,
                    )),
                    burning: None,
                },
                RejectReason::NotAFluid,
            ),
            (
                Occupants {
                    solid: Voxel::cube(water()),
                    fluid: None,
                    burning: None,
                },
                RejectReason::NotSolid,
            ),
        ];
        for (occupants, expected) in cases {
            let (meshes, result) = put_occupants(&mut app, [5, 8, 5], occupants);
            assert_eq!(result, Err(expected), "{occupants:?}");
            assert!(!meshes.is_dirty(origin()), "{occupants:?} dirtied a mesh");
        }
        assert_eq!(occupants_at(&app, [5, 8, 5]), was);
        assert_eq!(voxel_at(&app, [5, 8, 5]), Voxel::cube(water()));
    }

    /// The old whole-Voxel Edit reads through the one interpretation: water
    /// placed the old way is a source, stone displaces it, and breaking a solid
    /// that holds water leaves the water, stored the canonical way.
    #[test]
    fn set_voxel_goes_through_the_layer_rules() {
        let mut app = app_with(&[(origin(), ground())]);
        let water = water();
        let tables = watered();
        let put = |app: &mut App, at, voxel| {
            let mut store = app.world_mut().resource_mut::<ChunkStore>();
            set_voxel(&mut store, &mut MeshState::default(), &tables, at, voxel)
        };

        assert_eq!(put(&mut app, [5, 8, 5], Voxel::cube(water)), Ok(()));
        assert_eq!(
            occupants_at(&app, [5, 8, 5]).fluid,
            Some(Fluid::source(water))
        );
        assert_eq!(put(&mut app, [5, 8, 5], stone()), Ok(()));
        assert_eq!(occupants_at(&app, [5, 8, 5]).fluid, None);

        let wet_stair = Occupants {
            solid: stair(),
            fluid: Some(Fluid::source(water)),
            burning: Some(Burning { fuel: 2 }),
        };
        assert_eq!(put_occupants(&mut app, [6, 8, 5], wet_stair).1, Ok(()));
        assert_eq!(put(&mut app, [6, 8, 5], Voxel::AIR), Ok(()));
        assert_eq!(
            occupants_at(&app, [6, 8, 5]),
            Occupants {
                solid: Voxel::AIR,
                fluid: Some(Fluid::source(water)),
                burning: None,
            }
        );
        assert_eq!(
            voxel_at(&app, [6, 8, 5]),
            Voxel::cube(water),
            "not canonical"
        );
        let store = app.world().resource::<ChunkStore>();
        assert_eq!(store.overlay(origin()).unwrap().get(6, 8, 5), Overlay::NONE);
    }

    fn flowing() -> Fluid {
        Fluid {
            source: false,
            surface: 9,
            reach: 3,
            ..Fluid::source(water())
        }
    }

    #[test]
    fn a_source_is_placed_in_front_and_remove_takes_solid_before_source() {
        let water = water();
        let mut app = app_with(&[(origin(), ground())]);
        send(&mut app, down(EditAction::PlaceSource(water)));
        tick(&mut app);
        assert_eq!(voxel_at(&app, [5, 8, 5]), Voxel::cube(water));
        assert_eq!(
            occupants_at(&app, [5, 8, 5]).fluid,
            Some(Fluid::source(water))
        );

        send(&mut app, down(EditAction::Remove));
        tick(&mut app);
        assert_eq!(occupants_at(&app, [5, 8, 5]).fluid, None);
        assert_eq!(voxel_at(&app, [5, 7, 5]), stone(), "the bed went too");

        let wet_stair = Occupants {
            solid: stair(),
            fluid: Some(Fluid::source(water)),
            burning: None,
        };
        assert_eq!(put_occupants(&mut app, [5, 8, 5], wet_stair).1, Ok(()));
        send(&mut app, down(EditAction::Remove));
        tick(&mut app);
        assert_eq!(
            occupants_at(&app, [5, 8, 5]),
            Occupants {
                solid: Voxel::AIR,
                ..wet_stair
            }
        );
        assert!(rejections(&mut app).is_empty());
    }

    #[test]
    fn a_waterlogged_stair_is_removed_before_its_source() {
        let water = water();
        let mut app = app_with(&[(origin(), ground())]);
        put_occupants(
            &mut app,
            [5, 8, 5],
            Occupants {
                solid: stair(),
                fluid: None,
                burning: None,
            },
        )
        .1
        .unwrap();
        send(&mut app, down(EditAction::PlaceSource(water)));
        tick(&mut app);
        let wet = occupants_at(&app, [5, 8, 5]);
        assert_eq!(
            (wet.solid, wet.fluid),
            (stair(), Some(Fluid::source(water)))
        );
        assert_eq!(voxel_at(&app, [5, 8, 5]), stair(), "what collision reads");
        assert_eq!(occupants_at(&app, [5, 9, 5]).fluid, None);

        send(&mut app, down(EditAction::PlaceSource(water)));
        tick(&mut app);
        assert_eq!(rejections(&mut app), vec![RejectReason::Occupied]);
        assert_eq!(occupants_at(&app, [5, 8, 5]), wet);

        send(&mut app, down(EditAction::Remove));
        tick(&mut app);
        assert_eq!(
            occupants_at(&app, [5, 8, 5]).fluid,
            Some(Fluid::source(water))
        );
        assert_eq!(occupants_at(&app, [5, 8, 5]).solid, Voxel::AIR);
        send(&mut app, down(EditAction::Remove));
        tick(&mut app);
        assert_eq!(occupants_at(&app, [5, 8, 5]).fluid, None);
        assert!(rejections(&mut app).is_empty());
    }

    /// Placing a stair into a source keeps the source in its notch, breaking
    /// the stair leaves the source standing, and a cube displaces it. A stair
    /// refused at the shaped cap leaves the water exactly as it was.
    #[test]
    fn solid_edits_keep_or_displace_fluid_by_the_space_they_leave() {
        let water = water();
        let mut store = ChunkStore::default();
        store.insert(origin(), ground());
        let mut meshes = MeshState::default();
        let tables = watered();
        let at = [5, 8, 5];
        set_voxel(&mut store, &mut meshes, &tables, at, Voxel::cube(water)).unwrap();
        set_voxel(&mut store, &mut meshes, &tables, at, stair()).unwrap();
        let read = |store: &ChunkStore| store.occupants(at, &tables.fluids).unwrap();
        assert_eq!(read(&store).fluid, Some(Fluid::source(water)));
        assert_eq!(read(&store).solid, stair());
        set_voxel(&mut store, &mut meshes, &tables, at, Voxel::AIR).unwrap();
        assert_eq!(store.voxel(at), Some(Voxel::cube(water)), "a source again");
        set_voxel(&mut store, &mut meshes, &tables, at, stair()).unwrap();
        set_voxel(&mut store, &mut meshes, &tables, at, stone()).unwrap();
        assert_eq!(read(&store).fluid, None);

        let mut full = ground();
        let mut placed = 0;
        'fill: for y in 9..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    if placed == MAX_SHAPED_PER_CHUNK {
                        break 'fill;
                    }
                    full.set(x, y, z, stair());
                    placed += 1;
                }
            }
        }
        let mut store = ChunkStore::default();
        store.insert(origin(), full);
        set_voxel(&mut store, &mut meshes, &tables, at, Voxel::cube(water)).unwrap();
        let before = read(&store);
        assert_eq!(
            set_voxel(&mut store, &mut meshes, &tables, at, stair()),
            Err(RejectReason::TooManyShaped)
        );
        assert_eq!(read(&store), before);
    }

    /// A source is placed on flowing water of its own kind, and the ray to do
    /// it goes through flowing water: flow is packed air.
    #[test]
    fn a_source_replaces_flow_and_the_tool_aims_through_flow() {
        let water = water();
        let mut app = app_with(&[(origin(), ground())]);
        let flow = Occupants {
            solid: Voxel::AIR,
            fluid: Some(flowing()),
            burning: None,
        };
        put_occupants(&mut app, [5, 8, 5], flow).1.unwrap();
        put_occupants(&mut app, [5, 9, 5], flow).1.unwrap();
        send(&mut app, down(EditAction::PlaceSource(water)));
        tick(&mut app);
        assert_eq!(
            occupants_at(&app, [5, 8, 5]).fluid,
            Some(Fluid::source(water))
        );
        assert_eq!(occupants_at(&app, [5, 9, 5]).fluid, Some(flowing()));
        assert!(rejections(&mut app).is_empty());
    }

    /// Refused fluid intents change neither layer and say why, and they pass the
    /// origin and reach checks every intent does.
    #[test]
    fn the_fluid_tool_is_refused_like_any_edit_and_changes_nothing() {
        let water = water();
        let mut app = app_with(&[(origin(), ground())]);
        let flow = Occupants {
            solid: Voxel::AIR,
            fluid: Some(flowing()),
            burning: None,
        };
        put_occupants(&mut app, [6, 8, 5], flow).1.unwrap();
        let before: Vec<Occupants> = (0..16).map(|y| occupants_at(&app, [5, y, 5])).collect();
        let cases = [
            (
                down(EditAction::PlaceSource(
                    tesserae_core::testgen::fixtures().ground.rock,
                )),
                RejectReason::NotAFluid,
            ),
            (
                EditIntent {
                    direction: Vec3::Y,
                    ..down(EditAction::PlaceSource(water))
                },
                RejectReason::OutOfReach,
            ),
            (
                EditIntent {
                    origin: Vec3::new(5.5, 30.0, 5.5),
                    ..down(EditAction::Remove)
                },
                RejectReason::OutOfReach,
            ),
        ];
        for (intent, expected) in cases {
            send(&mut app, intent);
            tick(&mut app);
            assert_eq!(rejections(&mut app), vec![expected], "{intent:?}");
        }
        // Flowing water is not a source to remove: the ray finds the bed.
        send(
            &mut app,
            EditIntent {
                origin: Vec3::new(6.5, 12.0, 5.5),
                ..down(EditAction::Remove)
            },
        );
        tick(&mut app);
        assert!(rejections(&mut app).is_empty());
        assert_eq!(occupants_at(&app, [6, 8, 5]), flow);
        assert!(voxel_at(&app, [6, 7, 5]).is_air());

        send_from_afar(
            &mut app,
            EditIntent {
                origin: DOWN_EYE + Vec3::X * 40.0,
                ..down(EditAction::PlaceSource(water))
            },
        );
        assert_eq!(rejections(&mut app), vec![RejectReason::OriginTooFar]);
        let after: Vec<Occupants> = (0..16).map(|y| occupants_at(&app, [5, y, 5])).collect();
        assert_eq!(before, after);
    }

    /// The ordinary Edit's ray goes through flowing water as it goes through a
    /// source: a Break takes the bed and a Place fills the wet Voxel.
    #[test]
    fn the_solid_ray_aims_through_flowing_water() {
        let mut app = app_with(&[(origin(), ground())]);
        let flow = Occupants {
            solid: Voxel::AIR,
            fluid: Some(flowing()),
            burning: None,
        };
        put_occupants(&mut app, [5, 8, 5], flow).1.unwrap();
        send(&mut app, down(EditAction::Remove));
        tick(&mut app);
        assert!(voxel_at(&app, [5, 7, 5]).is_air(), "the bed under the flow");
        assert_eq!(occupants_at(&app, [5, 8, 5]), flow);

        send(&mut app, down(EditAction::Place(stone())));
        tick(&mut app);
        assert_eq!(
            occupants_at(&app, [5, 7, 5]).solid,
            stone(),
            "the hole is filled"
        );
        send(&mut app, down(EditAction::Place(stone())));
        tick(&mut app);
        assert_eq!(
            occupants_at(&app, [5, 8, 5]),
            Occupants {
                solid: stone(),
                fluid: None,
                burning: None,
            },
            "the stone displaced the flow"
        );
        assert!(rejections(&mut app).is_empty());
    }

    #[test]
    fn each_tick_applies_a_bounded_number() {
        let mut app = app_with(&[(origin(), ground())]);
        for x in 0..(EDITS_PER_TICK as i32 + 5) {
            send(
                &mut app,
                EditIntent {
                    origin: Vec3::new(x as f32 + 0.5, 12.0, 5.5),
                    ..down(EditAction::Remove)
                },
            );
        }
        tick(&mut app);
        assert_eq!(app.world().resource::<PendingEdits>().0.len(), 5);
    }

    /// The ground with declared wood, burning for 8 s, set into its top where
    /// `down` strikes, at (4, 7, 5) and at (5, 7, 5).
    fn wood_app() -> (App, MaterialId) {
        use tesserae_core::{Burn, CombustionDef, MaterialDef, Occlusion, RenderClass};
        let mut tables = watered();
        let wood = tables
            .materials
            .register(MaterialDef {
                emission: 0,
                dimming: 15,
                occlusion: Occlusion::Opaque,
                render: RenderClass::Opaque,
                solid: true,
            })
            .unwrap();
        tables
            .register_combustion(
                wood,
                CombustionDef {
                    threshold: 6,
                    ignition_delay_s: 2.0,
                    emission: 12,
                    heat: None,
                    burn: Burn::For {
                        seconds: 8.0,
                        residue: 0,
                    },
                },
            )
            .unwrap();
        let mut chunk = ground();
        chunk.set(4, 7, 5, Voxel::cube(wood));
        chunk.set(5, 7, 5, Voxel::cube(wood));
        (app_with_tables(&[(origin(), chunk)], tables), wood)
    }

    fn down_at(x: f32, action: EditAction) -> EditIntent {
        EditIntent {
            origin: Vec3::new(x, DOWN_EYE.y, DOWN_EYE.z),
            ..down(action)
        }
    }

    /// The fire tool lights the declared fuel its ray strikes with the whole
    /// 8 s a fresh fire has, as a Mod's ignition does, and puts it out,
    /// leaving the wood.
    #[test]
    fn the_fire_tool_lights_declared_fuel_with_its_whole_fuel_and_puts_it_out() {
        let (mut app, wood) = wood_app();
        let unlit = Occupants {
            solid: Voxel::cube(wood),
            fluid: None,
            burning: None,
        };
        send(&mut app, down(EditAction::Ignite));
        tick(&mut app);
        assert_eq!(
            occupants_at(&app, [5, 7, 5]),
            Occupants {
                burning: Some(Burning { fuel: 512 }),
                ..unlit
            }
        );
        send(&mut app, down(EditAction::Remove));
        tick(&mut app);
        assert_eq!(occupants_at(&app, [5, 7, 5]), unlit);
        assert!(rejections(&mut app).is_empty());
    }

    /// With no Game nothing is declared fuel, so every built-in solid is
    /// refused as one that does not burn.
    #[test]
    fn a_world_with_no_game_burns_nothing() {
        let mut chunk = ground();
        for x in 1..=6 {
            chunk.set(x, 7, 5, Voxel::cube(x as MaterialId));
        }
        let mut app = app_with_tables(&[(origin(), chunk)], crate::server::fixture_tables());
        for x in 1..=6 {
            send(&mut app, down_at(x as f32 + 0.5, EditAction::Ignite));
            tick(&mut app);
            assert_eq!(rejections(&mut app), [RejectReason::NotFlammable], "{x}");
            assert!(occupants_at(&app, [x, 7, 5]).burning.is_none());
        }
    }

    /// Non-fuel, fuel already alight, fuel under water, nothing in reach and
    /// nothing burning are refused with the reasons a Mod's fire Commands
    /// meet, and change nothing.
    #[test]
    fn the_fire_tool_refuses_what_a_fire_command_would_and_changes_nothing() {
        let (mut app, _) = wood_app();
        let source = Occupants {
            solid: Voxel::AIR,
            fluid: Some(Fluid::source(water())),
            burning: None,
        };
        put_occupants(&mut app, [4, 8, 5], source).1.unwrap();
        send(&mut app, down(EditAction::Ignite));
        tick(&mut app);
        assert!(rejections(&mut app).is_empty());
        let column = |app: &App| -> Vec<Occupants> {
            [4, 5, 6]
                .into_iter()
                .flat_map(|x| (0..16).map(move |y| [x, y, 5]))
                .map(|at| occupants_at(app, at))
                .collect()
        };
        let before = column(&app);
        let cases = [
            (down_at(6.5, EditAction::Ignite), "that does not burn"),
            (down(EditAction::Ignite), "it is already burning"),
            (down_at(4.5, EditAction::Ignite), "it is too wet to burn"),
            (
                EditIntent {
                    direction: Vec3::Y,
                    ..down(EditAction::Ignite)
                },
                "nothing in reach",
            ),
            (
                EditIntent {
                    origin: Vec3::new(5.5, 30.0, 5.5),
                    ..down(EditAction::Remove)
                },
                "nothing in reach",
            ),
        ];
        for (intent, expected) in cases {
            send(&mut app, intent);
            tick(&mut app);
            let said: Vec<_> = rejections(&mut app).iter().map(|r| r.message()).collect();
            assert_eq!(said, vec![expected], "{intent:?}");
        }
        assert_eq!(before, column(&app));
    }

    /// Both fire actions meet the origin check on arrival, reach from the
    /// Eye, and the wait for their Chunk's light that every intent meets.
    #[test]
    fn the_fire_tool_is_vetted_queued_and_deferred_like_any_edit() {
        let (mut app, _) = wood_app();
        for action in [EditAction::Ignite, EditAction::Remove] {
            send_from_afar(
                &mut app,
                EditIntent {
                    origin: DOWN_EYE + Vec3::X * 40.0,
                    ..down(action)
                },
            );
            assert_eq!(rejections(&mut app), vec![RejectReason::OriginTooFar]);
            assert!(app.world().resource::<PendingEdits>().0.is_empty());

            // The wood is 7 m below the origin and 9 m below the Eye.
            set_eye(&mut app, Vec3::new(5.5, 17.0, 5.5));
            send_from_afar(
                &mut app,
                EditIntent {
                    origin: Vec3::new(5.5, 15.0, 5.5),
                    ..down(action)
                },
            );
            tick(&mut app);
            assert_eq!(rejections(&mut app), vec![RejectReason::OutOfReach]);
        }
        let burning = |app: &App| occupants_at(app, [5, 7, 5]).burning.is_some();
        for (action, lit) in [(EditAction::Ignite, true), (EditAction::Remove, false)] {
            app.world_mut()
                .resource_mut::<MeshState>()
                .mark_unlit(origin());
            send(&mut app, down(action));
            tick(&mut app);
            assert_eq!(burning(&app), !lit, "{action:?} applied too early");
            assert_eq!(app.world().resource::<PendingEdits>().0.len(), 1);
            app.world_mut()
                .resource_mut::<MeshState>()
                .mark_lit(origin());
            tick(&mut app);
            assert_eq!(burning(&app), lit, "{action:?}");
        }
        assert!(rejections(&mut app).is_empty());
    }

    /// The per-Voxel rules a source or fire Command meets, shared with the
    /// fluid tool: room for the fluid, no other fluid or source, declared and
    /// dry fuel to light, and fire to put out.
    #[test]
    fn source_and_fire_rules_say_why_they_refuse() {
        use tesserae_core::{Burn, CombustionDef, FluidDef, MaterialDef, Occlusion, RenderClass};
        let mut tables = watered();
        let lava = tables
            .materials
            .register(MaterialDef {
                emission: 15,
                dimming: 15,
                occlusion: Occlusion::Opaque,
                render: RenderClass::Opaque,
                solid: false,
            })
            .unwrap();
        tables
            .register_fluid(
                lava,
                FluidDef {
                    waterlogs: false,
                    extinguishes: false,
                    ..WATER_FLUID
                },
            )
            .unwrap();
        let oil = tables
            .materials
            .register(MaterialDef {
                emission: 0,
                dimming: 4,
                occlusion: Occlusion::SameMaterial,
                render: RenderClass::Blend,
                solid: false,
            })
            .unwrap();
        tables
            .register_fluid(
                oil,
                FluidDef {
                    extinguishes: false,
                    ..WATER_FLUID
                },
            )
            .unwrap();
        tables
            .register_combustion(
                tesserae_core::testgen::fixtures().ground.rock,
                CombustionDef {
                    threshold: 6,
                    ignition_delay_s: 2.0,
                    emission: 12,
                    heat: None,
                    burn: Burn::For {
                        seconds: 1.0,
                        residue: 0,
                    },
                },
            )
            .unwrap();
        let water = water();
        let at = |solid, fluid| Occupants {
            solid,
            fluid,
            burning: None,
        };
        let flowing = Fluid {
            source: false,
            surface: 8,
            reach: 3,
            ..Fluid::source(water)
        };
        let flow = Some(flowing);
        let air = Voxel::AIR;
        assert_eq!(
            with_source(at(air, flow), water, &tables).unwrap().fluid,
            Some(Fluid::source(water)),
            "flow of the same fluid becomes the source"
        );
        for (here, material, reason) in [
            (at(air, flow), lava, RejectReason::Occupied),
            (
                at(air, Some(Fluid::source(lava))),
                lava,
                RejectReason::Occupied,
            ),
            (at(stone(), None), water, RejectReason::FluidExcluded),
            (at(stair(), None), lava, RejectReason::FluidExcluded),
            (
                at(air, None),
                tesserae_core::testgen::fixtures().lantern,
                RejectReason::NotAFluid,
            ),
        ] {
            assert_eq!(
                with_source(here, material, &tables),
                Err(reason),
                "{here:?}"
            );
        }
        assert!(with_source(at(stair(), None), water, &tables).is_ok());
        assert_eq!(without_source(at(air, flow)), Err(RejectReason::NoSource));

        // Alone in air, so only its own voids can wet it.
        struct Alone((Voxel, Overlay));
        impl tesserae_core::flow::CellView for Alone {
            fn stored(&self, at: [i32; 3]) -> Option<(Voxel, Overlay)> {
                Some(if at == [0; 3] {
                    self.0
                } else {
                    (Voxel::AIR, Overlay::NONE)
                })
            }
        }
        let ignite_alone = |here: Occupants| {
            let alone = Alone(here.encode(&tables.fluids));
            ignite(
                here,
                tesserae_core::fire::is_wet(&alone, &tables, [0; 3]),
                &tables,
            )
        };
        let lit = ignite_alone(at(stair(), None)).unwrap();
        assert_eq!(lit.burning, Some(Burning { fuel: 64 }));
        assert_eq!(lit.solid, stair());
        for (here, reason) in [
            (at(air, None), RejectReason::NoFuel),
            (
                at(
                    Voxel::cube(tesserae_core::testgen::fixtures().lantern),
                    None,
                ),
                RejectReason::NotFlammable,
            ),
            (lit, RejectReason::AlreadyBurning),
            (at(stair(), flow), RejectReason::Wet),
        ] {
            assert_eq!(ignite_alone(here), Err(reason), "{here:?}");
        }
        let oily = Some(Fluid {
            material: oil,
            ..flowing
        });
        assert!(
            ignite_alone(at(stair(), oily)).is_ok(),
            "a fluid that does not extinguish does not protect"
        );
        assert_eq!(extinguish(lit).unwrap().burning, None);
        assert_eq!(extinguish(at(stair(), None)), Err(RejectReason::NotBurning));
    }
    #[test]
    fn picking_samples_geometry_without_editing_and_rejects_forged_origins() {
        let voxel = Voxel::shaped(
            tesserae_core::testgen::fixtures().ground.rock,
            SHAPE_STAIR,
            tesserae_core::Orientation::IDENTITY.turned(),
        );
        let mut chunk = Chunk::air();
        chunk.set(5, 7, 5, voxel);
        let app = app_with(&[(origin(), chunk)]);
        let request = PickIntent {
            seq: 1,
            origin: DOWN_EYE,
            direction: Vec3::NEG_Y,
        };
        let store = app.world().resource::<ChunkStore>();
        let world = app.world().resource::<WorldSettings>();
        assert_eq!(
            pick_item(store, &watered(), world, request, DOWN_EYE),
            Some(AimItem::Solid(voxel))
        );
        assert_eq!(store.voxel([5, 7, 5]), Some(voxel));
        assert_eq!(
            pick_item(
                store,
                &watered(),
                world,
                PickIntent {
                    origin: Vec3::splat(100.0),
                    ..request
                },
                DOWN_EYE
            ),
            None
        );
    }

    #[test]
    fn aim_bounds_follow_shapes_and_the_ghost_matches_the_edit() {
        let rock = tesserae_core::testgen::fixtures().ground.rock;
        for (shape, top) in [
            (0, 8.0),
            (tesserae_core::SHAPE_SLAB, 7.5),
            (SHAPE_STAIR, 8.0),
        ] {
            let mut chunk = Chunk::air();
            chunk.set(5, 7, 5, Voxel::new(rock, shape, 0));
            let mut app = app_with(&[(origin(), chunk)]);
            let report = AimReport {
                seq: 1,
                origin: DOWN_EYE,
                direction: Vec3::NEG_Y,
                held: Some(AimItem::Solid(stone())),
            };
            let target = aim_target(
                app.world().resource::<ChunkStore>(),
                &watered(),
                app.world().resource::<WorldSettings>(),
                app.world().resource::<MeshState>(),
                report,
                at_eye(),
            );
            let bounds = target.outline.unwrap();
            assert_eq!(bounds.min, Vec3::new(5.0, 7.0, 5.0));
            assert_eq!(bounds.max, Vec3::new(6.0, top, 6.0));
            let ghost = target.ghost.unwrap();
            assert_eq!(ghost, Vec3::new(5.0, 8.0, 5.0));
            send(&mut app, down(EditAction::Place(stone())));
            tick(&mut app);
            assert_eq!(
                app.world()
                    .resource::<ChunkStore>()
                    .voxel(ghost.to_array().map(|v| v as i32)),
                Some(stone())
            );
            let fire = aim_target(
                app.world().resource::<ChunkStore>(),
                &watered(),
                app.world().resource::<WorldSettings>(),
                app.world().resource::<MeshState>(),
                AimReport {
                    held: Some(AimItem::Fire),
                    ..report
                },
                at_eye(),
            );
            assert_eq!(fire.ghost, None);
        }
    }
    #[test]
    fn a_ghost_waits_for_light_and_fire_outlines_fuel_without_a_ghost() {
        let (mut app, _) = wood_app();
        let report = AimReport {
            seq: 1,
            origin: DOWN_EYE,
            direction: Vec3::NEG_Y,
            held: Some(AimItem::Solid(stone())),
        };
        let target = |app: &App, held| {
            aim_target(
                app.world().resource::<ChunkStore>(),
                app.world().resource::<ContentTables>(),
                app.world().resource::<WorldSettings>(),
                app.world().resource::<MeshState>(),
                AimReport { held, ..report },
                at_eye(),
            )
        };
        assert!(target(&app, report.held).ghost.is_some());
        app.world_mut()
            .resource_mut::<MeshState>()
            .mark_unlit(origin());
        let waiting = target(&app, report.held);
        assert!(waiting.outline.is_some());
        assert_eq!(waiting.ghost, None);
        let fire = target(&app, Some(AimItem::Fire));
        assert!(fire.outline.is_some());
        assert_eq!(fire.ghost, None);
    }

    /// A ray from `camera` at the top of the ground Voxel `at`, with the Eye at
    /// [`DOWN_EYE`].
    fn camera_ray(camera: Vec3, at: [i32; 3]) -> EditIntent {
        let top = Vec3::new(at[0] as f32 + 0.5, at[1] as f32 + 1.0, at[2] as f32 + 0.5);
        EditIntent {
            origin: camera,
            direction: top - camera,
            action: EditAction::Remove,
        }
    }

    #[test]
    fn a_camera_ray_reaches_only_as_far_as_the_eye_does() {
        // Behind and over the right shoulder, as the third-person camera sits.
        let camera = DOWN_EYE + Vec3::new(0.5, 0.3, 4.0);
        let mut app = app_with(&[(origin(), ground())]);
        // 5.7 m from the Eye; farther from the camera.
        send_from_afar(&mut app, camera_ray(camera, [5, 7, 1]));
        tick(&mut app);
        assert!(rejections(&mut app).is_empty());
        assert!(voxel_at(&app, [5, 7, 1]).is_air());
        // 8.06 m from the Eye: out of reach though the camera sees it plainly.
        let mut app = app_with(&[(origin(), ground())]);
        let facing = DOWN_EYE + Vec3::new(0.5, 0.3, -4.0);
        send_from_afar(&mut app, camera_ray(facing, [5, 7, 12]));
        tick(&mut app);
        assert_eq!(rejections(&mut app), vec![RejectReason::OutOfReach]);
        assert_eq!(voxel_at(&app, [5, 7, 12]), stone());
    }

    #[test]
    fn what_stands_between_the_camera_and_the_player_is_not_aimed_at() {
        let mut chunk = ground();
        chunk.set(5, 11, 7, stone());
        let mut app = app_with(&[(origin(), chunk)]);
        send_from_afar(&mut app, camera_ray(Vec3::new(5.5, 12.0, 9.5), [5, 7, 1]));
        tick(&mut app);
        assert!(rejections(&mut app).is_empty());
        assert!(voxel_at(&app, [5, 7, 1]).is_air());
        assert_eq!(voxel_at(&app, [5, 11, 7]), stone(), "behind the Player");
    }

    #[test]
    fn a_target_the_camera_sees_around_a_corner_is_blocked_not_edited() {
        // Three metres to the Eye's side, looking straight down past a wall
        // that stands between the Eye and the ground the camera sees.
        let camera = DOWN_EYE + Vec3::X * 3.0;
        let report = AimReport {
            seq: 1,
            origin: camera,
            direction: Vec3::NEG_Y,
            held: Some(AimItem::Solid(stone())),
        };
        let mut walled = ground();
        for y in 9..12 {
            walled.set(7, y, 5, stone());
        }
        for (chunk, blocked) in [(ground(), false), (walled, true)] {
            let mut app = app_with(&[(origin(), chunk)]);
            let reach = reach_from(&camera_ray(camera, [8, 7, 5]), DOWN_EYE).unwrap();
            let target = aim_target(
                app.world().resource::<ChunkStore>(),
                &watered(),
                app.world().resource::<WorldSettings>(),
                app.world().resource::<MeshState>(),
                report,
                reach,
            );
            assert_eq!(target.blocked, blocked);
            assert_eq!(target.outline.is_none(), blocked);
            assert_eq!(target.ghost.is_none(), blocked);
            send_from_afar(
                &mut app,
                EditIntent {
                    origin: camera,
                    direction: Vec3::NEG_Y,
                    action: EditAction::Remove,
                },
            );
            tick(&mut app);
            if blocked {
                assert_eq!(rejections(&mut app), vec![RejectReason::Blocked]);
                assert_eq!(
                    voxel_at(&app, [8, 7, 5]),
                    stone(),
                    "never around the corner"
                );
            } else {
                assert!(rejections(&mut app).is_empty());
                assert!(voxel_at(&app, [8, 7, 5]).is_air());
            }
        }
    }

    #[test]
    fn camera_aim_keeps_eye_reach_and_corners_at_every_voxel_size() {
        for size in [1.0, 0.5, 0.25] {
            let chunks: Vec<_> = (0..4)
                .map(|z| (ChunkCoord::new(0, 0, z), ground()))
                .collect();
            let sized = |chunks: &[(ChunkCoord, Chunk)]| {
                let mut app = app_with(chunks);
                app.insert_resource(WorldSettings {
                    voxel_size: size,
                    ..default()
                });
                let eye = DOWN_EYE * size;
                set_eye(&mut app, eye);
                (app, eye)
            };
            let top = |at: [i32; 3]| {
                Vec3::new(at[0] as f32 + 0.5, at[1] as f32 + 1.0, at[2] as f32 + 0.5) * size
            };
            let cell = |metres: Vec3| (metres / size).floor().as_ivec3().to_array();

            // Over the shoulder and behind: 5 m out is struck, 8 m out is past
            // the Eye's reach however near the camera's line runs.
            for (ahead_m, reached) in [(5.0, true), (8.0, false)] {
                let (mut app, eye) = sized(&chunks);
                let at = cell(Vec3::new(eye.x, 7.5 * size, eye.z + ahead_m));
                let camera = eye + Vec3::new(0.5, 0.3, -4.0);
                send_from_afar(
                    &mut app,
                    EditIntent {
                        origin: camera,
                        direction: top(at) - camera,
                        action: EditAction::Remove,
                    },
                );
                tick(&mut app);
                assert_eq!(
                    voxel_at(&app, at).is_air(),
                    reached,
                    "{size} m, {ahead_m} m"
                );
            }

            // Three Voxels to the side, a wall the camera looks past hides the
            // ground from the Eye.
            let mut walled = chunks.clone();
            for y in 9..12 {
                walled[0].1.set(7, y, 5, stone());
            }
            for (chunks, blocked) in [(&chunks, false), (&walled, true)] {
                let (mut app, eye) = sized(chunks);
                let camera = eye + Vec3::X * 3.0 * size;
                send_from_afar(
                    &mut app,
                    EditIntent {
                        origin: camera,
                        direction: Vec3::NEG_Y,
                        action: EditAction::Remove,
                    },
                );
                tick(&mut app);
                assert_eq!(
                    rejections(&mut app) == vec![RejectReason::Blocked],
                    blocked,
                    "{size} m"
                );
                assert_eq!(voxel_at(&app, [8, 7, 5]).is_air(), !blocked, "{size} m");
            }
        }
    }

    #[test]
    fn stale_far_and_non_finite_aim_reports_do_not_replace_the_latest() {
        let mut app = app_with(&[(origin(), ground())]);
        let report = AimReport {
            seq: 4,
            origin: DOWN_EYE,
            direction: Vec3::NEG_Y,
            held: None,
        };
        app.world_mut().write_message(report);
        app.world_mut().run_system_once(receive_aim).unwrap();
        for bad in [
            AimReport { seq: 3, ..report },
            AimReport {
                seq: 5,
                origin: DOWN_EYE + Vec3::X * (EDIT_ORIGIN_BOUND_M + 1.0),
                ..report
            },
            AimReport {
                seq: 6,
                direction: Vec3::NAN,
                ..report
            },
        ] {
            app.world_mut().write_message(bad);
            app.world_mut().run_system_once(receive_aim).unwrap();
        }
        assert_eq!(app.world().resource::<AimState>().latest.unwrap().0.seq, 4);
    }
    #[test]
    fn remove_puts_out_fire_before_breaking_the_solid() {
        let (mut app, wood) = wood_app();
        send(&mut app, down(EditAction::Ignite));
        tick(&mut app);
        assert!(occupants_at(&app, [5, 7, 5]).burning.is_some());
        send(&mut app, down(EditAction::Remove));
        tick(&mut app);
        assert_eq!(occupants_at(&app, [5, 7, 5]).solid.material(), wood);
        assert!(occupants_at(&app, [5, 7, 5]).burning.is_none());
        send(&mut app, down(EditAction::Remove));
        tick(&mut app);
        assert!(occupants_at(&app, [5, 7, 5]).solid.is_air());
        assert!(rejections(&mut app).is_empty());
    }
    fn replay(app: &mut App, action: super::super::undo::HistoryAction) {
        use super::super::undo;
        app.world_mut().write_message(action);
        app.world_mut().run_system_once(undo::receive).unwrap();
        app.world_mut()
            .resource_mut::<Messages<undo::HistoryAction>>()
            .clear();
        app.world_mut().run_system_once(undo::apply).unwrap();
    }

    #[test]
    fn undo_redo_restores_solid_sources_and_fire_through_world_edits() {
        use super::super::undo::HistoryAction::{Redo, Undo};
        for action in [
            EditAction::Remove,
            EditAction::PlaceSource(water()),
            EditAction::Ignite,
        ] {
            let (mut app, _) = wood_app();
            let at = if matches!(action, EditAction::PlaceSource(_)) {
                [5, 8, 5]
            } else {
                [5, 7, 5]
            };
            let before = occupants_at(&app, at);
            send(&mut app, down(action));
            tick(&mut app);
            assert!(rejections(&mut app).is_empty());
            let after = occupants_at(&app, at);
            assert_ne!(before, after);
            replay(&mut app, Undo);
            assert_eq!(occupants_at(&app, at), before);
            replay(&mut app, Redo);
            assert_eq!(occupants_at(&app, at), after);
            assert!(rejections(&mut app).is_empty());
            assert!(app.world().resource::<MeshState>().is_dirty(origin()));
        }
    }

    #[test]
    fn history_refuses_conflicting_or_regenerated_cells_and_new_edits_clear_redo() {
        use super::super::undo::HistoryAction::{Redo, Undo};
        let mut app = app_with(&[(origin(), ground())]);
        replay(&mut app, Undo);
        assert_eq!(rejections(&mut app), [RejectReason::NoUndo]);
        send(&mut app, down(EditAction::Remove));
        tick(&mut app);
        let air = occupants_at(&app, [5, 7, 5]);
        let changed = Occupants {
            solid: stone(),
            ..air
        };
        put_occupants(&mut app, [5, 7, 5], changed).1.unwrap();
        replay(&mut app, Undo);
        assert_eq!(rejections(&mut app), [RejectReason::HistoryChanged]);
        assert_eq!(occupants_at(&app, [5, 7, 5]), changed);
        put_occupants(&mut app, [5, 7, 5], air).1.unwrap();
        replay(&mut app, Undo);
        assert!(rejections(&mut app).is_empty());
        send(&mut app, down(EditAction::Place(stair())));
        tick(&mut app);
        replay(&mut app, Redo);
        assert_eq!(rejections(&mut app), [RejectReason::NoRedo]);
        app.world_mut()
            .resource_mut::<ChunkStore>()
            .remove(origin());
        replay(&mut app, Undo);
        assert_eq!(rejections(&mut app), [RejectReason::NotLoaded]);
        app.world_mut()
            .resource_mut::<ChunkStore>()
            .insert(origin(), ground());
        replay(&mut app, Undo);
        assert_eq!(rejections(&mut app), [RejectReason::HistoryChanged]);
    }

    #[test]
    fn completed_hold_refuses_a_different_cell_material_or_burning_state() {
        let (mut app, _) = wood_app();
        let tables = app.world().resource::<ContentTables>();
        let expected = removal_target(
            app.world().resource::<ChunkStore>(),
            tables,
            &WorldSettings::default(),
            [5, 7, 5],
        )
        .unwrap();
        send(
            &mut app,
            down(EditAction::RemoveAt(AimRemoval {
                at: [4, 7, 5],
                ..expected
            })),
        );
        tick(&mut app);
        assert_eq!(rejections(&mut app), [RejectReason::TargetChanged]);
        send(
            &mut app,
            down(EditAction::RemoveAt(AimRemoval {
                burning: true,
                ..expected
            })),
        );
        tick(&mut app);
        assert_eq!(rejections(&mut app), [RejectReason::TargetChanged]);
        send(&mut app, down(EditAction::RemoveAt(expected)));
        tick(&mut app);
        assert!(voxel_at(&app, expected.at).is_air());
        assert!(rejections(&mut app).is_empty());
    }
    #[test]
    fn undo_waits_for_settled_light_and_preserves_its_entry_after_a_refusal() {
        use super::super::undo::HistoryAction::Undo;
        let mut app = app_with(&[(origin(), ground())]);
        send(&mut app, down(EditAction::Remove));
        tick(&mut app);
        app.world_mut()
            .resource_mut::<MeshState>()
            .mark_unlit(origin());
        replay(&mut app, Undo);
        assert_eq!(rejections(&mut app), [RejectReason::Busy]);
        assert!(voxel_at(&app, [5, 7, 5]).is_air());
        app.insert_resource(MeshState::default());
        replay(&mut app, Undo);
        assert_eq!(voxel_at(&app, [5, 7, 5]), stone());
        assert!(rejections(&mut app).is_empty());
    }
}
