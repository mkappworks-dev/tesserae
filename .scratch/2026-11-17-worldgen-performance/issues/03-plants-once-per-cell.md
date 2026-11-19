# 03: The Plants on a Cell are decided once

Status: ready-for-agent

Blocked by: 02

Spec: [Worldgen performance](../spec.md).

## What to build

The Plants standing on a Cell's columns are decided once, with the crop fields and footprints that decide them, and kept in a bounded map of compact entries (the feature, the base and the draw). A Chunk takes the Plants of its own and neighbouring Cells within a tree's reach and stands those whose Voxels fall in it, so the ten or so Chunks over a Cell stop deciding the same trees again. Reaching the bound drops the oldest half. Spawning clear of Plants reads the same map.

## Acceptance criteria

- [ ] Version 3 Chunks match the ticket 01 fingerprint; the committed version 3 digests pass unchanged.
- [ ] A tree reaching across a Chunk border is the same from both Chunks, warm or cold, in any order.
- [ ] A generator without Plants fills no entry.
- [ ] Bench figures before and after, and the mid-point flight profile, in Comments.
