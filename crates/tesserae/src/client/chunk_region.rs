//! Opaque and cutout Chunk meshes merged by region, so render entities and the
//! render world's per-entity work scale with regions rather than Chunks.
//!
//! A region is a cube of `edge` Chunks on one level's lattice, drawn by one
//! entity per mesh layer with one shared material. Each Chunk keeps its packed
//! layer here, so a re-mesh, an unload or a release re-assembles only its own
//! region, once a frame however many of its Chunks changed. Blended layers stay
//! one entity per Chunk: `blend_order` sorts within one Chunk's mesh.
//!
//! Visibility: Bevy's frustum (and any occlusion) culling sees a region whole.
//! The draw rule does not: `chunk_material.wesl` discards by world position
//! against the per-Chunk mask, so coverage and retention stay per Chunk inside
//! a region. Per-Chunk culling stays possible the same way a re-mesh is: a
//! culler leaves a Chunk out of its region's next assembly, at one region's
//! rebuild.

use std::collections::BTreeSet;
use std::sync::Arc;

use bevy::camera::visibility::RenderLayers;
use bevy::light::NotShadowCaster;
use bevy::mesh::MeshTag;
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
use tesserae_core::CHUNK_SIZE;

use crate::client::chunk_mesh::{PackedLayer, merged_mesh};
use crate::client::chunk_view::layer_of;
use crate::render::chunk_material::ChunkMaterial;
use crate::world::{ChunkCoord, WorldSettings};

/// Region edges are powers of two, so a region is a coordinate shift. 2 Chunks
/// an edge, not 4: Bevy culls a region whole, and in single captures 4 drew
/// enough extra geometry to cost 5-8% of a GPU-bound frame against 1-2% for 2.
const DEFAULT_SHIFT: u32 = 1;

/// 32 Chunks an edge is 1024 Voxels; the next power of two is past the range.
#[cfg(any(feature = "dev", feature = "profiling", test))]
const MAX_SHIFT: u32 = 5;

/// `TESSERAE_REGION_CHUNKS=1` under `dev` or `profiling` draws a Chunk per
/// entity, so one binary can compare region sizes.
#[cfg(any(feature = "dev", feature = "profiling"))]
fn configured_shift() -> u32 {
    std::env::var("TESSERAE_REGION_CHUNKS")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .filter(|edge| edge.is_power_of_two())
        .map_or(DEFAULT_SHIFT, |edge| edge.trailing_zeros().min(MAX_SHIFT))
}

#[cfg(not(any(feature = "dev", feature = "profiling")))]
fn configured_shift() -> u32 {
    DEFAULT_SHIFT
}

type Key = (u32, ChunkCoord);

#[derive(Default)]
struct Region {
    drawn: Option<(Entity, Handle<Mesh>)>,
    /// The hidden members, drawn only into the sun's shadow maps: culled
    /// from the camera is not culled from the light.
    shadow: Option<(Entity, Handle<Mesh>)>,
    members: BTreeSet<ChunkCoord>,
}

/// One mesh layer's regions on every level.
pub(super) struct Regions {
    shift: u32,
    parts: HashMap<Key, Arc<PackedLayer>>,
    regions: HashMap<Key, Region>,
    /// Regions whose members changed since `assemble` last ran.
    dirty: HashSet<Key>,
    /// Chunks a visibility walk does not reach, left out of their region.
    hidden: HashSet<Key>,
}

impl Default for Regions {
    fn default() -> Self {
        Regions {
            shift: configured_shift(),
            parts: default(),
            regions: default(),
            dirty: default(),
            hidden: default(),
        }
    }
}

/// What spawning a region's entity needs besides the mesh.
pub(super) struct Look<'a> {
    pub material: &'a Handle<ChunkMaterial>,
    pub casts_shadows: bool,
    /// Whether hidden members still cast; false only for a review control.
    pub hidden_cast: bool,
}

impl Regions {
    fn region_of(&self, (level, coord): Key) -> Key {
        let s = self.shift;
        (
            level,
            ChunkCoord::new(coord.x >> s, coord.y >> s, coord.z >> s),
        )
    }

