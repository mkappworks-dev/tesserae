//! What the Client draws: one entity per Chunk the Server has sent a mesh for.
//!
//! The Client has no access to World state at all — not even read access. It
//! receives a finished `ChunkMesh` and a coordinate to drop, which is what lets
//! `ChunkStore`'s mutators stay private to `crate::server`. See
//! `docs/adr/0002-client-server-from-day-one.md`.
//!
//! Terrain is drawn by one camera per LOD level, all on one target: each level
//! has its own render layer and its own depth, and a finer level's camera runs
//! after a coarser one's, so wherever a finer level has geometry it shows over
//! every coarser one. See `.scratch/2026-10-13-phase-6-lod/spec.md`, decision 8.

use bevy::camera::visibility::RenderLayers;
use bevy::ecs::system::SystemParam;
use bevy::light::NotShadowCaster;
use bevy::mesh::MeshTag;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use tesserae_core::{ChunkMesh, MeshLayer};

use crate::client::blend_order::{BlendOrder, sort_blended};
use crate::client::camera::MainCamera;
use crate::client::chunk_mesh::chunk_mesh_to_mesh;
use crate::render::chunk_material::{ChunkMaterial, ChunkMaterialExtension, LodParams};
use crate::server::streaming::{
    ChunkMeshReady, ChunkUnloaded, LodMeshReady, LodUnloaded, StreamingSettings,
};
use crate::server::{cuts, level_count};
use crate::world::{ChunkCoord, WorldSettings};

/// Level 0, the UI and everything that is not distant terrain.
const NEAR_LAYER: usize = 0;

fn layer_of(level: u32) -> RenderLayers {
    RenderLayers::layer(level as usize)
}

type LayerEntities = HashMap<(u32, ChunkCoord), Entity>;

/// The entities drawing each Chunk of each level, by the level's own lattice, one
/// map per mesh layer. Render entities, not World state, so the Client owns this
/// outright.
#[derive(Resource, Default)]
struct ChunkEntities {
    opaque: LayerEntities,
    blended: LayerEntities,
}

/// Two materials for every Chunk, as the Material index is a vertex attribute:
/// they differ in alpha mode only.
#[derive(Resource)]
struct TerrainMaterial {
    opaque: Handle<ChunkMaterial>,
    blended: Handle<ChunkMaterial>,
}

#[derive(SystemParam)]
struct IncomingMeshes<'w, 's> {
    level0: MessageReader<'w, 's, ChunkMeshReady>,
    coarse: MessageReader<'w, 's, LodMeshReady>,
}

impl IncomingMeshes<'_, '_> {
    fn read(&mut self) -> impl Iterator<Item = (u32, ChunkCoord, &ChunkMesh)> {
        let level0 = self.level0.read().map(|m| (0, m.coord, &m.mesh));
        let coarse = self.coarse.read().map(|m| (m.level, m.coord, &m.mesh));
        level0.chain(coarse)
    }
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<ChunkEntities>()
        .add_systems(Startup, (add_material, spawn_light))
        .add_systems(PostStartup, spawn_camera)
        // Chained: the Server can send a mesh and its unload in one frame, and an
        // unload handled first would leave the mesh with nothing to despawn it.
        .add_systems(
            Update,
            (
                upload_meshes,
                drop_meshes,
                sort_blended.run_if(blend_sort_on),
            )
                .chain(),
        );
}

/// `TESSERAE_BLEND_SORT=0` under `dev` draws blended quads in the mesher's
/// order, so a capture can show what the sort fixes.
#[cfg(feature = "dev")]
fn blend_sort_on() -> bool {
    !std::env::var("TESSERAE_BLEND_SORT").is_ok_and(|v| v == "0")
}

#[cfg(not(feature = "dev"))]
fn blend_sort_on() -> bool {
    true
}

fn add_material(
    mut commands: Commands,
    mut materials: ResMut<Assets<ChunkMaterial>>,
    streaming: Res<StreamingSettings>,
    world: Res<WorldSettings>,
) {
    let opaque = ChunkMaterial {
        base: StandardMaterial {
            perceptual_roughness: 0.95,
            ..default()
        },
        extension: ChunkMaterialExtension {
            lod: LodParams::new(
                debug_tint(),
                &cuts(&streaming, world.cell_size())
                    .iter()
                    .map(|c| (c.radius_m, c.half_height_m))
                    .collect::<Vec<_>>(),
            ),
            ..default()
        },
    };
    let blended = ChunkMaterial {
        base: StandardMaterial {
            alpha_mode: AlphaMode::Blend,
            ..opaque.base.clone()
        },
        extension: opaque.extension.clone(),
    };
    commands.insert_resource(TerrainMaterial {
        opaque: materials.add(opaque),
        blended: materials.add(blended),
    });
}

#[cfg(feature = "dev")]
fn debug_tint() -> u32 {
    std::env::var("TESSERAE_LOD_TINT").is_ok_and(|v| v != "0") as u32
}

#[cfg(not(feature = "dev"))]
fn debug_tint() -> u32 {
    0
}

/// What `upload_meshes` and `drop_meshes` share: the pieces of a Client frame
/// that spawn, replace and despawn a Chunk's entities.
struct Draw<'a, 'w, 's> {
    commands: &'a mut Commands<'w, 's>,
    meshes: &'a mut Assets<Mesh>,
    mesh_of: &'a dyn Fn(Entity) -> Option<Handle<Mesh>>,
}

