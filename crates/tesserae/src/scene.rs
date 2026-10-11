//! A fixed 3×3 grid of generated Chunks, so that step 2 has something to look
//! at. Step 3 deletes this module: Chunk streaming behind the client/server
//! split replaces it, and nothing else depends on it.

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use tesserae_core::mesh::{NEG_X, NEG_Y, NEG_Z, POS_X, POS_Y, POS_Z};
use tesserae_core::{Chunk, Neighborhood, greedy_mesh, testgen};

use crate::client::camera::{CameraHome, FlyCamera};
use crate::client::chunk_mesh::chunk_mesh_to_mesh;
use crate::render::chunk_material::{ChunkMaterial, ChunkMaterialExtension};
use crate::world::{ChunkCoord, WorldSettings};

/// Chunks either side of the origin, so 1 gives a 3×3 grid.
const GRID_RADIUS: i32 = 1;

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, (spawn_chunks, spawn_light, spawn_camera));
}

fn spawn_chunks(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ChunkMaterial>>,
    world: Res<WorldSettings>,
) {
    let coords: Vec<ChunkCoord> = (-GRID_RADIUS..=GRID_RADIUS)
        .flat_map(|x| (-GRID_RADIUS..=GRID_RADIUS).map(move |z| ChunkCoord::new(x, 0, z)))
        .collect();

    // Generated up front because meshing a Chunk reads across its faces: a
    // Chunk meshed before its neighbour exists emits a skin of faces buried in
    // the neighbour's rock.
    let chunks: HashMap<ChunkCoord, Chunk> = coords
        .iter()
        .map(|c| (*c, testgen::terrain_chunk(c.x, c.y, c.z)))
        .collect();

    let material = materials.add(ChunkMaterial {
        base: StandardMaterial {
            perceptual_roughness: 0.95,
            ..default()
        },
        extension: ChunkMaterialExtension::default(),
    });

    for coord in &coords {
        let mut neighbors: [Option<&Chunk>; 6] = [None; 6];
        neighbors[NEG_X] = chunks.get(&ChunkCoord::new(coord.x - 1, coord.y, coord.z));
        neighbors[POS_X] = chunks.get(&ChunkCoord::new(coord.x + 1, coord.y, coord.z));
        neighbors[NEG_Y] = chunks.get(&ChunkCoord::new(coord.x, coord.y - 1, coord.z));
        neighbors[POS_Y] = chunks.get(&ChunkCoord::new(coord.x, coord.y + 1, coord.z));
        neighbors[NEG_Z] = chunks.get(&ChunkCoord::new(coord.x, coord.y, coord.z - 1));
        neighbors[POS_Z] = chunks.get(&ChunkCoord::new(coord.x, coord.y, coord.z + 1));

        let chunk_mesh = greedy_mesh(&Neighborhood::new(&chunks[coord], neighbors));
        if chunk_mesh.is_empty() {
            continue;
        }

        commands.spawn((
            Mesh3d(meshes.add(chunk_mesh_to_mesh(&chunk_mesh, world.voxel_size))),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(world.chunk_origin(*coord)),
            *coord,
        ));
    }
}

fn spawn_light(mut commands: Commands, mut ambient: ResMut<GlobalAmbientLight>) {
    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(1.0, 2.2, 0.8).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // A flat fill standing in for sky light. Without it every face the sun does
    // not reach is black, which hides the Materials on vertical faces — the only
    // place Dirt and Stone are visible at all. Step 4's flood-fill replaces it.
    ambient.brightness = 900.0;
}

fn spawn_camera(mut commands: Commands, world: Res<WorldSettings>) {
    // Above the grid's near corner, far enough back that all nine Cells fit.
    // Expressed in Cells so the framing survives a change of Voxel size.
    let cell = world.cell_size();
    let target = Vec3::new(0.5, 0.6, 0.5) * cell;
    let eye = target + Vec3::new(-2.3, 1.3, -2.3) * cell;

    let pose = Transform::from_translation(eye).looking_at(target, Vec3::Y);
    commands.spawn((
        Camera3d::default(),
        pose,
        FlyCamera::looking_along(target - eye),
        CameraHome(pose),
    ));
}
