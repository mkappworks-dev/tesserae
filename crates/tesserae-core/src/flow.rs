//! Flowing fluid: what a source feeds and what recedes without one.
//!
//! The kernel reads immutable cell state through a [`CellView`] and returns
//! owned [`Proposal`]s; it writes nothing. The Server lands proposals through
//! its one World-change path, and tests drive the same [`advance`] over a
//! [`Region`] of their own.
//!
//! **Rules.** A source holds its own state; only flowing fluid is derived.
//! Flowing fluid at a Voxel is the best of what its neighbours offer: the fluid
//! above falls in, and a horizontal neighbour that cannot fall (or is a source)
//! spreads in with one step less reach. Nothing offers upwards, so flow never
//! climbs or fills a Cave towards a lake's waterline. Falling adds descent, and
//! descent of the definition's reset drop renews reach, keeping the remainder;
//! sideways steps add none, so a lower surface on flat ground never renews it.
//!
//! **Voids.** Fluid stands in a Voxel's [`Void`]s: the whole cell in empty
//! space, the open regions a partial Shape leaves under its Orientation and
//! actual arms, none in a cube. Each void is evaluated on its own and offers
//! pass only where two voids' open squares meet across a face, so sealed and
//! jointly sealed faces and partitions pass nothing. Nothing enters a void
//! sideways from one with a lower floor, and descent counts the sixteenths
//! fallen between floors, so half a Voxel inside a slab is half a Voxel of
//! descent. A Voxel holds one fluid Material, which the best offer into any
//! of its voids decides when it is dry.
//!
//! Every offer goes down a level or loses reach, so flow that no source
//! supports, a ring of it included, always has a highest member that nothing
//! offers its state to. That member recedes when next evaluated, and the
//! change wakes the rest; so recession needs no global pass.
//!
//! **Time.** Logical, in [`Step`]s of [`STEP_S`]. A change wakes the Voxels
//! that read it for the next step; one whose wanted state differs starts a
//! timer of the delay for that edge (its distance, one Voxel, over the
//! definition's rate in metres per second, as [`Pace`] rounds it) and changes
//! when it runs out; a
//! different wanted state restarts it. A step's evaluations all read the state
//! at its start, so dividing one step across any number of quota-bounded calls
//! gives the same proposals.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

use crate::agenda::{Agenda, Span};
use crate::chunk::{CHUNK_SIZE, Chunk};
use crate::fluid::FluidTable;
use crate::mesh::{FACE_STEPS, NEG_X, NEG_Y, NEG_Z, POS_X, POS_Y, POS_Z};
use crate::occupants::{ChunkOverlay, Fluid, Level, Occupants, Overlay};
use crate::shape::{Connections, MAX_VOIDS, SUBDIV, Void};
use crate::tables::Tables;
use crate::voxel::{MaterialId, Voxel};

/// Logical simulation time, in steps since the World opened.
pub type Step = u64;

/// One step of logical time, in seconds: the Server's 64 Hz tick, so a quiet
/// Server simulates in real time and a busy one slows rather than skipping.
pub const STEP_S: f64 = 1.0 / 64.0;

/// A Voxel position.
pub type Cell = [i32; 3];

/// Read access to stored cells. `None` is terrain nobody holds: unknown, not
/// empty, so nothing flows into it and nothing that reads it changes.
pub trait CellView {
    fn stored(&self, at: Cell) -> Option<(Voxel, Overlay)>;
}

/// Shared Chunk handles, read as one lattice: what a worker is given.
#[derive(Clone, Default)]
pub struct Region {
    chunks: HashMap<[i32; 3], (Arc<Chunk>, Arc<ChunkOverlay>)>,
}

impl Region {
    pub fn insert(&mut self, chunk: [i32; 3], voxels: Arc<Chunk>, overlay: Arc<ChunkOverlay>) {
        self.chunks.insert(chunk, (voxels, overlay));
    }

    pub fn contains(&self, chunk: [i32; 3]) -> bool {
        self.chunks.contains_key(&chunk)
    }
}

