use super::*;
use crate::agenda::Agenda;
use crate::fluid::FluidDef;
use crate::material::{STONE, WATER};

/// A box of held cells, air unless built otherwise; outside it is unknown.
struct Grid {
    cells: HashMap<Cell, (Voxel, Overlay)>,
    min: Cell,
    max: Cell,
}

impl CellView for Grid {
    fn stored(&self, at: Cell) -> Option<(Voxel, Overlay)> {
        let inside = (0..3).all(|a| self.min[a] <= at[a] && at[a] <= self.max[a]);
        inside.then(|| {
            self.cells
                .get(&at)
                .copied()
                .unwrap_or((Voxel::AIR, Overlay::NONE))
        })
    }
}

/// The kernel driven the way the Server drives it, with landing reduced to
/// writing the proposal.
struct Sim {
    grid: Grid,
    tables: Tables,
    scale: FlowScale,
    voxel_m: f32,
    agenda: Agenda,
    timers: Timers,
    step: Step,
    quota: Quota,
    work: Work,
    /// Reactions an earlier landing of their step undid.
    stale: usize,
}

const UNLIMITED: Quota = Quota {
    evaluations: usize::MAX,
};

impl Sim {
    fn new(voxel_m: f32, min: Cell, max: Cell) -> Self {
        Self::with_tables(voxel_m, min, max, Tables::builtin())
    }

    fn with_tables(voxel_m: f32, min: Cell, max: Cell, tables: Tables) -> Self {
        Sim {
            scale: FlowScale::new(&tables, voxel_m),
            tables,
            voxel_m,
            grid: Grid {
                cells: HashMap::new(),
                min,
                max,
            },
            agenda: Agenda::default(),
            timers: Timers::default(),
            step: 0,
            quota: UNLIMITED,
            work: Work::default(),
            stale: 0,
        }
    }

    /// Cells per metre, for fixtures written in metres.
    fn cells(&self, metres: f32) -> i32 {
        (metres / self.voxel_m).round() as i32
    }

    fn build(&mut self, at: Cell, voxel: Voxel) {
        self.grid.cells.insert(at, (voxel, Overlay::NONE));
    }

    fn occupants(&self, at: Cell) -> Occupants {
        let (voxel, overlay) = self.grid.stored(at).expect("held");
        Occupants::of(voxel, overlay, &self.tables.fluids)
    }

    fn fluid(&self, at: Cell) -> Option<Fluid> {
        self.occupants(at).fluid
    }

    /// An Edit between steps, read from the next step on.
    fn edit(&mut self, at: Cell, occupants: Occupants) {
        self.tables.vet_occupants(occupants).expect("valid");
        let before = self.occupants(at);
        let stored = occupants.encode(&self.tables.fluids);
        self.grid.cells.insert(at, stored);
        self.woken(at, before, occupants, self.step);
    }

    /// Wakes what reads a changed position, as the Server's landing does: the
    /// flow around it, and the fuel within heat's reach of it.
    fn woken(&mut self, at: Cell, before: Occupants, after: Occupants, step: Step) {
        let reach = Reach::of(&self.tables);
        wake_around(&mut self.agenda, at, step, reach);
        let grid = &self.grid;
        if before.solid != after.solid {
            let shapes = &self.tables.shapes;
            wake_solid_around(&mut self.agenda, at, step, reach, |c| {
                grid.stored(c).is_some_and(|(v, _)| shapes.is_connected(v))
            });
        }
        if crate::fire::alters_exposure(&self.tables, &before, &after) {
            let combustion = &self.tables.combustion;
            crate::fire::wake_fuel_around(
                &mut self.agenda,
                at,
                step,
                &self.scale.heat,
                |_| true,
                |c| {
                    grid.stored(c)
                        .is_some_and(|(v, _)| !v.is_air() && combustion.contains(v.material()))
                },
            );
        }
    }

    /// Lands a step's proposals in coordinate order, each only if it still
    /// applies after those before it; the rest are evaluated again next step.
    fn land(&mut self, mut proposals: Vec<Proposal>) {
        proposals.sort_unstable_by_key(|p| p.at);
        for p in proposals {
            assert!(
                !p.next.fluid.is_some_and(|f| f.source) || p.next.fluid == p.read.fluid,
                "flow made a source"
            );
            if !still_applies(&self.grid, &self.tables, &p) {
                assert!(p.contact.is_some(), "flow proposal read stale state");
                self.stale += 1;
                self.agenda.rewake(p.at, self.step + 1);
                continue;
            }
            self.tables.vet_occupants(p.next).expect("a valid proposal");
            self.grid
                .cells
                .insert(p.at, p.next.encode(&self.tables.fluids));
            self.woken(p.at, p.read, p.next, self.step + 1);
        }
    }

    /// Wakes every held Voxel, as admitting the whole grid would.
    fn wake_all(&mut self) {
        let (min, max) = (self.grid.min, self.grid.max);
        for x in min[0]..=max[0] {
            for y in min[1]..=max[1] {
                for z in min[2]..=max[2] {
                    self.agenda.wake([x, y, z], self.step);
                }
            }
        }
    }

    fn place_source(&mut self, at: Cell) {
        let here = self.occupants(at);
        self.edit(
            at,
            Occupants {
                fluid: Some(Fluid::source(WATER)),
                ..here
            },
        );
    }

    fn remove_fluid(&mut self, at: Cell) {
        let here = self.occupants(at);
        self.edit(
            at,
            Occupants {
                fluid: None,
                ..here
            },
        );
    }

    fn put(&mut self, at: Cell, solid: Voxel) {
        self.edit(
            at,
            Occupants {
                solid,
                fluid: None,
                burning: None,
            },
        );
    }

    fn run_step(&mut self) -> Finished {
        let due = self.agenda.take_due(self.step).cells;
        let mut cont = Continuation::start(self.step, due, std::mem::take(&mut self.timers));
        let done = loop {
            match advance(&self.grid, &self.tables, &self.scale, cont, self.quota) {
                Advance::Partial(next) => cont = next,
                Advance::Complete(done) => break done,
            }
        };
        self.land(done.proposals.clone());
        for &(step, at) in &done.wakeups {
            self.agenda.rewake(at, step);
        }
        self.agenda.end_step();
        self.timers = done.timers.clone();
        self.work += done.work;
        self.step += 1;
        done
    }

    /// [`Sim::run_step`] the way the Server divides a step among workers:
    /// `slices` parts, each with its Voxels' timers, finishing last part first.
    fn run_step_sliced(&mut self, slices: usize) {
        let due = self.agenda.take_due(self.step).cells;
        let size = due.len().div_ceil(slices).max(1);
        let mut parts: Vec<Finished> = due
            .chunks(size)
            .map(|part| {
                let timers = self.timers.split_off(part);
                Continuation::start(self.step, part.to_vec(), timers)
            })
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .map(
                |cont| match advance(&self.grid, &self.tables, &self.scale, cont, UNLIMITED) {
                    Advance::Complete(done) => done,
                    Advance::Partial(_) => unreachable!("unlimited"),
                },
            )
            .collect();
        parts.reverse();
        let proposals: Vec<Proposal> = parts.iter().flat_map(|d| d.proposals.clone()).collect();
        self.land(proposals);
        for done in parts {
            for &(step, at) in &done.wakeups {
                self.agenda.rewake(at, step);
            }
            self.timers.merge(done.timers);
            self.work += done.work;
        }
        self.agenda.end_step();
        self.step += 1;
    }

    fn run(&mut self, steps: Step) {
        for _ in 0..steps {
            self.run_step();
        }
    }

    fn run_seconds(&mut self, seconds: f64) {
        self.run((seconds / STEP_S).round() as Step);
    }

    /// Steps until nothing is waiting.
    fn settle(&mut self) -> Step {
        let start = self.step;
        while !self.agenda.is_empty() {
            self.run_step();
            assert!(self.step - start < 100_000, "never settled");
        }
        self.step - start
    }

    fn wet(&self) -> Vec<Cell> {
        let mut wet: Vec<Cell> = self
            .grid
            .cells
            .iter()
            .filter(|(at, _)| self.fluid(**at).is_some())
            .map(|(at, _)| *at)
            .collect();
        wet.sort_unstable();
        wet
    }

    fn sources(&self) -> usize {
        self.wet()
            .iter()
            .filter(|at| self.fluid(**at).unwrap().source)
            .count()
    }

    fn state(&self) -> Vec<(Cell, Occupants)> {
        let mut all: Vec<_> = self
            .grid
            .cells
            .keys()
            .map(|at| (*at, self.occupants(*at)))
            .collect();
        all.sort_unstable_by_key(|(at, _)| *at);
        all
    }
}

fn stone() -> Voxel {
    Voxel::cube(STONE)
}

const SIZES: [f32; 3] = [1.0, 0.5, 0.25];

/// A one-Voxel-wide corridor along +x and -x at floor level y = 1, `half_m`
/// long each way, with a stone floor at y = 0 and stone walls at z = +-1.
fn corridor(voxel_m: f32, half_m: f32) -> Sim {
    let n = (half_m / voxel_m).round() as i32;
    let mut sim = Sim::new(voxel_m, [-n - 1, 0, -1], [n + 1, 3, 1]);
    for x in -n - 1..=n + 1 {
        sim.build([x, 0, 0], stone());
        for y in 0..=3 {
            sim.build([x, y, -1], stone());
            sim.build([x, y, 1], stone());
        }
    }
    for y in 1..=3 {
        sim.build([-n - 1, y, 0], stone());
        sim.build([n + 1, y, 0], stone());
    }
    sim
}

