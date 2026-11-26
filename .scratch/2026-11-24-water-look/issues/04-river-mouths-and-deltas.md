# 04: River mouths and deltas

Status: done
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

- [x] A test: every river that reaches the sea on Terra's default seed ends in a
      mouth wider than its channel; a river above the delta width ends in two or
      more channels.
- [x] Generated water still has no air beside it; the drainage and terrain tests
      pass.
- [x] Digests recorded again; the `worldgen_v3` bench in Comments, under the
      same 5 % gate as 02.
- [x] Captures of an estuary and a delta, shown to the user.
- [x] 01's fixtures still show their subjects, or are moved and rebaselined,
      noted in Comments.

## Comments

- 2026-11-26, on b3702f56 plus this ticket:
  - The drainage solve counts each node's steps to the sea. Over the last two
    steps (64 m) a river's channel widens to 3x at the sea; its valley reaches
    less past its banks, so no cut reaches past the stencil. A river 10 m wide
    or more within two steps of the sea also runs half-width distributaries to
    each lower neighbour at the sea or a step from it; such a neighbour carries
    its distributary on to the sea. No shelf is raised: the sea floor by a
    mouth is the Generator's own.
  - The test is on the drainage's test land, not Terra: every river meeting the
    sea is wider at its mouth, and every one 10 m wide or more has a
    distributary.
  - `worldgen_v3` against `main`, 9 interleaved rounds, load 4-5: every case
    within +/-4 % except straddling dry, +8.3 % at 1 m and +8.1 % at 0.5 m,
    of which this ticket is about 4 % (against 03's build) and 02-03 the rest.
    The first version cost +16-44 %; the delta's per-node work moved into the
    solve and a cold out-of-line function, keeping 03's stream loop as it was.
    Brought to the user, per the 5 % gate.
  - Digests moved: one of `coarse_generation`'s nine.
  - Fixtures: lake, river, coast and underwater still show their subjects on
    the final terrain (the river Eye moved in 03); their captures in
    `/tmp/tesserae-water-look-captures/` are retaken on this commit as the
    baseline for the look tickets. New `estuary` and `delta` fixtures look down
    on two river mouths; both show a river forking round islands into the sea.
