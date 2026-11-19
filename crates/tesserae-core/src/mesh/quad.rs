//! One record per quad: what the mesher emits and the renderer expands.
//!
//! The two geometry words are the GPU record's, as `chunk_vertex.wesl` decodes
//! them; see `docs/adr/0030-a-chunk-mesh-is-one-record-per-quad.md`. Positions
//! are in sixteenths of a Voxel, the grid every Shape, fluid surface and flame
//! lies on.

use crate::light::Light;
use crate::shape::SUBDIV;

/// The corner each of a quad's six vertices is, as two triangles.
pub const QUAD_CORNERS: [usize; 6] = [0, 1, 2, 0, 2, 3];

use layout::*;

/// The record's bit layout: `chunk_vertex.wesl` reads it through shader defs,
/// and the Client's test-only `expand` mirrors the shader with it.
pub mod layout {
    /// The kind of a Cross's quad: one diagonal of its cell, front or back.
    pub const KIND_CROSS: u32 = 6;
    /// The kind of a Plane form's quad: across the thin axis of its box.
    pub const KIND_PLANE: u32 = 7;
    /// Kinds below this are faces, in `FACE_STEPS` order.
    pub const FACES: u32 = 6;
    pub const KIND_BITS: u32 = 3;

    /// A face's plane is stored this many sixteenths up, so a flame or coarse
    /// water just below the Chunk still fits.
    pub const PLANE_BIAS: i32 = 16;
    pub const PLANE_SHIFT: u32 = 3;
    pub const PLANE_BITS: u32 = 10;
    pub const CORNER_BITS: u32 = 9;
    pub const U0_SHIFT: u32 = 13;
    pub const V0_SHIFT: u32 = 22;
    /// In the high word: each axis's extent less one.
    pub const DV_SHIFT: u32 = 9;

    pub const CELL_SHIFT: u32 = 3;
    /// A Cross's or Plane form's cell, this many bits an axis.
    pub const CELL_BITS: u32 = 5;
    pub const DIAGONAL_SHIFT: u32 = 18;
    pub const AXIS_SHIFT: u32 = 18;
    pub const AXIS_BITS: u32 = 2;
    pub const REVERSED_SHIFT: u32 = 20;
    pub const OFFSET_SHIFT: u32 = 21;
    pub const OFFSET_BITS: u32 = 5;
    /// A Plane form's high word is four fields this wide: its low corner on
    /// `u`, its high corner less one, then the same on `v`.
    pub const SPAN_BITS: u32 = 4;

    /// The high word's bits below this are geometry; the Client puts its table
    /// index above them.
    pub const PAINT_SHIFT: u32 = 18;
    /// How many (Material, light) pairs a layer's table can index.
    pub const PAINT_SLOTS: usize = 1 << (32 - PAINT_SHIFT);
    pub const GEOMETRY_MASK: u32 = mask(PAINT_SHIFT);

    pub const fn mask(bits: u32) -> u32 {
        (1 << bits) - 1
    }
}

const UNIT: f32 = 1.0 / SUBDIV as f32;
const SUB: u32 = SUBDIV as u32;

/// What a record's kind bits name.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum QuadKind {
    /// An axis-aligned rectangle, facing `FACE_STEPS[face]`.
    Face(usize),
    Cross,
    Plane,
}

/// One vertex of a quad, in Voxel-index space.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Corner {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

/// One quad of a Chunk's mesh.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Quad {
    /// The GPU record's two words, its high word's top bits left zero for the
    /// Client's table index.
    pub geometry: [u32; 2],
    /// A Material, or [`super::FLAME`].
    pub material: u32,
    /// The light the quad's face looks out at.
    pub light: Light,
}