/// How far flowing water reaches along +x from the source at x = 0, metres.
fn wet_length_m(sim: &Sim, y: i32) -> f32 {
    let mut x = 1;
    while sim.grid.stored([x, y, 0]).is_some() && sim.fluid([x, y, 0]).is_some() {
        x += 1;
    }
    (x - 1) as f32 * sim.voxel_m
}

#[test]
fn the_scale_turns_metres_and_metres_per_second_into_voxels_and_steps() {
    let tables = Tables::builtin();
    for (v, reach, reset, sideways, downward) in [
        (1.0, 7, 8, 16, 8),
        (0.5, 14, 16, 8, 4),
        (0.25, 28, 32, 4, 2),
    ] {
        let s = *FlowScale::new(&tables, v).get(WATER).unwrap();
        assert_eq!((s.reach, s.reset, s.rank), (reach, reset, 0), "{v} m");
        assert_eq!(
            [s.sideways, s.downward, s.recession].map(Pace::steps),
            [sideways, downward, sideways],
            "{v} m"
        );
        for low in [-3, 0, 5] {
            assert_eq!(s.sideways.across(low), sideways, "water divides a step");
        }
        assert_eq!(reach as f32 * v, 7.0, "reach is metres at {v} m");
    }
    assert!(FlowScale::new(&tables, 1.0).get(STONE).is_none());
}

/// Reach is metres of path, surfaces fall away from the source, and the
/// spread stops there however much open floor is left: a lower surface on flat
/// ground does not renew reach.
#[test]
fn a_source_spreads_its_reach_in_metres_at_every_voxel_size() {
    for v in SIZES {
        let mut sim = corridor(v, 12.0);
        sim.place_source([0, 1, 0]);
        sim.settle();
        assert_eq!(wet_length_m(&sim, 1), 7.0, "{v} m");
        let surfaces: Vec<u8> = (1..=sim.cells(7.0))
            .map(|x| sim.fluid([x, 1, 0]).unwrap().surface)
            .collect();
        assert!(
            surfaces.windows(2).all(|w| w[0] >= w[1]),
            "{v} m: {surfaces:?}"
        );
        assert!(
            surfaces[0] > *surfaces.last().unwrap(),
            "{v} m: {surfaces:?}"
        );
        assert!(surfaces.iter().all(|s| *s < Fluid::FULL_SURFACE));
        assert_eq!(sim.sources(), 1, "flow became a source at {v} m");
        // Symmetric: the same the other way.
        assert!(sim.fluid([-sim.cells(7.0), 1, 0]).is_some());
        assert!(sim.fluid([-sim.cells(7.0) - 1, 1, 0]).is_none());
    }
}

/// The front moves at the declared sideways rate in metres per second: after
/// `t` seconds it is `4 t` metres out, within one Voxel, at every size.
#[test]
fn the_front_advances_at_the_declared_rate_at_every_voxel_size() {
    for v in SIZES {
        let mut sim = corridor(v, 12.0);
        sim.place_source([0, 1, 0]);
        let mut elapsed = 0.0;
        for seconds in [0.25, 0.5, 1.0, 1.5] {
            sim.run_seconds(seconds - elapsed);
            elapsed = seconds;
            let expected = (FluidDef::WATER.sideways_m_per_s as f64 * seconds) as f32;
            let got = wet_length_m(&sim, 1);
            assert!(
                (got - expected).abs() <= v + 1e-4,
                "{v} m at {seconds} s: {got} m, expected {expected} m"
            );
        }
    }
}

/// Removing the source recedes everything, at the recession rate, and leaves
/// nothing behind.
#[test]
fn removing_the_source_recedes_every_flowing_voxel_at_the_recession_rate() {
    for v in SIZES {
        let mut sim = corridor(v, 12.0);
        sim.place_source([0, 1, 0]);
        sim.settle();
        sim.remove_fluid([0, 1, 0]);
        // Halfway through the time 7 m takes at the rate, some is still left.
        let rate = FluidDef::WATER.recession_m_per_s as f64;
        sim.run_seconds(3.5 / rate);
        let left = wet_length_m(&sim, 1);
        assert!(left > 0.0, "{v} m: receded too fast");
        let steps = sim.settle();
        assert!(sim.wet().is_empty(), "{v} m: {:?}", sim.wet());
        let total_s = (3.5 / rate) + steps as f64 * STEP_S;
        assert!(
            (total_s - 7.0 / rate).abs()
                <= sim.scale.get(WATER).unwrap().recession.steps() as f64 * STEP_S,
            "{v} m: receded in {total_s} s"
        );
    }
}

/// A source over a cliff feeds a waterfall that falls first, then spreads its
/// full reach again where it lands, at every Voxel size.
#[test]
fn a_waterfall_falls_then_spreads_its_reach_again_where_it_lands() {
    for v in SIZES {
        let cliff = (4.0 / v) as i32;
        let n = (12.0 / v) as i32;
        let mut sim = Sim::new(v, [-3, -1, -1], [n + 1, cliff + 2, 1]);
        for x in -3..=n + 1 {
            for y in -1..=cliff + 2 {
                sim.build([x, y, -1], stone());
                sim.build([x, y, 1], stone());
            }
            // The cliff top is x <= -1, its face at x = 0.
            let top = if x < 0 { cliff } else { 0 };
            for y in -1..top {
                sim.build([x, y, 0], stone());
            }
        }
        sim.place_source([-1, cliff, 0]);
        sim.settle();

        // Out over the edge and straight down: every Voxel of the column wet,
        // falling ones full.
        for y in 0..cliff {
            let f = sim
                .fluid([0, y, 0])
                .unwrap_or_else(|| panic!("{v} m: dry at {y}"));
            assert_eq!(f.surface, Fluid::FULL_SURFACE, "{v} m at {y}");
        }
        // Landed and spread 7 m again from the foot of the fall.
        let mut x = 1;
        while sim.fluid([x, 0, 0]).is_some() {
            x += 1;
        }
        assert_eq!((x - 1) as f32 * v, 7.0, "{v} m");
        // Nothing climbed: no water above the floor beyond the fall.
        for x in 1..=n {
            assert!(sim.fluid([x, 1, 0]).is_none(), "{v} m: water rose at {x}");
        }
        assert_eq!(sim.sources(), 1);
    }
}

/// Terraces 4 m long and 1 m apart carry a stream further than any one reach:
/// each drop renews it, and there is no bound on the total.
#[test]
fn successive_drops_renew_reach_with_no_bound_on_the_total() {
    for v in SIZES {
        let run = (4.0 / v) as i32;
        let drop = (1.0 / v) as i32;
        let terraces = 4;
        let width = run * terraces + run;
        let top = drop * terraces + 2;
        let mut sim = Sim::new(v, [-1, -1, -1], [width + 1, top + 1, 1]);
        let floor = |x: i32| drop * (terraces - (x / run).min(terraces));
        for x in -1..=width + 1 {
            for y in -1..=top + 1 {
                sim.build([x, y, -1], stone());
                sim.build([x, y, 1], stone());
            }
            let f = if x < 0 { top + 1 } else { floor(x) };
            for y in -1..f {
                sim.build([x, y, 0], stone());
            }
        }
        sim.place_source([0, floor(0), 0]);
        sim.settle();
        let reached = (0..=width)
            .filter(|&x| sim.fluid([x, floor(x), 0]).is_some())
            .max();
        assert!(
            reached.unwrap() as f32 * v >= 4.0 * terraces as f32 + 3.0,
            "{v} m: stopped at {reached:?}"
        );
    }
}

/// A drop renews reach once gravity descent reaches the reset drop, 0.5 m: a
/// single quarter-metre step does not, and a half-metre one does.
#[test]
fn reach_renews_after_half_a_metre_of_descent_and_not_at_a_voxel_boundary() {
    // Floor at y = 0 up to `step_at_m`, then `drop_m` lower.
    let spread = |v: f32, step_at_m: f32, drop_m: f32| {
        let n = (20.0 / v) as i32;
        let at = (step_at_m / v) as i32;
        let drop = (drop_m / v).round() as i32;
        let mut sim = Sim::new(v, [-1, -drop - 1, -1], [n + 1, 3, 1]);
        for x in -1..=n + 1 {
            for y in -drop - 1..=3 {
                sim.build([x, y, -1], stone());
                sim.build([x, y, 1], stone());
            }
            let floor = if x <= at { 0 } else { -drop };
            for y in -drop - 1..floor {
                sim.build([x, y, 0], stone());
            }
        }
        sim.build([-1, 0, 0], stone());
        sim.place_source([0, 0, 0]);
        sim.settle();
        let mut x = at + 1;
        while sim.fluid([x, -drop, 0]).is_some() {
            x += 1;
        }
        // Metres of floor wet beyond the foot of the step.
        (x - at - 2) as f32 * v
    };
    // At 0.25 m: one quarter-metre step keeps what was left of 7 m...
    assert_eq!(spread(0.25, 3.0, 0.25), 7.0 - 3.0 - 0.25);
    // ...and a half-metre one renews it, as it does at 0.5 m.
    assert_eq!(spread(0.25, 3.0, 0.5), 7.0);
    assert_eq!(spread(0.5, 3.0, 0.5), 7.0);
    // A metre renews at every size.
    for v in SIZES {
        assert_eq!(spread(v, 3.0, 1.0), 7.0, "{v} m");
    }
}

