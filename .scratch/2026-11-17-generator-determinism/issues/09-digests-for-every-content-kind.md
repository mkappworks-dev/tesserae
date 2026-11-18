# 09: Digests cover every content kind and spawn

Status: ready-for-agent

Spec: [Generator determinism](../spec.md).

Blocked by: 06, 07, 08.

## What to build

Digest cases over 08's content, recorded on macOS against today's code. Each
`(seed, Voxel size)` case adds a box on the spawn point sized to reach the
Settlement's disc and a box on a Structure, found by sweeping generated Chunks
for the Template's Material. One case's origin stands in the sea, so the dry
land search runs. Each Voxel's generated Overlay, derived through the Plant
table, is hashed by fluid name, level and source flag. Each case records its
spawn position from 06's query.

## Acceptance criteria

- [ ] Each box asserts what it holds before its digest is compared: a Settlement
      building, a Structure, a Plant, water.
- [ ] Spawn positions are recorded per case and compared bit for bit.
- [ ] Some case reaches each site the correction touches: Settlement founding,
      the spawn ring's angle step, roads, building orientation, the level-disc
      test, crop field reach near water, the dry land search, Surface feature
      chance, tree crowding, and the landform and wetness `powi` (the existing
      boxes may be cited). Shown once with a temporary counter, not committed,
      and noted in a comment here.
- [ ] The digest tests take under a minute in a release build; the time is noted
      in a comment here.
