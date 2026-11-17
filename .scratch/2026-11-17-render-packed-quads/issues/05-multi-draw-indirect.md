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