/// Flow fills down and along but never up: the far arm of a U stays dry, and
/// so does the Cave above the stream's floor.
#[test]
fn flow_never_climbs_to_fill_a_u_or_a_cave_towards_the_source() {
    let mut sim = Sim::new(1.0, [-1, -1, -1], [8, 8, 1]);
    for x in -1..=8 {
        for y in -1..=8 {
            sim.build([x, y, -1], stone());
            sim.build([x, y, 1], stone());
        }
    }
    // Columns x = 0 and x = 6 open from y = 1 to 7, joined along y = 1.
    for x in -1..=8 {
        for y in -1..=8 {
            let open = (y >= 1 && (x == 0 || x == 6)) || (y == 1 && (0..=6).contains(&x));
            if !open {
                sim.build([x, y, 0], stone());
            }
        }
    }
    sim.place_source([0, 7, 0]);
    sim.settle();
    for y in 1..7 {
        assert!(sim.fluid([0, y, 0]).is_some(), "the drop at {y}");
    }
    assert!(sim.fluid([6, 1, 0]).is_some(), "along the bottom");
    for y in 2..=7 {
        assert!(sim.fluid([6, y, 0]).is_none(), "climbed to {y}");
    }
}

/// A ring of flow around a pillar is the cycle a naive recession leaves
/// standing: every Voxel in it has a wet neighbour. Taking the source away
/// clears all of it.
#[test]
fn removing_the_source_clears_a_ring_that_could_support_itself() {
    let mut sim = Sim::new(1.0, [-4, 0, -4], [4, 3, 4]);
    for x in -4..=4 {
        for z in -4..=4 {
            sim.build([x, 0, z], stone());
            let ring = x.abs().max(z.abs()) == 1;
            if !ring {
                for y in 1..=3 {
                    sim.build([x, y, z], stone());
                }
            }
        }
    }
    sim.place_source([0, 1, 1]);
    sim.settle();
    let ring = sim.wet().len();
    assert_eq!(ring, 8, "the whole ring is wet");
    sim.remove_fluid([0, 1, 1]);
    sim.settle();
    assert!(sim.wet().is_empty(), "{:?}", sim.wet());
}

/// Blocking the stream next to its source dries what is downstream and keeps
/// the source and what it still feeds.
#[test]
fn blocking_a_stream_dries_downstream_and_keeps_the_source() {
    let mut sim = corridor(1.0, 12.0);
    sim.place_source([0, 1, 0]);
    sim.settle();
    sim.put([2, 1, 0], stone());
    sim.settle();
    assert_eq!(sim.fluid([0, 1, 0]), Some(Fluid::source(WATER)));
    assert!(sim.fluid([1, 1, 0]).is_some(), "still fed");
    for x in 3..=8 {
        assert!(sim.fluid([x, 1, 0]).is_none(), "{x} still wet");
    }
    assert!(
        sim.fluid([-7, 1, 0]).is_some(),
        "the other way is untouched"
    );
}

/// The quota bounds one call and changes nothing else: the state after every
/// step is the same however the step's evaluations are divided.
#[test]
fn dividing_steps_by_any_quota_gives_the_same_state_at_every_step() {
    let script = |quota: usize| {
        let mut sim = corridor(0.5, 6.0);
        sim.quota = Quota { evaluations: quota };
        sim.place_source([0, 1, 0]);
        let mut states = Vec::new();
        for step in 0..400 {
            if step == 150 {
                sim.put([3, 1, 0], stone());
            }
            if step == 250 {
                sim.remove_fluid([0, 1, 0]);
            }
            sim.run_step();
            states.push(sim.state());
        }
        (states, sim.work)
    };
    let (whole, work) = script(usize::MAX);
    for quota in [1, 3, 17] {
        let (divided, divided_work) = script(quota);
        assert!(whole == divided, "quota {quota} diverged");
        assert_eq!(work, divided_work, "quota {quota}");
    }
}

/// A call never evaluates more than its quota, and hands back the rest.
#[test]
fn one_call_evaluates_at_most_its_quota() {
    let mut sim = corridor(1.0, 12.0);
    sim.place_source([0, 1, 0]);
    let due = sim.agenda.take_due(0).cells;
    assert!(due.len() > 3);
    let total = due.len();
    let cont = Continuation::start(0, due, Timers::default());
    let quota = Quota { evaluations: 3 };
    let Advance::Partial(cont) = advance(&sim.grid, &sim.tables, &sim.scale, cont, quota) else {
        panic!("finished past its quota");
    };
    assert_eq!(cont.remaining(), total - 3);
    assert_eq!(cont.work.evaluated, 3);
}

/// Unknown terrain is never air: flow stops at the edge of what is held and the
/// Voxel beside it decides nothing.
#[test]
fn flow_pauses_at_terrain_nobody_holds() {
    // The region ends at x = 3; the corridor would carry on.
    let mut sim = corridor(1.0, 12.0);
    sim.grid.max[0] = 3;
    sim.place_source([0, 1, 0]);
    sim.settle();
    assert!(sim.fluid([2, 1, 0]).is_some());
    assert!(sim.fluid([3, 1, 0]).is_none(), "decided against unknown");
    assert!(sim.work.paused > 0);
}

/// Quiet water costs nothing: once settled, nothing is waiting and no step
/// evaluates anything, however long it runs.
#[test]
fn settled_flow_does_no_further_work() {
    let mut sim = corridor(1.0, 12.0);
    sim.place_source([0, 1, 0]);
    sim.settle();
    let before = sim.work;
    sim.run(500);
    assert_eq!(sim.work, before);
    assert!(sim.timers.is_empty());
}

/// Flow goes around a solid and not through it, and placing a source on
/// flowing water makes that Voxel a source and nothing else.
#[test]
fn solids_block_flow_and_a_source_placed_on_flow_is_the_only_new_source() {
    let mut sim = corridor(1.0, 12.0);
    sim.put([3, 1, 0], stone());
    sim.place_source([0, 1, 0]);
    sim.settle();
    assert!(sim.fluid([4, 1, 0]).is_none(), "through the stone");
    assert!(sim.fluid([2, 1, 0]).is_some_and(|f| !f.source));
    sim.place_source([-3, 1, 0]);
    sim.settle();
    assert_eq!(sim.sources(), 2);
    // Fed from both, it reaches further left than one source alone did.
    assert!(sim.fluid([-10, 1, 0]).is_some());
}

/// Generated terrain, as a worker sees it: generated lake water is the packed
/// cube generation writes, and it is a source. A shaft dug through a lake's bed
/// fills with falling water and nothing else changes: no flow becomes a source
/// and nothing rises above the waterline.
#[test]
fn a_shaft_dug_under_a_generated_lake_fills_from_it() {
    use crate::worldgen::{Worldgen, WorldgenParams};
    let generator = Worldgen::new(7, 1.0, WorldgenParams::default());
    let level = generator
        .water_level_m()
        .expect("the default World has water") as i32;
    let tables = Tables::builtin();
    let size = CHUNK_SIZE as i32;
    // A lake Voxel over a bed at least five Voxels deep, in the generator's own
    // output near the origin.
    let cy = level.div_euclid(size);
    let (bed, chunk) = (-8..8)
        .flat_map(|cx| (-8..8).flat_map(move |cz| [[cx, cy - 1, cz], [cx, cy, cz]]))
        .find_map(|c| {
            let voxels = generator.chunk(c[0], c[1], c[2]);
            let solid = |y: usize, x: usize, z: usize| {
                let v = voxels.get(x, y, z);
                v.is_full_cube() && tables.materials.solid(v)
            };
            for x in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    for y in 5..CHUNK_SIZE - 1 {
                        let wet = voxels.get(x, y + 1, z) == Voxel::cube(WATER);
                        if wet && (y - 5..=y).all(|y| solid(y, x, z)) {
                            let at = [
                                c[0] * size + x as i32,
                                c[1] * size + y as i32,
                                c[2] * size + z as i32,
                            ];
                            return Some((at, c));
                        }
                    }
                }
            }
            None
        })
        .expect("a lake near the origin");
    let mut region = Region::default();
    for dx in -1..=1 {
        for dy in -1..=1 {
            for dz in -1..=1 {
                let c = [chunk[0] + dx, chunk[1] + dy, chunk[2] + dz];
                let voxels = generator.chunk(c[0], c[1], c[2]);
                region.insert(c, Arc::new(voxels), Arc::new(ChunkOverlay::none()));
            }
        }
    }
    let read = |region: &Region, at: Cell| {
        let (v, o) = region.stored(at).unwrap();
        Occupants::of(v, o, &tables.fluids)
    };
    let above_bed = [bed[0], bed[1] + 1, bed[2]];
    assert_eq!(
        read(&region, above_bed).fluid,
        Some(Fluid::source(WATER)),
        "generated water reads as a source"
    );
    let write = |region: &mut Region, at: Cell, occupants: Occupants| {
        let (c, [x, y, z]) = chunk_of(at);
        let (voxel, overlay) = occupants.encode(&tables.fluids);
        let (voxels, overlays) = region.chunks.get_mut(&c).unwrap();
        Arc::make_mut(voxels).set(x, y, z, voxel);
        Arc::make_mut(overlays).set(x, y, z, overlay);
    };
    let sources_before: usize = count_sources(&region, &tables);

    let scale = FlowScale::new(&tables, 1.0);
    let mut agenda = Agenda::default();
    let shaft: Vec<Cell> = (0..5).map(|d| [bed[0], bed[1] - d, bed[2]]).collect();
    for &at in &shaft {
        write(
            &mut region,
            at,
            Occupants {
                solid: Voxel::AIR,
                fluid: None,
                burning: None,
            },
        );
        wake_around(&mut agenda, at, 0, Reach::default());
    }
    let mut timers = Timers::default();
    let mut step = 0;
    while !agenda.is_empty() {
        let due = agenda.take_due(step).cells;
        let cont = Continuation::start(step, due, std::mem::take(&mut timers));
        let Advance::Complete(done) = advance(&region, &tables, &scale, cont, UNLIMITED) else {
            unreachable!("unlimited");
        };
        for p in &done.proposals {
            assert!(p.next.fluid.is_none_or(|f| !f.source));
            assert!(p.at[1] <= level, "water rose to {}", p.at[1]);
            write(&mut region, p.at, p.next);
            wake_around(&mut agenda, p.at, step + 1, Reach::default());
        }
        for &(s, at) in &done.wakeups {
            agenda.rewake(at, s);
        }
        agenda.end_step();
        timers = done.timers;
        step += 1;
        assert!(step < 20_000, "never settled");
    }
    for &at in &shaft {
        assert!(
            read(&region, at).fluid.is_some(),
            "the shaft is dry at {at:?}"
        );
    }
    assert_eq!(count_sources(&region, &tables), sources_before);
}

