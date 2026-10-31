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

use bevy::light::NotShadowCaster;
use bevy::mesh::MeshTag;
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
use tesserae_core::{CHUNK_SIZE, MeshLayer};

use crate::client::chunk_mesh::{PackedLayer, merged_mesh, pack};
use crate::client::chunk_view::layer_of;
use crate::render::chunk_material::ChunkMaterial;
use crate::world::{ChunkCoord, WorldSettings};

/// Region edges are powers of two, so a region is a coordinate shift. 4 Chunks
/// an edge at level 0 is 128 Voxels, well inside the packed position's ±2047.
const DEFAULT_SHIFT: u32 = 2;

/// 32 Chunks an edge is 1024 Voxels; the next power of two is past the range.
const MAX_SHIFT: u32 = 5;

/// `TESSERAE_REGION_CHUNKS=1` under `dev` or `profiling` draws a Chunk per
/// entity again, so one binary can compare region sizes.
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
    members: BTreeSet<ChunkCoord>,
}

/// One mesh layer's regions on every level.
pub(super) struct Regions {
    shift: u32,
    parts: HashMap<Key, Arc<PackedLayer>>,
    regions: HashMap<Key, Region>,
    /// Regions whose members changed since `assemble` last ran.
    dirty: HashSet<Key>,
}

impl Default for Regions {
    fn default() -> Self {
        Regions {
            shift: configured_shift(),
            parts: default(),
            regions: default(),
            dirty: default(),
        }
    }
}

/// What spawning a region's entity needs besides the mesh.
pub(super) struct Look<'a> {
    pub material: &'a Handle<ChunkMaterial>,
    pub casts_shadows: bool,
}

impl Regions {
    fn region_of(&self, (level, coord): Key) -> Key {
        let s = self.shift;
        (
            level,
            ChunkCoord::new(coord.x >> s, coord.y >> s, coord.z >> s),
        )
    }

    /// The Chunk's layer is now `layer`; an empty one removes it.
    pub(super) fn set(&mut self, key: Key, layer: &MeshLayer) {
        if layer.is_empty() {
            self.remove(key);
            return;
        }
        self.parts.insert(key, Arc::new(pack(layer)));
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

    /// The entity drawing the Chunk's layer, once its region is assembled.
    pub(super) fn entity(&self, key: Key) -> Option<Entity> {
        if !self.parts.contains_key(&key) {
            return None;
        }
        let (entity, _) = self.regions.get(&self.region_of(key))?.drawn.as_ref()?;
        Some(*entity)
    }

    /// The Chunk's own layer, packed, in Voxels from its origin.
    pub(super) fn part(&self, key: Key) -> Option<&Arc<PackedLayer>> {
        self.parts.get(&key)
    }

    #[cfg(feature = "profiling")]
    pub(super) fn entity_count(&self) -> usize {
        self.regions.values().filter(|r| r.drawn.is_some()).count()
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
            if region.members.is_empty() {
                if let Some((entity, mesh)) = region.drawn.take() {
                    meshes.remove(&mesh);
                    commands.entity(entity).despawn();
                }
                self.regions.remove(&key);
                continue;
            }
            let (level, at) = key;
            let base = IVec3::new(at.x, at.y, at.z) * edge;
            let parts = region.members.iter().map(|&coord| {
                let offset = (IVec3::new(coord.x, coord.y, coord.z) - base) * CHUNK_SIZE as i32;
                (offset, &*self.parts[&(level, coord)])
            });
            let mesh = merged_mesh(parts);
            match &region.drawn {
                Some((_, handle)) => {
                    meshes
                        .insert(handle.id(), mesh)
                        .expect("a region's own handle is live");
                }
                None => {
                    let handle = meshes.add(mesh);
                    let origin = ChunkCoord::new(base.x, base.y, base.z);
                    let mut entity = commands.spawn((
                        Mesh3d(handle.clone()),
                        MeshMaterial3d(look.material.clone()),
                        Transform::from_translation(world.chunk_origin_at(origin, level))
                            .with_scale(Vec3::splat(world.voxel_size_at(level))),
                        MeshTag::new(level),
                        layer_of(level),
                    ));
                    if !look.casts_shadows {
                        entity.insert(NotShadowCaster);
                    }
                    region.drawn = Some((entity.id(), handle));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::chunk_mesh::decoded;

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
