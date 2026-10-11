//! Greedy meshing: merge coplanar faces of identical full cubes into quads.

use crate::chunk::{CHUNK_SIZE, Chunk};
use crate::light::{ChunkLight, Light};
use crate::shape::ShapeTable;
use crate::voxel::{MaterialId, Voxel};

/// Chunk edge plus one Voxel of neighbour on each side — exactly what face
/// culling needs.
pub const PADDED: usize = CHUNK_SIZE + 2;

const PADDED_VOLUME: usize = PADDED * PADDED * PADDED;

/// Face index into [`Neighborhood::new`]'s neighbour array.
pub const NEG_X: usize = 0;
pub const POS_X: usize = 1;
pub const NEG_Y: usize = 2;
pub const POS_Y: usize = 3;
pub const NEG_Z: usize = 4;
pub const POS_Z: usize = 5;

/// A Chunk and its light, which the mesher always needs together.
///
/// One parameter instead of two parallel arrays, so [`Neighborhood::new`] and
/// [`mesh_chunk`] keep two arguments rather than four — four of which the
/// workspace's `-D warnings` would reject as `clippy::too_many_arguments`.
#[derive(Clone, Copy)]
pub struct LitChunk<'a> {
    pub chunk: &'a Chunk,
    pub light: &'a ChunkLight,
}

impl<'a> LitChunk<'a> {
    pub fn new(chunk: &'a Chunk, light: &'a ChunkLight) -> Self {
        LitChunk { chunk, light }
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
}

impl Neighborhood {
    pub fn new(center: LitChunk<'_>, neighbors: [Option<LitChunk<'_>>; 6]) -> Self {
        let mut voxels = vec![Voxel::AIR; PADDED_VOLUME].into_boxed_slice();
        let mut light = vec![Light::DARK; PADDED_VOLUME].into_boxed_slice();
        let last = CHUNK_SIZE - 1;

        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let i = pad_index(x as i32, y as i32, z as i32);
                    voxels[i] = center.chunk.get(x, y, z);
                    light[i] = center.light.get(x, y, z);
                }
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

        Neighborhood { voxels, light }
    }

    /// Treats everything outside the Chunk as air. For tests, benches, and the
    /// edge of generated world.
    pub fn isolated(center: LitChunk<'_>) -> Self {
        Self::new(center, [None; 6])
    }

    #[inline]
    pub fn at(&self, x: i32, y: i32, z: i32) -> Voxel {
        self.voxels[pad_index(x, y, z)]
    }

    #[inline]
    pub fn light_at(&self, x: i32, y: i32, z: i32) -> Light {
        self.light[pad_index(x, y, z)]
    }
}

#[inline]
fn pad_index(x: i32, y: i32, z: i32) -> usize {
    debug_assert!((-1..=CHUNK_SIZE as i32).contains(&x));
    debug_assert!((-1..=CHUNK_SIZE as i32).contains(&y));
    debug_assert!((-1..=CHUNK_SIZE as i32).contains(&z));
    ((y + 1) as usize * PADDED + (z + 1) as usize) * PADDED + (x + 1) as usize
}

/// Triangle soup for one Chunk, in Voxel-index space. The caller scales by the
/// World's Voxel size; nothing here is in metres.
#[derive(Default, Debug)]
pub struct ChunkMesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub materials: Vec<u32>,
    /// The packed [`Light`] of the Voxel each vertex's face looks out at, one
    /// per vertex. Baked rather than sampled: a torch is a per-Voxel light
    /// level, and thousands of them as GPU point lights is not viable.
    pub light: Vec<u32>,
    pub indices: Vec<u32>,
    /// Non-cube Voxels this pass ignored. The second mesher pass consumes them.
    pub skipped_shaped: u32,
}

