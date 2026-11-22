# 09: The default curve, fog and Terra's ceiling

Status: done
Blocked by: 03, 04, 05, 06, 07, 11, 12

Spec: [LOD features](../spec.md).

## What to build

The default reaches well past today's: Medium, `u` 256 m (8 Chunks) to 6 km, with Terra's ceiling
raised from 3000 m and the engine's default from 4096 m to 8192 m so High (`u` 384 m to 8 km) can be
chosen, and fog lightened
so the far field reads. It becomes the default only when, on the M5 with Terra
at `stationary` and `sprint-flight`, with Impostors and Canopy cover in:

- coarse vertex memory on the GPU is at most 192 MB;
- the whole field is drawn within 10 s of spawn;
- GPU frame is at most 1 ms over `main`;
- level 0's fill time does not regress.

Otherwise the default is the widest curve that passes, recorded in the spec. High is measured too and
may exceed them, as a Player's choice; its numbers go in Comments.
No shadow series: 07's test keeps coarse meshes off the shadow casters.
Terra's level 0 is measured at 192 m (6 Chunks, the engine default every other Game uses) beside
its 160 m: Terra moves to 192 m only if level 0's fill time and GPU frame stay within the budgets
above; either way the reason is recorded in the spec, since 160 m has none on record.
DESIGN.md's LOD section and
step 22 note are updated.

## Acceptance criteria

- [x] Budget series against 01's, in Comments: the only full series
      after 01's.
- [x] Captures from fixed Eyes at 0.5, 1, 2, 4 and 8 km beside your reference
      shots, shown to you; the fog's value chosen on them. (Judged by the
      implementer against 01's, as asked, not shown.)
- [x] The chosen default, and why, in the spec.
- [x] Terra's level 0 at 160 m and 192 m in the budget series; the kept value, and why, in the spec.

## Comments

2026-11-22: Done on `t3/lod-default-curve`.

**Low keeps its reach.** `view-params` gains `low-lod-distance-m:
option<f32>`, Low's reach where it is nearer than `lod-distance-m`, now the
ceiling; absent, Low reaches the ceiling. Terra declares 160 m, Low 3000 m and
an 8192 m ceiling; a Game with no view gets 192 m, Low 4096 m (still
`StreamingSettings::default()`) and 8192 m (`DeclaredStreaming::default()`).
`QualityCurve::low` reads `DeclaredStreaming::low_reach_m`. The Server streams
Low until a Client asks; the LOD generators, the camera's layers and far plane
are sized from the ceiling, and the fog follows the streamed reach.
`profile-frames --quality` passes `TESSERAE_QUALITY`.

**Fill time.** Levels 1 to 4 filling together was the shared cap: dispatch runs
once a 64 Hz tick with 16 generate jobs in flight, about 1,000 Chunks a second,
while main's coarse jobs were 6.8 s of CPU on three workers against an 8.7 s
fill. `LodBudget::idle_scale` (8) multiplies every cap in a tick that starts
with no level-0 job in flight. Dev build, Terra's spawn, same period, two runs
each: Low 10.2 s with scale 1, 5.8 s with 8, level 0 1.26 to 1.29 s in both;
scale 16 took the worst tick from 6 to 10.5 ms. Above that the coarse field is
CPU-bound: six async workers ran each job 1.5 to 2 times slower (E-cores) and
moved level 0 from 1.66 to 1.73-1.84 s, so the pool stays at three. Halving the
busy cap to 8 did not move level 0. Test:
`server::lod::tests::the_caps_scale_only_once_level_zero_has_nothing_in_flight`.

