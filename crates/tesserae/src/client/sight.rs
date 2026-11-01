//! What a visibility walk from the camera would cull, counted per frame for the
//! profiler under `TESSERAE_PROFILE_VISIBILITY`. Nothing is culled. The walk
//! reads the connectivity each `ChunkMeshReady` carries, never World storage
//! (`docs/adr/0002-client-server-from-day-one.md`).

use bevy::camera::primitives::{Aabb, Frustum};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::shape::ViewFrustum;
use tesserae_core::visibility::{Bounds, Connectivity, Visited, sight};

use super::camera::MainCamera;
use super::chunk_view::ChunkEntities;
use crate::server::streaming::{Rings, StreamingSettings};
use crate::session::SessionApp;
use crate::world::{ChunkCoord, WorldSettings};

/// Within this of a Chunk, the camera walks from that Chunk too, so a Chunk
/// the camera is about to enter is never counted as culled.
const MARGIN_M: f32 = 4.0;

/// The connectivity the Client was sent and the last walk over it.
#[derive(Resource, Default)]
pub(super) struct SightLines {
    lines: HashMap<ChunkCoord, Connectivity>,
    /// The Chunks the last walk started from.
    starts: Vec<[i32; 3]>,
    stale: bool,
    visible: Option<Visited>,
}

impl SightLines {
    /// A level-0 mesh arrived with this connectivity.
    pub(super) fn saw(&mut self, coord: ChunkCoord, connectivity: Connectivity) {
        self.stale |= self.lines.insert(coord, connectivity) != Some(connectivity);
    }

    /// The Server unloaded a level-0 Chunk; unknown counts as open.
    pub(super) fn lost(&mut self, coord: ChunkCoord) {
        self.stale |= self.lines.remove(&coord).is_some();
    }
}

/// This frame's level-0 Chunks with geometry in the frustum, and those of
/// them the walk does not enter.
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct Sight {
    pub counted: bool,
    pub drawn: u32,
    /// Delivered and not entered: what culling would remove.
    pub culled: u32,
    /// Retained and not entered: culling must draw these anyway, since a
    /// retained mesh stands in for a hole.
    pub retained_hidden: u32,
}

pub(super) fn plugin(app: &mut App) {
    if std::env::var_os("TESSERAE_PROFILE_CSV").is_none()
        || std::env::var_os("TESSERAE_PROFILE_VISIBILITY").is_none()
    {
        return;
    }
    app.init_resource::<Sight>()
        .init_session_resource::<SightLines>()
        .add_session_systems(
            PostUpdate,
            (walk, count)
                .chain()
                .after(bevy::transform::TransformSystems::Propagate),
        );
}

/// The Chunks within [`MARGIN_M`] of the camera, which a walk starts from.
fn starts(camera_m: Vec3, cell_m: f32) -> Vec<[i32; 3]> {
    let span = |a: f32| {
        (((a - MARGIN_M) / cell_m).floor() as i32)..=(((a + MARGIN_M) / cell_m).floor() as i32)
    };
    let mut out = Vec::new();
    for z in span(camera_m.z) {
        for y in span(camera_m.y) {
            for x in span(camera_m.x) {
                out.push([x, y, z]);
            }
        }
    }
    out
}