fn count_sources(region: &Region, tables: &Tables) -> usize {
    let mut n = 0;
    for (voxels, overlay) in region.chunks.values() {
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let o =
                        Occupants::of(voxels.get(x, y, z), overlay.get(x, y, z), &tables.fluids);
                    n += usize::from(o.fluid.is_some_and(|f| f.source));
                }
            }
        }
    }
    n
}

/// The cells outside a Chunk that read into it are exactly the spans
/// `readers_outside` names, and those inside that read out lie on its faces:
/// checked against [`reads`] Voxel by Voxel, for a Chunk at negative
/// coordinates, with and without connected Shapes widening what is read.
#[test]
fn the_voxels_reading_across_a_chunk_boundary_are_the_ones_named() {
    let chunk = [-1, -2, 0];
    let home = crate::agenda::Span::chunk(chunk);
    for (connected, contacts) in [(false, false), (false, true), (true, false), (true, true)] {
        let reach = Reach {
            connected,
            contacts,
        };
        let read = reads(reach);
        let readers = readers_outside(chunk, reach);
        let faces = reading_outside(chunk, reach);
        let reads_into = |c: Cell| read.iter().any(|&r| home.contains_cell(step_by(c, r)));
        let reads_out = |c: Cell| read.iter().any(|&r| !home.contains_cell(step_by(c, r)));
        for x in home.min[0] - 3..=home.max[0] + 3 {
            for y in home.min[1] - 3..=home.max[1] + 3 {
                for z in home.min[2] - 3..=home.max[2] + 3 {
                    let c = [x, y, z];
                    if home.contains_cell(c) {
                        if reads_out(c) {
                            assert!(faces.iter().any(|s| s.contains_cell(c)), "{c:?}");
                        }
                        continue;
                    }
                    let named = readers.iter().any(|s| s.contains_cell(c));
                    assert_eq!(named, reads_into(c), "{reach:?} {c:?}");
                }
            }
        }
        assert!(readers.iter().all(|s| s.home() != chunk));
    }
    let wide = Reach {
        connected: true,
        contacts: false,
    };
    let touching = Reach {
        connected: false,
        contacts: true,
    };
    assert!(READS.iter().all(|r| reads(wide).contains(r)));
    assert!(reads(wide).len() > READS.len());
    assert_eq!(reads(Reach::default()), READS);
    assert!(reads(touching).contains(&[0, -1, 0]) && !READS.contains(&[0, -1, 0]));
    for d in FACE_STEPS {
        assert!(reads(touching).contains(&d), "a face it can touch: {d:?}");
    }
}

/// A Voxel waiting on a timer when what it reads goes unknown drops the timer:
/// nothing decided from terrain that is gone lands later.
#[test]
fn going_unknown_cancels_a_waiting_change() {
    let mut sim = corridor(1.0, 12.0);
    sim.place_source([0, 1, 0]);
    sim.run(2);
    assert_eq!(sim.timers.len(), 2, "the neighbours are waiting");
    sim.grid.max[0] = 1;
    sim.grid.min[0] = -1;
    sim.agenda.wake([1, 1, 0], sim.step);
    sim.agenda.wake([-1, 1, 0], sim.step);
    sim.run_step();
    assert!(sim.timers.is_empty(), "{} timers left", sim.timers.len());
    sim.grid.max[0] = 13;
    sim.grid.min[0] = -13;
    sim.wake_all();
    sim.settle();
    assert_eq!(wet_length_m(&sim, 1), 7.0, "admission resumes the flow");
}

/// However a step's Voxels are divided among workers, and whichever finishes
/// first, the state after every step and the work are the same.
#[test]
fn dividing_a_step_among_workers_in_any_order_changes_nothing() {
    let script = |slices: usize| {
        let mut sim = corridor(0.5, 6.0);
        sim.place_source([0, 1, 0]);
        let mut states = Vec::new();
        for step in 0..300 {
            if step == 120 {
                sim.put([3, 1, 0], stone());
            }
            if step == 200 {
                sim.remove_fluid([0, 1, 0]);
            }
            if slices == 1 {
                sim.run_step();
            } else {
                sim.run_step_sliced(slices);
            }
            states.push(sim.state());
        }
        (states, sim.work)
    };
    let (whole, work) = script(1);
    for slices in [2, 3, 7] {
        let (divided, divided_work) = script(slices);
        assert!(whole == divided, "{slices} slices diverged");
        assert_eq!(work, divided_work);
    }
}

/// An agenda far too small for the stream slows it and changes nothing it
/// settles to: every wake it could not hold came back through a rescan.
#[test]
fn a_tiny_agenda_slows_flow_but_settles_to_the_same_state() {
    let run = |limits: Option<crate::agenda::Limits>| {
        let mut sim = corridor(0.25, 6.0);
        if let Some(limits) = limits {
            sim.agenda = Agenda::new(limits);
        }
        sim.place_source([0, 1, 0]);
        let mut peak = 0;
        let mut steps = 0;
        while !sim.agenda.is_empty() {
            sim.run_step();
            peak = peak.max(sim.agenda.len());
            steps += 1;
            assert!(steps < 100_000, "never settled");
        }
        sim.remove_fluid([0, 1, 0]);
        while !sim.agenda.is_empty() {
            sim.run_step();
            peak = peak.max(sim.agenda.len());
            steps += 1;
            assert!(steps < 100_000, "never settled");
        }
        (sim.state(), peak, steps, sim.agenda.overflowed())
    };
    let (free, _, free_steps, none) = run(None);
    let limits = crate::agenda::Limits {
        capacity: 6,
        reserve: 2,
        scan_per_step: 5,
    };
    let (tight, peak, tight_steps, overflowed) = run(Some(limits));
    assert_eq!(none, 0);
    assert!(overflowed > 0, "the bound never bit");
    assert!(peak <= 8, "held {peak}");
    assert!(tight_steps > free_steps);
    assert!(free == tight, "a coalesced wake was lost");
}

mod waterlogging {
    use super::*;
    use crate::mesh::{NEG_X, NEG_Y, POS_X};
    use crate::shape::{Arm, ConnectDef, NodeBox, Orientation, ShapeDef};
    use crate::{SHAPE_SLAB, SHAPE_STAIR};

    fn slab(up: usize) -> Voxel {
        Voxel::new(STONE, SHAPE_SLAB, Orientation::new(up, 0).bits())
    }

    fn stair(turn: u8) -> Voxel {
        Voxel::new(
            STONE,
            SHAPE_STAIR,
            Orientation::new(crate::mesh::POS_Y, turn).bits(),
        )
    }

    /// Every void's level here, wet or not.
    fn levels(sim: &Sim, at: Cell) -> Vec<bool> {
        let here = sim.occupants(at);
        let voids = sim.tables.shapes.voids(here.solid, Connections::NONE).len();
        (0..voids)
            .map(|r| here.fluid.is_some_and(|f| f.level(r).is_wet()))
            .collect()
    }

    /// A walled corridor along x at y = 1 over a stone floor, `n` each way,
    /// with `tables`.
    fn lane(tables: Tables, n: i32) -> Sim {
        let mut sim = Sim::with_tables(1.0, [-n - 1, 0, -1], [n + 1, 3, 1], tables);
        for x in -n - 1..=n + 1 {
            sim.build([x, 0, 0], stone());
            for y in 0..=3 {
                sim.build([x, y, -1], stone());
                sim.build([x, y, 1], stone());
            }
        }
        sim
    }

