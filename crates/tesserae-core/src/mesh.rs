//! Greedy meshing: merge coplanar faces of identical full cubes into quads.

use crate::chunk::{CHUNK_SIZE, Chunk};
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

/// A Chunk plus the one-Voxel shell of its six face neighbours.
///
/// Without the shell, every Chunk emits a full skin of faces at its boundary —
/// wrong, and the dominant cost in a streamed world. Edge and corner cells of
/// the shell are left as air and are never read: face visibility only ever looks
/// at the Voxel directly across a face.
pub struct Neighborhood {
    voxels: Box<[Voxel]>,
}

impl Neighborhood {
    pub fn new(center: &Chunk, neighbors: [Option<&Chunk>; 6]) -> Self {
        let mut voxels = vec![Voxel::AIR; PADDED_VOLUME].into_boxed_slice();
        let last = CHUNK_SIZE - 1;

        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    voxels[pad_index(x as i32, y as i32, z as i32)] = center.get(x, y, z);
                }
            }
        }

        for a in 0..CHUNK_SIZE {
            for b in 0..CHUNK_SIZE {
                let (ai, bi) = (a as i32, b as i32);
                if let Some(n) = neighbors[NEG_X] {
                    voxels[pad_index(-1, ai, bi)] = n.get(last, a, b);
                }
                if let Some(n) = neighbors[POS_X] {
                    voxels[pad_index(CHUNK_SIZE as i32, ai, bi)] = n.get(0, a, b);
                }
                if let Some(n) = neighbors[NEG_Y] {
                    voxels[pad_index(ai, -1, bi)] = n.get(a, last, b);
                }
                if let Some(n) = neighbors[POS_Y] {
                    voxels[pad_index(ai, CHUNK_SIZE as i32, bi)] = n.get(a, 0, b);
                }
                if let Some(n) = neighbors[NEG_Z] {
                    voxels[pad_index(ai, bi, -1)] = n.get(a, b, last);
                }
                if let Some(n) = neighbors[POS_Z] {
                    voxels[pad_index(ai, bi, CHUNK_SIZE as i32)] = n.get(a, b, 0);
                }
            }
        }

        Neighborhood { voxels }
    }

    /// Treats everything outside the Chunk as air. For tests, benches, and the
    /// edge of generated world.
    pub fn isolated(center: &Chunk) -> Self {
        Self::new(center, [None; 6])
    }

    #[inline]
    pub fn at(&self, x: i32, y: i32, z: i32) -> Voxel {
        self.voxels[pad_index(x, y, z)]
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
/// The padded copy costs 39 thousand Voxel writes before `greedy_mesh` sees
/// anything, and a streamed 3D ring asks for a mesh of mostly uniform Chunks.
/// A `None` neighbour means unknown, not air — the caller is expected to have
/// all six loaded, and [`Neighborhood::isolated`]'s "outside is air" is a
/// different question.
pub fn mesh_chunk(center: &Chunk, neighbors: [Option<&Chunk>; 6]) -> ChunkMesh {
    if emits_no_faces(center, neighbors) {
        return ChunkMesh::default();
    }
    greedy_mesh(&Neighborhood::new(center, neighbors))
}

/// Whether `center` provably emits nothing, from `uniform()` alone.
fn emits_no_faces(center: &Chunk, neighbors: [Option<&Chunk>; 6]) -> bool {
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
        && neighbors
            .iter()
            .all(|n| n.and_then(|n| n.uniform()).is_some_and(Voxel::is_full_cube))
}

pub fn greedy_mesh(n: &Neighborhood) -> ChunkMesh {
    let mut mesh = ChunkMesh::default();
    let mut mask = vec![None::<MaterialId>; CHUNK_SIZE * CHUNK_SIZE];

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
                        if n.at(c[0], c[1], c[2]).is_full_cube() {
                            continue;
                        }
                        mask[a * CHUNK_SIZE + b] = Some(here.material());
                    }
                }

                let mut a = 0;
                while a < CHUNK_SIZE {
                    let mut b = 0;
                    while b < CHUNK_SIZE {
                        let Some(m) = mask[a * CHUNK_SIZE + b] else {
                            b += 1;
                            continue;
                        };

                        let mut w = 1;
                        while b + w < CHUNK_SIZE && mask[a * CHUNK_SIZE + b + w] == Some(m) {
                            w += 1;
                        }

                        let mut h = 1;
                        'grow: while a + h < CHUNK_SIZE {
                            for k in 0..w {
                                if mask[(a + h) * CHUNK_SIZE + b + k] != Some(m) {
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
