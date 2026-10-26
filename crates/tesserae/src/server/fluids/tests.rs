use super::*;
use bevy::ecs::system::RunSystemOnce;
use bevy::tasks::{AsyncComputeTaskPool, TaskPoolBuilder};
use std::time::{Duration, Instant};
use tesserae_core::collide::VoxelVolume;
use tesserae_core::flow::STEP_S;
use tesserae_core::light::light_cells;
use tesserae_core::testgen::register_water;
use tesserae_core::worldgen::{LANTERN, STONE};
use tesserae_core::{
    Chunk, Fluid, LitChunk, MaterialId, Occupants, Orientation, SHAPE_STAIR, Voxel, mesh_chunk,
};

use super::super::edit::{EditAction, EditIntent, EditRejected, PendingEdits, RejectReason};
use super::super::mods::runtime::{self as mod_runtime, PendingCommands};
use super::super::player::Player;
use super::super::streaming::face_neighbors;
use super::super::streaming::tests::{Harness, UNLIMITED, lit, same_mesh, small};

// A hang detector, not a budget: the slowest test using it takes 14 s on a laptop
// and ran past 60 s on CI, whose runner is slower and builds dependencies unoptimised.
const CEILING: Duration = Duration::from_secs(300);

fn stone() -> Voxel {
    Voxel::cube(STONE)
}

/// The Server's default tables with the fixture water, which every test here
/// places unless it loads a Game. Building them costs milliseconds; the replays
/// read them every step.
static WATERED: std::sync::LazyLock<(Tables, MaterialId)> = std::sync::LazyLock::new(|| {
    let mut tables = Arc::unwrap_or_clone(ContentTables::default().0);
    let water = register_water(&mut tables);
    (tables, water)
});

fn tables() -> Tables {
    WATERED.0.clone()
}

fn water() -> MaterialId {
    WATERED.1
}

const Y: i32 = 5;
const Z: i32 = 5;

/// The corridor of `tesserae_core::flow`'s tests, held by the Server: one Voxel
/// wide along x at z = 5 with its floor at y = 4, `half_m` each way from `cx`,
/// in whichever Chunks that takes. Positions are relative to `cx`.
struct Fixture {
    app: App,
    voxel_m: f32,
    half: i32,
    cx: i32,
}

impl Fixture {
    fn new(voxel_m: f32, half_m: f32, cx: i32) -> Self {
        Self::with_tables(voxel_m, half_m, cx, tables())
    }

    fn with_tables(voxel_m: f32, half_m: f32, cx: i32, tables: Tables) -> Self {
        AsyncComputeTaskPool::get_or_init(|| TaskPoolBuilder::new().num_threads(4).build());
        let half = (half_m / voxel_m).round() as i32;
        let mut chunks: HashMap<ChunkCoord, Chunk> = HashMap::default();
        let mut put = |x: i32, y: i32, z: i32, voxel| {
            let v = VoxelCoord::of([cx + x, y, z]);
            chunks
                .entry(v.chunk)
                .or_insert_with(Chunk::air)
                .set(v.x, v.y, v.z, voxel);
        };
        for x in -half - 1..=half + 1 {
            put(x, Y - 1, Z, stone());
            for y in Y - 1..=Y + 2 {
                put(x, y, Z - 1, stone());
                put(x, y, Z + 1, stone());
            }
        }
        for y in Y..=Y + 2 {
            put(-half - 1, y, Z, stone());
            put(half + 1, y, Z, stone());
        }
        let world = WorldSettings {
            voxel_size: voxel_m,
            ..default()
        };
        let mut app = App::new();
        app.insert_resource(world)
            .insert_resource(ContentTables(Arc::new(tables)))
            .init_resource::<ChunkStore>()
            .init_resource::<MeshState>()
            .init_resource::<FluidSim>()
            .add_plugins(edit::plugin);
        let mut store = app.world_mut().resource_mut::<ChunkStore>();
        for (coord, chunk) in chunks {
            store.insert(coord, chunk);
        }
        let mut f = Fixture {
            app,
            voxel_m,
            half,
            cx,
        };
        let eye = f.over(0);
        f.app.world_mut().spawn(Player::at(eye));
        f
    }

    /// Where the Eye goes to aim straight down at the corridor at `x`.
    fn over(&self, x: i32) -> Vec3 {
        let v = self.voxel_m;
        Vec3::new(
            (self.cx + x) as f32 * v + 0.5 * v,
            (Y + 2) as f32 * v,
            (Z as f32 + 0.5) * v,
        )
    }

    fn stand_over(&mut self, x: i32) {
        let eye = self.over(x);
        let mut q = self.app.world_mut().query::<&mut Player>();
        q.single_mut(self.app.world_mut()).unwrap().set_eye(eye);
    }

    /// An intent straight down from over `x`, through the queued-intent path a
    /// Client's click takes.
    fn send(&mut self, x: i32, action: EditAction) {
        self.stand_over(x);
        let intent = EditIntent {
            action,
            origin: self.over(x),
            direction: Vec3::NEG_Y,
        };
        self.app.world_mut().write_message(intent);
        self.app.world_mut().run_system_once(edit::receive).unwrap();
        self.app
            .world_mut()
            .resource_mut::<Messages<EditIntent>>()
            .clear();
    }

    fn rejections(&mut self) -> Vec<RejectReason> {
        self.app
            .world_mut()
            .resource_mut::<Messages<EditRejected>>()
            .drain()
            .map(|m| m.reason)
            .collect()
    }

    fn apply_intents(&mut self) {
        self.app
            .world_mut()
            .run_system_once(edit::apply_pending)
            .unwrap();
    }

    /// One Server tick of what changes the World: the queued intents, then the
    /// simulation, in the order the schedule runs them.
    fn tick(&mut self) {
        self.apply_intents();
        self.app.world_mut().run_system_once(simulate).unwrap();
    }

    fn sim(&self) -> &FluidSim {
        self.app.world().resource::<FluidSim>()
    }

    fn sim_mut(&mut self) -> Mut<'_, FluidSim> {
        self.app.world_mut().resource_mut::<FluidSim>()
    }

    fn wait_if_running(&self) {
        if self.sim().job.in_flight() > 0 {
            std::thread::sleep(Duration::from_micros(50));
        }
    }

    /// Ticks until nothing is queued, waiting, running or landing.
    fn settle(&mut self) {
        let started = Instant::now();
        loop {
            self.tick();
            if self.sim().is_idle() && self.app.world().resource::<PendingEdits>().is_empty() {
                return;
            }
            assert!(started.elapsed() < CEILING, "the simulation never settled");
            self.wait_if_running();
        }
    }

    fn store(&self) -> &ChunkStore {
        self.app.world().resource::<ChunkStore>()
    }

    fn occupants(&self, x: i32) -> Occupants {
        let tables = self.app.world().resource::<ContentTables>();
        self.store()
            .occupants([self.cx + x, Y, Z], &tables.fluids)
            .expect("held")
    }

    fn fluid(&self, x: i32) -> Option<Fluid> {
        self.occupants(x).fluid
    }

    fn wet_length_m(&self) -> f32 {
        let mut x = 1;
        while x <= self.half && self.fluid(x).is_some() {
            x += 1;
        }
        (x - 1) as f32 * self.voxel_m
    }

    fn wet(&self) -> Vec<i32> {
        (-self.half..=self.half)
            .filter(|&x| self.fluid(x).is_some())
            .collect()
    }

    fn state(&self) -> Vec<Occupants> {
        (-self.half - 1..=self.half + 1)
            .map(|x| self.occupants(x))
            .collect()
    }
}

/// The whole cycle through the Server: a fluid-tool intent places a source, the
/// tick runs the kernel on workers until the stream has spread its 7 m, and a
/// second intent takes the source away and everything it fed recedes. At three
/// Voxel sizes, in the same logical time within the quantization.
#[test]
fn a_placed_source_flows_then_recedes_through_the_server_tick() {
    let mut spread_s = Vec::new();
    for v in [1.0, 0.5, 0.25] {
        let mut f = Fixture::new(v, 10.0, 0);
        f.send(0, EditAction::PlaceSource(water()));
        f.tick();
        assert_eq!(f.fluid(0), Some(Fluid::source(water())), "{v} m");
        let start = f.sim().now();
        f.settle();
        spread_s.push((f.sim().now() - start) as f64 * STEP_S);
        assert_eq!(f.wet_length_m(), 7.0, "{v} m");
        assert_eq!(f.wet().len(), 2 * (7.0 / v) as usize + 1, "{v} m");
        let sources = f
            .wet()
            .iter()
            .filter(|&&x| f.fluid(x).unwrap().source)
            .count();
        assert_eq!(sources, 1, "flow became a source at {v} m");
        // Water landed only through the Server's own path: none of it is the
        // generator's packed form, and every flowing Voxel is still air below.
        assert!(
            f.wet()
                .iter()
                .all(|&x| x == 0 || f.occupants(x).solid.is_air())
        );

        f.send(0, EditAction::RemoveSource);
        f.settle();
        assert!(f.wet().is_empty(), "{v} m: {:?}", f.wet());
        assert_eq!(
            f.store().voxel([0, Y - 1, Z]),
            Some(stone()),
            "the bed went with the source"
        );
        assert!(f.rejections().is_empty());
        assert_eq!(f.sim().stale, 0, "nothing raced");
    }
    // 7 m at 4 m/s is 1.75 s; settling is noticed a step or two after the last
    // Voxel lands.
    for seconds in spread_s {
        assert!(
            (1.75..1.75 + 4.0 * STEP_S).contains(&seconds),
            "spread took {seconds} s"
        );
    }
}

