//! Which faces of a Chunk see each other through it: the 15 face-pair bits
//! Minecraft's `VisGraph` and Sodium compute per section, for a walk from the
//! camera's Chunk that skips what nothing in front of it lets the eye reach.

use crate::chunk::{CHUNK_SIZE, Chunk, ROWS};
use crate::material::{MaterialTable, Occlusion};
use crate::mesh::{NEG_X, NEG_Y, NEG_Z, POS_X, POS_Y, POS_Z};
use crate::voxel::Voxel;

const N: usize = CHUNK_SIZE;
/// A row along x is one `u32`, bit `x` set where the Voxel lets sight through.
type Row = u32;
const ENDS: Row = 1 | 1 << (N - 1);

/// One bit per unordered pair of the mesher's six faces.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Connectivity(u16);

impl Connectivity {
    pub const NONE: Connectivity = Connectivity(0);
    pub const ALL: Connectivity = Connectivity((1 << 15) - 1);

    /// Whether a line of sight entering through face `a` can leave through
    /// face `b`. A face always reaches itself.
    pub fn connects(self, a: usize, b: usize) -> bool {
        a == b || self.0 & pair(a, b) != 0
    }

    pub fn bits(self) -> u16 {
        self.0
    }
}

/// The bit of the unordered pair `{a, b}`, `a != b`.
fn pair(a: usize, b: usize) -> u16 {
    let (a, b) = (a.min(b), a.max(b));
    debug_assert!(a != b && b < 6);
    1 << (a * 5 - a * a.saturating_sub(1) / 2 + b - a - 1)
}

/// Which faces of `chunk` see each other through the Voxels that do not block
/// sight. Only a full cube of an [`Occlusion::Opaque`] Material blocks it:
/// glass, leaves and every Shape but the cube let it through, which errs
/// towards drawing.
pub fn connectivity(chunk: &Chunk, materials: &MaterialTable) -> Connectivity {
    let blocks = |v: Voxel| v.is_full_cube() && materials.occlusion(v) == Occlusion::Opaque;
    if let Some(v) = chunk.uniform() {
        return if blocks(v) {
            Connectivity::NONE
        } else {
            Connectivity::ALL
        };
    }
    let mut open = [0 as Row; ROWS];
    chunk.rows_where(|v| !blocks(v), &mut open);
    let blocked: u32 = open.iter().map(|row| row.count_zeros()).sum();
    // Every pair of faces is joined by N² disjoint paths, so separating any
    // two takes at least one blocking Voxel per path.
    if (blocked as usize) < ROWS {
        return Connectivity::ALL;
    }
    flood(&open)
}

/// Floods each open region that touches the boundary, a row of Voxels at a
/// time, and joins every pair of faces one region touches.
fn flood(open: &[Row; ROWS]) -> Connectivity {
    let mut seen = [0 as Row; ROWS];
    let mut stack = Vec::new();
    let mut out = Connectivity::NONE;
    for y in 0..N {
        for z in 0..N {
            let i = y * N + z;
            let on_boundary = y == 0 || y == N - 1 || z == 0 || z == N - 1;
            let reach = if on_boundary { Row::MAX } else { ENDS };
            loop {
                let seeds = open[i] & !seen[i] & reach;
                if seeds == 0 {
                    break;
                }
                let faces = region(open, &mut seen, &mut stack, i, seeds & seeds.wrapping_neg());
                out.0 |= joined(faces);
                if out == Connectivity::ALL {
                    return out;
                }
            }
        }
    }
    out
}

/// Marks the open region holding bit `seed` of row `start` seen, and returns
/// the faces it touches as a bit per face.
fn region(
    open: &[Row; ROWS],
    seen: &mut [Row; ROWS],
    stack: &mut Vec<(usize, Row)>,
    start: usize,
    seed: Row,
) -> u8 {
    let mut faces = 0u8;
    stack.push((start, seed));
    while let Some((i, from)) = stack.pop() {
        let free = open[i] & !seen[i];
        if from & free == 0 {
            continue;
        }
        let filled = along(from & free, free);
        seen[i] |= filled;
        let (y, z) = (i / N, i % N);
        faces |= u8::from(filled & 1 != 0) << NEG_X
            | u8::from(filled >> (N - 1) != 0) << POS_X
            | u8::from(y == 0) << NEG_Y
            | u8::from(y == N - 1) << POS_Y
            | u8::from(z == 0) << NEG_Z
            | u8::from(z == N - 1) << POS_Z;
        let mut visit = |j: usize| {
            let next = filled & open[j] & !seen[j];
            if next != 0 {
                stack.push((j, next));
            }
        };
        if y > 0 {
            visit(i - N);
        }
        if y < N - 1 {
            visit(i + N);
        }
        if z > 0 {
            visit(i - 1);
        }
        if z < N - 1 {
            visit(i + 1);
        }
    }
    faces
}

