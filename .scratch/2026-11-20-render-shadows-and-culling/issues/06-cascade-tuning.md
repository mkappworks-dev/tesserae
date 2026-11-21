# 06: Cascade tuning (1b)

Status: done
Blocked by: 03, 04, 05

Spec: [Shadows and culling](../spec.md).

## What to build

Bevy's defaults: four cascades with far bounds 10, 25, 61 and 150 m, overlap
0.2, a 2048 map, Gaussian filtering. Each cascade is a render pass, a cull and
two parameter builds; the overlap band samples two cascades in the main pass.

- Measure a few candidates by the GPU series alone: three cascades, a 1536 or
  1024 map, a shorter far bound, overlap 0. Choose against today's 192 m view
  distance; step 22 re-checks it when it raises the default.
- Captures of each candidate shown to you; you pick, or keep the defaults.
- The chosen config is set on the sun with one line of why; ticket 03's far
  cascades are revisited if the count or bounds change.

## Acceptance criteria

- [x] Candidates' GPU numbers and captures in Comments.
- [x] Your choice gated by 01's method; numbers in Comments.

## Comments

Candidates (2026-11-21), each one change against the defaults at `b4f2ecbc`,
GPU series in mirrored order (defaults, c3, m1024, far100, ov0, then back),
three fixtures, logging-on runs only, load 4.5 to 6. Mean of six runs, ms. A
1536 map is out: Bevy wants a power of two.

| candidate | GPU frame flight / transparency / stationary | frame p50 | pixels changed, stationary / flight |
| --- | --- | --- | --- |
| defaults: 10, 25, 61, 150 m, 2048, overlap 0.2 | 3.58 / 4.01 / 2.98 | 3.57 / 3.96 / 3.04 | 58 / 91 between two runs |
| c3: 10, 61, 150 m | 3.47 / 3.47 / 2.79 | 3.47 / 3.56 / 3.03 | 1 / 12,800 |
| m1024: 1024 map | 3.47 / 3.68 / 2.93 | 3.49 / 3.87 / 3.11 | 220,000 / 478,000 |
| far100: 10, 25, 61, 100 m | 3.64 / 3.58 / 2.88 | 3.56 / 3.65 / 3.03 | 157,000 / 95,000 |
| ov0: overlap 0 | 3.58 / 3.56 / 2.93 | 3.48 / 3.57 / 3.04 | 34,000 / 132,000 |

`transparency` was noisy throughout (the defaults ranged 3.57 to 4.26 ms).
m1024 softens near shadows (an eave's on its wall goes), far100 drops tree
shadows past 100 m, and ov0 ends leaf shadows in a hard edge at 61 m instead
of the overlap's fade. c3 saves 0.19 ms of GPU frame in `stationary`, beyond
run spread (2.77 to 2.80 against 2.96 to 3.01), and 0.11 in `flight`, inside
it; frame p50 no worse. Chosen: c3, `SUN_CASCADE_BOUNDS` on the sun, cutout out
of cascade 2 so leaves still reach 61 m (03). Cascade draws in `stationary`: 2,
32 and 31 against 2, 4, 32 and 31.
