//! Which Chunks each LOD level draws, as a buffer the chunk shader reads.
//!
//! A level-k fragment is discarded wherever a finer level draws the Chunk at
//! that point, an empty mesh included, so every point is drawn by the finest
//! level that has it, and by one level only. See
//! `docs/adr/0019-a-level-is-drawn-until-another-replaces-it.md`.
//!
//! The layout, which `chunk_material.wesl`'s `drawn_at` mirrors: word 0 is the
//! level count `L`; then, for each level `k` in `0..L`, eight words: the offset
//! of its bits, its window's origin as three `i32`, its window's size as three
//! `u32`, and its Cell size in metres as `f32` bits. Then the bits, x fastest,
//! then y, then z.

use bevy::platform::collections::HashSet;
use bevy::prelude::*;

use crate::world::{ChunkCoord, WorldSettings};

const SLOT: usize = 8;

/// Cells added on every side when a window is rebuilt, so a Chunk landing just
/// past the last one does not rebuild it again.
const MARGIN: i32 = 4;

/// One level's drawn Chunks, and the box of bits the shader is given for them.
#[derive(Default)]
struct Window {
    members: HashSet<IVec3>,
    origin: IVec3,
    size: UVec3,
    bits: Vec<u32>,
}

impl Window {
    fn index(&self, coord: IVec3) -> Option<usize> {
        let local = coord - self.origin;
        (local.min_element() >= 0 && local.as_uvec3().cmplt(self.size).all())
            .then(|| index(local, self.size))
    }

    fn set(&mut self, coord: IVec3, on: bool) {
        let changed = if on {
            self.members.insert(coord)
        } else {
            self.members.remove(&coord)
        };
        if !changed {
            return;
        }
        match self.index(coord) {
            Some(i) if on => self.bits[i / 32] |= 1 << (i % 32),
            Some(i) => self.bits[i / 32] &= !(1 << (i % 32)),
            None if on => self.rebuild(),
            None => {}
        }
    }

    fn rebuild(&mut self) {
        let Some(min) = self.members.iter().copied().reduce(IVec3::min) else {
            *self = Window::default();
            return;
        };
        let max = self.members.iter().copied().fold(min, IVec3::max);
        self.origin = min - MARGIN;
        self.size = (max - min + 1 + 2 * MARGIN).as_uvec3();
        let count = (self.size.x * self.size.y * self.size.z) as usize;
        self.bits = vec![0; count.div_ceil(32)];
        for &coord in &self.members {
            let i = index(coord - self.origin, self.size);
            self.bits[i / 32] |= 1 << (i % 32);
        }
    }
}

fn index(local: IVec3, size: UVec3) -> usize {
    let local = local.as_uvec3();
    ((local.z * size.y + local.y) * size.x + local.x) as usize
}

/// The drawn set per level, kept as the shader's bitsets and updated a Chunk
/// at a time.
#[derive(Default)]
pub(super) struct DrawnMask {
    levels: Vec<Window>,
}

impl DrawnMask {
    pub fn set(&mut self, (level, coord): (u32, ChunkCoord), on: bool) {
        let level = level as usize;
        if self.levels.len() <= level {
            self.levels.resize_with(level + 1, Window::default);
        }
        self.levels[level].set(IVec3::new(coord.x, coord.y, coord.z), on);
    }

    pub fn words(&self, world: &WorldSettings) -> Vec<u32> {
        let header = 1 + SLOT * self.levels.len();
        let mut words = vec![0; header];
        words[0] = self.levels.len() as u32;
        for (level, window) in self.levels.iter().enumerate() {
            let slot = 1 + SLOT * level;
            words[slot] = words.len() as u32;
            words[slot + 1..slot + 4].copy_from_slice(&window.origin.to_array().map(|v| v as u32));
            words[slot + 4..slot + 7].copy_from_slice(&window.size.to_array());
            words[slot + 7] = world.cell_size_at(level as u32).to_bits();
            words.extend_from_slice(&window.bits);
        }
        words
    }
}

