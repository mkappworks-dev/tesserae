//! Shapes: a Voxel's geometry, as rows in a table.
//!
//! A Shape is data rather than an enum variant so that a Mod can declare one
//! at load time. It is declared as a union of boxes in sixteenths of a Voxel,
//! and everything the engine asks of it - what it hides, what light it stops,
//! what the mesher emits, what a body bumps into - is **derived once** at
//! registration from one 16x16x16 occupancy grid. One derivation means a
//! rotation bug is one bug, found for every Shape at once, instead of three
//! hand-kept tables drifting apart.
//!
//! Nothing here is in metres: a box is a fraction of a Voxel, which is
//! dimensionless, and whoever turns one into a distance multiplies by the
//! World's Voxel size. `docs/adr/0003-voxel-size-is-per-world.md`.

use crate::mesh::{NEG_X, NEG_Y, NEG_Z, POS_X, POS_Y, POS_Z};
use crate::voxel::{SHAPE_FULL_CUBE, ShapeId, Voxel};

/// Sub-cells along a Voxel's edge. Sixteenths because they are exact: every
/// coverage test is an integer comparison, so no float epsilon decides whether
/// a hole appears.
pub const SUBDIV: usize = 16;

pub const SHAPE_SLAB: ShapeId = 1;
pub const SHAPE_STAIR: ShapeId = 2;
pub const SHAPE_POST: ShapeId = 3;

/// How many rotations a Shape is registered under.
pub const ORIENTATIONS: usize = 24;

/// Bounds a Mod's declaration so one Shape cannot cost unbounded registration
/// time or collision work.
pub const MAX_BOXES: usize = 16;

/// The most shaped Voxels a Chunk may hold.
///
/// A shaped Voxel emits up to about a dozen quads that nothing merges, where a
/// cube merges into almost nothing, so a Chunk of them is hundreds of thousands
/// of quads. Enforced where Voxels come from - the edit path rejects a Place
/// that would cross it, and worldgen places none - and not by the mesher,
/// which dropping geometry silently would make a hole nobody could explain.
///
/// Sized by measurement (`cargo bench -p tesserae-core`, Apple M5): terrain with
/// a stair on top of every column, which is 1024 of them, meshes in 527 us
/// against 350 us for the terrain alone - 41% of a `mesh_chunk/straddling`, and
/// the rule was 50%, because a tick has twelve meshes in it. A Chunk of nothing
/// but stairs takes 5.2 ms.
pub const MAX_SHAPED_PER_CHUNK: usize = 1024;

/// Set in [`ShapeTable::light_mask`] for a shaped Voxel, meaning light is also
/// stopped from leaving through the sealed faces. A full cube lacks it: a cube
/// only ever holds its own emission, which leaves freely.
pub const LIGHT_EXIT_CHECKED: u8 = 0x40;

/// A box inside one Voxel, `0..=16` sixteenths on each axis.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NodeBox {
    pub min: [u8; 3],
    pub max: [u8; 3],
}

impl NodeBox {
    pub const fn new(min: [u8; 3], max: [u8; 3]) -> Self {
        NodeBox { min, max }
    }

    const CELL: NodeBox = NodeBox::new([0; 3], [SUBDIV as u8; 3]);
}

/// What a Mod declares for one Shape.
#[derive(Clone, Debug)]
pub struct ShapeDef {
    /// The geometry. Boxes may overlap.
    pub boxes: Vec<NodeBox>,
    /// What a body collides with when it differs from what is drawn - a
    /// pressure plate is thin to look at and nothing to walk into. `None` means
    /// the same boxes.
    pub collision: Option<Vec<NodeBox>>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShapeError {
    NoBoxes,
    TooManyBoxes,
    /// Out of range, or with no volume.
    BadBox,
    TableFull,
}

impl std::fmt::Display for ShapeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ShapeError::NoBoxes => "a Shape needs at least one box",
            ShapeError::TooManyBoxes => "a Shape has more than MAX_BOXES boxes",
            ShapeError::BadBox => "a box is outside the Voxel or has no volume",
            ShapeError::TableFull => "all 256 ShapeIds are taken",
        })
    }
}

impl std::error::Error for ShapeError {}

/// Faces in the order the rest of the crate uses, indexed by `up`. The identity
/// is first so a Voxel with no orientation set is upright.
const UP_FACES: [usize; 6] = [POS_Y, NEG_Y, POS_X, NEG_X, POS_Z, NEG_Z];

type Matrix = [[i8; 3]; 3];

