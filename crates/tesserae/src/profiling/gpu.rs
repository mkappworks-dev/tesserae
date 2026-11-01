//! GPU time per view and phase, from timestamps at the boundaries of empty
//! compute passes.
//!
//! Apple GPUs sample only at encoder boundaries (wgpu's `TIMESTAMP_QUERY`
//! without `TIMESTAMP_QUERY_INSIDE_ENCODERS`), so Bevy's own render diagnostics
//! record no GPU time on Metal. A pass placed between two of Bevy's passes
//! marks the moment the GPU reached it; it dispatches one empty workgroup,
//! because Metal skips an encoder with no work and samples nothing. A marker is ordered by the schedule, not by the GPU: work
//! the GPU overlaps across a boundary is charged to whichever side it ends on.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use bevy::core_pipeline::core_3d::{main_opaque_pass_3d, main_transparent_pass_3d};
use bevy::core_pipeline::schedule::{Core3d, Core3dSystems};
use bevy::prelude::*;
use bevy::render::camera::ExtractedCamera;
use bevy::render::render_resource::{BufferDescriptor, BufferUsages, CommandEncoder, MapMode};
use bevy::render::renderer::{
    RenderContext, RenderDevice, RenderGraph, RenderGraphSystems, RenderQueue, ViewQuery,
};
use bevy::render::{Render, RenderApp};

/// Frames whose timestamps can be in flight at once.
const SLOTS: usize = 8;
/// Markers one frame may write: a start and an end, and six per view.
const PER_SLOT: u32 = 64;

/// `(frame id, marker, milliseconds since the frame's first marker)`; `None`
/// when the sample was not this frame's.
pub type GpuRow = (usize, String, Option<f64>);

#[derive(Resource, Default, Clone)]
pub struct GpuSamples(pub Arc<Mutex<Vec<GpuRow>>>, pub Arc<AtomicBool>);

struct Slot {
    read: wgpu::Buffer,
    mapped: Arc<AtomicBool>,
    busy: bool,
    frame: Option<usize>,
    labels: Vec<String>,
}

#[derive(Resource)]
struct Timer {
    pipeline: Option<wgpu::ComputePipeline>,
    queries: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    slots: Vec<Slot>,
    current: Option<usize>,
    /// Ended last frame and resolved at the start of this one: a sample read
    /// back in the command buffer that wrote it, or in one submitted just after,
    /// was sometimes still the slot's previous value.
    ended: Option<usize>,
    next: usize,
    period_ns: f32,
}

pub(super) fn plugin(app: &mut App, samples: GpuSamples) {
    let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
        return;
    };
    render_app
        .insert_resource(samples)
        .add_systems(RenderGraph, begin.in_set(RenderGraphSystems::Begin))
        .add_systems(RenderGraph, end.in_set(RenderGraphSystems::Finish))
        .add_systems(
            Core3d,
            (
                mark::<0>.before(Core3dSystems::Prepass),
                mark::<1>
                    .after(Core3dSystems::Prepass)
                    .before(Core3dSystems::MainPass),
                // Shadows are ordered only before the main pass, so they fall
                // between these two.
                mark::<2>
                    .in_set(Core3dSystems::MainPass)
                    .before(main_opaque_pass_3d),
                mark::<3>
                    .in_set(Core3dSystems::MainPass)
                    .after(main_opaque_pass_3d)
                    .before(main_transparent_pass_3d),
                mark::<4>
                    .after(Core3dSystems::MainPass)
                    .before(Core3dSystems::EarlyPostProcess),
                mark::<5>.after(Core3dSystems::PostProcess),
            ),
        )
        .add_systems(Render, create.run_if(not(resource_exists::<Timer>)));
}

const MARKS: [&str; 6] = [
    "view_start",
    "prepass_end",
    "main_start",
    "opaque_end",
    "main_end",
    "view_end",
];

