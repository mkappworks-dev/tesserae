//! First- and third-person views. The camera is presentation: it never moves
//! the Player, and what an Edit strikes is still the Server's answer, judged
//! from the Eye.
use super::actions::{Action, Actions};
use super::avatar::AvatarShown;
use super::camera::FlyCamera;
use super::chunk_view::ChunkEntities;
use super::preview::HandCamera;
use super::profile::{Preferences, Store};
use crate::world::{ChunkCoord, WorldSettings};
use bevy::ecs::system::SystemParam;
use bevy::mesh::{Indices, VertexAttributeValues};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::ops::RangeInclusive;
use std::sync::Arc;

pub(crate) const ZOOM_M: RangeInclusive<f32> = 1.5..=6.0;
pub(crate) const DEFAULT_ZOOM_M: f32 = 4.0;
const ZOOM_STEP_M: f32 = 0.5;
/// Right of and above the Eye, in the camera's frame. On the look line itself
/// the figure's head would cover what the crosshair points at.
const SHOULDER: Vec3 = Vec3::new(0.5, 0.3, 0.0);
/// Wide enough that the 0.1 m near plane's corners stay out of terrain, and
/// under the 0.18 m between the Eye and the top of the body, so a body
/// touching a ceiling does not start inside it.
const CAMERA_RADIUS_M: f32 = 0.1;
const SKIN_M: f32 = 1e-3;
/// Nearer than this the camera is inside the figure's head.
const HIDE_FIGURE_BELOW_M: f32 = 0.8;
/// Per second. Pulling in is immediate; easing back out is not, so a camera
/// brushing past a pillar does not snap back and forth.
const EASE_OUT_RATE: f32 = 4.0;
/// A zoom gesture is many notches; the profile is written once it ends.
const SAVE_AFTER_S: f32 = 1.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum View {
    #[default]
    FirstPerson,
    ThirdPerson,
}

/// How far behind the shoulder the camera is now; `None` in first-person, so
/// entering third-person cuts straight to the clear length.
#[derive(Resource, Default)]
pub(super) struct Boom(Option<f32>);

#[derive(Resource, Default)]
struct PendingSave(Option<f32>);

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Boom>()
        .init_resource::<PendingSave>()
        .add_systems(
            Update,
            (
                switch.after(super::input::resolve),
                place
                    .after(super::pose::apply_correction)
                    .after(super::camera::look),
                save,
            )
                .chain(),
        );
}

fn switch(
    ownership: Res<super::input::Ownership>,
    actions: Res<Actions>,
    mut preferences: ResMut<Preferences>,
    mut pending: ResMut<PendingSave>,
) {
    if !ownership.gameplay() {
        return;
    }
    let mut view = preferences.view;
    if actions.started(Action::View) {
        view = match view {
            View::FirstPerson => View::ThirdPerson,
            View::ThirdPerson => View::FirstPerson,
        };
    }
    let mut zoom_m = preferences.zoom_m;
    if view == View::ThirdPerson {
        let steps =
            actions.started(Action::ZoomOut) as i32 - actions.started(Action::ZoomIn) as i32;
        zoom_m = (zoom_m + steps as f32 * ZOOM_STEP_M).clamp(*ZOOM_M.start(), *ZOOM_M.end());
    }
    // Written only on a change: display settings re-apply whenever the
    // preferences change, which rebuilds the window's swapchain.
    if (view, zoom_m) != (preferences.view, preferences.zoom_m) {
        preferences.view = view;
        preferences.zoom_m = zoom_m;
        pending.0 = Some(SAVE_AFTER_S);
    }
}

fn save(
    mut commands: Commands,
    time: Res<Time>,
    mut pending: ResMut<PendingSave>,
    store: Option<ResMut<Store>>,
    bindings: Res<super::actions::Bindings>,
    preferences: Res<Preferences>,
) {
    let Some(left) = pending.0.as_mut() else {
        return;
    };
    *left -= time.delta_secs();
    if *left > 0.0 {
        return;
    }
    pending.0 = None;
    if let Some(mut store) = store
        && let Err(error) = store.save(&bindings, &preferences)
    {
        warn!("Could not save the view: {error}");
        super::menu_ui::notify(
            &mut commands,
            "Could not save the view preference",
            super::menu_ui::Tone::Alert,
        );
    }
}

