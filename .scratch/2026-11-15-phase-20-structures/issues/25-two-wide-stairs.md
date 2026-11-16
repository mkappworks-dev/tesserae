# 25: Stairs two cells wide, with room overhead

Status: ready-for-agent

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

- [ ] Every stair cell in every Game's Templates has a stair beside it, turned
      the same way, across its rise.
- [ ] Above every stair's tread at least 2.5 m is clear (three cells at 1 m),
      tested through each Game's Templates.
- [ ] The walk test still reaches every level each building is meant to reach,
      and every Template still fits the undo budget at 0.25 m.
- [ ] `node mockup/export-templates.mjs --check` passes.

## Comments