impl Quad {
    /// An axis-aligned rectangle facing `face`, at `plane` and spanning `u` and
    /// `v` along the face's other two axes in their cyclic order, all in
    /// sixteenths. Corners and UVs run as pass 1 always wrote them: the UV is
    /// the position inside the Voxel the quad starts in, so a tiling sampler
    /// repeats per Voxel and a half-height slab side shows half the texture.
    pub fn face(
        face: usize,
        plane: i32,
        u: [u16; 2],
        v: [u16; 2],
        material: u32,
        light: Light,
    ) -> Quad {
        let biased = plane + PLANE_BIAS;
        debug_assert!(face < FACES as usize);
        debug_assert!((0..1 << PLANE_BITS).contains(&biased), "plane {plane}");
        debug_assert!(u[0] < u[1] && u[1] <= 1 << CORNER_BITS, "u {u:?}");
        debug_assert!(v[0] < v[1] && v[1] <= 1 << CORNER_BITS, "v {v:?}");
        let (u0, v0) = (u32::from(u[0]), u32::from(v[0]));
        Quad {
            geometry: [
                face as u32 | (biased as u32) << PLANE_SHIFT | u0 << U0_SHIFT | v0 << V0_SHIFT,
                (u32::from(u[1]) - u0 - 1) | (u32::from(v[1]) - v0 - 1) << DV_SHIFT,
            ],
            material,
            light,
        }
    }

    /// One side of one diagonal of a Cross in `cell`: `diagonal` 0 runs from
    /// the cell's low corner to its high x and z, 1 the other way.
    pub fn cross(
        cell: [i32; 3],
        diagonal: u32,
        reversed: bool,
        material: u32,
        light: Light,
    ) -> Quad {
        debug_assert!(diagonal < 2);
        Quad {
            geometry: [
                KIND_CROSS
                    | cell_bits(cell)
                    | diagonal << DIAGONAL_SHIFT
                    | u32::from(reversed) << REVERSED_SHIFT,
                0,
            ],
            material,
            light,
        }
    }

    /// One side of a Plane form's quad in `cell`, across `axis` at `offset`
    /// sixteenths into the cell and spanning `[u, v]` within it.
    pub fn plane_form(
        cell: [i32; 3],
        axis: usize,
        reversed: bool,
        offset: u32,
        [u, v]: [[u32; 2]; 2],
        material: u32,
        light: Light,
    ) -> Quad {
        debug_assert!(axis < 3 && offset <= SUB);
        debug_assert!(u[0] < u[1] && u[1] <= SUB && v[0] < v[1] && v[1] <= SUB);
        Quad {
            geometry: [
                KIND_PLANE
                    | cell_bits(cell)
                    | (axis as u32) << AXIS_SHIFT
                    | u32::from(reversed) << REVERSED_SHIFT
                    | offset << OFFSET_SHIFT,
                u[0] | (u[1] - 1) << SPAN_BITS
                    | v[0] << (2 * SPAN_BITS)
                    | (v[1] - 1) << (3 * SPAN_BITS),
            ],
            material,
            light,
        }
    }

    pub fn kind(&self) -> QuadKind {
        match self.geometry[0] & mask(KIND_BITS) {
            KIND_CROSS => QuadKind::Cross,
            KIND_PLANE => QuadKind::Plane,
            face => QuadKind::Face(face as usize),
        }
    }

    /// A face's plane in sixteenths, or `None` for a flat form.
    pub fn plane(&self) -> Option<i32> {
        matches!(self.kind(), QuadKind::Face(_))
            .then(|| (self.geometry[0] >> PLANE_SHIFT & mask(PLANE_BITS)) as i32 - PLANE_BIAS)
    }

    /// A face's extents along its other two axes, in sixteenths.
    pub fn spans(&self) -> Option<([u16; 2], [u16; 2])> {
        matches!(self.kind(), QuadKind::Face(_)).then(|| {
            let [lo, hi] = self.geometry;
            let u0 = lo >> U0_SHIFT & mask(CORNER_BITS);
            let v0 = lo >> V0_SHIFT & mask(CORNER_BITS);
            let u1 = u0 + (hi & mask(CORNER_BITS)) + 1;
            let v1 = v0 + (hi >> DV_SHIFT & mask(CORNER_BITS)) + 1;
            ([u0 as u16, u1 as u16], [v0 as u16, v1 as u16])
        })
    }

