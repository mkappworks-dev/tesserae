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
//! Every offer goes down a level or loses reach, so flow that no source
//! supports, a ring of it included, always has a highest member that nothing
//! offers its state to. That member recedes when next evaluated, and the
//! change wakes the rest; so recession needs no global pass.
//!
//! **Time.** Logical, in [`Step`]s of [`STEP_S`]. A change wakes the Voxels
//! that read it for the next step; one whose wanted state differs starts a
//! timer of the delay for that edge (its distance, one Voxel, over the
//! definition's rate in metres per second) and changes when it runs out; a
//! different wanted state restarts it. A step's evaluations all read the state
//! at its start, so dividing one step across any number of quota-bounded calls
//! gives the same proposals.

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use crate::chunk::{CHUNK_SIZE, Chunk};
use crate::fluid::FluidTable;
use crate::occupants::{ChunkOverlay, Fluid, Occupants, Overlay};
use crate::shape::SUBDIV;
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

/// A fluid's definition at one World's Voxel size: whole Voxel steps of reach
/// and whole steps of time. Rounding is to the nearest step and never below
/// one, so an equal physical fixture at any Voxel size agrees within one Voxel
/// of reach and one step of time per Voxel travelled.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Scaled {
    pub reach: u16,
    /// [`crate::FluidDef::reset_drop_m`] in sixteenths of a Voxel.
    pub reset: u16,
    pub sideways: Step,
    pub downward: Step,
    pub recession: Step,
}

/// Every fluid's [`Scaled`] definition, by Material.
#[derive(Clone, Debug, Default)]
pub struct FlowScale {
    by_material: Vec<Option<Scaled>>,
}

impl FlowScale {
    pub fn new(fluids: &FluidTable, voxel_size_m: f32) -> Self {
        let v = voxel_size_m as f64;
        let steps = |rate: f32| ((v / rate as f64) / STEP_S).round().max(1.0) as Step;
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
                sideways: steps(def.sideways_m_per_s),
                downward: steps(def.downward_m_per_s),
                recession: steps(def.recession_m_per_s),
            });
        }
        FlowScale { by_material }
    }

    pub fn get(&self, material: MaterialId) -> Option<&Scaled> {
        self.by_material.get(material as usize)?.as_ref()
    }
}

/// Voxels waiting to be evaluated, each at the earliest step anything asked.
#[derive(Clone, Debug, Default)]
pub struct Wakeups {
    queue: BTreeSet<(Step, Cell)>,
    at: HashMap<Cell, Step>,
}

impl Wakeups {
    pub fn wake(&mut self, cell: Cell, step: Step) {
        match self.at.get(&cell) {
            Some(&due) if due <= step => {}
            Some(&due) => {
                self.queue.remove(&(due, cell));
                self.queue.insert((step, cell));
                self.at.insert(cell, step);
            }
            None => {
                self.queue.insert((step, cell));
                self.at.insert(cell, step);
            }
        }
    }

    /// Wakes, at `step`, every Voxel whose wanted state reads `at`.
    pub fn wake_around(&mut self, at: Cell, step: Step) {
        for d in DEPENDENTS {
            self.wake([at[0] + d[0], at[1] + d[1], at[2] + d[2]], step);
        }
    }

    /// Everything due by `step`, in canonical coordinate order.
    pub fn take_due(&mut self, step: Step) -> Vec<Cell> {
        let mut due = Vec::new();
        while let Some(&(when, cell)) = self.queue.first() {
            if when > step {
                break;
            }
            self.queue.pop_first();
            self.at.remove(&cell);
            due.push(cell);
        }
        due.sort_unstable();
        due
    }

    pub fn len(&self) -> usize {
        self.at.len()
    }

    pub fn is_empty(&self) -> bool {
        self.at.is_empty()
    }

