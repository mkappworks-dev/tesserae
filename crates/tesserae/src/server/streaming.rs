//! Chunk streaming: which Chunks exist, and which the Client has a mesh for.
//!
//! Four nested extents, in Cells, each with a job:
//!
//! | Extent  | Meaning                                                  |
//! | ------- | -------------------------------------------------------- |
//! | `mesh`  | gets a mesh, from `view_distance_m`                      |
//! | `keep`  | keeps the mesh it has — hysteresis at the visible edge    |
//! | `load`  | is generated, so `keep`'s outermost Chunk has neighbours  |
//! | `evict` | is still held — hysteresis on generation                  |
//!
//! The ordering is load-bearing rather than defensive slack. Face visibility
//! reads one Voxel across each face, so a Chunk meshed before its six
//! neighbours exist emits a skin of faces buried in the neighbour's rock —
//! hence `load` is `keep` grown by one Cell. And because `evict` is in turn
//! wider than `load`, an evicted Chunk is never adjacent to a Chunk the Client
//! holds a mesh for, so eviction can never leave a stale face behind and needs
//! no re-mesh of its own.

use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use tesserae_core::mesh::{NEG_X, NEG_Y, NEG_Z, POS_X, POS_Y, POS_Z};
use tesserae_core::{ChunkMesh, mesh_chunk};

use super::WorldGenerator;
use super::world_state::ChunkStore;
use crate::world::{ChunkCoord, WorldSettings};

/// The entity the World streams around. The Client puts this on its camera and
/// the Server reads that entity's `Transform`.
///
/// Intent, not state: the camera is a Client entity and its pose is Client
/// data. The component is declared here because the Server is the side that
/// decides what intent it accepts.
#[derive(Component)]
pub struct StreamingAnchor;

/// A Chunk's finished geometry, for the Client to upload.
///
/// Carries a `tesserae-core` `ChunkMesh` with no Bevy in it. Sent even when
/// empty, so the Client has one rule: empty means despawn.
#[derive(Message)]
pub struct ChunkMeshReady {
    pub coord: ChunkCoord,
    pub mesh: ChunkMesh,
}

/// The Client should stop drawing this Chunk.
#[derive(Message)]
pub struct ChunkUnloaded {
    pub coord: ChunkCoord,
}

/// How far the Server streams. Not World save data — see `crate::world`.
#[derive(Resource, Clone, Copy, Debug)]
pub struct StreamingSettings {
    /// Horizontal radius, in metres, that the Client gets geometry for.
    pub view_distance_m: f32,
    /// Half-extent above and below the anchor, in metres.
    pub vertical_distance_m: f32,
    /// How far past an extent a Chunk travels before the Server acts on it, so
    /// an anchor sitting on a Cell boundary does not load and evict the same
    /// Chunk every tick.
    pub hysteresis_m: f32,
}

impl Default for StreamingSettings {
    fn default() -> Self {
        StreamingSettings {
            view_distance_m: 192.0,
            vertical_distance_m: 96.0,
            hysteresis_m: 32.0,
        }
    }
}

/// How much the Server does per tick. Measured, not guessed — see the comment
/// on [`StreamingBudget::default`].
#[derive(Resource, Clone, Copy, Debug)]
pub struct StreamingBudget {
    pub generate_per_tick: usize,
    pub mesh_per_tick: usize,
}

impl Default for StreamingBudget {
    // TODO(.scratch/2026-10-11-worldgen-chunk-streaming/issues/06): a guess
    // until ticket 06 measures it. Do not read these as measured numbers.
    fn default() -> Self {
        StreamingBudget {
            generate_per_tick: 24,
            mesh_per_tick: 12,
        }
    }
}

/// A cylinder of Cells around the anchor: a radius in x and z, a half-extent in
/// y. Round horizontally because a box gives a visibly square horizon, and
/// terrain is wide and shallow so the two axes do not want the same number.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Extent {
    pub horizontal: i32,
    pub vertical: i32,
}

impl Extent {
    pub fn contains(self, delta: IVec3) -> bool {
        delta.y.abs() <= self.vertical
            && delta.x * delta.x + delta.z * delta.z <= self.horizontal * self.horizontal
    }

