# 11: Canopy cover with relief

Status: ready-for-agent
Blocked by: 05

Spec: [LOD features](../spec.md).
[ADR-0035](../../../docs/adr/0035-a-coarse-level-draws-level-0s-answer.md).

## What to build

In the first capture review, 05's Canopy cover read as a flat green sheet laid
over the terrain's steps, and the level 1 to 2 join changed abruptly from single
trees to that sheet. Break the sheet up so a forest reads as a mass of trees:
denser columns stand one coarse Voxel higher than sparser ones, and each
column's leaf shade varies a little with its cover, both from the same cover
value 05 already computes at the column's centre. Nothing is stored, and it
stays a pure function of seed and metres.

## Acceptance criteria

- [ ] A dense forest's top has more than one height at levels 2 to 4; a column
      with the same cover always gets the same height and shade.
- [ ] 05's exclusions and its agreement bound still hold.
- [ ] Coarse-Chunk CPU time per level from an interleaved micro-benchmark
      against the integration branch's tip, in Comments: under 0.1 ms per
      Chunk added.
- [ ] Before and after captures at the level 1 to 2 and 2 to 3 joins and at
      2 km, shown to you.
