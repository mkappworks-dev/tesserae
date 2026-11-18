# 15: A version 3 spawn stands clear of what is under the Player

Status: done

Spec: [Generator determinism](../spec.md).

Blocked by: None (can start immediately).

## What to build

Ticket 04 (see its Comments) found that at seed 50 version 3 spawns the Player
at the origin, 138.9 m up, overlapping something generated under the body;
`the_first_spawn_stands_clear_of_a_surface_feature_under_the_body` stays pinned
to version 2 because of it. Find why (the spawn query's fallback, the Server's
clearance, or the thing generated there) and fix it, inside the pre-release
exception ADR-0032 allows.

## Acceptance criteria

- [x] That test runs under version 3 and passes, unweakened.
- [x] A test fails before the fix and passes after it.
- [x] The cause, and which spawn positions and digests the fix moves, are noted
      in a comment here.

## Comments

- Cause: the Server's clearance, not the spawn query or what is generated. At
  seed 50 the origin is a rugged outcrop where version 3's overhang density
  fills ground to Voxel 141 over a base surface at 136.9 m, and
  `Worldgen::clear_height_m` counted only `top_index` of the base surface, so
  the Player's body stood inside the pushed rock.
- Fix: `clear_height_m` now finds each column's top as the Chunk does, the
  density's push included (`Worldgen::ground_top`). The Chunk and it share the
  per-column push (`overhang_amp_at`) and what it keeps off
  (`overhang_kept_off`), so the two cannot drift.
- Red before the fix:
  `terrain_v3::a_body_clears_the_overhang_density_above_the_base_surface` (clear
  at 136 m under ground to Voxel 137) and the server test under version 3 (seed
  50 inside something at (0, 138.89, 0)).
- Generation is unchanged: no Chunk, digest or fingerprint moves, and the
  pre-release exception is not used. Only a spawn Eye standing over a pushed
  column rises, to clear the rock; no test records such a position, so none was
  re-recorded.
