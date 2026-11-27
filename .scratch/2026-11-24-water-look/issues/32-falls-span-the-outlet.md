# 32: Falls span the stream's outlet

Status: done
Blocked by: 31

Spec: [Water that looks natural](../spec.md), Rivers. From the user's review
(seed 817232797318845137 near -692, 1320): the water pours over a few separate
strips one Voxel wide, not across the stream.

## What to build

A Fall pours across the whole width of the stream where it leaves its pool.
A lip finds its lower pool along its cardinal step and along the two diagonals
either side of it, so a stream that crosses the edge at a slant still pours
along all of it.

## Acceptance criteria

- [x] At the reported spot the Fall spans the outlet with no dry gaps.
- [x] Over the generated boxes, more of a Fall's lip pours; a test.
- [x] A capture shown to the user.

## Comments

- 2026-11-27: a lip looks for its lower pool straight on and along the two
  diagonals beside that line, nearest first; and a lip on a slanting edge pours
  over both its open faces under one film. Fall columns round the test boxes:
  192 to 249 (`a_slanting_stream_pours_along_its_lip`); test
  `a_lip_on_a_slant_pours_over_both_its_open_faces`. At the reported spot the
  two drops have 8 and 5 lips in a line, and their water spans the outlet but
  for a column at each end: `/tmp/tesserae-water-look-captures/34-userfall/faces.png`.
