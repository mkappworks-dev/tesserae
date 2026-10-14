//! Which Chunks each LOD level has drawn, as a buffer the chunk shader reads.
//!
//! A finer level's blended pixel composites over a coarser one instead of
//! replacing it, so where both have water it is drawn twice. The coarse level
//! cannot know from its own geometry where the finer one has drawn, which
//! depends on where the Eye has been and what has streamed in, so the Client
//! tells it: a coarse blended fragment inside a Chunk the finer level was sent
//! a mesh for is discarded. See ticket 07 of
//! `.scratch/2026-10-12-material-table-transparency/`.
//!
//! The layout, which `chunk_material.wesl`'s `finer_drawn` mirrors:
//! word 0 is the level count `L`; then, for each level `k` in `0..L`, eight
//! words describing the Chunks of level `k - 1` (level 0's are unused): the
//! offset of its bits, its window's origin as three `i32`, its window's size as
//! three `u32`, and level `k - 1`'s Cell size in metres as `f32` bits. Then the
//! bits, x fastest, then y, then z.

use bevy::platform::collections::HashSet;
use bevy::prelude::*;

use crate::world::{ChunkCoord, WorldSettings};

const SLOT: usize = 8;

/// The words for the delivered set, every level from 1 to `levels - 1` asking
/// about the level below it.
pub(super) fn encode(
    levels: u32,
    delivered: &HashSet<(u32, ChunkCoord)>,
    world: &WorldSettings,
) -> Vec<u32> {
    let header = 1 + SLOT * levels as usize;
    let mut words = vec![0; header];
    words[0] = levels;
    for level in 1..levels {
        let finer = level - 1;
        let coords: Vec<IVec3> = delivered
            .iter()
            .filter(|(l, _)| *l == finer)
            .map(|(_, c)| IVec3::new(c.x, c.y, c.z))
            .collect();
        let Some(origin) = coords.iter().copied().reduce(IVec3::min) else {
            continue;
        };
        let size = (coords.iter().copied().fold(origin, IVec3::max) - origin + 1).as_uvec3();
        let slot = 1 + SLOT * level as usize;
        let offset = words.len();
        words[slot] = offset as u32;
        words[slot + 1..slot + 4].copy_from_slice(&origin.to_array().map(|v| v as u32));
        words[slot + 4..slot + 7].copy_from_slice(&size.to_array());
        words[slot + 7] = world.cell_size_at(finer).to_bits();
        let bits = (size.x * size.y * size.z) as usize;
        words.resize(offset + bits.div_ceil(32), 0);
        for c in coords {
            let i = index(c - origin, size);
            words[offset + i / 32] |= 1 << (i % 32);
        }
    }
    words
}

fn index(local: IVec3, size: UVec3) -> usize {
    let local = local.as_uvec3();
    ((local.z * size.y + local.y) * size.x + local.x) as usize
}

/// How far behind a face its own Voxel is sampled, as a fraction of a Cell.
/// `chunk_material.wesl`'s `BEHIND_A_FACE`.
#[cfg(test)]
const BEHIND_A_FACE: f32 = 1.0 / 1024.0;

/// Whether the level below `level` has drawn the Chunk holding the face at
/// `point_m` facing `normal`: what the shader asks, for the tests to ask the
/// same way, the sample a hair behind the face included.
#[cfg(test)]
pub fn contains(words: &[u32], level: u32, point_m: Vec3, normal: Vec3) -> bool {
    if level == 0 || words.first().is_none_or(|&levels| level >= levels) {
        return false;
    }
    let slot = 1 + SLOT * level as usize;
    let size = UVec3::new(words[slot + 4], words[slot + 5], words[slot + 6]);
    if size.min_element() == 0 {
        return false;
    }
    let origin = IVec3::new(
        words[slot + 1] as i32,
        words[slot + 2] as i32,
        words[slot + 3] as i32,
    );
    let cell_m = f32::from_bits(words[slot + 7]);
    let behind = point_m - normal.normalize() * cell_m * BEHIND_A_FACE;
    let local = (behind / cell_m).floor().as_ivec3() - origin;
    if local.min_element() < 0 || local.as_uvec3().cmpge(size).any() {
        return false;
    }
    let i = index(local, size);
    words[words[slot] as usize + i / 32] & (1 << (i % 32)) != 0
}
