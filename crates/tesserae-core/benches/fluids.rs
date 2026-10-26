use tesserae_core::testgen::fixtures;
use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use std::collections::HashMap;
use std::hint::black_box;
use std::sync::Arc;
use tesserae_core::agenda::{Agenda, Limits, Span};
use tesserae_core::fire::{alters_exposure, wake_fuel_around};
use tesserae_core::flow::{
    Advance, Cell, CellView, Continuation, FlowScale, Quota, Reach, Region, Step, Timers, Work,
    advance, chunk_of, reading_outside, still_applies, wake_around, wake_solid_around,
};
use tesserae_core::mesh::POS_Y;
use tesserae_core::{
    Burn, Burning, CHUNK_SIZE, Chunk, ChunkOverlay, CombustionDef, Fluid, FluidDef, Heat,
    MaterialDef, MaterialId, Occlusion, Occupants, Orientation, Overlay, Phase, Reactant,
    ReactionDef, RenderClass, SHAPE_POST, SHAPE_SLAB, Side, Tables, Voxel, Worldgen,
    WorldgenParams, testgen,
};

const SEED: u64 = 0x7e55_e2ae_0000_0001;

/// The Server's division of a step: what a worker call evaluates.
const QUOTA: Quota = Quota { evaluations: 1024 };

const SIZES: [f32; 3] = [1.0, 0.5, 0.25];

/// The demo's content: lava cooling to stone against water, wood burning to
/// ash with the spec's tuning.
#[derive(Clone, Copy)]
struct Ids {
    water: MaterialId,
    lava: MaterialId,
    wood: MaterialId,
}

fn water_tables() -> (Tables, MaterialId) {
    let mut tables = fixtures().tables.clone();
    let water = fixtures().water;
    (tables, water)
}

fn demo_tables() -> (Tables, Ids) {
    let fixture_rock = fixtures().ground.rock;
    // Water first: an equal claim goes to the earlier registered fluid.
    let (mut tables, water) = water_tables();
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
    let solid = |tables: &mut Tables| {
        tables
            .materials
            .register(MaterialDef {
                emission: 0,
                dimming: 15,
                occlusion: Occlusion::Opaque,
                render: RenderClass::Opaque,
                solid: true,
            })
            .unwrap()
    };
    let wood = solid(&mut tables);
    let ash = solid(&mut tables);
    let heat = Some(Heat {
        strength: 12,
        reach_m: 2.0,
    });
    tables
        .register_fluid(
            lava,
            FluidDef {
                reach_m: 3.0,
                sideways_m_per_s: 0.75,
                downward_m_per_s: 1.5,
                recession_m_per_s: 0.75,
                reset_drop_m: 0.5,
                waterlogs: false,
                extinguishes: false,
                heat,
            },
        )
        .unwrap();
    tables
        .register_reaction(ReactionDef {
            first: Reactant {
                fluid: lava,
                phase: Phase::Any,
            },
            second: Reactant {
                fluid: water,
                phase: Phase::Any,
            },
            consumed: Side::First,
            product: Some(fixture_rock),
            receiving: None,
        })
        .unwrap();
    tables
        .register_combustion(
            wood,
            CombustionDef {
                threshold: 6,
                ignition_delay_s: 2.0,
                emission: 12,
                heat,
                burn: Burn::For {
                    seconds: 8.0,
                    residue: ash,
                },
            },
        )
        .unwrap();
    (tables, Ids { water, lava, wood })
}

/// Held Chunks, outside which terrain is unknown.
#[derive(Clone, Default)]
struct Held {
    chunks: HashMap<[i32; 3], (Arc<Chunk>, Arc<ChunkOverlay>)>,
}

impl CellView for Held {
    fn stored(&self, at: Cell) -> Option<(Voxel, Overlay)> {
        let (chunk, [x, y, z]) = chunk_of(at);
        let (voxels, overlay) = self.chunks.get(&chunk)?;
        Some((voxels.get(x, y, z), overlay.get(x, y, z)))
    }
}

