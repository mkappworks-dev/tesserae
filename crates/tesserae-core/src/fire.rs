//! Heat exposure, ignition and burning of declared fuel, evaluated beside flow
//! by [`crate::flow::advance`] for the same due Voxels and the same quota.
//!
//! **Exposure.** A hot fluid and a burning fuel that declare [`Heat`] expose
//! the open space around them. Open space is the [`Void`]s flow passes
//! through, fluid-filled or not, and heat crosses between two of them exactly
//! where their open squares meet across a face, so a solid wall, a Shape's
//! partition and a sealed or jointly sealed face pass nothing. Terrain nobody
//! holds is a wall too. There is no temperature: nothing is stored in the
//! space between, and solids conduct nothing.
//!
//! [`Void`]: crate::shape::Void
//!
//! **Path length** is counted in face steps between voids, each one Voxel
//! edge, so in metres it is steps times the World's Voxel size. It starts in
//! the space a source touches (a hot fluid's own voids; a burning solid's
//! voids and the voids across its faces that meet its surface) and ends in
//! the space the fuel touches, alike. A gap of open space `n` Voxels wide is
//! so `n` steps from a hot fluid and `n - 1` from a burning solid: distance is
//! resolved to one Voxel.
//!
//! **Attenuation.** `k` steps from a source of `strength` over `reach_m`,
//! exposure is `floor(strength * (reach_m - k * v) / reach_m)` for Voxel size
//! `v`, and 0 once `k * v` reaches `reach_m`: tabulated per source when the
//! World opens, never increasing with `k`. The strongest source decides;
//! overlapping sources never add up.
//!
//! **Ignition.** Unburning fuel whose exposure is at least its threshold
//! starts a timer of its ignition delay, rounded to whole steps and at least
//! one, and catches when the timer runs out with its whole fuel. An
//! evaluation that finds it below the threshold, or wet, drops the timer, so
//! progress starts again from nothing.
//!
//! **Burning** runs a timer of the fuel it caught with; when it runs out the
//! solid's Material becomes the declared residue, keeping its Shape and
//! Orientation, or the solid goes if the residue is air. A permanent fire
//! never runs out. Burning emits its declared block light and heat.
//!
//! **Wet.** An extinguishing fluid touching the fuel protects the whole of it
//! and puts out its fire at once, with no residue: a wet part of a Shape
//! protects its dry part. It touches from any of the Voxel's own voids, or
//! from a void across one of its faces that meets the solid there, so water
//! against a cube's side, on its top or under it wets it, while water behind
//! a neighbour's sealed face or partition does not. Fire lives in the open
//! space beside fuel, and a full cube has no void of its own.
//!
//! **Paused.** A fuel beside terrain nobody holds decides nothing, as flow
//! beside it does, but its ignition progress or fuel left is frozen rather
//! than dropped and goes on, to within a step, once that terrain is held, so
//! a fire at the edge of the held World neither starts its burn again nor
//! stalls its spread. Replacing the solid meanwhile drops it.
//!
//! **Reads.** An exposure search reads as far as [`HeatScale::radius`], wider
//! than flow's [`crate::flow::reads`]: a change of solid, fire or hot fluid
//! there must wake the fuel ([`wake_fuel_around`]), and an ignition decided
//! before it is stale ([`crate::flow::Proposal::heated`]).

use std::collections::{HashMap, HashSet};

use crate::agenda::{Agenda, Span};
use crate::combustion::{Burn, Heat};
use crate::flow::{Cell, CellView, Continuation, STEP_S, Seen, Step, Work, look, step_by};
use crate::mesh::FACE_STEPS;
use crate::occupants::{Burning, Fluid, Occupants};
use crate::shape::FaceMask;
use crate::tables::Tables;
use crate::voxel::{MaterialId, Voxel};

/// Heat and fire at one World's Voxel size, fixed when the World opens.
#[derive(Clone, Debug, Default)]
pub struct HeatScale {
    /// Exposure `k` steps from a source, by the Material giving it off: a hot
    /// fluid, or a fuel while it burns.
    falloff: Vec<Option<Box<[u8]>>>,
    /// The strongest exposure any source gives `k` steps away.
    ceiling: Box<[u8]>,
    /// Ignition delay in steps, by fuel.
    delay: Vec<Option<Step>>,
    /// The deepest search worth making: past it no source reaches the lowest
    /// threshold. `None` without fuel or heat.
    steps: Option<usize>,
    fuel: bool,
}

