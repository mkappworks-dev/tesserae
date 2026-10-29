# 15: Check the targets

Status: ready-for-agent
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