/// The Shape's own +Y rotated onto each of [`UP_FACES`].
const TILT: [Matrix; 6] = [
    [[1, 0, 0], [0, 1, 0], [0, 0, 1]],
    [[1, 0, 0], [0, -1, 0], [0, 0, -1]],
    [[0, 1, 0], [-1, 0, 0], [0, 0, 1]],
    [[0, -1, 0], [1, 0, 0], [0, 0, 1]],
    [[1, 0, 0], [0, 0, -1], [0, 1, 0]],
    [[1, 0, 0], [0, 0, 1], [0, -1, 0]],
];

/// A quarter-turn about +Y, right-handed, by `turn`.
const TURN: [Matrix; 4] = [
    [[1, 0, 0], [0, 1, 0], [0, 0, 1]],
    [[0, 0, 1], [0, 1, 0], [-1, 0, 0]],
    [[-1, 0, 0], [0, 1, 0], [0, 0, -1]],
    [[0, 0, -1], [0, 1, 0], [1, 0, 0]],
];

fn mul(a: &Matrix, b: &Matrix) -> Matrix {
    let mut m = [[0i8; 3]; 3];
    for (i, row) in m.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            *cell = (0..3).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    m
}

fn apply(m: &Matrix, v: [i32; 3]) -> [i32; 3] {
    std::array::from_fn(|i| (0..3).map(|k| m[i][k] as i32 * v[k]).sum())
}

/// One of the 24 axis-aligned rotations of a Shape, in the Voxel's 8
/// orientation bits as `up * 4 + turn`.
///
/// One encoding serves every Shape: a slab's up and down is `up`, a stair's four
/// facings are `turn`, an upside-down stair is both. A Shape with symmetries
/// simply has duplicates, which cost nothing because the table is indexed.
/// The top three bits are reserved and must be zero.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub struct Orientation(u8);

impl Orientation {
    pub const IDENTITY: Orientation = Orientation(0);

    /// `up` is one of the face constants in [`crate::mesh`]: which way the
    /// Shape's own +Y points. `turn` counts quarter-turns about that axis and
    /// wraps.
    pub fn new(up: usize, turn: u8) -> Self {
        let up = UP_FACES
            .iter()
            .position(|&f| f == up)
            .expect("`up` is a face constant");
        Orientation((up * 4) as u8 + turn % 4)
    }

    /// `None` for 24 and above. A Voxel read with such a byte behaves as the
    /// identity, and an Edit carrying one is rejected.
    pub const fn from_bits(bits: u8) -> Option<Self> {
        if (bits as usize) < ORIENTATIONS {
            Some(Orientation(bits))
        } else {
            None
        }
    }

    pub const fn bits(self) -> u8 {
        self.0
    }

    pub fn up(self) -> usize {
        UP_FACES[(self.0 / 4) as usize]
    }

    pub const fn turn(self) -> u8 {
        self.0 % 4
    }

    /// The next quarter-turn about the same `up`.
    pub const fn turned(self) -> Self {
        Orientation(self.0 / 4 * 4 + (self.0 % 4 + 1) % 4)
    }

    /// The next `up`, with the same `turn`. Alternating this with
    /// [`Orientation::turned`] reaches all 24, which is how a player steps
    /// through them with two keys.
    pub const fn flipped(self) -> Self {
        Orientation((self.0 + 4) % ORIENTATIONS as u8)
    }

    pub fn all() -> impl Iterator<Item = Orientation> {
        (0..ORIENTATIONS as u8).map(Orientation)
    }

    pub(crate) fn matrix(self) -> Matrix {
        mul(&TILT[(self.0 / 4) as usize], &TURN[self.turn() as usize])
    }
}

/// Which sub-squares of a face plane a Shape's solid touches. Rows run along
/// the face's second tangent axis, bits along its first, where a face on axis
/// `d` has tangents `(d + 1) % 3` and `(d + 2) % 3` - the mesher's own
/// convention, so a Voxel's face and the matching face of the Voxel across it
/// index the same squares.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FaceMask([u16; SUBDIV]);

impl FaceMask {
    pub const EMPTY: FaceMask = FaceMask([0; SUBDIV]);
    pub const FULL: FaceMask = FaceMask([u16::MAX; SUBDIV]);

    pub fn is_full(&self) -> bool {
        *self == Self::FULL
    }

    pub fn is_empty(&self) -> bool {
        *self == Self::EMPTY
    }

