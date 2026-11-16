//! What a lattice cell stands, kept once worked out: every Chunk a Structure or a Settlement
//! reaches would otherwise ask the ground under all of it again.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex};

use super::settlements::{Site, Town};
use super::structures::Structure;

/// Lattice cells by row index and cell.
type CellKey = (usize, [i32; 2]);

/// Shared by every clone of one generator, as streaming clones it per Chunk. Each value is a
/// pure function of the generator and its key, so which thread or Chunk works it out first
/// changes nothing.
#[derive(Default)]
pub(super) struct Layouts {
    structures: Memo<CellKey, Option<Structure>>,
    sites: Memo<CellKey, Option<Site>>,
    towns: Memo<CellKey, Option<Arc<Town>>>,
}

impl Layouts {
    pub(super) fn structure(
        &self,
        key: CellKey,
        work: impl FnOnce() -> Option<Structure>,
    ) -> Option<Structure> {
        self.structures.get_or(key, STRUCTURES_KEPT, work)
    }

    pub(super) fn site(&self, key: CellKey, work: impl FnOnce() -> Option<Site>) -> Option<Site> {
        self.sites.get_or(key, SITES_KEPT, work)
    }

    pub(super) fn town(
        &self,
        key: CellKey,
        work: impl FnOnce() -> Option<Town>,
    ) -> Option<Arc<Town>> {
        self.towns.get_or(key, TOWNS_KEPT, || work().map(Arc::new))
    }
}

impl std::fmt::Debug for Layouts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Layouts")
    }
}

const STRUCTURES_KEPT: usize = 16_384;
const SITES_KEPT: usize = 4_096;
/// A Town at the largest radius holds about 85 KB of road and grade.
const TOWNS_KEPT: usize = 32;

#[derive(Default)]
struct Memo<K, V>(Mutex<HashMap<K, V>>);

impl<K: Eq + Hash, V: Clone> Memo<K, V> {
    /// Forgets everything once `kept` are held: cheaper than ordering them, and a cell asked again
    /// is only worked out again.
    fn get_or(&self, key: K, kept: usize, work: impl FnOnce() -> V) -> V {
        if let Some(v) = self.lock().get(&key) {
            return v.clone();
        }
        // Worked out unlocked, so another thread's cell never waits on this one.
        let v = work();
        let mut map = self.lock();
        if map.len() >= kept {
            map.clear();
        }
        map.entry(key).or_insert(v).clone()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<K, V>> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
