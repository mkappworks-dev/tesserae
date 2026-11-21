# 01: Baseline: agreement and budgets on main

Status: ready-for-agent
Blocked by: none

Spec: [LOD features](../spec.md).

## What to build

Measure what the coarse levels get wrong, and what they cost, before anything
changes.

- **Agreement test.** On Terra's default seed, over windows holding coast and
  forest, for each coarse level: the share of coarse columns whose water or
  land, and whose top Material, differ from level 0's at the column's centre in
  metres, and whether level 0 stands trees there. It records today's rates and
  asserts nothing yet; 02 makes it assert.
- **Budget series on `origin/main`**, run once in a quiet window after the
  other tickets stop building (it may land after them, but before 09),, at Terra's declared 160 m and 3000 m, with the packed-quads method (spec,
  Testing Decisions): per level, Chunks held, non-empty and meshed, quads and
  vertex bytes; total coarse vertex memory; time for each level and the whole
  field to fill from spawn; frame timing and a `--instruments gpu` series at
  `flight` and `stationary`.
- **Shadow comparison** in the same GPU series: shadows on, the `no-shadows`
  switch, and a build whose far cascades hold nothing (render shadows ticket
  07), run in mirrored order.

## Acceptance criteria

- [ ] The agreement test runs in CI and prints per-level rates; the rates are in
      Comments.
- [ ] Budget and shadow numbers in Comments, with the load average and the
      commit measured.
- [ ] Captures from fixed Eyes at 0.5, 1, 2 and 4 km on Terra, in one `/tmp`
      folder, the "before" of the first review batch.
