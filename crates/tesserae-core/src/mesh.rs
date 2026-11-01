//! Meshing in two passes. Pass 1 greedy-merges coplanar faces of identical full
//! cubes into quads; pass 2 gives each shaped Voxel the geometry of its own
//! Shape, then merges the quads that abut across Voxels. Fluid standing below a
//! full surface takes a third and fluid in a Shape's voids a fourth, only in a
//! Chunk whose overlay holds fluid. A burning solid's flames take a fifth.

use std::collections::{HashMap, HashSet};

use crate::chunk::{CHUNK_SIZE, Chunk};
use crate::light::{ChunkLight, Light};
use crate::material::{MaterialTable, Occlusion, RenderClass};
use crate::occupants::{ChunkOverlay, Fluid};
use crate::shape::{Connections, FaceMask, SUBDIV, ShapeQuad};
use crate::tables::Tables;
use crate::voxel::{MaterialId, Voxel};

/// Chunk edge plus one Voxel of neighbour on each side — exactly what face
/// culling needs.
pub const PADDED: usize = CHUNK_SIZE + 2;

const PADDED_VOLUME: usize = PADDED * PADDED * PADDED;
/// How far [`pad_index`] moves for one step along x, y and z.
const STRIDES: [usize; 3] = [1, PADDED * PADDED, PADDED];

/// Face index into [`Neighborhood::new`]'s neighbour array.
pub const NEG_X: usize = 0;
pub const POS_X: usize = 1;
pub const NEG_Y: usize = 2;
pub const POS_Y: usize = 3;
pub const NEG_Z: usize = 4;
pub const POS_Z: usize = 5;

/// The Material index a flame quad carries: past every [`MaterialId`], so a
/// renderer tells flames from any Material without a Look pack.
pub const FLAME: u32 = MaterialId::MAX as u32 + 1;

/// How far outside a burning Voxel's face its flame is drawn, in Voxels: off
/// the face, so the two never fight for depth.
const FLAME_OFFSET: f32 = 1.0 / SUBDIV as f32;

/// Which way each face looks, as the offset to the neighbour across it.
pub const FACE_STEPS: [[i32; 3]; 6] = [
    [-1, 0, 0], // NEG_X
    [1, 0, 0],  // POS_X
    [0, -1, 0], // NEG_Y
    [0, 1, 0],  // POS_Y
    [0, 0, -1], // NEG_Z
    [0, 0, 1],  // POS_Z
];

/// A Chunk and its light, which the mesher always needs together.
///
/// One parameter instead of two parallel arrays, so [`Neighborhood::new`] and
/// [`mesh_chunk`] keep two arguments rather than four — four of which the
/// workspace's `-D warnings` would reject as `clippy::too_many_arguments`.
#[derive(Clone, Copy)]
pub struct LitChunk<'a> {
    pub chunk: &'a Chunk,
    pub light: &'a ChunkLight,
    /// Fluid beside the Voxels, which has geometry of its own.
    pub overlay: &'a ChunkOverlay,
    /// Read only by [`mesh_chunk_with_seams`].
    pub openings: &'a SeamOpenings,
}

static DRY: ChunkOverlay = ChunkOverlay::none();
static EVERYWHERE: SeamOpenings = SeamOpenings::everywhere();

impl<'a> LitChunk<'a> {
    pub fn new(chunk: &'a Chunk, light: &'a ChunkLight) -> Self {
        LitChunk {
            chunk,
            light,
            overlay: &DRY,
            openings: &EVERYWHERE,
        }
    }

    pub fn with_overlay(self, overlay: &'a ChunkOverlay) -> Self {
        LitChunk { overlay, ..self }
    }

    pub fn with_openings(self, openings: &'a SeamOpenings) -> Self {
        LitChunk { openings, ..self }
    }
}

/// A seam plane every half Chunk: where a finer Chunk, half a coarse one, can end.
pub(crate) const SEAM_EVERY: usize = CHUNK_SIZE / 2;

/// The Voxels of a Chunk against a seam plane, along the axis across it.
pub(crate) const SEAM_LAYERS: [usize; 4] = [0, SEAM_EVERY - 1, SEAM_EVERY, CHUNK_SIZE - 1];

const _: () = assert!(
    CHUNK_SIZE == u32::BITS as usize,
    "a row of openings is one u32"
);

/// Where a coarse Chunk's finer ground leaves a Voxel's side open on a seam
/// plane: the Voxels against a plane across x or z whose finer Voxels against
/// it are not all opaque cubes. Only there can a seam wall across the plane be
/// seen, once the finer level ends on it and draws that ground in its place.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SeamOpenings {
    /// By axis (x, z), layer of [`SEAM_LAYERS`] and y; a bit per Voxel along
    /// the other horizontal axis.
    rows: [u32; 2 * SEAM_LAYERS.len() * CHUNK_SIZE],
}

impl SeamOpenings {
    /// What a Chunk without finer ground to read has: every wall is kept.
    pub const fn everywhere() -> Self {
        SeamOpenings {
            rows: [u32::MAX; 2 * SEAM_LAYERS.len() * CHUNK_SIZE],
        }
    }

    pub const fn nowhere() -> Self {
        SeamOpenings {
            rows: [0; 2 * SEAM_LAYERS.len() * CHUNK_SIZE],
        }
    }

    /// `axis` is 0 for x or 2 for z; `cell` must lie against a seam plane
    /// across it.
    pub fn set_open(&mut self, axis: usize, cell: [usize; 3]) {
        let (row, bit) = Self::slot(axis, cell).expect("a Voxel against a seam plane");
        self.rows[row] |= bit;
    }

    /// False for a Voxel against no seam plane across `axis`.
    pub fn is_open(&self, axis: usize, cell: [usize; 3]) -> bool {
        Self::slot(axis, cell).is_some_and(|(row, bit)| self.rows[row] & bit != 0)
    }

    fn slot(axis: usize, cell: [usize; 3]) -> Option<(usize, u32)> {
        debug_assert!(axis == 0 || axis == 2);
        let layer = SEAM_LAYERS.iter().position(|&l| l == cell[axis])?;
        let row = (axis / 2 * SEAM_LAYERS.len() + layer) * CHUNK_SIZE + cell[1];
        Some((row, 1 << cell[2 - axis]))
    }
}

/// A Chunk plus the one-Voxel shell of its six face neighbours, with the light
/// of both.
///
/// Without the shell, every Chunk emits a full skin of faces at its boundary —
/// wrong, and the dominant cost in a streamed world. Edge and corner cells of
/// the shell are left as air and are never read: face visibility only ever looks
/// at the Voxel directly across a face, and so does the light a face is lit by.
pub struct Neighborhood {
    voxels: Box<[Voxel]>,
    light: Box<[Light]>,
    /// Fluid below a full surface, by cell, as its Material and surface. Empty
    /// unless one of the seven Chunks holds fluid in its overlay; a full one is
    /// in `voxels` as a cube of its Material, which is how pass 1 draws water.
    surfaces: Box<[Option<(MaterialId, u8)>]>,
    /// Fluid in the voids of a solid, by cell: drawn clipped to those voids,
    /// and what hides fluid faces beside them.
    logged: HashMap<usize, Fluid>,
    /// Cells `voxels` holds full-surface flow in as a cube, whose packed Voxel
    /// is air: what a connected Shape's arms are read from.
    flowed: HashSet<usize>,
    /// Burning solids of the centre, in coordinate order.
    burning: Vec<[i32; 3]>,
    shaped: bool,
    /// The palettes of all seven Chunks, so the translucency gate is a scan of
    /// tens of entries and not of the 39 thousand Voxels copied above.
    kinds: Vec<Voxel>,
}

