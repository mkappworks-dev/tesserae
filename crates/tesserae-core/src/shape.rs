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

pub mod examples;

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

/// The most shaped Voxels a Chunk may hold, counted in stairs: a connected Shape
/// is charged [`ShapeTable::cap_units`] of them.
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
///
/// A connected Shape is charged for its most armed self. The reference fence
/// costs 7 stairs, so 146 fit: 1024 of them, uncharged, meshed in 5.2 ms against
/// 1.5 ms for 1024 stairs in the same run (a loaded machine; the ratio is the
/// point), and at 146 a Chunk of fences takes about what a Chunk of stairs does.
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

/// A box a body collides with, in sixteenths of a Voxel and signed so that it
/// may stand outside the cell: `-16..=32` per axis, one whole Voxel of
/// [overhang](ShapeTable::overhang) at most on each side. A fence is
/// `[6, 0, 6]..[10, 24, 10]`, a post a Voxel and a half tall that you cannot
/// jump. Drawn geometry stays a [`NodeBox`]: only collision acts at a distance.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CollisionBox {
    pub min: [i8; 3],
    pub max: [i8; 3],
}

impl CollisionBox {
    pub const fn new(min: [i8; 3], max: [i8; 3]) -> Self {
        CollisionBox { min, max }
    }
}

impl From<NodeBox> for CollisionBox {
    fn from(b: NodeBox) -> Self {
        CollisionBox {
            min: b.min.map(|c| c as i8),
            max: b.max.map(|c| c as i8),
        }
    }
}

/// An arm a connected Shape grows towards one face-neighbour.
#[derive(Clone, Debug)]
pub struct Arm {
    /// The face constant of the neighbour it reaches for.
    pub face: usize,
    /// What is drawn when it is out. May touch only the boundary plane of
    /// `face`; see [`ShapeError::ArmTouchesOtherFace`].
    pub boxes: Vec<NodeBox>,
    /// What a body collides with when it differs from what is drawn. `None`
    /// means the same boxes.
    pub collision: Option<Vec<CollisionBox>>,
}

/// How a Shape grows arms towards the Voxels around it.
///
/// Derived at read time from the neighbours and never stored in a Voxel, so a
/// Chunk holds no fact that is a function of other facts.
/// `docs/adr/0008-connections-are-derived-not-stored.md`.
#[derive(Clone, Debug)]
pub struct ConnectDef {
    /// The connection groups this Shape belongs to, one bit each.
    pub group: u32,
    /// Grow an arm towards a neighbour in any of these groups.
    pub connects_to: u32,
    /// Also grow one towards a neighbour that seals the face facing us: a fence
    /// meeting a wall.
    pub to_sealed: bool,
    pub arms: Vec<Arm>,
}

/// Which of a Voxel's six faces have an arm out, as bits indexed by face
/// constant. Computed by [`ShapeTable::connections`], from the six
/// neighbours; the same answer wherever it is asked.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct Connections(u8);

impl Connections {
    pub const NONE: Connections = Connections(0);

    pub const fn bits(self) -> u8 {
        self.0
    }

    pub const fn has(self, face: usize) -> bool {
        self.0 >> face & 1 != 0
    }

    pub const fn with(self, face: usize) -> Self {
        Connections(self.0 | 1 << face)
    }
}

/// What a Mod declares for one Shape.
#[derive(Clone, Debug)]
pub struct ShapeDef {
    pub connect: Option<ConnectDef>,
    /// The geometry. Boxes may overlap.
    pub boxes: Vec<NodeBox>,
    /// What a body collides with when it differs from what is drawn - a
    /// pressure plate is thin to look at and nothing to walk into. `None` means
    /// the same boxes.
    pub collision: Option<Vec<CollisionBox>>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShapeError {
    NoBoxes,
    TooManyBoxes,
    /// Out of range, or with no volume.
    BadBox,
    TableFull,
    /// A connected Shape with no arms, an arm on a face that does not exist or
    /// is declared twice, or an arm with no boxes.
    BadArm,
    /// Some combination of arms seals a face. A Shape that does is a cube with
    /// extra steps: pass 1 and both light solvers assume a connected Shape
    /// seals nothing, so that they never have to look at a neighbour.
    ConnectedSeals,
    /// An arm reaches a boundary plane other than its own face's. Coverage
    /// toward a neighbour then depends on more than one connection, and exact
    /// culling would need a second hop.
    ArmTouchesOtherFace,
}

impl std::fmt::Display for ShapeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ShapeError::NoBoxes => "a Shape needs at least one box",
            ShapeError::TooManyBoxes => "a Shape has more than MAX_BOXES boxes",
            ShapeError::BadBox => "a box is outside the Voxel or has no volume",
            ShapeError::TableFull => "all 256 ShapeIds are taken",
            ShapeError::BadArm => "a connected Shape's arms are missing, repeated or empty",
            ShapeError::ConnectedSeals => "a connected Shape seals a face under some arms",
            ShapeError::ArmTouchesOtherFace => "an arm reaches a face other than its own",
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
    /// Index into `ShapeTable::connects`, or [`NO_CONNECT`]. A connected Shape's
    /// 24 slots are all its bare variant (no arms out), so `variant` and
    /// everything that reads only it - `seals`, `coverage`, `volume` - need no
    /// neighbour.
    connect: u16,
}

