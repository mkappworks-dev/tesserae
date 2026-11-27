# 32: Falls span the stream's outlet

Status: ready-for-agent
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

- [ ] At the reported spot the Fall spans the outlet with no dry gaps.
- [ ] Over the generated boxes, more of a Fall's lip pours; a test.
- [ ] A capture shown to the user.
