//! Which level-0 Chunks a walk from the camera sees, from the connectivity
//! each `ChunkMeshReady` carries, never World storage
//! (`docs/adr/0002-client-server-from-day-one.md`). `TESSERAE_SIGHT_CULL=1`
//! under `dev` or `profiling` leaves the rest out of what the camera draws,
//! though not out of the sun's shadow maps;
//! `TESSERAE_PROFILE_VISIBILITY` counts what that would remove.

#[cfg(feature = "profiling")]
use bevy::camera::primitives::{Aabb, Frustum};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
#[cfg(feature = "profiling")]
use bevy::shape::ViewFrustum;
use tesserae_core::visibility::{Bounds, Connectivity, Visited, sight};

use super::camera::MainCamera;
#[cfg(feature = "profiling")]
use super::chunk_view::{ChunkEntities, DrawnChunk};
use crate::server::streaming::{Rings, StreamingSettings};
#[cfg(feature = "profiling")]
use crate::session::SessionApp;
use crate::world::{ChunkCoord, WorldSettings};

/// Within this of a Chunk, the camera walks from that Chunk too: a camera
/// transform a frame behind cannot then cull a Chunk it has just come to
/// see. A frame of a 64 m/s sprint at 30 fps is about 2 m.
const MARGIN_M: f32 = 4.0;

/// The connectivity the Client was sent and the last walk over it.
#[derive(Resource, Default)]
pub(super) struct Culling {
    /// Whether to leave unseen Chunks out of what is drawn.
    pub cull: bool,
    /// `TESSERAE_SIGHT_CULL=noshadow`: culled Chunks cast no shadow either. A
    /// control for review captures only.
    pub shadowless: bool,
    /// Whether to count what that would remove.
    pub count: bool,
    lines: HashMap<ChunkCoord, Connectivity>,
    /// The Chunks the last walk started from.
    starts: Vec<[i32; 3]>,
    stale: bool,
    pub visible: Option<Visited>,
}

impl Culling {
    pub(super) fn from_env() -> Self {
        let on = |var| {
            cfg!(any(feature = "dev", feature = "profiling")) && std::env::var_os(var).is_some()
        };
        #[cfg(any(feature = "dev", feature = "profiling"))]
        let sight_cull = std::env::var("TESSERAE_SIGHT_CULL").ok();
        #[cfg(not(any(feature = "dev", feature = "profiling")))]
        let sight_cull: Option<String> = None;
        Culling {
            cull: sight_cull
                .as_deref()
                .is_some_and(|v| v == "1" || v == "noshadow"),
            shadowless: sight_cull.as_deref() == Some("noshadow"),
            count: cfg!(feature = "profiling")
                && on("TESSERAE_PROFILE_VISIBILITY")
                && on("TESSERAE_PROFILE_CSV"),
            ..default()
        }
    }
}

impl Culling {
    /// A level-0 mesh arrived with this connectivity.
    pub(super) fn saw(&mut self, coord: ChunkCoord, connectivity: Connectivity) {
        if self.cull || self.count {
            self.stale |= self.lines.insert(coord, connectivity) != Some(connectivity);
        }
    }

    /// The Server unloaded a level-0 Chunk; unknown counts as open.
    pub(super) fn lost(&mut self, coord: ChunkCoord) {
        self.stale |= self.lines.remove(&coord).is_some();
    }
}

pub(super) fn wanted(culling: Res<Culling>) -> bool {
    culling.cull || culling.count
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
/// camera is near changed. Without the frustum, which Bevy
/// culls by already, so turning on the spot changes nothing here.
pub(super) fn walk(
    mut culling: ResMut<Culling>,
    streaming: Res<StreamingSettings>,
    world: Res<WorldSettings>,
    camera: Query<&GlobalTransform, With<MainCamera>>,
) {
    let culling = &mut *culling;
    let Some(camera_m) = camera.iter().next().map(GlobalTransform::translation) else {
        culling.visible = None;
        return;
    };
    let cell_m = world.cell_size();
    let starts = starts(camera_m, cell_m);
    if !culling.stale && culling.visible.is_some() && starts == culling.starts {
        return;
    }
    let at = world.chunk_at(camera_m);
    let keep = Rings::new(*streaming, cell_m).keep;
    let bounds = Bounds::around(
        [at.x, at.y, at.z],
        [keep.horizontal + 1, keep.vertical + 1, keep.horizontal + 1],
    );
    let lines = &culling.lines;
    let of = |[x, y, z]: [i32; 3]| {
        lines
            .get(&ChunkCoord::new(x, y, z))
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
    culling.visible = visible;
    culling.starts = starts;
    culling.stale = false;
}

/// This frame's level-0 draws, as their regions make them, and what a
/// walk from the camera would remove from them.
///
/// Bevy culls a region whole, so a Chunk is drawn if its region (its own box
/// for a blended layer) is in the frustum. A Chunk the walk does not enter
/// would be left out of its region's next assembly; a region none of whose
/// Chunks it enters would not be drawn at all.
#[cfg(feature = "profiling")]
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

#[cfg(feature = "profiling")]
pub(super) fn plugin(app: &mut App) {
    if std::env::var_os("TESSERAE_PROFILE_CSV").is_none()
        || std::env::var_os("TESSERAE_PROFILE_VISIBILITY").is_none()
    {
        return;
    }
    app.init_resource::<Sight>().add_session_systems(
        PostUpdate,
        count.after(bevy::transform::TransformSystems::Propagate),
    );
}

#[cfg(feature = "profiling")]
fn count(
    mut out: ResMut<Sight>,
    culling: Res<Culling>,
    entities: Option<Res<ChunkEntities>>,
    streaming: Res<StreamingSettings>,
    world: Res<WorldSettings>,
    camera: Option<Single<(&GlobalTransform, &Projection), With<MainCamera>>>,
) {
    *out = Sight::default();
    let (Some(entities), Some(camera), Some(walked)) = (entities, camera, &culling.visible) else {
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
    *out = tally(walked, &entities.level_zero_drawn(), box_shown);
}

/// `box_shown` answers for a box of `edge` Chunks from its least corner.
#[cfg(feature = "profiling")]
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

#[cfg(all(test, feature = "profiling"))]
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
        sight([0; 3], Bounds::around([0; 3], [7, 2, 2]), of)
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