    /// The same face moved to `plane` sixteenths.
    pub fn at_plane(self, plane: i32) -> Quad {
        let QuadKind::Face(face) = self.kind() else {
            panic!("only a face has a plane");
        };
        let (u, v) = self.spans().expect("a face");
        Quad::face(face, plane, u, v, self.material, self.light)
    }

    /// The four corners in drawing order, as the vertex shader expands them.
    pub fn corners(&self) -> [Corner; 4] {
        match self.kind() {
            QuadKind::Face(face) => self.face_corners(face),
            QuadKind::Cross => self.cross_corners(),
            QuadKind::Plane => self.plane_corners(),
        }
    }

    /// The least and greatest corner on each axis, without expanding a face:
    /// the Client bounds every quad it receives.
    pub fn bounds(&self) -> ([f32; 3], [f32; 3]) {
        let QuadKind::Face(face) = self.kind() else {
            let corners = self.corners().map(|c| c.position);
            let fold = |f: fn(f32, f32) -> f32, from: f32| {
                std::array::from_fn(|a| corners.iter().map(|c| c[a]).fold(from, f))
            };
            return (fold(f32::min, f32::MAX), fold(f32::max, f32::MIN));
        };
        let (u, v) = ((face / 2 + 1) % 3, (face / 2 + 2) % 3);
        let plane = self.plane().expect("a face") as f32 * UNIT;
        let ([u0, u1], [v0, v1]) = self.spans().expect("a face");
        let (mut least, mut most) = ([plane; 3], [plane; 3]);
        (least[u], most[u]) = (f32::from(u0) * UNIT, f32::from(u1) * UNIT);
        (least[v], most[v]) = (f32::from(v0) * UNIT, f32::from(v1) * UNIT);
        (least, most)
    }

    fn face_corners(&self, face: usize) -> [Corner; 4] {
        let positive = face % 2 == 1;
        let (d, u, v) = (face / 2, (face / 2 + 1) % 3, (face / 2 + 2) % 3);
        let plane = self.plane().expect("a face") as f32 * UNIT;
        let ([u0, u1], [v0, v1]) = self.spans().expect("a face");
        let sub = SUBDIV as u16;
        let (ubase, vbase) = (u0 / sub * sub, v0 / sub * sub);
        let at = if positive {
            [(u0, v0), (u1, v0), (u1, v1), (u0, v1)]
        } else {
            [(u0, v0), (u0, v1), (u1, v1), (u1, v0)]
        };
        let mut normal = [0.0f32; 3];
        normal[d] = if positive { 1.0 } else { -1.0 };
        at.map(|(cu, cv)| {
            let mut position = [0.0f32; 3];
            position[d] = plane;
            position[u] = f32::from(cu) * UNIT;
            position[v] = f32::from(cv) * UNIT;
            Corner {
                position,
                normal,
                uv: [f32::from(cu - ubase) * UNIT, f32::from(cv - vbase) * UNIT],
            }
        })
    }

    /// Both forms run their four corners the other way round for the back.
    fn flat_order(&self) -> [usize; 4] {
        if self.reversed() {
            [3, 2, 1, 0]
        } else {
            [0, 1, 2, 3]
        }
    }

    /// The renderer reads a Voxel's face from the normal's axis, so it is
    /// axis-aligned: Z picks the Material's side tile, upright, for diagonals
    /// with no axis of their own.
    fn flat_normal(&self, axis: usize) -> [f32; 3] {
        let mut normal = [0.0; 3];
        normal[axis] = if self.reversed() { -1.0 } else { 1.0 };
        normal
    }

    /// A flat form's back.
    fn reversed(&self) -> bool {
        self.geometry[0] >> REVERSED_SHIFT & 1 == 1
    }

