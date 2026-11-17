# 09: Underground water, lava and cave Biomes (17c)

Status: done

Blocked by: 06, 08, 11

Spec: [Phase 17](../spec.md).

## What to build

Springs, a water table, aquifers, underground lakes and rivers, lava near the floor, and cave Biomes chosen in 3D from a declared list capped at 16, each with its own ground, features and fog.

## Acceptance criteria

- [x] A cave Biome is chosen the same at every Voxel size and Chunk order.
- [x] Cave water uses the Game's fluid; none declared means dry.
- [x] More than 16 cave Biomes is a load error naming the Biome.

Built:

- Caverns under the water table are underground lakes of the Game's fluid.
- Perched aquifers raise the table across a third of 192 m regions, level within each; rock walls stand along a raised region's border so no lake drains into its neighbour.
- Springs well up as sealed one-Voxel pools in an aquifer's dry cavern floors; they stand rather than pour, so a fresh Chunk sets no fluid moving.
- A Game names its lava (`world.lava`); caverns within 10 m of the band's floor hold it, under 4 m of rock that keeps every cavern above apart.
- Cave Biomes (`world.cave-biomes`, at most 16, from the Game) are chosen by two 3D fields on a 4 m lattice in metres, from 12 m under the surface, each with its own rock, floor, features on dry floors and fog; the Server sends a cave Biome's fog when the Eye is in it.
- All of it version 3 only, saved by name in the generation snapshot.
