# 15: Plots are levelled and roads graded

Status: ready-for-agent

Blocked by: 14

Spec: [Phase 20](../spec.md).

## What to build

Each Plot is cut and filled level under its footprint and a 2 m apron, by at
most 2 m, blended out to the natural ground. Roads are laid in the path Material
and become stairs and slabs where they climb. Voxels are edited in the pass; the
heightmap is unchanged.

## Acceptance criteria

- [ ] Every Plot is level across its footprint, and no cut or fill exceeds 2 m.
- [ ] The apron blends into the natural ground with no cliff at its edge, by a
      slope test.
- [ ] A road never rises more than a slab per step and is walkable by the
      Player's step height.
- [ ] Levelling is the same whichever Chunk is generated first.
- [ ] Wilderness Structures keep the strict level rule and are never levelled.