/// The Chunk a Voxel is in, and where in it.
pub fn chunk_of(at: Cell) -> ([i32; 3], [usize; 3]) {
    let size = CHUNK_SIZE as i32;
    (
        at.map(|i| i.div_euclid(size)),
        at.map(|i| i.rem_euclid(size) as usize),
    )
}

impl CellView for Region {
    fn stored(&self, at: Cell) -> Option<(Voxel, Overlay)> {
        let (chunk, [x, y, z]) = chunk_of(at);
        let (voxels, overlay) = self.chunks.get(&chunk)?;
        Some((voxels.get(x, y, z), overlay.get(x, y, z)))
    }
}

/// A rate at one Voxel size: steps per Voxel, to a 65536th of a step.
///
/// An edge's delay is not this rounded, which would drift by up to half a
/// step every Voxel when the rate does not divide a step (0.75 m/s at 1 m is
/// 85.33 steps). It is the difference of the rounded times the front would
/// reach the edge's two Voxels from the World origin, so any straight path
/// takes its exact time to within one step, at every Voxel size.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Pace(u64);

impl Pace {
    const ONE: u64 = 1 << 16;

    fn new(voxel_m: f64, rate_m_per_s: f32) -> Pace {
        let steps = voxel_m / rate_m_per_s as f64 / STEP_S;
        Pace((steps * Self::ONE as f64).round() as u64)
    }

    /// The steps to cross from Voxel `low` to `low + 1` along one axis, or
    /// back. Never below one: a change is first read the step after it lands.
    pub fn across(self, low: i32) -> Step {
        (self.arrival(low + 1) - self.arrival(low)).max(1) as Step
    }

    /// `round(c * steps)`, floored for negative `c` alike.
    fn arrival(self, c: i32) -> i64 {
        ((c as i128 * self.0 as i128 + (Self::ONE / 2) as i128) >> 16) as i64
    }

    /// The delay of one Voxel rounded on its own, for a change with no edge.
    pub fn steps(self) -> Step {
        ((self.0 + Self::ONE / 2) >> 16).max(1)
    }
}

/// A fluid's definition at one World's Voxel size: whole Voxel steps of reach
/// and the [`Pace`] of each rate. An equal physical fixture at any Voxel size
/// agrees within one Voxel of reach, and a straight path within one step.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Scaled {
    pub reach: u16,
    /// [`crate::FluidDef::reset_drop_m`] in sixteenths of a Voxel.
    pub reset: u16,
    pub sideways: Pace,
    pub downward: Pace,
    pub recession: Pace,
    /// [`FluidTable::rank`], which settles an otherwise equal claim.
    pub rank: u16,
}

/// Every fluid's [`Scaled`] definition, by Material.
#[derive(Clone, Debug, Default)]
pub struct FlowScale {
    by_material: Vec<Option<Scaled>>,
}

impl FlowScale {
    pub fn new(fluids: &FluidTable, voxel_size_m: f32) -> Self {
        let v = voxel_size_m as f64;
        let mut by_material = Vec::new();
        for (material, def) in fluids.iter() {
            let i = material as usize;
            if by_material.len() <= i {
                by_material.resize(i + 1, None);
            }
            // The epsilon keeps 7 m at 0.25 m from flooring to 27 steps.
            let reach = (def.reach_m as f64 / v + 1e-6).floor();
            let reset = (def.reset_drop_m as f64 / v * SUBDIV as f64).round();
            by_material[i] = Some(Scaled {
                reach: reach.clamp(0.0, u16::MAX as f64) as u16,
                reset: reset.clamp(1.0, u16::MAX as f64) as u16,
                sideways: Pace::new(v, def.sideways_m_per_s),
                downward: Pace::new(v, def.downward_m_per_s),
                recession: Pace::new(v, def.recession_m_per_s),
                rank: fluids.rank(material).expect("a registered fluid"),
            });
        }
        FlowScale { by_material }
    }

    pub fn get(&self, material: MaterialId) -> Option<&Scaled> {
        self.by_material.get(material as usize)?.as_ref()
    }
}