/// How finely the quota cuts a step, and how many workers share it, change how
/// many worker calls and ticks a step takes, and neither what lands nor at
/// which logical step, through a whole flow and recession.
#[test]
fn dividing_a_step_changes_nothing_but_the_ticks_taken() {
    let run = |evaluations: usize, jobs: usize, scramble: bool| {
        let mut f = Fixture::new(0.5, 6.0, 0);
        f.sim_mut().set_division(evaluations, jobs);
        f.sim_mut().job().scramble = scramble;
        let mut trace: HashMap<Step, Vec<Occupants>> = HashMap::default();
        let started = Instant::now();
        for action in [EditAction::PlaceSource(water()), EditAction::RemoveSource] {
            f.send(0, action);
            loop {
                f.tick();
                // A step's state is complete once the step after it is current.
                trace.entry(f.sim().now()).or_insert_with(|| f.state());
                if f.sim().is_idle() {
                    break;
                }
                assert!(started.elapsed() < CEILING);
                f.wait_if_running();
            }
        }
        let (calls, _) = f.sim_mut().job().counts();
        (trace, f.sim().work, calls)
    };
    let (whole, work, fast) = run(EVALUATIONS_PER_JOB, JOBS_IN_FLIGHT, false);
    assert!(whole.len() > 100, "{} steps", whole.len());
    for (evaluations, jobs) in [(2, 1), (3, 8)] {
        let (cut, cut_work, slow) = run(evaluations, jobs, true);
        assert!(
            whole == cut,
            "{evaluations} x {jobs} changed what a step landed"
        );
        assert_eq!(work, cut_work);
        assert!(
            slow > fast,
            "the quota did not bind: {slow} vs {fast} calls"
        );
    }
}

/// A held worker read the source and is about to propose its first flow when
/// the player takes the source away. Those proposals are dropped at landing:
/// at no tick after the removal is there water beside where the source was.
#[test]
fn an_older_job_cannot_bring_back_a_removed_source_or_its_flow() {
    let mut f = Fixture::new(1.0, 10.0, 0);
    let placed_at = f.sim().now();
    f.send(0, EditAction::PlaceSource(water()));
    f.tick();
    // Placed on `placed_at`'s tick, first read on the step after, and its
    // neighbours' timers run the sideways delay from there.
    let scale = FlowScale::new(&tables(), 1.0);
    let proposing = placed_at + scale.get(water()).unwrap().sideways.steps();
    let started = Instant::now();
    while f.sim().now() + 1 < proposing || f.sim().job.in_flight() > 0 {
        f.tick();
        assert!(started.elapsed() < CEILING);
        f.wait_if_running();
    }
    assert!(f.fluid(1).is_none(), "flowed early");
    let hold = Arc::clone(&f.sim_mut().job().hold);
    hold.hold();
    while f.sim().job.in_flight() == 0 {
        f.tick();
        assert!(started.elapsed() < CEILING);
    }
    assert_eq!(
        f.sim().now(),
        proposing,
        "the held step is the proposing one"
    );

    f.send(0, EditAction::RemoveSource);
    f.apply_intents();
    assert!(f.fluid(0).is_none(), "removed");
    hold.release();
    while !f.sim().is_idle() {
        f.tick();
        assert!(
            f.fluid(1).is_none() && f.fluid(-1).is_none(),
            "an old step's proposal landed"
        );
        assert!(f.fluid(0).is_none(), "the source came back");
        assert!(started.elapsed() < CEILING);
        f.wait_if_running();
    }
    assert!(f.sim().stale >= 2, "{} stale", f.sim().stale);
    assert!(f.wet().is_empty(), "{:?}", f.wet());
}

/// A stone placed in the stream with the ordinary Edit, whose ray goes through
/// the flowing water to the floor, dries what is downstream of it and leaves
/// the source where it was.
#[test]
fn blocking_a_stream_with_an_edit_keeps_its_source() {
    let mut f = Fixture::new(1.0, 10.0, 0);
    f.send(0, EditAction::PlaceSource(water()));
    f.settle();
    f.send(2, EditAction::Place(stone()));
    f.settle();
    assert_eq!(f.occupants(2).solid, stone());
    assert_eq!(f.fluid(2), None, "the stone displaced the water");
    assert_eq!(f.fluid(0), Some(Fluid::source(water())));
    assert!(f.fluid(1).is_some());
    assert!((3..=8).all(|x| f.fluid(x).is_none()));
    assert!(f.fluid(-7).is_some());
    assert!(f.rejections().is_empty());
}

/// The fluid tool's refusals change neither layer: a removal that strikes no
/// source, and a source of something that does not flow.
#[test]
fn the_fluid_tool_refuses_what_it_cannot_do_and_changes_nothing() {
    let mut f = Fixture::new(1.0, 10.0, 0);
    f.send(0, EditAction::RemoveSource);
    f.send(0, EditAction::PlaceSource(STONE));
    f.settle();
    assert_eq!(
        f.rejections(),
        vec![RejectReason::NoSource, RejectReason::NotAFluid]
    );
    assert!(f.wet().is_empty());
    assert_eq!(f.store().voxel([0, Y - 1, Z]), Some(stone()));

    // On flowing water the removal strikes the floor through it.
    f.send(0, EditAction::PlaceSource(water()));
    f.settle();
    f.send(3, EditAction::RemoveSource);
    f.settle();
    assert_eq!(f.rejections(), vec![RejectReason::NoSource]);
    assert!(f.fluid(3).is_some_and(|w| !w.source));
}

fn fresh_light(store: &ChunkStore, coord: ChunkCoord) -> tesserae_core::ChunkLight {
    let chunk = store.get(coord).unwrap();
    light_cells(chunk, store.overlay(coord).unwrap(), [None; 6], &tables())
}

/// A lantern lights the corridor, all of it in one Chunk with nothing held
/// around it. Water flowing between dims what is beyond it and the light comes
/// back when the water recedes; both ways, what the incremental relight left is
/// what a fresh solve of the final cells gives.
#[test]
fn water_flowing_in_darkens_and_receding_brightens_as_a_fresh_solve_would() {
    let mut f = Fixture::new(1.0, 10.0, 16);
    let home = ChunkCoord::new(0, 0, 0);
    assert_eq!(f.store().coords().collect::<Vec<_>>(), vec![home]);
    let lantern = [16 - 9, Y, Z];
    let solved = fresh_light(f.store(), home);
    f.app
        .world_mut()
        .resource_mut::<ChunkStore>()
        .set_light(home, solved);
    f.app
        .world_mut()
        .resource_scope(|world, mut store: Mut<ChunkStore>| {
            let mut meshes = world.resource_mut::<MeshState>();
            edit::set_voxel(
                &mut store,
                &mut meshes,
                &tables(),
                lantern,
                Voxel::cube(LANTERN),
            )
            .unwrap();
        });
    let block = |f: &Fixture, x: i32| f.store().light_at([16 + x, Y, Z]).block();
    let dry = block(&f, 3);
    assert!(dry > 0, "the lantern reaches x = 3");
    assert!(f.store().light(home) == Some(&fresh_light(f.store(), home)));

    f.send(0, EditAction::PlaceSource(water()));
    f.settle();
    let wet = block(&f, 3);
    assert!(wet < dry, "water between did not dim it: {wet} vs {dry}");
    assert!(
        f.store().light(home) == Some(&fresh_light(f.store(), home)),
        "darkening disagrees with a fresh solve"
    );

    f.send(0, EditAction::RemoveSource);
    f.settle();
    assert_eq!(block(&f, 3), dry, "the light did not come back");
    assert!(
        f.store().light(home) == Some(&fresh_light(f.store(), home)),
        "brightening disagrees with a fresh solve"
    );
}

/// The fluid tool aimed at a stair in the lantern's corridor waterlogs it, and
/// water runs out of its notch along the floor towards the lantern and not
/// through its sealed back the other way; taking the source
/// out of the stair dries it all again and keeps the stair. Light dims and
/// comes back, each time equal to a fresh solve under the shared per-Voxel
/// approximation, which reads the stair's seals and the water's dimming.
#[test]
fn a_waterlogged_stair_feeds_flow_and_light_follows_as_a_fresh_solve_would() {
    let mut f = Fixture::new(1.0, 10.0, 16);
    let home = ChunkCoord::new(0, 0, 0);
    let lantern = [16 - 9, Y, Z];
    let stair = Voxel::shaped(
        STONE,
        SHAPE_STAIR,
        Orientation::new(tesserae_core::mesh::POS_Y, 1),
    );
    let solved = fresh_light(f.store(), home);
    f.app
        .world_mut()
        .resource_mut::<ChunkStore>()
        .set_light(home, solved);
    f.app
        .world_mut()
        .resource_scope(|world, mut store: Mut<ChunkStore>| {
            let mut meshes = world.resource_mut::<MeshState>();
            let tables = tables();
            edit::set_voxel(
                &mut store,
                &mut meshes,
                &tables,
                lantern,
                Voxel::cube(LANTERN),
            )
            .unwrap();
            edit::set_voxel(&mut store, &mut meshes, &tables, [16, Y, Z], stair).unwrap();
        });
    let block = |f: &Fixture, x: i32| f.store().light_at([16 + x, Y, Z]).block();
    let dry = block(&f, -2);
    assert!(f.store().light(home) == Some(&fresh_light(f.store(), home)));

    f.send(0, EditAction::PlaceSource(water()));
    f.settle();
    assert!(f.rejections().is_empty());
    let soaked = f.occupants(0);
    assert_eq!(
        (soaked.solid, soaked.fluid),
        (stair, Some(Fluid::source(water())))
    );
    assert!(f.fluid(-3).is_some(), "{:?}", f.wet());
    assert_eq!(f.fluid(1), None, "through the stair's back");
    assert!(block(&f, -2) < dry, "the water did not dim it");
    assert!(
        f.store().light(home) == Some(&fresh_light(f.store(), home)),
        "darkening disagrees with a fresh solve"
    );

    f.send(0, EditAction::RemoveSource);
    f.settle();
    assert!(f.wet().is_empty(), "{:?}", f.wet());
    assert_eq!(f.occupants(0).solid, stair);
    assert_eq!(block(&f, -2), dry, "the light did not come back");
    assert!(
        f.store().light(home) == Some(&fresh_light(f.store(), home)),
        "brightening disagrees with a fresh solve"
    );
}

