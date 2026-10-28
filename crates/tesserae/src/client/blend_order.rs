//! Draw order inside one blended mesh.
//!
//! Bevy sorts transparent entities back to front by mesh centre, which orders
//! two Chunks but not the quads of one: those draw in the order the mesher
//! emitted them, so from some views a nearer pane is drawn first and a farther
//! one blended over it. Bevy's OIT would fix it per pixel but needs MSAA off on
//! every LOD camera. See `.scratch/2026-10-12-material-table-transparency/`,
//! tickets 04 and 08.
//!
//! Every quad is axis-aligned, so two of them whose boxes are disjoint along an
//! axis are ordered by a plane between them: whichever is on the Eye's side of
//! it is in front. That relation depends only on where the Eye is against the
//! quads' box bounds, so the order is exact and changes only when the Eye
//! crosses one of them, which is when it is recomputed.

use std::collections::BinaryHeap;

use bevy::mesh::Indices;
use bevy::prelude::*;
use tesserae_core::MeshLayer;

use super::camera::MainCamera;

/// Above this many quads the pairwise order is not worth its square cost, and
/// a mesh falls back to drawing the farthest centre first, which is right
/// unless two long quads overlap. The largest blended mesh of a default World
/// is about 500 quads; with coplanar pairs skipped, its sort fits the budget.
const MAX_EXACT_QUADS: usize = 2048;

/// How long one frame may spend sorting. Meshes are sorted nearest first, and
/// one the budget does not reach keeps its old order a frame longer. A frame
/// can still overrun it by the one sort that crosses it.
const FRAME_BUDGET: std::time::Duration = std::time::Duration::from_micros(200);

/// A clipped box with a corner this close to the Eye is treated as covering its
/// whole octant rather than divided by nearly nothing.
const NEAR_DEPTH_M: f32 = 1e-3;

/// One quad, in metres from the entity's origin.
struct Quad {
    axis: usize,
    plane: f32,
    min: Vec3,
    max: Vec3,
    centre: Vec3,
}

/// What re-sorting one blended entity's quads needs, kept beside it.
#[derive(Component)]
pub(super) struct BlendOrder {
    quads: Vec<Quad>,
    /// The mesher's six indices per quad, in its order.
    indices: Vec<u32>,
    /// The distinct positions on each axis where a quad's box starts or ends,
    /// sorted, and which cell of them the Eye was in at the last sort: the order
    /// depends on nothing else.
    planes: [Vec<f32>; 3],
    cell: Option<[usize; 3]>,
    order: Vec<u32>,
    /// The mean of the quads' centres, which meshes are sorted nearest first by.
    centre: Vec3,
}

impl BlendOrder {
    /// `None` for a layer whose quads all lie in one plane facing one way, such
    /// as the surface of a lake: no two of them can overlap on screen, so their
    /// order cannot matter and the mesh is never re-indexed.
    pub(super) fn new(layer: &MeshLayer, voxel_size: f32) -> Option<Self> {
        let quads: Vec<Quad> = layer
            .indices
            .chunks(6)
            .map(|corners| {
                let at = |i: u32| Vec3::from(layer.positions[i as usize]) * voxel_size;
                let min = corners.iter().map(|&i| at(i)).fold(Vec3::MAX, Vec3::min);
                let max = corners.iter().map(|&i| at(i)).fold(Vec3::MIN, Vec3::max);
                let normal = Vec3::from(layer.normals[corners[0] as usize]);
                let axis = (0..3).find(|&a| normal[a] != 0.0).unwrap_or(1);
                Quad {
                    axis,
                    plane: min[axis],
                    min,
                    max,
                    centre: (min + max) / 2.0,
                }
            })
            .collect();
        let facing = |q: usize| layer.normals[layer.indices[q * 6] as usize];
        let flat =
            (0..quads.len()).all(|q| facing(q) == facing(0) && quads[q].plane == quads[0].plane);
        if flat {
            return None;
        }
        let planes = std::array::from_fn(|axis| {
            let mut on: Vec<f32> = quads
                .iter()
                .flat_map(|q| [q.min[axis], q.max[axis]])
                .collect();
            on.sort_by(f32::total_cmp);
            on.dedup();
            on
        });
        Some(BlendOrder {
            order: (0..quads.len() as u32).collect(),
            centre: quads.iter().map(|q| q.centre).sum::<Vec3>() / quads.len() as f32,
            quads,
            indices: layer.indices.clone(),
            planes,
            cell: None,
        })
    }

    /// Whether the Eye (metres from the entity's origin) has crossed one of the
    /// quads' box bounds since the last sort.
    fn stale(&self, eye: Vec3) -> bool {
        self.cell != Some(self.cell_of(eye))
    }