pub(super) fn place(
    time: Res<Time>,
    preferences: Res<Preferences>,
    mut boom: ResMut<Boom>,
    mut shown: ResMut<AvatarShown>,
    camera: Single<(&mut Transform, &mut GlobalTransform, &FlyCamera)>,
    mut hand: Query<&mut Camera, With<HandCamera>>,
    mut obstacles: Obstacles,
) {
    let (mut transform, mut global, fly) = camera.into_inner();
    let third = preferences.view == View::ThirdPerson;
    for mut camera in &mut hand {
        if camera.is_active == third {
            camera.is_active = !third;
        }
    }
    if !third {
        obstacles.forget_unused();
        boom.0 = None;
        shown.0 = false;
        transform.translation = fly.eye;
        *global = GlobalTransform::from(*transform);
        return;
    }
    let rotation = transform.rotation;
    let wanted_shoulder = fly.eye + rotation * SHOULDER;
    let shoulder = fly
        .eye
        .lerp(wanted_shoulder, obstacles.sweep(fly.eye, wanted_shoulder));
    let back = rotation * Vec3::Z;
    let clear =
        preferences.zoom_m * obstacles.sweep(shoulder, shoulder + back * preferences.zoom_m);
    let length = match boom.0 {
        Some(now) if now < clear => {
            now + (clear - now) * (1.0 - (-EASE_OUT_RATE * time.delta_secs()).exp())
        }
        _ => clear,
    };
    boom.0 = Some(length);
    shown.0 = length >= HIDE_FIGURE_BELOW_M;
    transform.translation = shoulder + back * length;
    *global = GlobalTransform::from(*transform);
    obstacles.forget_unused();
}

type QuadBoxes = Arc<[(Vec3, Vec3)]>;

/// The drawn faces of nearby Chunks the Server sent, as axis-aligned boxes.
/// What the camera collides with is what is drawn, never World storage.
#[derive(SystemParam)]
pub(super) struct Obstacles<'w, 's> {
    world: Res<'w, WorldSettings>,
    chunks: Res<'w, ChunkEntities>,
    layers: Query<'w, 's, (&'static Mesh3d, &'static Transform), Without<FlyCamera>>,
    meshes: Res<'w, Assets<Mesh>>,
    cache: Local<'s, HashMap<AssetId<Mesh>, (bool, QuadBoxes)>>,
}

impl Obstacles<'_, '_> {
    /// How far from `from` to `to`, as a fraction, a camera-sized sphere gets
    /// before it touches a drawn face. Faces it starts inside are ignored: a
    /// camera never starts there, and the Eye only by a float's width.
    fn sweep(&mut self, from: Vec3, to: Vec3) -> f32 {
        let pad = Vec3::splat(CAMERA_RADIUS_M);
        let lo = self.world.chunk_at(from.min(to) - pad);
        let hi = self.world.chunk_at(from.max(to) + pad);
        let mut nearest = 1.0f32;
        for x in lo.x..=hi.x {
            for y in lo.y..=hi.y {
                for z in lo.z..=hi.z {
                    let coord = ChunkCoord::new(x, y, z);
                    let entities: Vec<Entity> = self.chunks.solid_at(coord).collect();
                    for entity in entities {
                        let Ok((mesh, transform)) = self.layers.get(entity) else {
                            continue;
                        };
                        let (handle, offset) = (mesh.0.clone(), transform.translation);
                        let Some(boxes) = self.boxes(&handle) else {
                            continue;
                        };
                        for &(min, max) in boxes.iter() {
                            if let Some(t) =
                                enters(from, to, min + offset - pad, max + offset + pad)
                            {
                                nearest = nearest.min(t);
                            }
                        }
                    }
                }
            }
        }
        if nearest < 1.0 {
            // Stop short, so a sweep starting where this one ended starts
            // outside the face rather than on it, where it would be ignored.
            nearest = (nearest - SKIN_M / from.distance(to)).max(0.0);
        }
        nearest
    }

    fn boxes(&mut self, handle: &Handle<Mesh>) -> Option<QuadBoxes> {
        let id = handle.id();
        if let Some((used, boxes)) = self.cache.get_mut(&id) {
            *used = true;
            return Some(boxes.clone());
        }
        let boxes = quad_boxes(self.meshes.get(id)?)?;
        self.cache.insert(id, (true, boxes.clone()));
        Some(boxes)
    }

