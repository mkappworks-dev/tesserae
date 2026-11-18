# 04: Digests cover every content kind and spawn

Status: ready-for-agent

Spec: [Generator determinism](../spec.md).

Blocked by: 01, 02, 03.

## What to build

Digest cases over 03's content, recorded on macOS against today's code. Versions
2 and 3 with ground and Biomes get their own recorded digests; the no-Biome
version 2 case stays as it is. Each `(version, seed, Voxel size)` case adds a
box on the spawn point sized to reach the Settlement's disc and a box on a
Structure, found by sweeping generated Chunks for the Template's Material. One
version 3 case's origin stands in the sea, so the dry land search runs. Each
Voxel's generated Overlay, derived through the Plant table, is hashed by fluid
name, level and source flag. Each case records its spawn position from 01's
query.

## Acceptance criteria

- [ ] Each box asserts what it holds before its digest is compared: a Settlement
      building, a Structure, a Plant, water.
- [ ] Spawn positions are recorded per case and compared bit for bit.
- [ ] Some case reaches each site the correction touches: Settlement founding,
      the spawn ring's angle step, roads, building orientation, the version 3
      level-disc test, crop field reach near water, the dry land search, Surface
      feature chance, tree crowding, and version 3's landform and wetness `powi`
      (the existing version 3 boxes may be cited). Shown once with a temporary
      counter, not committed, and noted in a comment here.
- [ ] Version 1's digests and the no-Biome version 2 case are unchanged.
- [ ] The digest tests take under a minute in a release build; the time is noted
      in a comment here.
