//! What a generator keeps per Cell, by `[cx, cz]`: each a pure function of the seed, the
//! parameters and the Cell, so a cache only ever saves time.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Each Cell with when it was kept, oldest lowest.
type Aged<T> = HashMap<[i32; 2], (u64, Arc<T>)>;

/// A map of Cells that drops its older half on reaching its bound, so a flight keeps the Cells
/// round the Eye rather than starting over.
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
        cells.get(&key).map(|(_, t)| Arc::clone(t))
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
            let mut ages: Vec<u64> = cells.values().map(|(age, _)| *age).collect();
            let half = ages.len() / 2;
            let (_, &mut newer, _) = ages.select_nth_unstable(half);
            cells.retain(|_, (age, _)| *age >= newer);
        }
        let age = self.next.fetch_add(1, Ordering::Relaxed);
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
    fn a_cell_already_held_is_not_built_again() {
        let map = CellMap::new(2);
        map.get_or_build([0, 0], || 1);
        assert_eq!(*map.get_or_build([0, 0], || panic!("built twice")), 1);
    }
}