impl Draw<'_, '_, '_> {
    /// Brings one layer's entity for `key` in line with `layer`: spawned,
    /// replaced or, if the layer has nothing, despawned. The entity, if it
    /// draws something.
    fn show(
        &mut self,
        entities: &mut LayerEntities,
        material: &Handle<ChunkMaterial>,
        layer: &MeshLayer,
        key: (u32, ChunkCoord),
        world: &WorldSettings,
        casts_shadows: bool,
    ) -> Option<Entity> {
        if layer.is_empty() {
            self.hide(entities, key);
            return None;
        }
        let (level, coord) = key;
        let handle = self
            .meshes
            .add(chunk_mesh_to_mesh(layer, world.voxel_size_at(level)));
        match entities.get(&key) {
            // Replacing rather than despawning and respawning keeps the
            // entity id stable, and dropping the old handle is what stops a
            // few minutes of flying from leaking every mesh ever uploaded.
            Some(&entity) => {
                if let Some(old) = (self.mesh_of)(entity) {
                    self.meshes.remove(&old);
                }
                self.commands.entity(entity).insert(Mesh3d(handle));
                Some(entity)
            }
            None => {
                let mut entity = self.commands.spawn((
                    Mesh3d(handle),
                    MeshMaterial3d(material.clone()),
                    Transform::from_translation(world.chunk_origin_at(coord, level)),
                    coord,
                    MeshTag::new(level),
                    layer_of(level),
                ));
                if !casts_shadows {
                    entity.insert(NotShadowCaster);
                }
                entities.insert(key, entity.id());
                Some(entity.id())
            }
        }
    }

    fn hide(&mut self, entities: &mut LayerEntities, key: (u32, ChunkCoord)) {
        let Some(entity) = entities.remove(&key) else {
            return;
        };
        if let Some(mesh) = (self.mesh_of)(entity) {
            self.meshes.remove(&mesh);
        }
        self.commands.entity(entity).despawn();
    }
}

#[cfg_attr(
    feature = "dev",
    expect(
        clippy::too_many_arguments,
        reason = "a Bevy system; the extra parameter is the dev cost counter"
    )
)]
fn upload_meshes(
    mut commands: Commands,
    mut incoming: IncomingMeshes,
    mut entities: ResMut<ChunkEntities>,
    mut meshes: ResMut<Assets<Mesh>>,
    material: Res<TerrainMaterial>,
    world: Res<WorldSettings>,
    existing: Query<&Mesh3d>,
    #[cfg(feature = "dev")] mut cost: ResMut<crate::cost::MainThreadCost>,
) {
    #[cfg(feature = "dev")]
    let started = std::time::Instant::now();
    #[cfg(feature = "dev")]
    let mut uploaded = 0;
    let entities = &mut *entities;
    let mut draw = Draw {
        commands: &mut commands,
        meshes: &mut meshes,
        mesh_of: &|entity| existing.get(entity).ok().map(|m| m.0.clone()),
    };
    for (level, coord, mesh) in incoming.read() {
        let key = (level, coord);
        let opaque = draw.show(
            &mut entities.opaque,
            &material.opaque,
            &mesh.opaque,
            key,
            &world,
            true,
        );
        let blended = draw.show(
            &mut entities.blended,
            &material.blended,
            &mesh.blended,
            key,
            &world,
            // The flood-fill already dims sky light under water and glass, so
            // a shadow map treating them as solid would count it twice.
            false,
        );
        if let Some(entity) = blended {
            let order = BlendOrder::new(&mesh.blended, world.voxel_size_at(level));
            match order {
                Some(order) => draw.commands.entity(entity).insert(order),
                None => draw.commands.entity(entity).remove::<BlendOrder>(),
            };
        }
        #[cfg(feature = "dev")]
        {
            uploaded += u32::from(opaque.is_some()) + u32::from(blended.is_some());
        }
        #[cfg(not(feature = "dev"))]
        let _ = opaque;
    }
    #[cfg(feature = "dev")]
    if uploaded > 0 {
        cost.uploaded(started.elapsed(), uploaded);
    }
}

fn drop_meshes(
    mut commands: Commands,
    mut level0: MessageReader<ChunkUnloaded>,
    mut coarse: MessageReader<LodUnloaded>,
    mut entities: ResMut<ChunkEntities>,
    mut meshes: ResMut<Assets<Mesh>>,
    existing: Query<&Mesh3d>,
) {
    let level0 = level0.read().map(|m| (0, m.coord));
    let coarse = coarse.read().map(|m| (m.level, m.coord));
    let entities = &mut *entities;
    let mut draw = Draw {
        commands: &mut commands,
        meshes: &mut meshes,
        mesh_of: &|entity| existing.get(entity).ok().map(|m| m.0.clone()),
    };
    for key in level0.chain(coarse) {
        draw.hide(&mut entities.opaque, key);
        draw.hide(&mut entities.blended, key);
    }
}

fn spawn_light(
    mut commands: Commands,
    mut ambient: ResMut<GlobalAmbientLight>,
    settings: Res<StreamingSettings>,
) {
    let sun = Transform::from_xyz(1.0, 2.2, 0.8).looking_at(Vec3::ZERO, Vec3::Y);
    // A light's layers are also the shadow-caster mask for every view it lights,
    // so one sun on every layer would put level 0 into the coarse cameras'
    // cascades for nothing: two suns, one shadowed.
    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: true,
            ..default()
        },
        sun,
        RenderLayers::layer(NEAR_LAYER),
    ));
    let coarse = (1..level_count(&settings).max(2)).map(|level| level as usize);
    commands.spawn((
        DirectionalLight::default(),
        sun,
        RenderLayers::from_layers(&coarse.collect::<Vec<_>>()),
    ));

    // The sky dome's radiance, no longer a flat fill: the chunk shader gates it
    // per face by the flood-fill's sky channel, so a face that cannot see the
    // sky does not receive it. The sun's own occlusion is the shadow map plus
    // the same sky channel; see `render/chunk_material.wesl`.
    ambient.brightness = 900.0;
}

