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
use tesserae_core::collide::{VoxelVolume, raycast};
use tesserae_core::mesh::{FACE_STEPS, NEG_X, NEG_Y, NEG_Z, POS_X, POS_Y, POS_Z};
use tesserae_core::{MAX_SHAPED_PER_CHUNK, Orientation, SHAPE_FULL_CUBE, Tables, Voxel, relight};

use super::ContentTables;
use super::player::{self, Player};
use super::streaming::MeshState;
use super::world_state::{ChunkStore, LightLattice, VoxelCoord};
use crate::world::{ChunkCoord, WorldSettings};

/// How far from the Player's Eye an Edit can reach, in **metres**: a reach in
/// Voxels would be calibrated to one Voxel size.
/// `docs/adr/0003-voxel-size-is-per-world.md`.
///
/// The ray is shortened by however far its claimed origin is from the Eye, so
/// the tolerance below cannot add to this.
pub const EDIT_REACH_M: f32 = 8.0;

/// How far an Edit's ray may start from the Player's Eye, in metres. An honest
/// Client in the same process sends 0; over a wire it is the distance a Player
/// moves while an intent travels, so a third of a second at a sprint. A constant
/// until the controller has a speed to derive it from.
///
/// It bounds how stale an origin may be and not how far an Edit reaches, which is
/// [`EDIT_REACH_M`] from the Eye however far inside this the origin is.
pub const EDIT_ORIGIN_TOLERANCE_M: f32 = 2.0;

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
    /// Remove the Voxel the ray strikes.
    Break,
    /// Put this Voxel in the empty space in front of the face the ray strikes.
    Place(Voxel),
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
    /// The ray starts more than [`EDIT_ORIGIN_TOLERANCE_M`] from the Player.
    OriginTooFar,
    NotLoaded,
    /// The place a Voxel would go already holds one.
    Occupied,
    UnknownShape,
    BadOrientation,
    /// A Place of air, which is a Break by another name.
    NoMaterial,
    /// The Chunk's shaped Voxels, in stairs, would pass [`MAX_SHAPED_PER_CHUNK`].
    TooManyShaped,
    /// Its Chunk's light never settled, or too many edits were queued.
    Busy,
}

impl RejectReason {
    /// For the Client to put on screen.
    pub fn message(self) -> &'static str {
        match self {
            RejectReason::OutOfReach => "nothing in reach",
            RejectReason::OriginTooFar => "too far from where the server has you",
            RejectReason::NotLoaded => "that part of the world is not loaded",
            RejectReason::Occupied => "something is already there",
            RejectReason::UnknownShape => "unknown shape",
            RejectReason::BadOrientation => "invalid orientation",
            RejectReason::NoMaterial => "nothing to place",
            RejectReason::TooManyShaped => "too many shaped blocks in this chunk",
            RejectReason::Busy => "the server is busy, try again",
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
    /// [`EDIT_REACH_M`] less how far the origin was from the Eye on arrival, so
    /// that whatever the ray strikes is within [`EDIT_REACH_M`] of the Eye.
    reach_m: f32,
}

/// Intents received and not yet applied. Server-private: nothing outside this
/// module can push to it, so the Client can only ask by message.
#[derive(Resource, Default)]
pub(in crate::server) struct PendingEdits(VecDeque<Pending>);

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<PendingEdits>()
        .add_message::<EditIntent>()
        .add_message::<EditRejected>()
        // Every frame, so a click is never lost to a frame that runs no tick.
        .add_systems(
            RunFixedMainLoop,
            receive
                .in_set(RunFixedMainLoopSystems::BeforeFixedMainLoop)
                .after(player::PlayerInput),
        )
        // Before `FixedUpdate`, where `streaming::stream` meshes, so the same
        // tick's `publish` sees the edit.
        .add_systems(FixedPreUpdate, apply_pending);
}

fn receive(
    mut intents: MessageReader<EditIntent>,
    mut pending: ResMut<PendingEdits>,
    mut rejected: MessageWriter<EditRejected>,
    player: Single<&Player>,
) {
    for intent in intents.read() {
        // Here and not in `apply_pending`: an intent can wait a second there, by
        // when the Player has moved on from where the Client meant.
        let off_eye_m = match vet_origin(intent, player.eye()) {
            Ok(distance) => distance,
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
            reach_m: EDIT_REACH_M - off_eye_m,
        });
    }
}

