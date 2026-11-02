# 07: Check the targets

Status: ready-for-agent
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

- [ ] Each target recorded as met or missed, with p50, p95, p99 and max.
- [ ] For each missed target, what dominates it and a proposed next ticket
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

Still to do: your rerun of `stationary` and `sprint-walk` with the window
focused (output `/tmp/claude-gpu-pixel-cost/presented-03b`, command in the
2026-11-02 session), and the offscreen check of every fixture.

## Validation and authorization

frame-profiling.md. Local work only; no push or PR without approval.
