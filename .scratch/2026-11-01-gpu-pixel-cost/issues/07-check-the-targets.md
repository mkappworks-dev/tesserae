# 07: Check the targets

Status: ready-for-agent
Blocked by: 03, 04, 05, 06

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

## Validation and authorization

frame-profiling.md. Local work only; no push or PR without approval.