const NO_CONNECT: u16 = u16::MAX;

/// What an unknown Shape and air both resolve to: nothing there at all.
const NOTHING: Variant = Variant {
    seals: 0,
    coverage: [FaceMask::EMPTY; 6],
    quads: (0, 0),
    collision: (0, 0),
    selection: (0, 0),
    volume: 0,
    connect: NO_CONNECT,
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
    boxes: Vec<CollisionBox>,
    selection: Vec<NodeBox>,
    overhang: [u8; 6],
    /// Per Shape id: its groups, for the neighbour's side of the predicate.
    groups: Vec<u32>,
    connects: Vec<ConnectInfo>,
    /// Every connected Shape's variants, one per submask of its arm faces.
    mask_variants: Vec<Variant>,
}

/// What `connections` needs of one connected Shape.
#[derive(Clone, Copy, Debug)]
struct ConnectInfo {
    connects_to: u32,
    to_sealed: bool,
    /// The faces that have an arm.
    faces: u8,
    /// Where its variants start in `mask_variants`; indexed by the connections
    /// squeezed onto `faces`.
    first: u32,
    /// What it costs against [`MAX_SHAPED_PER_CHUNK`], in units of one stair.
    weight: u32,
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
            overhang: [0; 6],
            groups: Vec::new(),
            connects: Vec::new(),
            mask_variants: Vec::new(),
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
                    connect: None,
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
        let base_collision = resolve(def.collision.as_deref(), &def.boxes)?;
        if let Some(connect) = &def.connect {
            return self.register_connected(&def, &base_collision, connect);
        }