    fn cross_corners(&self) -> [Corner; 4] {
        let [x, y, z] = cell_of(self.geometry[0]);
        let across = if self.geometry[0] >> DIAGONAL_SHIFT & 1 == 0 {
            [0, 1, 1, 0]
        } else {
            [1, 0, 0, 1]
        };
        let normal = self.flat_normal(2);
        self.flat_order().map(|i| Corner {
            position: [
                (x + across[i]) as f32,
                (y + [0, 0, 1, 1][i]) as f32,
                (z + [0, 1, 1, 0][i]) as f32,
            ],
            normal,
            uv: UNIT_SQUARE[i],
        })
    }

    fn plane_corners(&self) -> [Corner; 4] {
        let [lo, hi] = self.geometry;
        let cell = cell_of(lo);
        let d = (lo >> AXIS_SHIFT & mask(AXIS_BITS)) as usize;
        let (u, v) = ((d + 1) % 3, (d + 2) % 3);
        let offset = lo >> OFFSET_SHIFT & mask(OFFSET_BITS);
        let span = |k: u32| hi >> (k * SPAN_BITS) & mask(SPAN_BITS);
        let (u0, u1) = (span(0), span(1) + 1);
        let (v0, v1) = (span(2), span(3) + 1);
        let at = [(u0, v0), (u1, v0), (u1, v1), (u0, v1)];
        let normal = self.flat_normal(d);
        let sixteenths = |axis: usize, s: u32| (cell[axis] * SUB as i32 + s as i32) as f32 * UNIT;
        self.flat_order().map(|i| {
            let mut position = [0.0f32; 3];
            position[d] = sixteenths(d, offset);
            position[u] = sixteenths(u, at[i].0);
            position[v] = sixteenths(v, at[i].1);
            Corner {
                position,
                normal,
                uv: UNIT_SQUARE[i],
            }
        })
    }
}

/// A flat form's UVs by corner, before the back's reversal.
const UNIT_SQUARE: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];

fn cell_bits([x, y, z]: [i32; 3]) -> u32 {
    debug_assert!([x, y, z].iter().all(|c| (0..1 << CELL_BITS).contains(c)));
    (x as u32 | (y as u32) << CELL_BITS | (z as u32) << (2 * CELL_BITS)) << CELL_SHIFT
}

fn cell_of(lo: u32) -> [i32; 3] {
    let c = lo >> CELL_SHIFT;
    [0, 1, 2].map(|a| (c >> (a * CELL_BITS) & mask(CELL_BITS)) as i32)
}

/// A layer as triangles: four vertices a quad and six indices, for readers
/// that want them rather than records.
#[derive(Default, Debug, Clone, PartialEq)]
pub struct Soup {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub materials: Vec<u32>,
    /// The packed [`Light`] of each vertex's quad.
    pub light: Vec<u32>,
    pub indices: Vec<u32>,
}

