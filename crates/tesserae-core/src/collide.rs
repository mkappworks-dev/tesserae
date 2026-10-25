//! Geometric questions asked of the grid, in metres: what a body can move
//! through, and what a ray strikes first.
//!
//! "Sweep an AABB against the voxel grid directly" is DESIGN.md's answer to
//! terrain collision, and a physics engine is its rejected alternative. What a
//! Voxel contributes is its Shape's collision boxes, and nothing at all if its
//! Material is not solid: a body sinks through water and a ray aims past it.
//!
//! **Everything here is in metres**, and `voxel_size` is the only conversion to
//! a Voxel index. A half-height slab is 0.5 m at 1 m Voxels and 0.125 m at
//! 0.25 m ones, because a Shape is a fraction of a Voxel and the World decides
//! what a Voxel is worth. `docs/adr/0003-voxel-size-is-per-world.md`.
//!
//! Step height, gravity and what a body does with a blocked axis are a
//! controller's policy, not the grid's, and are not here.

use crate::mesh::{FACE_STEPS, NEG_X, POS_X};
use crate::shape::{CollisionBox, Connections, NodeBox, SUBDIV, ShapeTable};
use crate::tables::Tables;
use crate::voxel::{SHAPE_FULL_CUBE, Voxel};

/// Random access to a World's Voxels, in Voxel coordinates.
///
/// A trait so this crate never learns how Chunks are held and keeps its zero
/// dependencies; the Server implements it over its store. `None` is a Voxel the
/// volume does not hold, which a sweep treats as solid and a ray treats as the
/// end of what is known.
pub trait VoxelVolume {
    fn voxel(&self, at: [i32; 3]) -> Option<Voxel>;
}

/// An axis-aligned box in metres.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Aabb {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl Aabb {
    pub const fn new(min: [f32; 3], max: [f32; 3]) -> Self {
        Aabb { min, max }
    }
}

/// What a sweep did.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Sweep {
    /// How far the box actually moved on each axis; the requested delta,
    /// shortened where something was in the way.
    pub moved: [f32; 3],
    /// Which axes were stopped short. `blocked[1]` while moving down is "on the
    /// ground", and a blocked horizontal axis is a wall to slide along.
    pub blocked: [bool; 3],
    /// Where the box ended up.
    pub aabb: Aabb,
}

/// Tolerance for boxes that touch exactly: a body resting on a floor has its
/// underside on the floor's top, and must neither be stuck to it nor sink in.
/// Far below the 1/16 of the smallest Voxel anyone will use.
const EPS: f32 = 1e-5;

/// Moves `aabb` by `delta`, one axis at a time, y then x then z, stopping each
/// at the first collision box in the way.
///
/// y first so that landing is resolved before sliding; the axes are independent
/// otherwise, which is what lets a body slide along a wall. A body that already
/// overlaps a box is not pushed out of it - it can move out, and is not stopped
/// by what it is already inside.
pub fn sweep(
    vol: &impl VoxelVolume,
    tables: &Tables,
    voxel_size: f32,
    aabb: Aabb,
    delta: [f32; 3],
) -> Sweep {
    let mut aabb = aabb;
    let mut moved = [0.0; 3];
    let mut blocked = [false; 3];
    for axis in [1, 0, 2] {
        let d = delta[axis];
        if d == 0.0 {
            continue;
        }
        let allowed = clip(vol, tables, voxel_size, &aabb, axis, d);
        moved[axis] = allowed;
        blocked[axis] = allowed != d;
        aabb.min[axis] += allowed;
        aabb.max[axis] += allowed;
    }
    Sweep {
        moved,
        blocked,
        aabb,
    }
}

/// How far `aabb` can move along `axis` towards `d` before something stops it.
fn clip(
    vol: &impl VoxelVolume,
    tables: &Tables,
    voxel_size: f32,
    aabb: &Aabb,
    axis: usize,
    d: f32,
) -> f32 {
    let (o1, o2) = ((axis + 1) % 3, (axis + 2) % 3);
    let mut lo = aabb.min;
    let mut hi = aabb.max;
    if d > 0.0 {
        hi[axis] += d;
    } else {
        lo[axis] += d;
    }
    // One cell of slack at the top: a box ending exactly on a grid line reaches
    // the cell beyond it, which costs one box test and is filtered by it.
    //
    // And as many more as the table's Shapes overhang: a box belonging to the cell
    // below that climbs into ours is found from the cell below, so the range
    // reaches down by what boxes reach up. Zero, and the range unchanged, until a
    // Shape overhangs.
    let over = tables.shapes.overhang();
    let cells = |a: usize| {
        let below = over[2 * a + 1] as i32;
        let above = over[2 * a] as i32;
        ((lo[a] / voxel_size).floor() as i32 - below)
            ..=((hi[a] / voxel_size).floor() as i32 + above)
    };

    let mut allowed = d;
    for z in cells(2) {
        for y in cells(1) {
            for x in cells(0) {
                let cell = [x, y, z];
                for b in obstacles(vol, tables, cell) {
                    let (min, max) = in_metres(b, cell, voxel_size);
                    let overlaps =
                        |a: usize| aabb.min[a] < max[a] - EPS && aabb.max[a] > min[a] + EPS;
                    if !overlaps(o1) || !overlaps(o2) {
                        continue;
                    }
                    if d > 0.0 && aabb.max[axis] <= min[axis] + EPS {
                        allowed = allowed.min(min[axis] - aabb.max[axis]);
                    } else if d < 0.0 && aabb.min[axis] >= max[axis] - EPS {
                        allowed = allowed.max(max[axis] - aabb.min[axis]);
                    }
                }
            }
        }
    }
    allowed
}