        let grid = rasterise(&def.boxes);
        for orientation in Orientation::all() {
            let variant = self.derive(&grid, &base_collision, &def.boxes, &orientation.matrix());
            self.variants.push(variant);
        }
        self.groups.push(0);
        self.shapes += 1;
        Ok((self.shapes - 1) as ShapeId)
    }

    /// One variant for every submask of the arms, from the one grid each time
    /// (the base plus the arms the mask selects), and the bare one in all 24
    /// orientation slots: a connected Shape cannot be rotated.
    fn register_connected(
        &mut self,
        def: &ShapeDef,
        base_collision: &[CollisionBox],
        connect: &ConnectDef,
    ) -> Result<ShapeId, ShapeError> {
        if self.connects.len() >= NO_CONNECT as usize {
            return Err(ShapeError::TableFull);
        }
        let mut faces = 0u8;
        let mut arms = Vec::new();
        for arm in &connect.arms {
            if arm.face >= 6 || faces >> arm.face & 1 != 0 || arm.boxes.is_empty() {
                return Err(ShapeError::BadArm);
            }
            faces |= 1 << arm.face;
            validate(&arm.boxes)?;
            let collision = resolve(arm.collision.as_deref(), &arm.boxes)?;
            if arm.boxes.iter().any(|b| reaches_other_plane(b, arm.face)) {
                return Err(ShapeError::ArmTouchesOtherFace);
            }
            arms.push((arm.face, &arm.boxes, collision));
        }
        if arms.is_empty() {
            return Err(ShapeError::BadArm);
        }
        let drawn: usize = def.boxes.len() + arms.iter().map(|a| a.1.len()).sum::<usize>();
        let bumped: usize = base_collision.len() + arms.iter().map(|a| a.2.len()).sum::<usize>();
        if drawn > MAX_BOXES || bumped > MAX_BOXES {
            return Err(ShapeError::TooManyBoxes);
        }

        let identity = Orientation::IDENTITY.matrix();
        let submasks: Vec<u8> = (0..1u16 << 6)
            .map(|m| m as u8)
            .filter(|m| m & !faces == 0)
            .collect();
        // Index `compact(mask)` is the mask's bits squeezed onto the arm faces,
        // which is ascending submask order of `faces` too.
        let mut derived = Vec::new();
        for &mask in &submasks {
            let mut boxes = def.boxes.clone();
            let mut collision = base_collision.to_vec();
            for (face, drawn, bumped) in &arms {
                if mask >> face & 1 != 0 {
                    boxes.extend_from_slice(drawn);
                    collision.extend_from_slice(bumped);
                }
            }
            let grid = rasterise(&boxes);
            if (0..6).any(|f| face_coverage(&grid, f).is_full()) {
                return Err(ShapeError::ConnectedSeals);
            }
            derived.push((grid, boxes, collision));
        }

        let first = self.mask_variants.len() as u32;
        let index = self.connects.len() as u16;
        let mut by_compact = vec![None; submasks.len()];
        for (&mask, (grid, boxes, collision)) in submasks.iter().zip(&derived) {
            by_compact[compact(mask, faces)] = Some(self.derive(grid, collision, boxes, &identity));
        }
        self.mask_variants.extend(
            by_compact
                .into_iter()
                .map(|v| v.expect("every submask is derived")),
        );
        // The cap was sized for stairs, and a fully armed fence emits several
        // times a stair's quads, so a connected Shape is charged for the most it
        // can emit: a Chunk at the cap then meshes about as fast whatever fills
        // it.
        let worst = self.mask_variants[first as usize..]
            .iter()
            .map(|v| v.quads.1)
            .max()
            .unwrap_or(0);
        let stair = self.variants[SHAPE_STAIR as usize * ORIENTATIONS]
            .quads
            .1
            .max(1);
        self.connects.push(ConnectInfo {
            connects_to: connect.connects_to,
            to_sealed: connect.to_sealed,
            faces,
            first,
            weight: worst.div_ceil(stair),
        });

        let bare = Variant {
            connect: index,
            ..self.mask_variants[first as usize]
        };
        self.variants
            .extend(std::iter::repeat_n(bare, ORIENTATIONS));
        self.groups.push(connect.group);
        self.shapes += 1;
        Ok((self.shapes - 1) as ShapeId)
    }

    /// Everything one (Shape, Orientation) or (Shape, connections) asks of the
    /// table, from one grid rotated by `m`, appended to the shared arrays.
    fn derive(
        &mut self,
        grid: &Grid,
        collision: &[CollisionBox],
        drawn: &[NodeBox],
        m: &Matrix,
    ) -> Variant {
        let rotated = rotate_grid(grid, m);

        let quads_start = self.quads.len() as u32;
        extract_quads(&rotated, &mut self.quads);
        let boxes_start = self.boxes.len() as u32;
        for b in collision {
            let rotated = rotate_collision_box(b, m);
            self.fold_overhang(&rotated);
            self.boxes.push(rotated);
        }
        let selection_start = self.selection.len() as u32;
        self.selection.extend(drawn.iter().map(|b| {
            let r = rotate_collision_box(&(*b).into(), m);
            NodeBox::new(r.min.map(|c| c as u8), r.max.map(|c| c as u8))
        }));

        let coverage: [FaceMask; 6] = std::array::from_fn(|f| face_coverage(&rotated, f));
        let seals = (0..6)
            .filter(|&f| coverage[f].is_full())
            .fold(0u8, |acc, f| acc | 1 << f);
        Variant {
            seals,
            coverage,
            quads: (quads_start, self.quads.len() as u32 - quads_start),
            collision: (boxes_start, self.boxes.len() as u32 - boxes_start),
            selection: (
                selection_start,
                self.selection.len() as u32 - selection_start,
            ),
            volume: rotated.iter().filter(|&&set| set).count() as u16,
            connect: NO_CONNECT,
        }
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

    /// The variant for `voxel` with the arms in `c` out. A Shape that is not
    /// connected has one variant whatever `c` says.
    #[inline]
    fn variant_with(&self, voxel: Voxel, c: Connections) -> &Variant {
        let bare = self.variant(voxel);
        if bare.connect == NO_CONNECT {
            return bare;
        }
        let info = &self.connects[bare.connect as usize];
        &self.mask_variants[info.first as usize + compact(c.0 & info.faces, info.faces)]
    }

    /// The Voxel's exposed surface in sixteenths, ready to emit.
    pub fn quads(&self, voxel: Voxel, c: Connections) -> &[ShapeQuad] {
        let (start, len) = self.variant_with(voxel, c).quads;
        &self.quads[start as usize..(start + len) as usize]
    }

    /// What a body collides with, rotated, in sixteenths.
    pub fn collision(&self, voxel: Voxel, c: Connections) -> &[CollisionBox] {
        let (start, len) = self.variant_with(voxel, c).collision;
        &self.boxes[start as usize..(start + len) as usize]
    }

    /// What `voxel` costs against [`MAX_SHAPED_PER_CHUNK`]: nothing for air and
    /// cubes, one for a Shape, and for a connected Shape as many as it has times
    /// a stair's quads, at its most armed.
    pub fn cap_units(&self, voxel: Voxel) -> u32 {
        if !voxel.is_shaped() {
            return 0;
        }
        match self.variant(voxel).connect {
            NO_CONNECT => 1,
            i => self.connects[i as usize].weight,
        }
    }

    /// Whether `voxel` grows arms towards its neighbours at all.
    #[inline]
    pub fn is_connected(&self, voxel: Voxel) -> bool {
        self.variant(voxel).connect != NO_CONNECT
    }

    /// Which arms `voxel` has out, given the Voxels across its six faces in
    /// face-constant order. The one rule the mesher, the sweep and the ray all
    /// ask, so that a fence is never drawn with an arm you walk through.
    ///
    /// An arm grows towards a neighbour whose groups meet `connects_to`, or, for
    /// a Shape that asks, one that seals the face looking back at us. Air, an
    /// unknown Shape and a Voxel the caller does not hold (pass air) belong to
    /// no group and seal nothing, so never connect.
    pub fn connections(&self, voxel: Voxel, around: [Voxel; 6]) -> Connections {
        let bare = self.variant(voxel);
        if bare.connect == NO_CONNECT {
            return Connections::NONE;
        }
        let info = &self.connects[bare.connect as usize];
        let mut c = Connections::NONE;
        for (face, &across) in around.iter().enumerate() {
            if info.faces >> face & 1 != 0 && self.connects_through(info, across, face) {
                c = c.with(face);
            }
        }
        c
    }

    fn connects_through(&self, info: &ConnectInfo, across: Voxel, face: usize) -> bool {
        self.groups_of(across) & info.connects_to != 0
            || (info.to_sealed && self.seals(across, face ^ 1))
    }

    fn groups_of(&self, voxel: Voxel) -> u32 {
        let shape = voxel.shape() as usize;
        if voxel.is_air() || shape >= self.shapes {
            return 0;
        }
        self.groups[shape]
    }

    /// The coverage of `voxel`'s `face` when `other` is the Voxel across it: the
    /// bare coverage plus the arm's end if it has one out that way.
    ///
    /// Exact with one neighbour, not two, because an arm touches only its own
    /// face's plane: whether it is out depends on the Voxel across that face and
    /// nothing else, and the mesher is already holding that Voxel.
    pub fn coverage_against(&self, voxel: Voxel, face: usize, other: Voxel) -> &FaceMask {
        let bare = self.variant(voxel);
        if bare.connect == NO_CONNECT {
            return self.coverage(voxel, face);
        }
        let info = &self.connects[bare.connect as usize];
        let out = info.faces >> face & 1 != 0 && self.connects_through(info, other, face);
        let c = if out {
            Connections::NONE.with(face)
        } else {
            Connections::NONE
        };
        &self.variant_with(voxel, c).coverage[face]
    }

    /// How far any registered collision box reaches past each face of its cell,
    /// in whole Voxels (0 or 1), indexed by face constant. A sweep has to look
    /// that many cells further to find a box that belongs to a neighbour; all
    /// zero, and so free, until a Shape overhangs.
    pub fn overhang(&self) -> [u8; 6] {
        self.overhang
    }

    fn fold_overhang(&mut self, b: &CollisionBox) {
        let cell = SUBDIV as i8;
        for a in 0..3 {
            if b.min[a] < 0 {
                self.overhang[2 * a] = 1;
            }
            if b.max[a] > cell {
                self.overhang[2 * a + 1] = 1;
            }
        }
    }

    /// What a ray picks: the declared boxes, rotated. Not [`ShapeTable::collision`],
    /// which a Shape may make larger, smaller or empty without changing what is
    /// drawn, and a player aims at what is drawn.
    pub fn selection(&self, voxel: Voxel, c: Connections) -> &[NodeBox] {
        let (start, len) = self.variant_with(voxel, c).selection;
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

/// A declared collision list, or the drawn boxes as collision. Empty is allowed:
/// it is how a Shape is walked through.
fn resolve(
    collision: Option<&[CollisionBox]>,
    boxes: &[NodeBox],
) -> Result<Vec<CollisionBox>, ShapeError> {
    match collision {
        Some(collision) => {
            if collision.len() > MAX_BOXES {
                return Err(ShapeError::TooManyBoxes);
            }
            collision.iter().try_for_each(check_collision_box)?;
            Ok(collision.to_vec())
        }
        None => Ok(boxes.iter().map(|&b| b.into()).collect()),
    }
}

/// Whether `b` touches a boundary plane of the cell other than `face`'s own.
fn reaches_other_plane(b: &NodeBox, face: usize) -> bool {
    let cell = SUBDIV as u8;
    (0..6).any(|plane| {
        let (axis, positive) = (plane / 2, plane % 2 == 1);
        let touches = if positive {
            b.max[axis] == cell
        } else {
            b.min[axis] == 0
        };
        touches && plane != face
    })
}

/// `mask`'s bits squeezed onto the set bits of `faces`, so a connected Shape with
/// four arms keeps 16 variants and not 64.
fn compact(mask: u8, faces: u8) -> usize {
    let mut out = 0;
    let mut k = 0;
    for f in 0..6 {
        if faces >> f & 1 != 0 {
            out |= ((mask >> f & 1) as usize) << k;
            k += 1;
        }
    }
    out
}

fn check_collision_box(b: &CollisionBox) -> Result<(), ShapeError> {
    let cell = SUBDIV as i8;
    let ok = (0..3).all(|a| b.min[a] < b.max[a] && b.min[a] >= -cell && b.max[a] <= 2 * cell);
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
/// centre is 8 and the arithmetic is the same without the doubling. Signed, so a
/// box standing outside the cell rotates onto the right axis.
fn rotate_collision_box(b: &CollisionBox, m: &Matrix) -> CollisionBox {
    let half = SUBDIV as i32 / 2;
    let lo = apply(m, b.min.map(|c| c as i32 - half));
    let hi = apply(m, b.max.map(|c| c as i32 - half));
    CollisionBox {
        min: std::array::from_fn(|a| (lo[a].min(hi[a]) + half) as i8),
        max: std::array::from_fn(|a| (lo[a].max(hi[a]) + half) as i8),
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
                let collision: Vec<NodeBox> = t
                    .collision(v, Connections::NONE)
                    .iter()
                    .map(|b| NodeBox::new(b.min.map(|c| c as u8), b.max.map(|c| c as u8)))
                    .collect();
                assert_eq!(
                    t.selection(v, Connections::NONE),
                    collision,
                    "shape {shape} {o:?}"
                );
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
                let sum: u32 = t
                    .quads(v, Connections::NONE)
                    .iter()
                    .map(ShapeQuad::area)
                    .sum();
                assert_eq!(sum, area, "shape {shape} orientation {o:?}");
            }
        }
    }

    #[test]
    fn a_boundary_quad_always_faces_outwards() {
        let t = ShapeTable::builtin();
        for shape in 0..t.len() as ShapeId {
            for o in Orientation::all() {
                for q in t.quads(Voxel::new(1, shape, o.bits()), Connections::NONE) {
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
        let slab_top = t.collision(voxel(SHAPE_SLAB, NEG_Y, 0), Connections::NONE);
        assert_eq!(slab_top, [CollisionBox::new([0, 8, 0], [16, 16, 16])]);
        for shape in 0..t.len() as ShapeId {
            for o in Orientation::all() {
                for b in t.collision(Voxel::new(1, shape, o.bits()), Connections::NONE) {
                    assert!(
                        (0..3).all(|a| b.min[a] < b.max[a] && b.max[a] <= 16),
                        "{b:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn collision_may_stand_one_voxel_outside_the_cell_and_no_further() {
        let mut t = ShapeTable::builtin();
        let tall = |min, max| ShapeDef {
            connect: None,
            boxes: vec![NodeBox::new([6, 0, 6], [10, 16, 10])],
            collision: Some(vec![CollisionBox::new(min, max)]),
        };
        assert!(t.register(tall([6, -16, 6], [10, 32, 10])).is_ok());
        assert_eq!(
            t.register(tall([6, -17, 6], [10, 16, 10])),
            Err(ShapeError::BadBox)
        );
        assert_eq!(
            t.register(tall([6, 0, 6], [10, 33, 10])),
            Err(ShapeError::BadBox)
        );
        assert_eq!(
            t.register(tall([6, 4, 6], [10, 4, 10])),
            Err(ShapeError::BadBox)
        );
    }

    use super::examples::{FENCE_GROUP, fence, pipe};

    fn fence_table() -> (ShapeTable, Voxel, Voxel) {
        let mut t = ShapeTable::builtin();
        let f = t.register(fence()).unwrap();
        let p = t.register(pipe()).unwrap();
        (t, Voxel::new(1, f, 0), Voxel::new(1, p, 0))
    }

    fn bits(faces: &[usize]) -> Connections {
        faces.iter().fold(Connections::NONE, |c, &f| c.with(f))
    }

    /// The Voxel across each face, in face order.
    fn around(across: &[(usize, Voxel)]) -> [Voxel; 6] {
        let mut v = [Voxel::AIR; 6];
        for &(face, voxel) in across {
            v[face] = voxel;
        }
        v
    }

    #[test]
    fn a_fence_has_an_arm_exactly_where_its_connections_say_in_every_list() {
        let (t, fence, _) = fence_table();
        let arm = |face: usize| -> NodeBox {
            match face {
                f if f == POS_X => NodeBox::new([10, 6, 7], [16, 9, 9]),
                f if f == NEG_X => NodeBox::new([0, 6, 7], [6, 9, 9]),
                f if f == POS_Z => NodeBox::new([7, 6, 10], [9, 9, 16]),
                _ => NodeBox::new([7, 6, 0], [9, 9, 6]),
            }
        };
        let arms = [NEG_X, POS_X, NEG_Z, POS_Z];
        for mask in 0..16u8 {
            let out: Vec<usize> = arms
                .iter()
                .enumerate()
                .filter(|(i, _)| mask >> i & 1 != 0)
                .map(|(_, &f)| f)
                .collect();
            let c = bits(&out);
            let drawn = t.selection(fence, c);
            assert_eq!(drawn.len(), 1 + 2 * out.len(), "mask {mask}");
            for &f in &arms {
                assert_eq!(
                    drawn.contains(&arm(f)),
                    out.contains(&f),
                    "mask {mask} face {f}"
                );
            }
            assert_eq!(t.collision(fence, c).len(), 1 + out.len(), "mask {mask}");
            // More arms means more surface, and none means the bare post's.
            let area: u32 = t.quads(fence, c).iter().map(ShapeQuad::area).sum();
            let bare: u32 = t
                .quads(fence, Connections::NONE)
                .iter()
                .map(ShapeQuad::area)
                .sum();
            assert_eq!(area > bare, !out.is_empty(), "mask {mask}");
        }
    }

    #[test]
    fn a_fence_connects_to_its_group_to_what_seals_towards_it_and_to_nothing_else() {
        let (t, fence, pipe) = fence_table();
        let cube = Voxel::cube(1);
        let slab = Voxel::shaped(1, SHAPE_SLAB, Orientation::new(POS_Y, 0));
        let unknown = Voxel::new(1, 200, 0);

        // Fence beside fence.
        assert_eq!(
            t.connections(fence, around(&[(POS_X, fence)])),
            bits(&[POS_X])
        );
        // A different group does not.
        assert_eq!(
            t.connections(fence, around(&[(POS_X, pipe)])),
            Connections::NONE
        );
        // A cube seals every face, so a fence meets a wall.
        assert_eq!(
            t.connections(fence, around(&[(NEG_Z, cube)])),
            bits(&[NEG_Z])
        );
        // A bottom slab seals only its underside, so a fence beside one does not
        // meet it.
        assert_eq!(
            t.connections(fence, around(&[(POS_X, slab), (POS_Z, slab)])),
            Connections::NONE
        );
        // Air and an unknown Shape connect to nothing.
        assert_eq!(
            t.connections(fence, around(&[(POS_X, Voxel::AIR), (NEG_X, unknown)])),
            Connections::NONE
        );
        // All four at once; the vertical faces have no arm to grow.
        let all = around(&[
            (NEG_X, fence),
            (POS_X, fence),
            (NEG_Z, cube),
            (POS_Z, fence),
            (POS_Y, fence),
            (NEG_Y, cube),
        ]);
        assert_eq!(
            t.connections(fence, all),
            bits(&[NEG_X, POS_X, NEG_Z, POS_Z])
        );
        // A Voxel that is not connected asks nothing of its neighbours.
        assert_eq!(t.connections(cube, all), Connections::NONE);
        assert!(t.is_connected(fence) && !t.is_connected(cube));
    }

    #[test]
    fn the_rule_is_asked_of_each_side_so_it_can_be_one_way() {
        let mut t = ShapeTable::builtin();
        let mut def = fence();
        // This fence reaches for pipes; pipes do not reach back.
        def.connect.as_mut().unwrap().connects_to = super::examples::PIPE_GROUP;
        let one_way = Voxel::new(1, t.register(def).unwrap(), 0);
        let pipe = Voxel::new(1, t.register(pipe()).unwrap(), 0);
        assert_eq!(
            t.connections(one_way, around(&[(POS_X, pipe)])),
            bits(&[POS_X])
        );
        assert_eq!(
            t.connections(pipe, around(&[(NEG_X, one_way)])),
            Connections::NONE
        );
        let _ = FENCE_GROUP;
    }

    #[test]
    fn a_connected_voxel_ignores_its_orientation_byte() {
        let (t, fence, _) = fence_table();
        let c = bits(&[POS_X, NEG_Z]);
        for byte in [0u8, 1, 5, 23, 24, 200, 255] {
            let turned = Voxel::new(1, fence.shape(), byte);
            assert_eq!(t.quads(turned, c), t.quads(fence, c), "byte {byte}");
            assert_eq!(t.selection(turned, c), t.selection(fence, c), "byte {byte}");
            assert_eq!(t.collision(turned, c), t.collision(fence, c), "byte {byte}");
        }
    }

    #[test]
    fn a_connected_shape_seals_nothing_and_blocks_no_light_whatever_its_arms() {
        let (t, fence, pipe) = fence_table();
        for v in [fence, pipe] {
            assert_eq!(t.light_mask(v), 0);
            assert!((0..6).all(|f| !t.seals(v, f)));
        }
    }

    #[test]
    fn coverage_against_a_neighbour_is_the_coverage_with_just_that_arm_out() {
        let (t, fence, pipe) = fence_table();
        let cube = Voxel::cube(1);
        for (voxel, other_connecting, other_not) in [(fence, fence, pipe), (pipe, pipe, fence)] {
            for face in 0..6 {
                let bare = *t.coverage(voxel, face);
                let with_arm = t.variant_with(voxel, bits(&[face])).coverage[face];
                let has_arm = t.connects[t.variant(voxel).connect as usize].faces >> face & 1 != 0;
                assert_eq!(*t.coverage_against(voxel, face, other_not), bare);
                assert_eq!(*t.coverage_against(voxel, face, Voxel::AIR), bare);
                let expected = if has_arm { with_arm } else { bare };
                assert_eq!(*t.coverage_against(voxel, face, other_connecting), expected);
                assert!(
                    t.coverage_against(voxel, face, other_connecting)
                        .covers(&bare)
                );
            }
        }
        // An arm's end is a small patch, not the whole face.
        let end = t.coverage_against(fence, POS_X, fence);
        assert!(!end.is_empty() && !end.is_full());
        // To a wall the same.
        assert_eq!(*t.coverage_against(fence, POS_X, cube), *end);
    }

    #[test]
    fn volume_is_the_bare_variants_whatever_the_arms() {
        let (t, fence, _) = fence_table();
        // 4 x 16 x 4 sub-cells of post, and LOD is not told about rails.
        assert_eq!(t.volume(fence), 4 * 16 * 4);
    }

    #[test]
    fn a_connected_shape_is_charged_against_the_cap_for_its_most_armed_self() {
        let (t, fence, pipe) = fence_table();
        let stair = Voxel::shaped(1, SHAPE_STAIR, Orientation::IDENTITY);
        assert_eq!(t.cap_units(Voxel::AIR), 0);
        assert_eq!(t.cap_units(Voxel::cube(1)), 0);
        assert_eq!(t.cap_units(stair), 1);
        assert_eq!(t.cap_units(Voxel::new(1, SHAPE_POST, 0)), 1);

        let stair_quads = t.quads(stair, Connections::NONE).len();
        for (voxel, all) in [
            (fence, bits(&[NEG_X, POS_X, NEG_Z, POS_Z])),
            (pipe, bits(&[0, 1, 2, 3, 4, 5])),
        ] {
            let worst = t.quads(voxel, all).len();
            let units = t.cap_units(voxel) as usize;
            assert!(units > 1, "{units}");
            assert!(
                units * stair_quads >= worst,
                "{units} stairs of {stair_quads} < {worst}"
            );
            assert!(
                (units - 1) * stair_quads < worst,
                "{units} is more than it needs"
            );
        }
    }

    #[test]
    fn registration_rejects_a_connected_shape_that_cannot_be_trusted() {
        let mut t = ShapeTable::builtin();
        let mutate = |f: &dyn Fn(&mut ConnectDef)| {
            let mut def = fence();
            f(def.connect.as_mut().unwrap());
            def
        };
        assert_eq!(
            t.register(mutate(&|c| c.arms.clear())),
            Err(ShapeError::BadArm)
        );
        assert_eq!(
            t.register(mutate(&|c| c.arms[0].face = 6)),
            Err(ShapeError::BadArm)
        );
        assert_eq!(
            t.register(mutate(&|c| c.arms[1].face = c.arms[0].face)),
            Err(ShapeError::BadArm)
        );
        assert_eq!(
            t.register(mutate(&|c| c.arms[0].boxes.clear())),
            Err(ShapeError::BadArm)
        );
        assert_eq!(
            t.register(mutate(
                &|c| c.arms[0].boxes[0] = NodeBox::new([0, 0, 0], [17, 4, 4])
            )),
            Err(ShapeError::BadBox)
        );
        // An arm that also reaches the floor, which is not its face.
        assert_eq!(
            t.register(mutate(
                &|c| c.arms[0].boxes[0] = NodeBox::new([0, 0, 7], [6, 9, 9])
            )),
            Err(ShapeError::ArmTouchesOtherFace)
        );
        // Sixteen boxes is the bound for a Shape, counting every arm.
        assert_eq!(
            t.register(mutate(&|c| {
                let extra = c.arms[0].boxes[0];
                c.arms[0].boxes.extend(std::iter::repeat_n(extra, 8));
            })),
            Err(ShapeError::TooManyBoxes)
        );
        // Four arms are fine, and nothing was registered by the failures.
        assert_eq!(t.len(), SHAPE_POST as usize + 1);
        assert!(t.register(fence()).is_ok());
    }

    #[test]
    fn a_connected_shape_whose_arms_would_seal_a_face_is_rejected() {
        let mut t = ShapeTable::builtin();
        // A frame around the +X face with a hole in the middle, and an arm that
        // fills the hole: sealed only when the arm is out, which is the case a
        // check of the bare Shape would miss.
        let mut def = fence();
        def.boxes = vec![
            NodeBox::new([12, 0, 0], [16, 16, 6]),
            NodeBox::new([12, 0, 10], [16, 16, 16]),
            NodeBox::new([12, 0, 6], [16, 6, 10]),
            NodeBox::new([12, 10, 6], [16, 16, 10]),
        ];
        def.collision = None;
        def.connect.as_mut().unwrap().arms[1].boxes = vec![NodeBox::new([10, 6, 6], [16, 10, 10])];
        assert_eq!(t.register(def), Err(ShapeError::ConnectedSeals));
        // A base that seals is rejected the same way.
        let mut def = pipe();
        def.boxes = vec![NodeBox::new([0, 0, 0], [16, 16, 16])];
        assert_eq!(t.register(def), Err(ShapeError::ConnectedSeals));
        assert_eq!(t.len(), SHAPE_POST as usize + 1);
    }

    #[test]
    fn a_shape_can_be_declared_thin_to_look_at_and_nothing_to_walk_into() {
        let mut t = ShapeTable::builtin();
        let plate = t
            .register(ShapeDef {
                connect: None,
                boxes: vec![NodeBox::new([1, 0, 1], [15, 1, 15])],
                collision: Some(vec![]),
            })
            .unwrap();
        let v = Voxel::new(1, plate, 0);
        assert!(t.collision(v, Connections::NONE).is_empty());
        assert!(!t.quads(v, Connections::NONE).is_empty());
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
                connect: None,
                boxes: vec![
                    NodeBox::new([0, 0, 0], [16, 8, 16]),
                    NodeBox::new([0, 4, 0], [16, 16, 16]),
                ],
                collision: None,
            })
            .unwrap();
        let sum: u32 = t
            .quads(Voxel::new(1, tee, 0), Connections::NONE)
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
            connect: None,
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
            assert!(
                t.quads(v, Connections::NONE).is_empty()
                    && t.collision(v, Connections::NONE).is_empty()
            );
            assert_eq!((t.volume(v), t.light_mask(v)), (0, 0));
        }
    }

    #[test]
    fn an_invalid_orientation_byte_reads_as_the_identity() {
        let t = ShapeTable::builtin();
        let wild = Voxel::new(1, SHAPE_STAIR, 200);
        let upright = Voxel::new(1, SHAPE_STAIR, 0);
        assert_eq!(
            t.quads(wild, Connections::NONE),
            t.quads(upright, Connections::NONE)
        );
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
