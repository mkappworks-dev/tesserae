# 16: Spawn and town tests stop relying on version 2's map

Status: done

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

- [x] All three run under version 3 and keep asserting their original subject.
- [ ] None takes more than a few seconds longer than before; any search reach is
      a named constant.
- [x] No generation code or Game content changes.

## Comments

- The spawn test's World is all sea: Sea level and the lake at 70 m, and
  `landforms.height_m` 0.5, under the metre above water a spawn needs, so the
  6 km dry-land search fails and the Player spawns over the origin's water.
  `height_m` 0 is not usable: `Base::base_height_m` divides by it
  (`crest_m = h * SUMMIT_SHARE`), the heights go NaN and `Drainage::solve`
  never empties its heap. That hang, not the search, may be why ticket 04's
  probe did not finish; the search itself takes 0.7 s.
- Terra's nearest town at seed 7 under version 3 is about 5.2 km out
  (`TOWN_SEARCH_M` is 8192 m). `terra_town_m` finds it with a copy of the
  World whose non-town Settlement rows name no Settlement (row places kept, so
  draws match), then the tests assert on the real World there.
- Timings (test profile, host load about 7), version 2 then 3: spawn 0.00 s to
  0.73 s; town 1.2 s to 6.3 s; Settlement chest 3.4 s to 10.3 s. The search is
  1.3 s of each town test; the rest is version 3's Chunk generation, cut by
  `Generated::around_ground` (only a band round the Settlement's ground) and by
  `Generated` filling its Chunk rows in parallel. The chest test's 7 s is past
  "a few seconds"; generating less of the town would mean asserting on less of
  it.
