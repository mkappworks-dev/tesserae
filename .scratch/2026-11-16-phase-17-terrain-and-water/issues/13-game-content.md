# 13: Games declare the new Biomes and water (17d)

Status: done

Blocked by: 09, 11, 12

Spec: [Phase 17](../spec.md).

## What to build

Each demo Game declares ocean, shore, river, alpine and its own Biomes and its water fluid. Meadow stands its first footbridges.

## Acceptance criteria

- [x] Every new Material is Obtainable (Terra's new Biomes use Materials it already has).
- [x] Each Game's World has ocean, a river and a cave Biome within a short flight (4 km; at the test seed rivers run some 3.5 km from spawn).

Built:

- Every demo Game declares ocean, shore (Terra's beach), river and alpine Biomes by the land's shape, and two or three cave Biomes in its own Materials; Ember and Terra name their lava.
- Meadow and Lumen gain water a metre under Sea level, where version 2's ground never sinks, so only version 3 shows their sea.
- A river Biome reads the new `river` selector.
- Meadow's footbridge grows to 17 m, long enough to land dry either side of a version 3 river, and stands over its rivers.
- The Games' pinned Biome tables leave out the Biomes of the land's shape, which version 2 never chooses.
