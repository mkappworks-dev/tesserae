//! Which faces of a Chunk see each other through it: the 15 face-pair bits
//! Minecraft's `VisGraph` and Sodium compute per section, for a walk from the
//! camera's Chunk that skips what nothing in front of it lets the eye reach.

use crate::chunk::{CHUNK_SIZE, Chunk, ROWS};
use crate::material::{MaterialTable, Occlusion};
use crate::mesh::{FACE_STEPS, NEG_X, NEG_Y, NEG_Z, POS_X, POS_Y, POS_Z};
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
                let faces = region(open, &mut seen, &mut stack, i, 1 << seeds.trailing_zeros());
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

/// A box of Chunk coordinates, both corners inclusive, that a walk stays in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Bounds {
    pub min: [i32; 3],
    pub max: [i32; 3],
}

impl Bounds {
    /// The box `radius` Chunks each way from `center`, per axis.
    pub fn around(center: [i32; 3], radius: [i32; 3]) -> Self {
        Bounds {
            min: std::array::from_fn(|a| center[a] - radius[a]),
            max: std::array::from_fn(|a| center[a] + radius[a]),
        }
    }

    fn index(&self, c: [i32; 3]) -> Option<usize> {
        let mut at = 0;
        for a in (0..3).rev() {
            if !(self.min[a]..=self.max[a]).contains(&c[a]) {
                return None;
            }
            at = at * (self.max[a] - self.min[a] + 1) as usize + (c[a] - self.min[a]) as usize;
        }
        Some(at)
    }

    fn volume(&self) -> usize {
        (0..3)
            .map(|a| (self.max[a] - self.min[a] + 1).max(0) as usize)
            .product()
    }
}

/// The Chunks a walk entered.
pub struct Visited {
    bounds: Bounds,
    seen: Vec<u8>,
}

impl Visited {
    pub fn contains(&self, c: [i32; 3]) -> bool {
        self.bounds.index(c).is_some_and(|i| self.seen[i] != 0)
    }

    pub fn len(&self) -> usize {
        self.seen.iter().filter(|&&s| s != 0).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Adds what `other` entered, which walked the same `bounds`.
    pub fn extend(&mut self, other: &Visited) {
        assert_eq!(self.bounds, other.bounds, "walks of different boxes");
        for (a, b) in self.seen.iter_mut().zip(&other.seen) {
            *a |= b;
        }
    }
}

fn step(c: [i32; 3], face: usize) -> [i32; 3] {
    let d = FACE_STEPS[face];
    [c[0] + d[0], c[1] + d[1], c[2] + d[2]]
}

/// Every Chunk an eye at `start` could move to through open space, face to
/// face: a sealed cave is not in it. `of` answers for every Chunk in `bounds`,
/// [`Connectivity::ALL`] for one it does not know, so nothing unknown seals.
pub fn reach(start: [i32; 3], bounds: Bounds, of: impl Fn([i32; 3]) -> Connectivity) -> Visited {
    // A bit per face a Chunk has been entered through; bit 6 for the start.
    let mut seen = vec![0u8; bounds.volume()];
    let Some(i) = bounds.index(start) else {
        return Visited { bounds, seen };
    };
    seen[i] = 1 << 6;
    let mut queue = vec![(start, None::<usize>)];
    while let Some((at, entered)) = queue.pop() {
        let c = of(at);
        for exit in 0..6 {
            if entered.is_some_and(|f| !c.connects(f, exit)) {
                continue;
            }
            let next = step(at, exit);
            let entry = exit ^ 1;
            let Some(j) = bounds.index(next) else {
                continue;
            };
            if seen[j] & 1 << entry == 0 {
                seen[j] |= 1 << entry;
                queue.push((next, Some(entry)));
            }
        }
    }
    Visited { bounds, seen }
}

/// Minecraft's walk from the camera's Chunk: breadth first, each Chunk once,
/// leaving one only through a face its connectivity joins to the face it was
/// entered through, and never stepping back against a direction already
/// taken. What it never enters is hidden. Conservative only up to the one
/// entry per Chunk it keeps: a corner that sight bends around twice is culled,
/// as in Minecraft.
pub fn sight(start: [i32; 3], bounds: Bounds, of: impl Fn([i32; 3]) -> Connectivity) -> Visited {
    let mut seen = vec![0u8; bounds.volume()];
    let Some(i) = bounds.index(start) else {
        return Visited { bounds, seen };
    };
    seen[i] = 1;
    let mut queue = std::collections::VecDeque::from([(start, None::<usize>, 0u8)]);
    while let Some((at, entered, taken)) = queue.pop_front() {
        let c = of(at);
        for exit in 0..6 {
            if taken & 1 << (exit ^ 1) != 0 || entered.is_some_and(|f| !c.connects(f, exit)) {
                continue;
            }
            let next = step(at, exit);
            let Some(j) = bounds.index(next) else {
                continue;
            };
            if seen[j] != 0 {
                continue;
            }
            seen[j] = 1;
            queue.push_back((next, Some(exit ^ 1), taken | 1 << exit));
        }
    }
    Visited { bounds, seen }
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

    /// A world of Chunks: `open` everywhere except where `walls` says.
    fn world(walls: &[([i32; 3], Connectivity)]) -> impl Fn([i32; 3]) -> Connectivity + '_ {
        move |c| {
            walls
                .iter()
                .find(|(at, _)| *at == c)
                .map_or(Connectivity::ALL, |&(_, k)| k)
        }
    }

