# 03: Meanders that grow with the river

Status: ready-for-agent
Blocked by: 02

Spec: [Water that looks natural](../spec.md), Hydrology.

## What to build

Large rivers swing in wide bends and brooks in tight ones: the meander warp's
amplitude and period scale with river width. Today the warp is taken once per
point before the segments are walked; scaling it by width means a warp per
segment in reach, so this ticket measures that cost. The amplitude stays inside
the valley's reach, so it still fits the drainage stencil. Generator version 3
changes in place; new code uses no platform maths (ADR-0032).

## Acceptance criteria

- [ ] A test: meander amplitude grows with river width, and no river's valley
      reaches past the stencil (no cut ends at a region border).
- [ ] "A river valley is not a cliff across a region border" and the other
      terrain tests pass.
- [ ] Digests recorded again; the `worldgen_v3` bench in Comments, under the
      same 5 % gate as 02.
- [ ] Before and after captures of a large river and a brook, shown to the user.
