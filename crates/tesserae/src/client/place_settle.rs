//! A placed block settling in: a short-lived cube over the Voxel that grows
//! from 0.85 to full size, drawn from the moment the Server confirms the Place
//! until the chunk mesh has had time to show. Client-only; the chunk meshing
//! is untouched.

use super::{
    edit::{Holding, Hotbar},
    hand_anim::still,
    menu::Swatches,
    profile::Preferences,
};
use crate::server::edit::{AppliedKind, EditApplied};
use crate::session::SessionApp;
use crate::world::WorldSettings;
use bevy::prelude::*;
use tesserae_core::SHAPE_FULL_CUBE;

const FROM_SCALE: f32 = 0.85;
const SETTLE_S: f32 = 0.12;
/// Placing as fast as Repeat allows must not pile up cubes.
const OVERLAY_CAP: usize = 8;

#[derive(Component)]
struct Settling {
    age_s: f32,
    material: Handle<StandardMaterial>,
}

#[derive(Resource, Default)]
struct Cube(Handle<Mesh>);

pub(super) fn plugin(app: &mut App) {
    app.init_session_state::<Cube>()
        .add_session_systems(Update, (spawn, settle).chain());
}

/// The cube's size as a fraction of the Voxel, `age_s` after the Place landed.
fn scale_at(age_s: f32) -> f32 {
    let t = (age_s / SETTLE_S).clamp(0.0, 1.0);
    let eased = 1.0 - (1.0 - t).powi(3);
    FROM_SCALE + (1.0 - FROM_SCALE) * eased
}

fn spawn(
    mut commands: Commands,
    mut applied: MessageReader<EditApplied>,
    (world, swatches, hotbar): (Res<WorldSettings>, Res<Swatches>, Res<Hotbar>),
    preferences: Option<Res<Preferences>>,
    (mut cube, mut meshes, mut materials): (
        ResMut<Cube>,
        ResMut<Assets<Mesh>>,
        ResMut<Assets<StandardMaterial>>,
    ),
    live: Query<(), With<Settling>>,
) {
    let mut alive = live.iter().count();
    for edit in applied.read() {
        let AppliedKind::Placed(material) = edit.kind else {
            continue;
        };
        // The overlay is a cube, so it is only true for a cube; the Server's
        // message does not name the Shape, and the hand still holds it.
        let held_cube = matches!(
            hotbar.held(),
            Holding::Solid { material: held, shape: SHAPE_FULL_CUBE, .. } if held == material
        );
        if still(&preferences) || !held_cube || alive >= OVERLAY_CAP {
            continue;
        }
        alive += 1;
        if cube.0 == Handle::default() {
            cube.0 = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
        }
        let rgba = swatches.rgba(Holding::solid(material, SHAPE_FULL_CUBE));
        let handle = materials.add(StandardMaterial {
            base_color: Color::linear_rgb(rgba.x, rgba.y, rgba.z),
            unlit: true,
            ..default()
        });
        let centre = Vec3::from_array(edit.at.map(|i| (i as f32 + 0.5) * world.voxel_size));
        commands.spawn((
            Settling {
                age_s: 0.0,
                material: handle.clone(),
            },
            Mesh3d(cube.0.clone()),
            MeshMaterial3d(handle),
            bevy::light::NotShadowCaster,
            Transform::from_translation(centre)
                .with_scale(Vec3::splat(world.voxel_size * scale_at(0.0))),
        ));
    }
}

