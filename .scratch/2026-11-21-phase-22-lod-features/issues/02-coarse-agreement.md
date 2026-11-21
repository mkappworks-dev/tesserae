# 02: Coarse agreement

Status: ready-for-agent
Blocked by: 01

Spec: [LOD features](../spec.md).
[ADR-0035](../../../docs/adr/0035-a-coarse-level-draws-level-0s-answer.md).

## What to build

Flying toward a coast, the shore stays where it is. Every coarse level answers
water or land, and its top Material, as level 0 does at each column's centre in
metres: the same terrain height against that column's own water level, sea or
lake, the Biome sampled at the centre. A dry column's top is raised to at least
that water level. A column under an Edit or saved change keeps the downsample's
answer. This
replaces the per-level half-Voxel lowering and its tie, so level 4 draws its
shallow sea again. At levels 1 and 2 it applies after the downsample.

The rule lives in the coarse path only: `Worldgen` at a Voxel size is also
level 0 for a World of that size, and version 3's output is frozen.

## Acceptance criteria

- [ ] 01's agreement test asserts exact agreement on water or land and top
      Material at every sampled centre, levels 1 to 5.
- [ ] Every existing worldgen, determinism and golden test passes unchanged.
- [ ] A Chunk with an Edit or saved change still shows the change at every level
      it did.
- [ ] Coarse-Chunk CPU time per level from an interleaved micro-benchmark
      against the integration branch's tip, in Comments.
- [ ] Captures of a coast at each ring join, for the review batch after 05.
