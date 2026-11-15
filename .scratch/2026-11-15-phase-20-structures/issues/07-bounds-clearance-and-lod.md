# 07: Bounds, spawn and coarse levels know about Structures

Status: ready-for-agent

Blocked by: 03

Spec: [Phase 20](../spec.md).

## What to build

The vertical bounds the Server trusts for streaming and open sky widen by the
tallest and deepest Template any Biome lists. Spawn clearance counts Structures,
so a Player never begins inside one. Coarse levels leave Structures out and do
not vote on them.

## Acceptance criteria

- [ ] A tower taller than the terrain band and a cellar deeper than the deep-
      rock early-out are generated and lit correctly.
- [ ] A World with no Structure keeps its exact bounds and spawn clearance,
      pinned by a test.
- [ ] A Player never spawns inside a Structure.
- [ ] Coarse levels contain no Structure Voxel.