/// Through the streaming loop: water stands in a fence's voids beside a
/// stream; building beside the fence grows it an arm, which changes its voids,
/// and both the flow and the mesh the Client holds follow, each time equal to
/// a fresh mesh of the store. The fence's collision and picking read only its
/// packed Voxel, which waterlogging never changes.
#[test]
fn a_connection_change_wakes_passage_and_remeshes_the_water_in_it() {
    let mut h = Harness::new(small(), UNLIMITED);
    h.app.insert_resource(ContentTables(Arc::new(tables())));
    h.settle();
    let rock = ChunkCoord::new(0, -1, 0);
    let (y, z) = (-16, 16);
    let fence = Voxel::new(STONE, tesserae_core::shape::examples::FENCE_ID, 0);
    let run = |h: &mut Harness, change: &dyn Fn(&mut ChunkStore, &mut MeshState, &Tables)| {
        h.app
            .world_mut()
            .resource_scope(|world, mut store: Mut<ChunkStore>| {
                let tables = world.resource::<ContentTables>().0.clone();
                let mut meshes = world.resource_mut::<MeshState>();
                change(&mut store, &mut meshes, &tables);
            });
        let mut held: HashMap<ChunkCoord, tesserae_core::ChunkMesh> = HashMap::default();
        let started = Instant::now();
        loop {
            h.app.world_mut().run_system_once(simulate).unwrap();
            let (ready, _) = h.tick();
            for m in ready {
                held.insert(m.coord, m.mesh);
            }
            if h.app.world().resource::<FluidSim>().is_idle() {
                for m in h.settle().0 {
                    held.insert(m.coord, m.mesh);
                }
                break;
            }
            assert!(started.elapsed() < CEILING);
            std::thread::sleep(Duration::from_micros(50));
        }
        let store = h.store();
        let tables = h.app.world().resource::<ContentTables>().0.clone();
        let neighbors: [Option<LitChunk>; 6] = face_neighbors(rock).map(|n| Some(lit(store, n)));
        let fresh = mesh_chunk(lit(store, rock), neighbors, &tables);
        assert!(
            held.get(&rock).is_some_and(|m| same_mesh(m, &fresh)),
            "the Client holds a stale mesh"
        );
        store
            .occupants([20, y, z], &tables.fluids)
            .unwrap()
            .fluid
            .map(|f| f.levels())
    };
    run(&mut h, &|store, meshes, tables| {
        for x in 1..=31 {
            for (dy, dz) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                edit::set_voxel(store, meshes, tables, [x, y + dy, z + dz], stone()).unwrap();
            }
            let open = (2..=30).contains(&x);
            let voxel = if open { Voxel::AIR } else { stone() };
            edit::set_voxel(store, meshes, tables, [x, y, z], voxel).unwrap();
        }
        edit::set_voxel(store, meshes, tables, [20, y, z], fence).unwrap();
    });
    let source = |store: &mut ChunkStore, meshes: &mut MeshState, tables: &Tables| {
        let occupants = Occupants {
            solid: Voxel::AIR,
            fluid: Some(Fluid::source(water())),
            burning: None,
        };
        set_occupants(store, meshes, tables, [17, y, z], occupants).unwrap();
    };
    run(&mut h, &source).expect("the fence is waterlogged");
    assert_eq!(h.store().voxel([20, y, z]), Some(fence));
    // An arm towards the stone at x = 21 changes the fence's geometry.
    let after = run(&mut h, &|store, meshes, tables| {
        edit::set_voxel(store, meshes, tables, [21, y, z], stone()).unwrap();
    });
    assert!(after.is_some(), "the fence is still waterlogged");
    assert_eq!(h.store().voxel([20, y, z]), Some(fence));
}

/// Through the streaming loop: a stream in a corridor cut in deep rock is
/// meshed, delivered and redelivered as it flows and recedes, and what the
/// Client holds at the end of each is what a fresh mesh of the store makes,
/// water's partial surfaces included.
#[test]
fn full_detail_meshes_follow_the_water_as_it_flows_and_recedes() {
    let mut h = Harness::new(small(), UNLIMITED);
    h.app.insert_resource(ContentTables(Arc::new(tables())));
    h.settle();
    let rock = ChunkCoord::new(0, -1, 0);
    let (y, z) = (-16, 16);
    h.app
        .world_mut()
        .run_system_once(
            move |mut store: ResMut<ChunkStore>,
                  mut meshes: ResMut<MeshState>,
                  tables: Res<ContentTables>| {
                // Sealed first, so a Cave the generator cut nearby cannot drain it.
                for x in 1..=31 {
                    for (dy, dz) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                        let at = [x, y + dy, z + dz];
                        edit::set_voxel(&mut store, &mut meshes, &tables, at, stone()).unwrap();
                    }
                    let open = (2..=30).contains(&x);
                    let voxel = if open { Voxel::AIR } else { stone() };
                    edit::set_voxel(&mut store, &mut meshes, &tables, [x, y, z], voxel).unwrap();
                }
            },
        )
        .unwrap();
    h.settle();

    let source = [16, y, z];
    let mut held: HashMap<ChunkCoord, tesserae_core::ChunkMesh> = HashMap::default();
    // Sets the source Voxel's fluid, runs flow and streaming until both are
    // quiet, and checks what the Client holds against a fresh mesh.
    let mut cycle = |h: &mut Harness, fluid: Option<Fluid>| {
        h.app
            .world_mut()
            .resource_scope(|world, mut store: Mut<ChunkStore>| {
                let mut meshes = world.resource_mut::<MeshState>();
                let occupants = Occupants {
                    solid: Voxel::AIR,
                    fluid,
                    burning: None,
                };
                set_occupants(&mut store, &mut meshes, &tables(), source, occupants).unwrap();
            });
        let started = Instant::now();
        loop {
            h.app.world_mut().run_system_once(simulate).unwrap();
            let (ready, _) = h.tick();
            for m in ready {
                held.insert(m.coord, m.mesh);
            }
            if h.app.world().resource::<FluidSim>().is_idle() {
                for m in h.settle().0 {
                    held.insert(m.coord, m.mesh);
                }
                break;
            }
            assert!(started.elapsed() < CEILING);
            std::thread::sleep(Duration::from_micros(50));
        }
        let store = h.store();
        let neighbors: [Option<LitChunk>; 6] = face_neighbors(rock).map(|n| Some(lit(store, n)));
        let fresh = mesh_chunk(lit(store, rock), neighbors, &tables());
        assert!(
            same_mesh(&held[&rock], &fresh),
            "the Client holds a stale mesh"
        );
        fresh.blended.quad_count()
    };

    let dry = cycle(&mut h, None);
    assert_eq!(dry, 0);
    let wet = cycle(&mut h, Some(Fluid::source(water())));
    assert!(wet > 0, "no water drawn");
    let surfaces: Vec<u8> = (17..=23)
        .map(|x| {
            h.store()
                .occupants([x, y, z], &tables().fluids)
                .unwrap()
                .fluid
                .unwrap()
                .surface
        })
        .collect();
    assert!(surfaces.windows(2).all(|w| w[0] >= w[1]), "{surfaces:?}");
    let gone = cycle(&mut h, None);
    assert_eq!(gone, 0, "water left behind in the mesh");
}

/// How far each arm of the plus runs from the shaft's foot: past water's 7 m.
const ARM: i32 = 9;

/// Where the plus's source sits, five Voxels over the floor of its arms.
const SOURCE: Cell = [0, 6, 0];

/// The open Voxels of the plus: a shaft from `SOURCE` down to the arms, and
/// the arms along +-x and +-z at y = 1.
fn plus_open() -> BTreeSet<Cell> {
    let mut open = BTreeSet::new();
    for i in -ARM..=ARM {
        open.insert([i, 1, 0]);
        open.insert([0, 1, i]);
    }
    for y in 1..=SOURCE[1] {
        open.insert([0, y, 0]);
    }
    open
}

/// The plus at `origin`, walled in stone, as the Chunks a generator would make
/// of it; `source` puts generated water at `SOURCE`. 1 m Voxels.
fn plus_chunks(origin: Cell, source: bool) -> BTreeMap<ChunkCoord, Chunk> {
    let open = plus_open();
    let mut chunks: BTreeMap<ChunkCoord, Chunk> = BTreeMap::new();
    let at = |rel: Cell| [origin[0] + rel[0], origin[1] + rel[1], origin[2] + rel[2]];
    let mut put = |rel: Cell, voxel: Voxel| {
        let v = VoxelCoord::of(at(rel));
        chunks
            .entry(v.chunk)
            .or_insert_with(Chunk::air)
            .set(v.x, v.y, v.z, voxel);
    };
    for o in &open {
        for d in [-1, 0, 1] {
            for e in [-1, 0, 1] {
                for g in [-1, 0, 1] {
                    let c = [o[0] + d, o[1] + e, o[2] + g];
                    if !open.contains(&c) {
                        put(c, stone());
                    }
                }
            }
        }
    }
    if source {
        put(SOURCE, Voxel::cube(water()));
    } else {
        put(SOURCE, Voxel::AIR);
    }
    chunks
}

/// Every open Voxel of the plus and every stone around it.
fn plus_cells() -> BTreeSet<Cell> {
    let open = plus_open();
    let mut all = open.clone();
    for o in &open {
        for d in [-1, 0, 1] {
            for e in [-1, 0, 1] {
                for g in [-1, 0, 1] {
                    all.insert([o[0] + d, o[1] + e, o[2] + g]);
                }
            }
        }
    }
    all
}

/// What is at each of [`plus_cells`] of the plus at `origin`, relative to it;
/// `None` where nothing is held.
type PlusState = Vec<(Cell, Option<Occupants>)>;

fn plus_state(store: &ChunkStore, origin: Cell) -> PlusState {
    let fluids = &WATERED.0.fluids;
    plus_cells()
        .into_iter()
        .map(|rel| {
            let at = [origin[0] + rel[0], origin[1] + rel[1], origin[2] + rel[2]];
            (rel, store.occupants(at, fluids))
        })
        .collect()
}

fn fluid_in(state: &PlusState, rel: Cell) -> Option<Fluid> {
    let i = state
        .binary_search_by_key(&rel, |(c, _)| *c)
        .expect("a cell of the plus");
    state[i].1.and_then(|o| o.fluid)
}

fn wet_cells(state: &PlusState) -> usize {
    state
        .iter()
        .filter(|(_, o)| o.and_then(|o| o.fluid).is_some())
        .count()
}

/// A Server holding nothing but terrain the test admits and evicts, with the
/// simulation ticked alone: no streaming, so a Chunk counts as lit on arrival.
struct Scene {
    app: App,
    origin: Cell,
    generated: BTreeMap<ChunkCoord, Chunk>,
}

