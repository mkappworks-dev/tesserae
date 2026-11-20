# 08: Level 0 meets its targets, or the reason is recorded

Status: done

Blocked by: 02, 03, 04, 05, 06, 07, and the sister spec's 08

Spec: [Terrain fill latency](../spec.md).

## What to build

First the measure, after every other ticket of both specs: `flight`, `sprint-flight` and `teleport` against 01's figures. The targets: at 12 m/s level 0 is never missing within 96 m of the Eye; at 64 m/s the median is at least 64 m; a teleport's level 0 settles in under 1 s. If level 0 misses them while the workers have spare time, a sweep of `TESSERAE_BUDGET`, `TESSERAE_LOD_BUDGET` and `TESSERAE_SETTLE_GATE` picks new defaults, recorded in the doc comments of `StreamingBudget` and `LodBudget`. If no setting meets them, the pool's jobs are taken from a priority queue the tick re-scores when the anchor moves. If the workers are saturated, generation binds: closed with the figures, and LOD level 2 from direct columns is put to a human.

## Acceptance criteria

- [x] The end figures against 01's, and the verdict, in Comments.
- [ ] ~~If the defaults move: the sweep's table, three interleaved rounds per setting, the worst tick no worse beyond noise.~~
- [ ] ~~If a priority queue is built: the existing streaming tests (`hold`, `scramble`, the in-flight caps) pass on it, and the figures show it beat the best sweep.~~

## Comments

### The measure, the sweep and the verdict, 2026-11-20

The targets hold with the defaults, so they stay. At e414dbef (both specs' other tickets), from [ticket 05's figures](./05-one-light-solve-per-chunk.md#figures-2026-11-20) and [ticket 01's](./01-lag-instrument.md#the-starting-figures-2026-11-20): at 12 m/s level 0 is never missing nearer than 127 m to the Eye; at 64 m/s never nearer than 125 m, so its median is past the ring's edge; after a teleport the ground is whole within 96 m in 0.94 to 0.97 s with the defaults below (1.07 to 1.18 s with the holes instrument on, which costs frame time). The full settle, every Chunk of the ring lit to its edge, takes 2.0 to 2.2 s; the target is read as the ground around the Eye, which is what the loading screen and a Player wait for.

The workers have spare time after a teleport (about 1.5 s of CPU a second on three), so the caps were swept. One `teleport --repeats 3 --instruments lag` series per setting, with an untracked copy of `scripts/profile-frames` that passes `TESSERAE_BUDGET`, `TESSERAE_LOD_BUDGET` and `TESSERAE_SETTLE_GATE` to the game (the runner drops every `TESSERAE_` variable), on e414dbef, 13:52 to 14:12, `uptime` load 3.8 to 6.1, nothing else running. Time from the jump to level 0 whole within 32 m and 96 m, and the settle, per run:

| Setting | Within 32 m | Within 96 m | Settle |
| --- | --- | --- | --- |
| Defaults (64/128/128 in flight; LOD 16/32) | 0.75, 0.72, 0.85 s | 0.95, 0.94, 0.97 s | 2.03, 2.01, 2.17 s |
| Generation 16 in flight | 0.98, 0.99, 1.13 s | 1.39, 1.50, 1.72 s | 2.84, 3.08, 3.56 s |
| 32/64/64 in flight | 0.87, 0.85, 0.90 s | 1.15, 1.11, 1.11 s | 2.77, 2.73, 2.77 s |
| LOD 4/8 | 0.73, 0.79, 0.84 s | 0.97, 1.00, 1.06 s | 2.03, 2.02, 2.03 s |
| Settled-light gate off | 0.50, 0.49, 0.47 s | 0.69, 0.73, 0.76 s | 2.22, 2.22, 2.22 s |
| Generation 16 and LOD 4/8 | 0.93, 0.97, 0.94 s | 1.39, 1.45, 1.40 s | 2.58, 2.56, 2.58 s |

Smaller caps only slow the refill, so neither the caps nor the queue's order bind, and no priority queue is built. Only the settled-light gate's absence is sooner, by about a quarter of a second; it builds meshes about twice as often and shows ground at its first light before it relights. The user chose to keep the gate and the defaults.