/// [`Heat`]'s attenuation over `k` steps of `voxel_m`, while it is above 0.
fn falloff(heat: Heat, voxel_m: f64) -> Box<[u8]> {
    let reach = heat.reach_m as f64;
    (0..)
        .map(|k| {
            let d = k as f64 * voxel_m;
            if d >= reach {
                0
            } else {
                (heat.strength as f64 * (reach - d) / reach).floor() as u8
            }
        })
        .take_while(|&e| e > 0)
        .collect()
}

impl HeatScale {
    pub fn new(tables: &Tables, voxel_size_m: f32) -> Self {
        let v = voxel_size_m as f64;
        let mut scale = HeatScale::default();
        let mut put = |material: MaterialId, heat: Heat| {
            let i = material as usize;
            if scale.falloff.len() <= i {
                scale.falloff.resize(i + 1, None);
            }
            scale.falloff[i] = Some(falloff(heat, v));
        };
        for (material, def) in tables.fluids.iter() {
            if let Some(heat) = def.heat {
                put(material, heat);
            }
        }
        for (fuel, def) in tables.combustion.iter() {
            if let Some(heat) = def.heat {
                put(fuel, heat);
            }
        }
        let mut lowest = None::<u8>;
        for (fuel, def) in tables.combustion.iter() {
            let i = fuel as usize;
            if scale.delay.len() <= i {
                scale.delay.resize(i + 1, None);
            }
            let steps = (def.ignition_delay_s as f64 / STEP_S).round() as Step;
            scale.delay[i] = Some(steps.max(1));
            lowest = Some(lowest.map_or(def.threshold, |t| t.min(def.threshold)));
            scale.fuel = true;
        }
        let longest = scale.falloff.iter().flatten().map(|f| f.len()).max();
        scale.ceiling = (0..longest.unwrap_or(0))
            .map(|k| {
                scale
                    .falloff
                    .iter()
                    .flatten()
                    .map(|f| f.get(k).copied().unwrap_or(0))
                    .max()
                    .unwrap_or(0)
            })
            .collect();
        scale.steps = lowest.and_then(|t| scale.ceiling.iter().rposition(|&e| e >= t));
        scale
    }

    /// Whether any Material is declared fuel: without one nothing thermal
    /// happens at all.
    pub fn has_fuel(&self) -> bool {
        self.fuel
    }

    /// How far, in Voxels along any axis, a fuel's evaluation reads: the
    /// deepest search, one more for the fuel's own surface and the burning
    /// solids beside a void, and one more for a connected Shape's arms; with
    /// no search at all, still its faces and their arms, which say whether it
    /// is wet. A change within this of a fuel can change its exposure.
    pub fn radius(&self) -> i32 {
        if !self.fuel {
            return 0;
        }
        self.steps.map_or(2, |k| k as i32 + 3)
    }

    /// Exposure `k` steps from `source` while it gives off heat.
    pub fn exposure(&self, source: MaterialId, k: usize) -> u8 {
        self.falloff
            .get(source as usize)
            .and_then(Option::as_ref)
            .and_then(|f| f.get(k).copied())
            .unwrap_or(0)
    }

    fn gives_heat(&self, material: MaterialId) -> bool {
        self.falloff
            .get(material as usize)
            .is_some_and(Option::is_some)
    }

    fn delay(&self, fuel: MaterialId) -> Step {
        self.delay
            .get(fuel as usize)
            .copied()
            .flatten()
            .unwrap_or(1)
    }
}

/// A change of fire a Voxel wants.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Fire {
    /// Catches with this much fuel.
    Ignite {
        fuel: u16,
    },
    Extinguish,
    /// Burnt out: the solid becomes this, or air.
    Burnout {
        residue: Voxel,
    },
}

impl Fire {
    /// The change of fire `p` makes, read back from the state it proposes.
    pub(crate) fn of(p: &crate::flow::Proposal) -> Option<Fire> {
        match (p.read.burning, p.next.burning) {
            (None, Some(b)) => Some(Fire::Ignite { fuel: b.fuel }),
            (Some(_), None) if p.next.solid == p.read.solid => Some(Fire::Extinguish),
            (Some(_), None) => Some(Fire::Burnout {
                residue: p.next.solid,
            }),
            _ => None,
        }
    }

