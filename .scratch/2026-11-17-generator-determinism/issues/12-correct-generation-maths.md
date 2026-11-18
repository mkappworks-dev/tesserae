# 12: Version 3 uses no platform maths

Status: ready-for-agent

Spec: [Generator determinism](../spec.md).

Blocked by: 10, 11, 13.

## What to build

The one in-place correction ADR-0032 allows. Every `sin`, `cos` and `atan2` that
feeds generation calls `libm`: Settlement founding and the spawn ring, roads,
building orientation, the level-disc test, crop field reach near water and the
dry land search. Every `powi(2)` in generation becomes a multiplication. `libm`
is pinned to its exact current version. 11's temporary allows go. The commit
re-records the version 3 digests and spawn positions on macOS, and its message
names every case that moved, or says none did.

## Acceptance criteria

- [ ] No temporary allow naming 12 remains, and the Lint job passes, so no
      listed method is left in generation.
- [ ] The `powi` replacement alone moves no digest (checked before the
      trigonometry changes, noted in a comment here).
- [ ] `libm` is pinned to an exact version.
- [ ] The Determinism job passes on every target; 10's allowances are removed
      and it becomes a required check.
- [ ] The commit message lists the digests and spawn positions that moved.