    fn tunnel(a: usize, b: usize) -> Connectivity {
        Connectivity(pair(a, b))
    }

    const R: Bounds = Bounds {
        min: [-4, -4, -4],
        max: [4, 4, 4],
    };

    #[test]
    fn open_space_reaches_and_sees_the_whole_box() {
        assert_eq!(reach([0; 3], R, world(&[])).len(), 9 * 9 * 9);
        assert_eq!(sight([0; 3], R, world(&[])).len(), 9 * 9 * 9);
    }

    #[test]
    fn a_sealed_cave_is_out_of_reach_and_its_shell_is_not() {
        // A cave at the origin, every Chunk around it solid rock.
        let mut walls = vec![([0, 0, 0], Connectivity::ALL)];
        for face in 0..6 {
            walls.push((step([0; 3], face), Connectivity::NONE));
        }
        let from_outside = reach([3, 0, 0], R, world(&walls));
        assert!(!from_outside.contains([0, 0, 0]));
        assert!(from_outside.contains([1, 0, 0]), "the rock it would mesh");
        let from_inside = reach([0, 0, 0], R, world(&walls));
        assert_eq!(from_inside.len(), 7);
    }

    #[test]
    fn reach_follows_a_tunnel_round_a_bend_that_sight_cannot() {
        // Up from the start, along x, and back down: a U the eye cannot see
        // round but could fly through.
        let mut walls = vec![([1, 0, 0], Connectivity::NONE)];
        for y in -4..=4 {
            for z in -4..=4 {
                for x in -4..=4 {
                    let c = [x, y, z];
                    if !matches!(c, [0..=2, 0..=1, 0]) {
                        walls.push((c, Connectivity::NONE));
                    }
                }
            }
        }
        walls.push(([0, 0, 0], tunnel(POS_Y, NEG_Y)));
        walls.push(([0, 1, 0], tunnel(NEG_Y, POS_X)));
        walls.push(([1, 1, 0], tunnel(NEG_X, POS_X)));
        walls.push(([2, 1, 0], tunnel(NEG_X, NEG_Y)));
        walls.push(([2, 0, 0], tunnel(POS_Y, NEG_X)));
        assert!(reach([0; 3], R, world(&walls)).contains([2, 0, 0]));
        let seen = sight([0; 3], R, world(&walls));
        assert!(seen.contains([2, 1, 0]));
        assert!(!seen.contains([2, 0, 0]), "seen only by looking back down");
    }

    #[test]
    fn sight_stops_at_the_first_wall_and_sees_the_wall() {
        let walls = [([2, 0, 0], Connectivity::NONE)];
        let seen = sight([0; 3], Bounds::around([0; 3], [4, 0, 0]), world(&walls));
        assert!(seen.contains([2, 0, 0]));
        assert!(!seen.contains([3, 0, 0]));
    }

    #[test]
    fn sight_leaves_a_chunk_only_where_its_entry_face_leads() {
        // Entered from -x, the tunnel leads on to +x and nowhere else.
        let mut walls = vec![([1, 0, 0], tunnel(NEG_X, POS_X))];
        let line = Bounds::around([0; 3], [3, 0, 0]);
        let seen = sight([0; 3], line, world(&walls));
        assert!(seen.contains([3, 0, 0]));
        // Rock beside the Chunk above the tunnel, so only the tunnel could lead there.
        walls.extend([
            ([0, 1, 0], Connectivity::NONE),
            ([2, 1, 0], Connectivity::NONE),
        ]);
        let plane = Bounds {
            min: [0, 0, 0],
            max: [3, 1, 0],
        };
        let seen = sight([0; 3], plane, world(&walls));
        assert!(!seen.contains([1, 1, 0]), "the tunnel does not lead up");
    }
}