fn settle(
    mut commands: Commands,
    time: Res<Time>,
    world: Res<WorldSettings>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut overlays: Query<(Entity, &mut Settling, &mut Transform)>,
) {
    for (entity, mut settling, mut transform) in &mut overlays {
        settling.age_s += time.delta_secs();
        if settling.age_s >= SETTLE_S {
            materials.remove(settling.material.id());
            commands.entity(entity).despawn();
            continue;
        }
        transform.scale = Vec3::splat(world.voxel_size * scale_at(settling.age_s));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tesserae_core::Orientation;

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<WorldSettings>()
            .init_resource::<Swatches>()
            .init_resource::<Hotbar>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .add_message::<EditApplied>()
            .add_plugins(plugin);
        hold(&mut app, Holding::solid(3, SHAPE_FULL_CUBE));
        app
    }

    fn hold(app: &mut App, holding: Holding) {
        app.world_mut().resource_mut::<Hotbar>().outside = Some(holding);
    }

    fn placed(app: &mut App, material: u16) {
        app.world_mut().write_message(EditApplied {
            at: [1, 2, 3],
            kind: AppliedKind::Placed(material),
        });
    }

    fn step(app: &mut App, seconds: f32) {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(seconds));
        app.update();
    }

    fn overlays(app: &mut App) -> usize {
        app.world_mut()
            .query_filtered::<(), With<Settling>>()
            .iter(app.world())
            .count()
    }

    #[test]
    fn the_cube_grows_from_85_percent_to_full_size_without_overshoot() {
        assert!((scale_at(0.0) - FROM_SCALE).abs() < 1e-6);
        assert!((scale_at(SETTLE_S) - 1.0).abs() < 1e-6);
        let mut last = scale_at(0.0);
        for n in 1..=24 {
            let now = scale_at(SETTLE_S * n as f32 / 24.0);
            assert!(now >= last && now <= 1.0, "{now} after {last}");
            last = now;
        }
    }

    #[test]
    fn a_confirmed_place_shows_a_cube_that_despawns_when_the_settle_ends() {
        let mut app = app();
        placed(&mut app, 3);
        step(&mut app, 0.0);
        assert_eq!(overlays(&mut app), 1);
        let scale = app
            .world_mut()
            .query_filtered::<&Transform, With<Settling>>()
            .single(app.world())
            .unwrap()
            .scale
            .x;
        let voxel = app.world().resource::<WorldSettings>().voxel_size;
        assert!((scale / voxel - FROM_SCALE).abs() < 1e-4);

        step(&mut app, SETTLE_S * 0.5);
        assert_eq!(overlays(&mut app), 1, "still settling at half time");
        step(&mut app, SETTLE_S * 0.6);
        assert_eq!(overlays(&mut app), 0);
    }

    #[test]
    fn only_a_confirmed_place_of_the_held_cube_settles() {
        let mut app = app();
        app.world_mut().write_message(EditApplied {
            at: [0, 0, 0],
            kind: AppliedKind::Removed(3),
        });
        step(&mut app, 0.0);
        assert_eq!(overlays(&mut app), 0, "a Remove is not a Place");

        hold(
            &mut app,
            Holding::Solid {
                material: 3,
                shape: SHAPE_FULL_CUBE + 1,
                orientation: Orientation::IDENTITY,
            },
        );
        placed(&mut app, 3);
        step(&mut app, 0.0);
        assert_eq!(overlays(&mut app), 0, "a cube would misdraw a slab");

        hold(&mut app, Holding::Fluid(3));
        placed(&mut app, 3);
        step(&mut app, 0.0);
        assert_eq!(overlays(&mut app), 0, "fluid is not a cube");
    }

    #[test]
    fn reduced_motion_places_without_the_settle() {
        let mut app = app();
        app.insert_resource(Preferences {
            reduced_motion: true,
            ..default()
        });
        placed(&mut app, 3);
        step(&mut app, 0.0);
        assert_eq!(overlays(&mut app), 0);
    }

    #[test]
    fn a_burst_of_places_keeps_the_overlays_bounded() {
        let mut app = app();
        for _ in 0..OVERLAY_CAP + 5 {
            placed(&mut app, 3);
        }
        step(&mut app, 0.0);
        assert_eq!(overlays(&mut app), OVERLAY_CAP);
    }
}
