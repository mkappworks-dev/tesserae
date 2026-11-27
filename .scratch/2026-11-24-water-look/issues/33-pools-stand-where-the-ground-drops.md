# 33: Pools end where the ground drops

Status: done
Blocked by: 31

Spec: [Water that looks natural](../spec.md), Rivers. From the user's review
(seed 817232797318845137 near -692, 1320): a pool stands on a flat rock wall
some 9 m over the valley floor beside it.

## What to build

A falling segment's Fall stands where the ground under it drops, not at a
hashed point along it: the solve samples the base ground along the segment and
puts the face where it falls through the middle of the drop. So the upper pool
ends at the real edge instead of running on over lower ground, where the
containment walled it in.

## Acceptance criteria

- [x] At the reported spot the upper pool ends at the edge; no wall of more
      than a few metres holds it.
- [x] Over the generated boxes, the containment rock raised beside generated
      water is lower than before; a measure in Comments.
- [x] The no-step, containment and Fall tests still pass.
- [x] A capture shown to the user.

## Comments

- 2026-11-27: two causes. A column took the highest water in reach and the
  deepest cut, so a river or pond between lattice nodes stood over ground
  another channel had dug, or the ground had fallen away under, tens of metres
  deep. Now a river, pond or delta channel stands only as deep as its own over
  the ground the column has (asked of the base inside the drainage, so the
  level is the same whatever ground the caller asks about), and a lake no
  deeper than a stream over a channel cut beside it; Flow only where that water
  stands. And a Fall's face now stands where the ground under its segment drops
  through the middle of the fall, sampled in the solve, not at a hashed point.
  Columns raised more than 3 m round the test boxes: 351 of 645 to 69 of 391
  (`the_containment_raises_little_rock_round_generated_water`); at the reported
  spot the tallest is 3.5 m, from 9 m. What is left is lakes whose rim the 32 m
  lattice saw whole, such as one 37 m deep behind a sea cliff. Not timed: the
  user asked for no benchmarking.
