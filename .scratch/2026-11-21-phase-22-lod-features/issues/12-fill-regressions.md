# 12: Fill-time regressions from 04 and 08

Status: ready-for-agent
Blocked by: 04, 08

Spec: [LOD features](../spec.md).

## What to build

Ticket 11's captures showed holes at 10 s. A diagnosis (captures in
`/tmp/lod22-12-holes-hbyv`) found unfinished fill, not lost Chunks, from two
commits:

- **08 made Medium the default** before 09's budget gate. Medium holds 2.1
  times the Chunks of Terra's own rings at levels 1 to 3; the whole field fills
  in about 16 s against 9.5 s. Until 09 passes the budgets, the default preset
  is Low.
- **04 stands Structures and paths at every coarse level.** Level 5's Chunks
  went from 0.5 to 10 ms, the whole field fills 2.3 to 2.7 s later on Low, and
  level 0's fill went from 1.2 to 2.2 s, which the spec forbids. At 16 and 32 m
  a building is under one Voxel. Structures and paths stand at levels 1 to 3
  only, as bridges stand at level 1 only; level 4 and 5 columns keep the agreed
  ground.

If level 0's fill still regresses afterwards, find why coarse jobs delay it.

## Acceptance criteria

- [ ] The default preset is Low; a profile without a saved choice reads as Low.
- [ ] No Structure or path above level 3; tests say so.
- [ ] On Low, from the diagnosis's Eyes: level 0 filled in about 1.2 s and the
      whole field in about 9.5 s, as at `ea48f268`, in Comments with the load
      average.