/// What a Voxel reads to decide its state, as offsets: itself, above, the four
/// sides, and below each side, which says whether that side spreads.
pub const READS: [Cell; 10] = [
    [0, 0, 0],
    [0, 1, 0],
    [-1, 0, 0],
    [1, 0, 0],
    [0, 0, -1],
    [0, 0, 1],
    [-1, -1, 0],
    [1, -1, 0],
    [0, -1, -1],
    [0, -1, 1],
];

/// [`READS`] reversed: the Voxels that read a given one.
pub const DEPENDENTS: [Cell; 10] = [
    [0, 0, 0],
    [0, -1, 0],
    [1, 0, 0],
    [-1, 0, 0],
    [0, 0, 1],
    [0, 0, -1],
    [1, 1, 0],
    [-1, 1, 0],
    [0, 1, 1],
    [0, 1, -1],
];

/// [`READS`] and the face neighbours of each: a connected Shape's arms, and
/// so its voids, depend on the solids across its faces.
static WIDE_READS: LazyLock<Vec<Cell>> = LazyLock::new(|| {
    let mut cells: Vec<Cell> = READS
        .iter()
        .flat_map(|&r| std::iter::once(r).chain(FACE_STEPS.iter().map(move |&d| step_by(r, d))))
        .collect();
    cells.sort_unstable();
    cells.dedup();
    cells
});

static WIDE_DEPENDENTS: LazyLock<Vec<Cell>> =
    LazyLock::new(|| WIDE_READS.iter().map(|c| c.map(|i| -i)).collect());

/// Every position an evaluation may read: [`READS`], and with connected Shapes
/// registered also the solids that decide their arms. Fluid state is read at
/// [`READS`] only.
pub fn reads(connected: bool) -> &'static [Cell] {
    if connected { &WIDE_READS } else { &READS }
}

/// [`reads`] reversed: the Voxels a change of solid at a position can affect.
pub fn dependents(connected: bool) -> &'static [Cell] {
    if connected {
        &WIDE_DEPENDENTS
    } else {
        &DEPENDENTS
    }
}

/// Wakes, at `step`, every Voxel whose wanted state reads `at`'s fluid.
pub fn wake_around(agenda: &mut Agenda, at: Cell, step: Step) {
    for d in DEPENDENTS {
        agenda.wake(step_by(at, d), step);
    }
}

/// Wakes, at `step`, every Voxel a change of solid at `at` can affect: what
/// reads its voids, and what reads the voids of each connected Shape beside
/// it, whose arms it decides. `connected_at` says where those stand.
pub fn wake_solid_around(
    agenda: &mut Agenda,
    at: Cell,
    step: Step,
    connected_at: impl Fn(Cell) -> bool,
) {
    wake_around(agenda, at, step);
    for d in FACE_STEPS {
        let beside = step_by(at, d);
        if connected_at(beside) {
            wake_around(agenda, beside, step);
        }
    }
}

/// The Voxels outside `chunk` that read one inside it, as spans of the Chunks
/// they are in: what must look again when that Chunk arrives or leaves.
pub fn readers_outside(chunk: [i32; 3], connected: bool) -> Vec<Span> {
    let home = Span::chunk(chunk);
    let mut spans: Vec<Span> = Vec::new();
    for &d in dependents(connected) {
        if d == [0, 0, 0] {
            continue;
        }
        let shifted = Span::split(step_by(home.min, d), step_by(home.max, d));
        for s in shifted {
            if s.home() == chunk || spans.iter().any(|t| t.contains(&s)) {
                continue;
            }
            spans.retain(|t| !s.contains(t));
            spans.push(s);
        }
    }
    spans.sort_unstable_by_key(|s| (s.home(), s.min, s.max));
    spans
}

