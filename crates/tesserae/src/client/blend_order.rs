//! Draw order inside one blended mesh.
//!
//! Bevy sorts transparent entities back to front by mesh centre, which orders
//! two Chunks but not the quads of one: those draw in the order the mesher
//! emitted them, so from some views a nearer pane is drawn first and a farther
//! one blended over it. Re-sorting the quads by distance from the Eye fixes
//! every case the dev gallery produces, which a ray cast over 24 views of its
//! glass house and pool showed (and Bevy's OIT would need MSAA off on every LOD
//! camera). See `.scratch/2026-10-12-material-table-transparency/issues/04-*`.

use bevy::mesh::Indices;
use bevy::prelude::*;
use tesserae_core::MeshLayer;

use super::camera::MainCamera;

/// A sort holds until the Eye has moved this fraction of its distance from the
/// mesh, which turns the view of it by under four degrees. Without it every
/// lake shore in view re-sorted on every Voxel of movement: about 4500 sorts in
/// a 6 s flight over a flooded World, and up to 2.5 ms in one frame.
const RESORT_TURN: f32 = 1.0 / 16.0;

/// What re-sorting one blended entity's quads needs, kept beside it.
#[derive(Component)]
pub(super) struct BlendOrder {
    /// Each quad's centre, in metres from the entity's origin.
    centres: Vec<Vec3>,
    /// The mean of those, which the Eye's distance is measured to.
    centre: Vec3,
    /// The mesher's six indices per quad, in its order.
    indices: Vec<u32>,
    /// The least the Eye must move before the order is worth recomputing: a
    /// Voxel of this entity's level.
    step_m: f32,
    sorted_from: Option<Vec3>,
    order: Vec<u32>,
}

impl BlendOrder {
    /// `None` for a layer whose quads all lie in one plane facing one way, such
    /// as the surface of a lake: no two of them can overlap on screen, so their
    /// order cannot matter and the mesh is never re-indexed.
    pub(super) fn new(layer: &MeshLayer, voxel_size: f32) -> Option<Self> {
        let centres: Vec<Vec3> = layer
            .indices
            .chunks(6)
            .map(|quad| {
                // A quad is two triangles sharing a diagonal, so the mean of its
                // six corners is the midpoint of that diagonal, its centre.
                quad.iter()
                    .map(|&i| Vec3::from(layer.positions[i as usize]))
                    .sum::<Vec3>()
                    * (voxel_size / 6.0)
            })
            .collect();
        let normal = |q: usize| layer.normals[layer.indices[q * 6] as usize];
        let n0 = Vec3::from(normal(0));
        let flat = (0..centres.len())
            .all(|q| Vec3::from(normal(q)) == n0 && (centres[q] - centres[0]).dot(n0).abs() < 1e-4);
        if flat {
            return None;
        }
        Some(BlendOrder {
            order: (0..centres.len() as u32).collect(),
            centre: centres.iter().sum::<Vec3>() / centres.len() as f32,
            centres,
            indices: layer.indices.clone(),
            step_m: voxel_size,
            sorted_from: None,
        })
    }

    /// The indices to draw from `eye` (metres from the entity's origin),
    /// farthest quad first, or `None` if the mesh needs no change.
    fn resort(&mut self, eye: Vec3) -> Option<Vec<u32>> {
        let step_m = self.step_m.max(eye.distance(self.centre) * RESORT_TURN);
        if self
            .sorted_from
            .is_some_and(|from| from.distance(eye) < step_m)
        {
            return None;
        }
        self.sorted_from = Some(eye);
        let mut order = self.order.clone();
        order.sort_by(|&a, &b| {
            let far = |q: u32| self.centres[q as usize].distance_squared(eye);
            far(b).total_cmp(&far(a))
        });
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
}

pub(super) fn sort_blended(
    eye: Query<&GlobalTransform, With<MainCamera>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut blended: Query<(&mut BlendOrder, &Mesh3d, &Transform)>,
    #[cfg(feature = "dev")] mut cost: ResMut<crate::cost::MainThreadCost>,
) {
    let Ok(eye) = eye.single() else {
        return;
    };
    #[cfg(feature = "dev")]
    let (started, mut sorted) = (std::time::Instant::now(), 0);
    for (mut order, mesh, transform) in &mut blended {
        let Some(indices) = order.resort(eye.translation() - transform.translation) else {
            continue;
        };
        if let Some(mut mesh) = meshes.get_mut(&mesh.0) {
            mesh.insert_indices(Indices::U32(indices));
        }
        #[cfg(feature = "dev")]
        {
            sorted += 1;
        }
    }
    #[cfg(feature = "dev")]
    cost.sorted(started.elapsed(), sorted);
}
