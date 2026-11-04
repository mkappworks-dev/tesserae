# 04: The Mod drain on a batch

Status: ready-for-agent
Blocked by: 03

Spec: [bulk relight](../spec.md), decisions 3, 4 and 5.

## What to build

`PendingCommands::drain` in `server/mods/runtime.rs` writes a tick's Voxels into
one `edit::Batch` and finishes it once.

- `edit::apply_intent` takes the batch.
- The round-robin, the unlit-Chunk skip and queue order stay as they are.
- `MOD_VOXELS_PER_TICK` counts writes that change the World. A skipped no-op
  counts as applied in the report and against a separate, larger bound on
  Voxels read a tick, so a tick's work stays bounded by operation counts.
- Both numbers are set by a sweep with 01's lumen harness: the largest budget
  whose worst drain tick stays within what phase 7 accepted for the drain beside
  streaming (the cap of 2 measured 0.3 ms; streaming's worst tick is 0.7 ms),
  checked with an interleaved A/B against the base, CPU time.

## Acceptance criteria

- [ ] Every existing Mod runtime test passes, with counts that depended on two
      Voxels a tick updated to the new budget and nothing else.
- [ ] The fixture fill test still ends with light equal to a fresh solve, and
      its mesh redelivered.
- [ ] A Mod's fill of air over air drains at the read bound, not the write
      bound, and reports every Voxel applied.
- [ ] lumen at 0.5 m drains in under 10 s of ticks in 01's harness; the sweep,
      the chosen budgets, worst and mean drain CPU and the before/after figures
      are in Comments.

## Comments
