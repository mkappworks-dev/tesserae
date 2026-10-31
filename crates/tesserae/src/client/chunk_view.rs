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
//!
//! A mesh the Server unloads stays drawn until other levels cover its region;
//! see `docs/adr/0019-a-level-is-drawn-until-another-replaces-it.md`.

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::ecs::system::SystemParam;
use bevy::light::NotShadowCaster;
use bevy::mesh::MeshTag;
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;

use crate::session::SessionApp;
use bevy::render::storage::ShaderBuffer;
use tesserae_core::{ChunkMesh, MeshLayer};

use crate::client::blend_order::{BlendOrder, sort_blended};
use crate::client::camera::MainCamera;
use crate::client::chunk_mesh::chunk_mesh_to_mesh;
use crate::client::drawn_mask::DrawnMask;
use crate::render::chunk_material::{ChunkMaterial, ChunkMaterialExtension, LodParams};
use crate::render::fog::{self, FogSettings};
use crate::server::streaming::{
    ChunkMeshReady, ChunkUnloaded, LodMeshReady, LodUnloaded, StreamingSettings,
};
use crate::server::{LevelRings, level_count};
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
pub(super) struct ChunkEntities {
    opaque: LayerEntities,
    blended: LayerEntities,
    cutout: LayerEntities,
    /// Every `(level, coord)` sent a mesh, empty or not, and not unloaded since:
    /// the space each level has answered for.
    delivered: HashSet<(u32, ChunkCoord)>,
    /// Unloaded by the Server and still drawn, until `release` finds its region
    /// covered.
    retained: HashSet<(u32, ChunkCoord)>,
    /// Delivered and retained together, as the shader reads them.
    mask: DrawnMask,
    /// Since `publish_drawn` last ran.
    drawn_changed: bool,
    /// Something was delivered or unloaded since `release` last ran.
    release_due: bool,
    /// Each level's anchor when `release` last ran.
    anchors: Vec<ChunkCoord>,
    /// The words `publish_drawn` last wrote, for the hole count.
    #[cfg(feature = "profiling")]
    published: Vec<u32>,
}

impl ChunkEntities {
    /// Whether the Chunk is drawn, delivered or retained.
    #[cfg(feature = "profiling")]
    pub(super) fn draws(&self, key: (u32, ChunkCoord)) -> bool {
        self.delivered.contains(&key) || self.retained.contains(&key)
    }

    #[cfg(feature = "profiling")]
    pub(super) fn published(&self) -> &[u32] {
        &self.published
    }

    fn deliver(&mut self, key: (u32, ChunkCoord)) {
        self.retained.remove(&key);
        if self.delivered.insert(key) {
            self.mask.set(key, true);
            self.drawn_changed = true;
        }
        self.release_due = true;
    }

    /// Kept drawn until `release` finds its region covered.
    fn unload(&mut self, key: (u32, ChunkCoord)) {
        if self.delivered.remove(&key) {
            self.retained.insert(key);
            self.release_due = true;
        }
    }

    /// Whether some coarser level's delivered Chunk draws all of this one's
    /// region.
    fn under_delivered(&self, (level, coord): (u32, ChunkCoord), levels: u32) -> bool {
        let mut covering = coord;
        (level + 1..levels).any(|coarser| {
            covering = parent(covering);
            self.delivered.contains(&(coarser, covering))
        })
    }

    /// Whether finer levels' delivered Chunks draw all of this one's region.
    fn over_delivered(&self, (level, coord): (u32, ChunkCoord)) -> bool {
        level > 0
            && children(coord).all(|child| {
                let key = (level - 1, child);
                self.delivered.contains(&key) || self.over_delivered(key)
            })
    }

    #[cfg(feature = "profiling")]
    pub(super) fn entity_count(&self) -> usize {
        self.opaque.len() + self.blended.len() + self.cutout.len()
    }

    /// The level-0 layers a camera cannot pass through: water and glass are
    /// blended and let it in.
    pub(super) fn solid_at(&self, coord: ChunkCoord) -> impl Iterator<Item = Entity> + '_ {
        [&self.opaque, &self.cutout]
            .into_iter()
            .filter_map(move |layer| layer.get(&(0, coord)).copied())
    }
    #[cfg(test)]
    pub(super) fn insert_opaque(&mut self, coord: ChunkCoord, entity: Entity) {
        self.opaque.insert((0, coord), entity);
    }
}

pub(super) fn terrain_arrived(entities: Option<Res<ChunkEntities>>) -> bool {
    entities.is_some_and(|e| !e.opaque.is_empty())
}

/// A material per mesh layer, for every Chunk, as the Material index is a
/// vertex attribute: they differ in alpha mode, and share the record of what
/// each level draws.
#[derive(Resource)]
struct TerrainMaterial {
    opaque: Handle<ChunkMaterial>,
    blended: Handle<ChunkMaterial>,
    cutout: Handle<ChunkMaterial>,
    drawn: Handle<ShaderBuffer>,
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
    app.init_session_state::<ChunkEntities>()
        .init_session_state::<super::material_textures::Textures>()
        .add_session_systems(Startup, (add_material, spawn_light))
        .add_session_systems(PostStartup, spawn_camera);
    add_view_systems(app);
}

/// One place for the per-frame chain, so the tests run the one the app does.
///
/// Chained: the Server can send a mesh and its unload in one frame, and an
/// unload handled first would leave the mesh with nothing to despawn it; and
/// what each level has drawn is published after both.
fn add_view_systems(app: &mut App) {
    app.init_session_state::<super::material_textures::Textures>();
    app.add_session_systems(
        Update,
        (
            update_textures.after(super::material_textures::receive),
            upload_meshes,
            drop_meshes,
            release,
            sort_blended.run_if(blend_sort_on),
            publish_drawn,
        )
            .chain(),
    );
}

fn update_textures(
    textures: Res<super::material_textures::Textures>,
    terrain: Res<TerrainMaterial>,
    mut materials: ResMut<Assets<ChunkMaterial>>,
) {
    if !textures.is_changed() {
        return;
    }
    for handle in [&terrain.opaque, &terrain.blended, &terrain.cutout] {
        if let Some(mut material) = materials.get_mut(handle) {
            textures.apply(&mut material.extension);
        }
    }
}

/// `TESSERAE_BLEND_SORT=0` under `dev` draws blended quads in the mesher's
/// order, so a capture can show what the sort fixes.
#[cfg(any(feature = "dev", feature = "profiling"))]
fn blend_sort_on() -> bool {
    !std::env::var("TESSERAE_BLEND_SORT").is_ok_and(|v| v == "0")
}