    fn forget_unused(&mut self) {
        self.cache.retain(|_, (used, _)| std::mem::take(used));
    }
}

/// One box per greedy quad, from the six indices that draw it.
fn quad_boxes(mesh: &Mesh) -> Option<QuadBoxes> {
    let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        return None;
    };
    let Some(Indices::U32(indices)) = mesh.indices() else {
        return None;
    };
    Some(
        indices
            .chunks(6)
            .map(|quad| {
                quad.iter()
                    .map(|&i| Vec3::from_array(positions[i as usize]))
                    .fold((Vec3::MAX, Vec3::MIN), |(lo, hi), p| (lo.min(p), hi.max(p)))
            })
            .collect(),
    )
}

/// Where the segment enters the box, as a fraction of it, if it does from
/// outside.
fn enters(from: Vec3, to: Vec3, min: Vec3, max: Vec3) -> Option<f32> {
    let delta = to - from;
    let mut enter = 0.0f32;
    let mut leave = 1.0f32;
    let mut outside = false;
    for a in 0..3 {
        let beside = from[a] < min[a] || from[a] > max[a];
        outside |= beside;
        if delta[a] == 0.0 {
            if beside {
                return None;
            }
            continue;
        }
        let (t1, t2) = ((min[a] - from[a]) / delta[a], (max[a] - from[a]) / delta[a]);
        enter = enter.max(t1.min(t2));
        leave = leave.min(t1.max(t2));
    }
    (outside && enter <= leave).then_some(enter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::asset::RenderAssetUsages;
    use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
    use bevy::input::touch::TouchPhase;
    use bevy::mesh::PrimitiveTopology;

    const EYE: Vec3 = Vec3::new(5.0, 5.0, 5.0);

    fn app(view: View) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(super::super::actions::test_plugin)
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_millis(250),
            ))
            .insert_resource(super::super::input::Ownership::playing())
            .insert_resource(Preferences { view, ..default() })
            .init_resource::<AvatarShown>()
            .init_resource::<ChunkEntities>()
            .init_resource::<WorldSettings>()
            .init_resource::<Assets<Mesh>>()
            .add_plugins(plugin);
        app.world_mut().spawn((
            Transform::default(),
            GlobalTransform::default(),
            FlyCamera {
                eye: EYE,
                ..default()
            },
        ));
        app.world_mut().spawn((HandCamera, Camera::default()));
        app
    }

    fn camera(app: &mut App) -> Vec3 {
        app.world_mut()
            .query_filtered::<&GlobalTransform, With<FlyCamera>>()
            .single(app.world())
            .unwrap()
            .translation()
    }

    /// A drawn face across the whole Chunk, `z` metres into it.
    fn wall(app: &mut App, z: f32) -> Entity {
        let mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![
                [0.0, 0.0, z],
                [32.0, 0.0, z],
                [32.0, 32.0, z],
                [0.0, 32.0, z],
            ],
        )
        .with_inserted_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]));
        let handle = app.world_mut().resource_mut::<Assets<Mesh>>().add(mesh);
        let entity = app
            .world_mut()
            .spawn((Mesh3d(handle), Transform::default()))
            .id();
        app.world_mut()
            .resource_mut::<ChunkEntities>()
            .insert_opaque(ChunkCoord::new(0, 0, 0), entity);
        entity
    }

    /// Drawn faces across the whole Chunk, each at `at` metres along `axis`,
    /// in the one mesh a Chunk layer has.
    fn faces(app: &mut App, faces: &[(usize, f32)]) {
        let edge = app.world().resource::<WorldSettings>().cell_size();
        let mut positions = Vec::new();
        let mut indices = Vec::new();
        for &(axis, at) in faces {
            let corner = |u: f32, v: f32| {
                let mut p = [0.0; 3];
                p[axis] = at;
                p[(axis + 1) % 3] = u;
                p[(axis + 2) % 3] = v;
                p
            };
            let base = positions.len() as u32;
            positions.extend([
                corner(0.0, 0.0),
                corner(edge, 0.0),
                corner(edge, edge),
                corner(0.0, edge),
            ]);
            indices.extend([0, 1, 2, 0, 2, 3].map(|i| base + i));
        }
        let mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_indices(Indices::U32(indices));
        let handle = app.world_mut().resource_mut::<Assets<Mesh>>().add(mesh);
        let entity = app
            .world_mut()
            .spawn((Mesh3d(handle), Transform::default()))
            .id();
        app.world_mut()
            .resource_mut::<ChunkEntities>()
            .insert_opaque(ChunkCoord::new(0, 0, 0), entity);
    }

    /// A one-metre corridor closed at both ends, with a floor and a ceiling
    /// over the body: turning, pitching and zooming at any update rate never
    /// put the camera into a face, not even for one frame.
    #[test]
    fn a_turning_zooming_camera_never_enters_a_narrow_space_at_any_rate_or_voxel_size() {
        let eye = Vec3::new(4.0, 4.0, 4.0);
        let walls = [(0, 3.5), (0, 4.5), (2, 2.5), (2, 6.0), (1, 2.25), (1, 5.0)];
        for size in [1.0, 0.5, 0.25] {
            for hz in [60.0, 144.0, 1000.0] {
                let mut app = app(View::ThirdPerson);
                app.insert_resource(WorldSettings {
                    voxel_size: size,
                    ..default()
                })
                .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                    std::time::Duration::from_secs_f64(1.0 / hz),
                ));
                faces(&mut app, &walls);
                let frames = (2.0 * hz) as usize;
                for frame in 0..frames {
                    let t = frame as f32 / hz as f32;
                    let (yaw, pitch) = (std::f32::consts::PI * t, 1.2 * (5.0 * t).sin());
                    app.world_mut().resource_mut::<Preferences>().zoom_m =
                        if (t * 2.0) as i32 % 2 == 0 { 6.0 } else { 1.5 };
                    {
                        let mut camera = app
                            .world_mut()
                            .query::<(&mut Transform, &mut FlyCamera)>()
                            .single_mut(app.world_mut())
                            .unwrap();
                        camera.1.eye = eye;
                        camera.0.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);
                    }
                    app.update();
                    let at = camera(&mut app);
                    let lo = Vec3::new(3.5, 2.25, 2.5) + CAMERA_RADIUS_M - 1e-3;
                    let hi = Vec3::new(4.5, 5.0, 6.0) - CAMERA_RADIUS_M + 1e-3;
                    assert!(
                        at.cmpge(lo).all() && at.cmple(hi).all(),
                        "{size} m, {hz} Hz, frame {frame}: camera at {at}"
                    );
                }
            }
        }
    }

    #[test]
    fn easing_back_out_takes_the_same_time_at_any_update_rate() {
        let booms: Vec<f32> = [60.0, 144.0, 1000.0]
            .into_iter()
            .map(|hz| {
                let mut app = app(View::ThirdPerson);
                app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                    std::time::Duration::from_secs_f64(1.0 / hz),
                ));
                faces(&mut app, &[(2, EYE.z + 1.0)]);
                app.update();
                *app.world_mut().resource_mut::<ChunkEntities>() = default();
                for _ in 0..(0.5 * hz) as usize {
                    app.update();
                }
                camera(&mut app).z - EYE.z
            })
            .collect();
        assert!(
            booms[0] > 1.0 && booms[0] < DEFAULT_ZOOM_M - 0.1,
            "{booms:?}"
        );
        for boom in &booms {
            assert!((boom - booms[2]).abs() < 0.05, "{booms:?}");
        }
    }

    #[test]
    fn first_person_sits_at_the_eye_with_the_corner_model_and_no_figure() {
        let mut app = app(View::FirstPerson);
        app.update();
        assert_eq!(camera(&mut app), EYE);
        assert!(!app.world().resource::<AvatarShown>().0);
        let hand = app
            .world_mut()
            .query_filtered::<&Camera, With<HandCamera>>()
            .single(app.world())
            .unwrap();
        assert!(hand.is_active);
    }

    #[test]
    fn third_person_sits_over_the_shoulder_and_a_wall_pulls_it_in_at_once() {
        let mut app = app(View::ThirdPerson);
        app.update();
        let open = EYE + SHOULDER + Vec3::Z * DEFAULT_ZOOM_M;
        assert!(camera(&mut app).abs_diff_eq(open, 1e-4));
        assert!(app.world().resource::<AvatarShown>().0);
        let hand = app
            .world_mut()
            .query_filtered::<&Camera, With<HandCamera>>()
            .single(app.world())
            .unwrap();
        assert!(!hand.is_active, "no corner model in third-person");

        let near = wall(&mut app, EYE.z + 2.0);
        app.update();
        let boom = camera(&mut app).z - EYE.z;
        assert!(
            (boom - (2.0 - CAMERA_RADIUS_M - SKIN_M)).abs() < 1e-4,
            "{boom}"
        );

        app.world_mut().despawn(near);
        wall(&mut app, EYE.z + 0.6);
        app.update();
        assert!(!app.world().resource::<AvatarShown>().0, "inside the head");

        // Clear again, the camera eases back out rather than jumping.
        *app.world_mut().resource_mut::<ChunkEntities>() = default();
        app.update();
        let boom = camera(&mut app).z - EYE.z;
        assert!(boom > 0.6 && boom < DEFAULT_ZOOM_M - 0.1, "{boom}");
        for _ in 0..40 {
            app.update();
        }
        assert!(camera(&mut app).abs_diff_eq(open, 1e-3));
    }

    #[test]
    fn an_idle_third_person_frame_leaves_the_preferences_unchanged() {
        let mut app = app(View::ThirdPerson);
        app.update();
        app.update();
        let tick = app.world().read_change_tick();
        app.update();
        assert!(
            !app.world()
                .resource_ref::<Preferences>()
                .last_changed()
                .is_newer_than(tick, app.world().read_change_tick()),
            "display settings re-apply on every change"
        );
    }

    #[test]
    fn the_view_toggles_zooms_in_steps_and_is_saved_once_the_gesture_ends() {
        let root = std::env::temp_dir().join(format!(
            "tesserae-view-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = root.join("profile.json");
        let mut app = app(View::FirstPerson);
        app.insert_resource(Store {
            path: path.clone(),
            error: None,
        });
        let notch = |app: &mut App, y: f32| {
            app.world_mut().write_message(MouseWheel {
                unit: MouseScrollUnit::Line,
                x: 0.0,
                y,
                window: Entity::PLACEHOLDER,
                phase: TouchPhase::Moved,
            });
            app.update();
        };
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::AltLeft);
        notch(&mut app, -1.0);
        assert_eq!(
            app.world().resource::<Preferences>().zoom_m,
            DEFAULT_ZOOM_M,
            "no zoom in first-person"
        );
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.release(KeyCode::AltLeft);
            keys.press(KeyCode::F5);
        }
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::F5);
        assert_eq!(
            app.world().resource::<Preferences>().view,
            View::ThirdPerson
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::AltLeft);
        notch(&mut app, -1.0);
        assert!(!path.exists(), "not while zooming");
        for _ in 0..8 {
            notch(&mut app, -1.0);
        }
        assert_eq!(app.world().resource::<Preferences>().zoom_m, *ZOOM_M.end());
        notch(&mut app, 1.0);
        assert_eq!(app.world().resource::<Preferences>().zoom_m, 5.5);
        for _ in 0..5 {
            app.update();
        }
        let (store, _, saved) = Store::load(path);
        assert!(store.error.is_none());
        assert_eq!((saved.view, saved.zoom_m), (View::ThirdPerson, 5.5));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_segment_stops_where_it_enters_a_box_and_ignores_one_it_starts_in() {
        let (min, max) = (Vec3::new(2.0, -1.0, -1.0), Vec3::new(3.0, 1.0, 1.0));
        assert_eq!(enters(Vec3::ZERO, Vec3::X * 4.0, min, max), Some(0.5));
        assert_eq!(enters(Vec3::ZERO, Vec3::X, min, max), None, "stops short");
        assert_eq!(
            enters(Vec3::Y * 2.0, Vec3::new(4.0, 2.0, 0.0), min, max),
            None
        );
        assert_eq!(
            enters(Vec3::new(2.5, 0.0, 0.0), Vec3::X * 4.0, min, max),
            None
        );
    }

    #[test]
    fn the_longest_boom_stays_inside_what_the_server_accepts_as_a_camera() {
        let farthest = SHOULDER.length() + ZOOM_M.end();
        assert!(farthest <= crate::server::edit::CAMERA_OFFSET_MAX_M);
        assert!(ZOOM_M.contains(&DEFAULT_ZOOM_M));
    }
}
