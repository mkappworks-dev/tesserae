//! What a generator keeps per Cell, by `[cx, cz]`: each a pure function of the seed, the
//! parameters and the Cell, so a cache only ever saves time.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Each Cell with when it was last built or read, oldest lowest. A read stores its age under the
/// read lock, so a racing read may lose its refresh; that costs only a rebuild.
type Aged<T> = HashMap<[i32; 2], (AtomicU64, Arc<T>)>;

/// A map of Cells that drops its least recently used half on reaching its bound, so a flight
/// keeps the Cells round the Eye rather than starting over.
pub(super) struct CellMap<T> {
    /// Tells this map from any other, for a thread's record of the last Cell it asked.
    id: u64,
    bound: usize,
    next: AtomicU64,
    cells: RwLock<Aged<T>>,
}

impl<T> std::fmt::Debug for CellMap<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CellMap")
            .field("id", &self.id)
            .field("bound", &self.bound)
            .finish_non_exhaustive()
    }
}

impl<T> CellMap<T> {
    pub(super) fn new(bound: usize) -> Self {
        CellMap {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            bound,
            next: AtomicU64::new(0),
            cells: RwLock::default(),
        }
    }

    pub(super) fn id(&self) -> u64 {
        self.id
    }

    pub(super) fn get(&self, key: [i32; 2]) -> Option<Arc<T>> {
        let cells = self.cells.read().expect("cells");
        let (age, t) = cells.get(&key)?;
        // A load and a compare, not a write, when nothing was built since: a held Cell is read
        // by every worker.
        let now = self.next.load(Ordering::Relaxed);
        if age.load(Ordering::Relaxed) != now {
            age.store(now, Ordering::Relaxed);
        }
        Some(Arc::clone(t))
    }

    /// Built outside the lock, so two workers may both build a Cell; they build the same, and
    /// the first kept is the one both get.
    pub(super) fn get_or_build(&self, key: [i32; 2], build: impl FnOnce() -> T) -> Arc<T> {
        if let Some(t) = self.get(key) {
            return t;
        }
        let built = Arc::new(build());
        let mut cells = self.cells.write().expect("cells");
        if !cells.contains_key(&key) && cells.len() >= self.bound {
            // By key within an age, since reads between two builds share one.
            let mut ages: Vec<(u64, [i32; 2])> = cells
                .iter()
                .map(|(key, (age, _))| (age.load(Ordering::Relaxed), *key))
                .collect();
            let half = ages.len() / 2;
            let (_, &mut newer, _) = ages.select_nth_unstable(half);
            cells.retain(|key, (age, _)| (*age.get_mut(), *key) >= newer);
        }
        let age = AtomicU64::new(self.next.fetch_add(1, Ordering::Relaxed));
        let (_, t) = cells.entry(key).or_insert((age, built));
        Arc::clone(t)
    }

    pub(super) fn clear(&self) {
        self.cells.write().expect("cells").clear();
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.cells.read().expect("cells").len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reaching_the_bound_keeps_the_newer_half() {
        let map = CellMap::new(8);
        for i in 0..8 {
            map.get_or_build([i, 0], || i);
        }
        map.get_or_build([8, 0], || 8);
        assert_eq!(map.len(), 5);
        for i in 0..4 {
            assert!(map.get([i, 0]).is_none(), "{i}");
        }
        for i in 4..9 {
            assert_eq!(map.get([i, 0]).as_deref(), Some(&i));
        }
    }

    #[test]
    fn a_cell_read_after_newer_ones_were_built_survives_the_drop() {
        let map = CellMap::new(8);
        for i in 0..8 {
            map.get_or_build([i, 0], || i);
        }
        map.get([0, 0]);
        map.get_or_build([8, 0], || 8);
        assert_eq!(map.get([0, 0]).as_deref(), Some(&0));
        assert_eq!(map.len(), 5);
    }

    #[test]
    fn a_cell_already_held_is_not_built_again() {
        let map = CellMap::new(2);
        map.get_or_build([0, 0], || 1);
        assert_eq!(*map.get_or_build([0, 0], || panic!("built twice")), 1);
    }
}
