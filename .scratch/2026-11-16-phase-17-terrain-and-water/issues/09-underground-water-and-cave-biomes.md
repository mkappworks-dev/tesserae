# 09: Underground water, lava and cave Biomes (17c)

Status: ready-for-agent

Blocked by: 06, 08, 11

Spec: [Phase 17](../spec.md).

## What to build

Springs, a water table, aquifers, underground lakes and rivers, lava near the floor, and cave Biomes chosen in 3D from a declared list capped at 16, each with its own ground, features and fog.

## Acceptance criteria

- [ ] A cave Biome is chosen the same at every Voxel size and Chunk order.
- [ ] Cave water uses the Game's fluid; none declared means dry.
- [ ] More than 16 cave Biomes is a load error naming the Biome.