/// Walks again from the camera when the connectivity sent or the Chunks the
/// camera is near changed. Without the frustum, which `count` applies, so
/// turning on the spot walks nothing again.
fn walk(
    mut lines: ResMut<SightLines>,
    streaming: Res<StreamingSettings>,
    world: Res<WorldSettings>,
    camera: Query<&GlobalTransform, With<MainCamera>>,
) {
    let lines = &mut *lines;
    let Some(camera_m) = camera.iter().next().map(GlobalTransform::translation) else {
        lines.visible = None;
        return;
    };
    let cell_m = world.cell_size();
    let starts = starts(camera_m, cell_m);
    if !lines.stale && lines.visible.is_some() && starts == lines.starts {
        return;
    }
    let at = world.chunk_at(camera_m);
    let keep = Rings::new(*streaming, cell_m).keep;
    let bounds = Bounds::around(
        [at.x, at.y, at.z],
        [keep.horizontal + 1, keep.vertical + 1, keep.horizontal + 1],
    );
    let held = &lines.lines;
    let of = |[x, y, z]: [i32; 3]| {
        held.get(&ChunkCoord::new(x, y, z))
            .copied()
            .unwrap_or(Connectivity::ALL)
    };
    let mut visible: Option<Visited> = None;
    for &start in &starts {
        let walked = sight(start, bounds, of);
        match &mut visible {
            Some(v) => v.extend(&walked),
            None => visible = Some(walked),
        }
    }
    lines.visible = visible;
    lines.starts = starts;
    lines.stale = false;
}

fn count(
    mut out: ResMut<Sight>,
    lines: Res<SightLines>,
    entities: Option<Res<ChunkEntities>>,
    streaming: Res<StreamingSettings>,
    world: Res<WorldSettings>,
    camera: Option<Single<(&GlobalTransform, &Projection), With<MainCamera>>>,
) {
    *out = Sight::default();
    let (Some(entities), Some(camera), Some(walked)) = (entities, camera, &lines.visible) else {
        return;
    };
    let (transform, projection) = camera.into_inner();
    let cell_m = world.cell_size();
    let clip_from_world = projection.get_clip_from_view() * transform.to_matrix().inverse();
    let frustum = Frustum(ViewFrustum::from_clip_from_world_custom_far(
        &clip_from_world,
        &transform.translation(),
        &transform.back(),
        streaming.view_distance_m + cell_m,
    ));
    let shown = |[x, y, z]: [i32; 3]| {
        let min = Vec3::new(x as f32, y as f32, z as f32) * cell_m;
        frustum.intersects_obb_identity(&Aabb::from_min_max(min, min + cell_m))
    };
    *out = tally(walked, entities.level_zero_drawn(), shown);
}

/// `drawn` is each level-0 Chunk with geometry and whether it is retained.
fn tally(
    walked: &Visited,
    drawn: impl Iterator<Item = (ChunkCoord, bool)>,
    shown: impl Fn([i32; 3]) -> bool,
) -> Sight {
    let mut out = Sight {
        counted: true,
        ..default()
    };
    for (c, retained) in drawn {
        let c = [c.x, c.y, c.z];
        if !shown(c) {
            continue;
        }
        out.drawn += 1;
        if walked.contains(c) {
            continue;
        }
        if retained {
            out.retained_hidden += 1;
        } else {
            out.culled += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_lies_behind_a_wall_of_rock_is_culled_unless_it_is_retained() {
        let bounds = Bounds::around([0; 3], [6, 2, 2]);
        let of = |c: [i32; 3]| {
            if c[0] == 2 {
                Connectivity::NONE
            } else {
                Connectivity::ALL
            }
        };
        let everywhere = |_: [i32; 3]| true;
        let walked = sight([0; 3], bounds, of);
        let c = |x| ChunkCoord::new(x, 0, 0);
        let drawn = [(c(1), false), (c(2), false), (c(3), false), (c(4), true)];
        let s = tally(&walked, drawn.into_iter(), everywhere);
        assert_eq!((s.drawn, s.culled, s.retained_hidden), (4, 1, 1));
    }

    #[test]
    fn a_chunk_outside_the_frustum_is_not_counted_as_drawn() {
        let bounds = Bounds::around([0; 3], [3, 0, 0]);
        let walked = sight([0; 3], bounds, |_| Connectivity::ALL);
        let drawn = [
            (ChunkCoord::new(-2, 0, 0), false),
            (ChunkCoord::new(2, 0, 0), false),
        ];
        let s = tally(&walked, drawn.into_iter(), |c| c[0] >= 0);
        assert_eq!((s.drawn, s.culled), (1, 0));
    }
}