fn create(
    mut commands: Commands,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    samples: Res<GpuSamples>,
) {
    if !device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
        samples.1.store(false, Ordering::Relaxed);
        commands.insert_resource(Timer::unavailable(&device));
        return;
    }
    samples.1.store(true, Ordering::Relaxed);
    let bytes = u64::from(PER_SLOT) * 8;
    let device_ref = device.wgpu_device();
    let queries = device_ref.create_query_set(&wgpu::QuerySetDescriptor {
        label: Some("profile_gpu_timestamps"),
        ty: wgpu::QueryType::Timestamp,
        count: PER_SLOT * SLOTS as u32,
    });
    let resolve = device_ref.create_buffer(&BufferDescriptor {
        label: Some("profile_gpu_resolve"),
        size: bytes * SLOTS as u64,
        usage: BufferUsages::QUERY_RESOLVE | BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let slots = (0..SLOTS)
        .map(|_| Slot {
            read: device_ref.create_buffer(&BufferDescriptor {
                label: Some("profile_gpu_read"),
                size: bytes,
                usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            mapped: Arc::default(),
            busy: false,
            frame: None,
            labels: Vec::new(),
        })
        .collect();
    let shader = device_ref.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("profile_gpu_marker"),
        source: wgpu::ShaderSource::Wgsl("@compute @workgroup_size(1) fn main() {}".into()),
    });
    let pipeline = device_ref.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("profile_gpu_marker"),
        layout: None,
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    commands.insert_resource(Timer {
        pipeline: Some(pipeline),
        queries,
        resolve,
        slots,
        current: None,
        ended: None,
        next: 0,
        period_ns: queue.get_timestamp_period(),
    });
}

impl Timer {
    /// A timer that never records, so the systems need no second condition.
    fn unavailable(device: &RenderDevice) -> Self {
        let device = device.wgpu_device();
        Timer {
            pipeline: None,
            queries: device.create_query_set(&wgpu::QuerySetDescriptor {
                label: None,
                ty: wgpu::QueryType::Occlusion,
                count: 1,
            }),
            resolve: device.create_buffer(&BufferDescriptor {
                label: None,
                size: 8,
                usage: BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }),
            slots: Vec::new(),
            current: None,
            ended: None,
            next: 0,
            period_ns: 0.0,
        }
    }

    fn write(&mut self, encoder: &mut CommandEncoder, label: String) {
        let (Some(slot), Some(pipeline)) = (self.current, &self.pipeline) else {
            return;
        };
        let labels = &mut self.slots[slot].labels;
        if labels.len() as u32 >= PER_SLOT {
            return;
        }
        let index = slot as u32 * PER_SLOT + labels.len() as u32;
        labels.push(label);
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("profile_gpu_marker"),
            timestamp_writes: Some(wgpu::ComputePassTimestampWrites {
                query_set: &self.queries,
                beginning_of_pass_write_index: Some(index),
                end_of_pass_write_index: None,
            }),
        });
        pass.set_pipeline(pipeline);
        pass.dispatch_workgroups(1, 1, 1);
    }
}

fn begin(
    timer: Option<ResMut<Timer>>,
    samples: Res<GpuSamples>,
    span: Res<super::RenderSpan>,
    (device, queue): (Res<RenderDevice>, Res<RenderQueue>),
    mut ctx: RenderContext,
) {
    let Some(mut timer) = timer else {
        return;
    };
    if let Some(slot) = timer.ended.take() {
        timer.resolve(slot, &device, &queue);
    }
    let period_ns = f64::from(timer.period_ns);
    for slot in &mut timer.slots {
        if !slot.busy || !slot.mapped.load(Ordering::Acquire) {
            continue;
        }
        {
            let Ok(bytes) = slot
                .read
                .slice(..slot.labels.len() as u64 * 8)
                .get_mapped_range()
            else {
                continue;
            };
            let ticks: Vec<u64> = bytes
                .as_chunks::<8>()
                .0
                .iter()
                .map(|&b| u64::from_le_bytes(b))
                .collect();
            if let Some(frame) = slot.frame {
                let first = ticks.first().copied().unwrap_or(0);
                let mut rows = samples.0.lock().expect("not poisoned");
                for (label, &tick) in slot.labels.iter().zip(&ticks) {
                    // Some samples come back as the slot's value from an earlier
                    // frame; one before this frame's start is left out.
                    let ms = (tick >= first).then(|| (tick - first) as f64 * period_ns / 1e6);
                    rows.push((frame, label.clone(), ms));
                }
            }
        }
        slot.read.unmap();
        slot.mapped.store(false, Ordering::Release);
        slot.busy = false;
    }
    let next = timer.next;
    timer.current = timer
        .slots
        .get(next)
        .filter(|slot| !slot.busy)
        .map(|_| next);
    if let Some(slot) = timer.current {
        timer.next = (next + 1) % SLOTS;
        timer.slots[slot].labels.clear();
        timer.slots[slot].frame = span.id;
        timer.write(ctx.command_encoder(), "frame_start".into());
    }
}

fn mark<const M: usize>(
    timer: Option<ResMut<Timer>>,
    view: ViewQuery<&ExtractedCamera>,
    mut ctx: RenderContext,
) {
    let Some(mut timer) = timer else {
        return;
    };
    let order = view.into_inner().order;
    timer.write(ctx.command_encoder(), format!("camera{order}:{}", MARKS[M]));
}

fn end(timer: Option<ResMut<Timer>>, device: Res<RenderDevice>, queue: Res<RenderQueue>) {
    let Some(mut timer) = timer else {
        return;
    };
    let Some(slot) = timer.current else {
        return;
    };
    let mut encoder = device.create_command_encoder(&Default::default());
    timer.write(&mut encoder, "frame_end".into());
    queue.submit([encoder.finish()]);
    timer.current = None;
    timer.ended = Some(slot);
    timer.slots[slot].busy = true;
}

impl Timer {
    fn resolve(&mut self, slot: usize, device: &RenderDevice, queue: &RenderQueue) {
        let count = self.slots[slot].labels.len() as u32;
        let first = slot as u32 * PER_SLOT;
        let offset = u64::from(first) * 8;
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.resolve_query_set(&self.queries, first..first + count, &self.resolve, offset);
        encoder.copy_buffer_to_buffer(
            &self.resolve,
            offset,
            &self.slots[slot].read,
            0,
            u64::from(count) * 8,
        );
        queue.submit([encoder.finish()]);
        let mapped = Arc::clone(&self.slots[slot].mapped);
        // A failed map leaves the slot busy for good: its later turns go
        // unrecorded rather than misread.
        self.slots[slot]
            .read
            .slice(..u64::from(count) * 8)
            .map_async(MapMode::Read, move |result| {
                mapped.store(result.is_ok(), Ordering::Release);
            });
    }
}