    /// The Chunk's layer is now `layer`; none removes it.
    pub(super) fn set(&mut self, key: Key, layer: Option<PackedLayer>) {
        let Some(layer) = layer else {
            self.remove(key);
            return;
        };
        self.parts.insert(key, Arc::new(layer));
        let region = self.region_of(key);
        self.regions
            .entry(region)
            .or_default()
            .members
            .insert(key.1);
        self.dirty.insert(region);
    }

    pub(super) fn remove(&mut self, key: Key) {
        if self.parts.remove(&key).is_none() {
            return;
        }
        let region = self.region_of(key);
        if let Some(r) = self.regions.get_mut(&region) {
            r.members.remove(&key.1);
        }
        self.dirty.insert(region);
    }

    /// Leaves the Chunk out of its region, or puts it back, from the region's
    /// next assembly on.
    pub(super) fn hide(&mut self, key: Key, hidden: bool) {
        let changed = if hidden {
            self.hidden.insert(key)
        } else {
            self.hidden.remove(&key)
        };
        if changed && self.parts.contains_key(&key) {
            self.dirty.insert(self.region_of(key));
        }
    }

    /// The entity drawing the Chunk's layer, once its region is assembled.
    #[cfg(test)]
    pub(super) fn entity(&self, key: Key) -> Option<Entity> {
        if !self.parts.contains_key(&key) {
            return None;
        }
        let (entity, _) = self.regions.get(&self.region_of(key))?.drawn.as_ref()?;
        Some(*entity)
    }

    /// The region drawing the Chunk, by the region lattice, and its edge in
    /// Chunks.
    /// The Chunks of `level` this layer holds a part for.
    pub(super) fn parts_at_level(&self, level: u32) -> impl Iterator<Item = ChunkCoord> + '_ {
        self.parts
            .keys()
            .filter(move |&&(l, _)| l == level)
            .map(|&(_, c)| c)
    }

    #[cfg(feature = "profiling")]
    pub(super) fn region_of_chunk(&self, key: Key) -> (ChunkCoord, i32) {
        (self.region_of(key).1, 1 << self.shift)
    }

    /// The entity drawing the Chunk's region into shadow maps only.
    #[cfg(test)]
    pub(super) fn shadow_entity(&self, key: Key) -> Option<Entity> {
        if !self.parts.contains_key(&key) {
            return None;
        }
        let (entity, _) = self.regions.get(&self.region_of(key))?.shadow.as_ref()?;
        Some(*entity)
    }

    /// The Chunk's own layer, packed, in Voxels from its origin.
    pub(super) fn part(&self, key: Key) -> Option<&Arc<PackedLayer>> {
        self.parts.get(&key)
    }

    #[cfg(feature = "profiling")]
    pub(super) fn entity_count(&self) -> usize {
        self.regions
            .values()
            .map(|r| usize::from(r.drawn.is_some()) + usize::from(r.shadow.is_some()))
            .sum()
    }

    /// Regions `assemble` would rebuild now.
    #[cfg(any(feature = "dev", feature = "profiling"))]
    pub(super) fn due(&self) -> usize {
        self.dirty.len()
    }

    pub(super) fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }

    #[cfg(test)]
    pub(super) fn insert_part(&mut self, key: Key, part: PackedLayer) {
        self.parts.insert(key, Arc::new(part));
    }

    /// Re-assembles every region whose members changed: its mesh replaced in
    /// place, so its entity and handle stay; spawned if new; despawned with its
    /// asset if it has no members left.
    pub(super) fn assemble(
        &mut self,
        commands: &mut Commands,
        meshes: &mut Assets<Mesh>,
        world: &WorldSettings,
        look: Look,
    ) {
        let edge = 1i32 << self.shift;
        for key in self.dirty.drain() {
            let Some(region) = self.regions.get_mut(&key) else {
                continue;
            };
            let (level, at) = key;
            let base = IVec3::new(at.x, at.y, at.z) * edge;
            let hidden = &self.hidden;
            let parts = &self.parts;
            let (shown, culled): (Vec<ChunkCoord>, Vec<ChunkCoord>) = region
                .members
                .iter()
                .partition(|&&coord| !hidden.contains(&(level, coord)));
            let mesh_of = |members: &[ChunkCoord]| {
                (!members.is_empty()).then(|| {
                    merged_mesh(members.iter().map(|&coord| {
                        let offset =
                            (IVec3::new(coord.x, coord.y, coord.z) - base) * CHUNK_SIZE as i32;
                        (offset, &*parts[&(level, coord)])
                    }))
                })
            };
            let spawn = Spawn {
                origin: world.chunk_origin_at(ChunkCoord::new(base.x, base.y, base.z), level),
                scale: world.voxel_size_at(level),
                level,
                material: look.material,
            };
            let main = (layer_of(level), !look.casts_shadows);
            spawn.place(&mut region.drawn, mesh_of(&shown), main, commands, meshes);
            let shadow = mesh_of(&culled).filter(|_| look.casts_shadows && look.hidden_cast);
            let shadow_only = (RenderLayers::layer(SHADOW_ONLY_LAYER), false);
            spawn.place(&mut region.shadow, shadow, shadow_only, commands, meshes);
            if region.members.is_empty() {
                self.regions.remove(&key);
            }
        }
    }
}

