//! Break feedback in the world: a darkening that grows over the block being
//! removed, and a short burst of cubes when the Server confirms the removal.
//! Client-only; the particles are capped and always despawn.

use super::{edit::Holding, menu::Swatches, profile::Preferences, removal::Progress};
use crate::server::edit::{AppliedKind, EditApplied};
use crate::session::SessionApp;
use crate::world::WorldSettings;
use bevy::prelude::*;

const PARTICLES_PER_BREAK: usize = 8;
const PARTICLE_CAP: usize = 40;
const LIFE_S: f32 = 0.4;
const GRAVITY_MPS2: f32 = 9.8;
const SIZE_FRACTION: f32 = 0.12;
const CRACK_MAX_ALPHA: f32 = 0.55;
/// Keeps the darkening off the block's own faces.
const CRACK_INFLATE_M: f32 = 0.006;

#[derive(Component)]
struct Particle {
    velocity: Vec3,
    age_s: f32,
    colour: Color,
    material: Handle<StandardMaterial>,
}

#[derive(Component)]
struct Crack;

#[derive(Resource, Default)]
struct Shared {
    cube: Handle<Mesh>,
    crack: Handle<StandardMaterial>,
}

pub(super) fn plugin(app: &mut App) {
    app.init_session_state::<Shared>()
        .add_session_systems(Update, (burst, fall, crack).after(super::removal::bar));
}

/// A repeatable spread from the Voxel's address, so a burst needs no RNG.
fn velocity(at: [i32; 3], n: usize) -> Vec3 {
    let mut h = (at[0] as u32).wrapping_mul(0x9E37_79B1)
        ^ (at[1] as u32).wrapping_mul(0x85EB_CA6B)
        ^ (at[2] as u32).wrapping_mul(0xC2B2_AE35)
        ^ (n as u32).wrapping_mul(0x27D4_EB2F);
    let mut unit = || {
        h ^= h << 13;
        h ^= h >> 17;
        h ^= h << 5;
        (h % 2001) as f32 / 1000.0 - 1.0
    };
    Vec3::new(unit() * 1.4, 1.0 + unit().abs() * 1.2, unit() * 1.4)
}

fn burst(
    mut commands: Commands,
    mut applied: MessageReader<EditApplied>,
    (world, swatches, preferences): (Res<WorldSettings>, Res<Swatches>, Option<Res<Preferences>>),
    (mut shared, mut meshes, mut materials): (
        ResMut<Shared>,
        ResMut<Assets<Mesh>>,
        ResMut<Assets<StandardMaterial>>,
    ),
    live: Query<(), With<Particle>>,
) {
    let still = preferences.is_some_and(|p| p.reduced_motion);
    let mut alive = live.iter().count();
    for edit in applied.read() {
        let AppliedKind::Removed(material) = edit.kind else {
            continue;
        };
        if still {
            continue;
        }
        if shared.cube == Handle::default() {
            shared.cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
        }
        let rgba = swatches.rgba(Holding::solid(material, 0));
        let colour = Color::linear_rgba(rgba.x, rgba.y, rgba.z, 1.0);
        let size = world.voxel_size * SIZE_FRACTION;
        let centre = Vec3::from_array(edit.at.map(|i| (i as f32 + 0.5) * world.voxel_size));
        for n in 0..PARTICLES_PER_BREAK {
            if alive >= PARTICLE_CAP {
                break;
            }
            alive += 1;
            let velocity = velocity(edit.at, n);
            let handle = materials.add(StandardMaterial {
                base_color: colour,
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            });
            commands.spawn((
                Particle {
                    velocity,
                    age_s: 0.0,
                    colour,
                    material: handle.clone(),
                },
                Mesh3d(shared.cube.clone()),
                MeshMaterial3d(handle),
                bevy::light::NotShadowCaster,
                Transform::from_translation(
                    centre + velocity.normalize_or_zero() * 0.2 * world.voxel_size,
                )
                .with_scale(Vec3::splat(size)),
            ));
        }
    }
}

