# 01: A version 3 bench and the starting figures

Status: ready-for-agent

Blocked by: None (can start immediately)

Spec: [Worldgen performance](../spec.md).

## What to build

A bench that times version 3's Chunks as interleaved CPU time, in the style of the Structures bench, over a synthetic World shaped like Terra: a forest Biome with a clumped tree, a dry Biome with its own ground and cave scale, and the fixture's ores, water and caves. Its Chunks, each found by a fixed scan: straddling, the sky and deep-rock early-outs, a river, a cavern, a coarse level-3 Chunk and a whole column of Chunks under the forest. A binary of the starting commit is kept for every later A/B. An uncommitted fingerprint of a fixed grid of version 3 Chunks at 1.0 and 0.5 m is recorded, against which every later ticket proves it changed nothing.

## Acceptance criteria

- [ ] The bench runs every case interleaved and prints medians of thread CPU time and the load it ran under.
- [ ] The starting bench figures and the starting flight profile (`scripts/profile-frames`, three interleaved rounds, worker time per level and stage) are in this ticket's Comments, with `uptime` and how they were taken.
- [ ] The starting fingerprint is kept with the baseline binary, outside the commit.
- [ ] No generator code changes.
