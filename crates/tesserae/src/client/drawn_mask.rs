//! Which Chunks each LOD level draws, as a buffer the chunk shader reads.
//!
//! A level-k fragment is discarded wherever a finer level draws the Chunk at
//! that point, an empty mesh included, so every point is drawn by the finest
//! level that has it, and by one level only. See
//! `docs/adr/0019-a-level-is-drawn-until-another-replaces-it.md`.
//!
//! Each level's Cell carries two bits: *some*, that its own level or a finer
//! one draws part of it, and *all*, that they draw all of it. A fragment asks
//! the level just below its own: all means discard, none means draw, and only
//! a Cell that is partly covered sends it a level further down. Asking every
//! finer level instead cost about a millisecond a frame at 1080p.
//!
//! The layout, which `coverage.wesl`'s `cover_at` mirrors: word 0 is the
//! level count `L`; then, for each level `k` in `0..L`, eight words: the offset
//! of its bits, its window's origin as three `i32`, its window's size as three
//! `u32`, and its Cell size in metres as `f32` bits. Then the bits, two per Cell
//! (some, then all), x fastest, then y, then z.

use bevy::platform::collections::HashSet;
use bevy::prelude::*;

use crate::world::{ChunkCoord, WorldSettings};

const SLOT: usize = 8;

/// Cells added on every side when a window is rebuilt, so a Chunk landing just
/// past the last one does not rebuild it again.
const MARGIN: i32 = 4;

pub const SOME: u32 = 1;
pub const ALL: u32 = 2;

/// One level's drawn Chunks, what they and finer levels cover, and the box of
/// bits the shader is given for it.
#[derive(Default)]
struct Level {
    drawn: HashSet<IVec3>,
    some: HashSet<IVec3>,
    all: HashSet<IVec3>,
    origin: IVec3,
    size: UVec3,
    bits: Vec<u32>,
}

impl Level {
    fn index(&self, coord: IVec3) -> Option<usize> {
        let local = coord - self.origin;
        (local.min_element() >= 0 && local.as_uvec3().cmplt(self.size).all())
            .then(|| index(local, self.size))
    }

    fn cover(&self, coord: IVec3) -> u32 {
        u32::from(self.some.contains(&coord)) * SOME + u32::from(self.all.contains(&coord)) * ALL
    }

    /// Records `cover` for `coord`, and whether it changed.
    fn put(&mut self, coord: IVec3, cover: u32) -> bool {
        if self.cover(coord) == cover {
            return false;
        }
        set_member(&mut self.some, coord, cover & SOME != 0);
        set_member(&mut self.all, coord, cover & ALL != 0);
        match self.index(coord) {
            Some(i) => self.write_bits(i, cover),
            None if cover != 0 => self.rebuild(),
            None => {}
        }
        true
    }

    fn write_bits(&mut self, i: usize, cover: u32) {
        let (word, shift) = (i / 16, (i % 16) * 2);
        self.bits[word] = (self.bits[word] & !(3 << shift)) | (cover << shift);
    }

    fn rebuild(&mut self) {
        let Some(min) = self.some.iter().copied().reduce(IVec3::min) else {
            self.origin = IVec3::ZERO;
            self.size = UVec3::ZERO;
            self.bits.clear();
            return;
        };
        let max = self.some.iter().copied().fold(min, IVec3::max);
        self.origin = min - MARGIN;
        self.size = (max - min + 1 + 2 * MARGIN).as_uvec3();
        let count = (self.size.x * self.size.y * self.size.z) as usize;
        self.bits = vec![0; count.div_ceil(16)];
        let cells: Vec<IVec3> = self.some.iter().copied().collect();
        for coord in cells {
            let i = index(coord - self.origin, self.size);
            self.write_bits(i, self.cover(coord));
        }
    }
}

fn set_member(set: &mut HashSet<IVec3>, coord: IVec3, on: bool) {
    if on {
        set.insert(coord);
    } else {
        set.remove(&coord);
    }
}

fn index(local: IVec3, size: UVec3) -> usize {
    let local = local.as_uvec3();
    ((local.z * size.y + local.y) * size.x + local.x) as usize
}

fn children(c: IVec3) -> impl Iterator<Item = IVec3> {
    (0..8).map(move |i| 2 * c + IVec3::new(i & 1, i >> 1 & 1, i >> 2))
}

