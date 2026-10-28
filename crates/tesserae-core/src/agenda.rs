//! Which Voxels a simulation evaluates at which logical step, within a bound.
//!
//! An entry is a Voxel and the earliest step anything asked for it. Entries
//! are capped: a wake that finds the agenda full is not lost but coalesced
//! into a rescan of a [`Span`] of its Chunk, and rescans are served a budget
//! of Voxels per step, round the Chunks in turn, each from a cursor. A
//! rescanned Voxel is evaluated exactly as a woken one, so overflow costs
//! evaluations and slows progress; it never changes what a Voxel becomes.
//!
//! A step takes its due Voxels and keeps their entries until
//! [`Agenda::end_step`], so whatever a step hands back for them through
//! [`Agenda::rewake`] always fits. `reserve` entries past `capacity` are open
//! to rescanned Voxels only: when wakes keep the rest full, a Chunk served by
//! a rescan can still start a timer, which is what keeps every Chunk moving.
//!
//! Nothing here iterates a hash map where order could show, so the same calls
//! give the same steps whatever the hasher's keys.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::chunk::CHUNK_SIZE;
use crate::flow::{Cell, Step, chunk_of};

/// The bounds, in Voxels. Part of a simulation's rules, like its step length:
/// unlike how a step is divided among workers, these decide when a Voxel under
/// load is evaluated.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Limits {
    /// Entries a wake may take.
    pub capacity: usize,
    /// Further entries only a rescanned Voxel may take.
    pub reserve: usize,
    /// Rescanned Voxels taken per step.
    pub scan_per_step: usize,
}

impl Limits {
    pub const UNBOUNDED: Limits = Limits {
        capacity: usize::MAX,
        reserve: 0,
        scan_per_step: usize::MAX,
    };
}

/// A box of Voxels inside one Chunk, both corners included.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Span {
    pub min: Cell,
    pub max: Cell,
}

impl Span {
    pub fn cell(at: Cell) -> Self {
        Span { min: at, max: at }
    }

    pub fn chunk(chunk: [i32; 3]) -> Self {
        let size = CHUNK_SIZE as i32;
        let min = chunk.map(|c| c * size);
        Span {
            min,
            max: min.map(|c| c + size - 1),
        }
    }

    /// The Chunk it lies in.
    pub fn home(&self) -> [i32; 3] {
        chunk_of(self.min).0
    }

    fn len(&self) -> usize {
        (0..3)
            .map(|a| (self.max[a] - self.min[a] + 1) as usize)
            .product()
    }

    pub fn contains(&self, other: &Span) -> bool {
        (0..3).all(|a| self.min[a] <= other.min[a] && other.max[a] <= self.max[a])
    }

    pub fn contains_cell(&self, at: Cell) -> bool {
        (0..3).all(|a| self.min[a] <= at[a] && at[a] <= self.max[a])
    }

    fn union(&self, other: &Span) -> Span {
        Span {
            min: std::array::from_fn(|a| self.min[a].min(other.min[a])),
            max: std::array::from_fn(|a| self.max[a].max(other.max[a])),
        }
    }

    /// The `i`th Voxel in coordinate order.
    fn nth(&self, i: usize) -> Cell {
        let dy = (self.max[1] - self.min[1] + 1) as usize;
        let dz = (self.max[2] - self.min[2] + 1) as usize;
        [
            self.min[0] + (i / (dy * dz)) as i32,
            self.min[1] + (i / dz % dy) as i32,
            self.min[2] + (i % dz) as i32,
        ]
    }

    /// Any box of Voxels, cut along Chunk boundaries, in Chunk order.
    pub fn split(min: Cell, max: Cell) -> Vec<Span> {
        let size = CHUNK_SIZE as i32;
        let lo = chunk_of(min).0;
        let hi = chunk_of(max).0;
        let mut spans = Vec::new();
        for cx in lo[0]..=hi[0] {
            for cy in lo[1]..=hi[1] {
                for cz in lo[2]..=hi[2] {
                    let c = [cx, cy, cz];
                    spans.push(Span {
                        min: std::array::from_fn(|a| min[a].max(c[a] * size)),
                        max: std::array::from_fn(|a| max[a].min(c[a] * size + size - 1)),
                    });
                }
            }
        }
        spans
    }
}

/// Spans a Chunk keeps per pass; past it the two whose covering box adds the
/// fewest Voxels merge, so separate clusters of work stay separate.
const MAX_SPANS: usize = 8;