/// What a simulation retains at its largest, sampled after every step.
#[derive(Clone, Copy, Default, Debug)]
struct Peak {
    agenda: usize,
    timers: usize,
    proposals: usize,
    overlay_bytes: usize,
    /// Steps proposing more than each of [`BOUNDS`] changes, and the changes
    /// past it summed over the run: what a landing bound there would defer.
    over: [(usize, usize); BOUNDS.len()],
}

/// Landing bounds the report weighs.
const BOUNDS: [usize; 4] = [64, 256, 1024, 4096];

/// The kernel driven as the Server drives it: one step's due Voxels, a Region
/// of the held Chunks, calls of `QUOTA` evaluations, and every proposal landed
/// whole in coordinate order. Landing is the store write alone: the Server's
/// relight, meshing and landing bound are measured where they run.
#[derive(Clone)]
struct Sim {
    held: Held,
    tables: Arc<Tables>,
    scale: Arc<FlowScale>,
    reach: Reach,
    agenda: Agenda,
    timers: Timers,
    step: Step,
    work: Work,
    landed: usize,
    peak: Peak,
}

impl Sim {
    fn new(tables: Tables, voxel_m: f32, limits: Limits) -> Self {
        Sim {
            scale: Arc::new(FlowScale::new(&tables, voxel_m)),
            reach: Reach::of(&tables),
            tables: Arc::new(tables),
            held: Held::default(),
            agenda: Agenda::new(limits),
            timers: Timers::default(),
            step: 0,
            work: Work::default(),
            landed: 0,
            peak: Peak::default(),
        }
    }

    /// Holds every Chunk from `lo` to `hi`, air unless `fill` says otherwise.
    fn hold(&mut self, lo: [i32; 3], hi: [i32; 3], fill: impl Fn(Cell) -> Voxel) {
        let size = CHUNK_SIZE as i32;
        for cx in lo[0]..=hi[0] {
            for cy in lo[1]..=hi[1] {
                for cz in lo[2]..=hi[2] {
                    let mut chunk = Chunk::air();
                    for x in 0..CHUNK_SIZE {
                        for y in 0..CHUNK_SIZE {
                            for z in 0..CHUNK_SIZE {
                                let at = [
                                    cx * size + x as i32,
                                    cy * size + y as i32,
                                    cz * size + z as i32,
                                ];
                                chunk.set(x, y, z, fill(at));
                            }
                        }
                    }
                    self.held.chunks.insert(
                        [cx, cy, cz],
                        (Arc::new(chunk), Arc::new(ChunkOverlay::none())),
                    );
                }
            }
        }
    }

    fn occupants(&self, at: Cell) -> Occupants {
        let (voxel, overlay) = self.held.stored(at).expect("held");
        Occupants::of(voxel, overlay, &self.tables.fluids)
    }

    fn write(&mut self, at: Cell, next: Occupants) {
        let (voxel, overlay) = next.encode(&self.tables.fluids);
        let (chunk, [x, y, z]) = chunk_of(at);
        let (voxels, overlays) = self.held.chunks.get_mut(&chunk).expect("held");
        Arc::make_mut(voxels).set(x, y, z, voxel);
        Arc::make_mut(overlays).set(x, y, z, overlay);
    }

    /// Wakes what reads a changed position, as the Server's landing does.
    fn woken(&mut self, at: Cell, before: Occupants, after: Occupants, step: Step) {
        wake_around(&mut self.agenda, at, step, self.reach);
        let held = &self.held;
        if before.solid != after.solid {
            let shapes = &self.tables.shapes;
            wake_solid_around(&mut self.agenda, at, step, self.reach, |c| {
                held.stored(c).is_some_and(|(v, _)| shapes.is_connected(v))
            });
        }
        if alters_exposure(&self.tables, &before, &after) {
            let combustion = &self.tables.combustion;
            wake_fuel_around(
                &mut self.agenda,
                at,
                step,
                &self.scale.heat,
                |_| true,
                |c| {
                    held.stored(c)
                        .is_some_and(|(v, _)| !v.is_air() && combustion.contains(v.material()))
                },
            );
        }
    }

