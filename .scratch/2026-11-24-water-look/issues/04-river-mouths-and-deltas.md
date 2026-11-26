# 04: River mouths and deltas

Status: ready-for-agent
Blocked by: 03

Spec: [Water that looks natural](../spec.md), Hydrology.

## What to build

Where a river reaches the sea its channel widens into an estuary; above a width
it fans into a few distributaries over a shallow shelf, a delta. Settlements,
bridges, coarse levels and spawn read the Water surface, so they follow
unchanged. Generator version 3 changes in place; new code uses no platform maths
(ADR-0032).

This is the last terrain change, so it also checks the fixtures from 01 still
show a lake, a river with lips, a coast and a lake floor, moving an Eye and
retaking its baseline on 01's commit if not.

## Acceptance criteria

- [ ] A test: every river that reaches the sea on Terra's default seed ends in a
      mouth wider than its channel; a river above the delta width ends in two or
      more channels.
- [ ] Generated water still has no air beside it; the drainage and terrain tests
      pass.
- [ ] Digests recorded again; the `worldgen_v3` bench in Comments, under the
      same 5 % gate as 02.
- [ ] Captures of an estuary and a delta, shown to the user.
- [ ] 01's fixtures still show their subjects, or are moved and rebaselined,
      noted in Comments.
