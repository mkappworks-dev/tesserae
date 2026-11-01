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

## Comments

### 2026-11-01: implemented; gate pending the final review series

Built: `client/chunk_region.rs`. Opaque and cutout layers are merged per region,
a cube of 2 Chunks an edge on each level's own lattice, drawn by one entity per
layer with the layer's one shared material. Each Chunk keeps its packed layer
(`chunk_mesh::pack`, positions from its own origin), and `assemble_regions`,
after `release` in the view chain, re-assembles once a frame only the regions
whose members changed (`merged_mesh` offsets each part by its place in the
region). The region's mesh is replaced in place, so its entity and handle stay
and Bevy refreshes its `Aabb` through `AssetChanged`. Blended stays one entity
per Chunk for `blend_order`. Camera collision reads each level-0 Chunk's packed
part instead of its entity's mesh. `TESSERAE_REGION_CHUNKS=1|2|4|..|32` under
`dev` or `profiling` sets the edge, 1 being the old entity per Chunk, so one
binary covers the comparison.

Visibility: Bevy's frustum (and any occlusion) culling sees a region whole.
Coverage and retention stay per Chunk: the shader discards by world position
against the per-Chunk mask, and a released Chunk leaves its region's mesh in
the frame its bit clears. Per-Chunk culling for 10 and 12 stays possible the
way a re-mesh is: leave the Chunk out of its region's next assembly, at one
region's rebuild.

Tests: neighbouring Chunks share one entity and each keeps its own geometry in
world space; a re-mesh of two Chunks in one frame rebuilds their region once
and no other region or level; a released retained Chunk leaves its region's
mesh in the frame its mask bit clears while its delivered neighbour stays; the
far corner of the widest region, with a flame a sixteenth outside, packs
exactly. The blended order tests pass unchanged.

Single captures (`/tmp/claude-streaming-impl/review09`, `shot.sh`), not a
series. Stationary, 16-20 s, by region edge:

| edge     | chunk_entities | visible_meshes_camera0 | render CPU p50 | frame p50 |
| -------- | -------------- | ---------------------- | -------------- | --------- |
| 08 (bin) | 1231           | 291                    | 1.32-1.38      | 4.75-4.84 |
| 1        | 1231           | 291                    | 1.36           | 4.84      |
| 2        | 625            | 159                    | 1.51           | 4.92      |
| 4        | 453            | 120                    | 1.36-1.51      | 5.18-5.19 |

`sprint-flight` at 64 m/s, 22-32 s: edge 1, 1436 entities, render CPU p50
1.65 ms, frame p50 5.03 ms; edge 2, 707, 1.52, 5.08; edge 4, 507, 1.58, 5.28
(max 14.8 against 11.1). Rule-hole and fill-hole frames 0 in every run.
Captures against `bin-08-7cd09f8` (stationary, teleport, lifted 90 m at -5
deg) differ in 0-64 pixels by more than 24/255; 08 against itself differs in
23.

Findings: render CPU barely follows the entity count while static, since Bevy
retains extraction and bins for unchanged entities. Larger regions cost GPU
time: GPU frame p50 5.11 ms at edge 1 against 5.53 ms at edge 4
(`TESSERAE_PROFILE_GPU=1`), equally in frames with and without uploads, so
culling a region whole draws more geometry. Hence the edge of 2. Main-thread
assembly runs 0.04-0.07 ms a frame on average while sprinting.

The effect gate is decided by the final review series, which can run edges 1,
2 and 4 from one binary. Status unchanged.