    /// `next` with this change made: what flow wants of the fluid is kept,
    /// moved into the residue's voids as a placement moves it.
    pub(crate) fn apply(self, next: Occupants, tables: &Tables) -> Occupants {
        match self {
            Fire::Ignite { fuel } => Occupants {
                burning: Some(Burning { fuel }),
                ..next
            },
            Fire::Extinguish => Occupants {
                burning: None,
                ..next
            },
            Fire::Burnout { residue } => next.place_solid(residue, tables),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct FireTimer {
    clock: Clock,
    want: Fire,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Clock {
    Runs {
        ready: Step,
    },
    /// Stopped beside terrain nobody holds with `left` steps to go, for the
    /// solid it was running for.
    Frozen {
        left: Step,
        solid: Voxel,
    },
}

impl FireTimer {
    pub(crate) fn new(ready: Step, want: Fire) -> Self {
        FireTimer {
            clock: Clock::Runs { ready },
            want,
        }
    }

    /// The step the change is due, evaluated at `step` for `solid`: a frozen
    /// timer goes on from where it stopped, unless the solid was replaced.
    fn ready(&self, step: Step, solid: Voxel) -> Option<Step> {
        match self.clock {
            Clock::Runs { ready } => Some(ready),
            Clock::Frozen { left, solid: was } => (was == solid).then_some(step + left),
        }
    }

    /// Whether this timer still describes `here`: an ignition of unburning
    /// fuel, or a fire that still burns.
    fn fits(&self, here: &Occupants) -> bool {
        match self.want {
            Fire::Ignite { .. } => here.burning.is_none(),
            Fire::Burnout { .. } | Fire::Extinguish => here.burning.is_some(),
        }
    }

    fn freeze(&mut self, step: Step, solid: Voxel) {
        if let Clock::Runs { ready } = self.clock {
            self.clock = Clock::Frozen {
                left: ready.saturating_sub(step),
                solid,
            };
        }
    }
}

/// A change of fire due this step, and the state it was decided from.
#[derive(Clone, Copy)]
pub(crate) struct Burnt {
    pub here: Occupants,
    pub change: Fire,
    /// Decided from exposure, so read as far as [`HeatScale::radius`].
    pub heated: bool,
}

fn extinguishes(tables: &Tables, fluid: Option<Fluid>) -> Option<Fluid> {
    fluid.filter(|f| {
        tables
            .fluids
            .get(f.material)
            .is_some_and(|d| d.extinguishes)
    })
}

/// Whether an extinguishing fluid touches the solid at `at`, `fuel` as seen
/// there: in its own voids, or across a face where it meets the solid. `None`
/// when it is dry where held and a neighbour is not held.
fn wetness(view: &impl CellView, tables: &Tables, at: Cell, fuel: &Seen) -> Option<bool> {
    if extinguishes(tables, fuel.occupants.fluid).is_some() {
        return Some(true);
    }
    let mut unknown = false;
    for (face, d) in FACE_STEPS.into_iter().enumerate() {
        let Some(other) = look(view, tables, step_by(at, d)) else {
            unknown = true;
            continue;
        };
        if let Some(f) = extinguishes(tables, other.occupants.fluid)
            && against(fuel, face, &other).any(|j| f.level(j).is_wet())
        {
            return Some(true);
        }
    }
    (!unknown).then_some(false)
}

/// Whether an extinguishing fluid touches the solid at `at`, as fuel evaluation
/// decides it: terrain nobody holds never counts.
pub fn is_wet(view: &impl CellView, tables: &Tables, at: Cell) -> bool {
    look(view, tables, at).is_some_and(|seen| wetness(view, tables, at, &seen) == Some(true))
}

/// Thermal evaluation of `at`, sharing the step's timers, wakeups and work.
pub(crate) fn evaluate(
    view: &impl CellView,
    tables: &Tables,
    heat: &HeatScale,
    at: Cell,
    cont: &mut Continuation,
) -> Option<Burnt> {
    if !heat.fuel {
        return None;
    }
    let idle = |cont: &mut Continuation| -> Option<Burnt> {
        cont.timers.fire.remove(&at);
        None
    };
    let Some(seen) = look(view, tables, at) else {
        return idle(cont);
    };
    let here = seen.occupants;
    let Some(def) = (!here.solid.is_air())
        .then(|| tables.combustion.get(here.solid.material()))
        .flatten()
    else {
        return idle(cont);
    };
    cont.work.fuel += 1;
    let Some(wet) = wetness(view, tables, at, &seen) else {
        cont.work.paused += 1;
        let step = cont.step;
        match cont.timers.fire.get_mut(&at) {
            Some(timer) if timer.fits(&here) => timer.freeze(step, here.solid),
            Some(_) => {
                cont.timers.fire.remove(&at);
            }
            None => {}
        }
        return None;
    };
    let (want, delay, heated) = match here.burning {
        Some(_) if wet => (Fire::Extinguish, 1, false),
        Some(burning) => match def.burn {
            Burn::Permanent => return idle(cont),
            Burn::For { residue, .. } => {
                let residue = if residue == crate::voxel::AIR {
                    Voxel::AIR
                } else {
                    Voxel::new(residue, here.solid.shape(), here.solid.orientation())
                };
                (Fire::Burnout { residue }, Step::from(burning.fuel), false)
            }
        },
        None if wet => return idle(cont),
        None => {
            if exposure(view, tables, heat, at, &seen, &mut cont.work) < def.threshold {
                return idle(cont);
            }
            let fuel = def.fuel_steps();
            (
                Fire::Ignite { fuel },
                heat.delay(here.solid.material()),
                true,
            )
        }
    };
    let step = cont.step;
    let ready = cont
        .timers
        .fire
        .get(&at)
        .filter(|timer| timer.want == want)
        .and_then(|timer| timer.ready(step, here.solid))
        .unwrap_or(step + delay.max(1) - 1);
    if ready > step {
        cont.timers.fire.insert(at, FireTimer::new(ready, want));
        cont.wakeups.push((ready, at));
        cont.work.waiting += 1;
        return None;
    }
    cont.timers.fire.remove(&at);
    Some(Burnt {
        here,
        change: want,
        heated,
    })
}

/// The squares of face `face` a solid covers: what is not open into one of its
/// voids. Nothing for empty space.
fn surface(seen: &Seen, face: usize) -> FaceMask {
    if seen.occupants.solid.is_air() {
        return FaceMask::EMPTY;
    }
    seen.voids
        .iter()
        .fold(FaceMask::EMPTY, |open, v| open.union(&v.open[face]))
        .complement()
}

/// The voids of `other`, across face `face` of `solid`, that meet its surface
/// there: the space touching it from that side.
fn against<'a>(solid: &Seen, face: usize, other: &Seen<'a>) -> impl Iterator<Item = usize> + 'a {
    let covered = surface(solid, face);
    let voids = other.voids;
    (0..voids.len()).filter(move |&j| voids[j].open[face ^ 1].intersects(&covered))
}

/// The strongest exposure reaching the space the fuel at `at` touches, by a
/// breadth-first search of open space that stops as soon as nothing farther
/// could be stronger.
fn exposure(
    view: &impl CellView,
    tables: &Tables,
    heat: &HeatScale,
    at: Cell,
    fuel: &Seen,
    work: &mut Work,
) -> u8 {
    let Some(limit) = heat.steps else {
        return 0;
    };
    let mut cache: HashMap<Cell, Option<Seen>> = HashMap::new();
    cache.insert(at, Some(*fuel));
    let mut lookup =
        |c: Cell| -> Option<Seen> { *cache.entry(c).or_insert_with(|| look(view, tables, c)) };
    let mut visited: HashSet<(Cell, usize)> = HashSet::new();
    let mut frontier: Vec<(Cell, usize)> = Vec::new();
    for i in 0..fuel.voids.len() {
        visited.insert((at, i));
        frontier.push((at, i));
    }
    for (face, d) in FACE_STEPS.into_iter().enumerate() {
        let beside = step_by(at, d);
        let Some(other) = lookup(beside) else {
            continue;
        };
        for j in against(fuel, face, &other) {
            if visited.insert((beside, j)) {
                frontier.push((beside, j));
            }
        }
    }
    let mut best = 0;
    for depth in 0..=limit {
        if frontier.is_empty() || best >= heat.ceiling[depth] {
            break;
        }
        let mut next = Vec::new();
        for &(c, i) in &frontier {
            work.searched += 1;
            let node = lookup(c).expect("entered only where held");
            let void = &node.voids[i];
            if let Some(f) = node.occupants.fluid
                && f.level(i).is_wet()
            {
                best = best.max(heat.exposure(f.material, depth));
            }
            if node.occupants.burning.is_some() {
                best = best.max(heat.exposure(node.occupants.solid.material(), depth));
            }
            for (face, d) in FACE_STEPS.into_iter().enumerate() {
                let beside = step_by(c, d);
                let Some(other) = lookup(beside) else {
                    continue;
                };
                if other.occupants.burning.is_some()
                    && heat.gives_heat(other.occupants.solid.material())
                    && void.open[face].intersects(&surface(&other, face ^ 1))
                {
                    best = best.max(heat.exposure(other.occupants.solid.material(), depth));
                }
                if depth == limit {
                    continue;
                }
                for (j, w) in other.voids.iter().enumerate() {
                    if void.open[face].intersects(&w.open[face ^ 1]) && visited.insert((beside, j))
                    {
                        next.push((beside, j));
                    }
                }
            }
        }
        frontier = next;
    }
    best
}

/// The exposure the solid at `at` is under, the number an evaluation holds
/// against its threshold: 0 where nothing is held there, and 0 below every
/// fuel's threshold, where the search stops as nothing there could catch.
pub fn exposure_at(view: &impl CellView, tables: &Tables, heat: &HeatScale, at: Cell) -> u8 {
    look(view, tables, at).map_or(0, |seen| {
        exposure(view, tables, heat, at, &seen, &mut Work::default())
    })
}

/// The box of Voxels within `radius` of `at` along every axis.
fn around(at: Cell, radius: i32) -> (Cell, Cell) {
    (at.map(|c| c - radius), at.map(|c| c + radius))
}

/// Wakes, at `step`, every fuel Voxel whose evaluation reads `at`: within
/// [`HeatScale::radius`] of a change of solid, of fire or of hot fluid there.
/// Only Chunks `holds_fuel` admits are looked into, so terrain without fuel
/// costs nothing.
pub fn wake_fuel_around(
    agenda: &mut Agenda,
    at: Cell,
    step: Step,
    heat: &HeatScale,
    holds_fuel: impl Fn([i32; 3]) -> bool,
    is_fuel: impl Fn(Cell) -> bool,
) {
    if !heat.fuel {
        return;
    }
    let (min, max) = around(at, heat.radius());
    for span in Span::split(min, max) {
        if !holds_fuel(span.home()) {
            continue;
        }
        for x in span.min[0]..=span.max[0] {
            for y in span.min[1]..=span.max[1] {
                for z in span.min[2]..=span.max[2] {
                    if is_fuel([x, y, z]) {
                        agenda.wake([x, y, z], step);
                    }
                }
            }
        }
    }
}

/// The parts of Chunks `holds_fuel` admits within [`HeatScale::radius`] of
/// any Voxel of `min..=max`, for a rescan when a whole Chunk arrives, leaves
/// or changes past the journal.
pub fn fuel_spans_around(
    min: Cell,
    max: Cell,
    heat: &HeatScale,
    holds_fuel: impl Fn([i32; 3]) -> bool,
) -> Vec<Span> {
    if !heat.fuel {
        return Vec::new();
    }
    let r = heat.radius();
    Span::split(min.map(|c| c - r), max.map(|c| c + r))
        .into_iter()
        .filter(|s| holds_fuel(s.home()))
        .collect()
}

/// Whether a change from `before` to `after` can change a fuel's exposure
/// nearby: a solid (passage, or fuel itself), a fire, or a hot fluid.
pub fn alters_exposure(tables: &Tables, before: &Occupants, after: &Occupants) -> bool {
    if tables.combustion.is_empty() {
        return false;
    }
    let hot = |o: &Occupants| {
        o.fluid
            .and_then(|f| tables.fluids.get(f.material))
            .is_some_and(|f| f.heat.is_some())
    };
    before.solid != after.solid
        || before.burning != after.burning
        || ((hot(before) || hot(after)) && before.fluid != after.fluid)
}
