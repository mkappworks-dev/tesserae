//! The quad buffer: one storage buffer holding every Chunk layer's words, in a
//! range each, which the terrain materials bind and `chunk_vertex.wesl` reads.
//!
//! The main world hands out ranges and queues writes; the render world applies
//! them once the buffer is prepared, so a write lands in the frame whose
//! extraction saw it. That is also why a range is reusable as soon as it is
//! freed: the frame still drawing from it was extracted before the free.

use std::collections::BTreeMap;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::BufferUsages;
use bevy::render::renderer::RenderQueue;
use bevy::render::storage::{GpuShaderBuffer, ShaderBuffer};
use bevy::render::{ExtractSchedule, MainWorld, Render, RenderApp, RenderSystems};

/// 4 MiB to start, doubling: ticket 01 of `.scratch/2026-11-17-render-packed-quads/` decides.
const INITIAL_WORDS: u32 = 1 << 20;

#[derive(Resource)]
pub struct QuadBuffer {
    pub handle: Handle<ShaderBuffer>,
    capacity: u32,
    /// Free ranges by start, never two adjacent.
    free: BTreeMap<u32, u32>,
    /// Queued since the render world last took them, as the bytes it writes.
    writes: Vec<(u32, Vec<u8>)>,
    resized: bool,
    /// What the GPU would hold, for tests that read a range back.
    #[cfg(test)]
    pub mirror: Vec<u32>,
}

impl QuadBuffer {
    pub fn new(buffers: &mut Assets<ShaderBuffer>) -> Self {
        QuadBuffer {
            handle: buffers.add(buffer(INITIAL_WORDS)),
            capacity: INITIAL_WORDS,
            free: BTreeMap::from([(0, INITIAL_WORDS)]),
            writes: Vec::new(),
            resized: false,
            #[cfg(test)]
            mirror: Vec::new(),
        }
    }

    /// The first free range of `len` words, the buffer doubling until one fits.
    pub fn alloc(&mut self, len: u32) -> u32 {
        loop {
            if let Some((&at, &free)) = self.free.iter().find(|&(_, &free)| free >= len) {
                self.free.remove(&at);
                if free > len {
                    self.free.insert(at + len, free - len);
                }
                return at;
            }
            let grown = self
                .capacity
                .checked_mul(2)
                .expect("a quad buffer under 16 GiB");
            self.release(self.capacity, grown - self.capacity);
            self.capacity = grown;
            self.resized = true;
        }
    }

    pub fn release(&mut self, mut at: u32, mut len: u32) {
        if let Some((&before, &free)) = self.free.range(..at).next_back()
            && before + free == at
        {
            self.free.remove(&before);
            at = before;
            len += free;
        }
        if let Some(after) = self.free.remove(&(at + len)) {
            len += after;
        }
        self.free.insert(at, len);
    }

    pub fn write(&mut self, at: u32, words: &[u32]) {
        #[cfg(test)]
        {
            let end = (at as usize + words.len()).max(self.mirror.len());
            self.mirror.resize(end, 0);
            self.mirror[at as usize..][..words.len()].copy_from_slice(words);
        }
        let bytes = words.iter().flat_map(|w| w.to_ne_bytes()).collect();
        self.writes.push((at, bytes));
    }

    /// Words in ranges handed out and not given back.
    #[cfg(test)]
    pub fn used(&self) -> u32 {
        self.capacity - self.free.values().sum::<u32>()
    }

    /// Writes queued and not yet taken: all of them, without a render world.
    #[cfg(test)]
    pub fn queued(&self) -> usize {
        self.writes.len()
    }
}

fn buffer(words: u32) -> ShaderBuffer {
    ShaderBuffer {
        label: "chunk quads".into(),
        buffer_usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
        // Grown on the GPU: a resize copies the old contents and replaces the
        // bind groups that held it, without touching the materials.
        copy_on_resize: true,
        ..ShaderBuffer::with_size(u64::from(words) * 4, RenderAssetUsages::RENDER_WORLD)
    }
}