    /// Water falls into a stair's void and the stair stays the solid it was;
    /// flow along the floor never climbs into it; the void recedes with its
    /// supply.
    #[test]
    fn a_stair_fills_from_above_keeps_its_solid_and_recedes() {
        let mut sim = lane(Tables::builtin(), 6);
        sim.build([2, 1, 0], stair(1));
        sim.place_source([-2, 1, 0]);
        sim.settle();
        assert!(sim.fluid([1, 1, 0]).is_some());
        assert_eq!(sim.fluid([2, 1, 0]), None, "flow climbed into the stair");

        sim.place_source([2, 2, 0]);
        sim.settle();
        let wet = sim.occupants([2, 1, 0]);
        assert_eq!(wet.solid, stair(1));
        let f = wet.fluid.expect("the stair is waterlogged");
        assert!(!f.source);
        assert_eq!(f.surface, Fluid::FULL_SURFACE);
        assert!(
            sim.fluid([3, 1, 0]).is_some(),
            "it spreads out of the stair"
        );

        sim.remove_fluid([2, 2, 0]);
        sim.remove_fluid([-2, 1, 0]);
        sim.settle();
        assert!(sim.wet().is_empty(), "{:?}", sim.wet());
        assert_eq!(sim.occupants([2, 1, 0]).solid, stair(1));
    }

    /// A stair's sealed back and a bottom slab beside a top slab pass nothing,
    /// though each face is open somewhere; the same pairs turned to share an
    /// opening do pass.
    #[test]
    fn sealed_and_jointly_sealed_faces_pass_nothing() {
        let tables = Tables::builtin();
        let back_to_west = (0..4)
            .map(stair)
            .find(|&v| tables.shapes.seals(v, NEG_X))
            .expect("a stair turned with its back west");
        let open_to_west = (0..4)
            .map(stair)
            .find(|&v| !tables.shapes.seals(v, NEG_X) && !tables.shapes.seals(v, POS_X))
            .expect("a stair turned side on");
        for (neighbour, passes) in [
            (slab(crate::mesh::NEG_Y), false),
            (slab(crate::mesh::POS_Y), true),
            (back_to_west, false),
            (open_to_west, true),
        ] {
            let mut sim = lane(Tables::builtin(), 4);
            sim.build([0, 1, 0], slab(crate::mesh::POS_Y));
            sim.build([1, 1, 0], neighbour);
            sim.build([-1, 1, 0], stone());
            for x in -4..=4 {
                if x != 0 {
                    sim.build([x, 2, 0], stone());
                }
            }
            sim.place_source([0, 2, 0]);
            sim.settle();
            assert!(sim.fluid([0, 1, 0]).is_some(), "the slab fills");
            assert_eq!(
                sim.fluid([1, 1, 0]).is_some(),
                passes,
                "{neighbour:?} beside a bottom slab"
            );
        }
    }

    /// A wall through the middle of a Voxel: water on one side stays there,
    /// and the void across the partition stays dry though its Voxel is wet.
    #[test]
    fn a_partition_keeps_the_far_void_and_beyond_dry() {
        let mut tables = Tables::builtin();
        let wall = tables
            .shapes
            .register(ShapeDef {
                connect: None,
                boxes: vec![NodeBox::new([7, 0, 0], [9, 16, 16])],
                collision: None,
            })
            .unwrap();
        let mut sim = lane(tables, 5);
        sim.build([1, 1, 0], Voxel::new(STONE, wall, 0));
        sim.place_source([-2, 1, 0]);
        sim.settle();
        assert_eq!(levels(&sim, [1, 1, 0]), vec![true, false]);
        assert_eq!(sim.fluid([2, 1, 0]), None, "flow crossed the partition");
        assert!(sim.fluid([0, 1, 0]).is_some());
    }

    /// A shelf splits a Voxel into a void above and one below, fed separately
    /// at different reach. Each keeps its own state, so neither can lend reach
    /// to the other and, with the source gone, a loop through them dries.
    #[test]
    fn separate_voids_keep_separate_state_and_dry_without_supply() {
        let mut tables = Tables::builtin();
        let shelf = tables
            .shapes
            .register(ShapeDef {
                connect: None,
                boxes: vec![NodeBox::new([0, 6, 0], [16, 8, 16])],
                collision: None,
            })
            .unwrap();
        let shelf = Voxel::new(STONE, shelf, 0);
        let mut sim = lane(tables, 8);
        for x in 1..=3 {
            sim.build([x, 1, 0], shelf);
        }
        for x in -8..=8 {
            if x != 1 {
                sim.build([x, 2, 0], stone());
            }
        }
        sim.place_source([1, 2, 0]);
        sim.settle();
        // Above: 7 m renewed by the fall in at x = 1, one less a step. Below:
        // renewed by the half Voxel off the end at x = 4, and back one step.
        let f = sim.fluid([3, 1, 0]).expect("wet");
        // Voids are numbered from their lowest sub-cell, so the one below is 0.
        let (bottom, top) = (f.level(0), f.level(1));
        assert_eq!((top.reach, bottom.reach), (5, 6), "{f:?}");

        sim.remove_fluid([1, 2, 0]);
        sim.settle();
        assert!(sim.wet().is_empty(), "{:?}", sim.wet());
    }

    /// A floor `h_m` up, as stone and a bottom slab where it falls half a Voxel
    /// short; returns the layer water stands in.
    fn floor(sim: &mut Sim, x: i32, h_m: f32) -> i32 {
        let half = (h_m / sim.voxel_m * 2.0).round() as i32;
        for y in 0..half / 2 {
            sim.build([x, y, 0], stone());
        }
        if half % 2 == 1 {
            sim.build([x, half / 2, 0], slab(crate::mesh::POS_Y));
        }
        half / 2
    }

    /// A drop of `drop_m` from a floor at `top_m` within slabs: metres of the
    /// lower floor wet beyond the foot of the step.
    fn spread_below(v: f32, top_m: f32, drop_m: f32) -> f32 {
        let n = (20.0 / v) as i32;
        let at = (3.0 / v) as i32;
        let mut sim = Sim::new(v, [-1, -1, -1], [n + 1, 6, 1]);
        for x in -1..=n + 1 {
            for y in -1..=6 {
                sim.build([x, y, -1], stone());
                sim.build([x, y, 1], stone());
            }
            sim.build([x, -1, 0], stone());
        }
        for y in 0..=6 {
            sim.build([-1, y, 0], stone());
        }
        let mut layer = 0;
        let mut low = 0;
        for x in 0..=n {
            let l = floor(&mut sim, x, if x <= at { top_m } else { top_m - drop_m });
            if x == 0 {
                layer = l;
            }
            if x > at {
                low = l;
            }
        }
        sim.place_source([0, layer, 0]);
        sim.settle();
        let mut x = at + 1;
        while sim.fluid([x, low, 0]).is_some() {
            x += 1;
        }
        (x - at - 2) as f32 * v
    }

    /// Half a metre of descent renews reach though part of it, or all, is
    /// inside a slab's Voxel: at 1 m the half-Voxel fall into a slab, at 0.5 m
    /// a step off a slab and a fall into one, at 0.25 m two of those around a
    /// whole Voxel. A quarter metre does not.
    #[test]
    fn half_a_metre_of_descent_inside_shapes_renews_reach_at_every_size() {
        for v in SIZES {
            assert_eq!(spread_below(v, 0.5 + v / 2.0, 0.5), 7.0, "{v} m");
        }
        assert!(spread_below(0.5, 0.5, 0.25) < 7.0 - 3.0);
    }

    /// A wall with a window that a connected arm plugs: whether the far void
    /// is reachable follows the arm, and taking away what the arm reaches for
    /// opens the passage.
    #[test]
    fn a_connected_arm_closes_and_opens_passage() {
        let mut tables = Tables::builtin();
        let b = NodeBox::new;
        let window = tables
            .shapes
            .register(ShapeDef {
                connect: Some(ConnectDef {
                    group: 1 << 5,
                    connects_to: 1 << 5,
                    to_sealed: true,
                    arms: vec![Arm {
                        face: POS_X,
                        boxes: vec![b([6, 6, 7], [16, 10, 9])],
                        collision: None,
                    }],
                }),
                boxes: vec![
                    b([0, 0, 7], [16, 6, 9]),
                    b([0, 10, 7], [16, 16, 9]),
                    b([0, 6, 7], [6, 10, 9]),
                    b([10, 6, 7], [16, 10, 9]),
                ],
                collision: None,
            })
            .unwrap();
        let window = Voxel::new(STONE, window, 0);
        let arm = Connections::NONE.with(POS_X);
        assert_eq!(tables.shapes.voids(window, Connections::NONE).len(), 1);
        assert_eq!(tables.shapes.voids(window, arm).len(), 2);

        let mut sim = Sim::with_tables(1.0, [-2, 0, -2], [2, 2, 2], tables);
        for x in -2..=2 {
            for z in -2..=2 {
                sim.build([x, 0, z], stone());
                sim.build([x, 2, z], stone());
                for y in 0..=2 {
                    if x.abs() == 2 || z.abs() == 2 {
                        sim.build([x, y, z], stone());
                    }
                }
            }
        }
        for (x, z) in [(-1, 0), (-1, 1), (1, 1), (-1, -1), (1, -1)] {
            sim.build([x, 1, z], stone());
        }
        sim.build([0, 1, 0], window);
        sim.build([1, 1, 0], stone());
        sim.place_source([0, 1, -1]);
        sim.wake_all();
        sim.settle();
        let wet = |sim: &Sim| {
            let f = sim.fluid([0, 1, 0]);
            (0..2)
                .map(|r| f.is_some_and(|f| f.level(r).is_wet()))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            wet(&sim),
            vec![true, false],
            "the arm keeps the far void dry"
        );
        assert_eq!(sim.fluid([0, 1, 1]), None, "and what lies past it");

        sim.put([1, 1, 0], Voxel::AIR);
        sim.settle();
        assert_eq!(wet(&sim), vec![true, false], "one void, wet");
        assert!(sim.fluid([0, 1, 1]).is_some(), "on through the window");

        sim.put([1, 1, 0], stone());
        sim.settle();
        assert_eq!(wet(&sim), vec![true, false]);
        assert_eq!(sim.fluid([0, 1, 1]), None, "the arm closes it again");
    }

