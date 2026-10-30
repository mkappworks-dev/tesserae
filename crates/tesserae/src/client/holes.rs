//! Holes in the view, counted per frame for the profiler from what the Client
//! was sent, never from World storage
//! (`docs/adr/0002-client-server-from-day-one.md`).
//!
//! Every Cell of every level's mesh shell that the camera's frustum touches is
//! classified. It is drawn if its own level delivered it and the draw rule in
//! force keeps it, if a coarser level delivered the point and keeps it, or if
//! all eight of its children are drawn. Otherwise it is a *rule hole* when
//! something was delivered there that the rule discards, and a *fill hole* when
//! nothing was. See `.scratch/2026-10-29-streaming-performance/`.
//!
//! The rule in force is the metre cut of `render/chunk_material.wesl`. A Cell
//! the cut's edge crosses is split into its children, down to
//! [`SPLIT_LEVELS`] levels finer, and below that the cut is tested at the
//! centre.

use bevy::camera::primitives::{Aabb, Frustum};
use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use bevy::shape::ViewFrustum;

use super::camera::{FlyCamera, MainCamera};
use super::chunk_view::ChunkEntities;
use crate::server::streaming::StreamingSettings;
use crate::server::{Cut, LevelRings, cuts, level_count};
use crate::world::{ChunkCoord, WorldSettings};

/// Past every level a World has at practical view distances.
pub const LEVELS: usize = 8;

/// How many levels finer a Cell the cut's edge crosses is split, bounding the
/// work to `8^SPLIT_LEVELS` Cells for each one the edge crosses.
const SPLIT_LEVELS: u32 = 3;

/// This frame's holes, per level, and the Chunk mesh entities drawing it.
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct Holes {
    pub rule: [u32; LEVELS],
    pub fill: [u32; LEVELS],
    pub chunk_entities: u32,
}

pub(super) fn plugin(app: &mut App) {
    if std::env::var_os("TESSERAE_PROFILE_CSV").is_none() {
        return;
    }
    app.init_resource::<Holes>().add_systems(
        PostUpdate,
        count.after(bevy::transform::TransformSystems::Propagate),
    );
}

/// Where a box lies against one level's cut.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Against {
    Kept,
    Cut,
    Straddles,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Cover {
    Drawn,
    Rule,
    Fill,
}

/// What a Cell's fragments meet: the delivered set and the rule that keeps them.
struct View<'a> {
    delivered: &'a HashSet<(u32, ChunkCoord)>,
    cuts: &'a [Cut],
    levels: u32,
    /// Where the shader measures the cut from: the camera, not the Eye.
    camera_m: Vec3,
    cell_m: f32,
}