/// How far the view tips down. Framing is a Client decision; where the Eye
/// starts is the Server's, so the Client's camera starts where the Player does.
const VIEW_PITCH: f32 = -0.3;

/// A coarse camera's near plane, as a fraction of the level-0 radius. The finer
/// levels always cover what it clips, so a coarse peak cannot fill the view of
/// an Eye hovering beside it.
const COARSE_NEAR_FRACTION: f32 = 0.25;

/// In `PostStartup`, so the Server's `Startup` commands have been applied and
/// [`SpawnPoint`] exists. Ordering through the schedule rather than through an
/// edge between two plugins' systems, which would have to be kept in step.
///
/// [`SpawnPoint`]: crate::server::world_state::SpawnPoint
fn spawn_camera(
    mut commands: Commands,
    spawn: Res<crate::server::world_state::SpawnPoint>,
    streaming: Res<StreamingSettings>,
    world: Res<WorldSettings>,
) {
    let eye = spawn.eye();
    let direction = Vec3::new(0.0, VIEW_PITCH, -1.0).normalize();
    let pose = Transform::from_translation(eye).looking_to(direction, Vec3::Y);

    // At least one, even with no coarse levels: the camera at the bottom of the
    // stack is the one that clears, and the Player's camera never does.
    let coarse_levels = level_count(&streaming).max(2) - 1;

    let near = commands
        .spawn((
            Camera3d::default(),
            Camera {
                order: coarse_levels as isize,
                clear_color: ClearColorConfig::None,
                ..default()
            },
            RenderLayers::layer(NEAR_LAYER),
            MainCamera,
            // The UI must target the camera that runs last, or the coarse layers
            // draw over it.
            IsDefaultUiCamera,
            pose,
            // Propagation runs in `PostUpdate`; without this the first frame's pose
            // report and intent would carry the origin, not the camera.
            GlobalTransform::from(pose),
            super::camera::FlyCamera::looking_along(direction),
            super::camera::CameraHome(pose),
        ))
        .id();

    for level in 1..=coarse_levels {
        commands.spawn((
            Camera3d::default(),
            Camera {
                // Coarsest first, so each finer level draws over the last.
                order: (coarse_levels - level) as isize,
                clear_color: if level == coarse_levels {
                    ClearColorConfig::Default
                } else {
                    ClearColorConfig::None
                },
                ..default()
            },
            layer_of(level),
            Projection::Perspective(PerspectiveProjection {
                near: streaming.view_distance_m * COARSE_NEAR_FRACTION,
                far: streaming.lod_distance_m.max(streaming.view_distance_m) + world.cell_size(),
                ..default()
            }),
            // Identity under the near camera, so it cannot drift from it.
            ChildOf(near),
            GlobalTransform::from(pose),
            CoarseCamera,
        ));
    }
}

