//! What the Client draws: one entity per Chunk the Server has sent a mesh for.
//!
//! The Client has no access to World state at all — not even read access. It
//! receives a finished `ChunkMesh` and a coordinate to drop, which is what lets
//! `ChunkStore`'s mutators stay private to `crate::server`. See
//! `docs/adr/0002-client-server-from-day-one.md`.

use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use crate::client::chunk_mesh::chunk_mesh_to_mesh;
use crate::render::chunk_material::{ChunkMaterial, ChunkMaterialExtension};
use crate::server::streaming::{ChunkMeshReady, ChunkUnloaded};
use crate::world::{ChunkCoord, WorldSettings};

/// The entity drawing each Chunk. Render entities, not World state, so the
/// Client owns this outright.
#[derive(Resource, Default)]
struct ChunkEntities(HashMap<ChunkCoord, Entity>);

/// One material for every Chunk, as the Material index is a vertex attribute.
#[derive(Resource)]
struct TerrainMaterial(Handle<ChunkMaterial>);

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<ChunkEntities>()
        .add_systems(Startup, (add_material, spawn_light))
        .add_systems(PostStartup, spawn_camera)
        .add_systems(Update, (upload_meshes, drop_meshes));
}

fn add_material(mut commands: Commands, mut materials: ResMut<Assets<ChunkMaterial>>) {
    commands.insert_resource(TerrainMaterial(materials.add(ChunkMaterial {
        base: StandardMaterial {
            perceptual_roughness: 0.95,
            ..default()
        },
        extension: ChunkMaterialExtension::default(),
    })));
}

fn upload_meshes(
    mut commands: Commands,
    mut incoming: MessageReader<ChunkMeshReady>,
    mut entities: ResMut<ChunkEntities>,
    mut meshes: ResMut<Assets<Mesh>>,
    material: Res<TerrainMaterial>,
    world: Res<WorldSettings>,
    existing: Query<&Mesh3d>,
) {
    for ChunkMeshReady { coord, mesh } in incoming.read() {
        if mesh.is_empty() {
            despawn(&mut commands, &mut meshes, &mut entities, &existing, *coord);
            continue;
        }

        let handle = meshes.add(chunk_mesh_to_mesh(mesh, world.voxel_size));
        match entities.0.get(coord) {
            // Replacing rather than despawning and respawning keeps the
            // entity id stable, and dropping the old handle is what stops a
            // few minutes of flying from leaking every mesh ever uploaded.
            Some(&entity) => {
                if let Ok(old) = existing.get(entity) {
                    meshes.remove(&old.0);
                }
                commands.entity(entity).insert(Mesh3d(handle));
            }
            None => {
                let entity = commands
                    .spawn((
                        Mesh3d(handle),
                        MeshMaterial3d(material.0.clone()),
                        Transform::from_translation(world.chunk_origin(*coord)),
                        *coord,
                    ))
                    .id();
                entities.0.insert(*coord, entity);
            }
        }
    }
}

fn drop_meshes(
    mut commands: Commands,
    mut outgoing: MessageReader<ChunkUnloaded>,
    mut entities: ResMut<ChunkEntities>,
    mut meshes: ResMut<Assets<Mesh>>,
    existing: Query<&Mesh3d>,
) {
    for ChunkUnloaded { coord } in outgoing.read() {
        despawn(&mut commands, &mut meshes, &mut entities, &existing, *coord);
    }
}

fn despawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    entities: &mut ChunkEntities,
    existing: &Query<&Mesh3d>,
    coord: ChunkCoord,
) {
    let Some(entity) = entities.0.remove(&coord) else {
        return;
    };
    if let Ok(mesh) = existing.get(entity) {
        meshes.remove(&mesh.0);
    }
    commands.entity(entity).despawn();
}

fn spawn_light(mut commands: Commands, mut ambient: ResMut<GlobalAmbientLight>) {
    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(1.0, 2.2, 0.8).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // The sky dome's radiance, no longer a flat fill: the chunk shader gates it
    // per face by the flood-fill's sky channel, so a face that cannot see the
    // sky does not receive it. The sun's own occlusion is the shadow map plus
    // the same sky channel; see `render/chunk_material.wesl`.
    ambient.brightness = 900.0;
}

/// Metres above the spawn point, and how far the view tips down. Framing is a
/// Client decision; the spawn point is the Server's. In metres, so the framing
/// survives a change of Voxel size.
const VIEW_ALTITUDE_M: f32 = 18.0;
const VIEW_PITCH: f32 = -0.3;