/// The faces of `chunk` whose Voxels read outside it, as deep as [`reads`]
/// reaches.
pub fn reading_outside(chunk: [i32; 3], connected: bool) -> Vec<Span> {
    let home = Span::chunk(chunk);
    let deep = |axis: usize, sign: i32| {
        reads(connected)
            .iter()
            .map(|r| (r[axis] * sign).max(0))
            .max()
            .unwrap_or(0)
    };
    let mut spans = Vec::new();
    for axis in 0..3 {
        let mut low = home;
        low.max[axis] = home.min[axis] + deep(axis, -1) - 1;
        let mut high = home;
        high.min[axis] = home.max[axis] - deep(axis, 1) + 1;
        spans.extend([low, high]);
    }
    spans
}

const SIDES: [Cell; 4] = [[-1, 0, 0], [1, 0, 0], [0, 0, -1], [0, 0, 1]];

/// A change a Voxel wants and the step it may make it, so a wanted change waits
/// out its delay even when something else wakes the Voxel sooner.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Timer {
    ready: Step,
    want: Option<Fluid>,
}

/// Every running timer. Owned by whoever holds the simulation between steps.
#[derive(Clone, Debug, Default)]
pub struct Timers(HashMap<Cell, Timer>);

impl Timers {
    /// Takes the timers of `cells`, for a worker evaluating just those.
    pub fn split_off(&mut self, cells: &[Cell]) -> Timers {
        Timers(
            cells
                .iter()
                .filter_map(|c| self.0.remove_entry(c))
                .collect(),
        )
    }

    pub fn merge(&mut self, other: Timers) {
        self.0.extend(other.0);
    }

    /// Makes `at` change to `want` when evaluated at `ready` or later, unless
    /// what it reads by then wants something else.
    pub fn hold(&mut self, at: Cell, ready: Step, want: Option<Fluid>) {
        self.0.insert(at, Timer { ready, want });
    }

    pub fn remove(&mut self, at: Cell) {
        self.0.remove(&at);
    }

    pub fn retain(&mut self, mut keep: impl FnMut(Cell) -> bool) {
        self.0.retain(|&c, _| keep(c));
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// One Voxel's next state, and the state it was computed from: landing applies
/// it only where the Voxel still reads `read`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Proposal {
    pub at: Cell,
    pub read: Occupants,
    pub next: Occupants,
}

/// Operations spent, counted whether or not they changed anything.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct Work {
    pub evaluated: usize,
    pub proposed: usize,
    /// Evaluations that read unknown terrain and so decided nothing.
    pub paused: usize,
    /// Wanted changes that started or kept waiting on a timer.
    pub waiting: usize,
}

impl std::ops::AddAssign for Work {
    fn add_assign(&mut self, other: Work) {
        self.evaluated += other.evaluated;
        self.proposed += other.proposed;
        self.paused += other.paused;
        self.waiting += other.waiting;
    }
}

/// How many evaluations one call may make.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Quota {
    pub evaluations: usize,
}

/// One step's work in progress, handed to [`advance`] and back.
#[derive(Clone, Debug)]
pub struct Continuation {
    pub step: Step,
    due: Vec<Cell>,
    next: usize,
    timers: Timers,
    proposals: Vec<Proposal>,
    wakeups: Vec<(Step, Cell)>,
    work: Work,
}

impl Continuation {
    /// `due` from [`Agenda::take_due`].
    pub fn start(step: Step, due: Vec<Cell>, timers: Timers) -> Self {
        Continuation {
            step,
            due,
            next: 0,
            timers,
            proposals: Vec::new(),
            wakeups: Vec::new(),
            work: Work::default(),
        }
    }

    /// The Voxels the next call of up to `quota` evaluations reads, for a caller
    /// deciding which Chunks to hand it.
    pub fn upcoming(&self, quota: Quota, connected: bool) -> impl Iterator<Item = Cell> + '_ {
        let end = (self.next + quota.evaluations).min(self.due.len());
        self.due[self.next..end]
            .iter()
            .flat_map(move |&c| reads(connected).iter().map(move |&d| step_by(c, d)))
    }

    pub fn remaining(&self) -> usize {
        self.due.len() - self.next
    }
}

