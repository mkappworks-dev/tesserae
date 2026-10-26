//! Server tick distributions, logical progress and source-to-visible latency
//! for fluid and fire scenarios against a control, on flat generated terrain
//! streamed and meshed by the real systems. A measurement, not a check: run
//! one scenario per process with uptime logged before and after, as
//! `TESSERAE_T09_SCENARIO`, `TESSERAE_T09_VOXEL_M` and `TESSERAE_T09_OUT` say.
//! `TESSERAE_T09_LANDED`, `TESSERAE_T09_EMITTERS` and `TESSERAE_T09_CAPACITY`
//! override the bounds for a sweep.

use super::*;
use bevy::ecs::schedule::{ScheduleLabel, SingleThreadedExecutor};
use std::fmt::Write as _;
use tesserae_core::mesh::FLAME;
use tesserae_core::{CHUNK_SIZE, ChunkMesh, WorldgenParams};

use super::super::super::jobs::Jobs;
use super::super::super::lod::{LodBudget, LodJobs, LodState};
use super::super::super::mods::runtime::tests::demo_loaded;
use super::super::super::streaming::{
    ChunkMeshReady, ChunkUnloaded, LodMeshReady, LodUnloaded, Rings, StreamingBudget,
    StreamingSettings, tests::add_stream_systems,
};
use super::super::super::world_state::VoxelCoord as At;

const TICK: Duration = Duration::from_micros(15_625);

/// Past this the scenario is reported unsettled rather than waited on.
const MAX_TICKS: usize = 64 * 150;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct Edits;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct Simulation;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct Streaming;

#[repr(C)]
struct Timespec {
    tv_sec: i64,
    tv_nsec: i64,
}

unsafe extern "C" {
    fn clock_gettime(clock: u32, ts: *mut Timespec) -> i32;
}

/// This thread's CPU time: what the tick itself spent, whatever else the
/// host was running.
fn thread_cpu() -> Duration {
    const CLOCK_THREAD_CPUTIME_ID: u32 = 16;
    let mut ts = Timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: `ts` is a valid out-pointer for the call's duration.
    unsafe { clock_gettime(CLOCK_THREAD_CPUTIME_ID, &mut ts) };
    Duration::new(ts.tv_sec as u64, ts.tv_nsec as u32)
}

