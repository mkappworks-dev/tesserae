//! Why streaming work ran, for the profiler. A mark that queues a Chunk for
//! work keeps the cause it was first made for, and dispatch counts the cause it
//! took. Without `dev` or `profiling` both are kept as nothing, and in those
//! builds only while [`crate::cost::collecting`].

use std::ops::Deref;

use bevy::platform::collections::HashSet;

use crate::world::ChunkCoord;

/// Room for the most causes any kind of work has.
#[cfg(any(feature = "dev", feature = "profiling"))]
pub const CAUSES: usize = 6;

pub trait Cause: Copy {
    #[cfg(any(feature = "dev", feature = "profiling"))]
    fn index(self) -> usize;
}

/// What wrote a level-0 Chunk's Voxels or light through an `edit::Batch`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Writer {
    /// The fluid simulation landing a step: fluid flowing and fire.
    Fluid,
    /// Anything else: an Edit, a Command, an undo, a Plant.
    #[default]
    Edit,
}

/// What a level-0 light solve was for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SolveCause {
    /// The Chunk landed.
    First,
    /// A neighbour's solve moved the plane across their shared face.
    Neighbour,
    /// A write dropped its last solve: one that ran while the write landed.
    Fluid,
    Edit,
}

impl SolveCause {
    /// The flow CSV's column suffixes, by [`Cause::index`].
    #[cfg(feature = "profiling")]
    pub const NAMES: [&str; 4] = ["first", "neighbour", "fluid", "edit"];

    pub fn spoilt_by(writer: Writer) -> Self {
        match writer {
            Writer::Fluid => SolveCause::Fluid,
            Writer::Edit => SolveCause::Edit,
        }
    }
}

impl Cause for SolveCause {
    #[cfg(any(feature = "dev", feature = "profiling"))]
    fn index(self) -> usize {
        self as usize
    }
}

/// Why a coarse Chunk was generated or meshed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LodCause {
    /// Generated because it was not held; meshed because it was generated so.
    First,
    /// A write to a level-0 Chunk under it, or for a mesh under a face
    /// neighbour, regenerated it.
    Fluid,
    Edit,
    /// An edited level-0 Chunk under it left level 0's store or came back from
    /// the save.
    Reload,
    /// A finer level's Chunk under it was regenerated and came out different.
    Finer,
    /// Meshes only: the anchor moved, and its mesh was released, or cancelled
    /// or landed outside `keep`.
    Moved,
}

impl LodCause {
    /// The flow CSV's column suffixes, by [`Cause::index`].
    #[cfg(feature = "profiling")]
    pub const NAMES: [&str; CAUSES] = ["first", "fluid", "edit", "reload", "finer", "moved"];

    pub fn of(writer: Option<Writer>) -> Self {
        match writer {
            Some(Writer::Fluid) => LodCause::Fluid,
            Some(Writer::Edit) => LodCause::Edit,
            None => LodCause::Reload,
        }
    }
}

impl Cause for LodCause {
    #[cfg(any(feature = "dev", feature = "profiling"))]
    fn index(self) -> usize {
        self as usize
    }
}

/// A set of Chunks marked for work, each with the cause of the mark that put it
/// there. A mark on a Chunk already marked keeps the first cause: it adds no
/// work.
pub struct Marks<C> {
    set: HashSet<ChunkCoord>,
    causes: kept::ByChunk<C>,
}

impl<C> Default for Marks<C> {
    fn default() -> Self {
        Marks {
            set: HashSet::default(),
            causes: kept::ByChunk::default(),
        }
    }
}

impl<C> Deref for Marks<C> {
    type Target = HashSet<ChunkCoord>;

    fn deref(&self) -> &HashSet<ChunkCoord> {
        &self.set
    }
}

impl<C: Cause> Marks<C> {
    pub fn insert(&mut self, coord: ChunkCoord, cause: C) -> bool {
        let new = self.set.insert(coord);
        if new {
            self.causes.insert(coord, cause);
        }
        new
    }

    pub fn remove(&mut self, coord: &ChunkCoord) -> bool {
        self.causes.remove(*coord);
        self.set.remove(coord)
    }

    pub fn retain(&mut self, mut keep: impl FnMut(&ChunkCoord) -> bool) {
        self.set.retain(|c| keep(c));
        self.causes.retain_in(&self.set);
    }