impl ChunkMesh {
    pub fn quad_count(&self) -> usize {
        self.indices.len() / 6
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
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
/// `shapes` is how a neighbour's Shape is asked what it hides.
///
/// The padded copy costs 39 thousand Voxel writes before `greedy_mesh` sees
/// anything, and a streamed 3D ring asks for a mesh of mostly uniform Chunks.
/// A `None` neighbour means unknown, not air — the caller is expected to have
/// all six loaded, and [`Neighborhood::isolated`]'s "outside is air" is a
/// different question.
pub fn mesh_chunk(
    center: LitChunk<'_>,
    neighbors: [Option<LitChunk<'_>>; 6],
    shapes: &ShapeTable,
) -> ChunkMesh {
    if emits_no_faces(center.chunk, neighbors) {
        return ChunkMesh::default();
    }
    greedy_mesh(&Neighborhood::new(center, neighbors), shapes)
}

/// Whether `center` provably emits nothing, from `uniform()` alone.
///
/// Light is deliberately not consulted. It can only split a quad that would
/// otherwise merge, never create or remove a face, because face visibility is a
/// question about geometry alone — a full cube beside a non-full-cube. So both
/// cases below hold whatever the light is, and
/// `light_does_not_revive_either_declined_case` is the test that says so.
fn emits_no_faces(center: &Chunk, neighbors: [Option<LitChunk<'_>>; 6]) -> bool {
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
    // all six neighbours are themselves uniform full cubes. One unknown
    // neighbour disqualifies it.
    voxel.is_full_cube()
        && neighbors.iter().all(|n| {
            n.and_then(|n| n.chunk.uniform())
                .is_some_and(Voxel::is_full_cube)
        })
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

pub fn greedy_mesh(n: &Neighborhood, shapes: &ShapeTable) -> ChunkMesh {
    let mut mesh = ChunkMesh::default();
    let mut mask = vec![None::<FaceKey>; CHUNK_SIZE * CHUNK_SIZE];

    for y in 0..CHUNK_SIZE as i32 {
        for z in 0..CHUNK_SIZE as i32 {
            for x in 0..CHUNK_SIZE as i32 {
                let v = n.at(x, y, z);
                if !v.is_air() && !v.is_full_cube() {
                    mesh.skipped_shaped += 1;
                }
            }
        }
    }

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

            for s in 0..CHUNK_SIZE {
                mask.fill(None);
                for a in 0..CHUNK_SIZE {
                    for b in 0..CHUNK_SIZE {
                        let mut c = [0i32; 3];
                        c[d] = s as i32;
                        c[u] = a as i32;
                        c[v] = b as i32;
                        let here = n.at(c[0], c[1], c[2]);
                        if !here.is_full_cube() {
                            continue;
                        }
                        c[d] = s as i32 + step;
                        // Hidden only if the Voxel across covers this whole
                        // face. A slab beside a cube hides the cube's face on
                        // the slab's sealed side and nowhere else.
                        //
                        // The cube test comes first and alone because it is
                        // nearly every answer, and `seals` for all of them
                        // measured 2-6% slower on `greedy_mesh/solid` and
                        // `/terrain`. `seals` is the same question, asked of
                        // the table only for a shaped Voxel.
                        let across = n.at(c[0], c[1], c[2]);
                        if across.is_full_cube()
                            || (across.shape() != 0 && shapes.seals(across, face ^ 1))
                        {
                            continue;
                        }
                        // The light of the Voxel across the face, not of the
                        // one the face belongs to: an opaque Voxel holds 0 by
                        // construction, which is what makes the dimming rule
                        // work, so lighting a face by its own Voxel would make
                        // every face black. Read from the cell this loop has
                        // already fetched to test occlusion.
                        let lit = n.light_at(c[0], c[1], c[2]);
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

                        let base = mesh.positions.len() as u32;
                        for k in 0..4 {
                            mesh.positions
                                .push(corner(d, u, v, plane, quad[k].0, quad[k].1));
                            mesh.normals.push(normal);
                            mesh.uvs.push(uv[k]);
                            mesh.materials.push(m as u32);
                            mesh.light.push(lit.packed() as u32);
                        }
                        mesh.indices.extend_from_slice(&[
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

    mesh
}
