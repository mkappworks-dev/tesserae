# 13: Cut level-0 job cost

Status: ready-for-agent
Blocked by: 01

Spec: [streaming performance](../spec.md).
PR boundary: own branch `tech/streaming-level-zero-jobs`.

## What to build

Only if 01's checkpoint shows level-0 jobs taking more than 50% of worker time
in a sprint fixture; otherwise close as `wontfix` with the numbers.

`StreamingBudget`'s notes give 252 us to generate, 140 us to light and 395 us to
mesh a surface Chunk a tunnel crosses, measured in a debug build at
`opt-level = 1` before slope smoothing; 01's release worker columns replace
them. Profile the stage that dominates in the sprint, mesh and light first,
and the `wait_for_settled_light` trade (a Chunk shows 0.2 to 0.4 s sooner
without it, meshed 2.2 times as often).

## Acceptance criteria

- [ ] The trigger evaluated and recorded here.
- [ ] If built: identical output, asserted, and a per-Chunk improvement that
      passes the spec's effect gate in the core comparison methodology.

## Comments