/// Every bit of `free` in a run of `free` that holds a bit of `seed`: a
/// Kogge-Stone fill each way along the row.
fn along(seed: Row, free: Row) -> Row {
    let (mut up, mut down) = (seed, seed);
    let (mut pu, mut pd) = (free, free);
    for shift in [1, 2, 4, 8, 16] {
        up |= pu & (up << shift);
        pu &= pu << shift;
        down |= pd & (down >> shift);
        pd &= pd >> shift;
    }
    up | down
}

/// The pair bits of every two faces in `faces`.
fn joined(faces: u8) -> u16 {
    let mut bits = 0;
    for a in 0..6 {
        for b in a + 1..6 {
            if faces >> a & faces >> b & 1 != 0 {
                bits |= pair(a, b);
            }
        }
    }
    bits
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testgen::fixtures;

    const FACES: [usize; 6] = [NEG_X, POS_X, NEG_Y, POS_Y, NEG_Z, POS_Z];
    const LAST: usize = CHUNK_SIZE - 1;

    fn of(chunk: &Chunk) -> Connectivity {
        connectivity(chunk, &fixtures().tables.materials)
    }

    fn rock() -> Voxel {
        Voxel::cube(fixtures().ground.rock)
    }

    fn pairs(c: Connectivity) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        for a in FACES {
            for b in FACES {
                if a < b && c.connects(a, b) {
                    out.push((a, b));
                }
            }
        }
        out
    }

    #[test]
    fn every_pair_has_its_own_bit() {
        let mut seen = 0u16;
        for a in 0..6 {
            for b in a + 1..6 {
                assert_eq!(seen & pair(a, b), 0, "{a}-{b} shares a bit");
                seen |= pair(a, b);
            }
        }
        assert_eq!(seen, Connectivity::ALL.bits());
    }

    #[test]
    fn open_air_connects_every_pair_of_faces() {
        assert_eq!(of(&Chunk::air()), Connectivity::ALL);
    }

    #[test]
    fn solid_rock_connects_nothing() {
        assert_eq!(of(&Chunk::filled(rock())), Connectivity::NONE);
    }

    #[test]
    fn a_sealed_pocket_connects_nothing() {
        let mut chunk = Chunk::filled(rock());
        for x in 8..24 {
            for y in 8..24 {
                for z in 8..24 {
                    chunk.set(x, y, z, Voxel::AIR);
                }
            }
        }
        assert_eq!(of(&chunk), Connectivity::NONE);
    }

    #[test]
    fn a_straight_tunnel_connects_only_its_two_ends() {
        let mut chunk = Chunk::filled(rock());
        for x in 0..CHUNK_SIZE {
            chunk.set(x, 16, 16, Voxel::AIR);
        }
        assert_eq!(pairs(of(&chunk)), [(NEG_X, POS_X)]);
    }

    #[test]
    fn a_bending_tunnel_connects_the_faces_it_opens_on() {
        let mut chunk = Chunk::filled(rock());
        for x in 0..=20 {
            chunk.set(x, 5, 9, Voxel::AIR);
        }
        for y in 5..CHUNK_SIZE {
            chunk.set(20, y, 9, Voxel::AIR);
        }
        // A side passage that starts and ends inside the Chunk changes nothing.
        for z in 9..30 {
            chunk.set(10, 5, z, Voxel::AIR);
        }
        assert_eq!(pairs(of(&chunk)), [(NEG_X, POS_Y)]);
    }

    #[test]
    fn a_wall_across_x_separates_its_two_sides_and_nothing_else() {
        let mut chunk = Chunk::air();
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                chunk.set(16, y, z, rock());
            }
        }
        let c = of(&chunk);
        assert!(!c.connects(NEG_X, POS_X));
        for side in [NEG_X, POS_X] {
            for face in [NEG_Y, POS_Y, NEG_Z, POS_Z] {
                assert!(c.connects(side, face), "{side} to {face}");
            }
        }
        assert!(c.connects(NEG_Y, POS_Y) && c.connects(NEG_Z, POS_Z));
    }

    #[test]
    fn a_wall_with_one_hole_does_not_separate() {
        let mut chunk = Chunk::air();
        for x in [16, 17] {
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    chunk.set(x, y, z, rock());
                }
            }
            chunk.set(x, LAST, 0, Voxel::AIR);
        }
        assert_eq!(of(&chunk), Connectivity::ALL);
    }

    #[test]
    fn walls_of_glass_leaves_or_slabs_do_not_block_sight() {
        let f = fixtures();
        let slab = Voxel::shaped(
            f.ground.rock,
            crate::SHAPE_SLAB,
            crate::Orientation::new(POS_Y, 0),
        );
        for wall in [Voxel::cube(f.glass), Voxel::cube(f.leaves), slab] {
            let mut chunk = Chunk::air();
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    chunk.set(16, y, z, wall);
                }
            }
            assert_eq!(of(&chunk), Connectivity::ALL, "{wall:?}");
        }
    }
}
