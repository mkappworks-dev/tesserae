# 05: Overhangs (17a)

Status: done

Blocked by: 03

Spec: [Phase 17](../spec.md).

## What to build

The bounded 3D density near the surface, zero where ruggedness is low. Levels 3 and up read the heightfield.

## Acceptance criteria

- [x] Arches and sheer cliffs appear on rugged ground; plains and sea floors are heightfields.
- [x] The band plus `overhang_m` holds as the one bound.
- Moved to ticket 14: flat-ground Chunks stay within the benchmark budget.
