# 06: Coarsest level first

Status: done
Blocked by: 01

Spec: [LOD features](../spec.md).

## What to build

At spawn or after a teleport, the horizon appears first and finer levels
replace it. Coarse jobs are prioritised by distance from the Eye over their
level's Cell, so the outermost level fills before the inner ones. A coarse
level stays drawn until the finer one arrives (ADR-0019), so nothing opens.
Level 0's own priority is unchanged.

## Acceptance criteria

- [x] A Bevy app test: from a cold start, the outermost level's first meshes
      land before the inner coarse levels'.
- [x] Time to a whole field and to each level, measured in 09's series; here, a
      test that level 0's job order is unchanged.

## Comments

- No engine change was needed: the hole pass of `server::lod::walk`, from the
  streaming-performance priority ticket, already dispatches the coarsest
  level's holes first, and the refinement pass ranks coarse Chunks by
  `score / 2^level`, distance over the level's Cell.
- `from_a_cold_start_and_after_a_teleport_the_outermost_level_lands_first` pins
  it at the default `LodBudget`; it fails with the hole pass walked finest
  first.
- `a_level_zero_chunk_ranks_by_its_distance_alone` pins level 0's scores
  against hand-worked values.
- Fill times are 09's.
