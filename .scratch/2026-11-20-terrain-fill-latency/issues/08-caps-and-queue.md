# 08: Level 0 meets its targets, or the reason is recorded

Status: ready-for-agent

Blocked by: 02, 03, 04, 05, 06, 07, and the sister spec's 08

Spec: [Terrain fill latency](../spec.md).

## What to build

First the measure, after every other ticket of both specs: `flight`, `sprint-flight` and `teleport` against 01's figures. The targets: at 12 m/s level 0 is never missing within 96 m of the Eye; at 64 m/s the median is at least 64 m; a teleport's level 0 settles in under 1 s. If level 0 misses them while the workers have spare time, a sweep of `TESSERAE_BUDGET`, `TESSERAE_LOD_BUDGET` and `TESSERAE_SETTLE_GATE` picks new defaults, recorded in the doc comments of `StreamingBudget` and `LodBudget`. If no setting meets them, the pool's jobs are taken from a priority queue the tick re-scores when the anchor moves. If the workers are saturated, generation binds: closed with the figures, and LOD level 2 from direct columns is put to a human.

## Acceptance criteria

- [ ] The end figures against 01's, and the verdict, in Comments.
- [ ] If the defaults move: the sweep's table, three interleaved rounds per setting, the worst tick no worse beyond noise.
- [ ] If a priority queue is built: the existing streaming tests (`hold`, `scramble`, the in-flight caps) pass on it, and the figures show it beat the best sweep.
