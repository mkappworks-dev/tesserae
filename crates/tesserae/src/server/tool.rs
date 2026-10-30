//! The Server's Use verb: what a held tool does, answered and never applied.
//!
//! Use is its own intent, not an Edit: it changes no Voxel and enters no
//! history, so it has its own outcome and refusals. Like an Edit it carries a
//! ray in metres, judged against the Player's Eye, and a target the Client
//! saw; a target that has changed since is refused rather than re-aimed. The
//! Server holds no measurement: it answers each Use with the corner struck and
//! the Client keeps what it was told.
//! See `docs/adr/0002-client-server-from-day-one.md`.

use bevy::prelude::*;
use tesserae_core::collide::{RayHit, VoxelVolume};
use tesserae_core::{NodeBox, Tables, Voxel};
pub use tesserae_mods::ToolKind;

use super::ContentTables;
use super::edit::{self, RejectReason};
use super::player::{self, Player};
use super::world_state::ChunkStore;
use crate::world::WorldSettings;

/// A tool's place in the Game's declared order.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct ToolId(pub u16);

/// The Game's tools, by declared order.
#[derive(Resource, Clone, Default, Debug)]
pub struct Tools(pub Vec<tesserae_mods::ToolDef>);

impl Tools {
    pub fn kind(&self, id: ToolId) -> Option<ToolKind> {
        self.0.get(usize::from(id.0)).map(|tool| tool.kind)
    }
}

/// The Voxel a Use struck, and the corner of it that was meant.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UseTarget {
    pub at: [i32; 3],
    /// What stood there, so a changed Voxel is a stale target.
    pub voxel: Voxel,
    /// Metres, on the struck face of the Voxel's drawn Shape.
    pub corner: Vec3,
}

/// What Use would do, or why not, for the preview before the press.
pub type UseAim = Result<UseTarget, UseRefusal>;