/// What a replayed trace does at a logical step. Cells are relative to the
/// plus's origin; a Chunk is named by a cell in it.
#[derive(Clone, Copy, Debug)]
enum Event {
    AdmitAll,
    Admit(Cell),
    Evict(Cell),
    /// A source placed (`true`) or taken away, through the World-change path.
    Source(Cell, bool),
    Solid(Cell, Voxel),
}

impl Scene {
    fn new(origin: Cell, source: bool) -> Self {
        AsyncComputeTaskPool::get_or_init(|| TaskPoolBuilder::new().num_threads(4).build());
        let mut app = App::new();
        app.insert_resource(WorldSettings {
            voxel_size: 1.0,
            ..default()
        })
        .insert_resource(ContentTables(Arc::new(tables())))
        .init_resource::<ChunkStore>()
        .init_resource::<MeshState>()
        .init_resource::<FluidSim>();
        Scene {
            app,
            origin,
            generated: plus_chunks(origin, source),
        }
    }

    fn abs(&self, rel: Cell) -> Cell {
        [
            self.origin[0] + rel[0],
            self.origin[1] + rel[1],
            self.origin[2] + rel[2],
        ]
    }

    fn chunk_at(&self, rel: Cell) -> ChunkCoord {
        VoxelCoord::of(self.abs(rel)).chunk
    }

    fn store(&self) -> &ChunkStore {
        self.app.world().resource::<ChunkStore>()
    }

    fn store_mut(&mut self) -> Mut<'_, ChunkStore> {
        self.app.world_mut().resource_mut::<ChunkStore>()
    }

    fn sim(&self) -> &FluidSim {
        self.app.world().resource::<FluidSim>()
    }

    fn sim_mut(&mut self) -> Mut<'_, FluidSim> {
        self.app.world_mut().resource_mut::<FluidSim>()
    }

    fn admit(&mut self, chunk: ChunkCoord) {
        let fresh = self.generated[&chunk].clone();
        self.store_mut().insert(chunk, fresh);
    }

    fn apply(&mut self, event: Event) {
        match event {
            Event::AdmitAll => {
                let missing: Vec<ChunkCoord> = self
                    .generated
                    .keys()
                    .copied()
                    .filter(|&c| !self.store().contains(c))
                    .collect();
                for c in missing {
                    self.admit(c);
                }
            }
            Event::Admit(rel) => {
                let c = self.chunk_at(rel);
                self.admit(c);
            }
            Event::Evict(rel) => {
                let c = self.chunk_at(rel);
                self.store_mut().remove(c);
            }
            Event::Source(rel, place) => {
                let fluid = place.then_some(Fluid::source(water()));
                self.set(rel, Voxel::AIR, fluid);
            }
            Event::Solid(rel, voxel) => self.set(rel, voxel, None),
        }
    }

    fn set(&mut self, rel: Cell, solid: Voxel, fluid: Option<Fluid>) {
        let at = self.abs(rel);
        self.app
            .world_mut()
            .resource_scope(|world, mut store: Mut<ChunkStore>| {
                let mut meshes = world.resource_mut::<MeshState>();
                let occupants = Occupants {
                    solid,
                    fluid,
                    burning: None,
                };
                set_occupants(&mut store, &mut meshes, &WATERED.0, at, occupants)
                    .expect("a valid change to held terrain");
            });
    }

    fn tick(&mut self) {
        self.app.world_mut().run_system_once(simulate).unwrap();
    }

    fn wait_if_running(&self) {
        if self.sim().job.in_flight() > 0 {
            std::thread::sleep(Duration::from_micros(50));
        }
    }

    fn occupants(&self, rel: Cell) -> Option<Occupants> {
        self.store().occupants(self.abs(rel), &WATERED.0.fluids)
    }

    fn fluid(&self, rel: Cell) -> Option<Fluid> {
        self.occupants(rel).and_then(|o| o.fluid)
    }

    fn state(&self) -> PlusState {
        plus_state(self.store(), self.origin)
    }

    fn settle(&mut self) {
        let started = Instant::now();
        loop {
            self.tick();
            if self.sim().is_idle() {
                return;
            }
            assert!(started.elapsed() < CEILING, "the simulation never settled");
            self.wait_if_running();
        }
    }
}

/// How a replay divides its steps, and its bounds.
#[derive(Clone, Copy)]
struct Division {
    evaluations: usize,
    jobs: usize,
    scramble: bool,
    /// Holds a step's results back for a few ticks when an event is due, and
    /// applies the event at one of them.
    jitter: bool,
    /// A pool of its own with this many threads.
    threads: Option<usize>,
    bounds: Option<(Limits, usize)>,
}

const WHOLE: Division = Division {
    evaluations: EVALUATIONS_PER_JOB,
    jobs: JOBS_IN_FLIGHT,
    scramble: false,
    jitter: false,
    threads: None,
    bounds: None,
};

struct Replay {
    /// The state at the start of every step, once the step before it landed.
    trace: BTreeMap<Step, PlusState>,
    work: Work,
    counts: [usize; 5],
    calls: usize,
    overflowed: usize,
}

/// Plays `events`, each when its step is current, until the simulation is idle
/// after the last of them.
fn replay(origin: Cell, source: bool, events: &[(Step, Event)], division: Division) -> Replay {
    let mut scene = Scene::new(origin, source);
    {
        let mut sim = scene.sim_mut();
        sim.set_division(division.evaluations, division.jobs);
        if let Some((limits, landed)) = division.bounds {
            sim.set_bounds(limits, landed);
        }
        sim.job().scramble = division.scramble;
        sim.job().pool = division
            .threads
            .map(|n| Arc::new(TaskPoolBuilder::new().num_threads(n).build()));
    }
    let mut trace = BTreeMap::new();
    let mut next = 0;
    let mut dice: u64 = 0x9e37_79b9;
    let started = Instant::now();
    loop {
        let now = scene.sim().now();
        if let Some(&(step, _)) = events.get(next) {
            assert!(step >= now, "step {step} went by unplayed");
            if step == now {
                if division.jitter && scene.sim().running() {
                    dice = dice.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
                    let held = (dice >> 33) % 4;
                    scene.sim_mut().stall = held as usize;
                    for _ in 0..(dice >> 40) % (held + 1) {
                        scene.tick();
                    }
                    assert_eq!(scene.sim().now(), now, "a stalled step landed");
                }
                while let Some(&(step, event)) = events.get(next)
                    && step == now
                {
                    scene.apply(event);
                    next += 1;
                }
            }
        }
        scene.tick();
        let now = scene.sim().now();
        trace.entry(now).or_insert_with(|| scene.state());
        if next == events.len() && scene.sim().is_idle() {
            break;
        }
        assert!(started.elapsed() < CEILING, "the replay never finished");
        scene.wait_if_running();
    }
    let sim = scene.sim();
    let counts = [sim.landed, sim.stale, sim.deferred, sim.parked, sim.scanned];
    let (calls, _) = scene.sim_mut().job().counts();
    Replay {
        trace,
        work: scene.sim().work,
        counts,
        calls,
        overflowed: scene.sim().agenda().overflowed(),
    }
}

/// Wholly inside Chunk (0, 0, 0).
const INSIDE: Cell = [16, 8, 16];
/// The shaft crosses y = 0 falling, the +x arm crosses x = 0 and the -z arm
/// crosses z = -33, all at negative coordinates.
const STRADDLING: Cell = [-3, -3, -29];
/// The -x arm crosses x = -33 and the +z arm crosses z = 0.
const STRADDLING_OTHER: Cell = [-30, -20, -3];

/// Water placed into the plus flows and recedes the same, step by step,
/// whether it lies inside one Chunk or straddles their faces. Between them
/// the placements cross faces in +x, -x, +z, -z and falling through -y: every
/// face of a Chunk is entered or left. Flow never climbs, so no stream enters
/// a Chunk through its bottom face.
#[test]
fn flow_crosses_every_chunk_face_as_if_there_were_none() {
    let events = [
        (0, Event::AdmitAll),
        (2, Event::Source(SOURCE, true)),
        (300, Event::Source(SOURCE, false)),
    ];
    let inside = replay(INSIDE, false, &events, WHOLE);
    assert_eq!(Scene::new(INSIDE, false).generated.len(), 1);
    let before_removal = &inside.trace[&299];
    let crossings: [(Cell, Cell, Cell); 5] = [
        (STRADDLING, [2, 1, 0], [3, 1, 0]),
        (STRADDLING, [0, 1, -3], [0, 1, -4]),
        (STRADDLING, [0, 3, 0], [0, 2, 0]),
        (STRADDLING_OTHER, [-2, 1, 0], [-3, 1, 0]),
        (STRADDLING_OTHER, [0, 1, 2], [0, 1, 3]),
    ];
    for (origin, from, to) in crossings {
        let scene = Scene::new(origin, false);
        assert_ne!(
            scene.chunk_at(from),
            scene.chunk_at(to),
            "{from:?} -> {to:?}"
        );
        for rel in [from, to] {
            assert!(fluid_in(before_removal, rel).is_some(), "{rel:?} is dry");
        }
    }
    assert_eq!(Scene::new(STRADDLING, false).generated.len(), 4);
    for origin in [STRADDLING, STRADDLING_OTHER] {
        let straddling = replay(origin, false, &events, WHOLE);
        assert!(
            straddling.trace == inside.trace,
            "{origin:?} differs from one Chunk"
        );
        assert_eq!(straddling.work, inside.work);
    }
    assert_eq!(wet_cells(inside.trace.values().last().unwrap()), 0);
}