impl View<'_> {
    fn cell_m(&self, level: u32) -> f32 {
        self.cell_m * 2f32.powi(level as i32)
    }

    fn centre_m(&self, level: u32, coord: ChunkCoord) -> Vec3 {
        (IVec3::new(coord.x, coord.y, coord.z).as_vec3() + 0.5) * self.cell_m(level)
    }

    fn delivered(&self, level: u32, coord: ChunkCoord) -> bool {
        self.delivered.contains(&(level, coord))
    }

    fn kept(&self, level: u32, point_m: Vec3) -> bool {
        let Some(cut) = self.cuts.get(level as usize) else {
            return true;
        };
        let from = point_m - self.camera_m;
        !(level != 0 && from.xz().length() < cut.radius_m && from.y.abs() < cut.half_height_m)
    }

    /// The shader keeps a fragment `r` or more out horizontally or `h` or more
    /// out vertically; this tests a box's nearest and farthest points.
    fn against(&self, level: u32, min: Vec3, max: Vec3) -> Against {
        let Some(cut) = self.cuts.get(level as usize).filter(|_| level != 0) else {
            return Against::Kept;
        };
        let near = (self.camera_m.clamp(min, max) - self.camera_m).abs();
        let far = (self.camera_m - min).abs().max((self.camera_m - max).abs());
        if near.xz().length() >= cut.radius_m || near.y >= cut.half_height_m {
            Against::Kept
        } else if far.xz().length() < cut.radius_m && far.y < cut.half_height_m {
            Against::Cut
        } else {
            Against::Straddles
        }
    }

    fn bounds(&self, level: u32, coord: ChunkCoord) -> (Vec3, Vec3) {
        let min = IVec3::new(coord.x, coord.y, coord.z).as_vec3() * self.cell_m(level);
        (min, min + self.cell_m(level))
    }

    /// Drawn everywhere: by a level that delivered it and keeps all of it, or,
    /// part by part, by its children.
    fn drawn(&self, level: u32, coord: ChunkCoord, splits: u32) -> bool {
        let (min, max) = self.bounds(level, coord);
        let mut straddled = false;
        for (l, c) in std::iter::once((level, coord)).chain(self.ancestors(level, coord)) {
            if !self.delivered(l, c) {
                continue;
            }
            match self.against(l, min, max) {
                Against::Kept => return true,
                Against::Straddles => straddled = true,
                Against::Cut => {}
            }
        }
        if level == 0 || (straddled && splits == 0) {
            let centre = self.centre_m(level, coord);
            return std::iter::once((level, coord))
                .chain(self.ancestors(level, coord))
                .any(|(l, c)| self.delivered(l, c) && self.kept(l, centre));
        }
        let splits = splits - u32::from(straddled);
        children(coord).all(|child| self.drawn(level - 1, child, splits))
    }

    fn ancestors(&self, level: u32, coord: ChunkCoord) -> impl Iterator<Item = (u32, ChunkCoord)> {
        (level + 1..self.levels).scan(coord, |c, l| {
            *c = parent(*c);
            Some((l, *c))
        })
    }

    fn classify(&self, level: u32, coord: ChunkCoord) -> Cover {
        if self.drawn(level, coord, SPLIT_LEVELS) {
            return Cover::Drawn;
        }
        let something = self.delivered(level, coord)
            || self
                .ancestors(level, coord)
                .any(|(l, c)| self.delivered(l, c))
            || (level > 0 && children(coord).any(|c| self.delivered(level - 1, c)));
        if something { Cover::Rule } else { Cover::Fill }
    }
}

