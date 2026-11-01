# 15: Check the targets

Status: done
Blocked by: 02, 03, 04, 05, 06, 07, 08, 09, 10, 11, 12, 13, 14

Spec: [streaming performance](../spec.md).
PR boundary: a measurement note on its own branch `tech/streaming-targets`.

## What to build

Once every other ticket is done or `wontfix`, run every fixture against `main`
under frame-profiling.md and compare with the spec's Targets and with 01's
baseline.

## Acceptance criteria

- [ ] Each target recorded here as met or missed, with p50, p95, p99 and max.
- [ ] For each missed target, what dominates it, from 01's attribution, and a
      proposed next ticket for you to accept or decline.

## Comments

### 2026-11-01: every target met

Series `/tmp/claude-streaming-impl/t15all`: release `profiling` builds,
offscreen, under the frame-profiling.md runner settings. The tip is the
integration branch at `df93f3b` (02 to 16 in, 09 reverted, 10 removed, 12's cull
path removed). Two tip runs per fixture; one interleaved pair against 01's
binary `86d1def`, which the final series backs with two more. Cut from three
rounds at your request. Milliseconds, medians over the tip's runs.

`main` itself (`ba97761`) cannot be the baseline: its `profiling` release build
panics at startup on a missing resource, the gating bug `a95e295` and `c9ba412`
fix here. 01's binary is `main` plus the profiler columns and fixtures, so it
stands in.

| Target                         | Fixture         | p50  | p95  | p99  | max   | Result |
| ------------------------------ | --------------- | ---- | ---- | ---- | ----- | ------ |
| p95 <= 8.3 ms                  | `stationary`    | 4.61 | 6.24 | 8.10 | 23.9  | met    |
| p95 <= 8.3 ms                  | `flight`        | 4.85 | 6.03 | 7.04 | 28.8  | met    |
| p95 <= 16.7 ms, none > 33 ms   | `sprint-flight` | 4.62 | 5.81 | 7.81 | 15.8  | met    |
| p95 <= 16.7 ms, none > 33 ms   | `sprint-walk`   | 4.57 | 5.54 | 6.25 | 25.9  | met    |
| zero fill-hole frames          | `dive`          | 3.33 | 4.88 | 5.89 | 24.4  | met    |
| stretch: 6.9 ms (144 Hz)       | `stationary`    | 4.61 | 6.24 | 8.10 | 23.9  | met    |

- Zero rule-hole frames in every fixture at the tip (01: `teleport` 520). Zero
  fill-hole frames in `dive` and both sprints. No sprint frame over 33 ms (01:
  one in each).
- Time from `teleport` to `streaming_settled`: 0.78-0.80 s (01: 1.01).
- Not targets, recorded: `restart` has 284 fill-hole frames over its reopen and
  two frames over 33 ms; `revisit` 13 frames over 16.7 ms; `cave` p95 5.50.
- One `transparency` run had a 296 ms frame at 30.4 s: main-thread wall 294 ms
  against 1.4 ms CPU, the Server's longest tick 175 ms, so the main thread
  waited rather than worked. The other run's worst frame was 40 ms. Unexplained;
  worth a ticket only if it recurs.

Against 01's baseline (one pair):

| Fixture         | 01 p50 | 01 p95 | 01 max | tip p50 | tip p95 | tip max |
| --------------- | ------ | ------ | ------ | ------- | ------- | ------- |
| `stationary`    | 7.80   | 9.72   | 15.3   | 4.61    | 6.24    | 23.9    |
| `active`        | 7.81   | 9.55   | 10.8   | 4.77    | 6.05    | 12.9    |
| `transparency`  | 9.17   | 11.81  | 19.0   | 5.37    | 6.88    | 295.9   |
| `flight`        | 8.73   | 11.45  | 31.9   | 4.85    | 6.03    | 28.8    |
| `sprint-flight` | 8.38   | 12.53  | 40.3   | 4.62    | 5.81    | 15.8    |
| `sprint-walk`   | 8.09   | 10.03  | 55.7   | 4.57    | 5.54    | 25.9    |
| `dive`          | 7.13   | 10.67  | 35.3   | 3.33    | 4.88    | 24.4    |
| `teleport`      | 8.67   | 10.90  | 40.4   | 4.64    | 5.85    | 31.3    |

- No target is missed, so no next ticket is proposed. Still owed under 01: the
  presented run on the 144 Hz display.