    /// Whether every square of `[u0, u1) x [v0, v1)` is set.
    pub fn contains_rect(&self, u0: u8, v0: u8, u1: u8, v1: u8) -> bool {
        let bits = (((1u32 << (u1 - u0)) - 1) << u0) as u16;
        self.0[v0 as usize..v1 as usize]
            .iter()
            .all(|row| row & bits == bits)
    }

    /// Whether `self` covers every square `other` does.
    pub fn covers(&self, other: &FaceMask) -> bool {
        self.0.iter().zip(&other.0).all(|(a, b)| b & !a == 0)
    }

    fn set(&mut self, u: usize, v: usize) {
        self.0[v] |= 1 << u;
    }
}

/// One rectangle of a Shape's exposed surface, in sixteenths.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ShapeQuad {
    /// Which way it faces, as a face constant.
    pub face: u8,
    /// Its position along the face's own axis, `0..=16`.
    pub plane: u8,
    pub u0: u8,
    pub v0: u8,
    pub u1: u8,
    pub v1: u8,
}

impl ShapeQuad {
    /// Whether it lies on the Voxel's own boundary and so looks out at a
    /// neighbour. Such a quad always faces outwards, because the solid is
    /// inside the cell; any other quad looks into the Voxel's own air.
    pub fn on_boundary(&self) -> bool {
        self.plane == 0 || self.plane == SUBDIV as u8
    }

    /// Area in sub-cell faces.
    pub fn area(&self) -> u32 {
        (self.u1 - self.u0) as u32 * (self.v1 - self.v0) as u32
    }
}

/// Everything derived for one (Shape, Orientation).
#[derive(Clone, Copy, Debug)]
struct Variant {
    seals: u8,
    coverage: [FaceMask; 6],
    quads: (u32, u32),
    collision: (u32, u32),
    selection: (u32, u32),
    volume: u16,
}

/// What an unknown Shape and air both resolve to: nothing there at all.
const NOTHING: Variant = Variant {
    seals: 0,
    coverage: [FaceMask::EMPTY; 6],
    quads: (0, 0),
    collision: (0, 0),
    selection: (0, 0),
    volume: 0,
};

/// Every registered Shape under every Orientation.
///
/// Built to be filled by one batched `register` per Shape at load and read
/// natively afterwards, so a Mod never makes a per-Voxel call across the WASM
/// boundary. `docs/adr/0001-wasm-mod-boundary.md`.
#[derive(Clone, Debug)]
pub struct ShapeTable {
    shapes: usize,
    variants: Vec<Variant>,
    quads: Vec<ShapeQuad>,
    boxes: Vec<NodeBox>,
    selection: Vec<NodeBox>,
}

impl Default for ShapeTable {
    fn default() -> Self {
        Self::builtin()
    }
}

impl ShapeTable {
    /// The Shapes every World has, under permanent ids: a Mod's ids start after
    /// them, and a saved World refers to them by number.
    pub fn builtin() -> Self {
        let mut table = ShapeTable {
            shapes: 0,
            variants: Vec::new(),
            quads: Vec::new(),
            boxes: Vec::new(),
            selection: Vec::new(),
        };
        let b = NodeBox::new;
        let defs = [
            vec![NodeBox::CELL],
            vec![b([0, 0, 0], [16, 8, 16])],
            vec![b([0, 0, 0], [16, 8, 16]), b([0, 8, 8], [16, 16, 16])],
            vec![b([6, 0, 6], [10, 16, 10])],
        ];
        for boxes in defs {
            table
                .register(ShapeDef {
                    boxes,
                    collision: None,
                })
                .expect("the built-in Shapes are valid");
        }
        debug_assert_eq!(table.shapes, SHAPE_POST as usize + 1);
        table
    }