pub fn plugin(app: &mut App) {
    app.add_systems(Last, grow);
    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        render_app
            .init_resource::<PendingWrites>()
            .add_systems(ExtractSchedule, take_writes)
            .add_systems(Render, apply_writes.in_set(RenderSystems::PrepareResources));
    }
}

/// Resizing marks the asset changed, which is what has Bevy grow the GPU
/// buffer, so it happens once a frame however many ranges grew it.
fn grow(quads: Option<ResMut<QuadBuffer>>, mut buffers: ResMut<Assets<ShaderBuffer>>) {
    let Some(mut quads) = quads else {
        return;
    };
    if !std::mem::take(&mut quads.resized) {
        return;
    }
    if let Some(mut buffer) = buffers.get_mut(&quads.handle) {
        buffer.resize_buffer(u64::from(quads.capacity) * 4);
    }
}

#[derive(Resource, Default)]
struct PendingWrites {
    buffer: Option<AssetId<ShaderBuffer>>,
    writes: Vec<(u32, Vec<u8>)>,
}

fn take_writes(mut main: ResMut<MainWorld>, mut pending: ResMut<PendingWrites>) {
    if let Some(mut quads) = main.get_resource_mut::<QuadBuffer>() {
        pending.buffer = Some(quads.handle.id());
        let writes = std::mem::take(&mut quads.writes);
        pending.writes.extend(writes);
    }
}

/// Holds back what does not fit yet: the buffer it lands in may still be
/// growing, a frame behind its range.
fn apply_writes(
    mut pending: ResMut<PendingWrites>,
    buffers: Res<RenderAssets<GpuShaderBuffer>>,
    queue: Res<RenderQueue>,
) {
    let Some(gpu) = pending.buffer.and_then(|id| buffers.get(id)) else {
        return;
    };
    let size = gpu.buffer.size();
    pending.writes.retain(|(at, bytes)| {
        let offset = u64::from(*at) * 4;
        if offset + bytes.len() as u64 > size {
            return true;
        }
        queue.write_buffer(&gpu.buffer, offset, bytes);
        false
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quads() -> QuadBuffer {
        QuadBuffer::new(&mut Assets::default())
    }

    #[test]
    fn ranges_are_handed_out_in_order_and_freed_ones_reused() {
        let mut q = quads();
        let a = q.alloc(10);
        let b = q.alloc(20);
        let c = q.alloc(5);
        assert_eq!((a, b, c), (0, 10, 30));
        q.release(b, 20);
        assert_eq!(q.alloc(15), 10, "the first gap that fits");
        assert_eq!(q.alloc(5), 25, "and what is left of it");
        assert_eq!(q.alloc(1), 35, "past the last range");
    }

    #[test]
    fn freed_neighbours_merge_into_one_range() {
        let mut q = quads();
        let ranges: Vec<u32> = (0..4).map(|_| q.alloc(8)).collect();
        q.release(ranges[0], 8);
        q.release(ranges[2], 8);
        q.release(ranges[1], 8);
        assert_eq!(q.alloc(24), 0, "three freed in any order are one range");
    }

    #[test]
    fn a_range_that_fits_nowhere_doubles_the_buffer_and_keeps_the_rest() {
        let mut q = quads();
        let first = q.alloc(INITIAL_WORDS - 4);
        let grown = q.alloc(8);
        assert_eq!(first, 0);
        assert_eq!(grown, INITIAL_WORDS - 4, "the old tail joins the new space");
        assert_eq!(q.capacity, INITIAL_WORDS * 2);
        assert!(q.resized);
        assert_eq!(q.alloc(INITIAL_WORDS - 4), INITIAL_WORDS + 4);
    }

    #[test]
    fn writes_are_queued_in_order_for_the_render_world() {
        let mut q = quads();
        let at = q.alloc(3);
        q.write(at, &[1, 2, 3]);
        q.write(at + 1, &[9]);
        let bytes = |w: &[u32]| w.iter().flat_map(|w| w.to_ne_bytes()).collect::<Vec<u8>>();
        assert_eq!(q.writes, vec![(0, bytes(&[1, 2, 3])), (1, bytes(&[9]))]);
        assert_eq!(q.mirror, [1, 9, 3]);
    }
}
