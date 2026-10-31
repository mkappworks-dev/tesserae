//! Holes in the view, counted per frame for the profiler from what the Client
//! was sent, never from World storage
//! (`docs/adr/0002-client-server-from-day-one.md`).
//!
//! Every Cell of every level's mesh shell that the camera's frustum touches is
//! classified at the centres of its children. A point is drawn if some level
//! draws its Chunk (delivered or retained, an empty mesh included) and no finer
//! level's bit in the mask the shader reads discards it. Otherwise it is a
//! *rule hole* when some level draws its Chunk but the mask discards it, and a
//! *fill hole* when no level does. See
//! `.scratch/2026-10-29-streaming-performance/`.
//!
//! The rule in force is `docs/adr/0019-a-level-is-drawn-until-another-replaces-it.md`,
//! read from the published mask words, so a mask out of step with the drawn
//! Chunks shows as rule holes.

use bevy::camera::primitives::{Aabb, Frustum};
use bevy::prelude::*;
use bevy::shape::ViewFrustum;

use super::camera::{FlyCamera, MainCamera};
use super::chunk_view::ChunkEntities;
use super::drawn_mask;
use crate::server::streaming::StreamingSettings;
use crate::server::{LevelRings, level_count};
use crate::session::SessionApp;
use crate::world::{ChunkCoord, WorldSettings};

/// Past every level a World has at practical view distances.
pub const LEVELS: usize = 8;

/// This frame's holes, per level, and the Chunk mesh entities drawing it.
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct Holes {
    /// Whether the holes were counted: `TESSERAE_PROFILE_HOLES`, since the
    /// count costs more than the profiler's overhead bound allows a timing run.
    pub counted: bool,
    pub rule: [u32; LEVELS],
    pub fill: [u32; LEVELS],
    pub chunk_entities: u32,
}

pub(super) fn plugin(app: &mut App) {
    if std::env::var_os("TESSERAE_PROFILE_CSV").is_none() {
        return;
    }
    app.init_resource::<Holes>().add_session_systems(
        PostUpdate,
        count.after(bevy::transform::TransformSystems::Propagate),
    );
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Cover {
    Drawn,
    Rule,
    Fill,
}

/// What a point's fragments meet: the Chunks drawn and the mask that discards.
struct View<'a> {
    drawn: &'a dyn Fn(u32, ChunkCoord) -> bool,
    words: &'a [u32],
    levels: u32,
    cell_m: f32,
}

impl View<'_> {
    fn cell_m(&self, level: u32) -> f32 {
        self.cell_m * 2f32.powi(level as i32)
    }

    fn at(&self, level: u32, point_m: Vec3) -> IVec3 {
        (point_m / self.cell_m(level)).floor().as_ivec3()
    }

    fn point(&self, point_m: Vec3) -> Cover {
        let mut something = false;
        for level in 0..self.levels {
            let c = self.at(level, point_m);
            if !(self.drawn)(level, ChunkCoord::new(c.x, c.y, c.z)) {
                continue;
            }
            something = true;
            if !drawn_mask::finer_drawn_at(self.words, level, point_m) {
                return Cover::Drawn;
            }
        }
        if something { Cover::Rule } else { Cover::Fill }
    }

    /// A rule hole anywhere in the Cell outranks a fill hole.
    fn classify(&self, level: u32, coord: ChunkCoord) -> Cover {
        let cell_m = self.cell_m(level);
        let min = IVec3::new(coord.x, coord.y, coord.z).as_vec3() * cell_m;
        let samples: Vec<Vec3> = if level == 0 {
            vec![min + cell_m / 2.0]
        } else {
            (0..8)
                .map(|i| {
                    let corner = Vec3::new((i & 1) as f32, (i >> 1 & 1) as f32, (i >> 2) as f32);
                    min + (corner * 2.0 + 1.0) * cell_m / 4.0
                })
                .collect()
        };
        let covers: Vec<Cover> = samples.into_iter().map(|p| self.point(p)).collect();
        if covers.contains(&Cover::Rule) {
            Cover::Rule
        } else if covers.contains(&Cover::Fill) {
            Cover::Fill
        } else {
            Cover::Drawn
        }
    }
}

struct Counting(bool);

impl Default for Counting {
    fn default() -> Self {
        Counting(std::env::var_os("TESSERAE_PROFILE_HOLES").is_some())
    }
}

type PlayerCamera = (
    &'static GlobalTransform,
    &'static Projection,
    &'static FlyCamera,
);

