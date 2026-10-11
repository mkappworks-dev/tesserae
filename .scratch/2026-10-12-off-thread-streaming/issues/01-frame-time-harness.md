# A frame-time harness that separates the fill from flight

Status: ready-for-agent
Blocked by: none

Everything after this is judged by it, so it goes first and lands on `main`'s
streaming unchanged. Spec "Performance".

## What to build

Extend `client/capture.rs`, `dev` only and inert without `TESSERAE_SCREENSHOT`,
so one run prints one machine-readable line:

- **Fill time**: seconds from start until nothing is stale, in flight or dirty.
  Needs a predicate on `streaming::stats::StreamingStats`, since the Client must
  not read `MeshState`: add `settled: bool` (and, from later tickets,
  `in_flight`) to the published summary.
- **Worst frame before the fill finished** and **worst frame after it**, the
  second while the camera flies. The existing "worst since 1.5 s" folds them
  together and cannot say which one moved.
- **99th-percentile and median frame after the fill**, from every frame's
  `delta_secs`, not the rolling window `FrameTimeDiagnosticsPlugin` keeps.
- Keep the existing fields (`loaded`, `meshed`, `drawn`) on the same line.

A `scripts/` helper (or a documented loop) runs the capture N times and prints
median and range per figure. It does not need to be elegant; it needs to be the
same tool for `main` and for the branch.

## Then measure

On this ticket's commit, five runs of `TESSERAE_SCREENSHOT=... cargo dev` with
the default fly and settle times. Record the table in this file's Comments: that
is the "before" the PR quotes. Note the machine, the load average, and that the
build is debug at `opt-level = 1`.

## Acceptance criteria

- One line per run with every figure above; the existing screenshot is still
  written.
- Fill time is defined by state, not by a constant, so it stays meaningful when
  streaming becomes asynchronous.
- Five baseline runs are recorded in Comments.
- `./scripts/cargo-ci` passes; the new fields are `dev`-gated, so the build
  without `--features dev` has no dead code.
