# 12: Plants in worldgen

Status: ready-for-agent

Blocked by: 02, 07, 08, 09, 10

Spec: [Phase 18](../spec.md).

## What to build

A Biome's `surface-features` can name a Plant. Generation places crops, ground
cover and vines at a hash-drawn Stage and full-grown trees, under the existing
feature rules (a full cube of the surface Material, Air above, inside the global
surface bound, never under a fluid or on a stair or slab or in a cave mouth); a
kelp `column` stands in the lake. A tree is a pure function of the seed and a
column position in metres, so each Chunk generates the Voxels of neighbouring
columns that fall inside it, scanning the Chunks within `canopy-radius-m`. Level
0 only; no new Generator version.

## Acceptance criteria

- [ ] A tree across a Chunk border is whole, and equal at 1.0, 0.5 and 0.25 m
      in metres, in a test that generates both neighbours in either order.
- [ ] A generated fruit tree has fruit at hash-drawn Stages inside its density,
      the same whichever Chunk generates a Voxel of it.
- [ ] A version 2 World whose Biomes name no Plant generates the digests
      recorded before Plants existed.
- [ ] Plant rows ride in the World's Biome snapshot; a World saved before
      Plants keeps its bare ground.
- [ ] Coarse levels generate without Plants and do not vote on them.
- [ ] A Chunk's generation time with trees does not exceed `main`'s by more
      than the bench's agreed margin.
