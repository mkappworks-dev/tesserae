# Does the streaming budget still hold

Status: ready-for-agent
Blocked by: 02, 03

Spec decision 6, and the off-thread spec's "What this assumes about per-Chunk
cost".

## What to build

- Run `scripts/frame-runs 5` on `main` and on this branch, interleaved. Read
  `worst_tick_ms` and `worst_cost_*`, not frame time.
- Re-measure the Client's cost per delivered mesh with cave meshes, because the
  mesh landing cap of 32 was sized at 6-12 us per upload.
- Recompute the staleness window `cap x mean Job cost / threads` with the cave
  mix of Job costs and compare it with the 23 ms the off-thread doc states.
- If a number moves, change it in `StreamingBudget` and rewrite its doc comment
  to carry the new measurement. If none does, say so there and leave the
  defaults alone.
- Update DESIGN.md's "Per-ring LOD chunk budget" open item.

## Acceptance criteria

- The tables are in Comments with machine and load.
- The doc comment contains no number whose reason is gone.
- `./scripts/cargo-ci` passes.

## Comments
