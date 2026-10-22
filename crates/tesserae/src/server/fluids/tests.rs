use super::*;
use bevy::ecs::system::RunSystemOnce;
use bevy::tasks::{AsyncComputeTaskPool, TaskPoolBuilder};
use std::time::{Duration, Instant};
use tesserae_core::collide::VoxelVolume;
use tesserae_core::flow::STEP_S;
use tesserae_core::light::light_chunk;
use tesserae_core::material::WATER;
use tesserae_core::worldgen::{LANTERN, STONE};
use tesserae_core::{Chunk, Fluid, LitChunk, Occupants, Voxel, mesh_chunk, optical_chunk};

use super::super::edit::{EditAction, EditIntent, EditRejected, PendingEdits, RejectReason};
use super::super::player::Player;
use super::super::streaming::face_neighbors;
use super::super::streaming::tests::{Harness, UNLIMITED, lit, same_mesh, small};

const CEILING: Duration = Duration::from_secs(60);

fn stone() -> Voxel {
    Voxel::cube(STONE)
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
            .init_resource::<ContentTables>()
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
        f.send(0, EditAction::PlaceSource(WATER));
        f.tick();
        assert_eq!(f.fluid(0), Some(Fluid::source(WATER)), "{v} m");
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

/// How finely the quota cuts a step and how few proposals land per tick change
/// how many worker calls and ticks a step takes, and neither what lands nor at
/// which logical step, through a whole flow and recession.
#[test]
fn the_quota_and_landing_cap_change_nothing_but_the_ticks_taken() {
    let run = |evaluations: usize, landed: usize| {
        let mut f = Fixture::new(0.5, 6.0, 0);
        f.sim_mut().set_quota(evaluations, landed);
        let mut trace: HashMap<Step, Vec<Occupants>> = HashMap::default();
        let started = Instant::now();
        for action in [EditAction::PlaceSource(WATER), EditAction::RemoveSource] {
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
    let (whole, work, fast) = run(EVALUATIONS_PER_JOB, LANDED_PER_TICK);
    let (cut, cut_work, slow) = run(2, 1);
    assert!(whole.len() > 100, "{} steps", whole.len());
    assert!(whole == cut, "a quota changed what a step landed");
    assert_eq!(work, cut_work);
    assert!(
        slow > fast,
        "the quota did not bind: {slow} vs {fast} calls"
    );
}

/// A held worker read the source and is about to propose its first flow when
/// the player takes the source away. Those proposals are dropped at landing:
/// at no tick after the removal is there water beside where the source was.
#[test]
fn an_older_job_cannot_bring_back_a_removed_source_or_its_flow() {
    let mut f = Fixture::new(1.0, 10.0, 0);
    let placed_at = f.sim().now();
    f.send(0, EditAction::PlaceSource(WATER));
    f.tick();
    // Placed on `placed_at`'s tick, first read on the step after, and its
    // neighbours' timers run the sideways delay from there.
    let scale = FlowScale::new(&Tables::builtin().fluids, 1.0);
    let proposing = placed_at + scale.get(WATER).unwrap().sideways;
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
    f.send(0, EditAction::PlaceSource(WATER));
    f.settle();
    f.send(2, EditAction::Place(stone()));
    f.settle();
    assert_eq!(f.occupants(2).solid, stone());
    assert_eq!(f.fluid(2), None, "the stone displaced the water");
    assert_eq!(f.fluid(0), Some(Fluid::source(WATER)));
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
    f.send(0, EditAction::PlaceSource(WATER));
    f.settle();
    f.send(3, EditAction::RemoveSource);
    f.settle();
    assert_eq!(f.rejections(), vec![RejectReason::NoSource]);
    assert!(f.fluid(3).is_some_and(|w| !w.source));
}

fn fresh_light(store: &ChunkStore, coord: ChunkCoord) -> tesserae_core::ChunkLight {
    let chunk = store.get(coord).unwrap();
    let seen = optical_chunk(chunk, store.overlay(coord).unwrap());
    light_chunk(&seen, [None; 6], &Tables::builtin())
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
                &Tables::builtin(),
                lantern,
                Voxel::cube(LANTERN),
            )
            .unwrap();
        });
    let block = |f: &Fixture, x: i32| f.store().light_at([16 + x, Y, Z]).block();
    let dry = block(&f, 3);
    assert!(dry > 0, "the lantern reaches x = 3");
    assert!(f.store().light(home) == Some(&fresh_light(f.store(), home)));

    f.send(0, EditAction::PlaceSource(WATER));
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

/// Through the streaming loop: a stream in a corridor cut in deep rock is
/// meshed, delivered and redelivered as it flows and recedes, and what the
/// Client holds at the end of each is what a fresh mesh of the store makes,
/// water's partial surfaces included.
#[test]
fn full_detail_meshes_follow_the_water_as_it_flows_and_recedes() {
    let mut h = Harness::new(small(), UNLIMITED);
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
                set_occupants(
                    &mut store,
                    &mut meshes,
                    &Tables::builtin(),
                    source,
                    occupants,
                )
                .unwrap();
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
        let fresh = mesh_chunk(lit(store, rock), neighbors, &Tables::builtin());
        assert!(
            same_mesh(&held[&rock], &fresh),
            "the Client holds a stale mesh"
        );
        fresh.blended.quad_count()
    };

    let dry = cycle(&mut h, None);
    assert_eq!(dry, 0);
    let wet = cycle(&mut h, Some(Fluid::source(WATER)));
    assert!(wet > 0, "no water drawn");
    let surfaces: Vec<u8> = (17..=23)
        .map(|x| {
            h.store()
                .occupants([x, y, z], &Tables::builtin().fluids)
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