/// A finished step: the proposals to land together, the timers back, and the
/// steps at which timers want their Voxels evaluated again.
#[derive(Clone, Debug)]
pub struct Finished {
    pub step: Step,
    pub proposals: Vec<Proposal>,
    pub wakeups: Vec<(Step, Cell)>,
    pub timers: Timers,
    pub work: Work,
}

#[derive(Clone, Debug)]
pub enum Advance {
    /// The quota ran out; call again with this.
    Partial(Continuation),
    Complete(Finished),
}

/// Evaluates up to `quota` of the step's due Voxels. Proposals are held until
/// every due Voxel has been evaluated, so no part of a step lands before the
/// rest has read the state it started from.
pub fn advance(
    view: &impl CellView,
    tables: &Tables,
    scale: &FlowScale,
    mut cont: Continuation,
    quota: Quota,
) -> Advance {
    let end = (cont.next + quota.evaluations).min(cont.due.len());
    while cont.next < end {
        let at = cont.due[cont.next];
        cont.next += 1;
        evaluate(view, tables, scale, at, &mut cont);
    }
    if cont.next < cont.due.len() {
        return Advance::Partial(cont);
    }
    Advance::Complete(Finished {
        step: cont.step,
        proposals: cont.proposals,
        wakeups: cont.wakeups,
        timers: cont.timers,
        work: cont.work,
    })
}

fn evaluate(
    view: &impl CellView,
    tables: &Tables,
    scale: &FlowScale,
    at: Cell,
    cont: &mut Continuation,
) {
    cont.work.evaluated += 1;
    let (here, want, delay) = match wanted(view, tables, scale, at) {
        // Whatever it was waiting for was decided from terrain that is gone;
        // admission rescans it.
        Wanted::Unknown => {
            cont.timers.0.remove(&at);
            cont.work.paused += 1;
            return;
        }
        Wanted::Fixed => {
            cont.timers.0.remove(&at);
            return;
        }
        Wanted::State { here, want, delay } => (here, want, delay),
    };
    if want == here.fluid {
        cont.timers.0.remove(&at);
        return;
    }
    let step = cont.step;
    let ready = match cont.timers.0.get(&at) {
        Some(timer) if timer.want == want => timer.ready,
        _ => step + delay - 1,
    };
    if ready > step {
        cont.timers.0.insert(at, Timer { ready, want });
        cont.wakeups.push((ready, at));
        cont.work.waiting += 1;
        return;
    }
    cont.timers.0.remove(&at);
    cont.proposals.push(Proposal {
        at,
        read: here,
        next: Occupants {
            fluid: want,
            ..here
        },
    });
    cont.work.proposed += 1;
}

enum Wanted {
    /// Something it reads is not held.
    Unknown,
    /// Not derived: a source, or a solid that leaves no void for fluid.
    Fixed,
    State {
        here: Occupants,
        want: Option<Fluid>,
        delay: Step,
    },
}

/// An offer of fluid into a void, from above or from a side.
#[derive(Clone, Copy)]
struct Offer {
    material: MaterialId,
    rank: u16,
    reach: u16,
    descent: u16,
    from_above: bool,
    /// The lower coordinate of the edge it crosses, on that edge's axis.
    low: i32,
}

impl Offer {
    /// Larger reach wins, then nearer renewal, then falling, then the earlier
    /// registered fluid.
    fn key(&self) -> (u16, u16, bool, std::cmp::Reverse<u16>) {
        (
            self.reach,
            self.descent,
            self.from_above,
            std::cmp::Reverse(self.rank),
        )
    }
}

fn rank(level: Level) -> Option<(u16, u16, u8)> {
    level
        .is_wet()
        .then_some((level.reach, level.descent, level.surface))
}

fn step_by(at: Cell, d: Cell) -> Cell {
    [at[0] + d[0], at[1] + d[1], at[2] + d[2]]
}

/// `drop` sixteenths more descent, renewing reach each time it reaches the
/// reset drop and keeping the remainder.
fn descend(reach: u16, descent: u16, drop: u32, s: &Scaled) -> (u16, u16) {
    let fallen = descent as u32 + drop;
    if fallen >= s.reset as u32 {
        (s.reach, (fallen % s.reset as u32) as u16)
    } else {
        (reach, fallen as u16)
    }
}