    /// The kernel's state with waterlogged Shapes is the same however a step
    /// is divided.
    #[test]
    fn waterlogged_flow_is_the_same_under_any_division() {
        let build = || {
            let mut sim = lane(Tables::builtin(), 8);
            for x in 1..=5 {
                sim.build([x, 1, 0], slab(crate::mesh::POS_Y));
            }
            sim.build([6, 1, 0], stair(0));
            sim.place_source([1, 2, 0]);
            sim
        };
        let mut a = build();
        let mut b = build();
        b.quota = Quota { evaluations: 3 };
        for _ in 0..300 {
            a.run_step();
            b.run_step();
            assert_eq!(a.state(), b.state());
        }
        assert!(a.fluid([5, 1, 0]).is_some());
        let _ = NEG_Y;
    }
}

/// Fluids other than water, run from their declared rows alone.
mod declared {
    use super::*;
    use crate::combustion::Heat;
    use crate::material::{MaterialDef, Occlusion, RenderClass};
    use crate::{Orientation, SHAPE_STAIR};

    /// The spec's proposed demo lava: 0.75 m/s does not divide a step at any
    /// of the three sizes.
    pub(super) const LAVA: FluidDef = FluidDef {
        reach_m: 3.0,
        sideways_m_per_s: 0.75,
        downward_m_per_s: 1.5,
        recession_m_per_s: 0.75,
        reset_drop_m: 0.5,
        waterlogs: false,
        extinguishes: false,
        heat: Some(Heat {
            strength: 12,
            reach_m: 2.0,
        }),
    };

    /// Neither water nor lava: shorter reach, its own rates and reset drop.
    pub(super) const SLIME: FluidDef = FluidDef {
        reach_m: 2.0,
        sideways_m_per_s: 2.0,
        downward_m_per_s: 3.0,
        recession_m_per_s: 1.0,
        reset_drop_m: 1.0,
        waterlogs: true,
        extinguishes: false,
        heat: None,
    };

    pub(super) fn liquid(tables: &mut Tables, emission: u8) -> MaterialId {
        tables
            .materials
            .register(MaterialDef {
                emission,
                dimming: 2,
                occlusion: Occlusion::SameMaterial,
                render: RenderClass::Blend,
                solid: false,
            })
            .unwrap()
    }

    /// Water's built-in table plus `defs`, registered in order.
    pub(super) fn with(defs: &[FluidDef]) -> (Tables, Vec<MaterialId>) {
        let mut tables = Tables::builtin();
        let ids = defs
            .iter()
            .map(|def| {
                let id = liquid(&mut tables, 0);
                tables.register_fluid(id, *def).unwrap();
                id
            })
            .collect();
        (tables, ids)
    }

    pub(super) fn corridor_of(tables: Tables, voxel_m: f32, half_m: f32) -> Sim {
        let mut sim = corridor(voxel_m, half_m);
        sim.scale = FlowScale::new(&tables, voxel_m);
        sim.tables = tables;
        sim
    }

    pub(super) fn source(sim: &mut Sim, at: Cell, material: MaterialId) {
        let here = sim.occupants(at);
        sim.edit(
            at,
            Occupants {
                fluid: Some(Fluid::source(material)),
                ..here
            },
        );
    }

    /// The step count after which each Voxel along `x_of` first held fluid.
    fn arrivals(sim: &mut Sim, cells: &[Cell], steps: Step) -> Vec<Option<Step>> {
        let mut first = vec![None; cells.len()];
        for _ in 0..steps {
            sim.run_step();
            for (i, at) in cells.iter().enumerate() {
                if first[i].is_none() && sim.fluid(*at).is_some() {
                    first[i] = Some(sim.step);
                }
            }
        }
        first
    }

    /// Every Voxel of a lava stream arrives within one step of its exact
    /// physical time, both ways along the corridor and at every size. Rounding
    /// each Voxel's delay alone would be four steps early at 3 m on 0.25 m.
    #[test]
    fn a_rate_that_does_not_divide_a_step_keeps_its_time_over_the_whole_path() {
        for v in SIZES {
            let (tables, ids) = with(&[LAVA]);
            let mut sim = corridor_of(tables, v, 5.0);
            source(&mut sim, [0, 1, 0], ids[0]);
            let n = sim.cells(LAVA.reach_m);
            let cells: Vec<Cell> = (1..=n).flat_map(|x| [[x, 1, 0], [-x, 1, 0]]).collect();
            let seconds = 2.0 * LAVA.reach_m as f64 / LAVA.sideways_m_per_s as f64;
            let first = arrivals(&mut sim, &cells, (seconds / STEP_S) as Step);
            for (at, step) in cells.iter().zip(first) {
                let metres = at[0].abs() as f64 * v as f64;
                let exact = metres / LAVA.sideways_m_per_s as f64 / STEP_S;
                let step = step.unwrap_or_else(|| panic!("{v} m: {at:?} never wet"));
                assert!(
                    (step as f64 - exact).abs() <= 1.0,
                    "{v} m: {at:?} at step {step}, exact {exact:.2}"
                );
            }
            sim.settle();
            assert_eq!(wet_length_m(&sim, 1), LAVA.reach_m, "{v} m");
        }
    }

    /// The same for a fall: down an open shaft at the downward rate.
    #[test]
    fn a_fall_keeps_its_time_to_within_one_step_at_every_size() {
        for v in SIZES {
            let (tables, ids) = with(&[LAVA]);
            let depth = (4.0 / v).round() as i32;
            let mut sim = Sim::with_tables(v, [-1, 0, -1], [1, depth + 1, 1], tables);
            for y in 0..=depth + 1 {
                for x in -1..=1 {
                    for z in -1..=1 {
                        if (x, z) != (0, 0) || y == 0 {
                            sim.build([x, y, z], stone());
                        }
                    }
                }
            }
            source(&mut sim, [0, depth, 0], ids[0]);
            let cells: Vec<Cell> = (1..depth).map(|y| [0, y, 0]).collect();
            let seconds = 4.0 / LAVA.downward_m_per_s as f64 + 1.0;
            let first = arrivals(&mut sim, &cells, (seconds / STEP_S) as Step);
            for (at, step) in cells.iter().zip(first) {
                let metres = (depth - at[1]) as f64 * v as f64;
                let exact = metres / LAVA.downward_m_per_s as f64 / STEP_S;
                let step = step.unwrap_or_else(|| panic!("{v} m: {at:?} never wet"));
                assert!(
                    (step as f64 - exact).abs() <= 1.0,
                    "{v} m: {at:?} at step {step}, exact {exact:.2}"
                );
            }
        }
    }

    /// Water, lava and a third fluid in the same corridor each spread their
    /// own metres at their own rate: the kernel reads rows, not a switch.
    #[test]
    fn each_declared_fluid_spreads_its_own_metres_at_its_own_rate() {
        for v in SIZES {
            for (def, material) in [(FluidDef::WATER, None), (LAVA, Some(0)), (SLIME, Some(1))] {
                let (tables, ids) = with(&[LAVA, SLIME]);
                let material = material.map_or(WATER, |i| ids[i]);
                let mut sim = corridor_of(tables, v, 8.0);
                source(&mut sim, [0, 1, 0], material);
                sim.run_seconds(1.0);
                let front = wet_length_m(&sim, 1);
                let expected = def.sideways_m_per_s.min(def.reach_m / 1.0);
                assert!(
                    (front - expected).abs() <= v + 1e-4,
                    "{v} m, {material}: {front} m after 1 s, expected {expected}"
                );
                sim.settle();
                assert_eq!(wet_length_m(&sim, 1), def.reach_m, "{v} m, {material}");
                assert_eq!(sim.sources(), 1);
                assert!(
                    sim.wet()
                        .iter()
                        .all(|at| sim.fluid(*at).unwrap().material == material)
                );
            }
        }
    }

    /// Two fluids reaching one empty Voxel together: the one registered first
    /// takes it whatever their Material ids, neither enters the other, and
    /// neither displaces the other's source.
    #[test]
    fn an_equal_claim_goes_to_the_earlier_registered_fluid_and_nothing_mixes() {
        let mut tables = Tables::builtin();
        let later = liquid(&mut tables, 0);
        let earlier = liquid(&mut tables, 0);
        assert!(later < earlier);
        tables.register_fluid(earlier, SLIME).unwrap();
        tables.register_fluid(later, SLIME).unwrap();
        let mut sim = corridor_of(tables, 1.0, 6.0);
        source(&mut sim, [-2, 1, 0], earlier);
        source(&mut sim, [2, 1, 0], later);
        sim.settle();
        let material = |sim: &Sim, x| sim.fluid([x, 1, 0]).map(|f| f.material);
        assert_eq!(material(&sim, 0), Some(earlier));
        assert_eq!(material(&sim, -1), Some(earlier));
        assert_eq!(material(&sim, 1), Some(later));
        assert_eq!(material(&sim, 3), Some(later));
        assert_eq!(material(&sim, -3), Some(earlier));
        for x in [-2, 2] {
            assert!(sim.fluid([x, 1, 0]).unwrap().source, "{x} displaced");
        }
        assert_eq!(sim.sources(), 2);
    }

