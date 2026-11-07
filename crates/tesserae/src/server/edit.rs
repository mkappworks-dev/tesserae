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

mod aim;
mod apply;
mod intake;
mod layout;
mod resolve;
#[cfg(test)]
pub(in crate::server) mod tests;

use std::collections::{HashMap, VecDeque};

use bevy::prelude::*;

use super::death::PlayerIntents;
use crate::session::SessionApp;
use tesserae_core::collide::{Aabb, RayHit, VoxelVolume, overlaps, raycast_until};
use tesserae_core::mesh::{FACE_STEPS, NEG_X, NEG_Y, NEG_Z, POS_X, POS_Y, POS_Z};
use tesserae_core::{
    Burning, Fluid, Light, MAX_SHAPED_PER_CHUNK, MaterialId, Occupants, Orientation, PlaceFault,
    SHAPE_FULL_CUBE, ShapeId, Tables, Voxel, relight_many,
};
use tesserae_mods::{EditKind, Intent};

#[cfg(test)]
pub use layout::LayoutCell;
pub use layout::{CellState, LayoutAim, LayoutAnswer, LayoutIntent, LayoutParams, Span};

use super::ContentTables;
use super::game_mode::PlayerMode;
use super::player::{self, Player};
use super::streaming::MeshState;
use super::undo::Cost;
use super::world_state::{ChunkStore, LightLattice, VoxelCoord};
use crate::world::{ChunkCoord, WorldSettings};

pub(in crate::server) use aim::*;
pub(in crate::server) use apply::*;
pub(in crate::server) use intake::*;
pub(in crate::server) use resolve::*;

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
    /// Place what the Item in the selected hotbar slot places: its solid in
    /// this Shape and Orientation, or a source of its fluid. Resolved when the
    /// intent arrives.
    PlaceHeld {
        shape: ShapeId,
        orientation: Orientation,
    },
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
    /// The tool in hand, which makes the answer carry a `use_aim`.
    pub tool: Option<super::tool::ToolId>,
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
    /// What Use would do with the tool in hand, or why not; `None` with none.
    pub use_aim: Option<super::tool::UseAim>,
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
    /// A solid Voxel that would land inside the Player's body.
    InsidePlayer,
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
    /// A layout the core refuses: no axes, an empty axis, or over its cap.
    BadLayout,
    /// Some of a layout's Voxels were skipped; the rest were placed.
    LayoutPartial,
    /// A Place with an empty hotbar slot selected.
    NothingHeld,
    /// A Place with an Item that places no Material.
    PlacesNothing,
    /// A Place of a Voxel the Player names rather than holds, outside Creative.
    CreativeOnly,
    /// A Layout in Survival that the Inventory cannot pay for in full.
    TooFewItems,
    /// An undo or redo that puts back a Voxel whose Item the Player does not
    /// hold.
    ItemNeeded,
}