**Budget series.** `7bc4eb5f` (this branch before Medium moved to 6 Chunks; the
curves are named with `--quality`), release `profiling`, Terra 160 m from
`scripts/build-guests`, 01's commands plus `--quality`, offscreen 1920 x 1080,
three repeats per run, median `[min-max]` of the logging-on runs. Per the scope
cut, one run per curve, a second only near a budget line (6 Chunks), and the
GPU series at `stationary` only. Every run started at a 1-minute load under 3
(2.78 to 2.98; 5.8 to 7.4 at the end, the engine's own); nothing else ran.
`main` is 01's six runs.

| curve, fixture                  | n   | level 0 s        | whole field s       | frame p50 | coarse held / delivered MB |
| ------------------------------- | --- | ---------------- | ------------------- | --------- | -------------------------- |
| main, stationary                | 6   | 1.51 [1.31-1.63] | 8.71 [8.44-8.97]    | 2.73      | 1.62 / 3.19                |
| main, sprint-flight             | 6   | 1.56 [1.45-1.64] | 8.74 [8.67-8.86]    | 3.13      | 1.68 / 3.29                |
| Low, stationary                 | 3   | 1.74 [1.60-2.69] | 7.31 [7.13-8.06]    | 2.86      | 2.40 / 4.74                |
| Low, sprint-flight              | 3   | 1.64 [1.57-1.77] | 7.17 [7.02-7.56]    | 3.87      | 2.39 / 4.71                |
| 8 Chunks to 6 km, stationary    | 3   | 1.72 [1.69-1.75] | 13.47 [12.25-13.88] | 3.13      | 5.41 / 10.68               |
| 8 Chunks to 6 km, sprint-flight | 3   | 1.70 [1.66-1.78] | 13.75 [13.61-14.32] | 4.03      | 6.40 / 12.63               |
| 7 Chunks to 6 km, stationary    | 3   | 1.61 [1.50-1.63] | 11.38 [10.28-11.58] | 2.96      | 4.45 / 8.79                |
| 7 Chunks to 6 km, sprint-flight | 3   | 1.63 [1.56-1.69] | 11.44 [11.28-11.61] | 4.01      | 5.00 / 9.85                |
| Medium, stationary              | 6   | 1.62 [1.46-1.72] | 9.60 [8.56-9.83]    | 3.04      | 3.60 / 7.11                |
| Medium, sprint-flight           | 6   | 1.69 [1.56-1.83] | 9.89 [9.41-10.25]   | 3.97      | 3.87 / 7.63                |
| High, stationary                | 3   | 1.60 [1.47-1.69] | 24.34 [22.27-24.50] | 3.58      | 10.68 / 21.06              |
| High, sprint-flight             | 3   | 1.53 [1.52-1.69] | not by 20 s         | 4.20      | 12.30 / 24.27              |
| Terra 192 m, Medium, stationary | 3   | 2.08 [1.93-2.10] | 10.39 [9.20-10.56]  | 2.94      | 3.50 / 6.90                |
| Terra 192 m, Medium, flight     | 3   | 2.17 [2.07-2.23] | 10.66 [10.58-10.83] | 4.01      | 3.85 / 7.59                |

Medium is 6 Chunks (`u` 192 m) to 6 km. Low holds exactly `main`'s Chunks at
every level (01's table). Medium holds 2167 at levels 1 to 5 at `stationary`
(meshed 773 to 776) and 2354 to 2464 in flight.

GPU frame, `--instruments gpu`, `stationary`, ms:

| curve  | n   | GPU p50          | GPU p95          |
| ------ | --- | ---------------- | ---------------- |
| main   | 6   | 2.70 [2.67-2.85] | 2.97 [2.79-3.27] |
| Low    | 3   | 3.14 [3.05-3.14] | 3.37 [3.18-3.40] |
| Medium | 3   | 3.17 [3.16-3.19] | 3.30 [3.29-3.30] |
| High   | 3   | 3.55 [3.52-3.57] | 3.72 [3.69-3.74] |

Against the budgets:

- Coarse vertex memory on the GPU: 3.9 MB for Medium, 12.3 MB for High, against
  192 MB.
- Whole field within 10 s: Low 7.3 s; Medium 9.6 s and 9.9 s, the only curve
  wider than Low that passes (one flight run 10.25 s); 7 and 8 Chunks and High
  fail.
- GPU frame at most 1 ms over `main`: Low +0.44, Medium +0.47, High +0.85 ms.
  Low pays the same as Medium, so it is step 22's Impostors and cover, not the
  curve. Frame p50 in `sprint-flight` is 0.7 to 0.9 ms over `main` at every
  curve, for the same reason.
- Level 0's fill: in the series 1.6 to 1.7 s against `main`'s 1.51 and 1.56,
  inside `main`'s own range at `stationary` and 0.1 s over it in flight. An
  interleaved A/B on the dev build at the spawn Eye (`main`'s binary against
  this branch, Low, two runs each, 18:14-18:19, load 2.9 to 3.5) put `main` at
  1.55 and 1.38 s and this branch at 1.49 and 1.53: no regression.
- Terra at 192 m: level 0 0.47 s later at `stationary` and 0.48 s in flight,
  the whole field over 10 s. Terra keeps 160 m.

**Fog.** The fog now derives its density from the streamed reach, so under
Medium it is lighter at every distance than on `main` (3 km: 53% against 88%).
On the captures 0.4, 0.6 and 0.88 at the reach showed its square edge as a step
on the sea horizon from the 8 km Eye; 0.95 blends it into the sky and the far
shore still reads at 2 km. `reach_opacity` is 0.95.

**Captures** (not committed): `/tmp/lod22-09-Mdma/captures/`. Release `dev`
build, Terra 160 m, Medium (`TESSERAE_QUALITY=192,6000`), offscreen 1920 x
1080, settle 30 s, yaw 0, 01's Eyes plus `at-8km` (1760.5, 660.5, 7264.5,
pitch -4). `medium-fog0.95/` is the chosen set (all five Eyes);
`medium-fog{0.4,0.6,0.88}/` hold the 2 and 8 km trials; `main/at-8km.png` is
`main` from 01's binary. 01's "before" shots are in
`/tmp/lod22-01-baseline-yovc/captures/`. Judged against them:

- Distant terrain is present to the reach at every Eye: islands, forest
  Impostors and Canopy cover, Settlements at 0.5 and 1 km, where 01 shows bare
  ground fading into haze by 2 to 3 km and, from 8 km, a square cut at 3 km.
- No holes at any Eye after 30 s, and no geometry seam at a join.
- One look seam: at 1 km the level-1 Impostor forest is darker and more olive
  than level 2's Canopy cover beside it, a visible tone change at the join.
  Ticket 10's ground tint or a cover colour matched to the Impostors' leaves is
  where it would go.
- The fog reads as haze toward the horizon, with no edge at the reach.

**Open.**

- Medium's margin in flight is thin (median 9.89 s, one run 10.25 s). Faster
  coarse jobs are the lever: level 1 costs about 5 ms a Chunk with Impostors
  against 1.6 on `main`, half of Medium's CPU.
- The level-1 / level-2 colour seam above.
- Frame p50 0.7 to 0.9 ms over `main` in flight at every curve.

Ran: `cargo test --locked -p tesserae-mods` (313 passed), `cargo test --locked
-p tesserae --features dev --no-fail-fast` (1,734 passed, 8 ignored), `cargo
clippy --locked -p tesserae -p tesserae-mods --all-targets --all-features`,
`cargo fmt --all`, `npm run fmt`. An earlier full workspace run on this branch
passed except one Terra declaration test, since fixed. Runs, logs and
`summarize09.py`: `/tmp/lod22-09-Mdma`.
