# 12: Count what a visibility graph would cull or defer

Status: ready-for-agent
Blocked by: 01, 03

Spec: [streaming performance](../spec.md).
PR boundary: own branch `tech/streaming-visibility`.

## What to build

- The 15-bit face-to-face connectivity of each Chunk, as Minecraft and Sodium
  compute it: in the generation job, and again in the mesh job after a change,
  so it stays off the tick (ADR-0007). It ships with the mesh; the Client never
  reads World storage (ADR-0002).
- A `cave` fixture: the Eye inside a cave of the demo's default seed, turning
  through 360 degrees in the measured phase.
- Two profiling-only walks, nothing culled or reordered yet:
  - on the Client, from the camera's Chunk: how many drawn Chunks it would
    cull;
  - on the Server, from the Eye's Chunk: the share of level-0 mesh worker time
    spent on Chunks it cannot reach, such as sealed caves.
- Build Client culling only if it removes at least 20% of drawn Chunks in a
  surface or cave fixture. It must not cull a Chunk 02 is retaining.
- Move unreachable Chunks to the lowest score in 03's order only if they take
  more than 30% of level-0 mesh worker time in a sprint fixture. They are never
  skipped, so entering a cave still finds them meshed.
- Client culling is a per-view optimization, so it is adopted only on 01's GPU
  and render attribution and the spec's effect gate.

## Acceptance criteria

- [ ] Connectivity computation costs under 5% of mesh job time.
- [ ] Cull fraction and unreachable mesh share recorded per fixture here.
- [ ] If built: zero rule-hole frames, no popping in `cave` review captures,
      and the spec's effect gate passed.

## Comments