/// Terrain nobody holds is unknown, never air: flow stops short of it and does
/// not fall into it. Admitting the rest, in any order and at any time, wakes
/// both sides and leaves no seam: generated water settles as if every Chunk
/// had been there from the start, and the simulation loads nothing itself.
#[test]
fn missing_terrain_is_unknown_and_any_admission_order_leaves_no_seam() {
    let scene = Scene::new(STRADDLING, true);
    let top = scene.chunk_at(SOURCE);
    let middle = scene.chunk_at([0, 1, 0]);
    let east = scene.chunk_at([ARM, 1, 0]);
    let north = scene.chunk_at([0, 1, -ARM]);
    assert_eq!(
        BTreeSet::from([top, middle, east, north]),
        scene.generated.keys().copied().collect()
    );

    let all_at_once = replay(STRADDLING, true, &[(0, Event::AdmitAll)], WHOLE);
    let settled = all_at_once.trace.values().last().unwrap().clone();
    assert!(wet_cells(&settled) > 20);

    // The source's Chunk alone, then the one under it, then the arms.
    let mut s = Scene::new(STRADDLING, true);
    s.apply(Event::Admit(SOURCE));
    s.settle();
    assert!(
        s.fluid([0, 3, 0]).is_some(),
        "the shaft above the gap filled"
    );
    assert!(
        s.occupants([0, 2, 0]).is_none(),
        "nothing put the Chunk below"
    );
    s.apply(Event::Admit([0, 1, 0]));
    s.settle();
    assert!(s.fluid([0, 2, 0]).is_some(), "admission woke the shaft");
    assert!(s.fluid([1, 1, 0]).is_some());
    assert!(
        s.fluid([2, 1, 0]).is_none(),
        "flowed beside unknown terrain as though it were air"
    );
    assert!(s.fluid([0, 1, -2]).is_some());
    assert!(s.fluid([0, 1, -3]).is_none());
    assert_eq!(s.store().coords().count(), 2, "flow loaded terrain");
    s.apply(Event::Admit([ARM, 1, 0]));
    s.apply(Event::Admit([0, 1, -ARM]));
    s.settle();
    assert!(s.state() == settled, "admission order left a seam");

    let orders: [&[(Step, Event)]; 2] = [
        &[
            (0, Event::Admit([0, 1, -ARM])),
            (10, Event::Admit([ARM, 1, 0])),
            (20, Event::Admit([0, 1, 0])),
            (30, Event::Admit(SOURCE)),
        ],
        &[
            (0, Event::Admit(SOURCE)),
            (40, Event::Admit([0, 1, 0])),
            (90, Event::Admit([0, 1, -ARM])),
            (250, Event::Admit([ARM, 1, 0])),
        ],
    ];
    for order in orders {
        let late = replay(STRADDLING, true, order, WHOLE);
        assert!(
            late.trace.values().last().unwrap() == &settled,
            "{order:?} left a seam"
        );
    }
}

/// Eviction drops a Chunk's water and work with it; what read it pauses, and
/// keeps no timer decided from it. Generating it again brings its generated
/// source back, under the same rules as solid Edits: the removal is undone.
#[test]
fn eviction_drops_a_chunks_work_and_regeneration_brings_its_source_back() {
    let mut s = Scene::new(STRADDLING, true);
    s.apply(Event::AdmitAll);
    s.settle();
    let settled = s.state();
    let east = s.chunk_at([ARM, 1, 0]);
    assert!(s.fluid([4, 1, 0]).is_some(), "the east arm is wet");

    s.apply(Event::Evict([ARM, 1, 0]));
    s.tick();
    assert!(!s.sim().is_simulated(east));
    s.settle();
    assert_eq!(s.sim().timers(), 0, "a timer outlived what it read");
    assert!(
        s.fluid([2, 1, 0]).is_some(),
        "the Voxel beside the gap is paused, not dried as if beside air"
    );
    s.apply(Event::Admit([ARM, 1, 0]));
    s.settle();
    assert!(s.state() == settled, "the east arm did not refill");

    s.apply(Event::Source(SOURCE, false));
    s.settle();
    assert_eq!(wet_cells(&s.state()), 0);
    s.apply(Event::Evict(SOURCE));
    s.tick();
    s.apply(Event::Admit(SOURCE));
    s.settle();
    assert_eq!(s.fluid(SOURCE), Some(Fluid::source(water())));
    assert!(s.state() == settled, "the regenerated source did not flow");
}

/// A step that read the east Chunk is held while that Chunk is evicted and a
/// fresh one put at its coordinate. Nothing the step computed from the old one
/// lands in the new one; the stream then enters it as it would have anyway.
#[test]
fn a_new_chunk_at_an_old_coordinate_rejects_work_for_the_old_one() {
    let events = [(0, Event::AdmitAll), (2, Event::Source(SOURCE, true))];
    let reference = replay(STRADDLING, false, &events, WHOLE);
    let scene = Scene::new(STRADDLING, false);
    let east = scene.chunk_at([ARM, 1, 0]);
    // The step that first wets the east Chunk, at x = 3.
    let enters = *reference
        .trace
        .iter()
        .find(|(_, state)| fluid_in(state, [3, 1, 0]).is_some())
        .unwrap()
        .0
        - 1;

    let mut s = scene;
    s.apply(Event::AdmitAll);
    let started = Instant::now();
    while s.sim().now() < 2 {
        s.tick();
    }
    s.apply(Event::Source(SOURCE, true));
    while !(s.sim().now() == enters && s.sim().running()) {
        s.tick();
        assert!(s.sim().now() <= enters);
        assert!(started.elapsed() < CEILING);
        s.wait_if_running();
    }
    s.sim_mut().stall = 3;
    let fresh = s.generated[&east].clone();
    s.store_mut().remove(east);
    s.store_mut().insert(east, fresh);
    let stale = s.sim().stale;
    while s.sim().now() == enters {
        s.tick();
        assert!(started.elapsed() < CEILING);
        s.wait_if_running();
    }
    assert!(s.sim().stale > stale, "the old step's proposals landed");
    assert!(s.fluid([3, 1, 0]).is_none(), "old-lifetime water landed");
    s.settle();
    assert!(s.state() == *reference.trace.values().last().unwrap());
}

/// Equal events at equal logical steps give equal state at every step, however
/// a step is divided among workers, however many threads run them, in
/// whatever order they finish, with each run's hash maps keyed afresh, and
/// whenever within its step an event arrives. Holds as well when the
/// simulation's own bounds are saturated.
#[test]
fn equal_step_indexed_events_give_equal_state_however_work_is_divided() {
    let events = [
        (0, Event::Admit(SOURCE)),
        (0, Event::Admit([0, 1, 0])),
        (0, Event::Admit([ARM, 1, 0])),
        (2, Event::Source(SOURCE, true)),
        (50, Event::Admit([0, 1, -ARM])),
        (90, Event::Solid([4, 1, 0], stone())),
        (140, Event::Evict([ARM, 1, 0])),
        (180, Event::Admit([ARM, 1, 0])),
        (230, Event::Source(SOURCE, false)),
        (240, Event::Source([0, 1, -5], true)),
        (300, Event::Solid([4, 1, 0], Voxel::AIR)),
    ];
    let divisions = [
        Division {
            evaluations: 3,
            jobs: 1,
            scramble: true,
            jitter: true,
            threads: Some(1),
            ..WHOLE
        },
        Division {
            evaluations: 7,
            jobs: 8,
            scramble: true,
            jitter: true,
            threads: Some(4),
            ..WHOLE
        },
        Division {
            evaluations: 1,
            jobs: 2,
            jitter: true,
            ..WHOLE
        },
    ];
    let saturated = Some((
        Limits {
            capacity: 12,
            reserve: 4,
            scan_per_step: 8,
        },
        2,
    ));
    for bounds in [None, saturated] {
        let whole = replay(STRADDLING, false, &events, Division { bounds, ..WHOLE });
        assert!(whole.trace.len() > 300);
        if bounds.is_some() {
            assert!(whole.overflowed > 0, "the agenda bound never bit");
            assert!(whole.counts[2] > 0, "the landing bound never bit");
        }
        for division in divisions {
            let divided = replay(STRADDLING, false, &events, Division { bounds, ..division });
            assert!(
                divided.trace == whole.trace,
                "{} x {} diverged",
                division.evaluations,
                division.jobs
            );
            assert_eq!(divided.work, whole.work);
            assert_eq!(divided.counts, whole.counts);
            assert!(divided.calls > whole.calls);
        }
    }
}

/// Settled water and dry terrain cost nothing per step. Dry Chunks arrive
/// without a single evaluation; a uniform lake beside rock is looked at once
/// along its faces and then sleeps; a stream, once settled, is never looked at
/// again however long the Server runs.
#[test]
fn settled_water_and_dry_terrain_cost_nothing_per_step() {
    let mut dry = Scene::new(INSIDE, false);
    for x in -1..=1 {
        dry.store_mut()
            .insert(ChunkCoord::new(x, 1, 0), Chunk::filled(stone()));
    }
    dry.apply(Event::AdmitAll);
    for _ in 0..50 {
        dry.tick();
    }
    assert_eq!(dry.sim().work, Work::default());
    assert_eq!(dry.sim().scanned, 0);

    dry.store_mut().insert(
        ChunkCoord::new(5, 0, 0),
        Chunk::filled(Voxel::cube(water())),
    );
    dry.store_mut()
        .insert(ChunkCoord::new(6, 0, 0), Chunk::filled(stone()));
    dry.settle();
    let once = dry.sim().scanned;
    assert!(once > 0 && once <= 7 * 32 * 32 * 6, "{once}");
    assert_eq!(dry.sim().landed, 0);

    let mut s = Scene::new(STRADDLING, true);
    s.apply(Event::AdmitAll);
    s.settle();
    assert!(s.sim().landed > 0);
    let (work, scanned) = (s.sim().work, s.sim().scanned);
    for _ in 0..500 {
        s.tick();
    }
    assert_eq!(s.sim().work, work, "settled water was evaluated again");
    assert_eq!(s.sim().scanned, scanned);
    assert!(s.sim().is_idle() && s.sim().timers() == 0);
}