impl Soup {
    pub fn of(quads: &[Quad]) -> Soup {
        let mut soup = Soup::default();
        for quad in quads {
            let base = soup.positions.len() as u32;
            for corner in quad.corners() {
                soup.positions.push(corner.position);
                soup.normals.push(corner.normal);
                soup.uvs.push(corner.uv);
                soup.materials.push(quad.material);
                soup.light.push(u32::from(quad.light.packed()));
            }
            soup.indices
                .extend(QUAD_CORNERS.map(|corner| base + corner as u32));
        }
        soup
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIT: Light = Light::new(15, 7);

    #[test]
    fn a_face_keeps_its_fields_at_their_extremes() {
        for face in 0..6 {
            for plane in [-PLANE_BIAS, 0, 512, 512 + PLANE_BIAS] {
                for (u, v) in [
                    ([0, 1], [0, 1]),
                    ([0, 512], [511, 512]),
                    ([511, 512], [0, 512]),
                ] {
                    let q = Quad::face(face, plane, u, v, super::super::FLAME, LIT);
                    assert_eq!(q.kind(), QuadKind::Face(face));
                    assert_eq!(q.plane(), Some(plane));
                    assert_eq!(q.spans(), Some((u, v)));
                    assert_eq!(q.geometry[1] >> PAINT_SHIFT, 0, "the table's bits are free");
                }
            }
        }
    }

    #[test]
    fn a_face_moves_to_another_plane_and_keeps_the_rest() {
        let q = Quad::face(3, 40, [16, 48], [0, 16], 7, LIT);
        let moved = q.at_plane(-3);
        assert_eq!(moved.plane(), Some(-3));
        assert_eq!(moved.spans(), q.spans());
        assert_eq!((moved.material, moved.light), (7, LIT));
    }

    #[test]
    fn a_face_s_uv_restarts_at_the_voxel_it_starts_in() {
        let q = Quad::face(1, 32, [20, 36], [0, 8], 1, LIT);
        let uvs = q.corners().map(|c| c.uv);
        assert_eq!(uvs, [[0.25, 0.0], [1.25, 0.0], [1.25, 0.5], [0.25, 0.5]]);
    }

    #[test]
    fn flat_forms_keep_their_cell_side_and_extent() {
        let cell = [31, 0, 17];
        for reversed in [false, true] {
            for diagonal in 0..2 {
                let q = Quad::cross(cell, diagonal, reversed, 2, LIT);
                assert_eq!(q.kind(), QuadKind::Cross);
                let normal = q.corners()[0].normal;
                assert_eq!(normal, [0.0, 0.0, if reversed { -1.0 } else { 1.0 }]);
                assert!(q.corners().iter().all(|c| c.position[0] >= 31.0));
            }
            for axis in 0..3 {
                let q = Quad::plane_form(cell, axis, reversed, 16, [[0, 16], [15, 16]], 2, LIT);
                assert_eq!(q.kind(), QuadKind::Plane);
                let c = q.corners();
                let on = |a: usize| c.iter().map(move |c| c.position[a]);
                assert!(on(axis).all(|p| p == (cell[axis] + 1) as f32));
                assert_eq!(q.plane(), None);
                assert_eq!(q.geometry[1] >> PAINT_SHIFT, 0);
            }
        }
    }

    #[test]
    fn the_bounds_are_exactly_the_corners_least_and_greatest() {
        let mut quads = Vec::new();
        for face in 0..6 {
            for plane in [-PLANE_BIAS, 0, 7, 512 + PLANE_BIAS] {
                quads.push(Quad::face(face, plane, [0, 512], [511, 512], 1, LIT));
                quads.push(Quad::face(face, plane, [20, 36], [3, 8], 1, LIT));
            }
        }
        for reversed in [false, true] {
            for diagonal in 0..2 {
                quads.push(Quad::cross([31, 0, 17], diagonal, reversed, 2, LIT));
            }
            for axis in 0..3 {
                for offset in [0, 9, 16] {
                    let spans = [[3, 16], [0, 7]];
                    quads.push(Quad::plane_form(
                        [5, 31, 0],
                        axis,
                        reversed,
                        offset,
                        spans,
                        2,
                        LIT,
                    ));
                }
            }
        }
        for q in quads {
            let corners = q.corners().map(|c| c.position);
            let least =
                std::array::from_fn(|a| corners.iter().map(|c| c[a]).fold(f32::MAX, f32::min));
            let most =
                std::array::from_fn(|a| corners.iter().map(|c| c[a]).fold(f32::MIN, f32::max));
            assert_eq!(q.bounds(), (least, most), "{:?}", q.kind());
        }
    }

    #[test]
    fn the_soup_is_four_corners_and_two_triangles_a_quad() {
        let quads = [
            Quad::face(0, 0, [0, 16], [0, 16], 1, LIT),
            Quad::cross([1, 2, 3], 1, true, 2, LIT),
        ];
        let soup = Soup::of(&quads);
        assert_eq!(soup.positions.len(), 8);
        assert_eq!(soup.indices, [0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7]);
        assert_eq!(soup.materials, [1, 1, 1, 1, 2, 2, 2, 2]);
        assert!(soup.light.iter().all(|&l| l == u32::from(LIT.packed())));
    }
}
