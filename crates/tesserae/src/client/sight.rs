//! What a visibility walk from the camera would cull, counted per frame for the
//! profiler under `TESSERAE_PROFILE_VISIBILITY`. Nothing is culled. The walk
//! reads the connectivity each `ChunkMeshReady` carries, never World storage
//! (`docs/adr/0002-client-server-from-day-one.md`). Ticket 12 of
//! `.scratch/2026-10-29-streaming-performance/`.

use bevy::camera::primitives::{Aabb, Frustum};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::shape::ViewFrustum;
use tesserae_core::visibility::{Bounds, Connectivity, Visited, sight};

use super::camera::MainCamera;
use super::chunk_view::{ChunkEntities, DrawnChunk};
use crate::server::streaming::{ChunkMeshReady, ChunkUnloaded, Rings, StreamingSettings};
use crate::session::SessionApp;
use crate::world::{ChunkCoord, WorldSettings};

/// This frame's level-0 draws, as ticket 09's regions make them, and what a
/// walk from the camera would remove from them.
///
/// Bevy culls a region whole, so a Chunk is drawn if its region (its own box
/// for a blended layer) is in the frustum. A Chunk the walk does not enter
/// would be left out of its region's next assembly; a region none of whose
/// Chunks it enters would not be drawn at all.
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct Sight {
    pub counted: bool,
    pub drawn: u32,
    /// Delivered and not entered: what culling would remove.
    pub culled: u32,
    /// Retained and not entered: culling must draw these anyway, since a
    /// retained mesh stands in for a hole.
    pub retained_hidden: u32,
    /// Opaque and cutout quads of the drawn Chunks, and of the culled ones.
    pub quads: u64,
    pub quads_culled: u64,
    /// Opaque and cutout regions drawn, and those culling would empty.
    pub regions: u32,
    pub regions_culled: u32,
}

/// The connectivity of every level-0 Chunk the Client holds a mesh for.
#[derive(Resource, Default)]
struct SightLines(HashMap<ChunkCoord, Connectivity>);

pub(super) fn plugin(app: &mut App) {
    if std::env::var_os("TESSERAE_PROFILE_CSV").is_none()
        || std::env::var_os("TESSERAE_PROFILE_VISIBILITY").is_none()
    {
        return;
    }
    app.init_resource::<Sight>()
        .init_session_resource::<SightLines>()
        .add_session_systems(Update, receive)
        .add_session_systems(
            PostUpdate,
            count.after(bevy::transform::TransformSystems::Propagate),
        );
}

/// Meshes before unloads, as `chunk_view` takes them.
fn receive(
    mut lines: ResMut<SightLines>,
    mut ready: MessageReader<ChunkMeshReady>,
    mut unloaded: MessageReader<ChunkUnloaded>,
) {
    for m in ready.read() {
        lines.0.insert(m.coord, m.connectivity);
    }
    for m in unloaded.read() {
        lines.0.remove(&m.coord);
    }
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
    let (Some(entities), Some(camera)) = (entities, camera) else {
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
    let box_shown = |[x, y, z]: [i32; 3], edge: i32| {
        let min = Vec3::new(x as f32, y as f32, z as f32) * cell_m;
        frustum.intersects_obb_identity(&Aabb::from_min_max(min, min + cell_m * edge as f32))
    };
    let shown = |c: [i32; 3]| box_shown(c, 1);
    let at = world.chunk_at(transform.translation());
    let keep = Rings::new(*streaming, cell_m).keep;
    let bounds = Bounds::around(
        [at.x, at.y, at.z],
        [keep.horizontal + 1, keep.vertical + 1, keep.horizontal + 1],
    );
    let of = |[x, y, z]: [i32; 3]| {
        lines
            .0
            .get(&ChunkCoord::new(x, y, z))
            .copied()
            .unwrap_or(Connectivity::ALL)
    };
    let walked = sight([at.x, at.y, at.z], bounds, of, shown);
    *out = tally(&walked, &entities.level_zero_drawn(), box_shown);
}

/// `box_shown` answers for a box of `edge` Chunks from its least corner.
fn tally(
    walked: &Visited,
    drawn: &[DrawnChunk],
    box_shown: impl Fn([i32; 3], i32) -> bool,
) -> Sight {
    let mut out = Sight {
        counted: true,
        ..default()
    };
    // Per drawn region: whether anything in it would still be drawn.
    let mut regions: HashMap<ChunkCoord, bool> = HashMap::default();
    for chunk in drawn {
        let c = [chunk.coord.x, chunk.coord.y, chunk.coord.z];
        let region_shown = chunk
            .region
            .filter(|&(r, edge)| box_shown([r.x * edge, r.y * edge, r.z * edge], edge));
        if region_shown.is_none() && !(chunk.blended && box_shown(c, 1)) {
            continue;
        }
        let kept = walked.contains(c) || chunk.retained;
        if let Some((region, _)) = region_shown {
            *regions.entry(region).or_default() |= kept;
            out.quads += chunk.quads as u64;
            if !kept {
                out.quads_culled += chunk.quads as u64;
            }
        }
        out.drawn += 1;
        if walked.contains(c) {
            continue;
        }
        if chunk.retained {
            out.retained_hidden += 1;
        } else {
            out.culled += 1;
        }
    }
    out.regions = regions.len() as u32;
    out.regions_culled = regions.values().filter(|&&kept| !kept).count() as u32;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(x: i32, retained: bool) -> DrawnChunk {
        DrawnChunk {
            coord: ChunkCoord::new(x, 0, 0),
            retained,
            region: Some((ChunkCoord::new(x >> 1, 0, 0), 2)),
            blended: false,
            quads: 10,
        }
    }

    /// Rock fills x = 2: the walk from the origin sees its face and nothing
    /// past it.
    fn walled() -> Visited {
        let of = |c: [i32; 3]| {
            if c[0] == 2 {
                Connectivity::NONE
            } else {
                Connectivity::ALL
            }
        };
        sight([0; 3], Bounds::around([0; 3], [7, 2, 2]), of, |_| true)
    }

    #[test]
    fn what_lies_behind_a_wall_of_rock_is_culled_unless_it_is_retained() {
        let drawn = [
            chunk(1, false),
            chunk(2, false),
            chunk(3, false),
            chunk(4, true),
        ];
        let s = tally(&walled(), &drawn, |_, _| true);
        assert_eq!((s.drawn, s.culled, s.retained_hidden), (4, 1, 1));
        assert_eq!((s.quads, s.quads_culled), (40, 10));
    }

    #[test]
    fn a_region_is_culled_only_when_nothing_in_it_is_still_drawn() {
        // Regions x/2 = 0, 1, 2, 3: (0, 1) seen, (2 seen, 3 not), (4, 5 not),
        // (6 retained, 7 not).
        let drawn: Vec<_> = (0..8).map(|x| chunk(x, x == 6)).collect();
        let s = tally(&walled(), &drawn, |_, _| true);
        assert_eq!((s.regions, s.regions_culled), (4, 1));
    }

    #[test]
    fn a_chunk_counts_as_drawn_when_its_region_is_in_the_frustum() {
        // Region 1 spans x = 2 and 3; only x >= 3 is in the frustum.
        let ahead = |c: [i32; 3], edge: i32| c[0] + edge > 3;
        let blended = DrawnChunk {
            region: None,
            blended: true,
            ..chunk(2, false)
        };
        let s = tally(
            &walled(),
            &[chunk(2, false), blended, chunk(0, false)],
            ahead,
        );
        assert_eq!(
            s.drawn, 1,
            "the region holding x = 2, not its blended layer alone"
        );
    }
}
