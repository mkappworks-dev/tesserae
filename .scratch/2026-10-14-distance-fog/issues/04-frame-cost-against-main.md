# Frame cost against main

Status: done
Blocked by: 03

Spec, Performance. Fog is GPU work, so the figure is the frame, not
`MainThreadCost`, and it is read with a load average in hand.

## What to build

Nothing in the product. A measurement, written into the spec's Outcome.

- Read `uptime` before and after every batch and quote it. Do not measure above
  a load of about 5: wall-clock figures lie, and the 60 Hz cap hides a fast
  frame (a run whose median flight frame is 16.6 ms is at the cap and is left
  out, with a count of how many were).
- Build `main` (at `14a8fae`, or whatever `main` is at the end) in its own
  checkout so its `target/` is its own, not the primary checkout's if another
  session is using that. Never a fixed `/tmp` name.
- `scripts/frame-runs` offscreen (`TESSERAE_SCREENSHOT_OFFSCREEN=1`), 1080p, the
  same flight, **interleaved**: main, branch, main, branch. At least five of
  each, more if the spread is wider than the difference. A third arm, the branch
  with `TESSERAE_FOG=0`, isolates fog from anything else the branch changed.
- Report the median and range of `median_flight_ms`, `p99_flight_ms`,
  `worst_flight_ms`, and `worst_tick_ms` for each arm, in a table, with the
  loads.
- If the difference is inside the spread, say it is not measurable and give the
  spread. If fog costs something, say how much per frame, and whether it grows
  with the number of cameras (six cameras are six fogged passes over their own
  pixels, but a pixel is only shaded once per camera that has geometry there).
- Note the pipeline specialisation: the chunk pipeline is built once more with
  `DISTANCE_FOG`. If the first-frame time moved, say by how much.

## Acceptance criteria

- The table, the loads, the number of runs and the number left out are in the
  spec's Outcome and will be in the PR.
- The build of `main` used is removed afterwards if it was a worktree this
  ticket created (`git worktree remove`), and nothing of it is left in the
  branch.
- No code changed. If something had to change to make the measurement possible
  (a flag in `frame-runs`), it is its own commit and the diff says why.
- `Status: done` is set in the commit that finishes this ticket.

## Comments

### Done

Measured, no code changed. Full table, loads and counts are in the spec's
Outcome, "Ticket 04: frame cost against main". In short:

- **Not measurable.** 14 interleaved rounds of main, branch and branch with
  `TESSERAE_FOG=0` (42 runs). Median flight frame 5.3 ms (main), 5.4 ms (branch),
  5.3 ms (`FOG=0`), each with a run-to-run range of about 1.5 ms. Fog costs under
  about 0.3 ms a frame at 1080p; I cannot say how much less.
- **13 of 42 runs left out** as at or near the 60 Hz cap (main 5, branch 3,
  `FOG=0` 5).
- **The load never got quiet.** 3.9 to 5.0 before each run, up to 7.4 after, in
  28 runs over 5. Other sessions were building and running throughout.
- **Per-camera scaling and the pipeline build were not resolved**: both are
  inside the spread (`l0_shown_s` 1.71 s on main and branch).
- `main` was built in a detached worktree at `f57f72d` and removed afterwards.
