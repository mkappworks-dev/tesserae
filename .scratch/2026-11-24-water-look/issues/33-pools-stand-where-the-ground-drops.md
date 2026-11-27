# 33: Pools end where the ground drops

Status: ready-for-agent
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

- [ ] At the reported spot the upper pool ends at the edge; no wall of more
      than a few metres holds it.
- [ ] Over the generated boxes, the containment rock raised beside generated
      water is lower than before; a measure in Comments.
- [ ] The no-step, containment and Fall tests still pass.
- [ ] A capture shown to the user.