fn add_span(spans: &mut Vec<Span>, span: Span) {
    if spans.iter().any(|s| s.contains(&span)) {
        return;
    }
    spans.retain(|s| !span.contains(s));
    spans.push(span);
    while spans.len() > MAX_SPANS {
        let mut best = (usize::MAX, 0, 1);
        for i in 0..spans.len() {
            for j in i + 1..spans.len() {
                let added = spans[i]
                    .union(&spans[j])
                    .len()
                    .saturating_sub(spans[i].len() + spans[j].len());
                if added < best.0 {
                    best = (added, i, j);
                }
            }
        }
        let (_, i, j) = best;
        let merged = spans[i].union(&spans[j]);
        spans.remove(j);
        spans[i] = merged;
        let keep = spans[i];
        let mut k = 0;
        spans.retain(|s| {
            k += 1;
            k - 1 == i || !keep.contains(s)
        });
    }
}

/// One Chunk's coalesced work: the pass under way and the one after it. A
/// span added mid-pass waits for the next, so a Voxel the cursor has passed is
/// still visited after the request.
#[derive(Clone, Debug, Default)]
struct Rescan {
    pass: Vec<Span>,
    span: usize,
    offset: usize,
    next: Vec<Span>,
    /// Voxels whose change could not land yet; served only once released.
    parked: Option<Span>,
}

impl Rescan {
    fn pending(&self) -> bool {
        self.span < self.pass.len() || !self.next.is_empty()
    }

    /// The Voxel under the cursor, starting the next pass if this one is done.
    fn current(&mut self) -> Option<Cell> {
        loop {
            if self.span >= self.pass.len() {
                if self.next.is_empty() {
                    return None;
                }
                self.pass = std::mem::take(&mut self.next);
                self.span = 0;
                self.offset = 0;
            }
            let span = self.pass[self.span];
            if self.offset < span.len() {
                return Some(span.nth(self.offset));
            }
            self.span += 1;
            self.offset = 0;
        }
    }
}

/// What [`Agenda::take_due`] hands a step.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Taken {
    /// In coordinate order.
    pub cells: Vec<Cell>,
    /// How many of them a rescan supplied, counted as work whether or not it
    /// found anything to do.
    pub scanned: usize,
}

#[derive(Clone, Debug)]
pub struct Agenda {
    limits: Limits,
    due: HashMap<Cell, Step>,
    queue: BTreeSet<(Step, Cell)>,
    in_step: HashSet<Cell>,
    rescans: BTreeMap<[i32; 3], Rescan>,
    /// The Chunk whose rescan was served last, so the next step starts after it.
    turn: Option<[i32; 3]>,
    overflowed: usize,
}

impl Default for Agenda {
    fn default() -> Self {
        Agenda::new(Limits::UNBOUNDED)
    }
}

impl Agenda {
    pub fn new(limits: Limits) -> Self {
        Agenda {
            limits,
            due: HashMap::new(),
            queue: BTreeSet::new(),
            in_step: HashSet::new(),
            rescans: BTreeMap::new(),
            turn: None,
            overflowed: 0,
        }
    }

    /// Checkpoint only between logical steps: worker-owned entries and timers
    /// must land before the Server takes its durable snapshot.
    pub fn checkpoint(&self, now: Step) -> Result<crate::save::Agenda, crate::save::Error> {
        if !self.in_step.is_empty() {
            return Err(crate::save::Error::Incompatible(
                "simulation step is still running".into(),
            ));
        }
        let mut wakes: Vec<_> = self
            .due
            .iter()
            .map(|(&at, &due)| (at, due.saturating_sub(now)))
            .collect();
        wakes.sort_unstable();
        Ok(crate::save::Agenda {
            capacity: self.limits.capacity,
            reserve: self.limits.reserve,
            scan_per_step: self.limits.scan_per_step,
            wakes,
            turn: self.turn,
            rescans: self
                .rescans
                .iter()
                .map(|(&home, r)| crate::save::Rescan {
                    home,
                    pass: r.pass.iter().map(|s| (s.min, s.max)).collect(),
                    span: r.span,
                    offset: r.offset,
                    next: r.next.iter().map(|s| (s.min, s.max)).collect(),
                    parked: r.parked.map(|s| (s.min, s.max)),
                })
                .collect(),
        })
    }