    #[cfg(test)]
    pub fn clear(&mut self) {
        self.set.clear();
        self.causes.clear();
    }

    /// `None` when not counting.
    pub fn cause(&self, coord: &ChunkCoord) -> Option<C> {
        self.causes.get(*coord)
    }
}

/// Work dispatched since the World opened, by cause.
pub struct Counts<C> {
    tally: kept::Tally,
    _cause: std::marker::PhantomData<C>,
}

impl<C> Default for Counts<C> {
    fn default() -> Self {
        Counts {
            tally: Default::default(),
            _cause: std::marker::PhantomData,
        }
    }
}

impl<C: Cause> Counts<C> {
    /// Counts nothing for `None`, a mark made while not counting.
    pub fn add(&mut self, cause: Option<C>) {
        if let Some(cause) = cause {
            self.tally.add(cause);
        }
    }

    /// Indexed by [`Cause::index`].
    #[cfg(any(feature = "dev", feature = "profiling"))]
    pub fn all(&self) -> [u64; CAUSES] {
        self.tally.all()
    }
}

/// The last writer of each held level-0 Chunk.
#[derive(Default)]
pub struct Writers {
    last: kept::ByChunk<Writer>,
}

impl Writers {
    pub fn wrote(&mut self, coord: ChunkCoord, writer: Writer) {
        self.last.insert(coord, writer);
    }

    /// `None` for a Chunk nothing wrote since it landed, or when not counting.
    pub fn of(&self, coord: ChunkCoord) -> Option<Writer> {
        self.last.get(coord)
    }

    pub fn forget(&mut self, coord: ChunkCoord) {
        self.last.remove(coord);
    }
}

#[cfg(any(feature = "dev", feature = "profiling"))]
mod kept {
    use bevy::platform::collections::{HashMap, HashSet};

    use super::{CAUSES, Cause};
    use crate::world::ChunkCoord;

    pub struct ByChunk<V>(HashMap<ChunkCoord, V>);

    impl<V> Default for ByChunk<V> {
        fn default() -> Self {
            ByChunk(HashMap::default())
        }
    }

    impl<V: Copy> ByChunk<V> {
        pub fn insert(&mut self, coord: ChunkCoord, value: V) {
            if crate::cost::collecting() {
                self.0.insert(coord, value);
            }
        }

        pub fn get(&self, coord: ChunkCoord) -> Option<V> {
            self.0.get(&coord).copied()
        }

        pub fn remove(&mut self, coord: ChunkCoord) {
            self.0.remove(&coord);
        }

        pub fn retain_in(&mut self, set: &HashSet<ChunkCoord>) {
            if !self.0.is_empty() {
                self.0.retain(|c, _| set.contains(c));
            }
        }

        #[cfg(test)]
        pub fn clear(&mut self) {
            self.0.clear();
        }
    }

    #[derive(Default)]
    pub struct Tally([u64; CAUSES]);

    impl Tally {
        pub fn add(&mut self, cause: impl Cause) {
            if crate::cost::collecting() {
                self.0[cause.index()] += 1;
            }
        }

        pub fn all(&self) -> [u64; CAUSES] {
            self.0
        }
    }
}

#[cfg(not(any(feature = "dev", feature = "profiling")))]
mod kept {
    use bevy::platform::collections::HashSet;

    use super::Cause;
    use crate::world::ChunkCoord;

    pub struct ByChunk<V>(std::marker::PhantomData<V>);

    impl<V> Default for ByChunk<V> {
        fn default() -> Self {
            ByChunk(std::marker::PhantomData)
        }
    }

    impl<V: Copy> ByChunk<V> {
        pub fn insert(&mut self, _: ChunkCoord, _: V) {}

        pub fn get(&self, _: ChunkCoord) -> Option<V> {
            None
        }

        pub fn remove(&mut self, _: ChunkCoord) {}

        pub fn retain_in(&mut self, _: &HashSet<ChunkCoord>) {}

        #[cfg(test)]
        pub fn clear(&mut self) {}
    }

    #[derive(Default)]
    pub struct Tally;

    impl Tally {
        pub fn add(&mut self, _: impl Cause) {}
    }
}
