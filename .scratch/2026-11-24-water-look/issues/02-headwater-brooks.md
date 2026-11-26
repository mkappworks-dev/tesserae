# 02: Headwater brooks

Status: ready-for-agent
Blocked by: 01

Spec: [Water that looks natural](../spec.md), Hydrology.

## What to build

Streams start high on mountain slopes: the drainage carries water from a smaller
catchment than today's 120 drainage lattice points (0.12 km²), as brooks that
widen downstream into today's river widths. The valley's dig scales with width,
so a brook draws a narrow wet band rather than today's minimum of about 5 m. A
player on a ridge sees brooks running down into the valley's river. Generator
version 3 changes in place before the first release; new code uses no platform
maths (ADR-0032).

## Acceptance criteria

- [ ] A test: on Terra's default seed, streams begin within a set distance of
      high ridges, and every brook joins a larger stream, a lake or the sea.
- [ ] A brook's wet columns are continuous, at least one Voxel wide at 1 m
      Voxels, and never narrow downstream.
- [ ] The existing drainage and terrain tests pass, "generated water has no air
      below or beside it" included.
- [ ] The recorded digests are recorded again; Comments list which moved.
- [ ] The `worldgen_v3` bench against `main`, interleaved, in Comments; a
      regression over 5 % is brought to the user before landing.
- [ ] Before and after captures of a mountain slope, shown to the user.