    /// Adds a Shape and returns its id. Ids are handed out in registration
    /// order, so a World's name-to-id registry is just that order saved.
    pub fn register(&mut self, def: ShapeDef) -> Result<ShapeId, ShapeError> {
        if self.shapes > ShapeId::MAX as usize {
            return Err(ShapeError::TableFull);
        }
        validate(&def.boxes)?;
        if let Some(collision) = &def.collision {
            // Empty is allowed: it is how a Shape is walked through.
            if collision.len() > MAX_BOXES {
                return Err(ShapeError::TooManyBoxes);
            }
            for b in collision {
                check_box(b)?;
            }
        }

        let grid = rasterise(&def.boxes);
        let collision = def.collision.as_deref().unwrap_or(&def.boxes);
        for orientation in Orientation::all() {
            let m = orientation.matrix();
            let rotated = rotate_grid(&grid, &m);

            let quads_start = self.quads.len() as u32;
            extract_quads(&rotated, &mut self.quads);
            let boxes_start = self.boxes.len() as u32;
            self.boxes
                .extend(collision.iter().map(|b| rotate_box(b, &m)));
            let selection_start = self.selection.len() as u32;
            self.selection
                .extend(def.boxes.iter().map(|b| rotate_box(b, &m)));

            let coverage: [FaceMask; 6] = std::array::from_fn(|f| face_coverage(&rotated, f));
            let seals = (0..6)
                .filter(|&f| coverage[f].is_full())
                .fold(0u8, |acc, f| acc | 1 << f);
            self.variants.push(Variant {
                seals,
                coverage,
                quads: (quads_start, self.quads.len() as u32 - quads_start),
                collision: (boxes_start, self.boxes.len() as u32 - boxes_start),
                selection: (
                    selection_start,
                    self.selection.len() as u32 - selection_start,
                ),
                volume: rotated.iter().filter(|&&set| set).count() as u16,
            });
        }
        self.shapes += 1;
        Ok((self.shapes - 1) as ShapeId)
    }

    pub fn len(&self) -> usize {
        self.shapes
    }

    pub fn is_empty(&self) -> bool {
        self.shapes == 0
    }

    pub fn contains(&self, shape: ShapeId) -> bool {
        (shape as usize) < self.shapes
    }

    #[inline]
    fn variant(&self, voxel: Voxel) -> &Variant {
        let shape = voxel.shape() as usize;
        if voxel.is_air() || shape >= self.shapes {
            return &NOTHING;
        }
        // An orientation byte past 23 reads as the identity rather than
        // indexing into another Shape's variants.
        let orientation = match voxel.orientation() as usize {
            o if o < ORIENTATIONS => o,
            _ => 0,
        };
        &self.variants[shape * ORIENTATIONS + orientation]
    }

    /// Whether `voxel` covers its whole `face`. The question both occlusion and
    /// light ask, and the cube and air cases never reach the table.
    #[inline]
    pub fn seals(&self, voxel: Voxel, face: usize) -> bool {
        if voxel.shape() == SHAPE_FULL_CUBE {
            return !voxel.is_air();
        }
        self.variant(voxel).seals >> face & 1 != 0
    }

    pub fn coverage(&self, voxel: Voxel, face: usize) -> &FaceMask {
        if voxel.shape() == SHAPE_FULL_CUBE && !voxel.is_air() {
            return &FaceMask::FULL;
        }
        &self.variant(voxel).coverage[face]
    }

    /// The Voxel's exposed surface in sixteenths, ready to emit.
    pub fn quads(&self, voxel: Voxel) -> &[ShapeQuad] {
        let (start, len) = self.variant(voxel).quads;
        &self.quads[start as usize..(start + len) as usize]
    }

    /// What a body collides with, rotated, in sixteenths.
    pub fn collision(&self, voxel: Voxel) -> &[NodeBox] {
        let (start, len) = self.variant(voxel).collision;
        &self.boxes[start as usize..(start + len) as usize]
    }

    /// What a ray picks: the declared boxes, rotated. Not [`ShapeTable::collision`],
    /// which a Shape may make larger, smaller or empty without changing what is
    /// drawn, and a player aims at what is drawn.
    pub fn selection(&self, voxel: Voxel) -> &[NodeBox] {
        let (start, len) = self.variant(voxel).selection;
        &self.selection[start as usize..(start + len) as usize]
    }

    /// Sub-cells occupied, out of `SUBDIV^3`. What LOD reads: a coarse Voxel is
    /// solid if its eight children's volumes sum to at least half.
    pub fn volume(&self, voxel: Voxel) -> u16 {
        self.variant(voxel).volume
    }

    /// The bits the light solvers test: faces light may not enter through, and
    /// [`LIGHT_EXIT_CHECKED`] if it may not leave through them either.
    #[inline]
    pub fn light_mask(&self, voxel: Voxel) -> u8 {
        if voxel.shape() == SHAPE_FULL_CUBE {
            return if voxel.is_air() { 0 } else { 0x3F };
        }
        match self.variant(voxel).seals {
            0 => 0,
            seals => seals | LIGHT_EXIT_CHECKED,
        }
    }
}

fn check_box(b: &NodeBox) -> Result<(), ShapeError> {
    let cell = SUBDIV as u8;
    let ok = (0..3).all(|a| b.min[a] < b.max[a] && b.max[a] <= cell);
    if ok { Ok(()) } else { Err(ShapeError::BadBox) }
}

