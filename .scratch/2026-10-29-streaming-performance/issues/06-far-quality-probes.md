# 06: Quality probes for cheaper far levels

Status: ready-for-human
Blocked by: 04

Spec: [streaming performance](../spec.md).
PR boundary: probe branch `tech/streaming-far-probes`; nothing merges without
your approval of captures.

## What to probe

Two ways to make far levels cheaper that change what is drawn:

- **Direct generation**: a level-k Chunk sampled from the level-k generator at
  its own resolution instead of downsampling eight generated children. About
  8 times fewer samples; caves and overhangs survive, the majority vote does
  not.
- **Column-only**: from some level up, a height and surface Material per
  column, with no caves or overhangs.

For each: worker CPU per coarse Chunk against 04's result, and side-by-side
review captures at 256 m, 1 km and 2.5 km on the default seed (the demo's
LOD reach is 3000 m).

## Acceptance criteria

- [ ] Worker CPU per coarse Chunk and review captures for both probes recorded
      here.
- [ ] Your decision recorded under Comments. A probe never counts as an
      equal-quality improvement (frame-profiling.md).

## Comments