/// Three plus-shaped streams and the player's corridor flow at once against
/// bounds far too small for them: the agenda overflows into rescans and every
/// step has more changes than it may land. Each stream still keeps changing
/// until it settles where it would have unbounded; the player's Edits apply on
/// the tick they are made; and two Mods' queues drain at the phase 7 rate of
/// two Voxels a tick, taking turns.
#[test]
fn under_saturation_every_chunk_player_edits_and_mod_queues_keep_moving() {
    let reference = {
        let mut s = Scene::new(INSIDE, false);
        s.apply(Event::AdmitAll);
        s.tick();
        s.apply(Event::Source(SOURCE, true));
        s.settle();
        s.state()
    };
    let corridor = {
        let mut f = Fixture::new(1.0, 10.0, 0);
        f.send(0, EditAction::PlaceSource(water()));
        f.send(3, EditAction::Place(stone()));
        f.settle();
        f.state()
    };

    let mut f = Fixture::new(1.0, 10.0, 0);
    let limits = Limits {
        capacity: 16,
        reserve: 8,
        scan_per_step: 16,
    };
    let landed_per_step = 3;
    f.sim_mut().set_bounds(limits, landed_per_step);
    f.app.init_resource::<PendingCommands>();
    let origins = [[80, 8, 16], [144, 8, 16], [16, 8, 80]];
    for origin in origins {
        for (coord, chunk) in plus_chunks(origin, false) {
            f.app
                .world_mut()
                .resource_mut::<ChunkStore>()
                .insert(coord, chunk);
        }
    }
    f.tick();
    f.app
        .world_mut()
        .resource_scope(|world, mut store: Mut<ChunkStore>| {
            let mut meshes = world.resource_mut::<MeshState>();
            for o in origins {
                let at = [o[0] + SOURCE[0], o[1] + SOURCE[1], o[2] + SOURCE[2]];
                let occupants = Occupants {
                    solid: Voxel::AIR,
                    fluid: Some(Fluid::source(water())),
                    burning: None,
                };
                set_occupants(&mut store, &mut meshes, &WATERED.0, at, occupants).unwrap();
            }
        });
    f.send(0, EditAction::PlaceSource(water()));
    {
        let mut queues = f.app.world_mut().resource_mut::<PendingCommands>();
        for (index, x) in [(0, 70), (1, 150)] {
            let places = (0..40)
                .map(|i| ([x + i % 8, 30, 10 + i / 8], stone()))
                .collect();
            queues.push_places(index, places);
        }
    }

    let areas = |f: &Fixture| -> Vec<Vec<Option<Occupants>>> {
        let mut all: Vec<Vec<Option<Occupants>>> = origins
            .iter()
            .map(|&o| {
                plus_state(f.store(), o)
                    .into_iter()
                    .map(|(_, o)| o)
                    .collect()
            })
            .collect();
        all.push(f.state().into_iter().map(Some).collect());
        all
    };
    let finals: Vec<Vec<Option<Occupants>>> =
        std::iter::repeat_n(reference.into_iter().map(|(_, o)| o).collect(), 3)
            .chain([corridor.iter().copied().map(Some).collect()])
            .collect();
    let mut last = areas(&f);
    let mut changed_at = vec![f.sim().now(); last.len()];
    let mut longest = 0;
    let mut applied = [0u32; 2];
    let mut blocked = false;
    let started = Instant::now();
    loop {
        let (step, landed) = (f.sim().now(), f.sim().landed);
        if step == 60 && !blocked {
            f.send(3, EditAction::Place(stone()));
            f.apply_intents();
            assert_eq!(f.occupants(3).solid, stone(), "the Edit waited on flow");
            blocked = true;
        }
        f.apply_intents();
        f.app
            .world_mut()
            .run_system_once(mod_runtime::drain)
            .unwrap();
        f.app.world_mut().run_system_once(simulate).unwrap();

        let queues = f.app.world().resource::<PendingCommands>();
        let now = [queues.applied(0), queues.applied(1)];
        if applied[1] < 40 {
            assert_eq!(
                (now[0] - applied[0], now[1] - applied[1]),
                (1, 1),
                "the Mod queues did not take turns at two Voxels a tick"
            );
        }
        applied = now;
        let sim = f.sim();
        assert!(sim.agenda().len() <= limits.capacity + limits.reserve);
        assert!(sim.job.in_flight() <= JOBS_IN_FLIGHT);
        assert!(sim.landed - landed <= landed_per_step);
        let state = areas(&f);
        if sim.now() != step {
            for (i, area) in state.iter().enumerate() {
                if *area != last[i] {
                    changed_at[i] = sim.now();
                } else if *area != finals[i] {
                    longest = longest.max(sim.now() - changed_at[i]);
                }
            }
            last = state;
        }
        if blocked && f.sim().is_idle() {
            break;
        }
        assert!(started.elapsed() < CEILING, "never settled");
        f.wait_if_running();
    }
    assert!(f.rejections().is_empty());
    assert!(
        f.sim().agenda().overflowed() > 0,
        "the agenda bound never bit"
    );
    assert!(f.sim().deferred > 0, "the landing bound never bit");
    for (i, (area, end)) in last.iter().zip(&finals).enumerate() {
        assert!(area == end, "area {i} settled elsewhere than unbounded");
    }
    assert!(
        longest <= 64,
        "an unsettled area went {longest} steps unchanged"
    );
}

/// While a simulated Chunk's light is being solved again nothing lands in it:
/// the changes due there sleep, parked, and are not evaluated again every step
/// meanwhile. Once its light lands they are rescanned, and the stream settles
/// where it would have.
#[test]
fn nothing_lands_in_a_chunk_while_its_light_is_solved() {
    let events = [(0, Event::AdmitAll), (2, Event::Source(SOURCE, true))];
    let reference = replay(STRADDLING, false, &events, WHOLE);
    let mut s = Scene::new(STRADDLING, false);
    let east = s.chunk_at([ARM, 1, 0]);
    s.apply(Event::AdmitAll);
    while s.sim().now() < 2 {
        s.tick();
    }
    s.apply(Event::Source(SOURCE, true));
    s.app
        .world_mut()
        .resource_mut::<MeshState>()
        .mark_unlit(east);
    s.settle();
    assert!(s.fluid([2, 1, 0]).is_some());
    assert!(s.fluid([3, 1, 0]).is_none(), "landed in a Chunk being lit");
    assert!(s.sim().parked > 0);
    let work = s.sim().work;
    for _ in 0..300 {
        s.tick();
    }
    assert_eq!(s.sim().work, work, "parked changes were evaluated again");

    s.app.world_mut().resource_mut::<MeshState>().mark_lit(east);
    s.settle();
    assert!(s.state() == *reference.trace.values().last().unwrap());
}

/// Past the landing bound each Chunk with changes lands one in turn, and the
/// next step carries on after the last Chunk served and, inside each, after
/// the last change it landed: under saturation no Chunk or Voxel waits behind
/// another for ever.
#[test]
fn landing_past_its_bound_takes_turns_round_chunks_and_voxels() {
    let mut sim = FluidSim {
        landed_per_step: 3,
        ..default()
    };
    let dry = Occupants {
        solid: Voxel::AIR,
        fluid: None,
        burning: None,
    };
    let wet = Occupants {
        fluid: Some(Fluid {
            source: false,
            ..Fluid::source(water())
        }),
        ..dry
    };
    let all: Vec<Proposal> = (0..5)
        .flat_map(|c| {
            (0..4).map(move |i| Proposal {
                at: [-64 + c * 32 + i, 0, 0],
                read: dry,
                next: wet,
                contact: None,
                heated: false,
            })
        })
        .collect();
    let mut served: BTreeMap<[i32; 3], Vec<Cell>> = BTreeMap::new();
    for call in 0..7 {
        let (chosen, deferred) = choose(&mut sim, &WATERED.0, all.clone());
        assert_eq!((chosen.len(), deferred.len()), (3, 17));
        let chunks: BTreeSet<[i32; 3]> = chosen.iter().map(|p| chunk_of(p.at).0).collect();
        assert_eq!(chunks.len(), 3, "a Chunk landed twice before all had once");
        for p in chosen {
            served.entry(chunk_of(p.at).0).or_default().push(p.at);
        }
        if call == 4 {
            for cells in served.values() {
                let along: Vec<i32> = cells.iter().map(|c| c[0].rem_euclid(32)).collect();
                assert_eq!(
                    along,
                    vec![0, 1, 2],
                    "each Chunk in turn, each Voxel in turn"
                );
            }
        }
    }
    for cells in served.values() {
        let distinct: BTreeSet<Cell> = cells.iter().copied().collect();
        assert_eq!(distinct.len(), 4, "a Voxel was passed over: {cells:?}");
    }
}

/// Changes that start or stop block light have a bound of their own: past
/// it the rest wait their turn while changes that move no light, in the same
/// Chunk or another, keep landing, and every one lands in a later step.
#[test]
fn emitters_past_their_bound_wait_while_other_changes_land() {
    let mut sim = FluidSim::default();
    sim.set_emitter_bound(2);
    let dry = Occupants {
        solid: Voxel::AIR,
        fluid: None,
        burning: None,
    };
    let wet = Occupants {
        fluid: Some(Fluid {
            source: false,
            ..Fluid::source(water())
        }),
        ..dry
    };
    let lamp = Occupants {
        solid: Voxel::cube(LANTERN),
        ..dry
    };
    let change = |at: Cell, next: Occupants| Proposal {
        at,
        read: dry,
        next,
        contact: None,
        heated: false,
    };
    let mut pending: Vec<Proposal> = (0..5)
        .map(|i| change([i, 0, 0], lamp))
        .chain((5..8).map(|i| change([i, 0, 0], wet)))
        .chain((0..3).map(|i| change([40 + i, 0, 0], wet)))
        .collect();
    let mut lamps_per_step = Vec::new();
    for step in 0.. {
        let (chosen, deferred) = choose(&mut sim, &WATERED.0, pending);
        if step == 0 {
            assert_eq!(
                chosen.iter().filter(|p| p.next == wet).count(),
                6,
                "water waited behind a lamp"
            );
        }
        lamps_per_step.push(chosen.iter().filter(|p| p.next == lamp).count());
        pending = deferred;
        if pending.is_empty() {
            break;
        }
    }
    assert_eq!(lamps_per_step, vec![2, 2, 1]);
}

