# 01: Water fixtures and a baseline

Status: ready-for-agent
Blocked by: none

Spec: [Water that looks natural](../spec.md).

## What to build

Four capture and profiling fixtures on Terra's default seed, and the baseline
every later ticket is judged against.

- **lake**: a lake's shallows and deep middle in one view, at a low angle so the
  far side shows the sky's reflection.
- **river**: a river stepping down at least two lips into pools, looking
  downstream.
- **coast**: a beach and open sea, looking out to sea.
- **underwater**: an Eye a few metres under a lake, looking across its floor
  into the distance.

`scripts/profile-frames` strips every `TESSERAE_*` variable, so the Server
places these Eyes the way the `cave` fixture does. Eyes sit off whole metres,
not on a Voxel plane. Positions are in metres; ticket 04 checks they still show
their subject after the hydrology changes.

## Acceptance criteria

- [x] The four fixtures run through `scripts/profile-frames` and are listed in
      the frame-profiling doc.
- [x] Captures from each, and from `flight`, with
      `TESSERAE_SCREENSHOT_OFFSCREEN=1`, in one `/tmp` folder (never committed),
      shown to the user.
- [ ] Frame timing and a `--instruments gpu` series for the four and `flight`
      (spec, Testing Decisions), in Comments with the load average and commit.

## Comments

- 2026-11-26, at 2db55d3a plus this ticket's fixtures: the Eyes were found by
  asking Terra's Generator, not by flying. Within 2.4 km of spawn there are
  only lakes; rivers start about 3 km west, and the nearest open sea beside a
  gentle beach is about 8 km west. Terra's sea stands at 24 m, its
  `sea_level_m`.
- Captures taken with `TESSERAE_SCREENSHOT_OFFSCREEN=1` at the runner's
  settings, by running the binary directly, in
  `/tmp/tesserae-water-look-captures/`.
- Timing and the `--instruments gpu` series are not run yet: the host had a load
  average of 24-27 and other sessions' Cargo builds running, which the runner
  refuses. Run them on a quiet host before 08 lands its first look change.
