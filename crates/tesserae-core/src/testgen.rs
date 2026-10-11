//! Deterministic terrain for tests and benches.
//!
//! Not the real World generation — that is a later step and belongs behind the
//! Mod-parameterised interface. This exists so the mesher is benchmarked against
//! a surface with real overhangs and material layers rather than a solid cube,
//! which would measure only the greedy fast path.

use crate::chunk::{CHUNK_SIZE, Chunk};
use crate::voxel::Voxel;

pub const STONE: u16 = 1;
pub const DIRT: u16 = 2;
pub const GRASS: u16 = 3;

const SEA_LEVEL: f32 = 12.0;
const RELIEF: f32 = 18.0;

#[inline]
fn mix(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    x
}

fn lattice(xi: i32, zi: i32, seed: u32) -> f32 {
    let h = mix(mix(xi as u32 ^ 0x9e37_79b9)
        .wrapping_add((zi as u32).wrapping_mul(0x85eb_ca6b))
        .wrapping_add(seed));
    (h >> 8) as f32 / ((1u32 << 24) as f32)
}

fn noise2(x: f32, z: f32, seed: u32) -> f32 {
    let (x0, z0) = (x.floor(), z.floor());
    let (tx, tz) = (x - x0, z - z0);
    let sx = tx * tx * (3.0 - 2.0 * tx);
    let sz = tz * tz * (3.0 - 2.0 * tz);
    let (xi, zi) = (x0 as i32, z0 as i32);
    let n0 = {
        let a = lattice(xi, zi, seed);
        a + (lattice(xi + 1, zi, seed) - a) * sx
    };
    let n1 = {
        let a = lattice(xi, zi + 1, seed);
        a + (lattice(xi + 1, zi + 1, seed) - a) * sx
    };
    n0 + (n1 - n0) * sz
}

fn surface_height(wx: f32, wz: f32) -> f32 {
    let mut value = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = 1.0 / 48.0;
    let mut total = 0.0;
    for octave in 0..3u32 {
        value += noise2(wx * frequency, wz * frequency, 0x5eed + octave) * amplitude;
        total += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    SEA_LEVEL + (value / total) * RELIEF
}

/// A Chunk of terrain at Chunk coordinates `(cx, cy, cz)`.
pub fn terrain_chunk(cx: i32, cy: i32, cz: i32) -> Chunk {
    let mut chunk = Chunk::air();
    let size = CHUNK_SIZE as i32;

    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            let wx = (cx * size + x as i32) as f32;
            let wz = (cz * size + z as i32) as f32;
            let height = surface_height(wx, wz) as i32;

            for y in 0..CHUNK_SIZE {
                let wy = cy * size + y as i32;
                if wy > height {
                    continue;
                }
                let material = if wy == height {
                    GRASS
                } else if wy > height - 4 {
                    DIRT
                } else {
                    STONE
                };
                chunk.set(x, y, z, Voxel::cube(material));
            }
        }
    }

    chunk
}

/// Alternating solid and air — the mesher's worst case, where nothing merges.
pub fn checkerboard_chunk() -> Chunk {
    let mut chunk = Chunk::air();
    for y in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                if (x + y + z) % 2 == 0 {
                    chunk.set(x, y, z, Voxel::cube(STONE));
                }
            }
        }
    }
    chunk
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terrain_is_deterministic() {
        let a = terrain_chunk(3, 0, -2);
        let b = terrain_chunk(3, 0, -2);
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    assert_eq!(a.get(x, y, z), b.get(x, y, z));
                }
            }
        }
    }

    #[test]
    fn terrain_has_a_surface_inside_the_first_chunk() {
        let c = terrain_chunk(0, 0, 0);
        assert!(c.uniform().is_none(), "expected both solid and air");
        assert!(c.palette_len() >= 3, "expected stone, dirt and grass");
    }

    #[test]
    fn checkerboard_alternates() {
        let c = checkerboard_chunk();
        assert!(!c.get(0, 0, 0).is_air());
        assert!(c.get(1, 0, 0).is_air());
    }
}
