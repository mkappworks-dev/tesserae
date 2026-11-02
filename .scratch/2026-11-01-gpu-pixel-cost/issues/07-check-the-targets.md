# 07: Check the targets

Status: done
Blocked by: 05, 06

Spec: [GPU cost per pixel](../spec.md).
PR boundary: a measurement note on its own branch `tech/gpu-targets`.

## What to build

Once 03 to 06 are done or `wontfix`, measure the spec's Targets at the tip of
the merged work, against 02's baselines and 01's presented run.

- Presented, by you: `stationary`, `flight`, `sprint-flight`, `sprint-walk`,
  `--instruments holes`, three rounds, at the chosen render scale and
  anti-aliasing, and once more at render scale 1.0. Command written here,
  output folder right after the script name.
- Offscreen 1920 x 1080 and 2560 x 1440, every fixture, `--instruments holes`.

## Acceptance criteria

- [x] Each target recorded as met or missed, with p50, p95, p99 and max.
- [x] For each missed target, what dominates it and a proposed next ticket
      for you to accept or decline.

## Presented after 03 (2026-11-02)

Your run of `bin-03` (03, render scale 1.0, MSAA 4x), 2560 x 1440 surface,
`Immediate` present, `--instruments holes`, three rounds, in
`/tmp/claude-gpu-pixel-cost/presented-03`. Before is 02's presented run of
`bin-02`: p50 about 9 ms, p95 14.3 to 15.9 ms.

| fixture       | p50       | p95       | p99        | max       | > 6.94 ms |
| ------------- | --------- | --------- | ---------- | --------- | --------- |
| stationary    | 4.68-4.81 | 5.95-7.60 | 6.38-9.25  | 7.1-22.7  | 0-5%      |
| flight        | 4.91-4.92 | 6.14-6.26 | 7.08-7.29  | 10.5-17.8 | 1%        |
| sprint-flight | 4.75-4.80 | 6.46-6.65 | 8.09-8.76  | 13.5-23.2 | 3%        |
| sprint-walk   | 4.82-5.05 | 6.09-8.87 | 6.75-10.83 | 11.5-22.4 | 0-14%     |

Zero rule-hole and fill-hole frames. Ten of twelve runs meet p95 at most
6.9 ms. The two that miss:

- `sprint-walk` round 3 was not focused for its whole window
  (`focused=false`): every second is slow, so the host, not the engine.
  Rejected as contended; it needs one more round.
- `stationary` round 1 is 5.8 ms at p95 in every second but 23 and 24 s,
  where 112 frames exceed 6.94 ms; rounds 2 and 3 have none. A transient in
  the series' first run, not reproduced.

Your rerun, `/tmp/claude-gpu-pixel-cost/presented-03b` (13:17 to 13:24, no
other engine run or build overlapping), does not reproduce those passing
runs:

| fixture     | round | p50  | p95  | p99   | > 6.94 ms | focused    |
| ----------- | ----- | ---- | ---- | ----- | --------- | ---------- |
| stationary  | 1     | 5.09 | 9.05 | 11.54 | 22%       | no         |
| stationary  | 2     | 5.32 | 8.95 | 10.49 | 22%       | no         |
| stationary  | 3     | 5.63 | 9.49 | 15.83 | 29%       | 235 frames |
| sprint-walk | 1     | 5.13 | 9.00 | 10.28 | 17%       | yes        |
| sprint-walk | 2     | 5.33 | 9.31 | 10.49 | 24%       | no         |
| sprint-walk | 3     | 5.28 | 9.44 | 11.21 | 26%       | yes        |

Zero rule-hole and fill-hole frames. The two fully focused runs still have
p95 9.0 and 9.4 ms, so focus alone does not explain it. Unproven: other
on-screen activity during the run (the agent session's window updating, focus
changes) shares the GPU with the compositor. The presented result is not
stable yet: when a presented run is asked for, the agent session stays idle
for its whole duration (no runs, builds or streaming output), the engine
window stays focused, and `focused` in `frames.csv` is checked before a run
is trusted.

## Final target check (2026-11-02)

Merged tip `ce567d9`, `bin-07` SHA-256 `a029a2ca82105ecd...`, release
`profiling`, SMAA, render scale 100%, holes on, GPU markers off. The chosen
scale is native, so one presented series serves both requested scales.

### Presented: target missed