/// How far the origin is from the Eye, if it is near enough.
fn vet_origin(intent: &EditIntent, eye: Vec3) -> Result<f32, RejectReason> {
    if !intent.origin.is_finite() || !intent.direction.is_finite() {
        return Err(RejectReason::OutOfReach);
    }
    let distance = intent.origin.distance(eye);
    if distance > EDIT_ORIGIN_TOLERANCE_M {
        return Err(RejectReason::OriginTooFar);
    }
    Ok(distance)
}

fn apply_pending(
    mut pending: ResMut<PendingEdits>,
    mut store: ResMut<ChunkStore>,
    mut meshes: ResMut<MeshState>,
    mut rejected: MessageWriter<EditRejected>,
    tables: Res<ContentTables>,
    world: Res<WorldSettings>,
) {
    let mut applied = 0;
    let mut waiting = VecDeque::new();
    while applied < EDITS_PER_TICK {
        let Some(mut p) = pending.0.pop_front() else {
            break;
        };
        let outcome =
            resolve(&store, &tables, &world, &p.intent, p.reach_m).and_then(|(at, voxel)| {
                // The incremental relight and the bulk solve must not both write one
                // Chunk, so an edit waits for its Chunk's light to settle.
                if meshes.is_unlit(VoxelCoord::of(at).chunk) {
                    return Err(RejectReason::Busy);
                }
                set_voxel(&mut store, &mut meshes, &tables, at, voxel)
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

/// Where an intent lands, and what goes there.
fn resolve(
    store: &ChunkStore,
    tables: &Tables,
    world: &WorldSettings,
    intent: &EditIntent,
    reach_m: f32,
) -> Result<([i32; 3], Voxel), RejectReason> {
    if !intent.origin.is_finite() || !intent.direction.is_finite() {
        return Err(RejectReason::OutOfReach);
    }
    let hit = raycast(
        store,
        tables,
        world.voxel_size,
        intent.origin.to_array(),
        intent.direction.to_array(),
        reach_m,
    )
    .ok_or(RejectReason::OutOfReach)?;

    match intent.action {
        EditAction::Break => Ok((hit.at, Voxel::AIR)),
        EditAction::Place(voxel) => {
            if voxel.is_air() {
                return Err(RejectReason::NoMaterial);
            }
            let step = FACE_STEPS[hit.face];
            let target = [
                hit.at[0] + step[0],
                hit.at[1] + step[1],
                hit.at[2] + step[2],
            ];
            match store.voxel(target) {
                None => Err(RejectReason::NotLoaded),
                // Water is not solid, so a Voxel placed into it replaces it.
                Some(there) if tables.materials.solid(there) => Err(RejectReason::Occupied),
                Some(_) => Ok((target, voxel)),
            }
        }
    }
}

/// Replaces one Voxel, re-floods the light around it, and marks every Chunk
/// whose mesh that made stale.
///
/// The one Edit, shared by the intents above and by the `dev` builders, so the
/// builders exercise the path a player's click does.
///
/// A mesh is stale in three kinds of Chunk: the edited one; any the relight
/// wrote to; and the neighbour across a boundary plane the Voxel sits on, whose
/// faces next to it change whether or not any light does. Removing a stone Voxel
/// in the dark moves no light and still leaves the next Chunk a face to grow.
pub(in crate::server) fn set_voxel(
    store: &mut ChunkStore,
    meshes: &mut MeshState,
    tables: &Tables,
    at: [i32; 3],
    voxel: Voxel,
) -> Result<(), RejectReason> {
    let place = VoxelCoord::of(at);
    let chunk = store.get(place.chunk).ok_or(RejectReason::NotLoaded)?;

    // Air is air whatever shape or orientation byte it carries.
    let voxel = if voxel.is_air() { Voxel::AIR } else { voxel };
    if !voxel.is_air() {
        if !tables.shapes.contains(voxel.shape()) {
            return Err(RejectReason::UnknownShape);
        }
        if Orientation::from_bits(voxel.orientation()).is_none() {
            return Err(RejectReason::BadOrientation);
        }
    }
    // A cube looks the same however it is turned, and a connected Shape ignores
    // its orientation, so the turn is dropped. Left in, two that look identical
    // would be two palette entries and unequal.
    let voxel = if voxel.shape() == SHAPE_FULL_CUBE || tables.shapes.is_connected(voxel) {
        Voxel::new(voxel.material(), voxel.shape(), 0)
    } else {
        voxel
    };
    // Net of what it replaces: swapping one shaped Voxel for another of the same
    // cost at the cap is not adding one. Counted in stairs, a connected Shape
    // being charged for its rails.
    let added = tables.shapes.cap_units(voxel);
    let removed = tables
        .shapes
        .cap_units(chunk.get(place.x, place.y, place.z));
    if added > removed
        && chunk.shaped_units(|v| tables.shapes.cap_units(v)) + (added - removed) as usize
            > MAX_SHAPED_PER_CHUNK
    {
        return Err(RejectReason::TooManyShaped);
    }

    let was = store.light_at(at);
    store.set_voxel(place, voxel);
    let mut lattice = LightLattice::new(store);
    relight(&mut lattice, tables, at, was);
    let lit = std::mem::take(&mut lattice.touched);

    meshes.voxels_changed(place.chunk);
    meshes.edited(place.chunk);
    for coord in lit {
        meshes.edited(coord);
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::time::TimeUpdateStrategy;
    use std::time::Duration;
    use tesserae_core::mesh::POS_X;
    use tesserae_core::shape::examples;
    use tesserae_core::worldgen::{LANTERN, STONE};
    use tesserae_core::{
        CHUNK_SIZE, Chunk, ChunkLight, LitChunk, SHAPE_POST, SHAPE_STAIR, mesh_chunk,
    };

    use super::super::player::{Player, PoseReport};
    use crate::server::world_state::{SPAWN_EYE_ALTITUDE_M, SpawnPoint};

    fn stone() -> Voxel {
        Voxel::cube(STONE)
    }

    fn stair() -> Voxel {
        Voxel::shaped(STONE, SHAPE_STAIR, Orientation::IDENTITY)
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
        let mut app = App::new();
        app.insert_resource(WorldSettings::default())
            .init_resource::<ContentTables>()
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

        send(&mut app, down(EditAction::Break));
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
                chunk.set(x, 8, z, Voxel::cube(tesserae_core::material::WATER));
            }
        }
        chunk
    }

    #[test]
    fn the_ray_goes_through_water_so_break_takes_the_bed_and_place_replaces_the_water() {
        let mut app = app_with(&[(origin(), pool())]);
        send(&mut app, down(EditAction::Break));
        tick(&mut app);
        assert!(
            voxel_at(&app, [5, 7, 5]).is_air(),
            "the bed under the water"
        );
        assert_eq!(
            voxel_at(&app, [5, 8, 5]).material(),
            tesserae_core::material::WATER
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
        let tables = Tables::builtin();
        set_voxel(
            &mut store,
            &mut meshes,
            &tables,
            [0, 0, 0],
            Voxel::new(STONE, 0, 13),
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
                Voxel::new(STONE, 0, 99)
            ),
            Err(RejectReason::BadOrientation)
        );
    }

    #[test]
    fn an_edit_re_floods_the_light_around_it() {
        let mut app = app_with(&[(origin(), ground())]);
        send(&mut app, down(EditAction::Place(Voxel::cube(LANTERN))));
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
            &Tables::builtin(),
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
        let mut t = Tables::builtin();
        assert_eq!(t.shapes.register(examples::fence()), Ok(examples::FENCE_ID));
        t
    }

    fn fence() -> Voxel {
        Voxel::new(STONE, examples::FENCE_ID, 0)
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
        let turned = Voxel::new(STONE, examples::FENCE_ID, Orientation::new(POS_X, 3).bits());
        edit_at(&mut app, &table, [5, 4, 5], turned);
        assert_eq!(voxel_at(&app, [5, 4, 5]), fence());
    }

    #[test]
    fn a_message_written_in_a_frame_with_no_tick_is_still_applied_by_a_later_one() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO))
            .insert_resource(WorldSettings::default())
            .init_resource::<ContentTables>()
            .init_resource::<ChunkStore>()
            .init_resource::<MeshState>()
            .add_plugins(plugin);
        app.world_mut()
            .resource_mut::<ChunkStore>()
            .insert(origin(), ground());
        app.world_mut().spawn(Player::at(DOWN_EYE));
        app.update();

        app.world_mut().write_message(down(EditAction::Break));
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
                    ..down(EditAction::Break)
                },
                RejectReason::OutOfReach,
            ),
            // Too far: 4 m of air and then a ground 20 m below.
            (
                EditIntent {
                    origin: Vec3::new(5.5, 30.0, 5.5),
                    ..down(EditAction::Break)
                },
                RejectReason::OutOfReach,
            ),
            (
                EditIntent {
                    origin: Vec3::NAN,
                    ..down(EditAction::Break)
                },
                RejectReason::OutOfReach,
            ),
            (
                down(EditAction::Place(Voxel::AIR)),
                RejectReason::NoMaterial,
            ),
            (
                down(EditAction::Place(Voxel::new(STONE, 200, 0))),
                RejectReason::UnknownShape,
            ),
            (
                down(EditAction::Place(Voxel::new(STONE, SHAPE_STAIR, 24))),
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

    fn from(eye: Vec3) -> EditIntent {
        EditIntent {
            origin: eye,
            ..down(EditAction::Break)
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
        let at = DOWN_EYE + Vec3::Z * EDIT_ORIGIN_TOLERANCE_M;
        send_from_afar(&mut app, from(at));
        assert_eq!(app.world().resource::<PendingEdits>().0.len(), 1);
        assert!(rejections(&mut app).is_empty());

        send_from_afar(&mut app, from(at + Vec3::Z * 0.01));
        assert_eq!(rejections(&mut app), vec![RejectReason::OriginTooFar]);
    }

    #[test]
    fn the_origin_is_judged_when_the_intent_arrives_not_when_it_is_applied() {
        let mut app = app_with(&[(origin(), ground())]);
        send_from_afar(&mut app, down(EditAction::Break));

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
    fn a_pose_report_is_taken_before_the_intents_of_its_frame() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(WorldSettings::default())
            .insert_resource(SpawnPoint::new(Vec3::ZERO))
            .init_resource::<ContentTables>()
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
                let pose = key_of(Box::new(IntoSystem::into_system(player::receive_pose)));
                let edit = key_of(Box::new(IntoSystem::into_system(receive)));
                schedule
                    .graph()
                    .conflicting_systems()
                    .iter()
                    .any(|(a, b, _)| (*a, *b) == (pose, edit) || (*a, *b) == (edit, pose))
            });
        assert!(!ambiguous, "nothing says which of the two runs first");
    }

    #[test]
    fn a_report_and_an_intent_from_one_frame_are_both_taken() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            // Real time lets a slow frame run a tick that applies the edit.
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO))
            .insert_resource(WorldSettings::default())
            // The Player spawns 10 m from where the intent starts: too far for the
            // intent alone, near enough for the report to bring it in budget.
            .insert_resource(SpawnPoint::new(
                DOWN_EYE + Vec3::new(10.0, -SPAWN_EYE_ALTITUDE_M, 0.0),
            ))
            .init_resource::<ContentTables>()
            .init_resource::<ChunkStore>()
            .init_resource::<MeshState>()
            .add_plugins((plugin, super::super::player::plugin));
        app.world_mut()
            .resource_mut::<ChunkStore>()
            .insert(origin(), ground());
        app.update();

        app.world_mut().write_message(down(EditAction::Break));
        app.world_mut().write_message(PoseReport {
            seq: 1,
            eye: DOWN_EYE,
        });
        app.update();

        assert_eq!(app.world().resource::<PendingEdits>().0.len(), 1);
        assert!(rejections(&mut app).is_empty());
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
                ..down(EditAction::Break)
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
        app.world_mut()
            .resource_mut::<ChunkStore>()
            .set_voxel(VoxelCoord::of([5, 8, 5]), Voxel::new(STONE, SHAPE_POST, 0));
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
        let tables = Tables::builtin();

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
        let swapped = Voxel::shaped(STONE, SHAPE_STAIR, Orientation::new(POS_Y, 1));
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

        send(&mut app, down(EditAction::Break));
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
        send(&mut app, down(EditAction::Break));
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
            app.world_mut().write_message(down(EditAction::Break));
        }
        app.world_mut().run_system_once(receive).unwrap();
        assert_eq!(app.world().resource::<PendingEdits>().0.len(), MAX_PENDING);
        assert_eq!(rejections(&mut app), vec![RejectReason::Busy; 10]);
    }

    #[test]
    fn each_tick_applies_a_bounded_number() {
        let mut app = app_with(&[(origin(), ground())]);
        for x in 0..(EDITS_PER_TICK as i32 + 5) {
            send(
                &mut app,
                EditIntent {
                    origin: Vec3::new(x as f32 + 0.5, 12.0, 5.5),
                    ..down(EditAction::Break)
                },
            );
        }
        tick(&mut app);
        assert_eq!(app.world().resource::<PendingEdits>().0.len(), 5);
    }
}
