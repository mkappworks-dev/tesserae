# 16: Spawn and town tests stop relying on version 2's map

Status: ready-for-agent

Spec: [Generator determinism](../spec.md).

Blocked by: None (can start immediately).

## What to build

Three tests ticket 04 left on version 2 (see its Comments) depend on where
version 2 put things, not on version 2 itself:

- `the_spawn_point_is_above_water_that_covers_the_ground_there` needs a World
  with no dry land within the search reach; it runs on a hand-built all-water
  World instead of a seed.
- Terra's `terra_lays_out_a_town_of_cobbled_roads` and the loot Settlement-chest
  test pinned a town at (356, 30); they find a town of cobbled roads under
  version 3 by search instead.

## Acceptance criteria

- [ ] All three run under version 3 and keep asserting their original subject.
- [ ] None takes more than a few seconds longer than before; any search reach is
      a named constant.
- [ ] No generation code or Game content changes.
