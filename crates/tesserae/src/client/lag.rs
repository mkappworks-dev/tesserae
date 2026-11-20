//! How far level 0 lags the Eye, measured per frame for the profiler from what
//! the Client was sent, never from World storage
//! (`docs/adr/0002-client-server-from-day-one.md`).

use std::time::Instant;

use bevy::platform::collections::HashSet;
use bevy::prelude::*;

use super::camera::MainCamera;
use super::chunk_view::ChunkEntities;
use super::holes::{PlayerCamera, frustum, in_view};
use crate::server::streaming::StreamingSettings;
use crate::session::SessionApp;
use crate::world::{ChunkCoord, WorldSettings};

#[derive(Resource, Default, Debug)]
pub struct Lag {
    /// This frame's distance from the Eye to the nearest Cell in view that
    /// level 0's mesh ring holds and level 0 does not draw; `None` when it
    /// draws every one.
    pub missing_m: Option<f32>,
    /// The level-0 Chunks the Client started (`true`) or stopped drawing,
    /// until the profiler takes them.
    pub changed: Vec<(Instant, ChunkCoord, bool)>,
}

pub(super) fn plugin(app: &mut App) {
    if std::env::var_os("TESSERAE_PROFILE_CSV").is_none()
        || std::env::var_os("TESSERAE_PROFILE_LAG").is_none()
    {
        return;
    }
    app.init_resource::<Lag>().add_session_systems(
        PostUpdate,
        measure.after(bevy::transform::TransformSystems::Propagate),
    );
}

/// Drawn changes are found by comparing with the last frame's level-0 Chunks
/// rather than at each landing, so a reopened session's empty view reads as
/// every Chunk undrawn.
fn measure(
    mut lag: ResMut<Lag>,
    mut was: Local<HashSet<ChunkCoord>>,
    entities: Option<Res<ChunkEntities>>,
    streaming: Res<StreamingSettings>,
    world: Res<WorldSettings>,
    camera: Option<Single<PlayerCamera, With<MainCamera>>>,
) {
    let now = Instant::now();
    let drawn: HashSet<ChunkCoord> = entities.iter().flat_map(|e| e.level_zero_draws()).collect();
    let lag = &mut *lag;
    lag.changed
        .extend(drawn.difference(&was).map(|&c| (now, c, true)));
    lag.changed
        .extend(was.difference(&drawn).map(|&c| (now, c, false)));
    *was = drawn;
    lag.missing_m = None;
    let (Some(entities), Some(camera)) = (entities, camera) else {
        return;
    };
    let (transform, projection, fly) = camera.into_inner();
    let frustum = frustum(transform, projection, &streaming, &world);
    lag.missing_m = nearest_missing(
        fly.eye,
        in_view(&frustum, &streaming, &world, fly.eye, 0),
        &world,
        &|level, coord| entities.draws((level, coord)),
    );
}

/// The distance from the Eye to the nearest point of the nearest Cell level 0
/// does not draw, whatever coarser level covers it. `None` when it draws all.
fn nearest_missing(
    eye: Vec3,
    cells: impl IntoIterator<Item = ChunkCoord>,
    world: &WorldSettings,
    drawn: &dyn Fn(u32, ChunkCoord) -> bool,
) -> Option<f32> {
    cells
        .into_iter()
        .filter(|&coord| !drawn(0, coord))
        .map(|coord| world.distance_to_chunk(eye, coord))
        .min_by(f32::total_cmp)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn missing(drawn: &[(u32, ChunkCoord)], cells: &[ChunkCoord]) -> Option<f32> {
        let drawn: HashSet<_> = drawn.iter().copied().collect();
        nearest_missing(
            Vec3::new(16.0, 16.0, 16.0),
            cells.iter().copied(),
            &WorldSettings::default(),
            &|level, coord| drawn.contains(&(level, coord)),
        )
    }

    #[test]
    fn a_level_zero_cell_covered_only_by_a_coarser_level_is_missing() {
        let beside = ChunkCoord::new(1, 0, 0);
        assert_eq!(
            missing(
                &[(0, ChunkCoord::new(0, 0, 0)), (1, ChunkCoord::new(0, 0, 0))],
                &[ChunkCoord::new(0, 0, 0), beside]
            ),
            Some(16.0)
        );
    }

    #[test]
    fn a_cell_level_zero_draws_is_not_missing() {
        let far = ChunkCoord::new(3, 0, 0);
        let cells = [ChunkCoord::new(0, 0, 0), ChunkCoord::new(1, 0, 0), far];
        let drawn = [(0, ChunkCoord::new(0, 0, 0)), (0, ChunkCoord::new(1, 0, 0))];
        assert_eq!(missing(&drawn, &cells), Some(80.0));
        assert_eq!(missing(&[drawn[0], drawn[1], (0, far)], &cells), None);
    }
}