    pub fn grown(self, cells: i32) -> Self {
        Extent {
            horizontal: self.horizontal + cells,
            vertical: self.vertical + cells,
        }
    }

    /// Every offset inside this extent, nearest first.
    pub fn offsets(self) -> Vec<IVec3> {
        let h = self.horizontal;
        let v = self.vertical;
        let mut offsets: Vec<IVec3> = (-v..=v)
            .flat_map(|y| (-h..=h).flat_map(move |z| (-h..=h).map(move |x| IVec3::new(x, y, z))))
            .filter(|&d| self.contains(d))
            .collect();
        offsets.sort_unstable_by_key(|d| d.length_squared());
        offsets
    }
}

/// The four extents, derived from [`StreamingSettings`] and the Cell size.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rings {
    pub mesh: Extent,
    pub keep: Extent,
    pub load: Extent,
    pub evict: Extent,
}

impl Rings {
    pub fn new(settings: StreamingSettings, cell_size_m: f32) -> Self {
        // Rounded up, so the configured distance is always reached rather than
        // approached, and at least one Cell so a view distance below one Cell
        // still renders the Chunk the anchor is standing in.
        let cells = |metres: f32| ((metres / cell_size_m).ceil() as i32).max(1);
        let hysteresis = cells(settings.hysteresis_m);
        let mesh = Extent {
            horizontal: cells(settings.view_distance_m),
            vertical: cells(settings.vertical_distance_m),
        };
        let keep = mesh.grown(hysteresis);
        let load = keep.grown(1);
        Rings {
            mesh,
            keep,
            load,
            evict: load.grown(hysteresis),
        }
    }
}

/// The load volume's offsets, sorted by distance, rebuilt only when the extent
/// changes. Walking this from the anchor gives "nearest missing Chunk" without
/// sorting a few thousand coordinates every tick.
#[derive(Resource, Default)]
struct LoadOrder {
    built_for: Option<Extent>,
    offsets: Vec<IVec3>,
}

/// Which loaded Chunks need a mesh, and which the Client holds one for.
#[derive(Resource, Default)]
struct MeshState {
    dirty: HashSet<ChunkCoord>,
    delivered: HashSet<ChunkCoord>,
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<StreamingSettings>()
        .init_resource::<StreamingBudget>()
        .init_resource::<LoadOrder>()
        .init_resource::<MeshState>()
        .add_message::<ChunkMeshReady>()
        .add_message::<ChunkUnloaded>()
        .add_systems(FixedUpdate, stream);
    #[cfg(feature = "dev")]
    app.add_plugins(stats::plugin);
}

/// The six face neighbours, in the mesher's face order.
fn face_neighbors(coord: ChunkCoord) -> [ChunkCoord; 6] {
    let mut n = [coord; 6];
    n[NEG_X] = ChunkCoord::new(coord.x - 1, coord.y, coord.z);
    n[POS_X] = ChunkCoord::new(coord.x + 1, coord.y, coord.z);
    n[NEG_Y] = ChunkCoord::new(coord.x, coord.y - 1, coord.z);
    n[POS_Y] = ChunkCoord::new(coord.x, coord.y + 1, coord.z);
    n[NEG_Z] = ChunkCoord::new(coord.x, coord.y, coord.z - 1);
    n[POS_Z] = ChunkCoord::new(coord.x, coord.y, coord.z + 1);
    n
}

fn offset(center: ChunkCoord, delta: IVec3) -> ChunkCoord {
    ChunkCoord::new(center.x + delta.x, center.y + delta.y, center.z + delta.z)
}

fn delta(from: ChunkCoord, to: ChunkCoord) -> IVec3 {
    IVec3::new(to.x - from.x, to.y - from.y, to.z - from.z)
}