    pub fn next_due(&self) -> Option<Step> {
        self.queue.first().map(|&(step, _)| step)
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
const DEPENDENTS: [Cell; 10] = [
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
    /// `due` from [`Wakeups::take_due`].
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
    pub fn upcoming(&self, quota: Quota) -> impl Iterator<Item = Cell> + '_ {
        let end = (self.next + quota.evaluations).min(self.due.len());
        self.due[self.next..end].iter().flat_map(|&c| {
            READS
                .iter()
                .map(move |d| [c[0] + d[0], c[1] + d[1], c[2] + d[2]])
        })
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
    let read = |c: Cell| {
        view.stored(c)
            .map(|(voxel, overlay)| Occupants::of(voxel, overlay, &tables.fluids))
    };
    let (here, want, delay) = match wanted(&read, scale, at) {
        Wanted::Unknown => {
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
    /// Not derived: a source, or a solid whose geometry decides what fluid it
    /// may hold.
    Fixed,
    State {
        here: Occupants,
        want: Option<Fluid>,
        delay: Step,
    },
}

/// An offer of fluid into a Voxel, from above or from a side.
#[derive(Clone, Copy)]
struct Offer {
    material: MaterialId,
    reach: u16,
    descent: u16,
    from_above: bool,
}

impl Offer {
    /// Larger reach wins, then nearer renewal, then falling, then the earlier
    /// registered fluid.
    fn key(&self) -> (u16, u16, bool, std::cmp::Reverse<MaterialId>) {
        (
            self.reach,
            self.descent,
            self.from_above,
            std::cmp::Reverse(self.material),
        )
    }
}

fn rank(fluid: Option<Fluid>) -> Option<(u16, u16, u8)> {
    fluid.map(|f| (f.reach, f.descent, f.surface))
}

fn step_by(at: Cell, d: Cell) -> Cell {
    [at[0] + d[0], at[1] + d[1], at[2] + d[2]]
}

/// Whether fluid of `material` above may fall into this Voxel.
fn takes_from_above(below: Occupants, material: MaterialId) -> bool {
    below.solid.is_air()
        && below
            .fluid
            .is_none_or(|f| !f.source && f.material == material)
}

fn wanted(read: &impl Fn(Cell) -> Option<Occupants>, scale: &FlowScale, at: Cell) -> Wanted {
    let Some(here) = read(at) else {
        return Wanted::Unknown;
    };
    if !here.solid.is_air() || here.fluid.is_some_and(|f| f.source) {
        return Wanted::Fixed;
    }
    // An existing fluid keeps out every other one until it has receded.
    let only = here.fluid.map(|f| f.material);
    let admits = |m: MaterialId| only.is_none_or(|o| o == m);

    let mut best: Option<Offer> = None;
    let mut fed_from_above: Option<MaterialId> = None;
    let mut offer = |o: Offer| {
        if best.is_none_or(|b| o.key() > b.key()) {
            best = Some(o);
        }
    };

    let Some(above) = read(step_by(at, [0, 1, 0])) else {
        return Wanted::Unknown;
    };
    if above.solid.is_air()
        && let Some(f) = above.fluid
        && admits(f.material)
        && let Some(s) = scale.get(f.material)
    {
        let (reach, descent) = if f.source {
            (s.reach, 0)
        } else {
            (f.reach, f.descent)
        };
        let fallen = descent as u32 + SUBDIV as u32;
        let (reach, descent) = if fallen >= s.reset as u32 {
            (s.reach, (fallen % s.reset as u32) as u16)
        } else {
            (reach, fallen as u16)
        };
        fed_from_above = Some(f.material);
        offer(Offer {
            material: f.material,
            reach,
            descent,
            from_above: true,
        });
    }

    for d in SIDES {
        let side_at = step_by(at, d);
        let Some(side) = read(side_at) else {
            return Wanted::Unknown;
        };
        let Some(f) = side.fluid else {
            continue;
        };
        if !side.solid.is_air() || !admits(f.material) {
            continue;
        }
        let Some(s) = scale.get(f.material) else {
            continue;
        };
        if !f.source {
            let Some(below) = read(step_by(side_at, [0, -1, 0])) else {
                return Wanted::Unknown;
            };
            if takes_from_above(below, f.material) {
                continue;
            }
        }
        let (reach, descent) = if f.source {
            (s.reach, 0)
        } else {
            (f.reach, f.descent)
        };
        if reach == 0 {
            continue;
        }
        offer(Offer {
            material: f.material,
            reach: reach - 1,
            descent,
            from_above: false,
        });
    }

    let want = best.map(|o| {
        let s = scale
            .get(o.material)
            .expect("offered by a fluid with a scale");
        let surface = if fed_from_above == Some(o.material) {
            Fluid::FULL_SURFACE
        } else {
            1 + (15 * o.reach as u32 / s.reach.max(1) as u32) as u8
        };
        Fluid {
            material: o.material,
            source: false,
            surface,
            reach: o.reach,
            descent: o.descent,
        }
    });
    let delay = match (best, rank(want) > rank(here.fluid)) {
        (Some(o), true) => {
            let s = scale.get(o.material).expect("scaled");
            if o.from_above { s.downward } else { s.sideways }
        }
        _ => here
            .fluid
            .and_then(|f| scale.get(f.material))
            .map_or(1, |s| s.recession),
    };
    Wanted::State { here, want, delay }
}

#[cfg(test)]
mod tests;
