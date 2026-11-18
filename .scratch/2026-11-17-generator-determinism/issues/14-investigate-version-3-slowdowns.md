# 14: Investigate two version 3 slowdowns

Status: ready-for-agent

Spec: [Generator determinism](../spec.md). Belongs with the worldgen version 3
performance work; move it there if that spec merges first.

Blocked by: None (can start immediately).

## What to build

Two slowdowns ticket 02 found when its benches moved to version 3 (see its
Comments) are explained, and fixed where the fix leaves generator output
bit-identical:

1. `lod/coarse_chunk/air_level_1` no longer takes its early-out (416 ns before,
   129 µs after).
2. In the `structures` bench a fresh generator costs about eight times a warm
   one per Chunk (451 µs against 3490 µs, bare).

## Acceptance criteria

- [ ] The cause of each is written in a comment here.
- [ ] Any fix moves no recorded digest.
- [ ] Figures are taken as interleaved CPU-time A/B against the integration
      branch, with the host load beside them.
