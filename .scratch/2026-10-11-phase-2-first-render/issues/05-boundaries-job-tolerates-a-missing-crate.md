# Make CI's `Boundaries` job tolerate a crate that does not exist yet

Status: done
Blocked by: none

## Task

`.github/workflows/ci.yml`'s `Boundaries` job runs `cargo tree -p
tesserae-mods`. That crate arrives in step 7, so the job fails on every branch
today. `cargo tree` exits non-zero for an unknown package and `set -e` turns
that into a failed job.

Skip a crate the workspace does not contain, and keep checking the ones it
does. Do not delete the check: it is the only thing enforcing invariant 3 in
[`docs/architecture.md`](../../../docs/architecture.md), and it becomes
load-bearing in this branch, which is where `bevy` first enters the workspace.

A crate silently vanishing from the check later is the failure mode worth
guarding — `tesserae-core` must never be skipped by accident.

## Done when

The job passes with `tesserae-mods` absent, still fails if `tesserae-core`
gains a `bevy` or `wgpu` edge, and fails rather than skips if `tesserae-core`
itself goes missing.

## Comments
