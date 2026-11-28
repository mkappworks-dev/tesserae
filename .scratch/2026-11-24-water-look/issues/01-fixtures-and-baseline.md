# 01: Water fixtures and a baseline

Status: done
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
- [x] Frame timing and a `--instruments gpu` series for the four and `flight`
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
- 2026-11-27, the baseline, at 51bca61d (ticket 06, before any look change), a
  release `profiling` build, three interleaved rounds each through
  `scripts/profile-frames` at 1920 x 1080 offscreen. Load average 6.8 at the
  start and 7.4 at the end: the runs' own process, at about 560 % CPU, with no
  build or other benchmark running.

  | Fixture    | Frame p50 | Frame p95 | GPU frame p50 |
  | ---------- | --------- | --------- | ------------- |
  | lake       | 4.82 ms   | 11.27 ms  | 3.53 ms       |
  | river      | 14.50 ms  | 16.17 ms  | 14.64 ms      |
  | coast      | 4.37 ms   | 9.96 ms   | 3.19 ms       |
  | underwater | 3.86 ms   | 8.93 ms   | 2.92 ms       |
  | flight     | 5.48 ms   | 11.61 ms  | 4.66 ms       |

  Frame columns are the plain series; the GPU column is the `--instruments gpu`
  series, whose frame medians ran 0.3-0.4 ms higher under the instrument
  (river 14.54). River is GPU-bound, and not by water: 11.1 ms of its 11.7 ms
  main view is the runner's setup span, between the prepass's end and the main
  pass's start, against 1.0 ms for `flight`; what in that span costs it is not
  yet known. The opaque and
  transparent passes are under 0.15 ms each in every fixture.