#[cfg(not(any(feature = "dev", feature = "profiling")))]
fn blend_sort_on() -> bool {
    true
}

fn add_material(
    mut commands: Commands,
    mut materials: ResMut<Assets<ChunkMaterial>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
    mut palette: MessageReader<crate::content::MaterialPalette>,
) {
    let mut extension = ChunkMaterialExtension::default();
    for message in palette.read() {
        extension.apply_palette(message);
    }
    let drawn = buffers.add(ShaderBuffer::new(vec![0u32], RenderAssetUsages::default()));
    let opaque = ChunkMaterial {
        base: StandardMaterial {
            perceptual_roughness: 0.95,
            ..default()
        },
        extension: ChunkMaterialExtension {
            lod: LodParams { tint: debug_tint() },
            drawn: drawn.clone(),
            ..extension
        },
    };
    let blended = ChunkMaterial {
        base: StandardMaterial {
            alpha_mode: AlphaMode::Blend,
            // The mesher hides water against water, so from inside a lake the
            // surface's underside is the only water face there is.
            double_sided: true,
            cull_mode: None,
            ..opaque.base.clone()
        },
        extension: opaque.extension.clone(),
    };
    let cutout = ChunkMaterial {
        base: StandardMaterial {
            alpha_mode: AlphaMode::Mask(0.5),
            ..opaque.base.clone()
        },
        extension: opaque.extension.clone(),
    };
    commands.insert_resource(TerrainMaterial {
        opaque: materials.add(opaque),
        blended: materials.add(blended),
        cutout: materials.add(cutout),
        drawn,
    });
}

#[cfg(any(feature = "dev", feature = "profiling"))]
fn debug_tint() -> u32 {
    std::env::var("TESSERAE_LOD_TINT").is_ok_and(|v| v != "0") as u32
}

#[cfg(not(any(feature = "dev", feature = "profiling")))]
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
    any(feature = "dev", feature = "profiling"),
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
    #[cfg(any(feature = "dev", feature = "profiling"))] mut cost: ResMut<
        crate::cost::MainThreadCost,
    >,
) {
    #[cfg(any(feature = "dev", feature = "profiling"))]
    let started = cost.enabled().then(std::time::Instant::now);
    #[cfg(any(feature = "dev", feature = "profiling"))]
    let mut uploaded = 0;
    let entities = &mut *entities;
    let mut draw = Draw {
        commands: &mut commands,
        meshes: &mut meshes,
        mesh_of: &|entity| existing.get(entity).ok().map(|m| m.0.clone()),
    };
    for (level, coord, mesh) in incoming.read() {
        let key = (level, coord);
        entities.deliver(key);
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
        let cutout = draw.show(
            &mut entities.cutout,
            &material.cutout,
            &mesh.cutout,
            key,
            &world,
            true,
        );
        if let Some(entity) = blended {
            let order = BlendOrder::new(&mesh.blended, world.voxel_size_at(level));
            match order {
                Some(order) => draw.commands.entity(entity).insert(order),
                None => draw.commands.entity(entity).remove::<BlendOrder>(),
            };
        }
        #[cfg(any(feature = "dev", feature = "profiling"))]
        {
            uploaded += [opaque, blended, cutout]
                .iter()
                .map(|e| u32::from(e.is_some()))
                .sum::<u32>();
        }
        #[cfg(not(any(feature = "dev", feature = "profiling")))]
        let _ = (opaque, cutout);
    }
    #[cfg(any(feature = "dev", feature = "profiling"))]
    if let Some(started) = started.filter(|_| uploaded > 0) {
        cost.uploaded(started.elapsed(), uploaded);
    }
}

fn drop_meshes(
    mut level0: MessageReader<ChunkUnloaded>,
    mut coarse: MessageReader<LodUnloaded>,
    mut entities: ResMut<ChunkEntities>,
) {
    let level0 = level0.read().map(|m| (0, m.coord));
    let coarse = coarse.read().map(|m| (m.level, m.coord));
    for key in level0.chain(coarse) {
        entities.unload(key);
    }
}

/// Despawns each retained mesh whose region other levels now cover, or which
/// lies outside every level's mesh extent around the camera. Its bit clears in
/// the same frame, so nothing is drawn twice or not at all.
fn release(
    mut commands: Commands,
    mut entities: ResMut<ChunkEntities>,
    mut meshes: ResMut<Assets<Mesh>>,
    existing: Query<&Mesh3d>,
    streaming: Res<StreamingSettings>,
    world: Res<WorldSettings>,
    camera: Query<&GlobalTransform, With<MainCamera>>,
) {
    let levels = level_count(&streaming);
    let camera_m = camera.iter().next().map(GlobalTransform::translation);
    let anchors: Vec<ChunkCoord> = camera_m
        .map(|at| (0..levels).map(|l| world.chunk_at_level(at, l)).collect())
        .unwrap_or_default();
    let entities = &mut *entities;
    if entities.retained.is_empty()
        || !(std::mem::take(&mut entities.release_due) || entities.anchors != anchors)
    {
        entities.anchors = anchors;
        return;
    }
    let outside = |(level, coord): (u32, ChunkCoord)| {
        !anchors.is_empty()
            && (level..levels).all(|at| {
                let covering = ancestor(coord, at - level);
                let rings = LevelRings::new(*streaming, world.cell_size(), at);
                let a = anchors[at as usize];
                let d = IVec3::new(covering.x - a.x, covering.y - a.y, covering.z - a.z);
                let outer = rings.mesh.outer;
                // `contains` squares the offset, which overflows a teleport away.
                d.abs().max_element() > outer.horizontal.max(outer.vertical) || !outer.contains(d)
            })
    };
    let released: Vec<_> = entities
        .retained
        .iter()
        .copied()
        .filter(|&key| {
            entities.under_delivered(key, levels) || entities.over_delivered(key) || outside(key)
        })
        .collect();
    let mut draw = Draw {
        commands: &mut commands,
        meshes: &mut meshes,
        mesh_of: &|entity| existing.get(entity).ok().map(|m| m.0.clone()),
    };
    for key in released {
        draw.hide(&mut entities.opaque, key);
        draw.hide(&mut entities.blended, key);
        draw.hide(&mut entities.cutout, key);
        entities.retained.remove(&key);
        entities.mask.set(key, false);
        entities.drawn_changed = true;
    }
    entities.anchors = anchors;
}

fn parent(c: ChunkCoord) -> ChunkCoord {
    ancestor(c, 1)
}

