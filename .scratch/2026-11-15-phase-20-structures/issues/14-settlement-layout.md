# 14: Worldgen lays out a Settlement

Status: ready-for-agent

Blocked by: 04, 05

Spec: [Phase 20](../spec.md).

## What to build

A Biome lists `settlements`: at most 2 rows naming a Settlement and a spacing.
A Settlement declares a radius of at most 64 m, an optional centre Template, up
to 24 Plots from weighted Templates with minimum and maximum counts, an overall
minimum, and a path Material and Shape. Settlements have their own salted
lattice and claim ground before single Structures, fields, trees and scatter,
with a 16 m margin that keeps single Structures out. Roads branch from the
centre on a slope-aware path that avoids water and over-steep ground; Plots grow
beside roads with each door facing the road. A Plot whose ground fails stays
empty, and the Settlement is dropped below its overall minimum. This ticket lays
roads on the natural ground; levelling is ticket 15.

## Acceptance criteria

- [ ] A Settlement is whole across Chunk borders, the same whichever Chunk is
      generated first, and equal in metres at 1.0, 0.5 and 0.25 m.
- [ ] Every Plot's door faces a road, no two Plots overlap, and counts stay
      within each Template's bounds.
- [ ] No single Structure, field or tree stands in a Settlement's disc, and no
      single Structure within its margin.
- [ ] A World whose Biomes list no Settlement generates its recorded digests.
- [ ] A third row, a radius over 64 m, more than 24 Plots or an unknown Template
      is a load error naming the Settlement.
- [ ] Settlements ride in the generation snapshot with the Templates they use.
