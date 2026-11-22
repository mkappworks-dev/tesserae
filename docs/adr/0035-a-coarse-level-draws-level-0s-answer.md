# A coarse level draws level 0's answer, and stands its trees and Structures

Status: accepted.

## Context

Flying toward distant terrain, shores turned from sand to sea and back, and
trees appeared, at each ring join. A measurement on the default Terra seed found
the coarse levels deciding water and land each by its own rounding: a seabed is
land from 23 m at level 0, 22 m at level 2, 20 m at level 3, 8 m at level 4 and
16 m at level 5, so 10 to 40% of coastal coarse columns are dry where level 0 is
wet. Level 4 lost the sea outright, because the tie the half-Voxel lowering
meant to break is overwritten before it is read. A coarse column also takes its
Biome at its corner, so seabed made land keeps the ocean Biome and shows as
sand. Coarse levels draw no trees or Structures
([ADR-0011](0011-coarse-levels-are-derived.md),
[ADR-0029](0029-structures-are-declared-templates-placed-as-pure-functions-of-metres.md)),
except at level 1 when its job happened to read held level-0 Chunks.

## Decision

**Coarse agreement.** Every coarse level classes each column as water or land,
and picks its top Material, by level 0's answer at the column's centre in
metres: the same terrain height against the same water level, the Biome sampled
at the centre. A dry column's top is raised to at least the waterline, so
rounding cannot sink land under the drawn sea. This replaces the per-level
half-Voxel lowering.

It lives in the coarse path, never in `Worldgen` at a Voxel size: a World with 2
m Voxels uses `Worldgen` at 2 m as its level 0, and Generator version 3's output
is frozen ([ADR-0033](0033-generator-version-3-is-the-only-version.md)).

**Impostors.** At level 1 a coarse Chunk stands every tree that level 0's pure
draws stand there, written into the coarse grid so that any solid beats air,
which the half-volume vote does not. Level 1 always runs the draw, so its trees
never depend on what the Server held. From level 2 up a coarse column draws
Canopy cover from the same Biome and density fields instead: level 2's tree
draws measured about 7 times its Chunk's cost without them. Structures,
Settlement Plots and paths stand at levels 1 to 3 only: at 16 and 32 m a
building is under one Voxel, and their draws took a level-5 Chunk from 0.5 to 10
ms, which delayed the whole field and level 0's own fill. Levels 4 and 5 keep
the agreed ground. Bridges stand at level 1 only, as their gap search costs
seconds per square kilometre above it.

**Nothing new is stored.** Impostors and Canopy cover are pure functions of the
seed, the parameters and metres, like every coarse Chunk. There is no LOD cache
on disk or between Sessions.

**The join stays a hard cut**
([ADR-0019](0019-a-level-is-drawn-until-another-replaces-it.md)). With agreement
and Impostors in place, a join shows a change of shape, never of place, and a
dither band would draw both levels across every overlap.

## Considered

- **A stored LOD, built from Chunks as they are visited**, as other voxel
  engines with distant terrain do. Needed only where the far field is not a pure
  function; here it would be a second source of truth.
- **Exact trees at every level.** Placement is pure, but only on level 0's
  lattice, which costs some 10⁴ times a level-3 Chunk today, for trees under a
  pixel.
- **Billboards or baked cards per species.** A new draw path and material for
  what coarse Voxels already show. Revisit if a capture shows coarse trees as
  blobs.
- **Fixing the level-4 tie alone.** It moves the error from wet-made-dry to
  dry-made-wet, 30% of coastal columns either way.

## Consequences

- Coarse Chunks cost more: a centre sample per column at every coarse level, and
  the level-0 draws for each Impostor. Both are measured against the step-6
  budget.
- Plants and ground cover stay out of coarse levels; their colour is not folded
  into the ground.
- Impostors cast no sun shadow: the shadow cascades end inside level 0's ring.