fn ancestor(c: ChunkCoord, up: u32) -> ChunkCoord {
    ChunkCoord::new(c.x >> up, c.y >> up, c.z >> up)
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

/// Rewrites the record of what each level draws, when it changed. Padded to a
/// power of two so its size, and with it the GPU buffer, rarely changes: Bevy
/// writes a same-sized buffer in place and remakes the materials' bind groups
/// only when it replaces one. Touching the materials instead would mark every
/// Chunk mesh changed.
fn publish_drawn(
    mut entities: ResMut<ChunkEntities>,
    terrain: Res<TerrainMaterial>,
    world: Res<WorldSettings>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    if !std::mem::take(&mut entities.drawn_changed) {
        return;
    }
    let mut words = entities.mask.words(&world);
    words.resize(words.len().next_power_of_two().max(MIN_DRAWN_WORDS), 0);
    if let Some(mut buffer) = buffers.get_mut(&terrain.drawn) {
        buffer.clear();
        buffer.extend_from_slice(&words);
    }
    #[cfg(feature = "profiling")]
    {
        entities.published = words;
    }
}

/// 16 KiB, so the mask grows past its buffer rarely.
const MIN_DRAWN_WORDS: usize = 4096;

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
    fog_settings: Res<FogSettings>,
) {
    let eye = spawn.entry_eye();
    let direction = Vec3::new(0.0, VIEW_PITCH, -1.0).normalize();
    let pose = Transform::from_translation(eye).looking_to(direction, Vec3::Y);

    // At least one, even with no coarse levels: the camera at the bottom of the
    // stack is the one that clears, and the Player's camera never does.
    let coarse_levels = level_count(&streaming).max(2) - 1;

    // Built once and cloned onto every camera: the same fragment must fog the
    // same whichever layer draws it.
    let fog = fog::camera_fog(
        fog::enabled(),
        &fog_settings,
        streaming.view_distance_m,
        streaming.lod_distance_m,
    );

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
            super::camera::FlyCamera {
                eye,
                ..super::camera::FlyCamera::looking_along(direction)
            },
            super::camera::CameraHome(pose),
        ))
        .id();
    if let Some(fog) = &fog {
        commands.entity(near).insert(fog.clone());
    }

    for level in 1..=coarse_levels {
        let coarse = commands
            .spawn((
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
                // The Player's near plane: a coarse Chunk is drawn wherever no
                // finer one is, however close to the Eye.
                Projection::Perspective(PerspectiveProjection {
                    far: streaming.lod_distance_m.max(streaming.view_distance_m)
                        + world.cell_size(),
                    ..default()
                }),
                // Identity under the near camera, so it cannot drift from it.
                ChildOf(near),
                GlobalTransform::from(pose),
                CoarseCamera,
            ))
            .id();
        if let Some(fog) = &fog {
            commands.entity(coarse).insert(fog.clone());
        }
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
    use tesserae_core::{Chunk, ChunkLight, LitChunk, MaterialId, Tables, Voxel, mesh_chunk};

    fn watered() -> (Tables, MaterialId) {
        let tables = crate::server::fixture_tables();
        let water = tesserae_core::testgen::fixtures().water;
        (tables, water)
    }

    fn camera_app(eye: Vec3, streaming: StreamingSettings) -> App {
        let mut app = App::new();
        app.insert_resource(crate::server::world_state::SpawnPoint::for_tests(eye))
            .insert_resource(streaming)
            .init_resource::<FogSettings>()
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

    fn terrain_fogs(app: &mut App) -> Vec<Option<DistanceFog>> {
        app.world_mut()
            .query_filtered::<Option<&DistanceFog>, With<Camera3d>>()
            .iter(app.world())
            .map(|fog| fog.cloned())
            .collect()
    }

    fn assert_same_fog(a: &DistanceFog, b: &DistanceFog) {
        use bevy::pbr::FogFalloff::ExponentialSquared;
        assert_eq!(a.color, b.color);
        assert_eq!(a.directional_light_color, b.directional_light_color);
        assert_eq!(a.directional_light_exponent, b.directional_light_exponent);
        let (ExponentialSquared { density: da }, ExponentialSquared { density: db }) =
            (&a.falloff, &b.falloff)
        else {
            panic!(
                "{:?} and {:?} are not both exponential squared",
                a.falloff, b.falloff
            );
        };
        assert_eq!(da, db);
    }

    #[test]
    fn every_terrain_camera_has_the_same_distance_fog() {
        let mut app = camera_app(Vec3::ZERO, StreamingSettings::default());
        let fogs = terrain_fogs(&mut app);
        assert_eq!(
            fogs.len(),
            level_count(&StreamingSettings::default()) as usize,
            "the near camera and one per coarse level"
        );
        let first = fogs[0].as_ref().expect("the near camera is fogged");
        for fog in &fogs {
            assert_same_fog(first, fog.as_ref().expect("a camera has no fog"));
        }
    }

    #[test]
    fn the_fog_is_derived_from_the_streaming_distances() {
        let streaming = StreamingSettings::default();
        let mut app = camera_app(Vec3::ZERO, streaming);
        let want = fog::distance_fog(
            &FogSettings::default(),
            streaming.view_distance_m,
            streaming.lod_distance_m,
        );
        for fog in terrain_fogs(&mut app) {
            assert_same_fog(&want, &fog.unwrap());
        }
    }

    #[test]
    fn the_coarse_projections_reach_the_lod_distance_and_start_at_the_players_near_plane() {
        let streaming = StreamingSettings::default();
        let mut app = camera_app(Vec3::ZERO, streaming);
        let cell = WorldSettings::default().cell_size();
        let near = projection_of::<With<MainCamera>>(&mut app).remove(0);
        let coarse = projection_of::<With<CoarseCamera>>(&mut app);
        assert!(!coarse.is_empty());
        for far in coarse {
            assert!(far.far >= streaming.lod_distance_m + cell);
            assert_eq!(far.near, near.near);
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
        app_with_palette(None)
    }

    fn app_with_palette(palette: Option<crate::content::MaterialPalette>) -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<ChunkMaterial>()
            .init_asset::<ShaderBuffer>()
            .init_resource::<WorldSettings>()
            .init_resource::<StreamingSettings>()
            .init_resource::<ChunkEntities>()
            .add_message::<ChunkMeshReady>()
            .add_message::<ChunkUnloaded>()
            .add_message::<LodMeshReady>()
            .add_message::<LodUnloaded>()
            .add_message::<crate::content::MaterialPalette>()
            .add_systems(Startup, add_material);
        add_view_systems(&mut app);
        // `upload_meshes` takes the dev-only counter as a `ResMut`, so without
        // it the system fails parameter validation instead of running.
        #[cfg(any(feature = "dev", feature = "profiling"))]
        app.init_resource::<crate::cost::MainThreadCost>();
        if let Some(palette) = palette {
            app.world_mut().write_message(palette);
        }
        app.update();
        app
    }

    #[test]
    fn terrain_and_item_models_share_one_texture_atlas_and_clear_removed_rows() {
        use crate::client::{
            edit::Holding,
            material_textures::{self, Textures},
        };
        use crate::content::MaterialTextures;
        let mut app = app();
        app.init_asset::<Image>()
            .add_plugins(material_textures::plugin);
        app.world_mut().write_message(MaterialTextures(vec![
            (23, vec![255; tesserae_mods::MATERIAL_TEXTURE_BYTES]),
            (63, vec![120; tesserae_mods::MATERIAL_TEXTURE_BYTES]),
        ]));
        app.update();
        let held = Holding::solid(23, 0);
        let first = app.world().resource::<Textures>().texture(held).unwrap();
        let terrain = app.world().resource::<TerrainMaterial>();
        let materials = app.world().resource::<Assets<ChunkMaterial>>();
        for handle in [&terrain.opaque, &terrain.blended, &terrain.cutout] {
            let extension = &materials.get(handle).unwrap().extension;
            assert_eq!(extension.texture.as_ref(), Some(&first));
            assert_eq!(extension.textured, UVec4::new(1 << 23, 1 << 31, 1, 0));
        }
        let item = app
            .world()
            .resource::<Textures>()
            .material(held, Vec4::ONE, false);
        assert_eq!(item.base_color_texture, Some(first.clone()));
        assert_eq!(app.world().resource::<Assets<Image>>().len(), 1);
        app.world_mut().write_message(MaterialTextures(Vec::new()));
        app.update();
        assert!(app.world().resource::<Textures>().texture(held).is_none());
        assert!(
            app.world()
                .resource::<Assets<Image>>()
                .get(first.id())
                .is_none()
        );
        let terrain = app.world().resource::<TerrainMaterial>();
        let materials = app.world().resource::<Assets<ChunkMaterial>>();
        for handle in [&terrain.opaque, &terrain.blended, &terrain.cutout] {
            assert_eq!(
                materials.get(handle).unwrap().extension.textured,
                UVec4::new(0, 0, 1, 0)
            );
        }
    }

    #[test]
    fn the_palette_message_reaches_all_three_terrain_material_uniforms() {
        let app = app_with_palette(Some(crate::content::MaterialPalette(vec![(
            8,
            [64, 128, 192, 128],
        )])));
        let terrain = app.world().resource::<TerrainMaterial>();
        let materials = app.world().resource::<Assets<ChunkMaterial>>();
        let expected = Color::srgba(64.0 / 255.0, 128.0 / 255.0, 192.0 / 255.0, 128.0 / 255.0)
            .to_linear()
            .to_vec4();
        for handle in [&terrain.opaque, &terrain.blended, &terrain.cutout] {
            let palette = materials.get(handle).unwrap().extension.palette;
            assert_eq!(palette[8], expected);
            assert_eq!(palette[9], Vec4::new(1.0, 0.0, 1.0, 1.0));
        }
    }

    /// Whether the materials discard a level-`level` upward face at `point`,
    /// read the way the shader reads it.
    fn cuts_at(app: &App, level: u32, point: Vec3) -> bool {
        cuts_facing(app, level, point, Vec3::Y)
    }

    /// `cuts_at` for a face of any orientation: the Chunk asked about is the one
    /// behind the face, so its normal decides which side of a boundary it is on.
    /// All three materials read one buffer, so all three answer alike.
    fn cuts_facing(app: &App, level: u32, point: Vec3, normal: Vec3) -> bool {
        let terrain = app.world().resource::<TerrainMaterial>();
        let materials = app.world().resource::<Assets<ChunkMaterial>>();
        for handle in [&terrain.opaque, &terrain.blended, &terrain.cutout] {
            assert_eq!(
                materials.get(handle).unwrap().extension.drawn,
                terrain.drawn,
                "every material reads the one record"
            );
        }
        let buffer = app
            .world()
            .resource::<Assets<ShaderBuffer>>()
            .get(&terrain.drawn)
            .unwrap();
        crate::client::drawn_mask::finer_drawn(
            buffer.cast_slice::<u32>().unwrap(),
            level,
            point,
            normal,
        )
    }

    /// Past every level's mesh extent, so nothing retained survives.
    const FAR_M: Vec3 = Vec3::new(1.0e7, 0.0, 1.0e7);

    #[test]
    fn coarse_water_is_cut_exactly_where_the_finer_level_has_delivered_its_chunk() {
        let mut app = app();
        let world = *app.world().resource::<WorldSettings>();
        let inside = |coord: ChunkCoord, level: u32| {
            world.chunk_origin_at(coord, level) + Vec3::splat(world.cell_size_at(level) / 2.0)
        };
        let (a, b, c) = (
            ChunkCoord::new(0, 0, 0),
            ChunkCoord::new(1, 0, 0),
            ChunkCoord::new(2, 0, 0),
        );
        send_mesh(&mut app, a, geometry());
        send_mesh(&mut app, c, geometry());
        // An empty mesh is still the finer level's answer for that space.
        send_mesh(
            &mut app,
            ChunkCoord::new(0, 1, 0),
            tesserae_core::ChunkMesh::default(),
        );
        assert!(cuts_at(&app, 1, inside(a, 0)));
        assert!(cuts_at(&app, 1, inside(c, 0)));
        assert!(cuts_at(&app, 1, inside(ChunkCoord::new(0, 1, 0), 0)));
        assert!(
            !cuts_at(&app, 1, inside(b, 0)),
            "inside the window, not sent"
        );
        assert!(
            !cuts_at(&app, 1, inside(ChunkCoord::new(9, 0, 0), 0)),
            "outside it"
        );

        put_eye(&mut app, FAR_M);
        app.world_mut().write_message(ChunkUnloaded { coord: a });
        app.update();
        assert!(!cuts_at(&app, 1, inside(a, 0)), "unloaded and released");

        send_coarse(&mut app, 1, a, geometry());
        assert!(cuts_at(&app, 2, inside(a, 1)), "level 2 asks level 1");
        assert!(!cuts_at(&app, 1, inside(a, 1) + Vec3::splat(40.0)));
    }

    #[test]
    fn coarse_water_is_discarded_where_any_finer_level_draws_and_only_there() {
        let mut app = app();
        let world = *app.world().resource::<WorldSettings>();
        let inside = |coord: ChunkCoord| {
            world.chunk_origin_at(coord, 0) + Vec3::splat(world.cell_size_at(0) / 2.0)
        };
        // Level 2's Chunk at the origin spans level-0 Chunks 0..4 on each axis;
        // level 1 draws none of it.
        send_coarse(&mut app, 2, ChunkCoord::new(0, 0, 0), blended_geometry());
        let sent = [ChunkCoord::new(0, 0, 0), ChunkCoord::new(3, 1, 2)];
        for coord in sent {
            send_mesh(&mut app, coord, blended_geometry());
        }
        for x in 0..4 {
            for y in 0..4 {
                for z in 0..4 {
                    let coord = ChunkCoord::new(x, y, z);
                    assert_eq!(
                        cuts_at(&app, 2, inside(coord)),
                        sent.contains(&coord),
                        "{coord:?}"
                    );
                }
            }
        }
    }

    /// Ticket 17: `level_the_water` puts a coarse water surface at level 0's own
    /// height, which a Mod's water level can place exactly on a Chunk boundary.
    #[test]
    fn a_face_on_a_chunk_boundary_asks_the_chunk_whose_mesh_holds_it() {
        let mut app = app();
        let world = *app.world().resource::<WorldSettings>();
        let sent = ChunkCoord::new(0, 0, 0);
        send_mesh(&mut app, sent, geometry());
        let cell_m = world.cell_size_at(0);
        let boundary = world.chunk_origin_at(ChunkCoord::new(0, 1, 0), 0)
            + Vec3::new(cell_m / 2.0, 0.0, cell_m / 2.0);
        assert!(
            cuts_facing(&app, 1, boundary, Vec3::Y),
            "an upward face there is the sent Chunk's own top"
        );
        assert!(
            !cuts_facing(&app, 1, boundary, Vec3::NEG_Y),
            "a downward face there belongs to the Chunk above, which was not sent"
        );
    }

    #[test]
    fn level_zero_is_never_discarded() {
        let mut app = app();
        send_mesh(&mut app, ChunkCoord::new(0, 0, 0), geometry());
        assert!(cuts_at(&app, 1, Vec3::splat(16.0)));
        assert!(!cuts_at(&app, 0, Vec3::splat(16.0)));
    }

    fn mask_has(app: &App, level: u32, coord: ChunkCoord) -> bool {
        let terrain = app.world().resource::<TerrainMaterial>();
        let buffer = app
            .world()
            .resource::<Assets<ShaderBuffer>>()
            .get(&terrain.drawn)
            .unwrap();
        crate::client::drawn_mask::cover_at(
            buffer.cast_slice::<u32>().unwrap(),
            level,
            IVec3::new(coord.x, coord.y, coord.z),
        ) & crate::client::drawn_mask::ALL
            != 0
    }

    fn unload_coarse(app: &mut App, level: u32, coord: ChunkCoord) {
        app.world_mut().write_message(LodUnloaded { level, coord });
        app.update();
    }

    #[test]
    fn a_coarse_chunk_the_server_unloads_is_drawn_until_all_eight_children_land() {
        let mut app = app();
        put_eye(&mut app, Vec3::ZERO);
        let coarse = ChunkCoord::new(0, 0, 0);
        send_coarse(&mut app, 1, coarse, geometry());
        unload_coarse(&mut app, 1, coarse);
        assert!(entity_at(&app, 1, coarse).is_some(), "retained");
        assert!(mask_has(&app, 1, coarse));

        let children: Vec<_> = super::children(coarse).collect();
        for &child in &children[..7] {
            send_mesh(&mut app, child, geometry());
        }
        assert!(entity_at(&app, 1, coarse).is_some(), "one child is missing");
        // An empty mesh is the finer level's answer too.
        send_mesh(&mut app, children[7], tesserae_core::ChunkMesh::default());
        assert_eq!(entity_at(&app, 1, coarse), None, "released");
        assert!(mask_has(&app, 1, coarse), "its children cover it");
    }

    #[test]
    fn a_coarse_chunk_is_covered_by_children_that_are_themselves_covered() {
        let mut app = app();
        put_eye(&mut app, Vec3::ZERO);
        let coarse = ChunkCoord::new(0, 0, 0);
        send_coarse(&mut app, 2, coarse, geometry());
        unload_coarse(&mut app, 2, coarse);
        let level_one: Vec<_> = super::children(coarse).collect();
        for &child in &level_one[1..] {
            send_coarse(&mut app, 1, child, geometry());
        }
        for grandchild in super::children(level_one[0]) {
            assert!(entity_at(&app, 2, coarse).is_some());
            send_mesh(&mut app, grandchild, geometry());
        }
        assert_eq!(entity_at(&app, 2, coarse), None);
    }

    #[test]
    fn a_finer_chunk_the_server_unloads_is_drawn_until_a_coarser_one_covers_it() {
        let mut app = app();
        put_eye(&mut app, Vec3::ZERO);
        let fine = ChunkCoord::new(5, 0, -3);
        send_mesh(&mut app, fine, geometry());
        app.world_mut().write_message(ChunkUnloaded { coord: fine });
        app.update();
        assert!(entity_for(&app, fine).is_some(), "retained");
        assert!(mask_has(&app, 0, fine));

        send_coarse(&mut app, 2, ChunkCoord::new(1, 0, -1), geometry());
        assert_eq!(entity_for(&app, fine), None, "its grandparent covers it");
        assert!(!mask_has(&app, 0, fine), "in the same frame");
        assert!(mask_has(&app, 2, ChunkCoord::new(1, 0, -1)));
    }

    #[test]
    fn a_retained_chunk_sent_again_is_delivered_and_kept() {
        let mut app = app();
        put_eye(&mut app, Vec3::ZERO);
        let coord = ChunkCoord::new(0, 0, 0);
        send_coarse(&mut app, 1, coord, geometry());
        unload_coarse(&mut app, 1, coord);
        send_coarse(&mut app, 1, coord, geometry());
        for child in super::children(coord) {
            send_mesh(&mut app, child, geometry());
        }
        assert!(
            entity_at(&app, 1, coord).is_some(),
            "delivered, not retained"
        );
    }

    #[test]
    fn a_teleport_releases_every_retained_mesh() {
        let mut app = app();
        put_eye(&mut app, Vec3::ZERO);
        send_mesh(&mut app, ChunkCoord::new(0, 0, 0), both_layers());
        send_coarse(&mut app, 1, ChunkCoord::new(4, 0, 0), both_layers());
        send_coarse(&mut app, 3, ChunkCoord::new(0, 0, 1), geometry());
        let world = app.world_mut();
        world.write_message(ChunkUnloaded {
            coord: ChunkCoord::new(0, 0, 0),
        });
        world.write_message(LodUnloaded {
            level: 1,
            coord: ChunkCoord::new(4, 0, 0),
        });
        world.write_message(LodUnloaded {
            level: 3,
            coord: ChunkCoord::new(0, 0, 1),
        });
        app.update();
        assert_eq!(drawn(&mut app), 5, "nothing covers them yet");
        put_eye(&mut app, FAR_M);
        assert_eq!(drawn(&mut app), 0);
        assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 0);
        assert!(app.world().resource::<ChunkEntities>().retained.is_empty());
    }

    /// Two panes of glass one behind the other along z, in one Chunk.
    fn two_panes() -> tesserae_core::ChunkMesh {
        let mut c = Chunk::air();
        c.set(
            5,
            5,
            5,
            Voxel::cube(tesserae_core::testgen::fixtures().glass),
        );
        c.set(
            5,
            5,
            10,
            Voxel::cube(tesserae_core::testgen::fixtures().glass),
        );
        let air = Chunk::air();
        let sky = ChunkLight::filled(Light::new(MAX_LIGHT, 0));
        mesh_chunk(
            LitChunk::new(&c, &sky),
            [Some(LitChunk::new(&air, &sky)); 6],
            &crate::server::fixture_tables(),
        )
    }

    /// Two pools side by side along x in a stone basin: two water quads in one
    /// plane, facing up, and nothing else blended.
    fn two_pools() -> tesserae_core::ChunkMesh {
        let (tables, water) = watered();
        let mut c = Chunk::air();
        for x in 0..=12 {
            for z in 0..=6 {
                c.set(
                    x,
                    4,
                    z,
                    Voxel::cube(tesserae_core::testgen::fixtures().ground.rock),
                );
                let rim = x == 0 || x == 6 || x == 12 || z == 0 || z == 6;
                c.set(
                    x,
                    5,
                    z,
                    Voxel::cube(if rim {
                        tesserae_core::testgen::fixtures().ground.rock
                    } else {
                        water
                    }),
                );
            }
        }
        let air = Chunk::air();
        let sky = ChunkLight::filled(Light::new(MAX_LIGHT, 0));
        let mesh = mesh_chunk(
            LitChunk::new(&c, &sky),
            [Some(LitChunk::new(&air, &sky)); 6],
            &tables,
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

    fn geometry() -> tesserae_core::ChunkMesh {
        let solid = Chunk::filled(Voxel::cube(tesserae_core::testgen::fixtures().ground.rock));
        let air = Chunk::air();
        let dark = ChunkLight::dark();
        let sky = ChunkLight::filled(Light::new(MAX_LIGHT, 0));
        let mut neighbors = [Some(LitChunk::new(&solid, &dark)); 6];
        neighbors[0] = Some(LitChunk::new(&air, &sky));
        let mesh = mesh_chunk(
            LitChunk::new(&solid, &dark),
            neighbors,
            &crate::server::fixture_tables(),
        );
        assert!(!mesh.is_empty(), "the fixture must have geometry");
        mesh
    }

    /// A Chunk of water beside open sky: all of its geometry is blended.
    fn blended_geometry() -> tesserae_core::ChunkMesh {
        let (tables, water) = watered();
        let water = Chunk::filled(Voxel::cube(water));
        let air = Chunk::air();
        let dark = ChunkLight::dark();
        let sky = ChunkLight::filled(Light::new(MAX_LIGHT, 0));
        let mut neighbors = [Some(LitChunk::new(&water, &dark)); 6];
        neighbors[0] = Some(LitChunk::new(&air, &sky));
        let mesh = mesh_chunk(LitChunk::new(&water, &dark), neighbors, &tables);
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
                "the level's camera draws it, or the mask and the sun miss it"
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
    fn moving_inside_one_cell_of_the_meshs_planes_never_reindexes() {
        let mut app = app();
        let coord = ChunkCoord::new(0, 0, 0);
        put_eye(&mut app, Vec3::new(5.5, 5.5, 8.4));
        send_mesh(&mut app, coord, two_panes());
        let entity = blended_at(&app, 0, coord).unwrap();
        let handle = app.world().get::<Mesh3d>(entity).unwrap().0.clone();
        app.world_mut()
            .resource_mut::<Messages<AssetEvent<Mesh>>>()
            .clear();
        // Between the panes and inside their x and y span, where a nearest-centre
        // order would change and the separating planes' cannot.
        for z in [8.0, 7.2, 9.6] {
            put_eye(&mut app, Vec3::new(5.4, 5.6, z));
        }
        let modified = app
            .world_mut()
            .resource_mut::<Messages<AssetEvent<Mesh>>>()
            .drain()
            .any(|e| e.is_modified(&handle));
        assert!(!modified);
    }

    /// One drawn quad of a blended mesh, axis-aligned, in the entity's metres.
    struct DrawnQuad {
        axis: usize,
        plane: f32,
        min: Vec3,
        max: Vec3,
        normal: Vec3,
    }

    /// The blended mesh's quads in the order the GPU draws them.
    fn drawn_quads(app: &App, entity: Entity) -> Vec<DrawnQuad> {
        let handle = &app.world().get::<Mesh3d>(entity).unwrap().0;
        let mesh = app.world().resource::<Assets<Mesh>>().get(handle).unwrap();
        let positions = positions(app, entity);
        let Some(VertexAttributeValues::Float32x3(normals)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("normals are Float32x3");
        };
        let Some(bevy::mesh::Indices::U32(indices)) = mesh.indices() else {
            panic!("U32 indices");
        };
        indices
            .chunks(6)
            .map(|quad| {
                let corners: Vec<Vec3> = quad
                    .iter()
                    .map(|&i| Vec3::from(positions[i as usize]))
                    .collect();
                let normal = Vec3::from(normals[quad[0] as usize]);
                let axis = (0..3).find(|&a| normal[a] != 0.0).unwrap();
                let min = corners.iter().copied().fold(Vec3::MAX, Vec3::min);
                let max = corners.iter().copied().fold(Vec3::MIN, Vec3::max);
                DrawnQuad {
                    axis,
                    plane: min[axis],
                    min,
                    max,
                    normal,
                }
            })
            .collect()
    }

    /// Rays from `eye` towards `target` that meet two blended quads drawn in the
    /// wrong order: the nearer one first, so the farther one blends over it.
    /// Back faces are culled, as `StandardMaterial` does.
    fn rays_drawn_out_of_order(quads: &[DrawnQuad], eye: Vec3, target: Vec3) -> usize {
        let forward = (target - eye).normalize();
        let right = forward.cross(Vec3::Y).normalize();
        let up = right.cross(forward);
        let n = 96;
        let half = 30f32.to_radians().tan();
        let mut wrong = 0;
        for py in 0..n {
            for px in 0..n {
                let sx = (px as f32 + 0.5) / n as f32 * 2.0 - 1.0;
                let sy = (py as f32 + 0.5) / n as f32 * 2.0 - 1.0;
                let d = (forward + sx * half * right + sy * half * up).normalize();
                let hits: Vec<f32> = quads
                    .iter()
                    .map(|q| {
                        if q.normal.dot(d) >= 0.0 || d[q.axis] == 0.0 {
                            return f32::NAN;
                        }
                        let t = (q.plane - eye[q.axis]) / d[q.axis];
                        let p = eye + t * d;
                        let inside = (0..3)
                            .filter(|&a| a != q.axis)
                            .all(|a| p[a] > q.min[a] && p[a] < q.max[a]);
                        if t > 0.0 && inside { t } else { f32::NAN }
                    })
                    .collect();
                let drawn: Vec<f32> = hits.into_iter().filter(|t| !t.is_nan()).collect();
                // In draw order, every later hit must be nearer.
                let farther_later =
                    (0..drawn.len()).any(|i| drawn[i + 1..].iter().any(|&later| later > drawn[i]));
                wrong += usize::from(farther_later);
            }
        }
        wrong
    }

    /// Eyes all around `target`, 8 headings at 10, 30 and 60 degrees up.
    fn around(target: Vec3, distance: f32) -> Vec<Vec3> {
        let mut eyes = vec![];
        for elevation in [10f32, 30.0, 60.0] {
            for step in 0..8 {
                let (e, a) = (
                    elevation.to_radians(),
                    step as f32 * std::f32::consts::FRAC_PI_4,
                );
                eyes.push(
                    target + distance * Vec3::new(e.cos() * a.sin(), e.sin(), e.cos() * a.cos()),
                );
            }
        }
        eyes
    }

    /// The dev gallery's glass house in front of its pool, a lantern inside.
    fn glass_house_and_pool() -> tesserae_core::ChunkMesh {
        let (tables, water) = watered();
        let mut c = Chunk::air();
        for x in 0..11 {
            for z in 0..12 {
                c.set(
                    x,
                    9,
                    z,
                    Voxel::cube(tesserae_core::testgen::fixtures().ground.rock),
                );
            }
        }
        for k in 0..=5 {
            for i in 0..=8 {
                let rim = i == 0 || i == 8 || k == 0 || k == 5;
                c.set(
                    i,
                    10,
                    k,
                    Voxel::cube(if rim {
                        tesserae_core::testgen::fixtures().ground.rock
                    } else {
                        water
                    }),
                );
            }
        }
        for k in 7..=10 {
            for i in 1..=5 {
                let wall = i == 1 || i == 5 || k == 7 || k == 10;
                for j in 0..=2 {
                    if wall {
                        c.set(
                            i,
                            10 + j,
                            k,
                            Voxel::cube(tesserae_core::testgen::fixtures().glass),
                        );
                    }
                }
                c.set(
                    i,
                    13,
                    k,
                    Voxel::cube(tesserae_core::testgen::fixtures().glass),
                );
            }
        }
        c.set(
            3,
            10,
            8,
            Voxel::cube(tesserae_core::testgen::fixtures().lantern),
        );
        let air = Chunk::air();
        let sky = ChunkLight::filled(Light::new(MAX_LIGHT, 0));
        mesh_chunk(
            LitChunk::new(&c, &sky),
            [Some(LitChunk::new(&air, &sky)); 6],
            &tables,
        )
    }

    /// A long strip of water over a small glass cube near one end: seen from
    /// above that end, the strip is in front of the cube, but its centre is far
    /// down the strip and farther from the Eye than the cube's.
    fn strip_over_a_cube() -> tesserae_core::ChunkMesh {
        let (tables, water) = watered();
        let mut c = Chunk::air();
        for x in 0..30 {
            c.set(x, 5, 10, Voxel::cube(water));
        }
        c.set(
            2,
            3,
            10,
            Voxel::cube(tesserae_core::testgen::fixtures().glass),
        );
        let air = Chunk::air();
        let sky = ChunkLight::filled(Light::new(MAX_LIGHT, 0));
        mesh_chunk(
            LitChunk::new(&c, &sky),
            [Some(LitChunk::new(&air, &sky)); 6],
            &tables,
        )
    }

    #[test]
    fn blended_layers_are_drawn_back_to_front_along_every_ray_from_all_around() {
        let house = Vec3::new(5.5, 11.5, 6.0);
        let panes = Vec3::new(5.5, 5.5, 8.0);
        let ends = [Vec3::new(5.5, 5.5, 60.0), Vec3::new(5.5, 5.5, -60.0)];
        let strip = Vec3::new(15.0, 5.0, 10.5);
        let above_the_cube = Vec3::new(0.0, 20.0, 10.5);
        for (name, mesh, target, eyes) in [
            (
                "glass house and pool",
                glass_house_and_pool(),
                house,
                around(house, 30.0),
            ),
            (
                "two panes",
                two_panes(),
                panes,
                [around(panes, 20.0), ends.to_vec()].concat(),
            ),
            (
                "strip over a cube",
                strip_over_a_cube(),
                Vec3::new(2.5, 4.0, 10.5),
                [around(strip, 25.0), vec![above_the_cube]].concat(),
            ),
        ] {
            let mut app = app();
            let coord = ChunkCoord::new(0, 0, 0);
            put_eye(&mut app, eyes[0]);
            send_mesh(&mut app, coord, mesh);
            let entity = blended_at(&app, 0, coord).unwrap();
            for eye in eyes {
                put_eye(&mut app, eye);
                let wrong = rays_drawn_out_of_order(&drawn_quads(&app, entity), eye, target);
                assert_eq!(wrong, 0, "{name}, eye at {eye}");
            }
        }
    }

    /// Glass and water in a 3D checkerboard, six Voxels a side, so every face is
    /// its own quad and the mesh has about 1300, over the old exact-order cap;
    /// and in the same Chunk the strip of water over a glass cube that a
    /// nearest-centre order draws wrong.
    fn checkerboard_and_a_strip() -> tesserae_core::ChunkMesh {
        let (tables, water) = watered();
        let mut c = Chunk::air();
        for x in 0..6 {
            for y in 0..6 {
                for z in 0..6 {
                    let m = if (x + y + z) % 2 == 0 {
                        tesserae_core::testgen::fixtures().glass
                    } else {
                        water
                    };
                    c.set(10 + x, 10 + y, 10 + z, Voxel::cube(m));
                }
            }
        }
        for x in 0..30 {
            c.set(x, 5, 25, Voxel::cube(water));
        }
        c.set(
            2,
            3,
            25,
            Voxel::cube(tesserae_core::testgen::fixtures().glass),
        );
        let air = Chunk::air();
        let sky = ChunkLight::filled(Light::new(MAX_LIGHT, 0));
        let mesh = mesh_chunk(
            LitChunk::new(&c, &sky),
            [Some(LitChunk::new(&air, &sky)); 6],
            &tables,
        );
        assert!(
            mesh.blended.quad_count() > 1000,
            "{}",
            mesh.blended.quad_count()
        );
        mesh
    }

    #[test]
    fn a_large_blended_mesh_is_drawn_back_to_front_too() {
        let checkerboard = Vec3::new(13.0, 13.0, 13.0);
        let cube = Vec3::new(2.5, 4.0, 25.5);
        let mut app = app();
        let coord = ChunkCoord::new(0, 0, 0);
        let mut views: Vec<(Vec3, Vec3)> = around(checkerboard, 25.0)
            .into_iter()
            .step_by(5)
            .map(|eye| (eye, checkerboard))
            .collect();
        views.push((Vec3::new(0.0, 20.0, 25.5), cube));
        put_eye(&mut app, views[0].0);
        send_mesh(&mut app, coord, checkerboard_and_a_strip());
        let entity = blended_at(&app, 0, coord).unwrap();
        for (eye, target) in views {
            put_eye(&mut app, eye);
            let wrong = rays_drawn_out_of_order(&drawn_quads(&app, entity), eye, target);
            assert_eq!(wrong, 0, "eye at {eye}");
        }
    }

    /// What the strip fixture is for: the nearest-centre order gets it wrong.
    #[test]
    fn a_nearest_centre_order_gets_the_strip_over_a_cube_wrong() {
        let mut app = app();
        let coord = ChunkCoord::new(0, 0, 0);
        let eye = Vec3::new(0.0, 20.0, 10.5);
        put_eye(&mut app, eye);
        send_mesh(&mut app, coord, strip_over_a_cube());
        let entity = blended_at(&app, 0, coord).unwrap();
        let mut quads = drawn_quads(&app, entity);
        let centre = |q: &DrawnQuad| (q.min + q.max) / 2.0;
        quads.sort_by(|a, b| eye.distance(centre(b)).total_cmp(&eye.distance(centre(a))));
        assert!(rays_drawn_out_of_order(&quads, eye, Vec3::new(2.5, 4.0, 10.5)) > 0);
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

    /// Two leaf cubes beside open sky: every face is in the cutout layer.
    fn leaves() -> tesserae_core::ChunkMesh {
        let mut c = Chunk::air();
        c.set(
            5,
            5,
            5,
            Voxel::cube(tesserae_core::testgen::fixtures().leaves),
        );
        c.set(
            6,
            5,
            5,
            Voxel::cube(tesserae_core::testgen::fixtures().leaves),
        );
        let air = Chunk::air();
        let sky = ChunkLight::filled(Light::new(MAX_LIGHT, 0));
        let mesh = mesh_chunk(
            LitChunk::new(&c, &sky),
            [Some(LitChunk::new(&air, &sky)); 6],
            &crate::server::fixture_tables(),
        );
        assert!(mesh.opaque.is_empty() && !mesh.cutout.is_empty());
        mesh
    }

    fn cutout_at(app: &App, level: u32, coord: ChunkCoord) -> Option<Entity> {
        app.world()
            .resource::<ChunkEntities>()
            .cutout
            .get(&(level, coord))
            .copied()
    }

    #[test]
    fn leaves_are_drawn_masked_casting_their_shadow_and_follow_their_chunk() {
        let mut app = app();
        let coord = ChunkCoord::new(0, 0, 0);
        send_mesh(&mut app, coord, leaves());
        let entity = cutout_at(&app, 0, coord).expect("a cutout entity");
        assert_eq!(drawn(&mut app), 1);
        let handle = &app
            .world()
            .get::<MeshMaterial3d<ChunkMaterial>>(entity)
            .unwrap()
            .0;
        let material = app
            .world()
            .resource::<Assets<ChunkMaterial>>()
            .get(handle)
            .unwrap();
        assert!(matches!(material.base.alpha_mode, AlphaMode::Mask(_)));
        assert!(
            app.world()
                .get::<bevy::light::NotShadowCaster>(entity)
                .is_none(),
            "a canopy's shadow has its holes, not none"
        );
        assert_eq!(app.world().get::<MeshTag>(entity).unwrap().value, 0);

        send_mesh(&mut app, coord, geometry());
        assert_eq!(
            cutout_at(&app, 0, coord),
            None,
            "a replacement without leaves"
        );
        send_mesh(&mut app, coord, leaves());
        put_eye(&mut app, FAR_M);
        app.world_mut().write_message(ChunkUnloaded { coord });
        app.update();
        assert_eq!(drawn(&mut app), 0);
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
        put_eye(&mut app, FAR_M);
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
        put_eye(&mut app, FAR_M);
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
        put_eye(&mut app, FAR_M);
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
        put_eye(&mut app, FAR_M);
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
        put_eye(&mut app, FAR_M);
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
        put_eye(&mut app, FAR_M);
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

    #[test]
    fn water_is_drawn_from_below_and_solid_terrain_is_not() {
        let app = app();
        let terrain = app.world().resource::<TerrainMaterial>();
        let materials = app.world().resource::<Assets<ChunkMaterial>>();
        let base = |handle| &materials.get(handle).unwrap().base;
        assert!(base(&terrain.blended).double_sided);
        assert_eq!(base(&terrain.blended).cull_mode, None);
        for handle in [&terrain.opaque, &terrain.cutout] {
            assert!(!base(handle).double_sided);
            assert!(base(handle).cull_mode.is_some());
        }
    }
}
