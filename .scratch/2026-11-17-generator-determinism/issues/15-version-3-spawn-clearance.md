# 15: A version 3 spawn stands clear of what is under the Player

Status: ready-for-agent

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

- [ ] That test runs under version 3 and passes, unweakened.
- [ ] A test fails before the fix and passes after it.
- [ ] The cause, and which spawn positions and digests the fix moves, are noted
      in a comment here.
