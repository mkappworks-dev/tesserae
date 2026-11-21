# 03: Tree Impostors at levels 1 and 2

Status: ready-for-agent
Blocked by: 02

Spec: [LOD features](../spec.md).
[ADR-0035](../../../docs/adr/0035-a-coarse-level-draws-level-0s-answer.md).

## What to build

Trees stand on the ground at levels 1 and 2, exactly where level 0 stands them,
and do not pop in at the join.

- **The placement generator.** The coarse jobs share a second level-0 generator
  with the World's seed, parameters, features and Templates. It answers tree,
  Structure and Settlement draws only, never a Chunk. It shares its 1 m column
  cache with level 1's source, and that pair gets its own caches, sized for
  level 1's ring, instead of level 0's, which nothing coarse then touches.
- **Level 2 measured first.** Its children are 2 m, so its draws find no warm
  1 m columns. If a level-2 Chunk's draws cost past the per-level budget, level
  2 takes Canopy cover (05) instead, recorded in the spec.
- **Impostors.** Each tree level 0 stands in a coarse Chunk's footprint, fully
  grown, is written into the coarse grid so that any trunk or canopy Voxel
  beats air; canopy wins over trunk, either over ground. Level 1 always runs
  the draw, whatever the Server holds.
- **Saved changes.** A branch holding an Edit or saved change runs no draw: its
  changed level-0 Voxels are coarsened with the same any-solid rule for tree
  Materials, so a felled tree stays felled and a standing one keeps its shape.

## Acceptance criteria

- [ ] The placement generator's tree draws equal level 0's for the same region.
- [ ] A level-1 Chunk built from held children equals one built from generated
      children, Edits aside.
- [ ] A tree's Impostor occupies the coarse Voxels its level-0 trunk and canopy
      reach.
- [ ] A felled tree whose Chunk is evicted is absent at level 1 in a persistent
      World.
- [ ] Coarse-Chunk CPU time per level from an interleaved micro-benchmark
      against the integration branch's tip, in Comments, with
      vertex bytes per level from a test, and level 0's column-cache hit rate
      unchanged.
- [ ] The level-2 decision, Impostors or Canopy cover, with its numbers.
- [ ] Captures of a forest edge across the level 0 to 1 and 1 to 2 joins, for
      the review batch after 05.
