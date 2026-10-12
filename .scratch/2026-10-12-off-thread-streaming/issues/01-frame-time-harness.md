# A frame-time harness that separates the fill from flight

Status: done
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

## Comments

**Baseline**, `main` plus this ticket's harness only (streaming unchanged),
Apple M5, debug at `opt-level = 1`, load average 2-3 from other sessions'
builds. `scripts/frame-runs 5`; median (min-max). Times are virtual seconds from
start.

| Figure             | 12 m/s (walk)    | 60 m/s (sprint)  |
| ------------------ | ---------------- | ---------------- |
| `shown_s`          | 1.10 (1.09-1.10) | 1.10 (1.09-1.10) |
| `fill_s`           | 7.24 (7.23-7.24) | 7.24 (7.24-7.24) |
| `worst_fill_ms`    | 32.0 (24.7-32.0) | 27.5 (24.3-32.2) |
| `worst_flight_ms`  | 7.4 (7.4-7.7)    | 7.5 (7.4-7.8)    |
| `median_flight_ms` | 6.9              | 6.9              |
| `p99_flight_ms`    | 7.3 (7.2-7.3)    | 7.3              |

What the numbers say, which is not what the brief assumed:

- **The fill is the whole problem.** Flight is already flat at the baseline,
  even sprinting 360 m (11 Cells): worst frame 0.5 ms above the median. There is
  no headroom to win there, so "lower and flatter while flying" can only mean
  "not worse". The figure to move is `worst_fill_ms`.
- **The slow frames are not at the start.** `fill_s` is 7.24 s, every run,
  because the tick is count-based and the ticks are fixed, so it is reproducible
  to a hundredth of a second. `shown_s` (every Chunk in the mesh extent has been
  sent) is 1.10 s. The 12-32 ms frames sit at about 5.3-5.9 s with the load
  volume already held and the mesh count unchanged, which fits the relight
  backlog draining and republishing as light settles rather than generation. Not
  profiled; ticket 04 will find out.
- **The first 0.2 s is startup, not streaming.** With the view distance cut to
  20 m the same frames at t = 0.02-0.16 s cost 16, 27, 57 and 25 ms, against 16,
  27, 62 and 24 ms with the full ring. So the 1.5 s exclusion
  (`SETTLES_AFTER_S`) hides window creation and shader compilation and nothing
  streaming does, and stays.
- `worst_fill_ms` varies 24-32 ms between runs while every other figure is
  identical to the hundredth; the variation is the frame it lands on, not the
  tick's work. Five runs is the floor.

Changes beyond the ticket's list, all `dev` only:

- The flight now starts when the fill ends (or after `FILL_TIMEOUT_S` = 30 s, in
  which case `fill_s=never`), where it used to start at t = 0 and overlap the
  fill. The shutter is `fly_for + settle_for` after that, so the screenshot is
  taken later than it was.
- `shown_s`, from a second `StreamingStats` flag, because `settled` also waits
  for the margin's light and is not what a player sees fill in.
- `TESSERAE_SCREENSHOT_SPEED_M_S`, because 12 m/s asks streaming for so little.
