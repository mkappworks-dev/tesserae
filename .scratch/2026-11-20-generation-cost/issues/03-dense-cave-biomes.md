# 03: A Chunk's cave Biomes in a dense array

Status: ready-for-agent

Blocked by: 01

Spec: [Generation cost](../spec.md).

## What to build

`Worldgen::chunk` keeps the cave Biome of each cave Biome cell it asks in a dense array indexed from the Chunk's origin cell, instead of a SipHash map keyed by `[i32; 3]`. Where a Voxel is at least a cave Biome cell wide, nothing is kept, since no two Voxels share a cell. `cave_biome_at` reads a noise corner cache for its two fields.

## Acceptance criteria

- [ ] Version 3 matches the fingerprint at 1.0 and 0.5 m, the crop-field grid included; the committed version 3 digests pass unchanged.
- [ ] Cave Biome floors and rock match at 1.0, 0.5 m and at a LOD level whose Voxel is wider than a cell.
- [ ] Bench figures against 01's binary in Comments, with `uptime`.