/// The arms the Voxel at `cell` has out, from its six neighbours through the
/// volume. A Voxel the volume does not hold reads as air, so it never connects:
/// the mesher's shell says the same where a Chunk is missing. Neighbours are
/// read only for a connected Shape.
fn connections_at(
    vol: &impl VoxelVolume,
    shapes: &ShapeTable,
    cell: [i32; 3],
    voxel: Voxel,
) -> Connections {
    // Cubes and air are most cells; they never reach the table for this.
    if !shapes.any_connected() || voxel.shape() == SHAPE_FULL_CUBE || !shapes.is_connected(voxel) {
        return Connections::NONE;
    }
    let around = FACE_STEPS.map(|d| {
        vol.voxel([cell[0] + d[0], cell[1] + d[1], cell[2] + d[2]])
            .unwrap_or(Voxel::AIR)
    });
    shapes.connections(voxel, around)
}

const WHOLE_CELL: [CollisionBox; 1] = [CollisionBox::new([0; 3], [SUBDIV as i8; 3])];

/// The collision boxes of the Voxel at `cell`, or the whole cell if the volume
/// does not hold it. Borrowed from the table, so visiting a cell allocates
/// nothing.
fn obstacles<'a>(vol: &impl VoxelVolume, tables: &'a Tables, cell: [i32; 3]) -> &'a [CollisionBox] {
    match vol.voxel(cell) {
        Some(voxel) if !tables.materials.solid(voxel) => &[],
        Some(voxel) => tables
            .shapes
            .collision(voxel, connections_at(vol, &tables.shapes, cell, voxel)),
        None => &WHOLE_CELL,
    }
}

/// A box in sixteenths of the Voxel at `cell`, as metres.
fn in_metres(b: &CollisionBox, cell: [i32; 3], voxel_size: f32) -> ([f32; 3], [f32; 3]) {
    let unit = voxel_size / SUBDIV as f32;
    let at = |a: usize, sixteenths: i8| (cell[a] * SUBDIV as i32 + sixteenths as i32) as f32 * unit;
    (
        std::array::from_fn(|a| at(a, b.min[a])),
        std::array::from_fn(|a| at(a, b.max[a])),
    )
}

/// The first drawn box a ray strikes.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct RayHit {
    /// The Voxel that was struck.
    pub at: [i32; 3],
    /// The face of that Voxel the ray came through, as a face constant. The
    /// Voxel across it, `at` stepped one in that direction, is where a Voxel
    /// placed against the surface goes.
    pub face: usize,
    pub distance_m: f32,
}

/// Casts a ray from `origin` along `dir` (any length) for at most `reach_m`
/// metres and returns the first box of a Shape's drawn geometry it strikes: what a
/// player aims at, not what a body bumps into.
///
/// A cell walk, testing the ray against each cell's boxes, so it enters a
/// stair's notch and stops at the tread rather than at the cell. It stops at the
/// first Voxel the volume does not hold: past the edge of what is known there is
/// nothing to hit. A ray that starts inside a box strikes it at distance zero.
///
/// It goes through what a body would pass through: water is aimed past, at the
/// bed.
pub fn raycast(
    vol: &impl VoxelVolume,
    tables: &Tables,
    voxel_size: f32,
    origin: [f32; 3],
    dir: [f32; 3],
    reach_m: f32,
) -> Option<RayHit> {
    raycast_until(vol, tables, voxel_size, origin, dir, reach_m, |v| {
        tables.materials.solid(v)
    })
}

/// [`raycast`], stopping at the Voxels `stops` accepts rather than at solid
/// ones: a fluid tool aims at a source a body would sink through.
pub fn raycast_until(
    vol: &impl VoxelVolume,
    tables: &Tables,
    voxel_size: f32,
    origin: [f32; 3],
    dir: [f32; 3],
    reach_m: f32,
    stops: impl Fn(Voxel) -> bool,
) -> Option<RayHit> {
    let len = dir.iter().map(|c| c * c).sum::<f32>().sqrt();
    if len == 0.0 || !len.is_finite() {
        return None;
    }
    // Voxel units from here on, so a cell is one unit wide.
    let d = dir.map(|c| c / len);
    let o = origin.map(|c| c / voxel_size);
    let t_max = reach_m / voxel_size;

    let mut cell = o.map(|c| c.floor() as i32);
    let step: [i32; 3] = d.map(|c| {
        if c > 0.0 {
            1
        } else if c < 0.0 {
            -1
        } else {
            0
        }
    });
    let t_delta: [f32; 3] = d.map(|c| {
        if c == 0.0 {
            f32::INFINITY
        } else {
            (1.0 / c).abs()
        }
    });
    let mut t_next: [f32; 3] = std::array::from_fn(|a| match step[a] {
        1 => (cell[a] as f32 + 1.0 - o[a]) / d[a],
        -1 => (o[a] - cell[a] as f32) / -d[a],
        _ => f32::INFINITY,
    });

    loop {
        let voxel = vol.voxel(cell)?;
        let nearest = stops(voxel)
            .then(|| {
                tables
                    .shapes
                    .selection(voxel, connections_at(vol, &tables.shapes, cell, voxel))
                    .iter()
                    .filter_map(|b| ray_box(o, d, cell, b))
                    .filter(|&(t, _)| t <= t_max)
                    .min_by(|a, b| a.0.total_cmp(&b.0))
            })
            .flatten();
        if let Some((t, face)) = nearest {
            return Some(RayHit {
                at: cell,
                face,
                distance_m: t * voxel_size,
            });
        }

        let axis = (0..3)
            .min_by(|&a, &b| t_next[a].total_cmp(&t_next[b]))
            .expect("three axes");
        if t_next[axis] > t_max {
            return None;
        }
        cell[axis] += step[axis];
        t_next[axis] += t_delta[axis];
    }
}