    /// An Edit between steps, read by the next one.
    fn edit(&mut self, at: Cell, next: Occupants) {
        self.tables.vet_occupants(next).expect("valid");
        let before = self.occupants(at);
        self.write(at, next);
        self.woken(at, before, next, self.step);
    }

    fn source(&mut self, at: Cell, material: MaterialId) {
        let here = self.occupants(at);
        self.edit(
            at,
            Occupants {
                fluid: Some(Fluid::source(material)),
                ..here
            },
        );
    }

    fn dry(&mut self, at: Cell) {
        let here = self.occupants(at);
        self.edit(
            at,
            Occupants {
                fluid: None,
                ..here
            },
        );
    }

    fn ignite(&mut self, at: Cell) {
        let here = self.occupants(at);
        let def = self.tables.combustion.get(here.solid.material()).unwrap();
        let fuel = def.fuel_steps();
        self.edit(
            at,
            Occupants {
                burning: Some(Burning { fuel }),
                ..here
            },
        );
    }

    fn run_step(&mut self) {
        let due = self.agenda.take_due(self.step).cells;
        let mut region = Region::default();
        for (&c, (voxels, overlay)) in &self.held.chunks {
            region.insert(c, Arc::clone(voxels), Arc::clone(overlay));
        }
        let mut cont = Continuation::start(self.step, due, std::mem::take(&mut self.timers));
        let done = loop {
            match advance(&region, &self.tables, &self.scale, cont, QUOTA) {
                Advance::Partial(next) => cont = next,
                Advance::Complete(done) => break done,
            }
        };
        drop(region);
        let mut proposals = done.proposals;
        self.peak.proposals = self.peak.proposals.max(proposals.len());
        for (i, bound) in BOUNDS.into_iter().enumerate() {
            if proposals.len() > bound {
                self.peak.over[i].0 += 1;
                self.peak.over[i].1 += proposals.len() - bound;
            }
        }
        proposals.sort_unstable_by_key(|p| p.at);
        let next = self.step + 1;
        for p in proposals {
            if !still_applies(&self.held, &self.tables, &p) {
                self.agenda.rewake(p.at, next);
                continue;
            }
            self.write(p.at, p.next);
            self.woken(p.at, p.read, p.next, next);
            self.landed += 1;
        }
        for (ready, at) in done.wakeups {
            self.agenda.rewake(at, ready);
        }
        self.agenda.end_step();
        self.timers = done.timers;
        self.work += done.work;
        self.step += 1;
        self.peak.agenda = self.peak.agenda.max(self.agenda.len());
        self.peak.timers = self.peak.timers.max(self.timers.len());
    }

    fn settle(&mut self) {
        let start = self.step;
        while !self.agenda.is_empty() {
            self.run_step();
            assert!(self.step - start < 200_000, "never settled");
        }
        let bytes = self.held.chunks.values().map(|(_, o)| o.heap_bytes()).sum();
        self.peak.overlay_bytes = self.peak.overlay_bytes.max(bytes);
    }
}

/// Voxels in `metres`, for fixtures written in metres.
fn cells(metres: f32, voxel_m: f32) -> i32 {
    (metres / voxel_m).round() as i32
}

/// Prints what a scenario evaluated and retained, once, beside its timing.
fn report(name: &str, sim: &Sim) {
    let w = sim.work;
    eprintln!(
        "ops {name}: steps {} evaluated {} proposed {} landed {} waiting {} paused {} \
         reacted {} blocked {} fuel {} searched {} | peak agenda {} timers {} proposals {} \
         overlay bytes {} | steps over, changes past bound {:?}",
        sim.step,
        w.evaluated,
        w.proposed,
        sim.landed,
        w.waiting,
        w.paused,
        w.reacted,
        w.blocked,
        w.fuel,
        w.searched,
        sim.peak.agenda,
        sim.peak.timers,
        sim.peak.proposals,
        sim.peak.overlay_bytes,
        BOUNDS.iter().zip(sim.peak.over).collect::<Vec<_>>(),
    );
}

