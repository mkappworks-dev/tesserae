# 02: Each column's Biome is asked once per Cell

Status: ready-for-agent

Blocked by: 01

Spec: [Worldgen performance](../spec.md).

## What to build

The per-Cell surface cache, renamed so it is no longer called a Stack, also holds each column's Biome, and every Biome a Chunk asks for reads it: Plants, grounds, ores and the cave-scale blend. A column outside the Chunk's Cell builds its neighbour's cache. Looking a Cell up takes a read lock and first checks the last Cell this thread asked; reaching the bound of 1024 Cells drops the oldest half.

## Acceptance criteria

- [ ] Version 3 Chunks match the ticket 01 fingerprint at 1.0 and 0.5 m; the committed version 3 digests pass unchanged.
- [ ] A Chunk generated after every cache is dropped equals one generated warm, in any order, with Biomes, Plants and ores.
- [ ] Reaching the bound keeps the newer half of the Cells.
- [ ] Bench figures before and after, against the baseline binary, in Comments.