/// How far behind a face its own Voxel is sampled, as a fraction of a Cell.
/// `chunk_material.wesl`'s `BEHIND_A_FACE`.
#[cfg(test)]
const BEHIND_A_FACE: f32 = 1.0 / 1024.0;

/// Whether `level` draws the Chunk at `coord`, read from the words the way the
/// shader reads them.
#[cfg(any(test, feature = "profiling"))]
pub fn drawn_at(words: &[u32], level: u32, coord: IVec3) -> bool {
    if words.first().is_none_or(|&levels| level >= levels) {
        return false;
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
        return false;
    }
    let i = index(local, size);
    words[words[slot] as usize + i / 32] & (1 << (i % 32)) != 0
}

/// The level-`level` Chunk holding `point_m`.
#[cfg(test)]
fn cell_of(words: &[u32], level: u32, point_m: Vec3) -> IVec3 {
    let cell_m = f32::from_bits(words[1 + SLOT * level as usize + 7]);
    (point_m / cell_m).floor().as_ivec3()
}

/// Whether a finer level than `level` draws the Chunk holding the face at
/// `point_m` facing `normal`: what the shader asks, for the tests to ask the
/// same way, the sample a hair behind the face included.
#[cfg(test)]
pub fn finer_drawn(words: &[u32], level: u32, point_m: Vec3, normal: Vec3) -> bool {
    let levels = words.first().copied().unwrap_or(0);
    (0..level.min(levels)).any(|finer| {
        let cell_m = f32::from_bits(words[1 + SLOT * finer as usize + 7]);
        let behind = point_m - normal.normalize() * cell_m * BEHIND_A_FACE;
        drawn_at(words, finer, cell_of(words, finer, behind))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world() -> WorldSettings {
        WorldSettings::default()
    }

    #[test]
    fn a_chunk_is_drawn_once_set_and_not_once_cleared_inside_and_outside_its_window() {
        let mut mask = DrawnMask::default();
        let near = (0, ChunkCoord::new(0, 0, 0));
        let far = (0, ChunkCoord::new(40, -3, 9));
        mask.set(near, true);
        mask.set(far, true);
        let words = mask.words(&world());
        assert!(drawn_at(&words, 0, IVec3::ZERO));
        assert!(drawn_at(&words, 0, IVec3::new(40, -3, 9)));
        assert!(!drawn_at(&words, 0, IVec3::new(1, 0, 0)));
        assert!(!drawn_at(&words, 1, IVec3::ZERO), "levels are separate");
        mask.set(far, false);
        let words = mask.words(&world());
        assert!(!drawn_at(&words, 0, IVec3::new(40, -3, 9)));
        assert!(drawn_at(&words, 0, IVec3::ZERO));
    }

    #[test]
    fn every_finer_level_is_asked_and_no_coarser_one() {
        let mut mask = DrawnMask::default();
        let cell_m = world().cell_size();
        mask.set((0, ChunkCoord::new(0, 0, 0)), true);
        mask.set((3, ChunkCoord::new(9, 9, 9)), true);
        let words = mask.words(&world());
        let inside = Vec3::splat(cell_m / 2.0);
        for level in 1..4 {
            assert!(finer_drawn(&words, level, inside, Vec3::Y), "level {level}");
        }
        assert!(!finer_drawn(&words, 0, inside, Vec3::Y));
        let mut mask = DrawnMask::default();
        mask.set((2, ChunkCoord::new(0, 0, 0)), true);
        mask.set((3, ChunkCoord::new(9, 9, 9)), true);
        let words = mask.words(&world());
        assert!(!finer_drawn(&words, 1, inside, Vec3::Y), "level 2 is coarser");
        assert!(finer_drawn(&words, 3, inside, Vec3::Y));
    }
}
