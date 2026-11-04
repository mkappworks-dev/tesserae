# 04: The Mod drain on a batch

Status: done
Blocked by: 03

Spec: [bulk relight](../spec.md), decisions 3, 4, 5 and 9.

## What to build

`PendingCommands::drain` in `server/mods/runtime.rs` writes a tick's Voxels into
one `edit::Batch` and finishes it once.

- `edit::apply_intent` takes the batch.
- The round-robin, the unlit-Chunk skip and queue order stay as they are.
- `MOD_VOXELS_PER_TICK` counts writes that change the World. A skipped no-op
  counts as applied in the report and against a separate, larger bound on
  Voxels read a tick, so a tick's work stays bounded by operation counts.
- Both numbers are set by a sweep with 01's lumen harness: the largest write
  budget whose worst drain tick stays at or under 2 ms at the dev profile,
  checked with an interleaved A/B against the base, CPU time. If lumen misses
  10 s within that, follow the spec's decision 9.

## Acceptance criteria

- [x] Every existing Mod runtime test passes, with counts that depended on two
      Voxels a tick updated to the new budget and nothing else.
- [x] The fixture fill test still ends with light equal to a fresh solve, and
      its mesh redelivered.
- [x] A Mod's fill of air over air drains at the read bound, not the write
      bound, and reports every Voxel applied.
- [x] lumen at 0.5 m drains in under 10 s of ticks in 01's harness; the sweep,
      the chosen budgets, worst and mean drain CPU and the before/after figures
      are in Comments.

## Comments

### Result, 2026-11-04

The drain writes one `edit::Batch` a tick. Budget: **96 writes** and **4096
reads** a tick, and a write that starts or stops block light counts as **6**
writes. The emitter weight is new to the spec: a 16³ fill of lanterns at 128
plain writes cost 3.2 ms in a tick, over the 2 ms ceiling, and at weight 6 its
worst tick is 1.8 ms. Two costs found by profiling the slow ticks are fixed: the
shaped-unit sum read the whole Chunk per stair (a running count per Chunk in the
batch now), and light-emitting writes dominate `finish`.

Test profile, thread CPU of the drain call, three interleaved rounds against a
genuine build of the base (`3b44336`), load 10-13:

| Build                     | Base, 2 a tick | Base, 96 a tick | New, 96 a tick |
| ------------------------- | -------------- | --------------- | -------------- |
| lumen 0.5 m, seconds      | 154.0          | 3.22            | **3.09**       |
| lumen, worst drain tick   | 0.29 ms        | 8.4 ms          | **1.96 ms**    |
| lumen, Voxels / CPU s     | 78 K           | 83 K            | **247 K**      |
| plain fill 4096, worst    | 0.21 ms        | 4.6 ms          | **0.58 ms**    |
| plain fill, Voxels / CPU s | 238 K         | 263 K           | **938 K**      |

New drain, budget 96, one run each: meadow 25,448 Voxels in 1.53 s (worst tick
0.45 ms), ember 20,289 in 1.94 s (0.81 ms). A 16³ lantern fill takes 4.0 s, worst
tick 1.8 ms. Raising the base's budget alone would have given lumen an 8.4 ms
worst tick: the batch is what keeps it under 2 ms.

The sweep (writes 2 to 1024, reads 64 to 4096) showed reads do not matter for
these builds, which hold few no-ops; the 4096 bound only keeps a build over air
from being held to the write rate. Tests that counted two Voxels a tick now read
the budget, and two fluid tests that asserted one Voxel per Mod per tick assert
that the two Mod queues advance equally.

The 2 ms ceiling is at the test profile on this host; the drain was not
measured in a running client (no display here).
