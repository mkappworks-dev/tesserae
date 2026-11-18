# 06: The Generator answers where a new Player spawns

Status: ready-for-agent

Spec: [Generator determinism](../spec.md).

Blocked by: None (can start immediately).

## What to build

The Server's spawn search becomes one public query on the Generator that always
returns a position: the nearest Settlement in widening rings, else the nearest
dry land to the origin, else the origin. The Server's spawn wrapper keeps only
what is not generation (clearance above the ground, water and anything generated
under the Player's body) and calls the query. A new Player spawns exactly where
they did before.

## Acceptance criteria

- [ ] The Server holds no part of the search, its rings or its fallback; it
      calls the Generator's query.
- [ ] For the existing test seeds the query returns the position the Server's
      search returned before (a test pins one).
- [ ] Generator output and the recorded digests are unchanged.