/// The slab test of a ray against one box of the Voxel at `cell`, in Voxel
/// units: the distance to where it enters and the face it enters by.
fn ray_box(o: [f32; 3], d: [f32; 3], cell: [i32; 3], b: &NodeBox) -> Option<(f32, usize)> {
    let unit = 1.0 / SUBDIV as f32;
    let mut enter = f32::NEG_INFINITY;
    let mut leave = f32::INFINITY;
    let mut axis = 0;
    for a in 0..3 {
        let min = cell[a] as f32 + b.min[a] as f32 * unit;
        let max = cell[a] as f32 + b.max[a] as f32 * unit;
        if d[a] == 0.0 {
            if o[a] < min || o[a] > max {
                return None;
            }
            continue;
        }
        let (t1, t2) = ((min - o[a]) / d[a], (max - o[a]) / d[a]);
        let (near, far) = (t1.min(t2), t1.max(t2));
        if near > enter {
            enter = near;
            axis = a;
        }
        leave = leave.min(far);
    }
    if enter > leave || leave < 0.0 {
        return None;
    }
    if enter < 0.0 {
        // Started inside: struck at once, from the side the ray is heading away
        // from along its dominant axis.
        let a = (0..3)
            .max_by(|&a, &b| d[a].abs().total_cmp(&d[b].abs()))
            .expect("three axes");
        return Some((
            0.0,
            if d[a] > 0.0 {
                NEG_X + 2 * a
            } else {
                POS_X + 2 * a
            },
        ));
    }
    // The face struck is the one facing back at the ray.
    Some((
        enter,
        if d[axis] > 0.0 {
            NEG_X + 2 * axis
        } else {
            POS_X + 2 * axis
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::{NEG_Y, NEG_Z, POS_X, POS_Y};
    use crate::shape::examples;
    use crate::shape::{CollisionBox, Orientation, SHAPE_POST, SHAPE_SLAB, SHAPE_STAIR, ShapeDef};
    use crate::worldgen::STONE;
    use std::collections::HashMap;

    /// A volume of what a test places, loaded over `lo..=hi` and unknown outside.
    struct Grid {
        voxels: HashMap<[i32; 3], Voxel>,
        lo: [i32; 3],
        hi: [i32; 3],
    }

    impl Grid {
        fn new() -> Self {
            Grid {
                voxels: HashMap::new(),
                lo: [-20; 3],
                hi: [20; 3],
            }
        }

        fn put(&mut self, at: [i32; 3], voxel: Voxel) -> &mut Self {
            self.voxels.insert(at, voxel);
            self
        }

        /// A floor of `voxel` one Voxel thick at layer `y`.
        fn floor(&mut self, y: i32, voxel: Voxel) -> &mut Self {
            for x in -10..=10 {
                for z in -10..=10 {
                    self.put([x, y, z], voxel);
                }
            }
            self
        }
    }

    impl VoxelVolume for Grid {
        fn voxel(&self, at: [i32; 3]) -> Option<Voxel> {
            if (0..3).all(|a| at[a] >= self.lo[a] && at[a] <= self.hi[a]) {
                Some(*self.voxels.get(&at).unwrap_or(&Voxel::AIR))
            } else {
                None
            }
        }
    }

    fn table() -> ShapeTable {
        ShapeTable::builtin()
    }

    /// The tests below are about Shapes, so they pass a `ShapeTable` and get the
    /// builtin Materials with it.
    fn sweep(
        vol: &impl VoxelVolume,
        shapes: &ShapeTable,
        voxel_size: f32,
        aabb: Aabb,
        delta: [f32; 3],
    ) -> Sweep {
        super::sweep(vol, &with_materials(shapes), voxel_size, aabb, delta)
    }

    fn raycast(
        vol: &impl VoxelVolume,
        shapes: &ShapeTable,
        voxel_size: f32,
        origin: [f32; 3],
        dir: [f32; 3],
        reach_m: f32,
    ) -> Option<RayHit> {
        super::raycast(
            vol,
            &with_materials(shapes),
            voxel_size,
            origin,
            dir,
            reach_m,
        )
    }

    fn with_materials(shapes: &ShapeTable) -> Tables {
        Tables {
            shapes: shapes.clone(),
            ..Tables::builtin()
        }
    }

    /// Stone at layer 0 and a pool of water on it at layer 1.
    fn pool() -> Grid {
        let mut g = Grid::new();
        g.floor(0, cube())
            .floor(1, Voxel::cube(crate::material::WATER));
        g
    }

    #[test]
    fn a_body_sinks_through_water_to_the_bed() {
        let fall = super::sweep(
            &pool(),
            &Tables::builtin(),
            1.0,
            body(0.5, 3.0, 0.5, 1.8),
            [0.0, -5.0, 0.0],
        );
        assert!(
            close(fall.aabb.min[1], 1.0),
            "rests at {}",
            fall.aabb.min[1]
        );
    }

    #[test]
    fn a_ray_through_water_strikes_the_bed() {
        let hit = super::raycast(
            &pool(),
            &Tables::builtin(),
            1.0,
            [0.5, 4.0, 0.5],
            [0.0, -1.0, 0.0],
            10.0,
        )
        .expect("the bed is in reach");
        assert_eq!(hit.at, [0, 0, 0]);
        assert_eq!(hit.face, POS_Y);
    }

    #[test]
    fn a_material_the_table_does_not_know_is_still_solid() {
        let mut g = Grid::new();
        g.floor(0, Voxel::cube(999));
        let fall = super::sweep(
            &g,
            &Tables::builtin(),
            1.0,
            body(0.5, 3.0, 0.5, 1.8),
            [0.0, -5.0, 0.0],
        );
        assert!(
            close(fall.aabb.min[1], 1.0),
            "rests at {}",
            fall.aabb.min[1]
        );
    }

    fn cube() -> Voxel {
        Voxel::cube(STONE)
    }

    fn slab(up: usize) -> Voxel {
        Voxel::shaped(STONE, SHAPE_SLAB, Orientation::new(up, 0))
    }

    fn stair(turn: u8) -> Voxel {
        Voxel::shaped(STONE, SHAPE_STAIR, Orientation::new(POS_Y, turn))
    }

    /// A body 0.4 m square and `height` tall standing with its feet at `(x, y, z)`.
    fn body(x: f32, y: f32, z: f32, height: f32) -> Aabb {
        Aabb::new([x - 0.2, y, z - 0.2], [x + 0.2, y + height, z + 0.2])
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn a_body_falls_to_the_top_of_a_bottom_slab_not_to_the_top_of_its_cell() {
        let mut g = Grid::new();
        g.floor(0, slab(POS_Y));
        let fall = sweep(
            &g,
            &table(),
            1.0,
            body(0.5, 3.0, 0.5, 1.8),
            [0.0, -5.0, 0.0],
        );
        assert!(
            close(fall.aabb.min[1], 0.5),
            "rests at {}",
            fall.aabb.min[1]
        );
        assert!(fall.blocked[1]);
        assert!(close(fall.moved[1], -2.5));
    }

    #[test]
    fn a_body_standing_on_a_slab_is_on_the_ground_and_does_not_sink() {
        let mut g = Grid::new();
        g.floor(0, slab(POS_Y));
        let still = sweep(
            &g,
            &table(),
            1.0,
            body(0.5, 0.5, 0.5, 1.8),
            [0.0, -0.1, 0.0],
        );
        assert!(still.blocked[1]);
        assert!(close(still.aabb.min[1], 0.5));
    }

    #[test]
    fn a_top_slab_is_a_floor_at_the_top_of_its_cell_and_a_ceiling_half_way_up() {
        let mut g = Grid::new();
        g.floor(0, slab(NEG_Y));
        let fall = sweep(
            &g,
            &table(),
            1.0,
            body(0.5, 3.0, 0.5, 1.8),
            [0.0, -5.0, 0.0],
        );
        assert!(
            close(fall.aabb.min[1], 1.0),
            "rests at {}",
            fall.aabb.min[1]
        );

        // Under one, a body 1.4 m tall standing on a cube floor has 0.1 m before
        // the slab's underside at 2.5 m.
        let mut g = Grid::new();
        g.floor(0, cube());
        g.put([0, 2, 0], slab(NEG_Y));
        let up = sweep(&g, &table(), 1.0, body(0.5, 1.0, 0.5, 1.4), [0.0, 1.0, 0.0]);
        assert!(up.blocked[1]);
        assert!(close(up.moved[1], 0.1), "moved {}", up.moved[1]);
    }

    #[test]
    fn a_stair_stops_a_walker_at_the_riser_and_a_body_on_the_tread_at_the_step_above() {
        let mut g = Grid::new();
        g.floor(0, cube());
        g.put([5, 1, 5], stair(0));

        // Feet on the floor at y=1: the stair's lower half is in the way.
        let walk = sweep(&g, &table(), 1.0, body(5.5, 1.0, 3.0, 1.8), [0.0, 0.0, 3.0]);
        assert!(walk.blocked[2]);
        assert!(
            close(walk.aabb.max[2], 5.0),
            "stopped at {}",
            walk.aabb.max[2]
        );

        // Feet on the tread at y=1.5: free over the lower half, then the upper
        // half's face, half way back.
        let step = sweep(&g, &table(), 1.0, body(5.5, 1.5, 4.9, 1.8), [0.0, 0.0, 1.0]);
        assert!(step.blocked[2]);
        assert!(
            close(step.aabb.max[2], 5.5),
            "stopped at {}",
            step.aabb.max[2]
        );
    }

    #[test]
    fn a_body_slides_along_a_wall() {
        let mut g = Grid::new();
        g.floor(0, cube());
        for z in -10..=10 {
            g.put([6, 1, z], cube());
            g.put([6, 2, z], cube());
        }
        let slide = sweep(&g, &table(), 1.0, body(4.5, 1.0, 0.5, 1.8), [2.0, 0.0, 1.0]);
        assert_eq!(slide.blocked, [true, false, false]);
        assert!(close(slide.moved[2], 1.0), "kept sliding");
        assert!(close(slide.aabb.max[0], 6.0));
    }

    #[test]
    fn a_thin_shape_is_passed_beside_and_stops_a_body_that_overlaps_it() {
        let mut g = Grid::new();
        g.floor(0, cube());
        g.put(
            [5, 1, 5],
            Voxel::shaped(STONE, SHAPE_POST, Orientation::IDENTITY),
        );
        // The post is x in [5.375, 5.625], z in [5.375, 5.625].
        let beside = sweep(
            &g,
            &table(),
            1.0,
            Aabb::new([5.0, 1.0, 3.0], [5.3, 2.8, 3.4]),
            [0.0, 0.0, 4.0],
        );
        assert!(!beside.blocked[2], "a body that does not overlap it passes");
        let into = sweep(
            &g,
            &table(),
            1.0,
            Aabb::new([5.45, 1.0, 3.0], [5.75, 2.8, 3.4]),
            [0.0, 0.0, 4.0],
        );
        assert!(into.blocked[2]);
        assert!(
            close(into.aabb.max[2], 5.375),
            "stopped at {}",
            into.aabb.max[2]
        );
    }

    #[test]
    fn a_shape_with_no_collision_is_walked_through() {
        let mut t = table();
        let plate = t
            .register(ShapeDef {
                connect: None,
                boxes: vec![NodeBox::new([1, 0, 1], [15, 1, 15])],
                collision: Some(vec![]),
            })
            .unwrap();
        let mut g = Grid::new();
        g.floor(0, cube());
        g.put([5, 1, 5], Voxel::new(STONE, plate, 0));
        let walk = sweep(&g, &t, 1.0, body(5.5, 1.0, 3.0, 1.8), [0.0, 0.0, 4.0]);
        assert!(!walk.blocked[2]);
    }

    #[test]
    fn a_ray_strikes_a_shape_that_is_walked_through() {
        let mut t = table();
        let plate = t
            .register(ShapeDef {
                connect: None,
                boxes: vec![NodeBox::new([1, 0, 1], [15, 1, 15])],
                collision: Some(vec![]),
            })
            .unwrap();
        let mut g = Grid::new();
        g.put([5, 1, 5], Voxel::new(STONE, plate, 0));
        let hit = raycast(&g, &t, 1.0, [5.5, 4.0, 5.5], [0.0, -1.0, 0.0], 10.0).unwrap();
        assert_eq!((hit.at, hit.face), ([5, 1, 5], POS_Y));
        assert!(
            close(hit.distance_m, 3.0 - 1.0 / 16.0),
            "{}",
            hit.distance_m
        );
    }

    #[test]
    fn a_ray_is_stopped_by_what_is_drawn_and_not_by_a_larger_collision_box() {
        let mut t = table();
        // A thin sign drawn on the cell's back wall, with a whole cell to bump into.
        let sign = t
            .register(ShapeDef {
                connect: None,
                boxes: vec![NodeBox::new([0, 0, 14], [16, 16, 16])],
                collision: Some(vec![NodeBox::new([0; 3], [16; 3]).into()]),
            })
            .unwrap();
        let mut g = Grid::new();
        g.put([5, 1, 5], Voxel::new(STONE, sign, 0));
        // The ray crosses the empty front of the cell and strikes the sign itself.
        let ray = raycast(&g, &t, 1.0, [5.5, 1.5, 3.0], [0.0, 0.0, 1.0], 10.0).unwrap();
        assert!(
            close(ray.distance_m, 2.0 + 14.0 / 16.0),
            "{}",
            ray.distance_m
        );
        // A body, by contrast, is stopped at the front of the cell.
        let walk = sweep(&g, &t, 1.0, body(5.5, 1.0, 3.0, 1.8), [0.0, 0.0, 4.0]);
        assert!(close(walk.aabb.max[2], 5.0), "{}", walk.aabb.max[2]);
    }

    /// A post a Voxel and a half tall to bump into, drawn one Voxel tall.
    fn fence_post(t: &mut ShapeTable) -> Voxel {
        let id = t
            .register(ShapeDef {
                connect: None,
                boxes: vec![NodeBox::new([6, 0, 6], [10, 16, 10])],
                collision: Some(vec![CollisionBox::new([6, 0, 6], [10, 24, 10])]),
            })
            .unwrap();
        Voxel::new(STONE, id, 0)
    }

    #[test]
    fn a_body_lands_on_the_overhang_of_the_voxel_below_it() {
        let mut t = table();
        let fence = fence_post(&mut t);
        for voxel_size in [1.0, 0.25] {
            let mut g = Grid::new();
            g.put([5, 1, 5], fence);
            // Feet in the cell above the fence: the fence is in the cell below,
            // and only its overhang reaches this one.
            let s = voxel_size;
            let fall = sweep(
                &g,
                &t,
                s,
                Aabb::new(
                    [5.5 * s - 0.2 * s, 2.6 * s, 5.5 * s - 0.2 * s],
                    [5.5 * s + 0.2 * s, 4.4 * s, 5.5 * s + 0.2 * s],
                ),
                [0.0, -0.3 * s, 0.0],
            );
            assert!(fall.blocked[1], "at {voxel_size}");
            assert!(
                close(fall.aabb.min[1], 2.5 * s),
                "rests at {} at {voxel_size}",
                fall.aabb.min[1]
            );
        }
    }

    #[test]
    fn a_fence_cannot_be_walked_into_at_the_height_of_its_overhang_and_can_above_it() {
        let mut t = table();
        let fence = fence_post(&mut t);
        let mut g = Grid::new();
        g.put([5, 1, 5], fence);
        // Feet at 2.0 m, in the cell above the fence, whose collision is 2.5 m.
        let low = sweep(&g, &t, 1.0, body(2.5, 2.0, 5.5, 1.8), [6.0, 0.0, 0.0]);
        assert!(low.blocked[0]);
        assert!(close(low.aabb.max[0], 5.375), "{}", low.aabb.max[0]);
        let high = sweep(&g, &t, 1.0, body(2.5, 2.5, 5.5, 1.8), [6.0, 0.0, 0.0]);
        assert!(!high.blocked[0]);
    }

    #[test]
    fn a_box_reaching_down_into_the_cell_above_is_found_from_it() {
        let mut t = table();
        let id = t
            .register(ShapeDef {
                connect: None,
                boxes: vec![NodeBox::new([6, 0, 6], [10, 16, 10])],
                collision: Some(vec![CollisionBox::new([6, -8, 6], [10, 16, 10])]),
            })
            .unwrap();
        let mut g = Grid::new();
        g.put([5, 3, 5], Voxel::new(STONE, id, 0));
        // Head at 1.8 m, rising: the box's underside is half a Voxel below its
        // cell, at 2.5 m, in a cell the head has not reached by its own cell.
        let rise = sweep(&g, &t, 1.0, body(5.5, 0.0, 5.5, 1.8), [0.0, 1.0, 0.0]);
        assert!(rise.blocked[1]);
        assert!(close(rise.moved[1], 0.7), "{}", rise.moved[1]);
    }

    #[test]
    fn an_overhang_rotates_with_its_shape() {
        let mut t = table();
        let fence = fence_post(&mut t);
        let on_its_side = Voxel::new(STONE, fence.shape(), Orientation::new(POS_X, 0).bits());
        let reach = t.collision(on_its_side, Connections::NONE)[0];
        assert_eq!((reach.min[0], reach.max[0]), (0, 24));
        assert_eq!((reach.min[1], reach.max[1]), (6, 10));
    }

    #[test]
    fn the_builtin_table_overhangs_nothing_so_a_sweep_visits_the_cells_it_always_did() {
        assert_eq!(table().overhang(), [0; 6]);
        let mut t = table();
        fence_post(&mut t);
        // Registered at all 24 Orientations, so one tall Shape widens every face.
        assert_eq!(t.overhang(), [1; 6]);
    }

    fn fences() -> (ShapeTable, Voxel, Voxel) {
        let mut t = table();
        let fence = Voxel::new(STONE, t.register(examples::fence()).unwrap(), 0);
        let pipe = Voxel::new(STONE, t.register(examples::pipe()).unwrap(), 0);
        (t, fence, pipe)
    }

    /// A body 0.4 Voxels square and 1.8 tall, in metres at `s` per Voxel.
    fn body_at(s: f32, x: f32, y: f32, z: f32) -> Aabb {
        Aabb::new(
            [(x - 0.2) * s, y * s, (z - 0.2) * s],
            [(x + 0.2) * s, (y + 1.8) * s, (z + 0.2) * s],
        )
    }

    #[test]
    fn a_body_is_stopped_by_an_arm_between_two_fences_and_not_between_unrelated_posts() {
        let (t, fence, pipe) = fences();
        for s in [1.0, 0.25] {
            // Crossing the line at x = 6.0, in the gap between the two posts.
            let cross = |g: &Grid| sweep(g, &t, s, body_at(s, 6.0, 1.0, 3.0), [0.0, 0.0, 4.0 * s]);

            let mut joined = Grid::new();
            joined.put([5, 1, 5], fence).put([6, 1, 5], fence);
            let stopped = cross(&joined);
            assert!(stopped.blocked[2], "at {s}");
            assert!(
                close(stopped.aabb.max[2] / s, 5.0 + 7.0 / 16.0),
                "{} at {s}",
                stopped.aabb.max[2] / s
            );

            // A pipe is another group, so no arm grows and the gap is open.
            let mut unrelated = Grid::new();
            unrelated.put([5, 1, 5], fence).put([6, 1, 5], pipe);
            assert!(!cross(&unrelated).blocked[2], "at {s}");
        }
    }

    #[test]
    fn a_ray_strikes_the_arm_and_passes_below_and_above_its_bar() {
        let (t, fence, _) = fences();
        for s in [1.0, 0.25] {
            let mut g = Grid::new();
            g.put([5, 1, 5], fence).put([6, 1, 5], fence);
            let along = |y: f32| {
                raycast(
                    &g,
                    &t,
                    s,
                    [6.0 * s, y * s, 3.0 * s],
                    [0.0, 0.0, 1.0],
                    10.0 * s,
                )
            };

            // The bar is 7/16 to 13/16 up, 7/16 across the arm's width.
            let bar = along(1.0 + 10.0 / 16.0).expect("a bar");
            assert_eq!((bar.at, bar.face), ([6, 1, 5], NEG_Z));
            assert!(
                close(bar.distance_m / s, 2.0 + 7.0 / 16.0),
                "{}",
                bar.distance_m
            );
            // Under and over the bar, and above the drawn fence in the part only a
            // body feels: nothing to pick.
            assert!(along(1.0 + 3.0 / 16.0).is_none(), "under the bar at {s}");
            assert!(along(1.0 + 14.0 / 16.0).is_none(), "over the bar at {s}");
            assert!(along(2.2).is_none(), "above the fence at {s}");
            // The same height does stop a body.
            let walk = sweep(&g, &t, s, body_at(s, 6.0, 2.0, 3.0), [0.0, 0.0, 4.0 * s]);
            assert!(walk.blocked[2], "at {s}");
        }
    }

    #[test]
    fn a_fence_beside_what_the_volume_does_not_hold_does_not_connect_to_it() {
        let (t, fence, _) = fences();
        let mut g = Grid::new();
        g.hi = [6, 20, 20];
        g.put([6, 1, 5], fence).put([5, 1, 5], fence);
        // The cell at x = 7 is unknown: solid to a sweep, absent to a connection.
        assert_eq!(
            connections_at(&g, &t, [6, 1, 5], fence),
            Connections::NONE.with(NEG_X)
        );
        let into = sweep(&g, &t, 1.0, body(5.0, 5.0, 3.0, 1.8), [3.0, 0.0, 0.0]);
        assert!(into.blocked[0], "the unknown region stops it");
    }

    #[test]
    fn what_a_body_bumps_into_what_a_ray_picks_and_what_is_drawn_have_the_same_arms() {
        let (t, fence, pipe) = fences();
        let mut g = Grid::new();
        let mut rng = 0x5eed_0006u64;
        let mut next = move || {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            rng
        };
        for z in 4..8 {
            for y in 4..8 {
                for x in 4..8 {
                    let v = match next() % 5 {
                        0 | 1 => fence,
                        2 => pipe,
                        3 => cube(),
                        _ => continue,
                    };
                    g.put([x, y, z], v);
                }
            }
        }
        for z in 4..8 {
            for y in 4..8 {
                for x in 4..8 {
                    let v = g.voxel([x, y, z]).unwrap();
                    if !t.is_connected(v) {
                        continue;
                    }
                    let c = connections_at(&g, &t, [x, y, z], v);
                    let around =
                        FACE_STEPS.map(|d| g.voxel([x + d[0], y + d[1], z + d[2]]).unwrap());
                    assert_eq!(c, t.connections(v, around), "at {:?}", [x, y, z]);
                    // Every arm that is out is in both lists, and no other is.
                    let arms = c.bits().count_ones() as usize;
                    assert_eq!(t.selection(v, c).len(), 1 + arms);
                    assert_eq!(t.collision(v, c).len(), 1 + arms);
                }
            }
        }
    }

    #[test]
    fn what_the_volume_does_not_hold_is_solid() {
        let mut g = Grid::new();
        g.hi = [5, 5, 5];
        let out = sweep(&g, &table(), 1.0, body(4.5, 2.0, 2.0, 1.0), [4.0, 0.0, 0.0]);
        assert!(out.blocked[0]);
        assert!(
            close(out.aabb.max[0], 6.0),
            "stopped at the edge: {}",
            out.aabb.max[0]
        );
    }

    /// ADR-0003: a half-height slab is 0.5 of a Voxel, whatever a Voxel is worth,
    /// and a body is the size it is in metres.
    #[test]
    fn the_same_scene_gives_the_same_answer_in_metres_at_any_voxel_size() {
        for voxel_size in [1.0f32, 0.5, 0.25] {
            let mut g = Grid::new();
            g.floor(0, slab(POS_Y));
            let fall = sweep(
                &g,
                &table(),
                voxel_size,
                body(0.1, 2.0, 0.1, 1.8),
                [0.0, -5.0, 0.0],
            );
            assert!(
                close(fall.aabb.min[1], 0.5 * voxel_size),
                "rests at {} m at {voxel_size} m Voxels",
                fall.aabb.min[1]
            );
        }
        // A wall one Voxel thick, 3 m away: the body stops 3 m away, not 3 Voxels.
        for voxel_size in [1.0f32, 0.5, 0.25] {
            let mut g = Grid::new();
            let wall = (3.0 / voxel_size) as i32;
            for y in -4..=20 {
                for z in -20..=20 {
                    g.put([wall, y, z], cube());
                }
            }
            let walk = sweep(
                &g,
                &table(),
                voxel_size,
                body(0.0, 0.0, 0.0, 1.0),
                [5.0, 0.0, 0.0],
            );
            assert!(
                close(walk.aabb.max[0], 3.0),
                "{} at {voxel_size}",
                walk.aabb.max[0]
            );
        }
    }

    #[test]
    fn a_ray_enters_a_stairs_notch_and_a_cube_stops_it_at_the_cell() {
        let mut g = Grid::new();
        g.put([5, 1, 5], stair(0));
        // Above the tread: y = 1.75 is local 0.75, in the notch. It strikes the
        // upper half's face half way through the cell.
        let notch = raycast(&g, &table(), 1.0, [5.5, 1.75, 3.0], [0.0, 0.0, 1.0], 10.0).unwrap();
        assert_eq!((notch.at, notch.face), ([5, 1, 5], NEG_Z));
        assert!(close(notch.distance_m, 2.5), "{}", notch.distance_m);

        // Level with the lower half: struck at the front of the cell.
        let low = raycast(&g, &table(), 1.0, [5.5, 1.25, 3.0], [0.0, 0.0, 1.0], 10.0).unwrap();
        assert!(close(low.distance_m, 2.0));

        g.put([5, 1, 5], cube());
        let whole = raycast(&g, &table(), 1.0, [5.5, 1.75, 3.0], [0.0, 0.0, 1.0], 10.0).unwrap();
        assert!(close(whole.distance_m, 2.0));
    }

    #[test]
    fn a_ray_reports_the_face_it_came_through_and_honours_its_reach() {
        let mut g = Grid::new();
        g.put([2, 2, 2], cube());

        let down = raycast(&g, &table(), 1.0, [2.5, 6.0, 2.5], [0.0, -1.0, 0.0], 10.0).unwrap();
        assert_eq!((down.at, down.face), ([2, 2, 2], POS_Y));
        assert!(close(down.distance_m, 3.0));

        let along = raycast(&g, &table(), 1.0, [-2.0, 2.5, 2.5], [1.0, 0.0, 0.0], 10.0).unwrap();
        assert_eq!(along.face, NEG_X);
        let back = raycast(&g, &table(), 1.0, [8.0, 2.5, 2.5], [-3.0, 0.0, 0.0], 10.0).unwrap();
        assert_eq!(back.face, POS_X, "any direction length works");

        assert!(raycast(&g, &table(), 1.0, [2.5, 6.0, 2.5], [0.0, -1.0, 0.0], 2.9).is_none());
        assert!(raycast(&g, &table(), 1.0, [2.5, 6.0, 2.5], [0.0, -1.0, 0.0], 3.1).is_some());
        assert!(raycast(&g, &table(), 1.0, [2.5, 6.0, 2.5], [0.0, 0.0, 0.0], 3.1).is_none());
    }

    #[test]
    fn a_ray_starting_inside_a_box_strikes_it_at_once_and_one_into_the_unknown_ends() {
        let mut g = Grid::new();
        g.put([0, 0, 0], cube());
        let inside = raycast(&g, &table(), 1.0, [0.5, 0.5, 0.5], [0.0, 0.0, 1.0], 5.0).unwrap();
        assert_eq!((inside.at, inside.distance_m), ([0, 0, 0], 0.0));
        assert_eq!(inside.face, NEG_Z);

        g.hi = [3, 3, 3];
        assert!(raycast(&g, &table(), 1.0, [1.5, 1.5, 1.5], [0.0, 0.0, 1.0], 50.0).is_none());
    }

    #[test]
    fn a_ray_is_measured_in_metres_whatever_a_voxel_is_worth() {
        for voxel_size in [1.0f32, 0.5, 0.25] {
            let mut g = Grid::new();
            let wall = (3.0 / voxel_size) as i32;
            g.put([wall, 0, 0], cube());
            let hit = raycast(
                &g,
                &table(),
                voxel_size,
                [0.0, 0.05, 0.05],
                [1.0, 0.0, 0.0],
                8.0,
            )
            .unwrap();
            assert!(
                close(hit.distance_m, 3.0),
                "{} at {voxel_size}",
                hit.distance_m
            );
            assert!(
                raycast(
                    &g,
                    &table(),
                    voxel_size,
                    [0.0, 0.05, 0.05],
                    [1.0, 0.0, 0.0],
                    2.9
                )
                .is_none()
            );
        }
    }

    #[test]
    fn a_ray_between_two_cells_on_a_grid_line_still_finds_the_one_it_enters() {
        let mut g = Grid::new();
        g.put([1, 0, 0], cube());
        let hit = raycast(&g, &table(), 1.0, [0.0, 0.0, 0.0], [1.0, 0.0, 0.0], 5.0).unwrap();
        assert_eq!(hit.at, [1, 0, 0]);
    }
}