#[expect(
    clippy::too_many_arguments,
    reason = "one tick, and splitting it would recompute the anchor and the \
              rings in each half or need a resource to carry them"
)]
fn stream(
    mut store: ResMut<ChunkStore>,
    mut order: ResMut<LoadOrder>,
    mut meshes: ResMut<MeshState>,
    mut ready: MessageWriter<ChunkMeshReady>,
    mut unloaded: MessageWriter<ChunkUnloaded>,
    generator: Res<WorldGenerator>,
    world: Res<WorldSettings>,
    settings: Res<StreamingSettings>,
    budget: Res<StreamingBudget>,
    anchor: Query<&Transform, With<StreamingAnchor>>,
) {
    // No anchor, or more than one, is not an error: the Client has not spawned
    // its camera yet on the first tick. Streaming around an arbitrary one of
    // several would be worse than streaming around none.
    let Ok(pose) = anchor.single() else {
        return;
    };

    let rings = Rings::new(*settings, world.cell_size());
    let center = world.chunk_at(pose.translation);
    if order.built_for != Some(rings.load) {
        order.offsets = rings.load.offsets();
        order.built_for = Some(rings.load);
    }

    retire(
        &mut store,
        &mut meshes,
        &mut unloaded,
        center,
        rings.keep,
        rings.evict,
    );
    generate(
        &mut store,
        &mut meshes,
        &generator,
        &order,
        center,
        budget.generate_per_tick,
    );
    publish(
        &store,
        &mut meshes,
        &mut ready,
        &order,
        center,
        rings.mesh,
        budget.mesh_per_tick,
    );
}

/// Drops meshes that left `keep` and Chunks that left `evict`.
fn retire(
    store: &mut ChunkStore,
    meshes: &mut MeshState,
    unloaded: &mut MessageWriter<ChunkUnloaded>,
    center: ChunkCoord,
    keep: Extent,
    evict: Extent,
) {
    let gone: Vec<ChunkCoord> = meshes
        .delivered
        .iter()
        .copied()
        .filter(|&c| !keep.contains(delta(center, c)))
        .collect();
    for coord in gone {
        meshes.delivered.remove(&coord);
        unloaded.write(ChunkUnloaded { coord });
        // The Chunk itself stays loaded, so marking it dirty again is the only
        // thing that will redraw it if the anchor comes back: `dirty` is
        // cleared on delivery and nothing sets it for a Chunk that is already
        // generated.
        meshes.dirty.insert(coord);
    }

    let stale: Vec<ChunkCoord> = store
        .coords()
        .filter(|&c| !evict.contains(delta(center, c)))
        .collect();
    for coord in stale {
        store.remove(coord);
        meshes.dirty.remove(&coord);
    }
}

/// Generates up to `budget` missing Chunks inside the load extent, nearest
/// first.
fn generate(
    store: &mut ChunkStore,
    meshes: &mut MeshState,
    generator: &WorldGenerator,
    order: &LoadOrder,
    center: ChunkCoord,
    budget: usize,
) {
    let mut made = 0;
    for &d in &order.offsets {
        if made == budget {
            break;
        }
        let coord = offset(center, d);
        if store.contains(coord) {
            continue;
        }
        store.insert(coord, generator.chunk(coord.x, coord.y, coord.z));
        meshes.dirty.insert(coord);
        made += 1;
    }
}

/// Meshes up to `budget` dirty Chunks inside the mesh extent, nearest first,
/// skipping any whose six neighbours are not all loaded.
///
/// Deliberately does **not** re-dirty a Chunk's neighbours when one arrives,
/// which looks like the obvious omission. It is unnecessary here and the reason
/// is worth stating, because it stops holding the moment a Voxel can be edited:
/// a Chunk is never replaced while loaded, `dirty` is cleared only on a
/// successful mesh, and a Chunk can only be meshed once all six neighbours are
/// present — so no delivered mesh can be invalidated by a later arrival. The
/// one thing that could invalidate it, a neighbour being evicted, cannot happen
/// while the Chunk is delivered; see the extent ordering in the module doc.
/// Build-order step 5 brings edits and with them the neighbour re-mesh.
fn publish(
    store: &ChunkStore,
    meshes: &mut MeshState,
    ready: &mut MessageWriter<ChunkMeshReady>,
    order: &LoadOrder,
    center: ChunkCoord,
    mesh: Extent,
    budget: usize,
) {
    let mut sent = 0;
    for &d in &order.offsets {
        if sent == budget {
            break;
        }
        if !mesh.contains(d) {
            continue;
        }
        let coord = offset(center, d);
        if !meshes.dirty.contains(&coord) {
            continue;
        }
        let Some(chunk) = store.get(coord) else {
            continue;
        };
        let neighbors = face_neighbors(coord).map(|n| store.get(n));
        if neighbors.iter().any(Option::is_none) {
            continue;
        }

        ready.write(ChunkMeshReady {
            coord,
            mesh: mesh_chunk(chunk, neighbors),
        });
        meshes.dirty.remove(&coord);
        meshes.delivered.insert(coord);
        sent += 1;
    }
}

