# 25: Stairs two cells wide, with room overhead

Status: done

Blocked by: 22

Spec: [Phase 20](../spec.md).

## What to build

Play-testing found every Template's stair one cell wide, and the opening in the
floor above it short enough that a Player climbing it hits their head.

- Every stair in every Game's Templates is two cells wide: the mockup's
  `flight()` lays a second lane beside the first, across the way it rises.
- Above every tread there is at least 2.5 m clear, so the opening in the floor
  above is two cells wide and long enough to climb through standing.
- The towers (watchtower, lighthouse, observatory) climb by a two-wide stair
  round a central newel, with square landings at the corners, from the cellar
  where there is one to the roof deck. Towers grow only as far as that needs.
- The stilt houses' and treehouse's outside stairs and the mounds' crypt
  stairs are two wide too.
- Shapes still come from the mockup (`mockup/structures.js`), exported by
  `node mockup/export-templates.mjs`.

## Acceptance criteria

- [x] Every stair cell in every Game's Templates has a stair beside it, turned
      the same way, across its rise.
- [x] Above every stair's tread at least 2.5 m is clear (three cells at 1 m),
      tested through each Game's Templates.
- [x] The walk test still reaches every level each building is meant to reach,
      and every Template still fits the undo budget at 0.25 m.
- [x] `node mockup/export-templates.mjs --check` passes.

## Comments

Mockup (`structures.js`):

- `flight(g, from, path, side)` lays a second lane beside the path across
  `side`, and `headroom` clears three cells over every tread (it cleared two),
  so the opening in the floor above is two wide and runs three cells back from
  the top step. Houses and cellars pass their side; the mounds' crypt stair is
  one flight (it was two one-wide ones); the stilt houses' and treehouse's
  outside stair has its twin lane and a landing two deep, on posts.
- `spiral(g, lo, hi, bottom, top)` replaces the towers' one-wide ring: flights
  two wide up each side, square landings at the corners, a log newel in the
  middle, continuous from the cellar floor to the roof deck, its south flight
  passing over the Door a turn up. The cellar's walls now stand under the
  tower's, so the spiral runs on down. It needs 5 cells inside: the watchtower
  grew from 6 to 7 m square and the lighthouse from 5 to 7 m; the observatory
  was 7.
- The farmhouse's table pair moved a cell south: the wider cellar opening
  had walled a strip of floor off between it, the chests and the table.

Test: `every_stair_walked_to_is_two_wide_with_room_overhead`
(`demo_games/rooms.rs`): every stair cell the walk from a door stands on has a
twin turned the same way beside it across its rise, and three open cells
above. Failed on the old Templates. The walk, door, undo-budget and export
checks pass.