fn validate(boxes: &[NodeBox]) -> Result<(), ShapeError> {
    if boxes.is_empty() {
        return Err(ShapeError::NoBoxes);
    }
    if boxes.len() > MAX_BOXES {
        return Err(ShapeError::TooManyBoxes);
    }
    boxes.iter().try_for_each(check_box)
}

/// x contiguous, matching `chunk::index`.
const fn at(x: usize, y: usize, z: usize) -> usize {
    (y * SUBDIV + z) * SUBDIV + x
}

type Grid = [bool; SUBDIV * SUBDIV * SUBDIV];

fn rasterise(boxes: &[NodeBox]) -> Grid {
    let mut grid = [false; SUBDIV * SUBDIV * SUBDIV];
    for b in boxes {
        for y in b.min[1]..b.max[1] {
            for z in b.min[2]..b.max[2] {
                for x in b.min[0]..b.max[0] {
                    grid[at(x as usize, y as usize, z as usize)] = true;
                }
            }
        }
    }
    grid
}

/// Rotates about the cell's centre. Sub-cell indices are doubled and centred
/// (`2i - 15`) so the centre is the origin and every rotation is exact integer
/// arithmetic.
fn rotate_grid(grid: &Grid, m: &Matrix) -> Grid {
    let half = SUBDIV as i32 - 1;
    let mut out = [false; SUBDIV * SUBDIV * SUBDIV];
    for y in 0..SUBDIV {
        for z in 0..SUBDIV {
            for x in 0..SUBDIV {
                if !grid[at(x, y, z)] {
                    continue;
                }
                let c = apply(
                    m,
                    [
                        2 * x as i32 - half,
                        2 * y as i32 - half,
                        2 * z as i32 - half,
                    ],
                );
                out[at(
                    ((c[0] + half) / 2) as usize,
                    ((c[1] + half) / 2) as usize,
                    ((c[2] + half) / 2) as usize,
                )] = true;
            }
        }
    }
    out
}

/// A box's corners are on grid lines rather than sub-cell centres, so the
/// centre is 8 and the arithmetic is the same without the doubling.
fn rotate_box(b: &NodeBox, m: &Matrix) -> NodeBox {
    let half = SUBDIV as i32 / 2;
    let lo = apply(m, b.min.map(|c| c as i32 - half));
    let hi = apply(m, b.max.map(|c| c as i32 - half));
    NodeBox {
        min: std::array::from_fn(|a| (lo[a].min(hi[a]) + half) as u8),
        max: std::array::from_fn(|a| (lo[a].max(hi[a]) + half) as u8),
    }
}

/// `(layer, u, v)` as a grid cell, for a face on axis `d`.
fn cell(d: usize, layer: usize, u: usize, v: usize) -> usize {
    let mut c = [0usize; 3];
    c[d] = layer;
    c[(d + 1) % 3] = u;
    c[(d + 2) % 3] = v;
    at(c[0], c[1], c[2])
}

fn face_coverage(grid: &Grid, face: usize) -> FaceMask {
    let (d, positive) = (face / 2, face % 2 == 1);
    let layer = if positive { SUBDIV - 1 } else { 0 };
    let mut mask = FaceMask::EMPTY;
    for v in 0..SUBDIV {
        for u in 0..SUBDIV {
            if grid[cell(d, layer, u, v)] {
                mask.set(u, v);
            }
        }
    }
    mask
}

/// The grid's exposed surface as rectangles: a face exists wherever a set
/// sub-cell has an empty one (or the outside) on that side. Faces between two
/// boxes of the same Shape are gone, which is why a Mod may overlap its boxes.
fn extract_quads(grid: &Grid, out: &mut Vec<ShapeQuad>) {
    for face in 0..6 {
        let (d, positive) = (face / 2, face % 2 == 1);
        for plane in 0..=SUBDIV {
            let (inside, outside) = if positive {
                (plane.checked_sub(1), Some(plane))
            } else {
                (Some(plane), plane.checked_sub(1))
            };
            let solid = |layer: Option<usize>, u, v| {
                layer.is_some_and(|l| l < SUBDIV && grid[cell(d, l, u, v)])
            };
            let mut exposed = [false; SUBDIV * SUBDIV];
            for v in 0..SUBDIV {
                for u in 0..SUBDIV {
                    exposed[v * SUBDIV + u] = solid(inside, u, v) && !solid(outside, u, v);
                }
            }
            merge_rects(&mut exposed, |u0, v0, u1, v1| {
                out.push(ShapeQuad {
                    face: face as u8,
                    plane: plane as u8,
                    u0,
                    v0,
                    u1,
                    v1,
                });
            });
        }
    }
}