    /// A fluid that does not waterlog flows past a stair's open notch that
    /// water would fill, and a stair placed into its source displaces it.
    #[test]
    fn a_fluid_that_does_not_waterlog_stays_out_of_shapes() {
        let (tables, ids) = with(&[LAVA]);
        let lava = ids[0];
        let stair = Voxel::new(
            STONE,
            SHAPE_STAIR,
            Orientation::new(crate::mesh::POS_Y, 1).bits(),
        );
        for (material, fills) in [(WATER, true), (lava, false)] {
            let mut sim = corridor_of(tables.clone(), 1.0, 6.0);
            sim.build([2, 1, 0], stair);
            source(&mut sim, [2, 2, 0], material);
            sim.settle();
            assert_eq!(sim.fluid([2, 1, 0]).is_some(), fills, "{material}");
            assert_eq!(sim.occupants([2, 1, 0]).solid, stair);
        }
        let pool = Occupants {
            solid: Voxel::AIR,
            fluid: Some(Fluid::source(lava)),
            burning: None,
        };
        assert_eq!(pool.place_solid(stair, &tables).fluid, None);
        let water = Occupants {
            fluid: Some(Fluid::source(WATER)),
            ..pool
        };
        assert!(water.place_solid(stair, &tables).fluid.is_some());
        assert_eq!(
            tables.vet_occupants(Occupants {
                solid: stair,
                ..pool
            }),
            Err(crate::PlaceFault::FluidExcluded)
        );
    }
}

/// Declared contact reactions, through the same kernel and landing as flow.
mod reactions {
    use super::declared::{LAVA, SLIME, corridor_of, liquid, source, with};
    use super::*;
    use crate::material::{MaterialDef, Occlusion, RenderClass};
    use crate::mesh::{NEG_X, POS_X, POS_Y};
    use crate::reaction::{Phase, Reactant, ReactionDef, Side};
    use crate::shape::{NodeBox, Orientation, ShapeDef};
    use crate::{SHAPE_STAIR, Voxel};

    fn solid(tables: &mut Tables) -> MaterialId {
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
    }

    fn any(fluid: MaterialId) -> Reactant {
        Reactant {
            fluid,
            phase: Phase::Any,
        }
    }

    /// `consumed` goes on touching `kept`, leaving `product`.
    fn rule(consumed: MaterialId, kept: MaterialId, product: Option<MaterialId>) -> ReactionDef {
        ReactionDef {
            first: any(consumed),
            second: any(kept),
            consumed: Side::First,
            product,
            receiving: None,
        }
    }

    /// The demo's rule: lava touching water becomes stone, water stays.
    fn cooling() -> (Tables, MaterialId) {
        let (mut tables, ids) = with(&[LAVA]);
        tables
            .register_reaction(rule(ids[0], WATER, Some(STONE)))
            .unwrap();
        (tables, ids[0])
    }

    fn material(sim: &Sim, at: Cell) -> Option<MaterialId> {
        sim.fluid(at).map(|f| f.material)
    }

    /// Whether any two face neighbours hold `a` and `b`.
    fn touching(sim: &Sim, a: MaterialId, b: MaterialId) -> bool {
        sim.wet().iter().any(|&at| {
            material(sim, at) == Some(a)
                && FACE_STEPS.iter().any(|&d| {
                    sim.grid.stored(step_by(at, d)).is_some()
                        && material(sim, step_by(at, d)) == Some(b)
                })
        })
    }

    /// Lava and water flowing towards each other along a corridor meet; the
    /// lava they touch turns to stone, the water is untouched, and the stone
    /// closes the corridor between them. Both sources stay. The same at
    /// every size, and mirrored when the two change ends.
    #[test]
    fn lava_meeting_water_becomes_stone_and_the_water_stays() {
        for v in SIZES {
            for flip in [1, -1] {
                let (tables, lava) = cooling();
                let mut sim = corridor_of(tables, v, 6.0);
                let end = sim.cells(4.0);
                source(&mut sim, [-flip * end, 1, 0], lava);
                source(&mut sim, [flip * end, 1, 0], WATER);
                sim.settle();
                let stones: Vec<i32> = (-end..=end)
                    .filter(|&x| sim.occupants([x, 1, 0]).solid == stone())
                    .collect();
                assert!(!stones.is_empty(), "{v} m {flip}: nothing reacted");
                assert!(
                    !touching(&sim, lava, WATER),
                    "{v} m {flip}: they still touch"
                );
                assert!(sim.fluid([-flip * end, 1, 0]).unwrap().source);
                assert!(sim.fluid([flip * end, 1, 0]).unwrap().source);
                assert_eq!(sim.sources(), 2);
                // Water reaches the stone; lava stays its own side of it.
                let wall = stones[0];
                for x in -end..=end {
                    let m = material(&sim, [x, 1, 0]);
                    if m.is_some() {
                        assert_eq!(
                            m == Some(WATER),
                            (x - wall) * flip > 0,
                            "{v} m {flip}: {m:?} at {x}"
                        );
                    }
                }
                assert!(sim.work.reacted > 0 && sim.work.reacted == stones.len());
            }
        }
    }

    /// A rule between two Mod fluids, named either way round, works the same
    /// way: the kernel reads the table, not a water/lava case.
    #[test]
    fn a_custom_pair_reacts_by_its_own_row() {
        for written_backwards in [false, true] {
            let (mut tables, ids) = with(&[SLIME, LAVA]);
            let (slime, other) = (ids[0], ids[1]);
            let ash = solid(&mut tables);
            let mut def = rule(other, slime, Some(ash));
            if written_backwards {
                def = ReactionDef {
                    first: def.second,
                    second: def.first,
                    consumed: Side::Second,
                    ..def
                };
            }
            tables.register_reaction(def).unwrap();
            let mut sim = corridor_of(tables, 1.0, 6.0);
            source(&mut sim, [-2, 1, 0], slime);
            source(&mut sim, [2, 1, 0], other);
            sim.settle();
            let solids: Vec<i32> = (-3..=3)
                .filter(|&x| sim.occupants([x, 1, 0]).solid == Voxel::cube(ash))
                .collect();
            assert!(!solids.is_empty(), "{written_backwards}");
            assert!(!touching(&sim, slime, other));
            assert!(sim.fluid([-2, 1, 0]).unwrap().source, "slime is kept");
            assert!(
                (-3..=3).all(|x| sim.occupants([x, 1, 0]).solid != stone()),
                "no stone: this pair leaves its own product"
            );
        }
    }

    /// A receiving side is gravity's: a rule that needs `a` to arrive at
    /// `b` matches `a` falling onto `b` and `a` beside it on a level floor,
    /// and never `a` below `b`. Source and flowing states are matched as
    /// declared.
    #[test]
    fn receiving_and_states_are_matched_as_declared() {
        let shaft = |tables: Tables, top: MaterialId, bottom: MaterialId| {
            let mut sim = Sim::with_tables(1.0, [-1, 0, -1], [1, 4, 1], tables);
            for y in 0..=4 {
                for x in -1..=1 {
                    for z in -1..=1 {
                        if (x, z) != (0, 0) || y == 0 {
                            sim.build([x, y, z], stone());
                        }
                    }
                }
            }
            source(&mut sim, [0, 1, 0], bottom);
            source(&mut sim, [0, 2, 0], top);
            sim.settle();
            (sim.fluid([0, 1, 0]), sim.fluid([0, 2, 0]))
        };
        let (base, ids) = with(&[SLIME, LAVA]);
        let (a, b) = (ids[0], ids[1]);
        let directed = |consumed_phase: Phase| {
            let mut tables = base.clone();
            tables
                .register_reaction(ReactionDef {
                    first: Reactant {
                        fluid: b,
                        phase: consumed_phase,
                    },
                    second: any(a),
                    consumed: Side::First,
                    product: None,
                    receiving: Some(Side::First),
                })
                .unwrap();
            tables
        };
        let (bottom, _) = shaft(directed(Phase::Any), a, b);
        let bottom = bottom.expect("a falls into where b was");
        assert_eq!(
            (bottom.material, bottom.source),
            (a, false),
            "a fell onto b: b goes"
        );
        let (bottom, top) = shaft(directed(Phase::Any), b, a);
        assert_eq!(
            bottom.map(|f| f.material),
            Some(a),
            "a below b arrives at nothing"
        );
        assert_eq!(top.map(|f| f.material), Some(b));
        let (bottom, _) = shaft(directed(Phase::Flowing), a, b);
        assert!(
            bottom.is_some_and(|f| f.source),
            "b is a source, the rule wants flow"
        );

        let mut sim = corridor_of(directed(Phase::Any), 1.0, 4.0);
        source(&mut sim, [0, 1, 0], b);
        source(&mut sim, [1, 1, 0], a);
        sim.settle();
        assert_eq!(material(&sim, [0, 1, 0]), Some(a), "level: a arrives at b");
    }

