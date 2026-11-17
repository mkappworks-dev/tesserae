# 16: Cave Biome names in the HUD

Status: done

Blocked by: 09

Spec: [Phase 17](../spec.md).

## What to build

Underground in a cave Biome, the minimap's band and the pause menu name the cave
Biome, not the surface Biome over it. The Server sends the cave Biome at the Eye
as it changes, and the World's facts list the cave Biomes' names.

## Acceptance criteria

- [x] In a cave Biome the Server sends its index and its fog.
- [x] The minimap's band names the cave Biome underground and the surface Biome
      again above it.
