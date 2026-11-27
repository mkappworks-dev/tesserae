# 09: Terra's day, reviewed whole

Status: done
Blocked by: 03, 05, 06, 07, 08

Spec: [Day and night cycle](../spec.md).

## What to build

The whole day on Terra, judged together once every part has landed, and the
records brought up to date.

- Captures at every baseline Eye at noon, dusk, full-moon and new-moon
  midnight, and one full day watched at speed, reviewed by the user; any last
  Day curve or cloud tuning lands here.
- The sky and the Cloud layer together at most 0.3 ms GPU frame time over
  `main` on `flight`, at noon and midnight.
- DESIGN.md's step 9 note records the cycle as done, with the measurement.

## Acceptance

- [x] The user approved the full day.
- [x] Frame time within budget, recorded in Comments.
- [x] DESIGN.md updated; the spec's Status set to done.

## Comments

2026-11-26, frame time against `main` (b6817a1b), the branch at 591ac51a:
1920×1080, release with `profiling`, Terra, two rounds each way under a load of
6 to 7; medians of the per-run p50, 6 runs a cell; midnight is a new moon.

| GPU frame, ms   | `main` | Noon  | Midnight |
| --------------- | ------ | ----- | -------- |
| `flight`        | 4.663  | 4.610 | 4.497    |
| `stationary`    | 3.952  | 4.035 | 3.962    |

The sky and the Cloud layer together are within noise of `main`, at most
+0.08 ms, under the 0.3 ms. Frame time without the GPU instrument, `main`
against noon: `flight` 4.488 and 4.600 ms (p95 9.83 and 9.99), `stationary`
3.815 and 3.937 ms (p95 7.55 and 8.08).

The first A/B, before 45c1e246 and 591ac51a, had the branch 1.2 to 1.3 ms over
`main`; tickets 06 and 07 record the two fixes.

2026-11-26: captures from the far Eye at 06:00, 07:00, 09:00, 12:00, 15:00,
17:00, 18:00, 18:30 and at full-moon midnight, new-moon midnight and new-moon
03:00, and from the near Eye at noon, 18:00 and both midnights. No tuning was
needed. The user approved the full day on 2026-11-26.