/// A tick that changes more positions than the journal holds, as a `dev`
/// builder may, coalesces the rest by Chunk rather than dropping them: the
/// source removed last, among thousands of other changes, still dries its
/// stream.
#[test]
fn a_journal_overflow_coalesces_by_chunk_and_drops_no_change() {
    let mut s = Scene::new(STRADDLING, false);
    s.apply(Event::AdmitAll);
    while s.sim().now() < 2 {
        s.tick();
    }
    s.apply(Event::Source(SOURCE, true));
    s.settle();
    assert!(wet_cells(&s.state()) > 20);
    let north = s.chunk_at([0, 1, -ARM]);
    let base = [north.x * 32, north.y * 32, north.z * 32];
    // More than the journal's 4096, in a corner of the north Chunk well away
    // from the arm.
    let fill: Vec<Cell> = (0..16)
        .flat_map(|x| (0..16).flat_map(move |y| (0..17).map(move |z| [x, y, z])))
        .map(|d| [base[0] + d[0], base[1] + d[1], base[2] + d[2]])
        .collect();
    assert!(fill.len() > 4096);
    s.app
        .world_mut()
        .resource_scope(|world, mut store: Mut<ChunkStore>| {
            let mut meshes = world.resource_mut::<MeshState>();
            for &at in &fill {
                edit::set_voxel(&mut store, &mut meshes, &WATERED.0, at, stone()).unwrap();
            }
        });
    s.apply(Event::Source(SOURCE, false));
    s.settle();
    assert_eq!(wet_cells(&s.state()), 0, "the source's removal was lost");
    assert_eq!(s.store().voxel(fill[4000]), Some(stone()));
}

/// Ticks until `seconds` of logical time have passed.
fn run_seconds(f: &mut Fixture, seconds: f64) {
    let until = f.sim().now() + (seconds / STEP_S).round() as Step;
    let started = Instant::now();
    while f.sim().now() < until {
        f.tick();
        assert!(started.elapsed() < CEILING);
        f.wait_if_running();
    }
}

/// Water and lava from the demo Game and slime from an independent Mod,
/// placed with the fluid tool, flow by their own declared metres through the
/// same Server tick: after a second each front is where its rate puts it, each settles
/// at its own reach, and each recedes. Lava is the slowest of the three.
#[test]
fn declared_fluids_flow_by_their_own_rows_through_the_server_tick() {
    for v in [1.0, 0.5, 0.25] {
        let loaded = mod_runtime::tests::demo_and_fluids(v);
        let id = |name| {
            loaded
                .registry
                .id_of(tesserae_mods::Kind::Material, name)
                .unwrap() as u16
        };
        let (lava, slime) = (id("demo:lava"), id("fluids:slime"));
        for (material, after_1_s, reach) in [
            (id("demo:water"), 4.0, 7.0),
            (lava, 0.75, 3.0),
            (slime, 2.0, 2.0),
        ] {
            let mut f = Fixture::with_tables(v, 8.0, 0, loaded.tables.clone());
            f.send(0, EditAction::PlaceSource(material));
            f.tick();
            assert_eq!(f.fluid(0), Some(Fluid::source(material)), "{v} m");
            run_seconds(&mut f, 1.0);
            let front = f.wet_length_m();
            assert!(
                (front - after_1_s).abs() <= v + 1e-4,
                "{v} m, {material}: {front} m after 1 s"
            );
            f.settle();
            assert_eq!(f.wet_length_m(), reach, "{v} m, {material}");
            assert!(
                f.wet()
                    .iter()
                    .all(|&x| f.fluid(x).unwrap().material == material
                        && (x == 0) == f.fluid(x).unwrap().source)
            );
            f.send(0, EditAction::RemoveSource);
            f.settle();
            assert!(f.wet().is_empty(), "{v} m, {material}: {:?}", f.wet());
            assert!(f.rejections().is_empty());
        }
    }
}

/// Lava lights a dark corridor as it flows in and the light goes as it
/// recedes, each time what a fresh solve of the final cells gives.
#[test]
fn lava_lights_the_dark_as_it_flows_and_the_light_goes_with_it() {
    let loaded = mod_runtime::tests::demo_and_fluids(1.0);
    let lava = loaded
        .registry
        .id_of(tesserae_mods::Kind::Material, "demo:lava")
        .unwrap() as u16;
    let tables = loaded.tables;
    let mut f = Fixture::with_tables(1.0, 10.0, 16, tables.clone());
    let home = ChunkCoord::new(0, 0, 0);
    let fresh = |f: &Fixture| {
        light_cells(
            f.store().get(home).unwrap(),
            f.store().overlay(home).unwrap(),
            [None; 6],
            &tables,
        )
    };
    let solved = fresh(&f);
    f.app
        .world_mut()
        .resource_mut::<ChunkStore>()
        .set_light(home, solved);
    let block = |f: &Fixture, x: i32| f.store().light_at([16 + x, Y, Z]).block();
    assert_eq!(block(&f, 5), 0, "the corridor starts dark");
    f.send(0, EditAction::PlaceSource(lava));
    f.settle();
    assert!(block(&f, 5) > 0, "lava 3 m away did not light x = 5");
    assert!(f.store().light(home) == Some(&fresh(&f)));
    f.send(0, EditAction::RemoveSource);
    f.settle();
    assert_eq!(block(&f, 5), 0);
    assert!(f.store().light(home) == Some(&fresh(&f)));
}

/// A source from the player's fluid tool, from a 0.1 Mod placing a water
/// Voxel and from a 0.2 Mod's source Command lands the same and flows the
/// same: all three reach the one World-change path.
#[test]
fn a_source_from_a_player_or_either_contract_line_flows_the_same() {
    use tesserae_mods::{Command, Line, Placement, Region};
    let states: Vec<Vec<Occupants>> = (0..3)
        .map(|way| {
            let mut f = Fixture::new(0.5, 6.0, 0);
            let at = [0.0, Y as f32 * 0.5, Z as f32 * 0.5];
            match way {
                0 => f.send(0, EditAction::PlaceSource(water())),
                1 => mod_runtime::tests::drain_validated(
                    &mut f.app,
                    Line::V01,
                    &[Command::Place(vec![Placement {
                        at,
                        voxel: Voxel::cube(water()),
                    }])],
                ),
                _ => mod_runtime::tests::drain_validated(
                    &mut f.app,
                    Line::V02,
                    &[Command::PlaceSources {
                        region: Region {
                            min: at,
                            max: at.map(|m| m + 0.5),
                        },
                        material: water(),
                    }],
                ),
            }
            f.tick();
            assert_eq!(f.fluid(0), Some(Fluid::source(water())), "way {way}");
            f.settle();
            f.state()
        })
        .collect();
    assert!(states[0] == states[1] && states[1] == states[2]);
}

/// The demo Game's and the `fluids` guest's content, from their real WASI
/// components, and a Material by full name.
fn declared(v: f32) -> (Tables, impl Fn(&str) -> u16) {
    let loaded = mod_runtime::tests::demo_and_fluids(v);
    let registry = loaded.registry;
    let id = move |name: &str| registry.id_of(tesserae_mods::Kind::Material, name).unwrap() as u16;
    (loaded.tables, id)
}

/// Whether two face neighbours along the corridor hold `a` and `b`.
fn touching(f: &Fixture, a: MaterialId, b: MaterialId) -> bool {
    let at = |x| f.fluid(x).map(|w| w.material);
    (-f.half..f.half).any(|x| {
        let pair = (at(x), at(x + 1));
        pair == (Some(a), Some(b)) || pair == (Some(b), Some(a))
    })
}

/// A lava source and a water source placed on one tick either side of the
/// face between Chunks -1 and 0: the demo's declared rule turns the lava to
/// stone on the next step and keeps the water, at every Voxel size, and the
/// change reaches the level-1 cover of the Chunk it landed in.
#[test]
fn lava_touching_water_across_a_chunk_face_cools_to_stone_at_every_size() {
    for v in [1.0, 0.5, 0.25] {
        let (tables, id) = declared(v);
        let water = id("demo:water");
        let (lava, stone_id) = (id("demo:lava"), id("core:stone"));
        assert_eq!(stone_id, STONE);
        let mut f = Fixture::with_tables(v, 4.0, 0, tables);
        f.send(-1, EditAction::PlaceSource(lava));
        f.send(0, EditAction::PlaceSource(water));
        f.apply_intents();
        assert_eq!(f.fluid(-1), Some(Fluid::source(lava)));
        assert_eq!(f.fluid(0), Some(Fluid::source(water)));
        let west = VoxelCoord::of([-1, Y, Z]).chunk;
        assert_ne!(
            west,
            VoxelCoord::of([0, Y, Z]).chunk,
            "the contact crosses a face"
        );
        f.app
            .world_mut()
            .resource_mut::<MeshState>()
            .take_cover_changes();
        f.settle();
        assert_eq!(f.occupants(-1).solid, stone(), "{v} m");
        assert_eq!(f.fluid(-1), None);
        assert_eq!(f.fluid(0), Some(Fluid::source(water)), "the water is kept");
        assert!(!touching(&f, lava, water));
        let cover = f
            .app
            .world_mut()
            .resource_mut::<MeshState>()
            .take_cover_changes();
        assert!(
            cover.contains(&west),
            "{v} m: the level-1 cover was not told"
        );
        assert!(f.app.world().resource::<MeshState>().is_dirty(west));
        assert!(f.sim().work.reacted >= 1 && f.rejections().is_empty());
    }
}

/// Lava and water flowing at each other meet; lava they touch becomes stone,
/// lava and water never end up beside each other, both sources stay, and the
/// lava's light goes where it cooled, equal to a fresh solve of the cells.
#[test]
fn flowing_lava_meeting_water_cools_and_its_light_goes_as_a_fresh_solve_would() {
    let (tables, id) = declared(1.0);
    let water = id("demo:water");
    let lava = id("demo:lava");
    let mut f = Fixture::with_tables(1.0, 10.0, 16, tables.clone());
    let home = ChunkCoord::new(0, 0, 0);
    let fresh = |f: &Fixture| {
        light_cells(
            f.store().get(home).unwrap(),
            f.store().overlay(home).unwrap(),
            [None; 6],
            &tables,
        )
    };
    let solved = fresh(&f);
    f.app
        .world_mut()
        .resource_mut::<ChunkStore>()
        .set_light(home, solved);
    let block = |f: &Fixture, x: i32| f.store().light_at([16 + x, Y, Z]).block();
    f.send(-6, EditAction::PlaceSource(lava));
    f.settle();
    let lit = block(&f, -2);
    assert!(lit > 0, "lava lights the corridor");
    f.send(4, EditAction::PlaceSource(water));
    f.settle();
    let stones: Vec<i32> = (-f.half..=f.half)
        .filter(|&x| f.occupants(x).solid == stone())
        .collect();
    assert!(!stones.is_empty(), "nothing cooled");
    assert!(!touching(&f, lava, water));
    assert_eq!(f.fluid(-6), Some(Fluid::source(lava)));
    assert_eq!(f.fluid(4), Some(Fluid::source(water)));
    assert!(block(&f, stones[0]) < lit, "cooled lava still glows");
    assert!(
        f.store().light(home) == Some(&fresh(&f)),
        "light disagrees with a fresh solve"
    );
}

