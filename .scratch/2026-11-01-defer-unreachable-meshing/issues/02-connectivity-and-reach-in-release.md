# 02: Connectivity and `reach` in release

Status: wontfix
Blocked by: a reopen trigger in the spec

Spec: [defer unreachable meshing](../spec.md). Kept for a reopen; see the
spec's Decision. A prefactor: it changes no output, so ticket 03 can.

## What to build

- The generation job works out a Chunk's connectivity in every build, and the
  mesh job again after an Edit, as `profiling` builds do now. The Server keeps
  it per Chunk in every build.
- The `reach` walk from the Eye's Chunk, over the load extent, runs in every
  build, but only when the Eye changes Chunk or a connectivity inside the load
  extent changes, the way 03 re-scores only on an anchor Cell change. Its
  result is a set dispatch can read.
- Its tick cost goes in a `reach_ms` column beside `rescore_ms`.
- The Client gets nothing new: `ChunkMeshReady` carries connectivity under
  `profiling` only, for 12's sight count (ADR-0002).

## Acceptance criteria

- [ ] Connectivity costs under 5% of mesh job time, on 12's criterion
      `meshing` rows; its share of generation job time reported.
- [ ] The walk costs at most 0.2 ms of tick at p99 in `sprint-flight`.
- [ ] Output unchanged: identical captures in `stationary` and `cave`.
- [ ] `tesserae-core` free of bevy, wgpu and rayon.