    pub fn restore(saved: &crate::save::Agenda, now: Step) -> Result<Self, crate::save::Error> {
        use crate::save::Error;
        let invalid = || Error::Corrupt("invalid simulation Agenda".into());
        let cap = saved
            .capacity
            .checked_add(saved.reserve)
            .ok_or_else(invalid)?;
        if saved.wakes.len() > cap || saved.scan_per_step == 0 {
            return Err(invalid());
        }
        let mut agenda = Self::new(Limits {
            capacity: saved.capacity,
            reserve: saved.reserve,
            scan_per_step: saved.scan_per_step,
        });
        for &(at, remaining) in &saved.wakes {
            let due = now.checked_add(remaining).ok_or_else(invalid)?;
            if agenda.due.insert(at, due).is_some() {
                return Err(invalid());
            }
            agenda.queue.insert((due, at));
        }
        for r in &saved.rescans {
            let span = |&(min, max): &(Cell, Cell)| -> Result<Span, Error> {
                if (0..3).any(|a| min[a] > max[a])
                    || chunk_of(min).0 != r.home
                    || chunk_of(max).0 != r.home
                {
                    return Err(invalid());
                }
                Ok(Span { min, max })
            };
            let pass: Vec<_> = r.pass.iter().map(span).collect::<Result<_, _>>()?;
            if r.span > pass.len() || (r.span < pass.len() && r.offset >= pass[r.span].len()) {
                return Err(invalid());
            }
            let restored = Rescan {
                pass,
                span: r.span,
                offset: r.offset,
                next: r.next.iter().map(span).collect::<Result<_, _>>()?,
                parked: r.parked.as_ref().map(span).transpose()?,
            };
            if agenda.rescans.insert(r.home, restored).is_some() {
                return Err(invalid());
            }
        }
        agenda.turn = saved.turn;
        Ok(agenda)
    }

    /// Re-admit paused work using the ordinary bounded overflow policy.
    pub fn resume(
        &mut self,
        saved: &crate::save::Agenda,
        now: Step,
    ) -> Result<(), crate::save::Error> {
        let restored = Self::restore(saved, now)?;
        for (home, mut scan) in restored.rescans {
            if let Some(existing) = self.rescans.remove(&home) {
                for span in existing.pass.into_iter().chain(existing.next) {
                    add_span(&mut scan.next, span);
                }
                if let Some(parked) = existing.parked {
                    add_span(&mut scan.next, parked);
                }
            }
            self.rescans.insert(home, scan);
        }
        if self.turn.is_none() {
            self.turn = restored.turn;
        }
        for (at, due) in restored.due {
            if self.due.contains_key(&at)
                || self.len() < self.limits.capacity.saturating_add(self.limits.reserve)
            {
                self.set_due(at, due);
            } else {
                self.wake(at, due);
            }
        }
        Ok(())
    }

    pub fn homes(&self) -> BTreeSet<[i32; 3]> {
        self.due
            .keys()
            .chain(self.in_step.iter())
            .map(|at| chunk_of(*at).0)
            .chain(self.rescans.keys().copied())
            .collect()
    }

    pub fn limits(&self) -> Limits {
        self.limits
    }

    /// Entries held, including those of the step under way.
    pub fn len(&self) -> usize {
        self.due.len() + self.in_step.len()
    }

    /// Nothing due, under way or waiting for a rescan. Parked Voxels sleep.
    pub fn is_empty(&self) -> bool {
        self.len() == 0 && !self.rescans.values().any(Rescan::pending)
    }

    /// Chunks with a rescan waiting or under way.
    pub fn rescans(&self) -> usize {
        self.rescans.values().filter(|r| r.pending()).count()
    }

    /// Wakes that found the agenda full and went to a rescan.
    pub fn overflowed(&self) -> usize {
        self.overflowed
    }

    pub fn next_due(&self) -> Option<Step> {
        self.queue.first().map(|&(step, _)| step)
    }

    fn set_due(&mut self, cell: Cell, step: Step) {
        match self.due.insert(cell, step) {
            Some(was) if was <= step => {
                self.due.insert(cell, was);
            }
            Some(was) => {
                self.queue.remove(&(was, cell));
                self.queue.insert((step, cell));
            }
            None => {
                self.queue.insert((step, cell));
            }
        }
    }

