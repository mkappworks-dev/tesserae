# 44: A larger market stall on corner stones

Status: done

Spec: [Phase 20](../spec.md).

## What to build

Play-testing: "make the market stall bit bigger and make sure the 4 post touch
the ground and their is a corner stone or block for the post".

- The stall grows from 5 × 4 to 7 × 5 cells between its posts.
- Its Ground layer is a paved floor flush with the ground, a stone at each
  corner; a second stone stands on each, and the post rises from it, so every
  post meets the ground.
- The chests stand on the floor, not in the Ground layer; the counter is a whole
  cell; the lamp hangs under the front awning.
- The mockup changes first; every Game's stall is exported from it.

## Acceptance criteria

- [x] Each of the four posts stands on two stones, the lower one in the Ground
      layer.
- [x] Nothing but the floor and the stones is in the Ground layer.
- [x] `node mockup/export-templates.mjs --check` passes.

## Comments

At the larger size Ember's largest village at the test seed laid 7 buildings,
under the demo World test's 8; its village radius now starts at 44 m rather
than 40 m.