/// The `fluids` Mod's own rule, slime that is flowing and touches its brine
/// becomes its ash: a second pair through the same native path, with its
/// state matching (the slime source beside nothing is never consumed).
#[test]
fn a_mods_own_pair_reacts_through_the_server() {
    let (tables, id) = declared(1.0);
    let brine = id("fluids:brine");
    let (slime, ash) = (id("fluids:slime"), id("fluids:ash"));
    let mut f = Fixture::with_tables(1.0, 8.0, 0, tables);
    f.send(0, EditAction::PlaceSource(slime));
    f.settle();
    assert_eq!(f.wet_length_m(), 2.0);
    f.send(3, EditAction::PlaceSource(brine));
    f.settle();
    let ashes: Vec<i32> = (-f.half..=f.half)
        .filter(|&x| f.occupants(x).solid == Voxel::cube(ash))
        .collect();
    assert_eq!(ashes, vec![2], "the flowing slime beside the brine");
    assert_eq!(f.fluid(0), Some(Fluid::source(slime)));
    assert_eq!(f.fluid(3), Some(Fluid::source(brine)));
    assert!(!touching(&f, slime, brine));
    assert!((-f.half..=f.half).all(|x| f.occupants(x).solid != stone()));
}

/// A held step read water beside the lava and would cool it; the player
/// takes the water away before it lands. The reaction is dropped whole and
/// the lava, re-evaluated without the water, stays lava.
#[test]
fn an_older_job_cannot_cool_lava_whose_water_is_gone() {
    let (tables, id) = declared(1.0);
    let water = id("demo:water");
    let lava = id("demo:lava");
    let mut f = Fixture::with_tables(1.0, 8.0, 0, tables);
    f.send(0, EditAction::PlaceSource(lava));
    f.settle();
    assert!(f.fluid(3).is_some_and(|l| l.material == lava) && f.fluid(4).is_none());
    let hold = Arc::clone(&f.sim_mut().job().hold);
    hold.hold();
    f.send(4, EditAction::PlaceSource(water));
    let started = Instant::now();
    while f.sim().job.in_flight() == 0 {
        f.tick();
        assert!(started.elapsed() < CEILING);
    }
    assert_eq!(
        f.fluid(4),
        Some(Fluid::source(water)),
        "the step read the water"
    );
    f.send(4, EditAction::RemoveSource);
    f.apply_intents();
    let stale = f.sim().stale;
    hold.release();
    while !f.sim().is_idle() {
        f.tick();
        assert_ne!(
            f.occupants(3).solid,
            stone(),
            "an old step's reaction landed"
        );
        assert!(started.elapsed() < CEILING);
        f.wait_if_running();
    }
    assert!(f.sim().stale > stale, "nothing was dropped");
    assert!(f.fluid(3).is_some_and(|l| l.material == lava));
    assert!(f.sim().work.reacted >= 1, "the held step did propose it");
}

/// Water beside lava from the player's fluid tool, from a 0.1 Mod placing a
/// water Voxel and from a 0.2 Mod's source Command: one World-change path,
/// so the same contact and the same stone.
#[test]
fn water_from_a_player_or_either_contract_line_cools_lava_the_same() {
    use tesserae_mods::{Command, Line, Placement, Region};
    let states: Vec<Vec<Occupants>> = (0..3)
        .map(|way| {
            let (tables, id) = declared(0.5);
            let water = id("demo:water");
            let mut f = Fixture::with_tables(0.5, 4.0, 0, tables);
            f.send(-2, EditAction::PlaceSource(id("demo:lava")));
            f.apply_intents();
            let at = [0.0, Y as f32 * 0.5, Z as f32 * 0.5];
            match way {
                0 => f.send(0, EditAction::PlaceSource(water)),
                1 => mod_runtime::tests::drain_validated(
                    &mut f.app,
                    Line::V01,
                    &[Command::Place(vec![Placement {
                        at,
                        voxel: Voxel::cube(water),
                    }])],
                ),
                _ => mod_runtime::tests::drain_validated(
                    &mut f.app,
                    Line::V02,
                    &[Command::PlaceSources {
                        region: Region {
                            min: at,
                            max: at.map(|m| m + 0.5),
                        },
                        material: water,
                    }],
                ),
            }
            f.settle();
            assert!(
                (-f.half..=f.half).any(|x| f.occupants(x).solid == stone()),
                "way {way}: nothing cooled"
            );
            f.state()
        })
        .collect();
    assert!(states[0] == states[1] && states[1] == states[2]);
}

/// Nine lava sources each under a water source: nine reactions due in one
/// step. Bounded to one landing a step they land one at a time, each whole,
/// and settle where the unbounded run does.
#[test]
fn reactions_past_the_landing_bound_land_whole_in_later_steps() {
    let run = |bound: Option<usize>| {
        let (tables, id) = declared(1.0);
        let water = id("demo:water");
        let lava = id("demo:lava");
        let mut f = Fixture::with_tables(1.0, 6.0, 0, tables);
        match bound {
            Some(bound) => f.sim_mut().set_bounds(AGENDA, bound),
            // Each cooling puts out lava's light, so the emitter bound counts too.
            None => f.sim_mut().set_emitter_bound(usize::MAX),
        }
        for x in -4..=4 {
            f.send(x, EditAction::PlaceSource(lava));
        }
        f.apply_intents();
        for x in -4..=4 {
            f.send(x, EditAction::PlaceSource(water));
        }
        f.apply_intents();
        let above = |f: &Fixture, x: i32| {
            let tables = f.app.world().resource::<ContentTables>();
            f.store()
                .occupants([x, Y + 1, Z], &tables.fluids)
                .unwrap()
                .fluid
        };
        assert!((-4..=4).all(|x| above(&f, x) == Some(Fluid::source(water))));
        let started = Instant::now();
        let mut cooled = 0;
        loop {
            f.tick();
            let now = (-4..=4)
                .filter(|&x| f.occupants(x).solid == stone())
                .count();
            if bound.is_some() {
                assert!(now <= cooled + 1, "{now} landed in one step");
            }
            cooled = now;
            if f.sim().is_idle() {
                break;
            }
            assert!(started.elapsed() < CEILING);
            f.wait_if_running();
        }
        assert_eq!(cooled, 9);
        assert!((-4..=4).all(|x| above(&f, x) == Some(Fluid::source(water))));
        (f.state(), f.sim().deferred)
    };
    let (free, none) = run(None);
    let (bounded, deferred) = run(Some(1));
    assert_eq!(none, 0);
    assert!(deferred > 0, "the bound never bit");
    assert!(free == bounded);
}

/// Through the streaming loop: lava and water meeting in a corridor cut in
/// deep rock leave stone, and what the Client holds is what a fresh mesh of
/// the store makes, the new stone and both fluids included.
#[test]
fn meshes_are_redelivered_after_a_reaction() {
    let mut h = Harness::new(small(), UNLIMITED);
    let (tables, id) = declared(WorldSettings::default().voxel_size);
    let water = id("demo:water");
    let lava = id("demo:lava");
    let tables = Arc::new(tables);
    h.app.insert_resource(ContentTables(Arc::clone(&tables)));
    h.settle();
    let rock = ChunkCoord::new(0, -1, 0);
    let (y, z) = (-16, 16);
    let mut held: HashMap<ChunkCoord, tesserae_core::ChunkMesh> = HashMap::default();
    let mut run = |h: &mut Harness, change: &dyn Fn(&mut ChunkStore, &mut MeshState, &Tables)| {
        h.app
            .world_mut()
            .resource_scope(|world, mut store: Mut<ChunkStore>| {
                let mut meshes = world.resource_mut::<MeshState>();
                change(&mut store, &mut meshes, &tables);
            });
        let started = Instant::now();
        loop {
            h.app.world_mut().run_system_once(simulate).unwrap();
            let (ready, _) = h.tick();
            for m in ready {
                held.insert(m.coord, m.mesh);
            }
            if h.app.world().resource::<FluidSim>().is_idle() {
                for m in h.settle().0 {
                    held.insert(m.coord, m.mesh);
                }
                break;
            }
            assert!(started.elapsed() < CEILING);
            std::thread::sleep(Duration::from_micros(50));
        }
        let store = h.store();
        let neighbors: [Option<LitChunk>; 6] = face_neighbors(rock).map(|n| Some(lit(store, n)));
        let fresh = mesh_chunk(lit(store, rock), neighbors, &tables);
        assert!(
            held.get(&rock).is_some_and(|m| same_mesh(m, &fresh)),
            "the Client holds a stale mesh"
        );
    };
    run(&mut h, &|store, meshes, tables| {
        for x in 1..=31 {
            for (dy, dz) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                edit::set_voxel(store, meshes, tables, [x, y + dy, z + dz], stone()).unwrap();
            }
            let open = (2..=30).contains(&x);
            let voxel = if open { Voxel::AIR } else { stone() };
            edit::set_voxel(store, meshes, tables, [x, y, z], voxel).unwrap();
        }
    });
    run(&mut h, &|store, meshes, tables| {
        for (x, material) in [(10, lava), (20, water)] {
            let source = Occupants {
                solid: Voxel::AIR,
                fluid: Some(Fluid::source(material)),
                burning: None,
            };
            set_occupants(store, meshes, tables, [x, y, z], source).unwrap();
        }
    });
    let store = h.store();
    let cooled = (11..20)
        .filter(|&x| store.voxel([x, y, z]) == Some(stone()))
        .count();
    assert!(cooled > 0, "nothing cooled");
}

mod fire;
#[cfg(feature = "dev")]
mod measure;