fn parent(c: ChunkCoord) -> ChunkCoord {
    ChunkCoord::new(c.x.div_euclid(2), c.y.div_euclid(2), c.z.div_euclid(2))
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

type PlayerCamera = (
    &'static GlobalTransform,
    &'static Projection,
    &'static FlyCamera,
);

fn count(
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
    let (transform, projection, fly) = camera.into_inner();
    let levels = level_count(&streaming).min(LEVELS as u32);
    let cuts = cuts(&streaming, world.cell_size());
    let view = View {
        delivered: entities.delivered(),
        cuts: &cuts,
        levels,
        camera_m: transform.translation(),
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

    const CELL_M: f32 = 32.0;

    fn view(delivered: &HashSet<(u32, ChunkCoord)>, cuts: &[Cut]) -> Owned {
        Owned {
            delivered: delivered.clone(),
            cuts: cuts.to_vec(),
        }
    }

    /// Owns what a [`View`] borrows, so a test reads as one call.
    struct Owned {
        delivered: HashSet<(u32, ChunkCoord)>,
        cuts: Vec<Cut>,
    }

    impl Owned {
        fn at(&self, level: u32, coord: ChunkCoord) -> Cover {
            View {
                delivered: &self.delivered,
                cuts: &self.cuts,
                levels: 3,
                camera_m: Vec3::ZERO,
                cell_m: CELL_M,
            }
            .classify(level, coord)
        }
    }

    fn no_cut() -> Vec<Cut> {
        vec![Cut::default(); 3]
    }

    #[test]
    fn nothing_delivered_is_a_fill_hole() {
        let v = view(&HashSet::default(), &no_cut());
        assert_eq!(v.at(0, ChunkCoord::new(0, 0, 0)), Cover::Fill);
    }

    #[test]
    fn a_delivered_cell_is_drawn_and_so_is_everything_under_its_coarse_parent() {
        let mut delivered = HashSet::default();
        delivered.insert((1, ChunkCoord::new(0, 0, 0)));
        let v = view(&delivered, &no_cut());
        assert_eq!(v.at(1, ChunkCoord::new(0, 0, 0)), Cover::Drawn);
        assert_eq!(v.at(0, ChunkCoord::new(1, 1, 0)), Cover::Drawn);
        assert_eq!(v.at(0, ChunkCoord::new(2, 0, 0)), Cover::Fill);
    }

    #[test]
    fn a_coarse_cell_is_drawn_by_all_eight_children_and_not_by_seven() {
        let mut delivered: HashSet<_> =
            children(ChunkCoord::new(0, 0, 0)).map(|c| (0, c)).collect();
        let v = view(&delivered, &no_cut());
        assert_eq!(v.at(1, ChunkCoord::new(0, 0, 0)), Cover::Drawn);
        delivered.remove(&(0, ChunkCoord::new(1, 1, 1)));
        let v = view(&delivered, &no_cut());
        assert_eq!(v.at(1, ChunkCoord::new(0, 0, 0)), Cover::Rule);
    }

    #[test]
    fn a_coarse_cell_the_cut_discards_over_an_undelivered_finer_one_is_a_rule_hole() {
        let mut delivered = HashSet::default();
        delivered.insert((1, ChunkCoord::new(0, 0, 0)));
        let cuts = vec![
            Cut::default(),
            Cut {
                radius_m: 200.0,
                half_height_m: 200.0,
            },
            Cut::default(),
        ];
        let v = view(&delivered, &cuts);
        assert_eq!(v.at(1, ChunkCoord::new(0, 0, 0)), Cover::Rule);
        assert_eq!(v.at(0, ChunkCoord::new(0, 0, 0)), Cover::Rule);
        delivered.insert((0, ChunkCoord::new(0, 0, 0)));
        let v = view(&delivered, &cuts);
        assert_eq!(v.at(0, ChunkCoord::new(0, 0, 0)), Cover::Drawn);
    }

    #[test]
    fn a_coarse_cell_the_cut_edge_crosses_is_drawn_where_it_is_kept_and_its_children_are() {
        let mut delivered = HashSet::default();
        delivered.insert((1, ChunkCoord::new(0, 0, 0)));
        // Level 1's Cell spans 0..64 m; the cut keeps x >= 48 m only.
        let cuts = vec![
            Cut::default(),
            Cut {
                radius_m: 48.0,
                half_height_m: 1000.0,
            },
            Cut::default(),
        ];
        let v = view(&delivered, &cuts);
        assert_eq!(v.at(1, ChunkCoord::new(0, 0, 0)), Cover::Rule);
        for c in children(ChunkCoord::new(0, 0, 0)) {
            delivered.insert((0, c));
        }
        delivered.remove(&(0, ChunkCoord::new(1, 1, 1)));
        let v = view(&delivered, &cuts);
        assert_eq!(
            v.at(1, ChunkCoord::new(0, 0, 0)),
            Cover::Drawn,
            "the one missing child is out where the coarse Cell is kept"
        );
    }

    #[test]
    fn parents_and_children_floor_towards_negative_infinity() {
        assert_eq!(
            parent(ChunkCoord::new(-1, -2, 3)),
            ChunkCoord::new(-1, -1, 1)
        );
        assert!(
            children(ChunkCoord::new(-1, 0, 0)).all(|c| parent(c) == ChunkCoord::new(-1, 0, 0))
        );
        assert_eq!(children(ChunkCoord::new(0, 0, 0)).count(), 8);
    }
}