/// The render layer only the sun draws: its shadow views take it, and no camera
/// does. Past every level's own layer.
pub(super) const SHADOW_ONLY_LAYER: usize = crate::server::MAX_LEVELS as usize;

/// What a region's entities share besides their mesh.
struct Spawn<'a> {
    origin: Vec3,
    scale: f32,
    level: u32,
    material: &'a Handle<ChunkMaterial>,
}

impl Spawn<'_> {
    /// Brings `slot` in line with `mesh`: replaced in place, spawned on
    /// `layers`, or despawned with its asset when there is no mesh.
    fn place(
        &self,
        slot: &mut Option<(Entity, Handle<Mesh>)>,
        mesh: Option<Mesh>,
        (layers, no_shadow): (RenderLayers, bool),
        commands: &mut Commands,
        meshes: &mut Assets<Mesh>,
    ) {
        match (mesh, &*slot) {
            (Some(mesh), Some((_, handle))) => {
                meshes
                    .insert(handle.id(), mesh)
                    .expect("a region's own handle is live");
            }
            (Some(mesh), None) => {
                let handle = meshes.add(mesh);
                let mut entity = commands.spawn((
                    Mesh3d(handle.clone()),
                    MeshMaterial3d(self.material.clone()),
                    Transform::from_translation(self.origin).with_scale(Vec3::splat(self.scale)),
                    MeshTag::new(self.level),
                    layers,
                ));
                if no_shadow {
                    entity.insert(NotShadowCaster);
                }
                *slot = Some((entity.id(), handle));
            }
            (None, _) => {
                if let Some((entity, handle)) = slot.take() {
                    meshes.remove(&handle);
                    commands.entity(entity).despawn();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::chunk_mesh::{decoded, pack};
    use tesserae_core::MeshLayer;

    #[test]
    fn the_far_corner_of_the_widest_region_packs_exactly() {
        let beyond = CHUNK_SIZE as f32 + 1.0 / 16.0;
        let layer = MeshLayer {
            positions: vec![[beyond; 3], [-1.0 / 16.0; 3], [0.0; 3]],
            normals: vec![[0.0, 1.0, 0.0]; 3],
            uvs: vec![[0.0; 2]; 3],
            materials: vec![1; 3],
            light: vec![0; 3],
            indices: vec![0, 1, 2],
        };
        let corner = IVec3::splat(((1 << MAX_SHIFT) - 1) * CHUNK_SIZE as i32);
        let mesh = merged_mesh([(corner, &pack(&layer))]);
        let moved: Vec<[f32; 3]> = layer
            .positions
            .iter()
            .map(|p| (Vec3::from(*p) + corner.as_vec3()).to_array())
            .collect();
        assert_eq!(decoded(&mesh).unwrap().positions, moved);
    }
}