impl Neighborhood {
    pub fn new(center: LitChunk<'_>, neighbors: [Option<LitChunk<'_>>; 6]) -> Self {
        let mut voxels = vec![Voxel::AIR; PADDED_VOLUME].into_boxed_slice();
        let mut light = vec![Light::DARK; PADDED_VOLUME].into_boxed_slice();
        let last = CHUNK_SIZE - 1;

        // By rows: read a Voxel at a time, the optimiser stopped hoisting the
        // palette's match out of this loop and it cost a third of the build.
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let i = pad_index(0, y as i32, z as i32);
                center.chunk.read_row(y, z, &mut voxels[i..]);
                center.light.read_row(y, z, &mut light[i..]);
            }
        }

        // The six faces, as (which neighbour, where it lands in the shell,
        // which of its own cells to read).
        for a in 0..CHUNK_SIZE {
            for b in 0..CHUNK_SIZE {
                let (ai, bi) = (a as i32, b as i32);
                let edge = CHUNK_SIZE as i32;
                let faces = [
                    (NEG_X, (-1, ai, bi), (last, a, b)),
                    (POS_X, (edge, ai, bi), (0, a, b)),
                    (NEG_Y, (ai, -1, bi), (a, last, b)),
                    (POS_Y, (ai, edge, bi), (a, 0, b)),
                    (NEG_Z, (ai, bi, -1), (a, b, last)),
                    (POS_Z, (ai, bi, edge), (a, b, 0)),
                ];
                for (face, shell, theirs) in faces {
                    let Some(n) = neighbors[face] else {
                        continue;
                    };
                    let i = pad_index(shell.0, shell.1, shell.2);
                    voxels[i] = n.chunk.get(theirs.0, theirs.1, theirs.2);
                    light[i] = n.light.get(theirs.0, theirs.1, theirs.2);
                }
            }
        }

        let seven = || std::iter::once(center).chain(neighbors.into_iter().flatten());
        let mut kinds: Vec<Voxel> = seven()
            .flat_map(|n| n.chunk.kinds().iter().copied())
            .collect();
        let wet = seven().any(|n| n.overlay.may_contain_fluid());
        let (surfaces, logged, flowed) = if wet {
            kinds.extend(seven().flat_map(|n| n.overlay.fluids().map(Voxel::cube)));
            wet_cells(&mut voxels, center, neighbors)
        } else {
            (Box::default(), HashMap::new(), HashSet::new())
        };

        let mut burning = Vec::new();
        if center.overlay.may_contain_burning() {
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        if center.overlay.get(x, y, z).burning.is_some() {
                            burning.push([x as i32, y as i32, z as i32]);
                        }
                    }
                }
            }
        }

        Neighborhood {
            voxels,
            light,
            surfaces,
            logged,
            flowed,
            burning,
            shaped: center.chunk.may_contain_shaped(),
            kinds,
        }
    }

    /// Whether any of the seven Chunks might hold a Voxel that is not air or
    /// opaque. The neighbours count: a pane of glass beside a stone Chunk fails
    /// to hide the stone's face, though the centre holds nothing clear.
    fn is_translucent(&self, materials: &MaterialTable) -> bool {
        self.kinds
            .iter()
            .any(|v| !v.is_air() && !materials.is_opaque(*v))
    }

    /// Treats everything outside the Chunk as air. For tests, benches, and the
    /// edge of generated world.
    pub fn isolated(center: LitChunk<'_>) -> Self {
        Self::new(center, [None; 6])
    }

    /// Whether the centre might hold a shaped Voxel, from its palette. Only the
    /// centre emits geometry, so a shaped Voxel in the shell is irrelevant here:
    /// it hides or does not hide a face, which pass 1 and pass 2 ask of `at`.
    pub fn may_contain_shaped(&self) -> bool {
        self.shaped
    }

    #[inline]
    pub fn at(&self, x: i32, y: i32, z: i32) -> Voxel {
        self.voxels[pad_index(x, y, z)]
    }

    #[inline]
    pub fn light_at(&self, x: i32, y: i32, z: i32) -> Light {
        self.light[pad_index(x, y, z)]
    }

    /// The Material and surface of fluid that fills less than its cell.
    #[inline]
    pub fn surface_at(&self, x: i32, y: i32, z: i32) -> Option<(MaterialId, u8)> {
        self.surfaces.get(pad_index(x, y, z)).copied().flatten()
    }

    /// The fluid in the voids of the solid at a cell.
    #[inline]
    pub fn logged_at(&self, x: i32, y: i32, z: i32) -> Option<Fluid> {
        if self.logged.is_empty() {
            return None;
        }
        self.logged.get(&pad_index(x, y, z)).copied()
    }

    /// The packed Voxel at a cell, which collision and the simulation read a
    /// connected Shape's arms from: air where [`Neighborhood::at`] has a cube of
    /// full flowing water.
    #[inline]
    fn packed_at(&self, x: i32, y: i32, z: i32) -> Voxel {
        let i = pad_index(x, y, z);
        if !self.flowed.is_empty() && self.flowed.contains(&i) {
            Voxel::AIR
        } else {
            self.voxels[i]
        }
    }

    /// The arms of the shaped Voxel at a centre cell, read from the shell.
    fn connections_at(&self, tables: &Tables, x: i32, y: i32, z: i32) -> Connections {
        let voxel = self.at(x, y, z);
        let inside = [x, y, z]
            .iter()
            .all(|&c| (0..CHUNK_SIZE as i32).contains(&c));
        if inside && tables.shapes.any_connected() && tables.shapes.is_connected(voxel) {
            tables.shapes.connections(
                voxel,
                FACE_STEPS.map(|d| self.packed_at(x + d[0], y + d[1], z + d[2])),
            )
        } else {
            Connections::NONE
        }
    }

    /// What of `face` of the waterlogged Voxel at a cell hides fluid `here`
    /// looking at it: its solid where its Material hides that fluid, and the
    /// voids its own fluid of the same Material fills to the top. A shell
    /// Voxel's voids are read without arms, whose neighbours this does not hold.
    fn fluid_cover(
        &self,
        tables: &Tables,
        at: [i32; 3],
        face: usize,
        here: Voxel,
        toward: Voxel,
    ) -> FaceMask {
        let [x, y, z] = at;
        let solid = self.at(x, y, z);
        let mut cover = if hides(&tables.materials, solid, here) {
            *tables.shapes.coverage_against(solid, face, toward)
        } else {
            FaceMask::EMPTY
        };
        if let Some(fluid) = self.logged_at(x, y, z)
            && fluid.material == here.material()
        {
            let arms = self.connections_at(tables, x, y, z);
            for (r, void) in tables.shapes.voids(solid, arms).iter().enumerate() {
                if fluid.level(r).surface >= Fluid::FULL_SURFACE {
                    cover = cover.union(&void.open[face]);
                }
            }
        }
        cover
    }
}