fn count(
    counting: Local<Counting>,
    mut holes: ResMut<Holes>,
    entities: Option<Res<ChunkEntities>>,
    streaming: Res<StreamingSettings>,
    world: Res<WorldSettings>,
    camera: Option<Single<PlayerCamera, With<MainCamera>>>,
) {
    *holes = Holes::default();
    let (Some(entities), Some(camera)) = (entities, camera) else {
        return;
    };
    holes.chunk_entities = entities.entity_count() as u32;
    if !counting.0 {
        return;
    }
    holes.counted = true;
    let (transform, projection, fly) = camera.into_inner();
    let levels = level_count(&streaming).min(LEVELS as u32);
    let drawn = |level, coord| entities.draws((level, coord));
    let view = View {
        drawn: &drawn,
        words: entities.published(),
        levels,
        cell_m: world.cell_size(),
    };
    let far_m = streaming.lod_distance_m.max(streaming.view_distance_m) + world.cell_size();
    let clip_from_world = projection.get_clip_from_view() * transform.to_matrix().inverse();
    let frustum = Frustum(ViewFrustum::from_clip_from_world_custom_far(
        &clip_from_world,
        &transform.translation(),
        &transform.back(),
        far_m,
    ));
    for level in 0..levels {
        let rings = LevelRings::new(*streaming, world.cell_size(), level);
        let anchor = world.chunk_at_level(fly.eye, level);
        let cell_m = view.cell_m(level);
        for d in rings.mesh.offsets() {
            let coord = ChunkCoord::new(anchor.x + d.x, anchor.y + d.y, anchor.z + d.z);
            let min = IVec3::new(coord.x, coord.y, coord.z).as_vec3() * cell_m;
            if !frustum.intersects_obb_identity(&Aabb::from_min_max(min, min + cell_m)) {
                continue;
            }
            match view.classify(level, coord) {
                Cover::Drawn => {}
                Cover::Rule => holes.rule[level as usize] += 1,
                Cover::Fill => holes.fill[level as usize] += 1,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::drawn_mask::DrawnMask;
    use bevy::platform::collections::HashSet;

    const CELL_M: f32 = 32.0;

    /// Owns what a [`View`] borrows, so a test reads as one call.
    struct Owned {
        drawn: HashSet<(u32, ChunkCoord)>,
        words: Vec<u32>,
    }

    impl Owned {
        /// The mask built from the drawn set, as the Client publishes it.
        fn new(drawn: &[(u32, ChunkCoord)]) -> Self {
            let mut mask = DrawnMask::default();
            for &key in drawn {
                mask.set(key, true);
            }
            Owned {
                drawn: drawn.iter().copied().collect(),
                words: mask.words(&WorldSettings::default()),
            }
        }

        fn at(&self, level: u32, coord: ChunkCoord) -> Cover {
            let drawn = |l, c| self.drawn.contains(&(l, c));
            View {
                drawn: &drawn,
                words: &self.words,
                levels: 3,
                cell_m: CELL_M,
            }
            .classify(level, coord)
        }
    }

    fn origin() -> ChunkCoord {
        ChunkCoord::new(0, 0, 0)
    }

    fn children(c: ChunkCoord) -> impl Iterator<Item = ChunkCoord> {
        (0..8).map(move |i| {
            ChunkCoord::new(
                2 * c.x + (i & 1),
                2 * c.y + (i >> 1 & 1),
                2 * c.z + (i >> 2),
            )
        })
    }

    #[test]
    fn nothing_drawn_is_a_fill_hole() {
        assert_eq!(Owned::new(&[]).at(0, origin()), Cover::Fill);
    }

    #[test]
    fn a_coarse_chunk_draws_everything_under_it() {
        let v = Owned::new(&[(1, origin())]);
        assert_eq!(v.at(1, origin()), Cover::Drawn);
        assert_eq!(v.at(0, ChunkCoord::new(1, 1, 0)), Cover::Drawn);
        assert_eq!(v.at(0, ChunkCoord::new(2, 0, 0)), Cover::Fill);
    }

    #[test]
    fn a_coarse_chunk_is_drawn_by_its_children_and_its_own_mesh_together() {
        let mut drawn: Vec<_> = children(origin()).map(|c| (0, c)).collect();
        drawn.pop();
        assert_eq!(Owned::new(&drawn).at(1, origin()), Cover::Fill);
        drawn.push((1, origin()));
        assert_eq!(Owned::new(&drawn).at(1, origin()), Cover::Drawn);
    }

    #[test]
    fn a_mask_bit_with_no_chunk_drawn_under_it_is_a_rule_hole() {
        let mut v = Owned::new(&[(1, origin()), (0, origin())]);
        v.drawn.remove(&(0, origin()));
        assert_eq!(v.at(0, origin()), Cover::Rule);
        assert_eq!(v.at(1, origin()), Cover::Rule);
    }
}