/// The drawn set per level and what it covers, kept as the shader's bits and
/// updated a Chunk at a time.
#[derive(Default)]
pub(super) struct DrawnMask {
    levels: Vec<Level>,
}

impl DrawnMask {
    pub fn set(&mut self, (level, coord): (u32, ChunkCoord), on: bool) {
        let level = level as usize;
        self.grow_to(level + 2);
        let coord = IVec3::new(coord.x, coord.y, coord.z);
        let drawn = &mut self.levels[level].drawn;
        if on == drawn.contains(&coord) {
            return;
        }
        set_member(drawn, coord, on);
        // Up the levels until a Cell's cover does not change.
        let mut cell = coord;
        for at in level..self.levels.len() {
            let cover = self.derive(at, cell);
            if !self.levels[at].put(cell, cover) {
                break;
            }
            cell = cell.div_euclid(IVec3::splat(2));
        }
    }

    fn derive(&self, at: usize, cell: IVec3) -> u32 {
        if self.levels[at].drawn.contains(&cell) {
            return SOME | ALL;
        }
        let Some(finer) = at.checked_sub(1).map(|f| &self.levels[f]) else {
            return 0;
        };
        let some = children(cell).any(|c| finer.some.contains(&c));
        let all = children(cell).all(|c| finer.all.contains(&c));
        u32::from(some) * SOME + u32::from(all) * ALL
    }

    /// A level added above the others starts with the cover of the one below.
    fn grow_to(&mut self, levels: usize) {
        while self.levels.len() < levels {
            let at = self.levels.len();
            self.levels.push(Level::default());
            let Some(finer) = at.checked_sub(1) else {
                continue;
            };
            let parents: HashSet<IVec3> = self.levels[finer]
                .some
                .iter()
                .map(|c| c.div_euclid(IVec3::splat(2)))
                .collect();
            for cell in parents {
                let cover = self.derive(at, cell);
                self.levels[at].put(cell, cover);
            }
        }
    }

    pub fn words(&self, world: &WorldSettings) -> Vec<u32> {
        let header = 1 + SLOT * self.levels.len();
        let mut words = vec![0; header];
        words[0] = self.levels.len() as u32;
        for (level, cells) in self.levels.iter().enumerate() {
            let slot = 1 + SLOT * level;
            words[slot] = words.len() as u32;
            words[slot + 1..slot + 4].copy_from_slice(&cells.origin.to_array().map(|v| v as u32));
            words[slot + 4..slot + 7].copy_from_slice(&cells.size.to_array());
            words[slot + 7] = world.cell_size_at(level as u32).to_bits();
            words.extend_from_slice(&cells.bits);
        }
        words
    }
}

/// The cover bits of `level`'s Chunk at `coord`, read from the words the way
/// the shader reads them.
#[cfg(any(test, feature = "profiling"))]
pub fn cover_at(words: &[u32], level: u32, coord: IVec3) -> u32 {
    if words.first().is_none_or(|&levels| level >= levels) {
        return 0;
    }
    let slot = 1 + SLOT * level as usize;
    let size = UVec3::new(words[slot + 4], words[slot + 5], words[slot + 6]);
    let origin = IVec3::new(
        words[slot + 1] as i32,
        words[slot + 2] as i32,
        words[slot + 3] as i32,
    );
    let local = coord - origin;
    if local.min_element() < 0 || local.as_uvec3().cmpge(size).any() {
        return 0;
    }
    let i = index(local, size);
    (words[words[slot] as usize + i / 16] >> ((i % 16) * 2)) & 3
}

/// Whether a level finer than `level` draws the point: the shader's walk.
#[cfg(any(test, feature = "profiling"))]
pub fn finer_drawn_at(words: &[u32], level: u32, point_m: Vec3) -> bool {
    let levels = words.first().copied().unwrap_or(0);
    let mut finer = level.min(levels);
    while finer > 0 {
        finer -= 1;
        let cell_m = f32::from_bits(words[1 + SLOT * finer as usize + 7]);
        match cover_at(words, finer, (point_m / cell_m).floor().as_ivec3()) {
            0 => return false,
            c if c & ALL != 0 => return true,
            _ => {}
        }
    }
    false
}

/// How far behind a face its own Voxel is sampled, as a fraction of a Cell.
/// `coverage.wesl`'s `BEHIND_A_FACE`.
#[cfg(test)]
const BEHIND_A_FACE: f32 = 1.0 / 1024.0;