fn stone() -> Voxel {
    let fixture_rock = fixtures().ground.rock;
    Voxel::cube(fixture_rock)
}

/// A stone floor at y = 0 under open air, `half_m` each way round the origin
/// and `up_m` high, in whole Chunks.
fn open_floor(tables: Tables, voxel_m: f32, half_m: f32, up_m: f32, limits: Limits) -> Sim {
    let mut sim = Sim::new(tables, voxel_m, limits);
    let size = CHUNK_SIZE as f32;
    let reach = (half_m / voxel_m / size).ceil() as i32;
    let up = (up_m / voxel_m / size).ceil() as i32;
    sim.hold([-reach, 0, -reach], [reach - 1, up - 1, reach - 1], |at| {
        if at[1] == 0 { stone() } else { Voxel::AIR }
    });
    sim
}

/// Settles a scenario from `start` and benches every repeat of it from that
/// same state, reporting its work once.
fn bench_settle(c: &mut Criterion, group: &str, name: &str, start: Sim, then: impl Fn(&mut Sim)) {
    let mut probe = start.clone();
    probe.settle();
    then(&mut probe);
    probe.settle();
    report(&format!("{group}/{name}"), &probe);
    let mut g = c.benchmark_group(group);
    g.sample_size(10);
    g.throughput(Throughput::Elements(probe.work.evaluated as u64));
    g.bench_function(name, |b| {
        b.iter_batched(
            || start.clone(),
            |mut sim| {
                sim.settle();
                then(&mut sim);
                sim.settle();
                black_box(sim.landed)
            },
            BatchSize::LargeInput,
        )
    });
    g.finish();
}

/// What a dry Chunk and a lake Chunk cost when a rescan visits every Voxel:
/// the activation scan a non-uniform lake Chunk gets on arrival, and a
/// journal-coalesced dry Chunk.
fn rescans(c: &mut Criterion) {
    let (tables, water) = water_tables();
    let lake = Worldgen::new(
        SEED,
        1.0,
        WorldgenParams::default().with_default_lake(water),
    );
    let dry = Worldgen::new(SEED, 1.0, WorldgenParams::default());
    let shore = (-8..8)
        .flat_map(|x| (-8..8).map(move |z| [x, 1, z]))
        .find(|&[x, y, z]| {
            let chunk = lake.chunk(x, y, z);
            chunk.uniform().is_none() && chunk.kinds().iter().any(|v| v.material() == water)
        })
        .expect("a lake shore near the origin");
    let mut g = c.benchmark_group("fluids/rescan");
    g.sample_size(20);
    g.throughput(Throughput::Elements(CHUNK_SIZE.pow(3) as u64));
    for (name, generator) in [("lake_shore", &lake), ("dry_terrain", &dry)] {
        let mut sim = Sim::new(tables.clone(), 1.0, Limits::UNBOUNDED);
        for d in [
            [0, 0, 0],
            [-1, 0, 0],
            [1, 0, 0],
            [0, -1, 0],
            [0, 1, 0],
            [0, 0, -1],
            [0, 0, 1],
        ] {
            let at = [shore[0] + d[0], shore[1] + d[1], shore[2] + d[2]];
            sim.held.chunks.insert(
                at,
                (
                    Arc::new(generator.chunk(at[0], at[1], at[2])),
                    Arc::new(ChunkOverlay::none()),
                ),
            );
        }
        sim.agenda.rescan(Span::chunk(shore));
        let mut probe = sim.clone();
        probe.settle();
        report(&format!("fluids/rescan/{name}"), &probe);
        g.bench_function(name, |b| {
            b.iter_batched(
                || sim.clone(),
                |mut sim| {
                    sim.run_step();
                    black_box(sim.work.evaluated)
                },
                BatchSize::LargeInput,
            )
        });
    }
    // A uniform lake beside held terrain is scanned along its faces only.
    let (tables, water) = water_tables();
    let mut sim = Sim::new(tables, 1.0, Limits::UNBOUNDED);
    sim.held.chunks.insert(
        [0, 0, 0],
        (
            Arc::new(Chunk::filled(Voxel::cube(water))),
            Arc::new(ChunkOverlay::none()),
        ),
    );
    for span in reading_outside([0, 0, 0], sim.reach) {
        sim.agenda.rescan(span);
    }
    let mut probe = sim.clone();
    probe.settle();
    report("fluids/rescan/uniform_lake_faces", &probe);
    g.throughput(Throughput::Elements(probe.work.evaluated as u64));
    g.bench_function("uniform_lake_faces", |b| {
        b.iter_batched(
            || sim.clone(),
            |mut sim| {
                sim.settle();
                black_box(sim.work.evaluated)
            },
            BatchSize::LargeInput,
        )
    });
    g.finish();

    // Settled terrain: a step with nothing due.
    let (tables, water) = water_tables();
    let mut settled = open_floor(tables, 1.0, 8.0, 4.0, Limits::UNBOUNDED);
    settled.source([0, 1, 0], water);
    settled.settle();
    let mut g = c.benchmark_group("fluids/idle");
    g.bench_function("settled_step", |b| {
        b.iter(|| {
            settled.run_step();
            black_box(settled.work.evaluated)
        })
    });
    g.finish();
}