fn timed(f: impl FnOnce()) -> (Duration, Duration) {
    let (wall, cpu) = (Instant::now(), thread_cpu());
    f();
    (wall.elapsed(), thread_cpu() - cpu)
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Want {
    Fluid(MaterialId),
    Dry(MaterialId),
    Burning,
    NotBurning,
}

/// A Voxel whose change a scenario waits for: when the state shows it, and
/// when a delivered mesh of its Chunk first shows it.
struct Probe {
    name: &'static str,
    at: Cell,
    want: Want,
    /// Counted only from this phase on.
    phase: usize,
    state: Option<(usize, Step)>,
    visible: Option<usize>,
    level_one: Option<usize>,
}

impl Probe {
    fn new(name: &'static str, at: Cell, want: Want, phase: usize) -> Self {
        Probe {
            name,
            at,
            want,
            phase,
            state: None,
            visible: None,
            level_one: None,
        }
    }

    fn shows(&self, o: &Occupants) -> bool {
        match self.want {
            Want::Fluid(m) => o.fluid.is_some_and(|f| f.material == m),
            Want::Dry(m) => o.fluid.is_none_or(|f| f.material != m),
            Want::Burning => o.burning.is_some(),
            Want::NotBurning => o.burning.is_none(),
        }
    }

    /// Whether `mesh`, of the probe's Chunk, draws what it waits for: a quad
    /// of it overlapping the probe's Voxel, not merely meeting its edge.
    fn drawn(&self, mesh: &ChunkMesh) -> bool {
        let local = self.at.map(|c| c.rem_euclid(CHUNK_SIZE as i32) as f32);
        let overlaps = |quad: &[[f32; 3]]| {
            (0..3).all(|a| {
                let lo = quad.iter().map(|p| p[a]).fold(f32::MAX, f32::min);
                let hi = quad.iter().map(|p| p[a]).fold(f32::MIN, f32::max);
                if hi - lo < 1e-3 {
                    lo >= local[a] - 0.1 && lo <= local[a] + 1.1
                } else {
                    lo < local[a] + 1.0 - 1e-3 && hi > local[a] + 1e-3
                }
            })
        };
        let any = |material: u32| {
            [&mesh.opaque, &mesh.blended, &mesh.cutout]
                .iter()
                .any(|layer| {
                    layer
                        .positions
                        .chunks(4)
                        .zip(layer.materials.chunks(4))
                        .any(|(quad, m)| m[0] == material && overlaps(quad))
                })
        };
        match self.want {
            Want::Fluid(m) => any(u32::from(m)),
            Want::Dry(m) => !any(u32::from(m)),
            Want::Burning => any(FLAME),
            Want::NotBurning => !any(FLAME),
        }
    }
}

#[derive(Default, Clone, Copy)]
struct Sample {
    tick: usize,
    phase: usize,
    step: Step,
    edits: (Duration, Duration),
    fluids: (Duration, Duration),
    stream: (Duration, Duration),
    evaluated: usize,
    landed: usize,
    deferred: usize,
    stale: usize,
    parked: usize,
    scanned: usize,
    agenda: usize,
    rescans: usize,
    timers: usize,
    meshes: usize,
    level_one: usize,
    dirty: usize,
    unlit: usize,
    jobs: usize,
    mod_applied: u32,
}

impl Sample {
    fn wall(&self) -> Duration {
        self.edits.0 + self.fluids.0 + self.stream.0
    }

    fn cpu(&self) -> Duration {
        self.edits.1 + self.fluids.1 + self.stream.1
    }
}

struct Run {
    h: Harness,
    v: f32,
    /// The first air Voxel above the generated floor.
    floor: i32,
    water: MaterialId,
    lava: MaterialId,
    wood: MaterialId,
    tick: usize,
    phase: usize,
    deadline: Instant,
    samples: Vec<Sample>,
    probes: Vec<Probe>,
    /// Ticks each Chunk has been waiting for a mesh, and the longest wait.
    dirty_for: HashMap<ChunkCoord, usize>,
    longest_dirty: usize,
    delivered: usize,
}

impl Run {
    fn new(v: f32) -> Self {
        AsyncComputeTaskPool::get_or_init(|| TaskPoolBuilder::new().num_threads(3).build());
        let loaded = demo_loaded(v);
        let id = |name: &str| {
            loaded
                .registry
                .id_of(tesserae_mods::Kind::Material, name)
                .unwrap() as MaterialId
        };
        let (water, lava, wood) = (id("demo:water"), id("demo:lava"), id("demo:wood"));
        let world = WorldSettings {
            voxel_size: v,
            ..default()
        };
        let params = WorldgenParams {
            relief_m: 0.0,
            caves: None,
            smoothing: None,
            water: None,
            ..WorldgenParams::default()
        };
        let settings = StreamingSettings {
            view_distance_m: 24.0,
            vertical_distance_m: 24.0,
            hysteresis_m: 8.0,
            lod_distance_m: 256.0,
        };
        let mut h = Harness::with_world(settings, StreamingBudget::default(), world, params);
        h.app
            .insert_resource(ContentTables(Arc::new(loaded.tables)))
            .insert_resource(LodBudget::default())
            .init_resource::<PendingCommands>()
            .add_systems(Edits, mod_runtime::drain)
            .add_systems(Simulation, simulate);
        add_stream_systems(&mut h.app, Streaming);
        // On the calling thread, so its CPU clock sees the systems' work.
        for label in [Edits.intern(), Simulation.intern(), Streaming.intern()] {
            h.app.edit_schedule(label, |s| {
                s.set_executor(SingleThreadedExecutor::new());
            });
        }
        let mut sim = h.app.world_mut().resource_mut::<FluidSim>();
        let landed = env_usize("TESSERAE_T09_LANDED");
        let capacity = env_usize("TESSERAE_T09_CAPACITY");
        if landed.is_some() || capacity.is_some() {
            let limits = sim.agenda().limits();
            let limits = Limits {
                capacity: capacity.unwrap_or(limits.capacity),
                reserve: capacity.map_or(limits.reserve, |c| c / 4),
                scan_per_step: limits.scan_per_step,
            };
            let landed = landed.unwrap_or(sim.landed_per_step);
            sim.set_bounds(limits, landed);
        }
        if let Some(emitters) = env_usize("TESSERAE_T09_EMITTERS") {
            h.app
                .world_mut()
                .resource_mut::<FluidSim>()
                .set_emitter_bound(emitters);
        }
        let floor = 0;
        h.anchor_at(Vec3::new(0.0, 32.0, 0.0));
        let mut run = Run {
            h,
            v,
            floor,
            water,
            lava,
            wood,
            tick: 0,
            phase: 0,
            deadline: Instant::now(),
            samples: Vec::new(),
            probes: Vec::new(),
            dirty_for: HashMap::new(),
            longest_dirty: 0,
            delivered: 0,
        };
        run.settle_streaming();
        let lowest = (24.0 / v).round() as i32 - 4;
        run.floor = (lowest..)
            .find(|&y| run.store().voxel([0, y, 0]) == Some(Voxel::AIR))
            .expect("open sky over the floor");
        assert!(
            run.store()
                .voxel([0, run.floor - 1, 0])
                .is_some_and(|v| !v.is_air()),
            "flat terrain has no floor at the origin"
        );
        run
    }

    fn cells(&self, metres: f32) -> i32 {
        (metres / self.v).floor() as i32
    }

    /// The Voxel `metres` from the origin, `up` metres above the floor.
    fn at(&self, x: f32, up: f32, z: f32) -> Cell {
        [self.cells(x), self.floor + self.cells(up), self.cells(z)]
    }

    fn store(&self) -> &ChunkStore {
        self.h.store()
    }

    fn sim(&self) -> &FluidSim {
        self.h.app.world().resource::<FluidSim>()
    }

    fn occupants(&self, at: Cell) -> Occupants {
        let tables = self.h.app.world().resource::<ContentTables>();
        self.store().occupants(at, &tables.fluids).expect("held")
    }

    fn change(
        &mut self,
        at: Cell,
        f: impl Fn(Occupants, &Tables) -> Occupants + Send + Sync + 'static,
    ) {
        self.h
            .app
            .world_mut()
            .run_system_once(
                move |mut store: ResMut<ChunkStore>,
                      mut meshes: ResMut<MeshState>,
                      tables: Res<ContentTables>| {
                    let now = store.occupants(at, &tables.fluids).expect("held");
                    let next = f(now, &tables);
                    edit::set_occupants(&mut store, &mut meshes, &tables, at, next)
                        .expect("the scenario's change applies");
                },
            )
            .unwrap();
    }

    fn put(&mut self, at: Cell, solid: Voxel) {
        self.change(at, move |now, tables| now.place_solid(solid, tables));
    }

    fn source(&mut self, at: Cell, material: MaterialId) {
        self.change(at, move |now, _| Occupants {
            fluid: Some(Fluid::source(material)),
            ..now
        });
    }

    fn dry(&mut self, at: Cell) {
        self.change(at, |now, _| Occupants { fluid: None, ..now });
    }

    fn ignite(&mut self, at: Cell) {
        self.change(at, |now, tables| {
            let fuel = tables
                .combustion
                .get(now.solid.material())
                .expect("fuel")
                .fuel_steps();
            Occupants {
                burning: Some(tesserae_core::Burning { fuel }),
                ..now
            }
        });
    }

    fn quiet(&self) -> bool {
        let world = self.h.app.world();
        let lod = world.resource::<LodJobs>();
        world.resource::<Jobs>().in_flight() == 0
            && lod.generate.in_flight() + lod.mesh.in_flight() == 0
            && !world.resource::<LodState>().any_pending()
            && world
                .resource::<MeshState>()
                .unlit_chunks()
                .next()
                .is_none()
    }

    /// Streams, lights and meshes everything in range, unrecorded.
    fn settle_streaming(&mut self) {
        self.h.settle();
        let started = Instant::now();
        let mut quiet = 0;
        while quiet < 32 {
            self.raw_tick();
            quiet = if self.quiet() { quiet + 1 } else { 0 };
            assert!(
                started.elapsed() < Duration::from_secs(600),
                "streaming never settled"
            );
            std::thread::sleep(Duration::from_micros(200));
        }
    }

    fn raw_tick(&mut self) -> (Duration, Duration, Duration, Duration, Duration, Duration) {
        let world = self.h.app.world_mut();
        let edits = timed(|| world.run_schedule(Edits));
        let fluids = timed(|| world.run_schedule(Simulation));
        let stream = timed(|| world.run_schedule(Streaming));
        (edits.0, edits.1, fluids.0, fluids.1, stream.0, stream.1)
    }

    /// One recorded Server tick at 64 Hz: a tick that overruns starts the
    /// next at once, as the fixed loop's catch-up does.
    fn tick(&mut self) {
        let now = Instant::now();
        if now < self.deadline {
            std::thread::sleep(self.deadline - now);
        }
        self.deadline = Instant::now().max(self.deadline) + TICK;
        let before = {
            let sim = self.sim();
            (
                sim.work.evaluated,
                sim.landed,
                sim.deferred,
                sim.stale,
                sim.parked,
                sim.scanned,
            )
        };
        let (ew, ec, fw, fc, sw, sc) = self.raw_tick();
        let world = self.h.app.world_mut();
        let ready: Vec<ChunkMeshReady> = world
            .resource_mut::<Messages<ChunkMeshReady>>()
            .drain()
            .collect();
        let lod: Vec<LodMeshReady> = world
            .resource_mut::<Messages<LodMeshReady>>()
            .drain()
            .collect();
        world.resource_mut::<Messages<ChunkUnloaded>>().clear();
        world.resource_mut::<Messages<LodUnloaded>>().clear();
        self.delivered += ready.len();

        let tick = self.tick;
        let step = self.sim().now();
        for i in 0..self.probes.len() {
            let p = &self.probes[i];
            if p.phase > self.phase {
                continue;
            }
            if p.state.is_none() && p.shows(&self.occupants(p.at)) {
                self.probes[i].state = Some((tick, step));
            }
            let p = &self.probes[i];
            if p.state.is_some() && p.visible.is_none() {
                let chunk = At::of(p.at).chunk;
                if ready.iter().any(|m| m.coord == chunk && p.drawn(&m.mesh)) {
                    self.probes[i].visible = Some(tick);
                }
            }
            let p = &self.probes[i];
            if p.state.is_some() && p.level_one.is_none() {
                let chunk = At::of(p.at).chunk;
                let cover = ChunkCoord::new(
                    chunk.x.div_euclid(2),
                    chunk.y.div_euclid(2),
                    chunk.z.div_euclid(2),
                );
                if lod.iter().any(|m| m.level == 1 && m.coord == cover) {
                    self.probes[i].level_one = Some(tick);
                }
            }
        }
        let meshes = self.h.app.world().resource::<MeshState>();
        let world = self.h.app.world();
        let settings = *world.resource::<StreamingSettings>();
        let rings = Rings::new(settings, world.resource::<WorldSettings>().cell_size());
        let center = world
            .resource::<WorldSettings>()
            .chunk_at(Vec3::new(0.0, 32.0, 0.0));
        let coords: Vec<ChunkCoord> = self
            .store()
            .coords()
            .filter(|c| {
                rings
                    .mesh
                    .contains(IVec3::new(c.x - center.x, c.y - center.y, c.z - center.z))
            })
            .collect();
        let mut dirty = 0;
        for c in coords {
            if meshes.is_dirty(c) {
                dirty += 1;
                let n = self.dirty_for.entry(c).or_default();
                *n += 1;
                self.longest_dirty = self.longest_dirty.max(*n);
            } else {
                self.dirty_for.remove(&c);
            }
        }
        let unlit = meshes.unlit_chunks().count();
        let sim = self.sim();
        let sample = Sample {
            tick,
            phase: self.phase,
            step,
            edits: (ew, ec),
            fluids: (fw, fc),
            stream: (sw, sc),
            evaluated: sim.work.evaluated - before.0,
            landed: sim.landed - before.1,
            deferred: sim.deferred - before.2,
            stale: sim.stale - before.3,
            parked: sim.parked - before.4,
            scanned: sim.scanned - before.5,
            agenda: sim.agenda().len(),
            rescans: sim.agenda().rescans(),
            timers: sim.timers(),
            meshes: ready.len(),
            level_one: lod.iter().filter(|m| m.level == 1).count(),
            dirty,
            unlit,
            jobs: self.h.app.world().resource::<Jobs>().in_flight(),
            mod_applied: {
                let q = self.h.app.world().resource::<PendingCommands>();
                q.applied(0) + q.applied(1)
            },
        };
        self.samples.push(sample);
        self.tick += 1;
    }

    fn idle(&self) -> bool {
        self.sim().is_idle()
    }

    /// Ticks until the simulation is idle and every probe of this phase has
    /// shown and been drawn, then `extra` more.
    fn run_phase(&mut self, extra: usize, during: &mut dyn FnMut(&mut Run)) -> bool {
        let start = self.tick;
        loop {
            during(self);
            self.tick();
            let done = self.idle()
                && self
                    .probes
                    .iter()
                    .filter(|p| p.phase <= self.phase)
                    .all(|p| p.visible.is_some() || p.state.is_none() && p.phase < self.phase);
            if done {
                break;
            }
            if self.tick - start > MAX_TICKS {
                return false;
            }
        }
        for _ in 0..extra {
            during(self);
            self.tick();
        }
        true
    }
}

fn env_usize(name: &str) -> Option<usize> {
    std::env::var(name).ok().map(|v| v.parse().unwrap())
}

struct Stats {
    p50: f64,
    p90: f64,
    p99: f64,
    max: f64,
    mean: f64,
    over: usize,
}

fn stats(mut ms: Vec<f64>) -> Stats {
    ms.sort_by(f64::total_cmp);
    let at = |q: f64| ms[((ms.len() - 1) as f64 * q).round() as usize];
    Stats {
        p50: at(0.5),
        p90: at(0.9),
        p99: at(0.99),
        max: *ms.last().unwrap(),
        mean: ms.iter().sum::<f64>() / ms.len() as f64,
        over: ms.iter().filter(|&&t| t > 15.625).count(),
    }
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1e3
}

/// One timed part of a tick, as a row of the report reads it.
type Part = fn(&Sample) -> Duration;

fn report(run: &Run, scenario: &str, settled: bool, idle_evaluations: usize) -> String {
    let mut out = String::new();
    let s = &run.samples;
    let sim = run.sim();
    let limits = sim.agenda().limits();
    writeln!(
        out,
        "scenario {scenario} voxel_m {} ticks {} steps {} settled {settled} \
         landed_bound {} emitter_bound {} capacity {} reserve {} scan {} \
         evaluations_per_job {} idle_evaluations_after {idle_evaluations}",
        run.v,
        s.len(),
        s.last().map_or(0, |l| l.step) - s.first().map_or(0, |f| f.step),
        sim.landed_per_step,
        sim.emitters_per_step,
        limits.capacity,
        limits.reserve,
        limits.scan_per_step,
        sim.quota.evaluations,
    )
    .unwrap();
    let rows: [(&str, Part); 8] = [
        ("tick_wall", Sample::wall),
        ("tick_cpu", Sample::cpu),
        ("fluids_wall", |x| x.fluids.0),
        ("fluids_cpu", |x| x.fluids.1),
        ("stream_wall", |x| x.stream.0),
        ("stream_cpu", |x| x.stream.1),
        ("edits_wall", |x| x.edits.0),
        ("edits_cpu", |x| x.edits.1),
    ];
    for (name, f) in &rows {
        let st = stats(s.iter().map(|x| ms(f(x))).collect());
        writeln!(
            out,
            "{name}_ms p50 {:.3} p90 {:.3} p99 {:.3} max {:.3} mean {:.3} over_15.625 {}",
            st.p50, st.p90, st.p99, st.max, st.mean, st.over
        )
        .unwrap();
    }
    let landing: Vec<&Sample> = s.iter().filter(|x| x.landed > 0).collect();
    if !landing.is_empty() {
        let per: Vec<f64> = landing
            .iter()
            .map(|x| ms(x.fluids.1) * 1e3 / x.landed as f64)
            .collect();
        let st = stats(per);
        writeln!(
            out,
            "landing_ticks {} fluids_cpu_us_per_landed p50 {:.1} p90 {:.1} max {:.1}",
            landing.len(),
            st.p50,
            st.p90,
            st.max
        )
        .unwrap();
    }
    let sum = |f: &dyn Fn(&Sample) -> usize| s.iter().map(f).sum::<usize>();
    let max = |f: &dyn Fn(&Sample) -> usize| s.iter().map(f).max().unwrap_or(0);
    writeln!(
        out,
        "work evaluated {} landed {} deferred {} stale {} parked {} scanned {} \
         | max per tick evaluated {} landed {} | max agenda {} rescans {} timers {} \
         | overflowed {} | meshes {} level1 {} max_dirty {} longest_dirty_ticks {} \
         max_unlit {} max_jobs {} mod_applied {}",
        sum(&|x| x.evaluated),
        sum(&|x| x.landed),
        sum(&|x| x.deferred),
        sum(&|x| x.stale),
        sum(&|x| x.parked),
        sum(&|x| x.scanned),
        max(&|x| x.evaluated),
        max(&|x| x.landed),
        max(&|x| x.agenda),
        max(&|x| x.rescans),
        max(&|x| x.timers),
        sim.agenda().overflowed(),
        sum(&|x| x.meshes),
        sum(&|x| x.level_one),
        max(&|x| x.dirty),
        run.longest_dirty,
        max(&|x| x.unlit),
        max(&|x| x.jobs),
        s.last().map_or(0, |x| x.mod_applied),
    )
    .unwrap();
    let starts: Vec<(usize, Step)> = (0..=run.phase)
        .map(|ph| {
            s.iter()
                .find(|x| x.phase == ph)
                .map_or((0, 0), |x| (x.tick, x.step))
        })
        .collect();
    for p in &run.probes {
        let (t0, s0) = starts[p.phase];
        let fmt = |o: Option<usize>| o.map_or("never".into(), |t| format!("{}", t - t0));
        writeln!(
            out,
            "probe {} phase {} state_ticks {} state_steps {} visible_ticks {} level1_ticks {}",
            p.name,
            p.phase,
            fmt(p.state.map(|s| s.0)),
            p.state
                .map_or("never".into(), |(_, step)| format!("{}", step - s0)),
            fmt(p.visible),
            fmt(p.level_one),
        )
        .unwrap();
    }
    out
}

fn raw(run: &Run) -> String {
    let mut out = String::from(
        "tick,phase,step,edits_wall_us,edits_cpu_us,fluids_wall_us,fluids_cpu_us,\
         stream_wall_us,stream_cpu_us,evaluated,landed,deferred,stale,parked,scanned,\
         agenda,rescans,timers,meshes,level1,dirty,unlit,jobs,mod_applied\n",
    );
    let us = |d: Duration| d.as_micros();
    for x in &run.samples {
        writeln!(
            out,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            x.tick,
            x.phase,
            x.step,
            us(x.edits.0),
            us(x.edits.1),
            us(x.fluids.0),
            us(x.fluids.1),
            us(x.stream.0),
            us(x.stream.1),
            x.evaluated,
            x.landed,
            x.deferred,
            x.stale,
            x.parked,
            x.scanned,
            x.agenda,
            x.rescans,
            x.timers,
            x.meshes,
            x.level_one,
            x.dirty,
            x.unlit,
            x.jobs,
            x.mod_applied,
        )
        .unwrap();
    }
    out
}

/// A player's stone every 64 ticks somewhere quiet, and two Mod queues of 40
/// placements each, drained at the phase 7 rate: the Edits every scenario
/// must keep landing, and the control's whole load besides streaming.
fn background(run: &mut Run) {
    let base = run.at(-20.0, 0.0, 20.0);
    let mut queues = run.h.app.world_mut().resource_mut::<PendingCommands>();
    for index in 0..2 {
        let places = (0..40)
            .map(|i| {
                (
                    [base[0] + i % 8, base[1] + index as i32, base[2] + i / 8],
                    Voxel::cube(STONE),
                )
            })
            .collect();
        queues.push_places(index, places);
    }
}

fn player_edits(run: &mut Run) {
    if run.tick.is_multiple_of(64) {
        let n = (run.tick / 64) as i32;
        let at = run.at(20.0, 0.0, -20.0);
        let at = [at[0] + n % 16, at[1], at[2] + n / 16];
        run.put(at, Voxel::cube(STONE));
        assert_eq!(
            run.occupants(at).solid,
            Voxel::cube(STONE),
            "the Edit waited"
        );
    }
}

#[test]
#[ignore = "a measurement, not a check: run by hand with uptime before and after"]
fn measure_fluid_and_fire_scenarios() {
    let scenario = std::env::var("TESSERAE_T09_SCENARIO").unwrap();
    let v: f32 = std::env::var("TESSERAE_T09_VOXEL_M")
        .unwrap()
        .parse()
        .unwrap();
    let out = std::path::PathBuf::from(std::env::var("TESSERAE_T09_OUT").unwrap());
    let mut run = Run::new(v);
    background(&mut run);
    let water = run.water;
    let mut edits = player_edits;
    let settled = match scenario.as_str() {
        "control" => {
            for _ in 0..64 * 20 {
                player_edits(&mut run);
                run.tick();
            }
            true
        }
        "waterfall" | "outlets" => {
            let spots: Vec<(f32, f32)> = if scenario == "waterfall" {
                vec![(0.0, 0.0)]
            } else {
                vec![(-12.0, 0.0), (12.0, 0.0), (0.0, -12.0), (0.0, 12.0)]
            };
            let sources: Vec<Cell> = spots.iter().map(|&(x, z)| run.at(x, 16.0, z)).collect();
            let names = [
                ("foot", "edge_3m", "foot_dry", "edge_3m_dry"),
                ("foot_b", "edge_3m_b", "foot_dry_b", "edge_3m_dry_b"),
                ("foot_c", "edge_3m_c", "foot_dry_c", "edge_3m_dry_c"),
                ("foot_d", "edge_3m_d", "foot_dry_d", "edge_3m_dry_d"),
            ];
            for (&(x, z), n) in spots.iter().zip(names) {
                let foot = run.at(x, 0.0, z);
                let edge = run.at(x + 3.0, 0.0, z);
                run.probes
                    .push(Probe::new(n.0, foot, Want::Fluid(water), 0));
                run.probes
                    .push(Probe::new(n.1, edge, Want::Fluid(water), 0));
                run.probes.push(Probe::new(n.2, foot, Want::Dry(water), 1));
                run.probes.push(Probe::new(n.3, edge, Want::Dry(water), 1));
            }
            for &s in &sources {
                run.source(s, water);
            }
            let a = run.run_phase(64, &mut edits);
            run.phase = 1;
            for &s in &sources {
                run.dry(s);
            }
            a && run.run_phase(64, &mut edits)
        }
        "basin" => {
            let at = run.at(0.0, 0.0, 0.0);
            run.probes.push(Probe::new(
                "edge_3m",
                run.at(3.0, 0.0, 0.0),
                Want::Fluid(water),
                0,
            ));
            run.probes.push(Probe::new(
                "edge_6.5m",
                run.at(6.5, 0.0, 0.0),
                Want::Fluid(water),
                0,
            ));
            run.probes.push(Probe::new(
                "edge_3m_dry",
                run.at(3.0, 0.0, 0.0),
                Want::Dry(water),
                1,
            ));
            run.probes.push(Probe::new(
                "edge_6.5m_dry",
                run.at(6.5, 0.0, 0.0),
                Want::Dry(water),
                1,
            ));
            run.source(at, water);
            let a = run.run_phase(64, &mut edits);
            run.phase = 1;
            run.dry(at);
            a && run.run_phase(64, &mut edits)
        }
        "lava" => {
            let at = run.at(0.0, 4.0, 0.0);
            let lava = run.lava;
            run.probes.push(Probe::new(
                "foot",
                run.at(0.0, 0.0, 0.0),
                Want::Fluid(lava),
                0,
            ));
            run.probes.push(Probe::new(
                "edge_2m",
                run.at(2.0, 0.0, 0.0),
                Want::Fluid(lava),
                0,
            ));
            run.probes.push(Probe::new(
                "edge_2m_dry",
                run.at(2.0, 0.0, 0.0),
                Want::Dry(lava),
                1,
            ));
            run.source(at, lava);
            let a = run.run_phase(64, &mut edits);
            run.phase = 1;
            run.dry(at);
            a && run.run_phase(64, &mut edits)
        }
        "fire" => fire_chain(&mut run, &mut edits),
        "relight" => {
            relight(&mut run, &out);
            return;
        }
        "saturated" => saturated(&mut run, &mut edits),
        other => panic!("no scenario {other}"),
    };
    let before = run.sim().work.evaluated;
    for _ in 0..64 {
        run.tick();
    }
    let idle = run.sim().work.evaluated - before;
    let name = format!(
        "{scenario}-{v}-l{}-e{}-c{}",
        run.sim().landed_per_step,
        run.sim().emitters_per_step,
        run.sim().agenda().limits().capacity
    );
    let summary = report(&run, &scenario, settled, idle);
    std::fs::write(out.join(format!("{name}.txt")), &summary).unwrap();
    std::fs::write(out.join(format!("{name}.csv")), raw(&run)).unwrap();
    println!("{summary}");
}

/// Two 16 m wood bars 1 m square, 12 m apart, each lit at its west end. A
/// stone wall 1 m thick and 3 m tall cuts bar A at its middle; bar B has
/// none, and water poured onto it 10 s in puts its fire out and keeps the
/// rest from catching.
fn fire_chain(run: &mut Run, edits: &mut dyn FnMut(&mut Run)) -> bool {
    let wood = Voxel::cube(run.wood);
    let n = run.cells(1.0).max(1);
    let length = run.cells(16.0);
    let x0 = -run.cells(8.0);
    for (z0, barrier) in [(0, true), (run.cells(12.0), false)] {
        for x in x0..x0 + length {
            for y in 0..n {
                for z in 0..n {
                    run.put([x, run.floor + y, z0 + z], wood);
                }
            }
        }
        if barrier {
            for x in 0..n {
                for y in 0..run.cells(3.0) {
                    for z in -run.cells(2.0)..run.cells(3.0) {
                        run.put([x, run.floor + y, z0 + z], Voxel::cube(STONE));
                    }
                }
            }
        }
    }
    run.settle_streaming();
    let b = run.cells(12.0);
    let top = run.floor + n - 1;
    for (name, x, z, want) in [
        ("a_catch_4m", -4.5, 0, Want::Burning),
        ("a_catch_7.5m", -0.6, 0, Want::Burning),
        ("b_catch_4m", -4.5, b, Want::Burning),
    ] {
        let at = [run.cells(x), top, z];
        run.probes.push(Probe::new(name, at, want, 0));
    }
    for y in 0..n {
        for z in 0..n {
            run.ignite([x0, run.floor + y, z]);
            run.ignite([x0, run.floor + y, b + z]);
        }
    }
    let start = run.sim().now();
    let pour = [run.cells(3.0), top + 1, b];
    let mut poured = None;
    let mut during = |run: &mut Run| {
        edits(run);
        if poured.is_none() && run.sim().now() >= start + 640 {
            run.source(pour, run.water);
            poured = Some(run.tick);
            // The front, burning when the water comes, goes out.
            let front = (x0..x0 + length)
                .rev()
                .map(|x| [x, top, b])
                .find(|&at| run.occupants(at).burning.is_some());
            if let Some(at) = front {
                run.probes
                    .push(Probe::new("b_front_out", at, Want::NotBurning, 0));
            }
        }
    };
    let settled = run.run_phase(64, &mut during);
    let a_past = [run.cells(1.5), top, 0];
    let b_past = [run.cells(5.0), top, b];
    let held = run.occupants(a_past).solid == wood && run.occupants(b_past).solid == wood;
    println!(
        "fire: barrier and water held {held} (a past the wall {:?}, b past the water {:?}); \
         poured at tick {poured:?}",
        run.occupants(a_past).solid,
        run.occupants(b_past).solid
    );
    settled && held
}

/// A breached dam: a wall of sources 16 m wide and 4 m high on the floor,
/// pouring out on every side at once, beside a 12 m square wood floor lit
/// along one edge, with the background Edits running. Everything watched is
/// inside the mesh extent at every Voxel size.
fn saturated(run: &mut Run, edits: &mut dyn FnMut(&mut Run)) -> bool {
    let wood = Voxel::cube(run.wood);
    let (x0, z0) = (run.cells(12.0), run.cells(-6.0));
    let side = run.cells(12.0);
    let thick = run.cells(1.0).max(1);
    for x in x0..x0 + side {
        for z in z0..z0 + side {
            for y in 0..thick {
                run.put([x, run.floor + y, z], wood);
            }
        }
    }
    run.settle_streaming();
    let top = run.floor + thick - 1;
    run.probes.push(Probe::new(
        "fire_far_edge",
        [x0 + side - 1, top, z0 + side / 2],
        Want::Burning,
        0,
    ));
    let dam: Vec<Cell> = (run.cells(-8.0)..run.cells(8.0))
        .flat_map(|z| (0..run.cells(4.0)).map(move |y| (y, z)))
        .map(|(y, z)| [run.cells(-6.0), run.floor + y, z])
        .collect();
    for &at in &dam {
        run.source(at, run.water);
    }
    run.probes.push(Probe::new(
        "flood_6m_out",
        run.at(-12.0, 0.0, 0.0),
        Want::Fluid(run.water),
        0,
    ));
    run.probes.push(Probe::new(
        "flood_corner",
        run.at(-1.0, 0.0, 9.0),
        Want::Fluid(run.water),
        0,
    ));
    for z in z0..z0 + side {
        for y in 0..thick {
            run.ignite([x0, run.floor + y, z]);
        }
    }
    run.run_phase(64, edits)
}

/// What one World change costs the main thread, by what it does to light:
/// water wetting and drying open-sky ground and a sealed dark room, lava
/// appearing and going in the dark, fire catching and going out, and a
/// surface change that moves no light. Each kind at 36 places a metre apart.
fn relight(run: &mut Run, out: &std::path::Path) {
    let n = run.cells(1.0).max(1);
    let (room, high) = (run.cells(6.0), run.cells(3.0));
    let bottom = run.floor - run.cells(8.0);
    let x0 = -room / 2;
    for x in x0..x0 + room {
        for y in bottom..bottom + high {
            for z in x0..x0 + room {
                run.put([x, y, z], Voxel::AIR);
            }
        }
    }
    run.settle_streaming();
    let dark: Vec<Cell> = (0..6)
        .flat_map(|i| (0..6).map(move |k| (i, k)))
        .map(|(i, k)| [x0 + i * n, bottom, x0 + k * n])
        .collect();
    let sky: Vec<Cell> = (0..6)
        .flat_map(|i| (0..6).map(move |k| (i, k)))
        .map(|(i, k)| [run.cells(4.0) + i * n, run.floor, run.cells(-3.0) + k * n])
        .collect();
    let (lava, wood) = (run.lava, Voxel::cube(run.wood));
    let flowing = |surface| {
        Fluid::flowing(
            run.water,
            [
                tesserae_core::Level {
                    surface,
                    reach: 3,
                    descent: 0,
                },
                tesserae_core::Level::DRY,
                tesserae_core::Level::DRY,
                tesserae_core::Level::DRY,
            ],
        )
    };
    let fluid = |f: Option<Fluid>| Occupants {
        solid: Voxel::AIR,
        fluid: f,
        burning: None,
    };
    let fuel = run
        .h
        .app
        .world()
        .resource::<ContentTables>()
        .combustion
        .get(run.wood)
        .unwrap()
        .fuel_steps();
    let burning = |on: bool| Occupants {
        solid: wood,
        fluid: None,
        burning: on.then_some(tesserae_core::Burning { fuel }),
    };
    let kinds: [(&str, &[Cell], Occupants, Occupants); 7] = [
        ("water_wets_sky", &sky, fluid(None), fluid(flowing(16))),
        ("water_dries_sky", &sky, fluid(flowing(16)), fluid(None)),
        ("water_wets_dark", &dark, fluid(None), fluid(flowing(16))),
        ("surface_only", &dark, fluid(flowing(16)), fluid(flowing(8))),
        (
            "lava_appears_dark",
            &dark,
            fluid(None),
            fluid(Some(Fluid::source(lava))),
        ),
        (
            "lava_goes_dark",
            &dark,
            fluid(Some(Fluid::source(lava))),
            fluid(None),
        ),
        ("fire_catches_dark", &dark, burning(false), burning(true)),
    ];
    let mut text = format!("relight voxel_m {}\n", run.v);
    let mut times = |name: &str, cells: &[Cell], from: Occupants, to: Occupants, run: &mut Run| {
        let mut us = Vec::new();
        for &at in cells {
            let world = run.h.app.world_mut();
            world.resource_scope(|world, mut store: Mut<ChunkStore>| {
                world.resource_scope(|world, mut meshes: Mut<MeshState>| {
                    let tables = world.resource::<ContentTables>();
                    set_occupants(&mut store, &mut meshes, tables, at, from).unwrap();
                    store.take_changed();
                    let (_, cpu) = timed(|| {
                        set_occupants(&mut store, &mut meshes, tables, at, to).unwrap();
                    });
                    us.push(cpu.as_secs_f64() * 1e6);
                    set_occupants(&mut store, &mut meshes, tables, at, fluid(None)).unwrap();
                    store.take_changed();
                });
            });
        }
        let st = stats(us);
        writeln!(
            text,
            "{name} cpu_us p50 {:.1} p90 {:.1} max {:.1} mean {:.1}",
            st.p50, st.p90, st.max, st.mean
        )
        .unwrap();
    };
    for (name, cells, from, to) in kinds {
        times(name, cells, from, to, run);
    }
    times(
        "fire_goes_out_dark",
        &dark,
        burning(true),
        burning(false),
        run,
    );
    std::fs::write(out.join(format!("relight-{}.txt", run.v)), &text).unwrap();
    println!("{text}");
}