/// Puts the overlays' fluid into a [`Neighborhood`]'s cells: in empty space a
/// full surface as a cube of its Material and anything lower into the returned
/// surfaces, and fluid beside a solid into the returned waterlogged cells.
#[allow(clippy::type_complexity)]
fn wet_cells(
    voxels: &mut [Voxel],
    center: LitChunk<'_>,
    neighbors: [Option<LitChunk<'_>>; 6],
) -> (
    Box<[Option<(MaterialId, u8)>]>,
    HashMap<usize, Fluid>,
    HashSet<usize>,
) {
    let mut surfaces = vec![None; PADDED_VOLUME].into_boxed_slice();
    let mut logged = HashMap::new();
    let mut flowed = HashSet::new();
    let mut put = |i: usize, overlay: &ChunkOverlay, at: (usize, usize, usize)| {
        let Some(fluid) = overlay.get(at.0, at.1, at.2).fluid else {
            return;
        };
        if !voxels[i].is_air() {
            logged.insert(i, fluid);
            return;
        }
        if fluid.surface >= Fluid::FULL_SURFACE {
            voxels[i] = Voxel::cube(fluid.material);
            flowed.insert(i);
        } else {
            surfaces[i] = Some((fluid.material, fluid.surface));
        }
    };
    if center.overlay.may_contain_fluid() {
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let i = pad_index(x as i32, y as i32, z as i32);
                    put(i, center.overlay, (x, y, z));
                }
            }
        }
    }
    let last = CHUNK_SIZE - 1;
    let edge = CHUNK_SIZE as i32;
    for a in 0..CHUNK_SIZE {
        for b in 0..CHUNK_SIZE {
            let (ai, bi) = (a as i32, b as i32);
            let faces = [
                (NEG_X, (-1, ai, bi), (last, a, b)),
                (POS_X, (edge, ai, bi), (0, a, b)),
                (NEG_Y, (ai, -1, bi), (a, last, b)),
                (POS_Y, (ai, edge, bi), (a, 0, b)),
                (NEG_Z, (ai, bi, -1), (a, b, last)),
                (POS_Z, (ai, bi, edge), (a, b, 0)),
            ];
            for (face, shell, theirs) in faces {
                let Some(n) = neighbors[face] else {
                    continue;
                };
                if n.overlay.may_contain_fluid() {
                    put(pad_index(shell.0, shell.1, shell.2), n.overlay, theirs);
                }
            }
        }
    }
    (surfaces, logged, flowed)
}

#[inline]
fn pad_index(x: i32, y: i32, z: i32) -> usize {
    debug_assert!((-1..=CHUNK_SIZE as i32).contains(&x));
    debug_assert!((-1..=CHUNK_SIZE as i32).contains(&y));
    debug_assert!((-1..=CHUNK_SIZE as i32).contains(&z));
    ((y + 1) as usize * PADDED + (z + 1) as usize) * PADDED + (x + 1) as usize
}

/// One draw call's worth of triangle soup, in Voxel-index space.
#[derive(Default, Debug)]
pub struct MeshLayer {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub materials: Vec<u32>,
    /// The packed [`Light`] of the Voxel each vertex's face looks out at, one
    /// per vertex. Baked rather than sampled: a torch is a per-Voxel light
    /// level, and thousands of them as GPU point lights is not viable.
    pub light: Vec<u32>,
    pub indices: Vec<u32>,
}

impl MeshLayer {
    pub fn quad_count(&self) -> usize {
        self.indices.len() / 6
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }
}

/// Mesh one isolated Shape from read-only geometry, in unit Voxel space.
pub fn mesh_shape(quads: &[ShapeQuad], material: MaterialId) -> MeshLayer {
    let mut layer = MeshLayer::default();
    for quad in quads {
        push_merged_quad(
            &mut layer,
            &Pending::new(
                [0; 3],
                quad,
                RenderClass::Opaque,
                material,
                Light::new(15, 0),
            ),
        );
    }
    layer
}

/// Triangle soup for one Chunk, in Voxel-index space. The caller scales by the
/// World's Voxel size; nothing here is in metres.
///
/// A layer per [`RenderClass`]: blended faces need sorting and no depth write,
/// cutout ones a pass that may discard, and opaque ones must pay for neither.
/// A quad is one Material, so it is never split across them.
#[derive(Default, Debug)]
pub struct ChunkMesh {
    pub opaque: MeshLayer,
    pub blended: MeshLayer,
    pub cutout: MeshLayer,
    /// Shaped Voxels pass 2 gave geometry to, including those whose every face
    /// turned out to be hidden.
    pub shaped_voxels: u32,
}

impl ChunkMesh {
    pub fn quad_count(&self) -> usize {
        self.opaque.quad_count() + self.blended.quad_count() + self.cutout.quad_count()
    }

    pub fn is_empty(&self) -> bool {
        self.opaque.is_empty() && self.blended.is_empty() && self.cutout.is_empty()
    }

    #[inline]
    fn layer(&mut self, class: RenderClass) -> &mut MeshLayer {
        match class {
            RenderClass::Opaque => &mut self.opaque,
            RenderClass::Blend => &mut self.blended,
            RenderClass::Cutout => &mut self.cutout,
        }
    }
}

/// UVs span the merged quad so a tiling sampler repeats per Voxel rather than
/// stretching one texel across the whole run.
#[inline]
fn corner(d: usize, u: usize, v: usize, plane: f32, au: usize, bv: usize) -> [f32; 3] {
    let mut c = [0.0f32; 3];
    c[d] = plane;
    c[u] = au as f32;
    c[v] = bv as f32;
    c
}