    /// A rule consuming slime touching water, with no product.
    fn dissolving() -> (Tables, MaterialId) {
        let (mut tables, ids) = with(&[SLIME]);
        tables.register_reaction(rule(ids[0], WATER, None)).unwrap();
        (tables, ids[0])
    }

    /// Water standing in one void of a walled Voxel never touches slime
    /// across the wall, and slime never enters the dry void beside it; with
    /// no wall the same water takes the slime source away.
    #[test]
    fn a_solid_partition_keeps_two_fluids_from_reacting_or_mixing() {
        for walled in [true, false] {
            let (mut tables, slime) = dissolving();
            let wall = tables
                .shapes
                .register(ShapeDef {
                    connect: None,
                    boxes: vec![NodeBox::new([7, 0, 0], [9, 16, 16])],
                    collision: None,
                })
                .unwrap();
            let mut sim = corridor_of(tables, 1.0, 5.0);
            if walled {
                sim.build([1, 1, 0], Voxel::new(STONE, wall, 0));
            }
            // Water reaches the walled Voxel before slime can.
            sim.place_source([0, 1, 0]);
            source(&mut sim, [2, 1, 0], slime);
            sim.settle();
            let here = sim.occupants([1, 1, 0]);
            assert_eq!(here.fluid.map(|f| f.material), Some(WATER), "{walled}");
            if walled {
                assert!(
                    !here.fluid.unwrap().level(1).is_wet(),
                    "the far void stays dry"
                );
                assert!(
                    sim.fluid([2, 1, 0])
                        .is_some_and(|f| f.source && f.material == slime)
                );
                assert_eq!(sim.work.reacted, 0);
            } else {
                assert_eq!(material(&sim, [2, 1, 0]), Some(WATER), "the slime went");
                assert!(
                    sim.wet()
                        .iter()
                        .all(|&at| material(&sim, at) == Some(WATER))
                );
            }
        }
    }

    /// A stair's sealed back between them is no contact either; turned
    /// side on, the same two touch through its notch.
    #[test]
    fn a_sealed_face_is_no_contact() {
        let (tables, slime) = dissolving();
        let stair = |turn| Voxel::new(STONE, SHAPE_STAIR, Orientation::new(POS_Y, turn).bits());
        let sealed = (0..4)
            .map(stair)
            .find(|&v| tables.shapes.seals(v, POS_X))
            .unwrap();
        let open = (0..4)
            .map(stair)
            .find(|&v| !tables.shapes.seals(v, NEG_X) && !tables.shapes.seals(v, POS_X))
            .unwrap();
        for (shape, reacts) in [(sealed, false), (open, true)] {
            let mut sim = corridor_of(tables.clone(), 1.0, 4.0);
            sim.build([0, 1, 0], shape);
            sim.place_source([0, 1, 0]);
            source(&mut sim, [1, 1, 0], slime);
            sim.settle();
            assert_eq!(sim.occupants([0, 1, 0]).solid, shape, "the stair stays");
            assert_eq!(
                material(&sim, [1, 1, 0]) != Some(slime),
                reacts,
                "{shape:?}"
            );
        }
    }

    /// A product needs empty solid space: water in a stair whose rule would
    /// turn it to ash is blocked, and the stair, the water and the slime all
    /// stay. Blocked, it sleeps. Breaking the stair is an input change: the
    /// water reacts and its Voxel becomes the product, slime untouched.
    #[test]
    fn a_product_never_replaces_a_waterlogged_shape_and_a_blocked_contact_sleeps() {
        let (mut tables, ids) = with(&[SLIME]);
        let slime = ids[0];
        let ash = solid(&mut tables);
        tables
            .register_reaction(rule(WATER, slime, Some(ash)))
            .unwrap();
        let stair = (0..4)
            .map(|turn| Voxel::new(STONE, SHAPE_STAIR, Orientation::new(POS_Y, turn).bits()))
            .find(|&v| !tables.shapes.seals(v, POS_X))
            .unwrap();
        let mut sim = corridor_of(tables, 1.0, 4.0);
        for x in [-4, -3, -2, -1, 2, 3, 4] {
            sim.build([x, 1, 0], stone());
        }
        sim.build([0, 1, 0], stair);
        sim.place_source([0, 1, 0]);
        source(&mut sim, [1, 1, 0], slime);
        sim.settle();
        let wet_stair = sim.occupants([0, 1, 0]);
        assert_eq!(wet_stair.solid, stair);
        assert_eq!(wet_stair.fluid, Some(Fluid::source(WATER)));
        assert_eq!(sim.fluid([1, 1, 0]), Some(Fluid::source(slime)));
        assert!(sim.work.blocked > 0);
        let work = sim.work;
        sim.run(200);
        assert_eq!(sim.work, work, "a blocked contact kept working");
        assert!(sim.agenda.is_empty() && sim.timers.is_empty());

        sim.edit(
            [0, 1, 0],
            Occupants {
                solid: Voxel::AIR,
                ..wet_stair
            },
        );
        sim.settle();
        assert_eq!(sim.occupants([0, 1, 0]).solid, Voxel::cube(ash));
        assert_eq!(sim.fluid([0, 1, 0]), None);
        assert_eq!(sim.fluid([1, 1, 0]), Some(Fluid::source(slime)));
    }

    /// Two reactions of one step that share a participant: in a row of A, B
    /// and C, B goes touching C and A goes touching B, keeping it. Landing
    /// goes in coordinate order and asks again: where B lands first, A's
    /// reaction no longer follows and A stays; where A lands first both do.
    #[test]
    fn simultaneous_contacts_land_in_coordinate_order_and_revalidate() {
        let (mut tables, ids) = with(&[SLIME, LAVA, SLIME]);
        let (a, b, c) = (ids[0], ids[1], ids[2]);
        let (p, q) = (solid(&mut tables), solid(&mut tables));
        tables.register_reaction(rule(b, c, Some(p))).unwrap();
        tables.register_reaction(rule(a, b, Some(q))).unwrap();
        for (order, a_survives) in [([a, b, c], false), ([c, b, a], true)] {
            let mut sim = corridor_of(tables.clone(), 1.0, 4.0);
            for (x, &m) in order.iter().enumerate() {
                source(&mut sim, [x as i32 - 1, 1, 0], m);
            }
            sim.run_step();
            let a_at = [order.iter().position(|&m| m == a).unwrap() as i32 - 1, 1, 0];
            assert_eq!(sim.occupants([0, 1, 0]).solid, Voxel::cube(p), "B went");
            assert_eq!(sim.stale, usize::from(a_survives));
            assert_eq!(material(&sim, a_at) == Some(a), a_survives, "{order:?}");
            sim.settle();
            assert_eq!(material(&sim, a_at) == Some(a), a_survives, "{order:?}");
        }
    }

    /// Unknown terrain beside a Voxel pauses its reactions: no result decided
    /// from what nobody holds. Admission lets the contact go.
    #[test]
    fn a_voxel_beside_unknown_terrain_does_not_react() {
        let (tables, lava) = cooling();
        let mut sim = Sim::with_tables(1.0, [-1, 0, 0], [0, 0, 0], tables);
        source(&mut sim, [-1, 0, 0], WATER);
        source(&mut sim, [0, 0, 0], lava);
        sim.settle();
        assert_eq!(
            material(&sim, [0, 0, 0]),
            Some(lava),
            "it reacted beside unknown"
        );
        sim.grid.min = [-2, -1, -1];
        sim.grid.max = [1, 1, 1];
        for x in -2..=1 {
            for z in -1..=1 {
                sim.build([x, -1, z], stone());
            }
        }
        for (x, z) in [(-2, 0), (1, 0), (-1, -1), (-1, 1), (0, -1), (0, 1)] {
            sim.build([x, 0, z], stone());
        }
        for x in -2..=1 {
            for z in -1..=1 {
                sim.build([x, 1, z], stone());
            }
        }
        sim.wake_all();
        sim.settle();
        assert_eq!(sim.occupants([0, 0, 0]).solid, stone());
        assert_eq!(sim.fluid([-1, 0, 0]), Some(Fluid::source(WATER)));
    }

    /// Reactions and flow divided among workers, in any order, change
    /// nothing: the same state at every step and the same work.
    #[test]
    fn reactions_are_the_same_under_any_division() {
        let script = |slices: usize| {
            let (tables, lava) = cooling();
            let mut sim = corridor_of(tables, 0.5, 6.0);
            source(&mut sim, [-8, 1, 0], lava);
            let mut states = Vec::new();
            for step in 0..400 {
                if step == 40 {
                    sim.place_source([8, 1, 0]);
                }
                if slices == 1 {
                    sim.run_step();
                } else {
                    sim.run_step_sliced(slices);
                }
                states.push(sim.state());
            }
            (states, sim.work)
        };
        let (whole, work) = script(1);
        assert!(work.reacted > 0);
        for slices in [2, 3, 7] {
            let (divided, divided_work) = script(slices);
            assert!(whole == divided, "{slices} slices diverged");
            assert_eq!(work, divided_work);
        }
    }

    /// With no rule naming a fluid, nothing reads past what flow reads.
    #[test]
    fn without_reactions_the_read_set_is_flows_own() {
        assert_eq!(Reach::of(&Tables::builtin()), Reach::default());
        assert!(Reach::of(&cooling().0).contacts);
        let _ = liquid;
    }
}

mod fire;