    fn cell_of(&self, eye: Vec3) -> [usize; 3] {
        std::array::from_fn(|a| self.planes[a].partition_point(|&p| p < eye[a]))
    }

    /// The indices to draw from `eye` (metres from the entity's origin),
    /// back to front, or `None` if the mesh needs no change.
    fn resort(&mut self, eye: Vec3) -> Option<Vec<u32>> {
        let cell = self.cell_of(eye);
        if self.cell == Some(cell) {
            return None;
        }
        self.cell = Some(cell);
        let order = if self.quads.len() <= MAX_EXACT_QUADS {
            self.separated(eye)
        } else {
            self.by_centre(eye)
        };
        if order == self.order {
            return None;
        }
        self.order = order;
        Some(
            self.order
                .iter()
                .flat_map(|&q| &self.indices[q as usize * 6..q as usize * 6 + 6])
                .copied()
                .collect(),
        )
    }

    /// Farthest centre first.
    fn by_centre(&self, eye: Vec3) -> Vec<u32> {
        let mut order: Vec<u32> = (0..self.quads.len() as u32).collect();
        order.sort_by(|&a, &b| {
            let far = |q: u32| self.quads[q as usize].centre.distance_squared(eye);
            far(b).total_cmp(&far(a))
        });
        order
    }

    /// A topological order of "behind", farthest centre first among the quads
    /// it leaves free and wherever it has a cycle (two greedy quads that cross,
    /// which only a checkerboard of two Materials makes).
    fn separated(&self, eye: Vec3) -> Vec<u32> {
        let n = self.quads.len();
        let mut ahead: Vec<Vec<u32>> = vec![Vec::new(); n];
        let mut behind_count = vec![0u32; n];
        for (a, b) in self.overlapping_pairs(eye) {
            let (back, front) = match behind(&self.quads[a], &self.quads[b], eye) {
                Some(true) => (a, b),
                Some(false) => (b, a),
                None => continue,
            };
            ahead[back].push(front as u32);
            behind_count[front] += 1;
        }
        // Squared distance is positive, so its bits order as the float does.
        let key = |q: usize| self.quads[q].centre.distance_squared(eye).to_bits();
        let mut ready: BinaryHeap<(u32, u32)> = (0..n)
            .filter(|&q| behind_count[q] == 0)
            .map(|q| (key(q), q as u32))
            .collect();
        let mut done = vec![false; n];
        let mut order = Vec::with_capacity(n);
        while order.len() < n {
            let q = match ready.pop() {
                Some((_, q)) if !done[q as usize] => q as usize,
                Some(_) => continue,
                None => (0..n)
                    .filter(|&q| !done[q])
                    .max_by_key(|&q| key(q))
                    .expect("a quad is left"),
            };
            done[q] = true;
            order.push(q as u32);
            for &f in &ahead[q] {
                let f = f as usize;
                behind_count[f] = behind_count[f].saturating_sub(1);
                if behind_count[f] == 0 && !done[f] {
                    ready.push((key(f), f as u32));
                }
            }
        }
        order
    }
}

impl BlendOrder {
    /// The pairs of quads that may overlap on screen. Every ray from the Eye runs
    /// into one octant around it, and inside an octant every point is in front of
    /// its diagonal, so each quad's box, clipped to the octant, projects onto a
    /// plane across that diagonal. Two quads whose projections do not overlap in
    /// any octant share no ray. Conservative, so no pair that overlaps is missed.
    fn overlapping_pairs(&self, eye: Vec3) -> Vec<(usize, usize)> {
        let mut pairs = Vec::new();
        for octant in 0..8 {
            let sign = Vec3::new(
                if octant & 1 == 0 { 1.0 } else { -1.0 },
                if octant & 2 == 0 { 1.0 } else { -1.0 },
                if octant & 4 == 0 { 1.0 } else { -1.0 },
            );
            let diagonal = sign / 3f32.sqrt();
            let right = diagonal.any_orthonormal_vector();
            let up = diagonal.cross(right);
            let mut boxes: Vec<(Vec2, Vec2, usize)> = self
                .quads
                .iter()
                .enumerate()
                .filter_map(|(i, q)| {
                    let (lo, hi) = clip_to_octant(q.min - eye, q.max - eye, sign)?;
                    let (mut a, mut b) = (Vec2::MAX, Vec2::MIN);
                    for k in 0..8 {
                        let v = Vec3::new(
                            if k & 1 == 0 { lo.x } else { hi.x },
                            if k & 2 == 0 { lo.y } else { hi.y },
                            if k & 4 == 0 { lo.z } else { hi.z },
                        );
                        let depth = v.dot(diagonal);
                        // A box that reaches the Eye covers its whole octant.
                        if depth < NEAR_DEPTH_M {
                            return Some((Vec2::MIN, Vec2::MAX, i));
                        }
                        let p = Vec2::new(v.dot(right), v.dot(up)) / depth;
                        a = a.min(p);
                        b = b.max(p);
                    }
                    Some((a, b, i))
                })
                .collect();
            // Sweep along x: a box overlaps only those that start before it ends.
            boxes.sort_by(|p, q| p.0.x.total_cmp(&q.0.x));
            for (k, &(lo, hi, a)) in boxes.iter().enumerate() {
                for &(lo_b, hi_b, b) in &boxes[k + 1..] {
                    if lo_b.x > hi.x {
                        break;
                    }
                    // Two quads in one plane never share a ray: a lake's surface
                    // is most of a shore's quads, and most of its pairs.
                    let (qa, qb) = (&self.quads[a], &self.quads[b]);
                    if qa.axis == qb.axis && qa.plane == qb.plane {
                        continue;
                    }
                    if lo_b.y <= hi.y && hi_b.y >= lo.y {
                        pairs.push((a.min(b), a.max(b)));
                    }
                }
            }
        }
        pairs.sort_unstable();
        pairs.dedup();
        pairs
    }
}