/// One read position: its occupants and the voids its solid leaves under its
/// actual arms.
#[derive(Clone, Copy)]
struct Seen<'a> {
    occupants: Occupants,
    voids: &'a [Void],
}

impl Seen<'_> {
    /// Whether fluid of `material` falling out of `void` (open at its bottom)
    /// has somewhere to go here.
    fn takes_from_above(&self, void: &Void, material: MaterialId) -> bool {
        self.voids
            .iter()
            .any(|b| b.open[POS_Y].intersects(&void.open[NEG_Y]))
            && self
                .occupants
                .fluid
                .is_none_or(|f| !f.source && f.material == material)
    }
}

/// The faces of [`SIDES`], in order.
const SIDE_FACES: [usize; 4] = [NEG_X, POS_X, NEG_Z, POS_Z];

fn wanted(view: &impl CellView, tables: &Tables, scale: &FlowScale, at: Cell) -> Wanted {
    let shapes = &tables.shapes;
    let connected = shapes.any_connected();
    let look = |c: Cell| -> Option<Seen<'_>> {
        let (voxel, overlay) = view.stored(c)?;
        let occupants = Occupants::of(voxel, overlay, &tables.fluids);
        let solid = occupants.solid;
        let arms = if connected && shapes.is_connected(solid) {
            let mut around = [Voxel::AIR; 6];
            for (face, across) in around.iter_mut().enumerate() {
                *across = view.stored(step_by(c, FACE_STEPS[face]))?.0;
            }
            shapes.connections(solid, around)
        } else {
            Connections::NONE
        };
        Some(Seen {
            occupants,
            voids: shapes.voids(solid, arms),
        })
    };
    let Some(here) = look(at) else {
        return Wanted::Unknown;
    };
    if here.occupants.fluid.is_some_and(|f| f.source) || here.voids.is_empty() {
        return Wanted::Fixed;
    }
    let Some(above) = look(step_by(at, [0, 1, 0])) else {
        return Wanted::Unknown;
    };
    // A flowing side's below says whether it falls instead of spreading. Read
    // here, but unknown only matters where an offer would ask it.
    let mut sides: [Option<(Seen, Option<Seen>)>; 4] = [None; 4];
    for (k, d) in SIDES.into_iter().enumerate() {
        let side_at = step_by(at, d);
        let Some(side) = look(side_at) else {
            return Wanted::Unknown;
        };
        let below = match side.occupants.fluid {
            Some(f) if !f.source => look(step_by(side_at, [0, -1, 0])),
            _ => None,
        };
        sides[k] = Some((side, below));
    }
    let unknown = std::cell::Cell::new(false);

    // The best offer into void `r`, of `only` if given, and whether fluid of
    // the winning Material falls into it.
    let best_for = |r: &Void, only: Option<MaterialId>| -> (Option<Offer>, bool) {
        let admits =
            |m: MaterialId| only.is_none_or(|o| o == m) && tables.holds(here.occupants.solid, m);
        let mut best: Option<Offer> = None;
        let mut fed_from_above: Option<MaterialId> = None;
        let mut offer = |o: Offer| {
            if best.is_none_or(|b| o.key() > b.key()) {
                best = Some(o);
            }
        };
        if let Some(f) = above.occupants.fluid
            && admits(f.material)
            && let Some(s) = scale.get(f.material)
        {
            for (a, void) in above.voids.iter().enumerate() {
                let level = f.level(a);
                if !level.is_wet() || !void.open[NEG_Y].intersects(&r.open[POS_Y]) {
                    continue;
                }
                let (reach, descent) = if f.source {
                    (s.reach, 0)
                } else {
                    (level.reach, level.descent)
                };
                let drop = SUBDIV as u32 + void.floor as u32 - r.floor as u32;
                let (reach, descent) = descend(reach, descent, drop, s);
                fed_from_above = Some(f.material);
                offer(Offer {
                    material: f.material,
                    rank: s.rank,
                    reach,
                    descent,
                    from_above: true,
                    low: at[1],
                });
            }
        }
        for (k, face) in SIDE_FACES.into_iter().enumerate() {
            let (side, below) = sides[k].expect("read above");
            let Some(f) = side.occupants.fluid else {
                continue;
            };
            if !admits(f.material) {
                continue;
            }
            let Some(s) = scale.get(f.material) else {
                continue;
            };
            for (j, void) in side.voids.iter().enumerate() {
                let level = f.level(j);
                // Flow never climbs, inside a Shape or out of one.
                if !level.is_wet()
                    || void.floor < r.floor
                    || !void.open[face ^ 1].intersects(&r.open[face])
                {
                    continue;
                }
                if !f.source {
                    match below {
                        None => {
                            unknown.set(true);
                            continue;
                        }
                        Some(b) if b.takes_from_above(void, f.material) => continue,
                        Some(_) => {}
                    }
                }
                let (reach, descent) = if f.source {
                    (s.reach, 0)
                } else {
                    (level.reach, level.descent)
                };
                if reach == 0 {
                    continue;
                }
                let drop = (void.floor - r.floor) as u32;
                let (reach, descent) = match drop {
                    0 => (reach - 1, descent),
                    _ => descend(reach - 1, descent, drop, s),
                };
                let axis = if SIDES[k][0] != 0 { 0 } else { 2 };
                offer(Offer {
                    material: f.material,
                    rank: s.rank,
                    reach,
                    descent,
                    from_above: false,
                    low: at[axis].min(at[axis] + SIDES[k][axis]),
                });
            }
        }
        let fed = best.is_some_and(|b| fed_from_above == Some(b.material));
        (best, fed)
    };

    // An existing fluid keeps out every other one until it has receded; into a
    // dry Voxel the best offer anywhere decides its one Material.
    let only = here.occupants.fluid_material();
    let mut bests = [(None, false); MAX_VOIDS];
    for (r, void) in here.voids.iter().enumerate() {
        bests[r] = best_for(void, only);
    }
    let winner = bests
        .iter()
        .filter_map(|(o, _)| *o)
        .max_by_key(|o| o.key())
        .map(|o| o.material);
    if only.is_none() && winner.is_some() {
        for (r, void) in here.voids.iter().enumerate() {
            if bests[r].0.is_some_and(|o| Some(o.material) != winner) {
                bests[r] = best_for(void, winner);
            }
        }
    }

    if unknown.get() {
        return Wanted::Unknown;
    }
    let mut levels = [Level::DRY; MAX_VOIDS];
    let mut delay: Option<Step> = None;
    for (r, void) in here.voids.iter().enumerate() {
        let (best, fed) = bests[r];
        let want = best.map_or(Level::DRY, |o| {
            let s = scale
                .get(o.material)
                .expect("offered by a fluid with a scale");
            let surface = if fed {
                Fluid::FULL_SURFACE
            } else {
                1 + (15 * o.reach as u32 / s.reach.max(1) as u32) as u8
            };
            Level {
                surface: surface.max(void.floor + 1),
                reach: o.reach,
                descent: o.descent,
            }
        });
        levels[r] = want;
        let had = here.occupants.fluid.map_or(Level::DRY, |f| f.level(r));
        if want == had {
            continue;
        }
        let edge = match (best, rank(want) > rank(had)) {
            (Some(o), true) => {
                let s = scale.get(o.material).expect("scaled");
                if o.from_above { s.downward } else { s.sideways }.across(o.low)
            }
            (worse, _) => here
                .occupants
                .fluid
                .and_then(|f| scale.get(f.material))
                .map_or(1, |s| {
                    worse.map_or(s.recession.steps(), |o| s.recession.across(o.low))
                }),
        };
        delay = Some(delay.map_or(edge, |d| d.min(edge)));
    }
    let want = winner.or(only).and_then(|m| Fluid::flowing(m, levels));
    Wanted::State {
        here: here.occupants,
        want,
        delay: delay.unwrap_or(1),
    }
}

#[cfg(test)]
mod tests;
