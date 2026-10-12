# Does the streaming budget still hold

Status: done
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

**Method.** One binary, A/B at run time: a `dev`-only `TESSERAE_CAVES=0` turns
the caves off (`server::worldgen_params`), so there is no build difference to
explain. `scripts/frame-runs 5`, interleaved caves-off, caves-on, caves-off,
caves-on: ten runs each, Apple M5, debug at `opt-level = 1`, 3 async compute
threads, medians with the range in brackets.

| Figure                    | caves off          | caves on           |
| ------------------------- | ------------------ | ------------------ |
| worst tick (ms)           | 0.6 [0.6-0.6]      | 0.6 [0.5-0.7]      |
| worst cost, fill (ms)     | 0.6 [0.5-0.6]      | 0.7 [0.6-0.7]      |
| worst cost, flight (ms)   | 0.4 [0.4-0.6]      | 0.5 [0.4-0.6]      |
| mean mesh upload (us)     | 8 [7-10]           | 8 [7-9]            |
| shown (s)                 | 1.57 [1.31-1.61]   | 1.57 [1.29-1.59]   |
| filled (s)                | 1.58 [1.54-1.94]   | 1.58 [1.56-1.98]   |
| worst frame, fill (ms)    | 17.5 [16.5-19.0]   | 17.9 [16.6-20.5]   |
| worst frame, flight (ms)  | 19.6 [18.5-27.5]   | 19.4 [18.4-34.9]   |
| relight landed / stale    | 10700 / 83         | 11033 / 75         |
| mesh landed / stale       | 1142 / 50          | 1114 / 23          |

Chunks loaded (2442) and meshed (910) are identical in every run.

**Nothing moved, and the reason is arithmetic, not luck.** The ring's vertical
extent reaches one deep layer inside the cave band (Chunk y = -1, 32 m to 1 m
below the origin) and the Chunks under it sit below `floor_m`. About three
quarters of that layer, a few hundred Chunks, carry a tunnel, at roughly 0.7 ms
across the three stages: on the order of 100-200 ms of worker time, spread over
a 1.6 s fill and three workers. The surface layers cost more to generate (104
-> 252 us when a tunnel crosses) but are a minority of them.

- **Client upload**: 8 us a mesh with caves, the same as without. The mesh
  landing cap of 32 stays at 0.3 ms of tick. This answers the ticket's
  "re-measure the cap" and the spec's remaining loose end.
- **Staleness window**, from the release benches, if every job were a Chunk a
  tunnel crosses: 64 x 252 + 128 x 140 + 128 x 395 us = 85 ms of work, 28 ms
  over three workers (it was 68 ms and 23 ms). About 1.25x longer, nowhere near
  the 10x that the off-thread spec named as the point to switch to a
  worker-time window.
- **Defaults unchanged.** `StreamingBudget`'s doc comment now carries the cave
  numbers and the A/B; DESIGN.md's open item says caves did not move the
  budget and names what would: LOD rings, or a Mod setting a cave floor deep
  enough to put several layers in the band.
- **Not measured**: a deep floor. By arithmetic, all 2442 loaded Chunks carved
  at 0.7 ms is 1.7 s of worker time, about 0.6 s on three workers, so a worst
  case fill near 2.2 s. That is a bound, not a run.