/// The part of the box `lo..=hi` (relative to the Eye) inside the octant whose
/// axes point along `sign`, or `None` if it has none.
fn clip_to_octant(lo: Vec3, hi: Vec3, sign: Vec3) -> Option<(Vec3, Vec3)> {
    let mut out = (lo, hi);
    for a in 0..3 {
        if sign[a] > 0.0 {
            if hi[a] < 0.0 {
                return None;
            }
            out.0[a] = lo[a].max(0.0);
        } else {
            if lo[a] > 0.0 {
                return None;
            }
            out.1[a] = hi[a].min(0.0);
        }
    }
    Some(out)
}

/// Whether `a` is behind `b` seen from `eye`: `Some(true)` if `a` must be drawn
/// first. Two quads whose boxes are disjoint along an axis have a plane between
/// them across it, and whichever is on the Eye's side of that plane is in
/// front. `None` where the boxes overlap on every axis (two greedy quads that
/// cross), where the Eye is between them (no ray meets both), and for two
/// quads in one plane, which cannot overlap.
fn behind(a: &Quad, b: &Quad, eye: Vec3) -> Option<bool> {
    if a.axis == b.axis && a.plane == b.plane {
        return None;
    }
    for k in 0..3 {
        let (low, high, a_low) = if a.max[k] <= b.min[k] {
            (a.max[k], b.min[k], true)
        } else if b.max[k] <= a.min[k] {
            (b.max[k], a.min[k], false)
        } else {
            continue;
        };
        let e = eye[k];
        if e >= high && e > low {
            // The Eye is on the high side, so the high quad is in front.
            return Some(a_low);
        }
        if e <= low && e < high {
            return Some(!a_low);
        }
        if low < high {
            return None;
        }
    }
    None
}

pub(super) fn sort_blended(
    eye: Query<&GlobalTransform, With<MainCamera>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut blended: Query<(&mut BlendOrder, &Mesh3d, &Transform)>,
    #[cfg(any(feature = "dev", feature = "profiling"))] mut cost: ResMut<
        crate::cost::MainThreadCost,
    >,
) {
    let Ok(eye) = eye.single() else {
        return;
    };
    let started = std::time::Instant::now();
    #[cfg(any(feature = "dev", feature = "profiling"))]
    let mut sorted = 0;
    let mut stale: Vec<_> = blended
        .iter_mut()
        .filter_map(|(order, mesh, transform)| {
            let local = eye.translation() - transform.translation;
            order
                .stale(local)
                .then(|| (local.distance_squared(order.centre), local, order, mesh))
        })
        .collect();
    stale.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (_, local, mut order, mesh) in stale {
        if started.elapsed() > FRAME_BUDGET {
            break;
        }
        let Some(indices) = order.resort(local) else {
            continue;
        };
        if let Some(mut mesh) = meshes.get_mut(&mesh.0) {
            mesh.insert_indices(Indices::U32(indices));
        }
        #[cfg(any(feature = "dev", feature = "profiling"))]
        {
            sorted += 1;
        }
    }
    #[cfg(any(feature = "dev", feature = "profiling"))]
    if cost.enabled() {
        cost.sorted(started.elapsed(), sorted);
    }
}
