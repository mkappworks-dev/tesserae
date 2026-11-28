# 07: Rebase onto the colour work

Status: done
Blocked by: none

Spec: [Water that looks natural](../spec.md), Order.

## What to build

Rebase this branch onto `main` once the colour work merges. It rewrites the
chunk shaders, takes face-word bits 30 and 31, adds haze to the fog and a sky
material, changes the `appearance` record and Terra's water texture, amends
ADR-0033 and records the version 3 digests again.

## Acceptance criteria

- [x] The colour work is merged to `main` (#75, `6a81409f`), and this branch is
      rebased onto it.
- [x] Digests recorded again over both branches' terrain changes: nothing to
      record, since the rebase came before this branch changed any code.
- [x] 01's captures are taken on the rebased commit: 01 had not started, so its
      captures are the "before" for every look ticket as written.
