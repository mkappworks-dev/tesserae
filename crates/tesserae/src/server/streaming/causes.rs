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
    /// Anything else: an Edit, a Command, an undo, a plant.
    #[default]
    Edit,
}

/// What a level-0 light solve was for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Solve {
    /// The Chunk landed.
    First,
    /// A neighbour's solve moved the plane across their shared face.
    Neighbour,
    /// A write dropped its last solve: one that ran while the write landed.
    Fluid,
    Edit,
}

impl Solve {
    /// The flow CSV's column suffixes, by [`Cause::index`].
    #[cfg(feature = "profiling")]
    pub const NAMES: [&str; 4] = ["first", "neighbour", "fluid", "edit"];

    pub fn spoilt_by(writer: Writer) -> Self {
        match writer {
            Writer::Fluid => Solve::Fluid,
            Writer::Edit => Solve::Edit,
        }
    }
}

impl Cause for Solve {
    #[cfg(any(feature = "dev", feature = "profiling"))]
    fn index(self) -> usize {
        self as usize
    }
}

/// Why a coarse Chunk was generated or meshed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lod {
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
    /// Meshes only, and last: the anchor moved, and its mesh was released, or
    /// cancelled or landed outside `keep`.
    Moved,
}

impl Lod {
    /// The flow CSV's column suffixes, by [`Cause::index`].
    #[cfg(feature = "profiling")]
    pub const NAMES: [&str; CAUSES] = ["first", "fluid", "edit", "reload", "finer", "moved"];

    pub fn of(writer: Option<Writer>) -> Self {
        match writer {
            Some(Writer::Fluid) => Lod::Fluid,
            Some(Writer::Edit) => Lod::Edit,
            None => Lod::Reload,
        }
    }
}

impl Cause for Lod {
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
    #[cfg(any(feature = "dev", feature = "profiling"))]
    causes: bevy::platform::collections::HashMap<ChunkCoord, C>,
    #[cfg(not(any(feature = "dev", feature = "profiling")))]
    causes: std::marker::PhantomData<C>,
}

impl<C> Default for Marks<C> {
    fn default() -> Self {
        Marks {
            set: HashSet::default(),
            causes: Default::default(),
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
        #[cfg(any(feature = "dev", feature = "profiling"))]
        if new && crate::cost::collecting() {
            self.causes.insert(coord, cause);
        }
        #[cfg(not(any(feature = "dev", feature = "profiling")))]
        let _ = cause;
        new
    }

    pub fn remove(&mut self, coord: &ChunkCoord) -> bool {
        #[cfg(any(feature = "dev", feature = "profiling"))]
        self.causes.remove(coord);
        self.set.remove(coord)
    }

    pub fn retain(&mut self, mut keep: impl FnMut(&ChunkCoord) -> bool) {
        self.set.retain(|c| keep(c));
        #[cfg(any(feature = "dev", feature = "profiling"))]
        if !self.causes.is_empty() {
            let set = &self.set;
            self.causes.retain(|c, _| set.contains(c));
        }
    }

    #[cfg(test)]
    pub fn clear(&mut self) {
        self.set.clear();
        #[cfg(any(feature = "dev", feature = "profiling"))]
        self.causes.clear();
    }

    /// `None` when not counting.
    pub fn cause(&self, coord: &ChunkCoord) -> Option<C> {
        #[cfg(any(feature = "dev", feature = "profiling"))]
        let cause = self.causes.get(coord).copied();
        #[cfg(not(any(feature = "dev", feature = "profiling")))]
        let cause = {
            let _ = coord;
            None
        };
        cause
    }
}

/// Work dispatched since the World opened, by cause.
pub struct Counts<C> {
    #[cfg(any(feature = "dev", feature = "profiling"))]
    counts: [u64; CAUSES],
    _cause: std::marker::PhantomData<C>,
}

impl<C> Default for Counts<C> {
    fn default() -> Self {
        Counts {
            #[cfg(any(feature = "dev", feature = "profiling"))]
            counts: [0; CAUSES],
            _cause: std::marker::PhantomData,
        }
    }
}

impl<C: Cause> Counts<C> {
    /// Counts nothing for `None`, a mark made while not counting.
    pub fn add(&mut self, cause: Option<C>) {
        #[cfg(any(feature = "dev", feature = "profiling"))]
        if let Some(cause) = cause.filter(|_| crate::cost::collecting()) {
            self.counts[cause.index()] += 1;
        }
        #[cfg(not(any(feature = "dev", feature = "profiling")))]
        let _ = cause;
    }

    /// Indexed by [`Cause::index`].
    #[cfg(any(feature = "dev", feature = "profiling"))]
    pub fn all(&self) -> [u64; CAUSES] {
        self.counts
    }
}

/// The last writer of each held level-0 Chunk.
#[derive(Default)]
pub struct Writers {
    #[cfg(any(feature = "dev", feature = "profiling"))]
    last: bevy::platform::collections::HashMap<ChunkCoord, Writer>,
}

impl Writers {
    pub fn wrote(&mut self, coord: ChunkCoord, writer: Writer) {
        #[cfg(any(feature = "dev", feature = "profiling"))]
        if crate::cost::collecting() {
            self.last.insert(coord, writer);
        }
        #[cfg(not(any(feature = "dev", feature = "profiling")))]
        let _ = (coord, writer);
    }

    /// `None` for a Chunk nothing wrote since it landed, or when not counting.
    pub fn of(&self, coord: ChunkCoord) -> Option<Writer> {
        #[cfg(any(feature = "dev", feature = "profiling"))]
        let writer = self.last.get(&coord).copied();
        #[cfg(not(any(feature = "dev", feature = "profiling")))]
        let writer = {
            let _ = coord;
            None
        };
        writer
    }

    pub fn forget(&mut self, coord: ChunkCoord) {
        #[cfg(any(feature = "dev", feature = "profiling"))]
        self.last.remove(&coord);
        #[cfg(not(any(feature = "dev", feature = "profiling")))]
        let _ = coord;
    }
}
