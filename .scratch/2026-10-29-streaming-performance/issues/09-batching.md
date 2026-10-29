# 09: Batch opaque and cutout

Status: ready-for-agent
Blocked by: 08

Spec: [streaming performance](../spec.md).
PR boundary: own branch `tech/streaming-batching`.

## What to build

- Merge opaque and cutout Chunk meshes into region meshes or batches, so draws
  and entities scale with regions rather than Chunks, using 08's
  region-local positions. Blended stays one entity per Chunk, because
  `blend_order.rs` sorts it per Chunk.
- Share one material instance across a region: a non-bindless
  `MaterialExtension` batches only with meshes sharing its instance and slab.
- Keep per-Chunk visibility possible inside a region, for 12's culling and
  10's results: record how, or why a region is culled whole.
- Stay within bevy_pbr and `MaterialExtension`.

## Acceptance criteria

- [ ] 01's entity and draw-call counts per frame drop against the baseline.
- [ ] Render CPU in the sprint fixtures passes the spec's effect gate.
- [ ] A Chunk re-mesh rebuilds at most its own region.
- [ ] Blended order tests keep passing; zero rule-hole frames still.

## Validation and authorization

frame-profiling.md. Seam: `client/chunk_view.rs`, which 07 also changes; rebase
on whichever lands first. Local work only; no push or PR without approval.