/// [`finer_drawn_at`] for a face, sampled a hair behind it as the shader does,
/// by the level-0 Cell.
#[cfg(test)]
pub fn finer_drawn(words: &[u32], level: u32, point_m: Vec3, normal: Vec3) -> bool {
    if words.first().is_none_or(|&l| l == 0) {
        return false;
    }
    let cell_m = f32::from_bits(words[1 + 7]);
    finer_drawn_at(
        words,
        level,
        point_m - normal.normalize() * cell_m * BEHIND_A_FACE,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world() -> WorldSettings {
        WorldSettings::default()
    }

    fn key(level: u32, x: i32, y: i32, z: i32) -> (u32, ChunkCoord) {
        (level, ChunkCoord::new(x, y, z))
    }

    #[test]
    fn a_chunk_is_drawn_once_set_and_not_once_cleared_inside_and_outside_its_window() {
        let mut mask = DrawnMask::default();
        mask.set(key(0, 0, 0, 0), true);
        mask.set(key(0, 40, -3, 9), true);
        let words = mask.words(&world());
        assert_eq!(cover_at(&words, 0, IVec3::ZERO), SOME | ALL);
        assert_eq!(cover_at(&words, 0, IVec3::new(40, -3, 9)), SOME | ALL);
        assert_eq!(cover_at(&words, 0, IVec3::new(1, 0, 0)), 0);
        mask.set(key(0, 40, -3, 9), false);
        let words = mask.words(&world());
        assert_eq!(cover_at(&words, 0, IVec3::new(40, -3, 9)), 0);
        assert_eq!(cover_at(&words, 0, IVec3::ZERO), SOME | ALL);
    }

    #[test]
    fn a_cell_is_partly_covered_by_some_children_and_wholly_by_all_eight() {
        let mut mask = DrawnMask::default();
        // Far away, so levels 1 to 3 exist.
        mask.set(key(3, 50, 50, 50), true);
        let kids: Vec<IVec3> = children(IVec3::ZERO).collect();
        for &c in &kids[..7] {
            mask.set(key(0, c.x, c.y, c.z), true);
        }
        let words = mask.words(&world());
        assert_eq!(cover_at(&words, 1, IVec3::ZERO), SOME);
        assert_eq!(cover_at(&words, 2, IVec3::ZERO), SOME, "and up the levels");
        let c = kids[7];
        mask.set(key(0, c.x, c.y, c.z), true);
        let words = mask.words(&world());
        assert_eq!(cover_at(&words, 1, IVec3::ZERO), SOME | ALL);
        assert_eq!(cover_at(&words, 2, IVec3::ZERO), SOME);
    }

    #[test]
    fn a_coarse_fragment_is_discarded_exactly_where_a_finer_level_draws() {
        let mut mask = DrawnMask::default();
        let cell_m = world().cell_size();
        // Level 3 draws; level 0 draws one Chunk inside it and level 1 another.
        mask.set(key(3, 0, 0, 0), true);
        mask.set(key(0, 1, 0, 0), true);
        mask.set(key(1, 2, 1, 0), true);
        let words = mask.words(&world());
        let at = |x: f32, y: f32| Vec3::new(x, y, 0.5) * cell_m;
        assert!(finer_drawn_at(&words, 3, at(1.5, 0.5)), "level 0's Chunk");
        assert!(!finer_drawn_at(&words, 3, at(0.5, 0.5)), "beside it");
        assert!(finer_drawn_at(&words, 3, at(4.5, 2.5)), "level 1's Chunk");
        assert!(!finer_drawn_at(&words, 3, at(6.5, 0.5)));
        assert!(
            !finer_drawn_at(&words, 0, at(1.5, 0.5)),
            "level 0 is never discarded"
        );
        assert!(finer_drawn_at(&words, 1, at(1.5, 0.5)));
        assert!(
            !finer_drawn_at(&words, 1, at(4.5, 2.5)),
            "level 1 draws its own"
        );
    }

    #[test]
    fn a_level_added_above_the_others_inherits_their_cover() {
        let mut mask = DrawnMask::default();
        mask.set(key(0, 5, 0, 0), true);
        mask.set(key(4, 9, 9, 9), true);
        let words = mask.words(&world());
        assert_eq!(cover_at(&words, 3, IVec3::ZERO), SOME);
        let cell_m = world().cell_size();
        assert!(finer_drawn_at(&words, 4, Vec3::new(5.5, 0.5, 0.5) * cell_m));
    }
}
