# 09: Underground water, lava and cave Biomes (17c)

Status: ready-for-agent

Blocked by: 06, 08, 11

Spec: [Phase 17](../spec.md).

## What to build

Springs, a water table, aquifers, underground lakes and rivers, lava near the floor, and cave Biomes chosen in 3D from a declared list capped at 16, each with its own ground, features and fog.

## Acceptance criteria

- [ ] A cave Biome is chosen the same at every Voxel size and Chunk order.
- [x] Cave water uses the Game's fluid; none declared means dry.
- [ ] More than 16 cave Biomes is a load error naming the Biome.

Built: caverns under the water table (2 m under Sea level) are underground lakes of the Game's fluid. Not built: springs, aquifers, lava near the floor (a Game cannot name a lava fluid yet), cave Biomes chosen in 3D and the 16-Biome cap for them; the first two criteria stay open.