/// Greedy rectangles over a 16x16 mask, rows being `v`.
fn merge_rects(mask: &mut [bool; SUBDIV * SUBDIV], mut emit: impl FnMut(u8, u8, u8, u8)) {
    for v in 0..SUBDIV {
        for u in 0..SUBDIV {
            if !mask[v * SUBDIV + u] {
                continue;
            }
            let mut w = 1;
            while u + w < SUBDIV && mask[v * SUBDIV + u + w] {
                w += 1;
            }
            let mut h = 1;
            'grow: while v + h < SUBDIV {
                for k in 0..w {
                    if !mask[(v + h) * SUBDIV + u + k] {
                        break 'grow;
                    }
                }
                h += 1;
            }
            for dv in 0..h {
                for du in 0..w {
                    mask[(v + dv) * SUBDIV + u + du] = false;
                }
            }
            emit(u as u8, v as u8, (u + w) as u8, (v + h) as u8);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn voxel(shape: ShapeId, up: usize, turn: u8) -> Voxel {
        Voxel::new(1, shape, Orientation::new(up, turn).bits())
    }

    fn sealed_faces(t: &ShapeTable, v: Voxel) -> Vec<usize> {
        (0..6).filter(|&f| t.seals(v, f)).collect()
    }

    #[test]
    fn for_the_builtins_what_is_drawn_and_what_is_bumped_into_are_the_same_boxes() {
        let t = ShapeTable::builtin();
        for shape in [SHAPE_FULL_CUBE, SHAPE_SLAB, SHAPE_STAIR, SHAPE_POST] {
            for o in Orientation::all() {
                let v = Voxel::new(1, shape, o.bits());
                assert_eq!(t.selection(v), t.collision(v), "shape {shape} {o:?}");
            }
        }
    }

    #[test]
    fn there_are_24_distinct_proper_rotations_and_byte_zero_is_the_identity() {
        let all: Vec<Matrix> = Orientation::all().map(Orientation::matrix).collect();
        assert_eq!(all.len(), ORIENTATIONS);
        assert_eq!(all[0], [[1, 0, 0], [0, 1, 0], [0, 0, 1]]);
        for (i, a) in all.iter().enumerate() {
            let det = a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1])
                - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
                + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0]);
            assert_eq!(det, 1, "orientation {i} is a reflection");
            for b in &all[i + 1..] {
                assert_ne!(a, b, "two orientations are the same rotation");
            }
        }
    }

    #[test]
    fn up_and_turn_round_trip_and_four_turns_come_home() {
        for o in Orientation::all() {
            assert_eq!(Orientation::new(o.up(), o.turn()), o);
        }
        for up in UP_FACES {
            let turned = (0..4).fold(Orientation::new(up, 0).matrix(), |m, _| mul(&m, &TURN[1]));
            assert_eq!(turned, Orientation::new(up, 0).matrix());
        }
        assert_eq!(Orientation::new(POS_Y, 5), Orientation::new(POS_Y, 1));
    }

    #[test]
    fn turning_and_flipping_reach_every_orientation_and_come_home() {
        let mut seen = std::collections::BTreeSet::new();
        let mut frontier = vec![Orientation::IDENTITY];
        while let Some(o) = frontier.pop() {
            if seen.insert(o.bits()) {
                frontier.extend([o.turned(), o.flipped()]);
            }
        }
        assert_eq!(seen.len(), ORIENTATIONS);
        assert!(seen.iter().all(|&b| Orientation::from_bits(b).is_some()));

        let o = Orientation::new(POS_Z, 1);
        assert_eq!((0..4).fold(o, |o, _| o.turned()), o);
        assert_eq!((0..6).fold(o, |o, _| o.flipped()), o);
        assert_eq!((o.turned().up(), o.turned().turn()), (o.up(), 2));
        assert_eq!((o.flipped().up() != o.up(), o.flipped().turn()), (true, 1));
    }

    #[test]
    fn only_bytes_below_24_are_orientations() {
        assert!(Orientation::from_bits(23).is_some());
        assert!(Orientation::from_bits(24).is_none());
        assert!(Orientation::from_bits(255).is_none());
        assert_eq!(Orientation::IDENTITY.bits(), 0);
    }

    #[test]
    fn canonical_shapes_seal_the_faces_they_should() {
        let t = ShapeTable::builtin();
        assert_eq!(sealed_faces(&t, voxel(SHAPE_FULL_CUBE, POS_Y, 0)).len(), 6);
        assert_eq!(sealed_faces(&t, voxel(SHAPE_SLAB, POS_Y, 0)), [NEG_Y]);
        assert_eq!(sealed_faces(&t, voxel(SHAPE_SLAB, NEG_Y, 0)), [POS_Y]);
        assert_eq!(
            sealed_faces(&t, voxel(SHAPE_STAIR, POS_Y, 0)),
            [NEG_Y, POS_Z]
        );
        assert!(sealed_faces(&t, voxel(SHAPE_POST, POS_Y, 0)).is_empty());
    }

    #[test]
    fn a_stairs_tall_side_follows_its_turn() {
        let t = ShapeTable::builtin();
        for (turn, back) in [(0, POS_Z), (1, POS_X), (2, NEG_Z), (3, NEG_X)] {
            let mut expected = vec![NEG_Y, back];
            expected.sort();
            assert_eq!(
                sealed_faces(&t, voxel(SHAPE_STAIR, POS_Y, turn)),
                expected,
                "turn {turn}"
            );
        }
        // Flipped over, the tread faces down and the tall side swings to -Z.
        assert_eq!(
            sealed_faces(&t, voxel(SHAPE_STAIR, NEG_Y, 0)),
            [POS_Y, NEG_Z]
        );
    }

    #[test]
    fn a_stairs_side_is_an_l_and_a_slabs_side_is_a_half() {
        let t = ShapeTable::builtin();
        let stair = voxel(SHAPE_STAIR, POS_Y, 0);
        for side in [NEG_X, POS_X] {
            let mask = t.coverage(stair, side);
            assert!(!mask.is_full() && !mask.is_empty());
            // Rows are z, bits are y for a face on x: (u, v) = (y, z).
            assert!(mask.contains_rect(0, 0, 8, 16), "the lower half");
            assert!(mask.contains_rect(8, 8, 16, 16), "the upper back");
            assert!(!mask.contains_rect(8, 0, 16, 8), "the notch");
        }
        let slab = t.coverage(voxel(SHAPE_SLAB, POS_Y, 0), POS_X);
        assert!(slab.contains_rect(0, 0, 8, 16));
        assert!(!slab.contains_rect(0, 0, 9, 16));
    }

    #[test]
    fn volumes_match_the_geometry_and_survive_rotation() {
        let t = ShapeTable::builtin();
        for (shape, volume) in [
            (SHAPE_FULL_CUBE, 4096),
            (SHAPE_STAIR, 3072),
            (SHAPE_SLAB, 2048),
            (SHAPE_POST, 256),
        ] {
            for o in Orientation::all() {
                let v = Voxel::new(1, shape, o.bits());
                assert_eq!(t.volume(v), volume, "shape {shape} orientation {o:?}");
            }
        }
    }

    /// Internal faces are gone: the sum of the quads is the exposed surface and
    /// nothing else, so a face between two boxes of one Shape would inflate it.
    #[test]
    fn quads_are_exactly_the_exposed_surface() {
        let t = ShapeTable::builtin();
        for (shape, area) in [
            (SHAPE_FULL_CUBE, 6 * 256),
            (SHAPE_SLAB, 2 * 256 + 4 * 128),
            // Tread, upper top, front, riser, back, bottom, and two L-shaped sides.
            (SHAPE_STAIR, 128 + 128 + 128 + 128 + 256 + 256 + 2 * 192),
            (SHAPE_POST, 4 * 64 + 2 * 16),
        ] {
            for o in Orientation::all() {
                let v = Voxel::new(1, shape, o.bits());
                let sum: u32 = t.quads(v).iter().map(ShapeQuad::area).sum();
                assert_eq!(sum, area, "shape {shape} orientation {o:?}");
            }
        }
    }

    #[test]
    fn a_boundary_quad_always_faces_outwards() {
        let t = ShapeTable::builtin();
        for shape in 0..t.len() as ShapeId {
            for o in Orientation::all() {
                for q in t.quads(Voxel::new(1, shape, o.bits())) {
                    if q.on_boundary() {
                        let positive = q.face % 2 == 1;
                        assert_eq!(q.plane == 16, positive, "{q:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn collision_boxes_rotate_with_the_orientation_and_stay_in_the_cell() {
        let t = ShapeTable::builtin();
        let slab_top = t.collision(voxel(SHAPE_SLAB, NEG_Y, 0));
        assert_eq!(slab_top, [NodeBox::new([0, 8, 0], [16, 16, 16])]);
        for shape in 0..t.len() as ShapeId {
            for o in Orientation::all() {
                for b in t.collision(Voxel::new(1, shape, o.bits())) {
                    assert!(
                        (0..3).all(|a| b.min[a] < b.max[a] && b.max[a] <= 16),
                        "{b:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_shape_can_be_declared_thin_to_look_at_and_nothing_to_walk_into() {
        let mut t = ShapeTable::builtin();
        let plate = t
            .register(ShapeDef {
                boxes: vec![NodeBox::new([1, 0, 1], [15, 1, 15])],
                collision: Some(vec![]),
            })
            .unwrap();
        let v = Voxel::new(1, plate, 0);
        assert!(t.collision(v).is_empty());
        assert!(!t.quads(v).is_empty());
        assert!(
            !t.seals(v, NEG_Y),
            "a plate inset from the edges covers nothing"
        );
        assert_eq!(t.volume(v), 14 * 14);
    }

    #[test]
    fn overlapping_boxes_do_not_leave_internal_faces() {
        let mut t = ShapeTable::builtin();
        let tee = t
            .register(ShapeDef {
                boxes: vec![
                    NodeBox::new([0, 0, 0], [16, 8, 16]),
                    NodeBox::new([0, 4, 0], [16, 16, 16]),
                ],
                collision: None,
            })
            .unwrap();
        let sum: u32 = t
            .quads(Voxel::new(1, tee, 0))
            .iter()
            .map(ShapeQuad::area)
            .sum();
        assert_eq!(
            sum,
            6 * 256,
            "two overlapping boxes that make a cube mesh as one"
        );
    }

    #[test]
    fn registration_rejects_what_it_should_and_hands_out_ids_in_order() {
        let mut t = ShapeTable::builtin();
        let ok = || NodeBox::new([0, 0, 0], [4, 4, 4]);
        let def = |boxes| ShapeDef {
            boxes,
            collision: None,
        };
        assert_eq!(t.register(def(vec![])), Err(ShapeError::NoBoxes));
        assert_eq!(
            t.register(def(vec![NodeBox::new([0, 0, 0], [17, 4, 4])])),
            Err(ShapeError::BadBox)
        );
        assert_eq!(
            t.register(def(vec![NodeBox::new([4, 0, 0], [4, 4, 4])])),
            Err(ShapeError::BadBox)
        );
        assert_eq!(
            t.register(def(vec![ok(); MAX_BOXES + 1])),
            Err(ShapeError::TooManyBoxes)
        );
        assert_eq!(t.register(def(vec![ok()])), Ok(4));
        assert_eq!(t.register(def(vec![ok()])), Ok(5));
        while t.len() < 256 {
            t.register(def(vec![ok()])).unwrap();
        }
        assert_eq!(t.register(def(vec![ok()])), Err(ShapeError::TableFull));
    }

    #[test]
    fn an_unknown_shape_and_air_are_nothing_at_all() {
        let t = ShapeTable::builtin();
        let unknown = Voxel::new(1, 200, 0);
        let air = Voxel::new(0, SHAPE_STAIR, 0);
        for v in [unknown, air, Voxel::AIR] {
            assert!((0..6).all(|f| !t.seals(v, f)), "{v:?}");
            assert!(t.quads(v).is_empty() && t.collision(v).is_empty());
            assert_eq!((t.volume(v), t.light_mask(v)), (0, 0));
        }
    }

    #[test]
    fn an_invalid_orientation_byte_reads_as_the_identity() {
        let t = ShapeTable::builtin();
        let wild = Voxel::new(1, SHAPE_STAIR, 200);
        let upright = Voxel::new(1, SHAPE_STAIR, 0);
        assert_eq!(t.quads(wild), t.quads(upright));
    }

    #[test]
    fn light_masks_separate_a_cubes_emission_from_a_shaped_voxels_exit() {
        let t = ShapeTable::builtin();
        assert_eq!(t.light_mask(Voxel::cube(1)), 0x3F);
        assert_eq!(t.light_mask(Voxel::AIR), 0);
        let stair = t.light_mask(voxel(SHAPE_STAIR, POS_Y, 0));
        assert_eq!(stair, LIGHT_EXIT_CHECKED | 1 << NEG_Y | 1 << POS_Z);
        assert_eq!(t.light_mask(voxel(SHAPE_POST, POS_Y, 0)), 0);
    }
}