/// In `PostStartup`, so the Server's `Startup` commands have been applied and
/// [`SpawnPoint`] exists. Ordering through the schedule rather than through an
/// edge between two plugins' systems, which would have to be kept in step.
///
/// [`SpawnPoint`]: crate::server::world_state::SpawnPoint
fn spawn_camera(mut commands: Commands, spawn: Res<crate::server::world_state::SpawnPoint>) {
    let eye = spawn.metres() + Vec3::Y * VIEW_ALTITUDE_M;
    let direction = Vec3::new(0.0, VIEW_PITCH, -1.0).normalize();
    let pose = Transform::from_translation(eye).looking_to(direction, Vec3::Y);

    commands.spawn((
        Camera3d::default(),
        pose,
        super::camera::FlyCamera::looking_along(direction),
        super::camera::CameraHome(pose),
        crate::server::streaming::StreamingAnchor,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use tesserae_core::light::{Light, MAX_LIGHT};
    use tesserae_core::{Chunk, ChunkLight, LitChunk, ShapeTable, Voxel, mesh_chunk, worldgen};

    /// `MinimalPlugins` rather than `DefaultPlugins`: these systems only need
    /// assets and messages, and `DefaultPlugins` wants a window.
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<ChunkMaterial>()
            .init_resource::<WorldSettings>()
            .init_resource::<ChunkEntities>()
            .add_message::<ChunkMeshReady>()
            .add_message::<ChunkUnloaded>()
            .add_systems(Startup, add_material)
            .add_systems(Update, (upload_meshes, drop_meshes));
        app.update();
        app
    }

    fn geometry() -> tesserae_core::ChunkMesh {
        let solid = Chunk::filled(Voxel::cube(worldgen::STONE));
        let air = Chunk::air();
        let dark = ChunkLight::dark();
        let sky = ChunkLight::filled(Light::new(MAX_LIGHT, 0));
        let mut neighbors = [Some(LitChunk::new(&solid, &dark)); 6];
        neighbors[0] = Some(LitChunk::new(&air, &sky));
        let mesh = mesh_chunk(
            LitChunk::new(&solid, &dark),
            neighbors,
            &ShapeTable::builtin(),
        );
        assert!(!mesh.is_empty(), "the fixture must have geometry");
        mesh
    }

    fn send_mesh(app: &mut App, coord: ChunkCoord, mesh: tesserae_core::ChunkMesh) {
        app.world_mut()
            .write_message(ChunkMeshReady { coord, mesh });
        app.update();
    }

    fn entity_for(app: &App, coord: ChunkCoord) -> Option<Entity> {
        app.world()
            .resource::<ChunkEntities>()
            .0
            .get(&coord)
            .copied()
    }

    fn drawn(app: &mut App) -> usize {
        app.world_mut()
            .query_filtered::<Entity, With<Mesh3d>>()
            .iter(app.world())
            .count()
    }

    #[test]
    fn a_mesh_with_geometry_spawns_one_chunk_entity_at_its_origin() {
        let mut app = app();
        let coord = ChunkCoord::new(2, 0, -1);
        send_mesh(&mut app, coord, geometry());

        let entity = entity_for(&app, coord).expect("an entity for the Chunk");
        assert_eq!(drawn(&mut app), 1);
        let world = *app.world().resource::<WorldSettings>();
        let transform = app.world().get::<Transform>(entity).expect("a Transform");
        assert_eq!(transform.translation, world.chunk_origin(coord));
        assert_eq!(app.world().get::<ChunkCoord>(entity), Some(&coord));
    }

    #[test]
    fn a_second_mesh_replaces_the_first_and_drops_its_asset() {
        let mut app = app();
        let coord = ChunkCoord::new(0, 0, 0);
        send_mesh(&mut app, coord, geometry());

        let entity = entity_for(&app, coord).unwrap();
        let first = app.world().get::<Mesh3d>(entity).unwrap().0.clone();

        send_mesh(&mut app, coord, geometry());
        assert_eq!(entity_for(&app, coord), Some(entity), "entity id moved");
        assert_eq!(drawn(&mut app), 1, "spawned a second entity");

        let second = app.world().get::<Mesh3d>(entity).unwrap().0.clone();
        assert_ne!(second, first, "the mesh was not replaced");
        // The leak this guards: a few minutes of flying re-meshes the same
        // Chunk many times over.
        assert!(
            app.world().resource::<Assets<Mesh>>().get(&first).is_none(),
            "the superseded Mesh asset is still loaded"
        );
    }

    #[test]
    fn an_empty_mesh_despawns_rather_than_drawing_nothing() {
        let mut app = app();
        let coord = ChunkCoord::new(0, 0, 0);
        send_mesh(&mut app, coord, geometry());
        assert_eq!(drawn(&mut app), 1);

        send_mesh(&mut app, coord, tesserae_core::ChunkMesh::default());
        assert_eq!(entity_for(&app, coord), None);
        assert_eq!(drawn(&mut app), 0);
    }

    #[test]
    fn an_empty_mesh_for_an_unknown_chunk_is_not_an_error() {
        let mut app = app();
        send_mesh(
            &mut app,
            ChunkCoord::new(9, 9, 9),
            tesserae_core::ChunkMesh::default(),
        );
        assert_eq!(drawn(&mut app), 0);
    }

    #[test]
    fn unloading_despawns_and_frees_the_mesh() {
        let mut app = app();
        let coord = ChunkCoord::new(-3, 1, 4);
        send_mesh(&mut app, coord, geometry());
        let handle = app
            .world()
            .get::<Mesh3d>(entity_for(&app, coord).unwrap())
            .unwrap()
            .0
            .clone();

        app.world_mut().write_message(ChunkUnloaded { coord });
        app.update();

        assert_eq!(entity_for(&app, coord), None);
        assert_eq!(drawn(&mut app), 0);
        assert!(
            app.world()
                .resource::<Assets<Mesh>>()
                .get(&handle)
                .is_none()
        );
    }

    #[test]
    fn unloading_a_chunk_leaves_its_neighbours_alone() {
        let mut app = app();
        let kept = ChunkCoord::new(0, 0, 0);
        let dropped = ChunkCoord::new(1, 0, 0);
        send_mesh(&mut app, kept, geometry());
        send_mesh(&mut app, dropped, geometry());
        assert_eq!(drawn(&mut app), 2);

        app.world_mut()
            .write_message(ChunkUnloaded { coord: dropped });
        app.update();

        assert_eq!(drawn(&mut app), 1);
        assert!(entity_for(&app, kept).is_some());
    }
}