/// A source 16 m over a flat floor wider than water's reach: it falls, spreads
/// its 7 m where it lands and, once removed, everything recedes.
fn flow_recession(c: &mut Criterion) {
    for v in SIZES {
        let (tables, water) = water_tables();
        let mut sim = open_floor(tables, v, 8.0, 17.0, Limits::UNBOUNDED);
        let top = [0, cells(16.0, v), 0];
        sim.source(top, water);
        bench_settle(c, "fluids/waterfall_16m", &format!("{v}"), sim, move |s| {
            s.dry(top)
        });
    }
}

/// Water poured onto a floor of bottom slabs running off into a floor of
/// posts, every flowing Voxel in a Shape's voids: the waterlogging-heavy case.
fn waterlogging(c: &mut Criterion) {
    let fixture_rock = fixtures().ground.rock;
    let slab = Voxel::shaped(fixture_rock, SHAPE_SLAB, Orientation::new(POS_Y, 0));
    let post = Voxel::shaped(fixture_rock, SHAPE_POST, Orientation::new(POS_Y, 0));
    for v in SIZES {
        let (tables, water) = water_tables();
        let mut sim = open_floor(tables, v, 8.0, 4.0, Limits::UNBOUNDED);
        let half = cells(7.5, v);
        for x in -half..half {
            for z in -half..half {
                let voxel = if x < 0 { post } else { slab };
                sim.write(
                    [x, 1, z],
                    Occupants::of(voxel, Overlay::NONE, &sim.tables.fluids),
                );
            }
        }
        let top = [cells(2.0, v), cells(2.0, v), 0];
        sim.source(top, water);
        bench_settle(
            c,
            "fluids/waterlogged_floor",
            &format!("{v}"),
            sim,
            move |s| s.dry(top),
        );
    }
}

/// Water and lava sources 6 m apart on a floor: where they meet the lava
/// cools to stone, until no lava touches water.
fn reactions(c: &mut Criterion) {
    for v in SIZES {
        let (tables, ids) = demo_tables();
        let mut sim = open_floor(tables, v, 8.0, 4.0, Limits::UNBOUNDED);
        let (w, l) = ([-cells(3.0, v), 1, 0], [cells(3.0, v), 1, 0]);
        sim.source(w, ids.water);
        sim.source(l, ids.lava);
        bench_settle(c, "fluids/reaction_front", &format!("{v}"), sim, |_| {});
    }
}