#[derive(Message, Clone, Copy, Debug)]
pub struct UseIntent {
    pub tool: ToolId,
    pub origin: Vec3,
    pub direction: Vec3,
    /// What the Client was shown; `None` takes whatever the ray strikes.
    pub expected: Option<UseTarget>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum UseResult {
    Measured(UseTarget),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UseRefusal {
    OutOfReach,
    /// The ray starts too far from the Player.
    OriginTooFar,
    /// The camera sees the target, but something stands between it and the Eye.
    Blocked,
    TargetChanged,
    /// The struck Voxel's Chunk is not held.
    NotLoaded,
    UnknownTool,
}

impl UseRefusal {
    pub fn message(self) -> &'static str {
        match self {
            UseRefusal::OutOfReach => "nothing in reach",
            UseRefusal::OriginTooFar => "too far from where the server has you",
            UseRefusal::Blocked => "something is in the way",
            UseRefusal::TargetChanged => "that changed; aim again",
            UseRefusal::NotLoaded => "that part of the world is not loaded",
            UseRefusal::UnknownTool => "that tool is not available",
        }
    }
}

impl From<RejectReason> for UseRefusal {
    fn from(reason: RejectReason) -> Self {
        match reason {
            RejectReason::OriginTooFar => UseRefusal::OriginTooFar,
            RejectReason::Blocked => UseRefusal::Blocked,
            RejectReason::NotLoaded => UseRefusal::NotLoaded,
            _ => UseRefusal::OutOfReach,
        }
    }
}

#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct UseOutcome(pub Result<UseResult, UseRefusal>);

pub(super) fn plugin(app: &mut App) {
    app.add_message::<UseIntent>()
        .add_message::<UseOutcome>()
        // Every frame, like an Edit's receive: an answer is never lost to a
        // frame that runs no tick, and it touches no World state.
        .add_systems(
            RunFixedMainLoop,
            answer_use
                .in_set(RunFixedMainLoopSystems::BeforeFixedMainLoop)
                .after(player::PlayerInput),
        );
}

fn answer_use(
    mut intents: MessageReader<UseIntent>,
    mut outcomes: MessageWriter<UseOutcome>,
    (store, tables, world, tools): (
        Res<ChunkStore>,
        Res<ContentTables>,
        Res<WorldSettings>,
        Res<Tools>,
    ),
    player: Single<&Player>,
) {
    for intent in intents.read() {
        let result = run_tool(&store, &tables, &world, &tools, intent, player.eye());
        outcomes.write(UseOutcome(result));
    }
}

fn run_tool(
    store: &ChunkStore,
    tables: &Tables,
    world: &WorldSettings,
    tools: &Tools,
    intent: &UseIntent,
    eye: Vec3,
) -> Result<UseResult, UseRefusal> {
    match tools.kind(intent.tool).ok_or(UseRefusal::UnknownTool)? {
        ToolKind::Measure => {
            let aimed =
                measure_target(store, tables, world, (intent.origin, intent.direction), eye)?;
            if intent
                .expected
                .is_some_and(|seen| (seen.at, seen.voxel) != (aimed.at, aimed.voxel))
            {
                return Err(UseRefusal::TargetChanged);
            }
            Ok(UseResult::Measured(aimed))
        }
    }
}

/// What a held tool would do with this ray, for the preview.
pub(super) fn aim_use(
    store: &ChunkStore,
    tables: &Tables,
    world: &WorldSettings,
    tools: &Tools,
    tool: ToolId,
    ray: (Vec3, Vec3),
    eye: Vec3,
) -> UseAim {
    match tools.kind(tool).ok_or(UseRefusal::UnknownTool)? {
        ToolKind::Measure => measure_target(store, tables, world, ray, eye),
    }
}

/// The corner of the face the Eye sees the ray strike, nearest the strike.
fn measure_target(
    store: &ChunkStore,
    tables: &Tables,
    world: &WorldSettings,
    ray: (Vec3, Vec3),
    eye: Vec3,
) -> Result<UseTarget, UseRefusal> {
    let (hit, struck) = edit::solid_hit(store, tables, world, ray, eye)?;
    let voxel = store.voxel(hit.at).ok_or(UseRefusal::NotLoaded)?;
    let boxes = edit::drawn_boxes(store, tables, hit.at, voxel);
    let sixteenth = world.voxel_size / 16.0;
    let origin = Vec3::from_array(hit.at.map(|i| i as f32 * world.voxel_size));
    let corner = snap_corner(boxes, hit, (struck - origin) / sixteenth);
    Ok(UseTarget {
        at: hit.at,
        voxel,
        corner: origin + corner * sixteenth,
    })
}

/// The vertex nearest `point` of the face the ray struck, with `point` and the
/// result in sixteenths of the Voxel. A Shape's boxes overlap and meet, so a
/// vertex of any box in the struck plane counts: a stair's tread ends at the
/// foot of its riser, which is a vertex of the upper box and not of the lower.
fn snap_corner(boxes: &[NodeBox], hit: RayHit, point: Vec3) -> Vec3 {
    let axis = hit.face / 2;
    let outward = hit.face % 2 == 1;
    let side = |b: &NodeBox| f32::from(if outward { b.max[axis] } else { b.min[axis] });
    let Some(plane) = boxes
        .iter()
        .map(side)
        .min_by(|a, b| (a - point[axis]).abs().total_cmp(&(b - point[axis]).abs()))
    else {
        return point;
    };
    let (u, v) = ((axis + 1) % 3, (axis + 2) % 3);
    let mut nearest = point;
    let mut best = f32::INFINITY;
    for b in boxes {
        for coord in [b.min[axis], b.max[axis]] {
            if f32::from(coord) != plane {
                continue;
            }
            for (cu, cv) in [
                (b.min[u], b.min[v]),
                (b.min[u], b.max[v]),
                (b.max[u], b.min[v]),
                (b.max[u], b.max[v]),
            ] {
                let mut corner = Vec3::ZERO;
                corner[axis] = plane;
                corner[u] = f32::from(cu);
                corner[v] = f32::from(cv);
                let d = corner.distance_squared(point);
                if d < best {
                    best = d;
                    nearest = corner;
                }
            }
        }
    }
    nearest
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use bevy::ecs::system::RunSystemOnce;
    use tesserae_core::collide::VoxelVolume;
    use tesserae_core::{Chunk, Orientation, SHAPE_POST, SHAPE_SLAB, SHAPE_STAIR};

    use super::*;
    use crate::world::ChunkCoord;

    const EYE: Vec3 = Vec3::new(5.5, 12.0, 5.5);

    fn rock() -> tesserae_core::MaterialId {
        tesserae_core::testgen::fixtures().ground.rock
    }

    fn measuring() -> Tools {
        Tools(vec![tesserae_mods::ToolDef {
            name: "demo:measure".into(),
            kind: ToolKind::Measure,
        }])
    }

    /// One chunk holding `voxels`, at Voxel size `size`.
    fn app_with(voxels: &[([usize; 3], Voxel)], size: f32) -> App {
        let mut chunk = Chunk::air();
        for &([x, y, z], voxel) in voxels {
            chunk.set(x, y, z, voxel);
        }
        let mut app = App::new();
        app.insert_resource(WorldSettings {
            voxel_size: size,
            ..default()
        })
        .insert_resource(ContentTables(Arc::new(crate::server::fixture_tables())))
        .insert_resource(measuring())
        .init_resource::<ChunkStore>()
        .init_resource::<super::super::streaming::MeshState>()
        .add_plugins(super::super::edit::plugin)
        .add_plugins(plugin);
        app.world_mut().spawn(Player::at(EYE));
        app.world_mut()
            .resource_mut::<ChunkStore>()
            .insert(ChunkCoord::new(0, 0, 0), chunk);
        app
    }

    fn use_from(
        app: &mut App,
        eye: Vec3,
        direction: Vec3,
        expected: Option<UseTarget>,
    ) -> UseOutcome {
        let mut players = app.world_mut().query::<&mut Player>();
        players.single_mut(app.world_mut()).unwrap().set_eye(eye);
        app.world_mut().write_message(UseIntent {
            tool: ToolId(0),
            origin: eye,
            direction,
            expected,
        });
        app.world_mut().run_system_once(answer_use).unwrap();
        let outcomes: Vec<_> = app
            .world_mut()
            .resource_mut::<Messages<UseOutcome>>()
            .drain()
            .collect();
        app.world_mut()
            .resource_mut::<Messages<UseIntent>>()
            .clear();
        assert_eq!(outcomes.len(), 1);
        outcomes[0]
    }

    fn measured(outcome: UseOutcome) -> UseTarget {
        match outcome.0 {
            Ok(UseResult::Measured(target)) => target,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_use_snaps_to_the_corner_of_the_struck_face_nearest_the_strike() {
        let mut app = app_with(&[([5, 7, 5], Voxel::cube(rock()))], 1.0);
        let target = measured(use_from(
            &mut app,
            Vec3::new(5.2, 12.0, 5.8),
            Vec3::NEG_Y,
            None,
        ));
        assert_eq!(target.at, [5, 7, 5]);
        assert_eq!(target.corner, Vec3::new(5.0, 8.0, 6.0));
    }

    #[test]
    fn a_stairs_inner_step_corner_is_selectable() {
        let stair = Voxel::shaped(rock(), SHAPE_STAIR, Orientation::IDENTITY);
        let mut app = app_with(&[([5, 7, 5], stair)], 1.0);
        // The tread is the lower half's top; it ends at the foot of the riser,
        // half way across, which is a vertex of the upper box only.
        let target = measured(use_from(
            &mut app,
            Vec3::new(5.2, 12.0, 5.4),
            Vec3::NEG_Y,
            None,
        ));
        assert_eq!(target.corner, Vec3::new(5.0, 7.5, 5.5));
    }

    #[test]
    fn the_corner_is_in_metres_at_any_voxel_size() {
        for size in [1.0, 0.5, 0.25] {
            let at = [
                (5.0 / size) as usize,
                (7.0 / size) as usize,
                (5.0 / size) as usize,
            ];
            let mut app = app_with(&[(at, Voxel::cube(rock()))], size);
            let target = measured(use_from(
                &mut app,
                Vec3::new(5.0 + size * 0.2, 7.5 + size, 5.0 + size * 0.8),
                Vec3::NEG_Y,
                None,
            ));
            assert_eq!(
                target.corner,
                Vec3::new(5.0, 7.0 + size, 5.0 + size),
                "size {size}"
            );
        }
    }

    #[test]
    fn shapes_snap_to_their_own_corners_at_every_voxel_size() {
        let stair = |_: f32| Voxel::shaped(rock(), SHAPE_STAIR, Orientation::IDENTITY);
        for size in [1.0f32, 0.5, 0.25] {
            let at = [
                (5.0 / size) as usize,
                (7.0 / size) as usize,
                (5.0 / size) as usize,
            ];
            let eye_y = 7.0 + size + 0.25;
            let strike = |voxel: Voxel, across: [f32; 2]| {
                let mut app = app_with(&[(at, voxel)], size);
                let eye = Vec3::new(5.0 + size * across[0], eye_y, 5.0 + size * across[1]);
                measured(use_from(&mut app, eye, Vec3::NEG_Y, None)).corner
            };
            // A slab is the lower half; its top corner is half a Voxel down.
            let slab = Voxel::shaped(rock(), SHAPE_SLAB, Orientation::IDENTITY);
            assert_eq!(
                strike(slab, [0.2, 0.8]),
                Vec3::new(5.0, 7.0 + size / 2.0, 5.0 + size),
                "slab {size}"
            );
            // A post stands in the middle quarter of its cell.
            let post = Voxel::shaped(rock(), SHAPE_POST, Orientation::IDENTITY);
            assert_eq!(
                strike(post, [0.45, 0.55]),
                Vec3::new(5.0 + size * 0.375, 7.0 + size, 5.0 + size * 0.625),
                "post {size}"
            );
            // A stair's tread ends half way across, at the foot of its riser.
            assert_eq!(
                strike(stair(size), [0.2, 0.4]),
                Vec3::new(5.0, 7.0 + size / 2.0, 5.0 + size / 2.0),
                "stair {size}"
            );
        }
    }

    #[test]
    fn a_gap_between_two_walls_is_measured_from_the_faces_that_bound_it() {
        let rock = Voxel::cube(rock());
        let mut app = app_with(&[([4, 7, 5], rock), ([7, 7, 5], rock)], 1.0);
        let eye = Vec3::new(6.0, 7.9, 5.2);
        let left = measured(use_from(&mut app, eye, Vec3::NEG_X, None)).corner;
        let right = measured(use_from(&mut app, eye, Vec3::X, None)).corner;
        assert_eq!(left, Vec3::new(5.0, 8.0, 5.0));
        assert_eq!(right, Vec3::new(7.0, 8.0, 5.0));
    }

    #[test]
    fn what_cannot_be_measured_is_refused_and_says_why() {
        let mut app = app_with(&[([5, 7, 5], Voxel::cube(rock()))], 1.0);
        let sky = use_from(&mut app, EYE, Vec3::Y, None);
        assert_eq!(sky.0, Err(UseRefusal::OutOfReach));
        let mut far = EYE;
        far.x += 100.0;
        app.world_mut().write_message(UseIntent {
            tool: ToolId(0),
            origin: far,
            direction: Vec3::NEG_Y,
            expected: None,
        });
        app.world_mut().run_system_once(answer_use).unwrap();
        let outcomes: Vec<_> = app
            .world_mut()
            .resource_mut::<Messages<UseOutcome>>()
            .drain()
            .collect();
        assert_eq!(outcomes[0].0, Err(UseRefusal::OriginTooFar));
    }

    #[test]
    fn a_target_that_changed_since_it_was_shown_is_refused() {
        let mut app = app_with(&[([5, 7, 5], Voxel::cube(rock()))], 1.0);
        let shown = measured(use_from(&mut app, EYE, Vec3::NEG_Y, None));
        let kept = use_from(&mut app, EYE, Vec3::NEG_Y, Some(shown));
        assert_eq!(measured(kept), shown);
        let moved = UseTarget {
            voxel: Voxel::AIR,
            ..shown
        };
        assert_eq!(
            use_from(&mut app, EYE, Vec3::NEG_Y, Some(moved)).0,
            Err(UseRefusal::TargetChanged)
        );
    }

    #[test]
    fn an_unregistered_tool_is_refused() {
        let mut app = app_with(&[([5, 7, 5], Voxel::cube(rock()))], 1.0);
        app.world_mut().insert_resource(Tools::default());
        assert_eq!(
            use_from(&mut app, EYE, Vec3::NEG_Y, None).0,
            Err(UseRefusal::UnknownTool)
        );
    }

    /// A camera three metres to the Eye's side, looking straight down at the
    /// floor, with or without a wall between the Eye and what the camera sees.
    fn camera_over_the_shoulder(walled: bool) -> (App, Vec3) {
        let rock = Voxel::cube(rock());
        let mut voxels: Vec<_> = (0..16)
            .flat_map(|x| (0..16).map(move |z| ([x, 7, z], rock)))
            .collect();
        if walled {
            voxels.extend((9..12).map(|y| ([7, y, 5], rock)));
        }
        let mut app = app_with(&voxels, 1.0);
        let mut players = app.world_mut().query::<&mut Player>();
        players.single_mut(app.world_mut()).unwrap().set_eye(EYE);
        (app, EYE + Vec3::X * 3.0)
    }

    #[test]
    fn a_corner_the_camera_sees_around_a_wall_is_blocked_and_the_preview_says_so() {
        for (walled, expected) in [(false, Ok(())), (true, Err(UseRefusal::Blocked))] {
            let (mut app, camera) = camera_over_the_shoulder(walled);
            app.world_mut().write_message(UseIntent {
                tool: ToolId(0),
                origin: camera,
                direction: Vec3::NEG_Y,
                expected: None,
            });
            app.world_mut().run_system_once(answer_use).unwrap();
            let outcome = app
                .world_mut()
                .resource_mut::<Messages<UseOutcome>>()
                .drain()
                .next()
                .unwrap();
            assert_eq!(outcome.0.map(|_| ()), expected, "walled {walled}");
            let preview = aim_use(
                app.world().resource::<ChunkStore>(),
                &crate::server::fixture_tables(),
                app.world().resource::<WorldSettings>(),
                &measuring(),
                ToolId(0),
                (camera, Vec3::NEG_Y),
                EYE,
            );
            assert_eq!(preview.map(|_| ()), expected, "walled {walled}");
        }
    }

    #[test]
    fn two_corners_can_be_farther_apart_than_one_reach_and_each_is_checked() {
        let rock = Voxel::cube(rock());
        let mut app = app_with(&[([2, 7, 2], rock), ([28, 7, 2], rock)], 1.0);
        let strike =
            |app: &mut App, x: f32| use_from(app, Vec3::new(x, 12.0, 2.5), Vec3::NEG_Y, None);
        let near = measured(strike(&mut app, 2.5));
        let far = measured(strike(&mut app, 28.5));
        assert!(near.corner.distance(far.corner) > edit::EDIT_REACH_M * 3.0);
        // Neither corner is reachable from the other's standing place.
        let mut players = app.world_mut().query::<&mut Player>();
        players
            .single_mut(app.world_mut())
            .unwrap()
            .set_eye(Vec3::new(2.5, 12.0, 2.5));
        app.world_mut().write_message(UseIntent {
            tool: ToolId(0),
            origin: Vec3::new(28.5, 12.0, 2.5),
            direction: Vec3::NEG_Y,
            expected: None,
        });
        app.world_mut().run_system_once(answer_use).unwrap();
        let refused = app
            .world_mut()
            .resource_mut::<Messages<UseOutcome>>()
            .drain()
            .next()
            .unwrap();
        assert_eq!(refused.0, Err(UseRefusal::OriginTooFar));
    }

    #[test]
    fn the_first_and_third_person_views_measure_the_same_corner() {
        let app_voxels = [([5, 7, 5], Voxel::cube(rock()))];
        let mut first = app_with(&app_voxels, 1.0);
        let mut third = app_with(&app_voxels, 1.0);
        let seen = Vec3::new(5.2, 8.0, 5.8);
        let eye = Vec3::new(5.2, 12.0, 5.8);
        let from_eye = measured(use_from(&mut first, eye, (seen - eye).normalize(), None));
        // Three metres behind and above the Eye, looking at the same point.
        let camera = eye + Vec3::new(2.0, 2.0, 0.0);
        let mut players = third.world_mut().query::<&mut Player>();
        players.single_mut(third.world_mut()).unwrap().set_eye(eye);
        third.world_mut().write_message(UseIntent {
            tool: ToolId(0),
            origin: camera,
            direction: (seen - camera).normalize(),
            expected: None,
        });
        third.world_mut().run_system_once(answer_use).unwrap();
        let from_camera = third
            .world_mut()
            .resource_mut::<Messages<UseOutcome>>()
            .drain()
            .next()
            .unwrap();
        assert_eq!(measured(from_camera), from_eye);
    }

    #[test]
    fn the_preview_answers_like_the_use_it_precedes() {
        let app = app_with(&[([5, 7, 5], Voxel::cube(rock()))], 1.0);
        let preview = |ray, tool| {
            aim_use(
                app.world().resource::<ChunkStore>(),
                &crate::server::fixture_tables(),
                app.world().resource::<WorldSettings>(),
                &measuring(),
                ToolId(tool),
                ray,
                EYE,
            )
        };
        let aimed = preview((EYE, Vec3::NEG_Y), 0).unwrap();
        assert_eq!(aimed.at, [5, 7, 5]);
        assert_eq!(preview((EYE, Vec3::Y), 0), Err(UseRefusal::OutOfReach));
        assert_eq!(preview((EYE, Vec3::NEG_Y), 9), Err(UseRefusal::UnknownTool));
    }

    #[test]
    fn using_a_tool_changes_nothing() {
        let mut app = app_with(&[([5, 7, 5], Voxel::cube(rock()))], 1.0);
        let before = app.world().resource::<ChunkStore>().voxel([5, 7, 5]);
        for _ in 0..3 {
            use_from(&mut app, EYE, Vec3::NEG_Y, None);
        }
        let store = app.world().resource::<ChunkStore>();
        assert_eq!(store.voxel([5, 7, 5]), before);
        assert!(app.world().resource::<edit::PendingEdits>().is_empty());
        assert!(
            app.world()
                .resource::<super::super::undo::Journal>()
                .is_empty()
        );
    }
}
