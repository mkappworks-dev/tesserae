# 13: Games declare the new Biomes and water (17d)

Status: ready-for-agent

Blocked by: 09, 11, 12

Spec: [Phase 17](../spec.md).

## What to build

Each demo Game declares ocean, shore, river, alpine and its own Biomes and its water fluid. Meadow stands its first footbridges.

## Acceptance criteria

- [x] Every new Material is Obtainable (Terra's new Biomes use Materials it already has).
- [ ] Each Game's World has ocean, a river and a cave Biome within a short flight.

Built: Terra declares ocean, beach and alpine Biomes by altitude. Not built: the other four Games, a river Biome (no selector reads a river), the Games' water fluid declarations, Meadow's footbridges, and a flight check of ocean, river and cave per Game.