/// A summary of what the Server is holding, for the `dev` overlay.
///
/// `dev` only, because a shipped build should not put engine internals on
/// screen — and because the alternative is a resource nothing reads. It exists
/// so the Client can show streaming working while reading a published summary
/// rather than the Chunk store itself; the invariant in
/// `docs/adr/0002-client-server-from-day-one.md` is that the Client has no
/// access to World state, and a count is not a window onto it.
#[cfg(feature = "dev")]
pub mod stats {
    use super::*;

    #[derive(Resource, Default, Clone, Copy, Debug)]
    pub struct StreamingStats {
        /// Chunks the Server holds, including the margin the Client never sees.
        pub loaded: usize,
        /// Chunks the Server has sent the Client a mesh for. Not the number
        /// drawn: most of a 3D ring is sky, and an empty mesh is a despawn.
        pub meshed: usize,
        /// The Chunk the anchor is standing in.
        pub anchor: ChunkCoord,
    }

    pub(super) fn plugin(app: &mut App) {
        app.init_resource::<StreamingStats>()
            .add_systems(FixedPostUpdate, publish_stats);
    }

    fn publish_stats(
        mut stats: ResMut<StreamingStats>,
        store: Res<ChunkStore>,
        meshes: Res<MeshState>,
        world: Res<WorldSettings>,
        anchor: Query<&Transform, With<StreamingAnchor>>,
    ) {
        let anchor = anchor
            .single()
            .map(|pose| world.chunk_at(pose.translation))
            .unwrap_or_default();
        *stats = StreamingStats {
            loaded: store.coords().count(),
            meshed: meshes.delivered.len(),
            anchor,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use tesserae_core::{Worldgen, WorldgenParams};

    fn rings(voxel_size: f32) -> Rings {
        let world = WorldSettings {
            voxel_size,
            ..default()
        };
        Rings::new(StreamingSettings::default(), world.cell_size())
    }

    #[test]
    fn the_extents_nest_outwards() {
        for view_distance_m in [1.0, 31.0, 32.0, 33.0, 100.0, 192.0, 1000.0] {
            let settings = StreamingSettings {
                view_distance_m,
                ..default()
            };
            let r = Rings::new(settings, 32.0);
            assert!(r.keep.horizontal >= r.mesh.horizontal, "{r:?}");
            assert!(r.load.horizontal > r.keep.horizontal, "{r:?}");
            assert!(r.evict.horizontal > r.load.horizontal, "{r:?}");
            assert!(r.load.vertical > r.keep.vertical, "{r:?}");
            assert!(r.evict.vertical > r.load.vertical, "{r:?}");
        }
    }

    /// The property eviction's correctness rests on. If an evicted Chunk could
    /// be adjacent to one the Client has a mesh for, that mesh's boundary faces
    /// would be stale and eviction would have to trigger a re-mesh.
    #[test]
    fn no_evicted_chunk_is_adjacent_to_one_that_keeps_its_mesh() {
        let r = rings(1.0);
        let reach = r.evict.horizontal.max(r.evict.vertical) + 2;
        for y in -reach..=reach {
            for z in -reach..=reach {
                for x in -reach..=reach {
                    let d = IVec3::new(x, y, z);
                    if r.evict.contains(d) {
                        continue;
                    }
                    for step in [IVec3::X, IVec3::Y, IVec3::Z] {
                        for sign in [1, -1] {
                            let n = d + step * sign;
                            assert!(!r.keep.contains(n), "evicted {d} touches kept {n}");
                        }
                    }
                }
            }
        }
    }

    /// Why `load` is `keep` grown by one: the outermost Chunk that keeps a mesh
    /// must have all six neighbours generated, or it meshes a skin of faces
    /// buried in rock that has not arrived yet.
    #[test]
    fn every_neighbour_of_a_kept_chunk_is_inside_the_load_extent() {
        let r = rings(1.0);
        for d in r.keep.offsets() {
            for step in [IVec3::X, IVec3::Y, IVec3::Z] {
                for sign in [1, -1] {
                    let n = d + step * sign;
                    assert!(r.load.contains(n), "kept {d} needs {n}");
                }
            }
        }
    }

    #[test]
    fn a_metre_view_distance_doubles_in_cells_when_voxel_size_halves() {
        // ADR-0003: the setting is in metres, so the same World streams the
        // same distance whatever a Voxel is worth.
        let coarse = rings(1.0);
        let fine = rings(0.5);
        assert_eq!(fine.mesh.horizontal, coarse.mesh.horizontal * 2);
        assert_eq!(fine.mesh.vertical, coarse.mesh.vertical * 2);
    }

    #[test]
    fn offsets_are_the_extent_exactly_and_nearest_first() {
        let extent = Extent {
            horizontal: 5,
            vertical: 2,
        };
        let offsets = extent.offsets();

        let unique: HashSet<IVec3> = offsets.iter().copied().collect();
        assert_eq!(unique.len(), offsets.len(), "duplicates");
        assert!(offsets.iter().all(|&d| extent.contains(d)));

        // Nothing inside the extent is missing.
        for y in -3..=3 {
            for z in -6..=6 {
                for x in -6..=6 {
                    let d = IVec3::new(x, y, z);
                    assert_eq!(extent.contains(d), unique.contains(&d), "{d}");
                }
            }
        }

        assert!(
            offsets[0] == IVec3::ZERO,
            "the anchor's own Cell comes first"
        );
        assert!(
            offsets
                .windows(2)
                .all(|w| w[0].length_squared() <= w[1].length_squared()),
            "not sorted by distance"
        );
    }

    #[test]
    fn a_view_distance_below_one_cell_still_renders_the_anchors_own_chunk() {
        let r = Rings::new(
            StreamingSettings {
                view_distance_m: 1.0,
                vertical_distance_m: 1.0,
                hysteresis_m: 1.0,
            },
            32.0,
        );
        assert!(r.mesh.contains(IVec3::ZERO));
    }

    // The loop itself, driven headlessly. `run_system_once` rather than letting
    // `FixedUpdate` fire, so a tick is a function call and nothing depends on
    // simulated time.

    /// Small enough to fill in one tick: mesh (1, 1), keep (2, 2),
    /// load (3, 3), evict (4, 4).
    fn small() -> StreamingSettings {
        StreamingSettings {
            view_distance_m: 32.0,
            vertical_distance_m: 32.0,
            hysteresis_m: 32.0,
        }
    }

    const UNLIMITED: StreamingBudget = StreamingBudget {
        generate_per_tick: usize::MAX,
        mesh_per_tick: usize::MAX,
    };

    struct Harness {
        app: App,
        anchor: Entity,
    }

    impl Harness {
        fn new(settings: StreamingSettings, budget: StreamingBudget) -> Self {
            let world_settings = WorldSettings::default();
            let generator = Worldgen::new(
                world_settings.seed,
                world_settings.voxel_size,
                WorldgenParams::default(),
            );
            let mut app = App::new();
            app.insert_resource(world_settings)
                .insert_resource(settings)
                .insert_resource(budget)
                .insert_resource(WorldGenerator(generator))
                .init_resource::<ChunkStore>()
                .init_resource::<LoadOrder>()
                .init_resource::<MeshState>()
                .add_message::<ChunkMeshReady>()
                .add_message::<ChunkUnloaded>();
            let anchor = app
                .world_mut()
                .spawn((Transform::default(), StreamingAnchor))
                .id();
            Harness { app, anchor }
        }

        fn anchor_at(&mut self, metres: Vec3) {
            self.app
                .world_mut()
                .entity_mut(self.anchor)
                .insert(Transform::from_translation(metres));
        }

        fn tick(&mut self) -> (Vec<ChunkMeshReady>, Vec<ChunkCoord>) {
            self.app
                .world_mut()
                .run_system_once(stream)
                .expect("the tick ran");
            let ready = self
                .app
                .world_mut()
                .resource_mut::<Messages<ChunkMeshReady>>()
                .drain()
                .collect();
            let unloaded = self
                .app
                .world_mut()
                .resource_mut::<Messages<ChunkUnloaded>>()
                .drain()
                .map(|m| m.coord)
                .collect();
            (ready, unloaded)
        }

        fn store(&self) -> &ChunkStore {
            self.app.world().resource::<ChunkStore>()
        }

        fn delivered(&self) -> HashSet<ChunkCoord> {
            self.app.world().resource::<MeshState>().delivered.clone()
        }

        fn rings(&self) -> Rings {
            let world = self.app.world().resource::<WorldSettings>();
            Rings::new(
                *self.app.world().resource::<StreamingSettings>(),
                world.cell_size(),
            )
        }
    }

    fn same_geometry(a: &ChunkMesh, b: &ChunkMesh) -> bool {
        a.positions == b.positions
            && a.normals == b.normals
            && a.uvs == b.uvs
            && a.materials == b.materials
            && a.indices == b.indices
    }

    #[test]
    fn one_tick_fills_the_load_volume_and_meshes_only_what_has_neighbours() {
        let mut h = Harness::new(small(), UNLIMITED);
        let (ready, unloaded) = h.tick();
        assert!(unloaded.is_empty(), "nothing to unload on the first tick");

        let rings = h.rings();
        let want: HashSet<ChunkCoord> = rings
            .load
            .offsets()
            .into_iter()
            .map(|d| offset(ChunkCoord::new(0, 0, 0), d))
            .collect();
        assert_eq!(h.store().coords().collect::<HashSet<_>>(), want);

        for message in &ready {
            let d = delta(ChunkCoord::new(0, 0, 0), message.coord);
            assert!(rings.mesh.contains(d), "{} is outside mesh", message.coord);
            for n in face_neighbors(message.coord) {
                assert!(
                    h.store().contains(n),
                    "sent {} without neighbour {n}",
                    message.coord
                );
            }
        }
        assert_eq!(h.delivered().len(), ready.len());
    }

    /// The property dirty-marking exists for. Generation arrives over many
    /// ticks, so a Chunk meshed early has neighbours that did not exist yet; if
    /// an arriving Chunk did not mark its neighbours dirty, the Client would be
    /// left holding a mesh with a skin of faces buried in rock.
    #[test]
    fn no_mesh_the_client_still_holds_is_stale() {
        let mut h = Harness::new(
            small(),
            StreamingBudget {
                generate_per_tick: 3,
                mesh_per_tick: 2,
            },
        );

        let mut latest: bevy::platform::collections::HashMap<ChunkCoord, ChunkMesh> =
            Default::default();
        // Long enough to fill the load volume at three Chunks a tick and then
        // drain the dirty set.
        for _ in 0..400 {
            let (ready, _) = h.tick();
            for message in ready {
                latest.insert(message.coord, message.mesh);
            }
        }

        assert!(!latest.is_empty(), "nothing was ever delivered");
        assert_eq!(
            h.delivered(),
            latest.keys().copied().collect::<HashSet<_>>(),
            "delivered set and messages disagree"
        );

        for (coord, held) in &latest {
            let chunk = h.store().get(*coord).expect("still loaded");
            let neighbors = face_neighbors(*coord).map(|n| h.store().get(n));
            let fresh = mesh_chunk(chunk, neighbors);
            assert!(
                same_geometry(held, &fresh),
                "{coord}: the Client holds a stale mesh ({} quads, should be {})",
                held.quad_count(),
                fresh.quad_count()
            );
        }
    }

    #[test]
    fn moving_the_anchor_loads_ahead_and_unloads_behind() {
        let mut h = Harness::new(small(), UNLIMITED);
        h.tick();
        let was_delivered = h.delivered();
        let was_loaded: HashSet<ChunkCoord> = h.store().coords().collect();
        assert!(!was_delivered.is_empty());

        // Ten Cells, well past the evict extent, so everything behind goes.
        let cell = h.app.world().resource::<WorldSettings>().cell_size();
        h.anchor_at(Vec3::new(10.0 * cell, 0.0, 0.0));
        let (ready, unloaded) = h.tick();

        assert_eq!(
            unloaded.iter().copied().collect::<HashSet<_>>(),
            was_delivered,
            "exactly the Chunks the Client was drawing should be dropped"
        );
        for coord in &was_loaded {
            assert!(!h.store().contains(*coord), "{coord} was not evicted");
        }
        assert!(
            ready.iter().all(|m| m.coord.x > 0),
            "new meshes should be ahead of the anchor"
        );
        assert!(!ready.is_empty(), "nothing streamed in ahead");
    }

    /// Hysteresis. Without it, an anchor hovering on a Cell boundary would
    /// evict and regenerate the same Chunks every tick.
    #[test]
    fn crossing_a_cell_boundary_does_not_thrash() {
        let mut h = Harness::new(small(), UNLIMITED);
        h.tick();
        let settled: HashSet<ChunkCoord> = h.store().coords().collect();

        let cell = h.app.world().resource::<WorldSettings>().cell_size();
        for step in [cell - 0.01, cell, cell + 0.01, cell - 0.01, 0.0] {
            h.anchor_at(Vec3::new(step, 0.0, 0.0));
            let (_, unloaded) = h.tick();
            assert!(unloaded.is_empty(), "thrashed at {step} m: {unloaded:?}");
        }
        assert!(
            settled.is_subset(&h.store().coords().collect()),
            "a Chunk was evicted and had to be regenerated"
        );
    }

    /// A Chunk can leave the visible extent and come back without being
    /// evicted, because `keep` is narrower than `evict`. Its mesh is dropped on
    /// the way out, so something has to ask for it again on the way back.
    #[test]
    fn a_chunk_that_leaves_the_visible_extent_is_redrawn_when_it_returns() {
        let mut h = Harness::new(small(), UNLIMITED);
        h.tick();
        let home = ChunkCoord::new(0, 0, 0);
        assert!(h.delivered().contains(&home));

        // Three Cells: past `keep` (2) so the mesh is dropped, inside `evict`
        // (4) so the Chunk itself stays loaded.
        let cell = h.app.world().resource::<WorldSettings>().cell_size();
        h.anchor_at(Vec3::new(3.0 * cell, 0.0, 0.0));
        let (_, unloaded) = h.tick();
        assert!(
            unloaded.contains(&home),
            "the mesh should have been dropped"
        );
        assert!(h.store().contains(home), "the Chunk should still be loaded");

        h.anchor_at(Vec3::ZERO);
        let (ready, _) = h.tick();
        assert!(
            ready.iter().any(|m| m.coord == home),
            "{home} came back into view and was never redrawn"
        );
    }

    #[test]
    fn a_tick_with_no_anchor_does_nothing() {
        let mut h = Harness::new(small(), UNLIMITED);
        h.app.world_mut().entity_mut(h.anchor).despawn();
        let (ready, unloaded) = h.tick();
        assert!(ready.is_empty() && unloaded.is_empty());
        assert_eq!(h.store().coords().count(), 0);
    }

    #[test]
    fn the_budget_caps_work_per_tick() {
        let mut h = Harness::new(
            small(),
            StreamingBudget {
                generate_per_tick: 5,
                mesh_per_tick: 1,
            },
        );
        let (ready, _) = h.tick();
        assert_eq!(h.store().coords().count(), 5);
        assert!(ready.len() <= 1);
    }

    #[test]
    fn face_neighbours_sit_on_the_faces_the_mesher_expects() {
        let n = face_neighbors(ChunkCoord::new(0, 0, 0));
        assert_eq!(n[NEG_X], ChunkCoord::new(-1, 0, 0));
        assert_eq!(n[POS_X], ChunkCoord::new(1, 0, 0));
        assert_eq!(n[NEG_Y], ChunkCoord::new(0, -1, 0));
        assert_eq!(n[POS_Y], ChunkCoord::new(0, 1, 0));
        assert_eq!(n[NEG_Z], ChunkCoord::new(0, 0, -1));
        assert_eq!(n[POS_Z], ChunkCoord::new(0, 0, 1));
    }
}
