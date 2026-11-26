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

- [ ] The four fixtures run through `scripts/profile-frames` and are listed in
      the frame-profiling doc.
- [ ] Captures from each, and from `flight`, with
      `TESSERAE_SCREENSHOT_OFFSCREEN=1`, in one `/tmp` folder (never committed),
      shown to the user.
- [ ] Frame timing and a `--instruments gpu` series for the four and `flight`
      (spec, Testing Decisions), in Comments with the load average and commit.
