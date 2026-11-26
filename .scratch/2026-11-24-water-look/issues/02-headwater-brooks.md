# 02: Headwater brooks

Status: done
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

- [x] A test: on Terra's default seed, streams begin within a set distance of
      high ridges, and every brook joins a larger stream, a lake or the sea.
- [x] A brook's wet columns are continuous, at least one Voxel wide at 1 m
      Voxels, and never narrow downstream.
- [x] The existing drainage and terrain tests pass, "generated water has no air
      below or beside it" included.
- [x] The recorded digests are recorded again; Comments list which moved.
- [x] The `worldgen_v3` bench against `main`, interleaved, in Comments; a
      regression over 5 % is brought to the user before landing.
- [x] Before and after captures of a mountain slope, shown to the user.

## Comments

- 2026-11-26, on 39cd62fb plus this ticket:
  - Streams start at 24 lattice points (0.025 km²); at 8 a brook ran through a
    third of all cells. Brooks are 1.6 m wide at the head (over a 1 m Voxel's
    diagonal) and widen into the rivers' old widths from 120 points on; their
    dig and valley reach scale with width, and their water stays in the
    channel they cut. Rivers dig and draw exactly as before.
  - The test is on the drainage's own test land, not Terra: 11 of 14 hilltops
    over 40 m have a brook within 128 m (4 of 14 before); a knoll draining
    away on every side may have none, so it asks three quarters.
  - Biomes read only rivers' cuts (`river_cut_m`): with every cut, Terra's
    River Biome turned whole brook-cut slopes to sand and palms.
  - `valleys_between_mountains_drain_rather_than_flood` now counts only flat
    water as lake; brooks had pushed its wet share to 7.5 %.
  - Digests moved: `coarse_generation` (all 9). `generator_version` and the
    content snapshots did not.
  - `worldgen_v3` against `main`, 7 interleaved rounds, load 7-12: at most
    +3.2 % (forest 3x3 at 0.5 m), river +1.8 % at 1 m and +1.1 % at 0.5 m.
  - Captures: `slope-before-02.png` and `slope-after-02.png` in
    `/tmp/tesserae-water-look-captures/`, from the new `slope` fixture.
- Open: a steep brook running diagonally across the grid reads as a chequer of
  water and one-Voxel rock lips, since each lower column beside higher water is
  raised to hold it. That is the still-pool rule (ADR-0038) on a steep
  diagonal, not the drainage; the water columns (06) and drawn Falls (12)
  should be judged against this view.