/// A wood bar 1 m square on a stone floor: lit at one end it burns along its
/// 16 m to ash; lit whole over 4 m with water poured on its middle, the water
/// puts it out.
fn fire(c: &mut Criterion) {
    for v in SIZES {
        let (tables, ids) = demo_tables();
        let bar = |length_m: f32| {
            let mut sim = open_floor(tables.clone(), v, 10.0, 4.0, Limits::UNBOUNDED);
            let (n, w) = (cells(length_m, v), cells(1.0, v));
            let cells: Vec<Cell> = (0..n)
                .flat_map(|x| (1..=w).flat_map(move |y| (0..w).map(move |z| [x - n / 2, y, z])))
                .collect();
            for &at in &cells {
                sim.edit(
                    at,
                    Occupants {
                        solid: Voxel::cube(ids.wood),
                        fluid: None,
                        burning: None,
                    },
                );
            }
            (sim, cells, n, w)
        };
        let (mut chain, cells, n, _) = bar(16.0);
        for &at in cells.iter().filter(|at| at[0] == -n / 2) {
            chain.ignite(at);
        }
        bench_settle(c, "fire/chain_16m", &format!("{v}"), chain, |_| {});

        let (mut burning, cells, _, w) = bar(4.0);
        for &at in &cells {
            burning.ignite(at);
        }
        burning.source([0, w + 1, 0], ids.water);
        bench_settle(c, "fire/extinguish", &format!("{v}"), burning, |_| {});
    }
}

/// More due than the agenda holds: every Voxel of four Chunks of shallow water
/// woken at once overflows into rescans, served a budget a step, and each step
/// is evaluated a quota a call.
fn saturated(c: &mut Criterion) {
    let limits = Limits {
        capacity: 16_384,
        reserve: 4_096,
        scan_per_step: 4_096,
    };
    let (tables, water) = water_tables();
    let mut sim = open_floor(tables, 0.25, 8.0, 8.0, limits);
    let half = cells(8.0, 0.25);
    for x in (-half..half).step_by(8) {
        for z in (-half..half).step_by(8) {
            sim.source([x, 1, z], water);
        }
    }
    sim.settle();
    let (work, landed, step) = (sim.work, sim.landed, sim.step);
    for cx in -1..=0 {
        for cz in -1..=0 {
            let home = Span::chunk([cx, 0, cz]);
            for x in home.min[0]..=home.max[0] {
                for y in home.min[1]..=home.max[1] {
                    for z in home.min[2]..=home.max[2] {
                        sim.agenda.wake([x, y, z], sim.step);
                    }
                }
            }
        }
    }
    sim.work = Work::default();
    sim.landed = 0;
    sim.peak = Peak::default();
    let mut probe = sim.clone();
    probe.settle();
    report("fluids/saturated/rescan_four_chunks", &probe);
    eprintln!(
        "ops fluids/saturated/flood_before_rescan: steps {step} evaluated {} landed {landed}",
        work.evaluated
    );
    let mut g = c.benchmark_group("fluids/saturated");
    g.sample_size(10);
    g.throughput(Throughput::Elements(probe.work.evaluated as u64));
    g.bench_function("rescan_four_chunks", |b| {
        b.iter_batched(
            || sim.clone(),
            |mut sim| {
                sim.settle();
                black_box(sim.work.evaluated)
            },
            BatchSize::LargeInput,
        )
    });

    // Eviction drops one Chunk's entries from a full agenda.
    let mut full = Agenda::new(limits);
    for i in 0..limits.capacity as i32 {
        full.wake([i % 256, i / 256 % 32, i / 8192], 1);
    }
    g.bench_function("forget_one_chunk_of_full_agenda", |b| {
        b.iter_batched(
            || full.clone(),
            |mut agenda| {
                agenda.forget([0, 0, 0]);
                black_box(agenda.len())
            },
            BatchSize::LargeInput,
        )
    });
    g.finish();
}

criterion_group!(
    benches,
    rescans,
    flow_recession,
    waterlogging,
    reactions,
    fire,
    saturated
);
criterion_main!(benches);