/// Mesh one Chunk against its six face neighbours, declining without building
/// a [`Neighborhood`] where no face can be emitted.
///
/// `tables` is how a neighbour's Shape and Material are asked what they hide.
///
/// The padded copy costs 39 thousand Voxel writes before `greedy_mesh` sees
/// anything, and a streamed 3D ring asks for a mesh of mostly uniform Chunks.
/// A `None` neighbour means unknown, not air — the caller is expected to have
/// all six loaded, and [`Neighborhood::isolated`]'s "outside is air" is a
/// different question.
pub fn mesh_chunk(
    center: LitChunk<'_>,
    neighbors: [Option<LitChunk<'_>>; 6],
    tables: &Tables,
) -> ChunkMesh {
    mesh_with(center, neighbors, tables, false)
}

/// [`mesh_chunk`] for a coarse level, plus seam walls on every plane across x
/// and z half a Chunk apart: the planes where a finer level's region can end.
///
/// Coarse ground there stands up to a coarse Voxel or two above the finer
/// ground beside it, which is how downsampling rounds, and with coarse ground
/// on both sides of the plane neither has a face on it. Once the finer side is
/// discarded, the step shows the inside of the coarse ground. A seam wall is
/// that face, drawn anyway, on each Voxel whose neighbour across the plane is
/// within two Voxels of open air above it, deeper ones being hidden, and is
/// open on the plane in its Chunk's [`SeamOpenings`]. Elsewhere it lies just
/// under the surface, inside the ground, and the renderer discards it where a
/// finer level draws the Voxel it belongs to.
///
/// Not across y: a level's top or bottom meets the ground only for an Eye
/// about that level's vertical extent above or below it.
pub fn mesh_chunk_with_seams(
    center: LitChunk<'_>,
    neighbors: [Option<LitChunk<'_>>; 6],
    tables: &Tables,
) -> ChunkMesh {
    mesh_with(center, neighbors, tables, true)
}

fn mesh_with(
    center: LitChunk<'_>,
    neighbors: [Option<LitChunk<'_>>; 6],
    tables: &Tables,
    seams: bool,
) -> ChunkMesh {
    // A Chunk this declines has solid neighbours all round, so the only seam
    // walls it loses are those under the diagonal neighbour's corner row.
    if !center.overlay.may_contain_fluid()
        && emits_no_faces(center.chunk, neighbors, &tables.materials)
    {
        return ChunkMesh::default();
    }
    let n = Neighborhood::new(center, neighbors);
    let mut mesh = greedy_mesh(&n, tables);
    if seams {
        mesh_seams(&n, (center, neighbors), tables, &mut mesh);
    }
    mesh
}

/// How far below open air the Voxel across a seam may be and still get a wall:
/// the most downsampling raises coarse ground above finer ground on a cliff.
const SEAM_DEPTH: i32 = 2;

/// The seam walls of [`mesh_chunk_with_seams`], merged as pass 2's quads are.
fn mesh_seams(
    n: &Neighborhood,
    (center, neighbors): (LitChunk<'_>, [Option<LitChunk<'_>>; 6]),
    tables: &Tables,
    mesh: &mut ChunkMesh,
) {
    let edge = CHUNK_SIZE as i32;
    let mut pending = Vec::new();
    let mut row: [Option<(Pending, bool)>; CHUNK_SIZE] = [None; CHUNK_SIZE];
    for d in [0, 2] {
        let w_axis = 2 - d;
        for plane in (0..=edge).step_by(SEAM_EVERY) {
            // The Voxel on the plane's far side looks back across it, and the
            // one on its near side looks forward.
            for (face, own) in [(d * 2, plane), (d * 2 + 1, plane - 1)] {
                if !(0..edge).contains(&own) {
                    continue;
                }
                let positive = face % 2 == 1;
                let quad = ShapeQuad {
                    face: face as u8,
                    plane: if positive { SUBDIV as u8 } else { 0 },
                    u0: 0,
                    v0: 0,
                    u1: SUBDIV as u8,
                    v1: SUBDIV as u8,
                };
                for row_y in 0..edge {
                    for (w, slot) in row.iter_mut().enumerate() {
                        let mut here = [0; 3];
                        here[d] = own;
                        here[1] = row_y;
                        here[w_axis] = w as i32;
                        *slot = None;
                        let voxel = n.at(here[0], here[1], here[2]);
                        if !voxel.is_full_cube() {
                            continue;
                        }
                        let class = tables.materials.render(voxel);
                        // Water levels with the finer level's, and two
                        // surfaces of it would blend twice.
                        if class == RenderClass::Blend {
                            continue;
                        }
                        let mut across = here;
                        across[d] += if positive { 1 } else { -1 };
                        let [x, y, z] = across;
                        // Where it does not cover the face, pass 1 drew it.
                        if !covers(tables, n.at(x, y, z), face ^ 1, voxel) {
                            continue;
                        }
                        // Above a neighbour's top row is a corner of the
                        // shell, which is never filled and reads as dark air;
                        // the cell over this Voxel's own column, in the shell
                        // above, stands in for it.
                        let over = |k: i32| {
                            if y + k < edge || (0..edge).contains(&x) && (0..edge).contains(&z) {
                                [x, y + k, z]
                            } else {
                                [here[0], y + k, here[2]]
                            }
                        };
                        let open = (1..=SEAM_DEPTH).take_while(|k| y + k <= edge).find(|&k| {
                            let [ox, oy, oz] = over(k);
                            !n.at(ox, oy, oz).is_full_cube()
                        });
                        let Some(k) = open else {
                            continue;
                        };
                        // Lit by the open air over the ground across, which
                        // the face looks into once that ground is not drawn.
                        let [ox, oy, oz] = over(k);
                        let lit = n.light_at(ox, oy, oz);
                        let wall = Pending::new(here, &quad, class, voxel.material(), lit);
                        let seen = opening_across(center, neighbors, d, across);
                        *slot = Some((wall, seen));
                    }
                    keep_spans(&row, &mut pending);
                }
            }
        }
    }
    merge_shaped(&mut pending, mesh);
}

/// Pushes, of each run of walls in `row` that could merge, those from the
/// first that can be seen to the last: one hidden between two seen ones is
/// inside the ground either way, and keeping it joins the two into one quad.
fn keep_spans(row: &[Option<(Pending, bool)>], pending: &mut Vec<Pending>) {
    let mut w = 0;
    while let Some(start) = (w..row.len()).find(|&i| row[i].is_some()) {
        let group = row[start].map(|(p, _)| p.group);
        let end = (start..row.len())
            .find(|&i| row[i].map(|(p, _)| p.group) != group)
            .unwrap_or(row.len());
        let seen = |i: &usize| row[*i].is_some_and(|(_, seen)| seen);
        if let (Some(first), Some(last)) = ((start..end).find(seen), (start..end).rev().find(seen))
        {
            pending.extend(row[first..=last].iter().flatten().map(|&(p, _)| p));
        }
        w = end;
    }
}

/// Whether the Voxel at `cell`, in or just beside `center` across `axis`, is
/// open on its seam plane.
fn opening_across(
    center: LitChunk<'_>,
    neighbors: [Option<LitChunk<'_>>; 6],
    axis: usize,
    mut cell: [i32; 3],
) -> bool {
    let edge = CHUNK_SIZE as i32;
    let owner = if cell[axis] < 0 {
        neighbors[axis * 2]
    } else if cell[axis] >= edge {
        neighbors[axis * 2 + 1]
    } else {
        Some(center)
    };
    cell[axis] = cell[axis].rem_euclid(edge);
    owner.is_some_and(|o| o.openings.is_open(axis, cell.map(|c| c as usize)))
}

/// Whether `center` provably emits nothing, from `uniform()` alone.
///
/// Light is deliberately not consulted. It can only split a quad that would
/// otherwise merge, never create or remove a face, because face visibility is a
/// question about geometry alone — a full cube beside a non-full-cube. So both
/// cases below hold whatever the light is, and
/// `light_does_not_revive_either_declined_case` is the test that says so.
fn emits_no_faces(
    center: &Chunk,
    neighbors: [Option<LitChunk<'_>>; 6],
    materials: &MaterialTable,
) -> bool {
    let Some(voxel) = center.uniform() else {
        return false;
    };

    // Only Voxels inside the centre Chunk emit faces, so uniform air emits
    // nothing whatever the neighbours hold. Deliberately not consulting them:
    // next to the case below that reads like an oversight.
    if voxel.is_air() {
        return true;
    }

    // A uniform full cube has every face occluded from the other side only if
    // all six neighbours are themselves uniform full cubes that hide it, and
    // only if it hides itself: a Chunk of leaves draws every face between two
    // leaves. One unknown neighbour disqualifies it.
    voxel.is_full_cube()
        && hides(materials, voxel, voxel)
        && neighbors.iter().all(|n| {
            n.and_then(|n| n.chunk.uniform()).is_some_and(|across| {
                // The usual neighbour is the same Voxel, already answered.
                across == voxel || (across.is_full_cube() && hides(materials, across, voxel))
            })
        })
}

/// Whether `across`, which covers the part of its face that matters, also hides
/// the face of `here` that looks at it. Geometry is the caller's question; this
/// is the Material's half of it.
///
/// An unproven face is drawn, never dropped: the failure mode is overdraw.
#[inline]
fn hides(materials: &MaterialTable, across: Voxel, here: Voxel) -> bool {
    match materials.occlusion(across) {
        Occlusion::Opaque => true,
        Occlusion::SameMaterial => across.material() == here.material(),
        Occlusion::Never => false,
    }
}

/// What two coplanar faces have to agree on to merge into one quad.
///
/// Light is in the key, and it has to be: a merged quad has four vertices and
/// cannot carry a different level per Voxel it spans. That is also why there is
/// no light-only re-mesh path — a light change can change the quad
/// decomposition, so there are no quads to reuse.
///
/// It costs nothing where light is flat, which is the common case: open terrain
/// is uniformly sky 15, so nothing that merged before stops merging, and a
/// lantern splits quads only within the 15 Voxels it reaches.
type FaceKey = (MaterialId, Light);

pub fn greedy_mesh(n: &Neighborhood, tables: &Tables) -> ChunkMesh {
    let mut mesh = ChunkMesh::default();
    // Only a Chunk with something clear in or beside it can need a Material's
    // answer, so terrain runs the pass that shipped before the Material table
    // and never reads one.
    // Partial surfaces get their own instance too: the one extra test of
    // pass 1 measured 4-7% on translucent Chunks that hold none.
    if !n.surfaces.is_empty() {
        pass1::<true, true>(n, tables, &mut mesh);
    } else if n.is_translucent(&tables.materials) {
        pass1::<true, false>(n, tables, &mut mesh);
    } else {
        pass1::<false, false>(n, tables, &mut mesh);
    }

    // Terrain has no shaped entry in its palette and so never pays for the scan.
    if n.may_contain_shaped() {
        mesh_shaped(n, tables, &mut mesh);
    }
    if !n.surfaces.is_empty() {
        mesh_surfaces(n, tables, &mut mesh);
    }
    if !n.logged.is_empty() {
        mesh_logged(n, tables, &mut mesh);
    }
    if !n.burning.is_empty() {
        mesh_flames(n, tables, &mut mesh);
    }
    mesh
}

/// Pass 5: a flame over every face of a burning solid that the Voxel across
/// does not cover, a sixteenth outside it, in the cutout layer with the
/// [`FLAME`] index and the light of the space it stands in. Drawn, not
/// solid: collision and selection read the Voxel alone and never see it.
fn mesh_flames(n: &Neighborhood, tables: &Tables, mesh: &mut ChunkMesh) {
    let layer = &mut mesh.cutout;
    for &[x, y, z] in &n.burning {
        let here = n.at(x, y, z);
        for (face, d) in FACE_STEPS.into_iter().enumerate() {
            let (ax, ay, az) = (x + d[0], y + d[1], z + d[2]);
            if covers(tables, n.at(ax, ay, az), face ^ 1, here) {
                continue;
            }
            push_flame(layer, [x, y, z], face, n.light_at(ax, ay, az));
        }
    }
}

/// One whole-face flame quad, in pass 1's corner order and winding.
fn push_flame(layer: &mut MeshLayer, cell: [i32; 3], face: usize, light: Light) {
    let positive = face % 2 == 1;
    let (d, u, v) = (face / 2, (face / 2 + 1) % 3, (face / 2 + 2) % 3);
    let plane = cell[d] as f32
        + if positive {
            1.0 + FLAME_OFFSET
        } else {
            -FLAME_OFFSET
        };
    let corners: [(f32, f32); 4] = if positive {
        [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]
    } else {
        [(0.0, 0.0), (0.0, 1.0), (1.0, 1.0), (1.0, 0.0)]
    };
    let mut normal = [0.0f32; 3];
    normal[d] = if positive { 1.0 } else { -1.0 };
    let base = layer.positions.len() as u32;
    for (cu, cv) in corners {
        let mut pos = [0.0f32; 3];
        pos[d] = plane;
        pos[u] = cell[u] as f32 + cu;
        pos[v] = cell[v] as f32 + cv;
        layer.positions.push(pos);
        layer.normals.push(normal);
        layer.uvs.push([cu, cv]);
        layer.materials.push(FLAME);
        layer.light.push(light.packed() as u32);
    }
    layer
        .indices
        .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

/// Pass 4: fluid in the voids of a solid, clipped to them by
/// [`crate::shape::Void::quads`]. The solid keeps its own geometry from pass 2,
/// and its collision and selection are untouched: only what is drawn of the
/// fluid is shaped by the voids. A quad on the boundary is hidden where the
/// Voxel across covers it, by its solid or by the same fluid filling its voids,
/// and lit by that Voxel's light; the surface inside is lit by the cell's own.
fn mesh_logged(n: &Neighborhood, tables: &Tables, mesh: &mut ChunkMesh) {
    let mut cells: Vec<(usize, Fluid)> = n.logged.iter().map(|(&i, &f)| (i, f)).collect();
    cells.sort_unstable_by_key(|&(i, _)| i);
    let mut pending = Vec::new();
    let mut quads = Vec::new();
    for (i, fluid) in cells {
        let (x, z, y) = (
            (i % PADDED) as i32 - 1,
            (i / PADDED % PADDED) as i32 - 1,
            (i / (PADDED * PADDED)) as i32 - 1,
        );
        let inside = [x, y, z]
            .iter()
            .all(|&c| (0..CHUNK_SIZE as i32).contains(&c));
        if !inside {
            continue;
        }
        let solid = n.at(x, y, z);
        let here = Voxel::cube(fluid.material);
        let class = tables.materials.render(here);
        let own = n.light_at(x, y, z);
        let arms = n.connections_at(tables, x, y, z);
        quads.clear();
        for (r, void) in tables.shapes.voids(solid, arms).iter().enumerate() {
            let level = fluid.level(r);
            if level.is_wet() {
                void.quads(level.surface, &mut quads);
            }
        }
        for q in &quads {
            let face = q.face as usize;
            let lit = if q.on_boundary() {
                let d = FACE_STEPS[face];
                let across = [x + d[0], y + d[1], z + d[2]];
                let cover = n.fluid_cover(tables, across, face ^ 1, here, solid);
                if cover.contains_rect(q.u0, q.v0, q.u1, q.v1) {
                    continue;
                }
                let top = match face / 2 {
                    0 => q.u1,
                    2 => q.v1,
                    _ => SUBDIV as u8,
                };
                if face != NEG_Y
                    && face != POS_Y
                    && n.surface_at(across[0], across[1], across[2])
                        .is_some_and(|(m, s)| m == fluid.material && s >= top)
                {
                    continue;
                }
                n.light_at(across[0], across[1], across[2])
            } else {
                own
            };
            pending.push(Pending::new([x, y, z], q, class, fluid.material, lit));
        }
    }
    merge_shaped(&mut pending, mesh);
}

/// Pass 3: fluid below a full surface, as a box from the cell's floor to its
/// surface. A side is hidden where the cell across covers it, and where fluid
/// of the same Material across stands as high; against lower fluid only the
/// strip above it shows. The top looks into the cell's own space and is lit by
/// its light, as a stair's tread is.
fn mesh_surfaces(n: &Neighborhood, tables: &Tables, mesh: &mut ChunkMesh) {
    let full = SUBDIV as u8;
    let mut pending = Vec::new();
    for y in 0..CHUNK_SIZE as i32 {
        for z in 0..CHUNK_SIZE as i32 {
            for x in 0..CHUNK_SIZE as i32 {
                let Some((material, surface)) = n.surface_at(x, y, z) else {
                    continue;
                };
                let fluid = Voxel::cube(material);
                let class = tables.materials.render(fluid);
                let own = n.light_at(x, y, z);
                let top = ShapeQuad {
                    face: POS_Y as u8,
                    plane: surface,
                    u0: 0,
                    v0: 0,
                    u1: full,
                    v1: full,
                };
                pending.push(Pending::new([x, y, z], &top, class, material, own));
                for face in [NEG_X, POS_X, NEG_Y, NEG_Z, POS_Z] {
                    let d = FACE_STEPS[face];
                    let (ax, ay, az) = (x + d[0], y + d[1], z + d[2]);
                    let across = n.at(ax, ay, az);
                    if covers(tables, across, face ^ 1, fluid)
                        || (n.logged_at(ax, ay, az).is_some()
                            && n.fluid_cover(tables, [ax, ay, az], face ^ 1, fluid, fluid)
                                .is_full())
                    {
                        continue;
                    }
                    let lower = match n.surface_at(ax, ay, az) {
                        Some((m, s)) if m == material && face != NEG_Y => s,
                        _ => 0,
                    };
                    if lower >= surface {
                        continue;
                    }
                    // The vertical extent is u for an X face and v for a Z face.
                    let (u0, u1, v0, v1) = match face {
                        NEG_X | POS_X => (lower, surface, 0, full),
                        NEG_Z | POS_Z => (0, full, lower, surface),
                        _ => (0, full, 0, full),
                    };
                    let quad = ShapeQuad {
                        face: face as u8,
                        plane: if face % 2 == 1 { full } else { 0 },
                        u0,
                        v0,
                        u1,
                        v1,
                    };
                    let lit = n.light_at(ax, ay, az);
                    pending.push(Pending::new([x, y, z], &quad, class, material, lit));
                }
            }
        }
    }
    merge_shaped(&mut pending, mesh);
}

/// Whether `across` hides the whole of its face `face` from `here`.
fn covers(tables: &Tables, across: Voxel, face: usize, here: Voxel) -> bool {
    if across.is_air() {
        return false;
    }
    let sealed = across.is_full_cube() || tables.shapes.seals(across, face);
    sealed && hides(&tables.materials, across, here)
}

/// Pass 1: greedy-merged full cubes.
///
/// `TRANSLUCENT` is [`Neighborhood::is_translucent`]. When it is false every
/// Voxel in sight is air or opaque, so "the Voxel across hides this face" is
/// exactly "it is a full cube, or a Shape that seals the face" - the rule before
/// Materials had a say - and the Material table is not consulted. `SURFACES`
/// is whether any cell holds fluid below a full surface, which is translucent.
fn pass1<const TRANSLUCENT: bool, const SURFACES: bool>(
    n: &Neighborhood,
    tables: &Tables,
    mesh: &mut ChunkMesh,
) {
    let shapes = &tables.shapes;
    let materials = &tables.materials;
    let mut mask = vec![None::<FaceKey>; CHUNK_SIZE * CHUNK_SIZE];

    for d in 0..3usize {
        // (d, u, v) is a cyclic permutation of (x, y, z), so u cross v = d and
        // the winding below is counter-clockwise seen from +d.
        let u = (d + 1) % 3;
        let v = (d + 2) % 3;

        for positive in [false, true] {
            let step = if positive { 1 } else { -1 };
            // Faces come in negative/positive pairs, so flipping the low bit is
            // the face of the neighbour that looks back at this one.
            let face = d * 2 + usize::from(positive);
            let under_surface = SURFACES && face == POS_Y;

            for s in 0..CHUNK_SIZE {
                mask.fill(None);
                for a in 0..CHUNK_SIZE {
                    // Indexed by strides: with the cell's coordinates in an
                    // array indexed by axis, they lived on the stack and the
                    // loop's speed came down to how the optimiser laid out the
                    // function around it.
                    let row = pad_index(0, 0, 0) + s * STRIDES[d] + a * STRIDES[u];
                    for b in 0..CHUNK_SIZE {
                        let i = row + b * STRIDES[v];
                        let here = n.voxels[i];
                        if !here.is_full_cube() {
                            continue;
                        }
                        let j = if positive {
                            i + STRIDES[d]
                        } else {
                            i - STRIDES[d]
                        };
                        let mut c = [0i32; 3];
                        c[d] = s as i32 + step;
                        c[u] = a as i32;
                        c[v] = b as i32;
                        // Hidden only if the Voxel across covers this whole
                        // face. A slab beside a cube hides the cube's face on
                        // the slab's sealed side and nowhere else.
                        //
                        // The cube test comes first and alone because it is
                        // nearly every answer, and `seals` for all of them
                        // measured 2-6% slower on `greedy_mesh/solid` and
                        // `/terrain`. `seals` is the same question, asked of
                        // the table only for a shaped Voxel.
                        let across = n.voxels[j];
                        let hidden = if across.is_full_cube() {
                            !TRANSLUCENT || hides(materials, across, here)
                        } else {
                            across.shape() != 0
                                && (shapes.seals(across, face ^ 1)
                                    && (!TRANSLUCENT || hides(materials, across, here))
                                    || TRANSLUCENT
                                        && n.logged_at(c[0], c[1], c[2])
                                            .is_some_and(|f| f.material == here.material())
                                        && n.fluid_cover(tables, c, face ^ 1, here, here).is_full())
                                // Lower fluid of the same Material standing on
                                // this one covers all of its top.
                                || under_surface
                                    && n.surface_at(c[0], c[1], c[2])
                                        .is_some_and(|(m, _)| m == here.material())
                        };
                        if hidden {
                            continue;
                        }
                        // The light of the Voxel across the face, not of the
                        // one the face belongs to: an opaque Voxel holds 0 by
                        // construction, which is what makes the dimming rule
                        // work, so lighting a face by its own Voxel would make
                        // every face black. Read from the cell this loop has
                        // already fetched to test occlusion.
                        let lit = n.light[j];
                        mask[a * CHUNK_SIZE + b] = Some((here.material(), lit));
                    }
                }

                let mut a = 0;
                while a < CHUNK_SIZE {
                    let mut b = 0;
                    while b < CHUNK_SIZE {
                        let Some(key) = mask[a * CHUNK_SIZE + b] else {
                            b += 1;
                            continue;
                        };
                        let (m, lit) = key;

                        let mut w = 1;
                        while b + w < CHUNK_SIZE && mask[a * CHUNK_SIZE + b + w] == Some(key) {
                            w += 1;
                        }

                        let mut h = 1;
                        'grow: while a + h < CHUNK_SIZE {
                            for k in 0..w {
                                if mask[(a + h) * CHUNK_SIZE + b + k] != Some(key) {
                                    break 'grow;
                                }
                            }
                            h += 1;
                        }

                        let plane = if positive { (s + 1) as f32 } else { s as f32 };
                        let (quad, uv) = if positive {
                            (
                                [(a, b), (a + h, b), (a + h, b + w), (a, b + w)],
                                [
                                    [0.0, 0.0],
                                    [h as f32, 0.0],
                                    [h as f32, w as f32],
                                    [0.0, w as f32],
                                ],
                            )
                        } else {
                            (
                                [(a, b), (a, b + w), (a + h, b + w), (a + h, b)],
                                [
                                    [0.0, 0.0],
                                    [0.0, w as f32],
                                    [h as f32, w as f32],
                                    [h as f32, 0.0],
                                ],
                            )
                        };

                        let mut normal = [0.0f32; 3];
                        normal[d] = if positive { 1.0 } else { -1.0 };

                        // A run is one Material, so it is one layer.
                        let layer = if TRANSLUCENT {
                            mesh.layer(materials.def(m).render)
                        } else {
                            &mut mesh.opaque
                        };
                        let base = layer.positions.len() as u32;
                        for k in 0..4 {
                            layer
                                .positions
                                .push(corner(d, u, v, plane, quad[k].0, quad[k].1));
                            layer.normals.push(normal);
                            layer.uvs.push(uv[k]);
                            layer.materials.push(m as u32);
                            layer.light.push(lit.packed() as u32);
                        }
                        layer.indices.extend_from_slice(&[
                            base,
                            base + 1,
                            base + 2,
                            base,
                            base + 2,
                            base + 3,
                        ]);

                        for i in 0..h {
                            for j in 0..w {
                                mask[(a + i) * CHUNK_SIZE + b + j] = None;
                            }
                        }
                        b += w;
                    }
                    a += 1;
                }
            }
        }
    }
}

/// Pass 2: each shaped Voxel's own exposed surface, appended to pass 1's.
///
/// A quad on the cell's boundary looks out at the neighbour, so it is hidden
/// when the neighbour covers its rectangle and otherwise lit by the
/// neighbour's light, exactly as a pass-1 face is. Any other quad - a stair's
/// tread - looks into the Voxel's own air and is lit by the Voxel's own light,
/// which a partly open Voxel has and a cube does not.
///
/// Light is one flat level per quad, as in pass 1, so the shader and the
/// vertex layout are untouched. Each surviving quad is then merged with the
/// ones it abuts (see [`merge_shaped`]): that is what makes a floor of slabs
/// cost a handful of quads and not one per Voxel.
fn mesh_shaped(n: &Neighborhood, tables: &Tables, mesh: &mut ChunkMesh) {
    let shapes = &tables.shapes;
    let any_connected = shapes.any_connected();
    let mut pending = Vec::new();
    for y in 0..CHUNK_SIZE as i32 {
        for z in 0..CHUNK_SIZE as i32 {
            for x in 0..CHUNK_SIZE as i32 {
                let voxel = n.at(x, y, z);
                if !voxel.is_shaped() {
                    continue;
                }
                mesh.shaped_voxels += 1;
                let own = n.light_at(x, y, z);
                // Read from the shell this job was handed, never the live store:
                // the six face-neighbours of a centre Voxel are all inside it.
                let connections = if any_connected && shapes.is_connected(voxel) {
                    shapes.connections(
                        voxel,
                        FACE_STEPS.map(|d| n.packed_at(x + d[0], y + d[1], z + d[2])),
                    )
                } else {
                    Connections::NONE
                };

                for q in shapes.quads(voxel, connections) {
                    let face = q.face as usize;
                    let lit = if q.on_boundary() {
                        let mut c = [x, y, z];
                        c[face / 2] += if face % 2 == 1 { 1 } else { -1 };
                        let across = n.at(c[0], c[1], c[2]);
                        // Against `voxel`, so that the end of an arm meeting an arm
                        // is hidden by it instead of drawn back to back.
                        if shapes
                            .coverage_against(across, face ^ 1, voxel)
                            .contains_rect(q.u0, q.v0, q.u1, q.v1)
                            && hides(&tables.materials, across, voxel)
                        {
                            continue;
                        }
                        n.light_at(c[0], c[1], c[2])
                    } else {
                        own
                    };
                    let class = tables.materials.render(voxel);
                    let quad = Pending::new([x, y, z], q, class, voxel.material(), lit);
                    if quad.can_merge() {
                        pending.push(quad);
                    } else {
                        push_merged_quad(mesh.layer(class), &quad);
                    }
                }
            }
        }
    }
    merge_shaped(&mut pending, mesh);
}

/// A [`RenderClass`] in the two bits of a pass-2 group key.
fn class_bits(class: RenderClass) -> u8 {
    match class {
        RenderClass::Opaque => 0,
        RenderClass::Blend => 1,
        RenderClass::Cutout => 2,
    }
}

fn class_of_bits(bits: u64) -> RenderClass {
    match bits {
        1 => RenderClass::Blend,
        2 => RenderClass::Cutout,
        _ => RenderClass::Opaque,
    }
}

/// One pass-2 quad as a rectangle over the whole Chunk, in sixteenths of a
/// Voxel, with everything two quads must share to merge.
#[derive(Clone, Copy)]
struct Pending {
    /// Layer, face, plane, Material and light, so quads that may merge compare
    /// equal and sort together.
    group: u64,
    a0: u16,
    a1: u16,
    b0: u16,
    b1: u16,
}

impl Pending {
    fn new(
        cell: [i32; 3],
        q: &ShapeQuad,
        class: RenderClass,
        material: MaterialId,
        lit: Light,
    ) -> Self {
        let sub = SUBDIV as i32;
        let d = q.face as usize / 2;
        let (u, v) = ((d + 1) % 3, (d + 2) % 3);
        let plane = cell[d] * sub + q.plane as i32;
        Pending {
            group: (plane as u64)
                | (q.face as u64) << 10
                | (class_bits(class) as u64) << 13
                | (material as u64) << 15
                | (lit.packed() as u64) << 31,
            a0: (cell[u] * sub + q.u0 as i32) as u16,
            a1: (cell[u] * sub + q.u1 as i32) as u16,
            b0: (cell[v] * sub + q.v0 as i32) as u16,
            b1: (cell[v] * sub + q.v1 as i32) as u16,
        }
    }
}

impl Pending {
    /// Whether any edge lies on a Voxel boundary. One that does not is inside a
    /// single Voxel, where nothing abuts it with equal extents, so it skips the
    /// sort.
    #[inline]
    fn can_merge(&self) -> bool {
        const MASK: u16 = SUBDIV as u16 - 1;
        self.a0 & MASK == 0 || self.a1 & MASK == 0 || self.b0 & MASK == 0 || self.b1 & MASK == 0
    }
}

/// Merge the quads of pass 2 that lie in one plane, abut along a full edge and
/// agree on Material, render layer and flat light, then write what is left.
///
/// Two quads of one Voxel never abut with equal extents (`extract_quads` is
/// already greedy), so every merge is across a Voxel boundary. Rows first, then
/// columns: not the optimum, the same greedy as pass 1 and enough for a floor.
/// Chunks mesh alone, so nothing merges across a Chunk boundary, and nothing is
/// added to what the unmerged quads covered, so a face culled stays culled.
///
/// Quads are first counted into one bucket per face and plane, which is O(n),
/// so the sorts that follow are over the handful sharing a plane and not over
/// the whole Chunk: terrain with a stair on every column spreads over hundreds
/// of planes, and a few hundred quads are cheaper to bucket than to sort.
#[inline(never)]
fn merge_shaped(pending: &mut [Pending], mesh: &mut ChunkMesh) {
    // A plane is `0..=CHUNK_SIZE * SUBDIV`, so there are this many per face.
    const PLANES: usize = CHUNK_SIZE * SUBDIV + 1;
    const BUCKETS: usize = 6 * PLANES;
    let Some(&first) = pending.first() else {
        return;
    };
    let bucket = |p: &Pending| (p.group >> 10 & 7) as usize * PLANES + (p.group & 0x3ff) as usize;

    // `end[b]` counts up to where bucket `b` stops, then walks back to where it
    // starts as the quads are placed from the back.
    let mut end = [0u32; BUCKETS + 1];
    for p in pending.iter() {
        end[bucket(p)] += 1;
    }
    let mut total = 0;
    for e in end.iter_mut() {
        total += *e;
        *e = total;
    }
    let mut sorted = vec![first; pending.len()];
    for p in pending.iter().rev() {
        let slot = &mut end[bucket(p)];
        *slot -= 1;
        sorted[*slot as usize] = *p;
    }

    // `end[b]` is now where bucket `b` starts, and `end[b + 1]` where it stops.
    for b in 0..BUCKETS {
        let run = &mut sorted[end[b] as usize..end[b + 1] as usize];
        let kept = if run.len() > 1 {
            merge_run(run)
        } else {
            run.len()
        };
        for p in &run[..kept] {
            push_merged_quad(mesh.layer(class_of_bits(p.group >> 13 & 0b11)), p);
        }
    }
}

/// Merge `run`, all in one plane, in place and return how many are left.
fn merge_run(run: &mut [Pending]) -> usize {
    run.sort_unstable_by_key(|p| (p.group, (p.b0 as u32) << 16 | p.b1 as u32, p.a0));
    let kept = compact(
        run,
        |last, p| last.group == p.group && last.b0 == p.b0 && last.b1 == p.b1 && last.a1 == p.a0,
        |last, p| last.a1 = p.a1,
    );
    let run = &mut run[..kept];
    run.sort_unstable_by_key(|p| (p.group, (p.a0 as u32) << 16 | p.a1 as u32, p.b0));
    compact(
        run,
        |last, p| last.group == p.group && last.a0 == p.a0 && last.a1 == p.a1 && last.b1 == p.b0,
        |last, p| last.b1 = p.b1,
    )
}

/// Fold each quad into the one before it where `abuts` says so, keeping the
/// survivors at the front.
fn compact(
    run: &mut [Pending],
    abuts: impl Fn(&Pending, &Pending) -> bool,
    grow: impl Fn(&mut Pending, &Pending),
) -> usize {
    let mut kept = 0;
    for i in 0..run.len() {
        let p = run[i];
        if kept > 0 && abuts(&run[kept - 1], &p) {
            grow(&mut run[kept - 1], &p);
        } else {
            run[kept] = p;
            kept += 1;
        }
    }
    kept
}

/// The same corner order, winding and UV axes as pass 1, so a pass-2 quad is
/// indistinguishable from a merged one downstream.
///
/// UVs are the position inside the Voxel the quad starts in, running on across
/// the next ones, so a half-height slab side shows the lower half of the
/// texture rather than a squashed whole one, and a tiling sampler still repeats
/// per Voxel.
fn push_merged_quad(mesh: &mut MeshLayer, p: &Pending) {
    const UNIT: f32 = 1.0 / SUBDIV as f32;
    let sub = SUBDIV as u16;
    let plane = (p.group & 0x3ff) as f32 * UNIT;
    let face = (p.group >> 10 & 0x7) as usize;
    let material = (p.group >> 15 & 0xffff) as u32;
    let light = (p.group >> 31) as u32;
    let positive = face % 2 == 1;
    let (d, u, v) = (face / 2, (face / 2 + 1) % 3, (face / 2 + 2) % 3);
    let (ubase, vbase) = (p.a0 / sub * sub, p.b0 / sub * sub);

    let corners = if positive {
        [(p.a0, p.b0), (p.a1, p.b0), (p.a1, p.b1), (p.a0, p.b1)]
    } else {
        [(p.a0, p.b0), (p.a0, p.b1), (p.a1, p.b1), (p.a1, p.b0)]
    };
    let mut normal = [0.0f32; 3];
    normal[d] = if positive { 1.0 } else { -1.0 };

    let base = mesh.positions.len() as u32;
    for (cu, cv) in corners {
        let mut pos = [0.0f32; 3];
        pos[d] = plane;
        pos[u] = cu as f32 * UNIT;
        pos[v] = cv as f32 * UNIT;
        mesh.positions.push(pos);
        mesh.normals.push(normal);
        mesh.uvs
            .push([(cu - ubase) as f32 * UNIT, (cv - vbase) as f32 * UNIT]);
        mesh.materials.push(material);
        mesh.light.push(light);
    }
    mesh.indices
        .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}