impl RejectReason {
    /// For the Client to put on screen.
    pub fn message(self) -> &'static str {
        match self {
            RejectReason::OutOfReach => "Nothing in reach.",
            RejectReason::OriginTooFar => "Too far from where the Server has you.",
            RejectReason::Blocked => "Something is in the way.",
            RejectReason::NotLoaded => "That part of the World is not loaded.",
            RejectReason::Occupied => "Something is already there.",
            RejectReason::UnknownMaterial => "That Material is unknown.",
            RejectReason::UnknownShape => "That Shape is unknown.",
            RejectReason::BadOrientation => "That Orientation is not allowed.",
            RejectReason::InsidePlayer => "You are standing there.",
            RejectReason::NoMaterial => "Nothing to place.",
            RejectReason::TooManyShaped => "Too many shaped Voxels in this Chunk.",
            RejectReason::NotSolid => "A fluid is not placed as a solid.",
            RejectReason::NotAFluid => "That is not a fluid.",
            RejectReason::FluidExcluded => "A full cube leaves no room for fluid.",
            RejectReason::BadSurface => "That fluid surface is not allowed.",
            RejectReason::NoFuel => "Nothing solid to burn.",
            RejectReason::NoSource => "No source there.",
            RejectReason::NotFlammable => "That does not burn.",
            RejectReason::AlreadyBurning => "It is already burning.",
            RejectReason::Wet => "It is too wet to burn.",
            RejectReason::NotBurning => "Nothing is burning there.",
            RejectReason::Busy => "The Server is busy. Try again.",
            RejectReason::NoUndo => "Nothing to undo.",
            RejectReason::NoRedo => "Nothing to redo.",
            RejectReason::HistoryChanged => {
                "That Voxel changed since, so it cannot be undone or redone."
            }
            RejectReason::TargetChanged => "The target changed. Hold again.",
            RejectReason::BadLayout => "That Layout is not allowed.",
            RejectReason::LayoutPartial => "Some Voxels of the Layout were skipped.",
            RejectReason::NothingHeld => "Nothing to place in the selected hotbar slot.",
            RejectReason::PlacesNothing => "The held Item places nothing.",
            RejectReason::CreativeOnly => "Only Creative places without an Item.",
            RejectReason::TooFewItems => "Not enough Items for this Layout.",
            RejectReason::ItemNeeded => "Putting that back needs an Item you don’t hold.",
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

/// A Place or Remove the Server applied, sent so the Client can show it landed.
/// Layouts, undo and redo are not announced: they have no single hand motion.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct EditApplied {
    pub at: [i32; 3],
    pub kind: AppliedKind,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AppliedKind {
    Placed(MaterialId),
    Removed(MaterialId),
}

struct Pending {
    intent: EditIntent,
    waited: u32,
    reach: Reach,
    /// The Item spent when it lands, in Survival.
    pay: Option<String>,
}

/// Intents received and not yet applied. Server-private: nothing outside this
/// module can push to it, so the Client can only ask by message.
#[derive(Resource, Default)]
pub(in crate::server) struct PendingEdits(VecDeque<Pending>, VecDeque<layout::PendingLayout>);

impl PendingEdits {
    pub(super) fn clear(&mut self) {
        self.0.clear();
        self.1.clear();
    }
    pub(in crate::server) fn is_empty(&self) -> bool {
        self.0.is_empty() && self.1.is_empty()
    }
}

pub(super) fn plugin(app: &mut App) {
    app.init_session_resource::<super::undo::Journal>()
        .add_player_intent::<super::undo::HistoryAction>()
        .init_session_resource::<AimState>()
        .init_session_resource::<super::tool::Tools>()
        .init_session_resource::<layout::LayoutAimState>()
        .add_player_intent::<LayoutIntent>()
        .add_session_message::<LayoutAim>()
        .add_session_message::<LayoutAnswer>()
        .add_session_message::<AimReport>()
        .add_session_message::<AimTarget>()
        .add_player_intent::<PickIntent>()
        .add_session_message::<PickedItem>()
        .init_session_resource::<PendingEdits>()
        .init_session_resource::<super::mods::EditHistory>()
        .add_player_intent::<EditIntent>()
        .add_session_message::<EditRejected>()
        .add_session_message::<super::undo::HistoryNeedsItems>()
        .add_session_message::<super::undo::RefundDropped>()
        .add_session_message::<EditApplied>()
        .init_session_resource::<super::entities::EntityStore>()
        .init_session_resource::<super::items::Items>()
        // Every frame, so a click is never lost to a frame that runs no tick.
        .add_session_systems(
            RunFixedMainLoop,
            (
                receive,
                receive_aim,
                layout::receive,
                layout::receive_aim,
                pick,
                super::undo::receive,
            )
                .in_set(RunFixedMainLoopSystems::BeforeFixedMainLoop)
                .after(player::PlayerInput),
        )
        // Before `FixedUpdate`, where `streaming::stream` meshes, so the same
        // tick's `publish` sees the edit.
        .add_session_systems(
            FixedPreUpdate,
            (
                apply_pending.run_if(super::persistence::playing),
                super::undo::apply
                    .run_if(super::persistence::playing)
                    .before(super::mods::Drain),
                answer_aim,
                layout::answer_aim,
            )
                .chain(),
        );
}