fn fall(
    mut commands: Commands,
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut particles: Query<(Entity, &mut Particle, &mut Transform)>,
) {
    let dt = time.delta_secs();
    for (entity, mut particle, mut transform) in &mut particles {
        particle.age_s += dt;
        if particle.age_s >= LIFE_S {
            materials.remove(particle.material.id());
            commands.entity(entity).despawn();
            continue;
        }
        particle.velocity.y -= GRAVITY_MPS2 * dt;
        transform.translation += particle.velocity * dt;
        let alpha = 1.0 - particle.age_s / LIFE_S;
        if let Some(mut material) = materials.get_mut(&particle.material) {
            material.base_color = particle.colour.with_alpha(alpha);
        }
    }
}

fn crack(
    mut commands: Commands,
    progress: Res<Progress>,
    (mut shared, mut meshes, mut materials): (
        ResMut<Shared>,
        ResMut<Assets<Mesh>>,
        ResMut<Assets<StandardMaterial>>,
    ),
    mut cracks: Query<(Entity, &mut Transform), With<Crack>>,
) {
    let Some(bounds) = progress.bounds() else {
        for (entity, _) in &cracks {
            commands.entity(entity).despawn();
        }
        return;
    };
    let centre = (bounds.min + bounds.max) * 0.5;
    let size = bounds.max - bounds.min + Vec3::splat(2.0 * CRACK_INFLATE_M);
    if shared.cube == Handle::default() {
        shared.cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    }
    if shared.crack == Handle::default() {
        shared.crack = materials.add(StandardMaterial {
            base_color: Color::srgba(0.0, 0.0, 0.0, 0.0),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        });
    }
    if let Some(mut material) = materials.get_mut(&shared.crack) {
        material.base_color = Color::srgba(0.0, 0.0, 0.0, CRACK_MAX_ALPHA * progress.fraction());
    }
    if let Ok((_, mut transform)) = cracks.single_mut() {
        *transform = Transform::from_translation(centre).with_scale(size);
        return;
    }
    commands.spawn((
        Crack,
        Mesh3d(shared.cube.clone()),
        MeshMaterial3d(shared.crack.clone()),
        bevy::light::NotShadowCaster,
        Transform::from_translation(centre).with_scale(size),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<WorldSettings>()
            .init_resource::<Swatches>()
            .init_resource::<Progress>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .add_message::<EditApplied>()
            .add_plugins(plugin);
        app
    }

    fn step(app: &mut App, seconds: f32) {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(seconds));
        app.update();
    }

    fn particles(app: &mut App) -> usize {
        app.world_mut()
            .query::<&Particle>()
            .iter(app.world())
            .count()
    }

    fn removed(app: &mut App, at: i32) {
        app.world_mut().write_message(EditApplied {
            at: [at, 0, 0],
            kind: AppliedKind::Removed(1),
        });
    }

    #[test]
    fn a_confirmed_removal_bursts_into_a_few_cubes_that_all_despawn_in_time() {
        let mut app = app();
        removed(&mut app, 0);
        step(&mut app, 0.016);
        assert_eq!(particles(&mut app), PARTICLES_PER_BREAK);
        step(&mut app, LIFE_S / 2.0);
        assert_eq!(particles(&mut app), PARTICLES_PER_BREAK);
        step(&mut app, LIFE_S);
        assert_eq!(particles(&mut app), 0);
        assert_eq!(app.world().resource::<Assets<StandardMaterial>>().len(), 0);
    }

    #[test]
    fn placing_spawns_nothing_and_the_burst_count_is_capped() {
        let mut app = app();
        app.world_mut().write_message(EditApplied {
            at: [0; 3],
            kind: AppliedKind::Placed(1),
        });
        step(&mut app, 0.016);
        assert_eq!(particles(&mut app), 0);
        for at in 0..20 {
            removed(&mut app, at);
        }
        step(&mut app, 0.016);
        assert_eq!(particles(&mut app), PARTICLE_CAP);
    }

    #[test]
    fn reduced_motion_spawns_no_particles() {
        let mut app = app();
        app.insert_resource(Preferences {
            reduced_motion: true,
            ..default()
        });
        removed(&mut app, 0);
        step(&mut app, 0.016);
        assert_eq!(particles(&mut app), 0);
    }
}