#[derive(Component)]
struct CoarseCamera;

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::mesh::VertexAttributeValues;
    use tesserae_core::light::{Light, MAX_LIGHT};
    use tesserae_core::{Chunk, ChunkLight, LitChunk, Tables, Voxel, mesh_chunk, worldgen};

    fn camera_app(eye: Vec3, streaming: StreamingSettings) -> App {
        let mut app = App::new();
        app.insert_resource(crate::server::world_state::SpawnPoint::for_tests(eye))
            .insert_resource(streaming)
            .init_resource::<WorldSettings>();
        app.world_mut().run_system_once(spawn_camera).unwrap();
        app
    }

    #[test]
    fn the_camera_is_where_it_is_placed_before_the_first_propagation() {
        let mut app = camera_app(Vec3::new(3.0, 4.0, 5.0), StreamingSettings::default());
        let (local, global) = app
            .world_mut()
            .query_filtered::<(&Transform, &GlobalTransform), With<MainCamera>>()
            .single(app.world())
            .unwrap();
        assert_eq!(global.translation(), local.translation);
        assert_ne!(global.translation(), Vec3::ZERO);
    }

    /// Every coarse camera, finest level first.
    fn coarse_cameras(app: &mut App) -> Vec<(Entity, Camera, RenderLayers, ChildOf, Transform)> {
        let mut rows: Vec<_> = app
            .world_mut()
            .query_filtered::<(Entity, &Camera, &RenderLayers, &ChildOf, &Transform), With<CoarseCamera>>()
            .iter(app.world())
            .map(|(e, c, l, p, t)| (e, c.clone(), l.clone(), p.clone(), *t))
            .collect();
        rows.sort_by_key(|row| std::cmp::Reverse(row.1.order));
        rows
    }

    #[test]
    fn there_is_a_camera_for_each_coarse_level_under_the_main_camera_at_its_origin() {
        let mut app = camera_app(Vec3::new(3.0, 4.0, 5.0), StreamingSettings::default());
        let main = app
            .world_mut()
            .query_filtered::<Entity, With<MainCamera>>()
            .single(app.world())
            .unwrap();
        let cameras = coarse_cameras(&mut app);
        assert_eq!(
            cameras.len() as u32,
            level_count(&StreamingSettings::default()) - 1
        );
        let main_at = app
            .world()
            .get::<GlobalTransform>(main)
            .unwrap()
            .translation();
        for (i, (entity, _, layers, parent, transform)) in cameras.iter().enumerate() {
            assert_eq!(parent.parent(), main);
            assert_eq!(*transform, Transform::IDENTITY);
            assert_eq!(
                app.world()
                    .get::<GlobalTransform>(*entity)
                    .unwrap()
                    .translation(),
                main_at
            );
            assert_eq!(*layers, RenderLayers::layer(i + 1), "one level a camera");
        }
    }

    fn projection_of<F: bevy::ecs::query::QueryFilter>(
        app: &mut App,
    ) -> Vec<PerspectiveProjection> {
        app.world_mut()
            .query_filtered::<&Projection, F>()
            .iter(app.world())
            .map(|projection| {
                let Projection::Perspective(p) = projection else {
                    panic!("every camera is perspective");
                };
                p.clone()
            })
            .collect()
    }

    #[test]
    fn the_coarse_projections_reach_the_lod_distance_and_start_a_quarter_out() {
        let streaming = StreamingSettings::default();
        let mut app = camera_app(Vec3::ZERO, streaming);
        let cell = WorldSettings::default().cell_size();
        let near = projection_of::<With<MainCamera>>(&mut app).remove(0);
        let coarse = projection_of::<With<CoarseCamera>>(&mut app);
        assert!(!coarse.is_empty());
        for far in coarse {
            assert!(far.far >= streaming.lod_distance_m + cell);
            assert_eq!(far.near, streaming.view_distance_m / 4.0);
            assert_eq!(far.fov, near.fov, "a seam between the layers otherwise");
        }
    }

    #[test]
    fn a_world_with_no_coarse_levels_still_has_a_far_plane_past_its_view() {
        let streaming = StreamingSettings {
            lod_distance_m: 100.0,
            ..default()
        };
        let mut app = camera_app(Vec3::ZERO, streaming);
        let coarse = projection_of::<With<CoarseCamera>>(&mut app);
        assert_eq!(coarse.len(), 1, "something has to clear the target");
        assert!(coarse[0].far > streaming.view_distance_m);
    }

    #[test]
    fn finer_levels_draw_over_coarser_ones_and_the_near_camera_over_all_and_owns_the_ui() {
        let mut app = camera_app(Vec3::ZERO, StreamingSettings::default());
        let (near, near_ui, near_layers) = {
            let mut query = app
                .world_mut()
                .query_filtered::<(&Camera, Has<IsDefaultUiCamera>, &RenderLayers), With<MainCamera>>();
            let (c, ui, l) = query.single(app.world()).unwrap();
            (c.clone(), ui, l.clone())
        };
        assert_eq!(near_layers, RenderLayers::layer(0));
        assert!(near_ui);
        assert!(matches!(near.clear_color, ClearColorConfig::None));

        let cameras = coarse_cameras(&mut app);
        let mut above = near.order;
        for (i, (entity, camera, _, _, _)) in cameras.iter().enumerate() {
            assert!(camera.order < above, "level {}", i + 1);
            above = camera.order;
            assert!(!app.world().entity(*entity).contains::<IsDefaultUiCamera>());
            let clears = matches!(camera.clear_color, ClearColorConfig::Default);
            assert_eq!(clears, i + 1 == cameras.len(), "level {}", i + 1);
        }
    }

    #[test]
    fn the_shadowed_sun_lights_level_zero_only_and_the_other_every_coarse_level() {
        let streaming = StreamingSettings::default();
        let mut app = App::new();
        app.init_resource::<GlobalAmbientLight>()
            .insert_resource(streaming);
        app.world_mut().run_system_once(spawn_light).unwrap();
        let mut suns = app
            .world_mut()
            .query::<(&DirectionalLight, &RenderLayers, &Transform)>();
        let suns: Vec<_> = suns.iter(app.world()).collect();
        assert_eq!(suns.len(), 2);
        let (shadowed, plain): (Vec<_>, Vec<_>) = suns
            .iter()
            .partition(|(light, _, _)| light.shadow_maps_enabled);
        let (_, near_layers, near_pose) = shadowed[0];
        let (_, far_layers, far_pose) = plain[0];
        assert_eq!(*near_layers, RenderLayers::layer(NEAR_LAYER));
        assert_eq!(near_pose, far_pose, "one sun, lit twice");
        assert!(!far_layers.intersects(&RenderLayers::layer(NEAR_LAYER)));
        for level in 1..level_count(&streaming) {
            assert!(far_layers.intersects(&RenderLayers::layer(level as usize)));
        }
    }

    /// `MinimalPlugins` rather than `DefaultPlugins`: these systems only need
    /// assets and messages, and `DefaultPlugins` wants a window.
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<ChunkMaterial>()
            .init_resource::<WorldSettings>()
            .init_resource::<StreamingSettings>()
            .init_resource::<ChunkEntities>()
            .add_message::<ChunkMeshReady>()
            .add_message::<ChunkUnloaded>()
            .add_message::<LodMeshReady>()
            .add_message::<LodUnloaded>()
            .add_systems(Startup, add_material)
            .add_systems(Update, (upload_meshes, drop_meshes, sort_blended).chain());
        app.update();
        app
    }

    /// Two panes of glass one behind the other along z, in one Chunk.
    fn two_panes() -> tesserae_core::ChunkMesh {
        let mut c = Chunk::air();
        c.set(5, 5, 5, Voxel::cube(tesserae_core::material::GLASS));
        c.set(5, 5, 10, Voxel::cube(tesserae_core::material::GLASS));
        let air = Chunk::air();
        let sky = ChunkLight::filled(Light::new(MAX_LIGHT, 0));
        mesh_chunk(
            LitChunk::new(&c, &sky),
            [Some(LitChunk::new(&air, &sky)); 6],
            &Tables::builtin(),
        )
    }

    /// Two pools side by side along x in a stone basin: two water quads in one
    /// plane, facing up, and nothing else blended.
    fn two_pools() -> tesserae_core::ChunkMesh {
        use tesserae_core::material::{STONE, WATER};
        let mut c = Chunk::air();
        for x in 0..=12 {
            for z in 0..=6 {
                c.set(x, 4, z, Voxel::cube(STONE));
                let rim = x == 0 || x == 6 || x == 12 || z == 0 || z == 6;
                c.set(x, 5, z, Voxel::cube(if rim { STONE } else { WATER }));
            }
        }
        let air = Chunk::air();
        let sky = ChunkLight::filled(Light::new(MAX_LIGHT, 0));
        let mesh = mesh_chunk(
            LitChunk::new(&c, &sky),
            [Some(LitChunk::new(&air, &sky)); 6],
            &Tables::builtin(),
        );
        assert_eq!(mesh.blended.quad_count(), 2, "the fixture is two pools");
        mesh
    }

    fn put_eye(app: &mut App, eye: Vec3) {
        let existing = app
            .world_mut()
            .query_filtered::<Entity, With<MainCamera>>()
            .iter(app.world())
            .next();
        let transform = GlobalTransform::from_translation(eye);
        match existing {
            Some(camera) => {
                app.world_mut().entity_mut(camera).insert(transform);
            }
            None => {
                app.world_mut().spawn((MainCamera, transform));
            }
        }
        app.update();
    }

    /// The z of each quad in the order it is drawn.
    fn drawn_z(app: &App, entity: Entity) -> Vec<f32> {
        let handle = &app.world().get::<Mesh3d>(entity).unwrap().0;
        let mesh = app.world().resource::<Assets<Mesh>>().get(handle).unwrap();
        let positions = positions(app, entity);
        let Some(bevy::mesh::Indices::U32(indices)) = mesh.indices() else {
            panic!("U32 indices");
        };
        indices
            .chunks(6)
            .map(|quad| quad.iter().map(|&i| positions[i as usize][2]).sum::<f32>() / 6.0)
            .collect()
    }

    fn geometry() -> tesserae_core::ChunkMesh {
        let solid = Chunk::filled(Voxel::cube(worldgen::STONE));
        let air = Chunk::air();
        let dark = ChunkLight::dark();
        let sky = ChunkLight::filled(Light::new(MAX_LIGHT, 0));
        let mut neighbors = [Some(LitChunk::new(&solid, &dark)); 6];
        neighbors[0] = Some(LitChunk::new(&air, &sky));
        let mesh = mesh_chunk(LitChunk::new(&solid, &dark), neighbors, &Tables::builtin());
        assert!(!mesh.is_empty(), "the fixture must have geometry");
        mesh
    }

    /// A Chunk of water beside open sky: all of its geometry is blended.
    fn blended_geometry() -> tesserae_core::ChunkMesh {
        let water = Chunk::filled(Voxel::cube(tesserae_core::material::WATER));
        let air = Chunk::air();
        let dark = ChunkLight::dark();
        let sky = ChunkLight::filled(Light::new(MAX_LIGHT, 0));
        let mut neighbors = [Some(LitChunk::new(&water, &dark)); 6];
        neighbors[0] = Some(LitChunk::new(&air, &sky));
        let mesh = mesh_chunk(LitChunk::new(&water, &dark), neighbors, &Tables::builtin());
        assert!(mesh.opaque.is_empty(), "the fixture must be all blended");
        assert!(!mesh.blended.is_empty(), "the fixture must have geometry");
        mesh
    }

    /// Stone and water in one Chunk, so both layers have geometry.
    fn both_layers() -> tesserae_core::ChunkMesh {
        tesserae_core::ChunkMesh {
            blended: blended_geometry().blended,
            ..geometry()
        }
    }

    fn send_mesh(app: &mut App, coord: ChunkCoord, mesh: tesserae_core::ChunkMesh) {
        app.world_mut()
            .write_message(ChunkMeshReady { coord, mesh });
        app.update();
    }

    fn send_coarse(app: &mut App, level: u32, coord: ChunkCoord, mesh: tesserae_core::ChunkMesh) {
        app.world_mut()
            .write_message(LodMeshReady { level, coord, mesh });
        app.update();
    }

    fn entity_at(app: &App, level: u32, coord: ChunkCoord) -> Option<Entity> {
        app.world()
            .resource::<ChunkEntities>()
            .opaque
            .get(&(level, coord))
            .copied()
    }

    fn blended_at(app: &App, level: u32, coord: ChunkCoord) -> Option<Entity> {
        app.world()
            .resource::<ChunkEntities>()
            .blended
            .get(&(level, coord))
            .copied()
    }

    fn entity_for(app: &App, coord: ChunkCoord) -> Option<Entity> {
        entity_at(app, 0, coord)
    }

    fn drawn(app: &mut App) -> usize {
        app.world_mut()
            .query_filtered::<Entity, With<Mesh3d>>()
            .iter(app.world())
            .count()
    }

    fn positions(app: &App, entity: Entity) -> Vec<[f32; 3]> {
        let handle = &app.world().get::<Mesh3d>(entity).unwrap().0;
        let mesh = app.world().resource::<Assets<Mesh>>().get(handle).unwrap();
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("positions are Float32x3");
        };
        positions.clone()
    }

    #[test]
    fn a_chunk_with_only_blended_geometry_is_drawn() {
        let mut app = app();
        send_mesh(&mut app, ChunkCoord::new(0, 0, 0), blended_geometry());
        assert_eq!(drawn(&mut app), 1);
    }

    #[test]
    fn a_chunk_with_both_layers_is_two_entities_and_one_with_only_opaque_is_one() {
        let mut app = app();
        send_mesh(&mut app, ChunkCoord::new(0, 0, 0), both_layers());
        assert_eq!(drawn(&mut app), 2);
        send_mesh(&mut app, ChunkCoord::new(1, 0, 0), geometry());
        assert_eq!(drawn(&mut app), 3, "no blended Voxel is still one entity");
    }

    #[test]
    fn a_blended_entity_is_placed_scaled_tagged_and_layered_like_the_opaque_one() {
        let mut app = app();
        let coord = ChunkCoord::new(3, -1, 2);
        send_mesh(&mut app, coord, both_layers());
        send_coarse(&mut app, 2, coord, both_layers());
        for level in [0, 2] {
            let opaque = entity_at(&app, level, coord).unwrap();
            let blended = blended_at(&app, level, coord).expect("a blended entity");
            assert_ne!(opaque, blended);
            let world = app.world();
            assert_eq!(
                world.get::<Transform>(blended),
                world.get::<Transform>(opaque),
                "level {level}"
            );
            assert_eq!(world.get::<ChunkCoord>(blended), Some(&coord));
            assert_eq!(world.get::<MeshTag>(blended).unwrap().value, level);
            assert_eq!(
                world.get::<RenderLayers>(blended),
                Some(&layer_of(level)),
                "the level's camera draws it, or the cut and the sun miss it"
            );
        }
        // A coarse Voxel is bigger, so the same layer spans more metres.
        let extent = |app: &App, entity| {
            let max = |p: Vec<[f32; 3]>| p.iter().map(|p| p[0]).fold(f32::MIN, f32::max);
            max(positions(app, entity))
        };
        let near = extent(&app, blended_at(&app, 0, coord).unwrap());
        let far = extent(&app, blended_at(&app, 2, coord).unwrap());
        assert_eq!(far, near * 4.0);
    }

    #[test]
    fn blended_surfaces_cast_no_shadow_and_opaque_ones_still_do() {
        use bevy::light::NotShadowCaster;
        let mut app = app();
        let coord = ChunkCoord::new(0, 0, 0);
        send_mesh(&mut app, coord, both_layers());
        send_mesh(&mut app, ChunkCoord::new(1, 0, 0), blended_geometry());
        let world = app.world();
        for blended in [
            blended_at(&app, 0, coord).unwrap(),
            blended_at(&app, 0, ChunkCoord::new(1, 0, 0)).unwrap(),
        ] {
            assert!(world.get::<NotShadowCaster>(blended).is_some());
        }
        assert!(
            world
                .get::<NotShadowCaster>(entity_for(&app, coord).unwrap())
                .is_none()
        );
    }

    #[test]
    fn blended_quads_are_drawn_farthest_first_from_wherever_the_eye_is() {
        let mut app = app();
        let coord = ChunkCoord::new(0, 0, 0);
        put_eye(&mut app, Vec3::new(5.5, 5.5, 60.0));
        send_mesh(&mut app, coord, two_panes());
        let entity = blended_at(&app, 0, coord).unwrap();
        let z = drawn_z(&app, entity);
        assert!(z.is_sorted(), "seen from +z, the far pane first: {z:?}");

        put_eye(&mut app, Vec3::new(5.5, 5.5, -60.0));
        let z = drawn_z(&app, entity);
        assert!(z.is_sorted_by(|a, b| a >= b), "seen from -z: {z:?}");
    }

    #[test]
    fn a_blended_mesh_is_not_resorted_until_the_eye_moves_a_voxel() {
        let mut app = app();
        let coord = ChunkCoord::new(0, 0, 0);
        put_eye(&mut app, Vec3::new(5.5, 5.5, 60.0));
        send_mesh(&mut app, coord, two_panes());
        let entity = blended_at(&app, 0, coord).unwrap();
        // Between the panes, the near face of each is nearest in turn.
        put_eye(&mut app, Vec3::new(5.5, 5.5, 8.4));
        let sorted = drawn_z(&app, entity);
        put_eye(&mut app, Vec3::new(5.5, 5.5, 7.9));
        assert_eq!(drawn_z(&app, entity), sorted, "half a Voxel is not enough");
        put_eye(&mut app, Vec3::new(5.5, 5.5, 6.9));
        assert_ne!(drawn_z(&app, entity), sorted, "a Voxel and a half is");
    }

    /// Far away, a few metres of movement turns the view by a fraction of a
    /// degree, which cannot change which of two quads is in front.
    #[test]
    fn a_distant_blended_mesh_waits_for_the_eye_to_move_in_proportion() {
        let mut app = app();
        let coord = ChunkCoord::new(0, 0, 0);
        put_eye(&mut app, Vec3::new(5.5, 5.5, 300.0));
        send_mesh(&mut app, coord, two_panes());
        let entity = blended_at(&app, 0, coord).unwrap();
        let sorted = drawn_z(&app, entity);
        // To the far side in one step resorts; a few Voxels sideways from there
        // does not, though it would have near by.
        put_eye(&mut app, Vec3::new(5.5, 5.5, -300.0));
        let flipped = drawn_z(&app, entity);
        assert_ne!(flipped, sorted);
        let handle = app.world().get::<Mesh3d>(entity).unwrap().0.clone();
        app.world_mut()
            .resource_mut::<Messages<AssetEvent<Mesh>>>()
            .clear();
        put_eye(&mut app, Vec3::new(9.5, 5.5, -300.0));
        let modified = app
            .world_mut()
            .resource_mut::<Messages<AssetEvent<Mesh>>>()
            .drain()
            .any(|e| e.is_modified(&handle));
        assert!(!modified, "4 m at 300 m is not worth a sort");
    }

    #[test]
    fn a_flat_sheet_of_water_is_never_reindexed() {
        let mut app = app();
        let coord = ChunkCoord::new(0, 0, 0);
        put_eye(&mut app, Vec3::new(60.0, 50.0, 3.0));
        send_mesh(&mut app, coord, two_pools());
        let entity = blended_at(&app, 0, coord).unwrap();
        let handle = app.world().get::<Mesh3d>(entity).unwrap().0.clone();
        app.world_mut()
            .resource_mut::<Messages<AssetEvent<Mesh>>>()
            .clear();
        put_eye(&mut app, Vec3::new(-60.0, 50.0, 3.0));
        let modified = app
            .world_mut()
            .resource_mut::<Messages<AssetEvent<Mesh>>>()
            .drain()
            .any(|e| e.is_modified(&handle));
        assert!(!modified, "a sheet's draw order cannot matter");
    }

    #[test]
    fn only_the_blended_entity_is_alpha_blended() {
        let mut app = app();
        let coord = ChunkCoord::new(0, 0, 0);
        send_mesh(&mut app, coord, both_layers());
        let alpha_of = |app: &App, entity: Entity| {
            let handle = &app
                .world()
                .get::<MeshMaterial3d<ChunkMaterial>>(entity)
                .unwrap()
                .0;
            app.world()
                .resource::<Assets<ChunkMaterial>>()
                .get(handle)
                .unwrap()
                .base
                .alpha_mode
        };
        assert_eq!(
            alpha_of(&app, blended_at(&app, 0, coord).unwrap()),
            AlphaMode::Blend
        );
        assert_eq!(
            alpha_of(&app, entity_for(&app, coord).unwrap()),
            AlphaMode::Opaque
        );
    }

    #[test]
    fn a_replacement_that_drops_or_adds_the_blended_layer_follows_it() {
        let mut app = app();
        let coord = ChunkCoord::new(0, 0, 0);
        send_mesh(&mut app, coord, both_layers());
        let blended = blended_at(&app, 0, coord).unwrap();
        let handle = app.world().get::<Mesh3d>(blended).unwrap().0.clone();

        send_mesh(&mut app, coord, geometry());
        assert_eq!(blended_at(&app, 0, coord), None);
        assert_eq!(drawn(&mut app), 1, "only the opaque entity is left");
        assert!(
            app.world()
                .resource::<Assets<Mesh>>()
                .get(&handle)
                .is_none(),
            "the dropped layer's Mesh asset is still loaded"
        );

        send_mesh(&mut app, coord, both_layers());
        assert!(blended_at(&app, 0, coord).is_some());
        assert_eq!(drawn(&mut app), 2);

        send_mesh(&mut app, coord, blended_geometry());
        assert_eq!(entity_for(&app, coord), None, "the opaque layer is gone");
        assert_eq!(drawn(&mut app), 1);
    }

    #[test]
    fn unloading_removes_both_entities_and_both_assets() {
        let mut app = app();
        let coord = ChunkCoord::new(0, 0, 0);
        send_mesh(&mut app, coord, both_layers());
        send_coarse(&mut app, 1, coord, both_layers());
        let before = app.world().resource::<Assets<Mesh>>().len();
        assert_eq!(before, 4);

        app.world_mut().write_message(ChunkUnloaded { coord });
        app.world_mut()
            .write_message(LodUnloaded { level: 1, coord });
        app.update();

        assert_eq!(drawn(&mut app), 0);
        assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 0);
    }

    #[test]
    fn a_mesh_with_a_blended_layer_and_its_unload_in_one_frame_leave_nothing_drawn() {
        let mut app = app();
        let coord = ChunkCoord::new(0, 0, 0);
        app.world_mut().write_message(ChunkMeshReady {
            coord,
            mesh: both_layers(),
        });
        app.world_mut().write_message(ChunkUnloaded { coord });
        app.update();
        assert_eq!(drawn(&mut app), 0);
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
    fn a_coarse_chunk_sits_at_its_levels_cell_and_is_scaled_by_its_levels_voxel() {
        let mut app = app();
        let coord = ChunkCoord::new(1, -1, 2);
        send_mesh(&mut app, coord, geometry());
        send_coarse(&mut app, 2, coord, geometry());

        let coarse = entity_at(&app, 2, coord).expect("an entity for the coarse Chunk");
        let fine = entity_for(&app, coord).expect("level 0 is a different entity");
        assert_ne!(coarse, fine);

        let world = *app.world().resource::<WorldSettings>();
        assert_eq!(
            app.world().get::<Transform>(coarse).unwrap().translation,
            Vec3::new(128.0, -128.0, 256.0)
        );
        assert_eq!(
            app.world().get::<Transform>(coarse).unwrap().translation,
            world.chunk_origin_at(coord, 2)
        );
        let fine_positions = positions(&app, fine);
        let coarse_positions = positions(&app, coarse);
        assert_eq!(fine_positions.len(), coarse_positions.len());
        for (f, c) in fine_positions.iter().zip(&coarse_positions) {
            assert_eq!(*c, f.map(|v| v * 4.0));
        }
    }

    #[test]
    fn the_material_carries_each_levels_cut_in_its_own_slot() {
        let lod = LodParams::new(
            1,
            &[
                (0.0, 0.0),
                (144.0, 94.0),
                (290.0, 190.0),
                (580.0, 380.0),
                (1160.0, 760.0),
            ],
        );
        assert_eq!(lod.tint, 1);
        assert_eq!(lod.cut_radius_m[0], Vec4::new(0.0, 144.0, 290.0, 580.0));
        assert_eq!(lod.cut_half_height_m[0], Vec4::new(0.0, 94.0, 190.0, 380.0));
        assert_eq!(lod.cut_radius_m[1].x, 1160.0);
        assert_eq!(
            lod.cut_radius_m[1].y, 0.0,
            "a level past the last is not cut"
        );
    }

    #[test]
    fn level_zero_is_on_the_near_layer_and_level_k_on_layer_k_with_its_tag() {
        let mut app = app();
        let coord = ChunkCoord::new(0, 0, 0);
        send_mesh(&mut app, coord, geometry());
        for level in [1, 3] {
            send_coarse(&mut app, level, coord, geometry());
        }

        let check = |app: &App, level: u32| {
            let entity = entity_at(app, level, coord).unwrap();
            let world = app.world();
            assert_eq!(world.get::<MeshTag>(entity).unwrap().value, level);
            assert_eq!(
                world.get::<RenderLayers>(entity),
                Some(&RenderLayers::layer(level as usize)),
                "level {level}"
            );
        };
        for level in [0, 1, 3] {
            check(&app, level);
        }
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
    fn a_second_coarse_mesh_replaces_the_first_and_drops_its_asset() {
        let mut app = app();
        let coord = ChunkCoord::new(4, 0, 1);
        send_coarse(&mut app, 1, coord, geometry());
        let entity = entity_at(&app, 1, coord).unwrap();
        let first = app.world().get::<Mesh3d>(entity).unwrap().0.clone();

        send_coarse(&mut app, 1, coord, geometry());

        assert_eq!(entity_at(&app, 1, coord), Some(entity));
        assert_eq!(drawn(&mut app), 1);
        assert!(
            app.world().resource::<Assets<Mesh>>().get(&first).is_none(),
            "the superseded coarse Mesh asset is still loaded"
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
    fn an_empty_coarse_mesh_despawns_only_its_own_level() {
        let mut app = app();
        let coord = ChunkCoord::new(0, 0, 0);
        send_mesh(&mut app, coord, geometry());
        send_coarse(&mut app, 1, coord, geometry());
        send_coarse(&mut app, 2, coord, geometry());
        let handle = app
            .world()
            .get::<Mesh3d>(entity_at(&app, 1, coord).unwrap())
            .unwrap()
            .0
            .clone();

        send_coarse(&mut app, 1, coord, tesserae_core::ChunkMesh::default());

        assert_eq!(entity_at(&app, 1, coord), None);
        assert!(entity_at(&app, 2, coord).is_some());
        assert!(entity_for(&app, coord).is_some());
        assert_eq!(drawn(&mut app), 2);
        assert!(
            app.world()
                .resource::<Assets<Mesh>>()
                .get(&handle)
                .is_none()
        );
    }

    #[test]
    fn an_empty_mesh_for_an_unknown_chunk_is_not_an_error() {
        let mut app = app();
        send_mesh(
            &mut app,
            ChunkCoord::new(9, 9, 9),
            tesserae_core::ChunkMesh::default(),
        );
        send_coarse(
            &mut app,
            3,
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
    fn unloading_a_coarse_chunk_frees_it_and_leaves_the_other_levels_alone() {
        let mut app = app();
        let coord = ChunkCoord::new(-3, 1, 4);
        send_mesh(&mut app, coord, geometry());
        send_coarse(&mut app, 1, coord, geometry());
        let handle = app
            .world()
            .get::<Mesh3d>(entity_at(&app, 1, coord).unwrap())
            .unwrap()
            .0
            .clone();

        app.world_mut()
            .write_message(LodUnloaded { level: 1, coord });
        app.update();

        assert_eq!(entity_at(&app, 1, coord), None);
        assert!(entity_for(&app, coord).is_some());
        assert_eq!(drawn(&mut app), 1);
        assert!(
            app.world()
                .resource::<Assets<Mesh>>()
                .get(&handle)
                .is_none()
        );
    }

    #[test]
    fn a_mesh_and_its_unload_in_one_frame_leave_nothing_drawn() {
        let mut app = app();
        let coord = ChunkCoord::new(2, 0, -1);
        let world = app.world_mut();
        world.write_message(ChunkMeshReady {
            coord,
            mesh: geometry(),
        });
        world.write_message(ChunkUnloaded { coord });
        world.write_message(LodMeshReady {
            level: 2,
            coord,
            mesh: geometry(),
        });
        world.write_message(LodUnloaded { level: 2, coord });
        app.update();

        assert_eq!(entity_for(&app, coord), None);
        assert_eq!(entity_at(&app, 2, coord), None);
        assert_eq!(drawn(&mut app), 0);
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
