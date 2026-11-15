# 05: Trees, scatter and fields keep off a Structure

Status: ready-for-agent

Blocked by: 03

Spec: [Phase 20](../spec.md).

## What to build

Trees, surface scatter and crop fields skip every column under a Structure's
footprint rectangle, after rotation. A Structure's acceptance never asks about
them, so the two never form a cycle.

## Acceptance criteria

- [ ] No tree, scatter Voxel or field cell stands in a Structure's footprint.
- [ ] A Biome with Structures and trees is whole and order-independent across
      Chunk borders.
- [ ] A World with no Structure rows generates the same trees, scatter and
      fields as before, by recorded digest.
