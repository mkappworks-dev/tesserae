# 05: Multi-draw indirect and GPU culling

Status: needs-info
Blocked by: 01, 02

Spec: [Packed quads](../spec.md).

## What to build

Item 4, only if ticket 01 points there: Chunk layers in one buffer drawn by
multi-draw indirect, culled on the GPU.

What is known from Bevy 0.20.1 and wgpu 30 on Metal:

- Draws are indirect already (GPU preprocessing in culling mode).
- Multi-draw indirect is a CPU loop of single draws on Metal, and
  `MULTI_DRAW_INDIRECT_COUNT` is unsupported, so no draw call is saved.
- GPU occlusion culling is likely usable but needs a depth prepass.

## Acceptance criteria

- [ ] Ticket 01's numbers show draw submission or culling as a cost.
- [ ] Otherwise `wontfix`, with the reason here.

## Comments

2026-11-19: kept `needs-info` by the user's choice, not `wontfix`. The limit is
wgpu's, not the hardware's or Bevy's: Apple GPUs can run GPU-written draws
through Metal's indirect command buffers, but wgpu-hal's Metal backend loops
single indirect draws and offers no GPU-written draw count. On Vulkan and DX12
(NVIDIA, AMD) multi-draw indirect is native, and Bevy's GPU-driven batching
already merges draws that share a pipeline, bind group and mesh slab, which
likely covers the Chunk layers. Revisit when a Windows or Linux build exists
(check first whether Bevy's batching already merges Chunk layers there) or when
wgpu's Metal backend uses indirect command buffers. Ticket 01's frame is
GPU-bound (about 4 ms GPU against 0.7 ms main CPU), so draw submission is not
the cost on this host.