Your completed `/tmp/claude-gpu-pixel-cost/presented-07` series, 19:06 to
19:19 local time, Apple M5, 2560 x 1440 surface. Backend logs show
`Immediate` after initial `Fifo` configuration in every run. This does not
measure display refresh or actual presentation elapsed time. No per-run
host-load, thermal or power observations were retained by this runner, so
host contention cannot be ruled out. No further engine run was launched
while waiting for your completion message.

Each row uses the whole 22 to 32 s fixture window; ms, never pooled:

| fixture | round | p50 | p95 | p99 | max | focused |
| ------- | ----- | --- | --- | --- | --- | ------- |
| stationary | 1 | 4.22 | 5.91 | 15.67 | 23.02 | no |
| stationary | 2 | 8.37 | 13.17 | 16.41 | 17.00 | yes |
| stationary | 3 | 8.37 | 13.24 | 16.09 | 20.84 | no |
| flight | 1 | 4.41 | 6.03 | 6.79 | 11.89 | yes |
| flight | 2 | 8.40 | 14.35 | 16.66 | 19.51 | yes |
| flight | 3 | 8.39 | 16.69 | 16.88 | 18.55 | yes |
| sprint-flight | 1 | 4.34 | 6.18 | 7.90 | 21.98 | yes |
| sprint-flight | 2 | 2.97 | 5.13 | 6.39 | 13.36 | no |
| sprint-flight | 3 | 8.43 | 15.84 | 16.77 | 19.67 | yes |
| sprint-walk | 1 | 4.47 | 6.09 | 6.68 | 15.57 | yes |
| sprint-walk | 2 | 8.39 | 15.90 | 16.78 | 17.04 | 21 lost |
| sprint-walk | 3 | 8.36 | 12.72 | 16.61 | 18.82 | yes |

The three focused round-1 movement runs pass p95 <= 6.9 ms. Later fully
focused runs fail: p50 is 8.36 to 8.43 ms and p95 12.72 to 16.69 ms.
Focus loss invalidates four runs for acceptance but does not explain the
five focused failures. Do not discard the slow rounds or claim a stable
144 Hz result from the first round. The older 02 presented baseline was
about 9 ms p50 and 14.3 to 15.9 ms p95; the new fast state improves it, but
the slow state remains in that range.

The added wall time is dominated by waiting in the render schedule:
focused slow runs have render-schedule p50 7.98 to 8.07 ms, versus 3.77 to
3.97 ms in the focused first round. Render-thread CPU p50 is only 1.30 to
1.34 ms and main-thread CPU p50 0.54 to 0.56 ms in the slow runs. GPU and
presentation elapsed are unavailable here. Presentation/compositor pacing,
GPU queue waits or host scheduling are candidates, not proven causes. The
8.4 ms / 16.7 ms clustering is suggestive of pacing, not proof of refresh.

Logging on/off pairs also cross these states (sprint-flight round 2 has
2.97 ms on versus 8.3 ms off at rounded p50). They cannot bound profiling
overhead in this series; the large variation prevents a small-effect claim.

### Offscreen: medians preserved, tail gate inconclusive

`/tmp/claude-gpu-pixel-cost/tgt07` and `tgt07b`, combined in
`tgt07-all.json`: 73 completed runs, three rounds of eight fixtures and
three arms, plus one duplicate sprint-walk 1440p run. The first attempt
stalled on the next run under host load; retained incomplete output is not
included in these statistics. The replacement series supplied the missing
15 runs. Retain the duplicate rather than choosing the better result.
Per-run host load was recorded, 3.2 to 10.4. Docker/bun activity was observed
during the series; tails cannot be treated as uncontended evidence.

Ranges of per-run percentiles in ms, including the duplicate:

| arm | fixture | p50 | p95 | p99 | max |
| --- | ------- | --- | --- | --- | --- |
| t1440 | stationary | 3.71-3.76 | 5.27-5.89 | 6.25-10.18 | 22.63-46.33 |
| t1440 | flight | 3.93-4.13 | 5.76-6.37 | 9.08-12.00 | 13.26-62.70 |
| t1440 | sprint-flight | 3.74-4.01 | 5.42-6.68 | 7.00-12.18 | 12.07-21.92 |
| t1440 | sprint-walk | 3.96-4.30 | 6.00-9.92 | 8.33-19.83 | 17.09-82.66 |
| t1440 | dive | 3.45-3.63 | 4.81-8.20 | 6.98-15.41 | 14.47-45.93 |
| t1440 | teleport | 3.92-4.15 | 5.72-6.44 | 7.66-10.57 | 12.71-35.21 |
| t1440 | transparency | 4.68-4.95 | 6.00-14.39 | 7.32-18.74 | 21.68-55.33 |
| t1440 | active | 3.82-5.03 | 5.26-14.11 | 6.72-24.06 | 36.22-47.61 |
| t1080 | stationary | 3.10-3.19 | 4.51-4.86 | 5.95-6.84 | 8.96-47.08 |
| t1080 | flight | 3.15-3.25 | 4.99-5.00 | 5.95-7.00 | 11.44-27.37 |
| t1080 | sprint-flight | 3.37-3.50 | 5.74-6.54 | 9.03-10.67 | 24.60-36.65 |
| t1080 | sprint-walk | 3.15-3.66 | 4.65-5.41 | 5.47-8.06 | 13.44-17.39 |
| t1080 | dive | 3.10-3.65 | 4.95-5.40 | 6.88-11.66 | 19.52-35.88 |
| t1080 | teleport | 3.28-3.57 | 4.89-8.37 | 5.95-11.49 | 9.13-24.36 |
| t1080 | transparency | 3.50-4.37 | 4.94-6.75 | 6.29-8.75 | 15.06-28.01 |
| t1080 | active | 3.23-3.77 | 4.62-6.67 | 5.21-11.01 | 11.02-82.28 |
| b02x1080 | stationary | 4.29-4.51 | 5.70-7.52 | 9.22-13.03 | 21.96-32.71 |
| b02x1080 | flight | 4.65-4.69 | 5.91-6.63 | 6.81-10.33 | 15.79-91.00 |
| b02x1080 | sprint-flight | 4.58-4.63 | 5.90-9.15 | 8.20-15.17 | 14.10-25.60 |
| b02x1080 | sprint-walk | 4.41-4.72 | 6.00-8.67 | 7.66-15.97 | 29.59-47.49 |
| b02x1080 | dive | 3.45-3.71 | 5.23-6.61 | 7.37-12.96 | 20.62-90.57 |
| b02x1080 | teleport | 4.51-4.68 | 5.94-10.50 | 7.15-17.01 | 18.42-34.76 |
| b02x1080 | transparency | 5.05-5.27 | 10.59-13.66 | 17.59-23.15 | 27.98-95.19 |
| b02x1080 | active | 4.68-4.73 | 6.15-8.37 | 8.14-17.44 | 20.97-51.12 |

`t1440` and `t1080` are the merged tip at native output size; `b02x1080`
is 02's baseline at 1080p. Every 1080p median range is below baseline
except dive, whose ranges overlap. No median regression is established.
The sprint-flight p99 range is 9.03 to 10.67 ms versus 8.20 to 15.17 ms
before; active also has a slower p99 in one round. With contention and
mixed tails, the no-consistent-p99-regression gate is inconclusive. A clean
matched repeat is needed before declaring the full 1080p target met.

### Holes: target met

Zero rule-hole and fill-hole frames in all 73 completed offscreen windows
and all 12 presented logging-on windows, including both sprints and dive.
All presented audit entries have complete windows, readiness before flight,
fixture readiness and simulation advancement. No `no ground` occurs in any
of the 98 native logs across these three folders (including off runs and
the incomplete attempt).

### Proposed follow-up, awaiting your decision

Investigate the two presented timing states before further quality cuts.
Record per-run host load, power/thermal state and actual display refresh;
add separate acquisition/submission/presentation wait observations where
available, with GPU attribution collected separately from timing. Compare
focused, unobstructed windows under controlled compositor/display conditions
and retain both states. Acceptance: explain the approximately 4.4 to 8.4 ms
step with measured waits, then repeat three uncontended native-scale rounds
of all four presented fixtures. Do not change present mode defaults without
your decision. Repeat the matched 1080p tip/baseline series on the quiet
host to settle its tail gate. This work is proposed, not implemented here.

07 is complete as a measurement ticket; the performance target remains
missed. 05 and 06 retain the choices you approved: SMAA, 100% scale, and
sharpening below 100%.

## Validation and authorization

frame-profiling.md. Local work only; no push or PR without approval.
