# 23: Springs and deeper mountain streams

Status: ready-for-agent
Blocked by: 22

Spec: [Water that looks natural](../spec.md), Rivers. From the user's review:
streams start from nowhere, and mountain streams are too shallow.

## What to build

A stream starts where its catchment and slope together pass a threshold
(area times slope squared), only in a hollow, and its head tapers back to a
small spring pool, so it rises from a spring high on a slope. Hillslope area
is spread to every downhill neighbour (D-infinity), so planar slopes stop
drawing parallel streams. Mountain streams are 1.5 to 2 m deep in their pools.

## Acceptance criteria

- [ ] Every stream begins at a spring pool or a lake's outlet, never mid-slope
      at full width.
- [ ] No two streams run side by side a node apart.
- [ ] A mountain stream's pools are at least 1.5 m deep.

## Comments