    /// Asks for `cell` to be evaluated at `step`. When the agenda is full the
    /// request becomes a rescan of that Voxel, which serves it at a later
    /// step: so a wake that may overflow must be one for the next step.
    pub fn wake(&mut self, cell: Cell, step: Step) {
        // A Voxel already holding an entry, or the step's, needs no new one.
        if self.due.contains_key(&cell)
            || self.in_step.remove(&cell)
            || self.len() < self.limits.capacity
        {
            self.set_due(cell, step);
        } else {
            self.overflowed += 1;
            self.rescan(Span::cell(cell));
        }
    }

    /// Wakes a Voxel the step under way took, into the entry it already holds.
    pub fn rewake(&mut self, cell: Cell, step: Step) {
        if self.in_step.remove(&cell) {
            self.set_due(cell, step);
        } else {
            self.wake(cell, step);
        }
    }

    /// Asks for every Voxel of `span` to be evaluated, in turn with every other
    /// Chunk's rescans.
    pub fn rescan(&mut self, span: Span) {
        let rescan = self.rescans.entry(span.home()).or_default();
        add_span(&mut rescan.next, span);
    }

    /// Lets go of a Voxel the step under way took, until [`Agenda::unpark`]
    /// rescans it with the rest of its Chunk's parked Voxels.
    pub fn park(&mut self, cell: Cell) {
        self.in_step.remove(&cell);
        let rescan = self.rescans.entry(chunk_of(cell).0).or_default();
        rescan.parked = Some(match rescan.parked {
            Some(p) => p.union(&Span::cell(cell)),
            None => Span::cell(cell),
        });
    }

    pub fn unpark(&mut self, chunk: [i32; 3]) {
        if let Some(rescan) = self.rescans.get_mut(&chunk)
            && let Some(parked) = rescan.parked.take()
        {
            add_span(&mut rescan.next, parked);
        }
    }

    /// Drops everything for one Chunk: what is due, under way, rescanning or
    /// parked.
    pub fn forget(&mut self, chunk: [i32; 3]) {
        let mine = |c: &Cell| chunk_of(*c).0 == chunk;
        self.due.retain(|c, _| !mine(c));
        self.queue.retain(|(_, c)| !mine(c));
        self.in_step.retain(|c| !mine(c));
        self.rescans.remove(&chunk);
    }

    /// Starts a step: rescanned Voxels join it while the budget and the
    /// entries allow, then every Voxel due by `step` is taken.
    pub fn take_due(&mut self, step: Step) -> Taken {
        debug_assert!(self.in_step.is_empty(), "a step is still under way");
        let scanned = self.scan(step);
        let mut cells = Vec::new();
        while let Some(&(when, cell)) = self.queue.first() {
            if when > step {
                break;
            }
            self.queue.pop_first();
            self.due.remove(&cell);
            self.in_step.insert(cell);
            cells.push(cell);
        }
        cells.sort_unstable();
        Taken { cells, scanned }
    }

    /// Releases every entry of the step that nothing woke again.
    pub fn end_step(&mut self) {
        self.in_step.clear();
    }

    /// Round the Chunks with work, starting after the last one served, each
    /// taking an equal share of the budget.
    fn scan(&mut self, step: Step) -> usize {
        let pending: Vec<[i32; 3]> = self
            .rescans
            .iter()
            .filter(|(_, r)| r.pending())
            .map(|(&c, _)| c)
            .collect();
        if pending.is_empty() {
            return 0;
        }
        let budget = self.limits.scan_per_step;
        let ceiling = self.limits.capacity.saturating_add(self.limits.reserve);
        let start = self
            .turn
            .map_or(0, |t| pending.partition_point(|&c| c <= t));
        let share = budget.div_ceil(pending.len()).max(1);
        let mut scanned = 0;
        'chunks: for i in 0..pending.len() {
            if scanned >= budget {
                break;
            }
            let chunk = pending[(start + i) % pending.len()];
            self.turn = Some(chunk);
            for _ in 0..share {
                if scanned >= budget {
                    break;
                }
                let rescan = self.rescans.get_mut(&chunk).expect("pending");
                let Some(cell) = rescan.current() else {
                    break;
                };
                let held = self.due.len() + self.in_step.len();
                if !self.due.contains_key(&cell) && held >= ceiling {
                    break 'chunks;
                }
                rescan.offset += 1;
                self.set_due(cell, step);
                scanned += 1;
            }
        }
        self.rescans
            .retain(|_, r| r.pending() || r.parked.is_some());
        scanned
    }
}

#[cfg(test)]
mod tests;
