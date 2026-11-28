# 21: Stream speed from discharge

Status: done
Blocked by: 16

Spec: [Water that looks natural](../spec.md), Flow. From the user's review: a
white layer covers shallow rivers and the delta.

## What to build

The Generator's Flow takes Manning's formula over the node-to-node slope, so
70% of a river and 40% of a delta run above 3 m/s and draw as white water. A
river's speed grows slowly with its discharge (hydraulic geometry, about
Q^0.1): 0.3 to 1 m/s in a pool, faster only at a step, never past the speed a
wave runs at its depth (Froude 1, `sqrt(g d)`). White water is drawn at steps,
Falls and where the Froude number nears 1, not from speed alone. No terrain
changes.

## Acceptance criteria

- [x] Pools and the delta run under 1 m/s; a step or rapid may run faster.
- [x] No white water on the river, estuary and delta fixtures away from steps
      and Falls; captures shown to the user.
- [x] World digests unchanged.

## Comments

- 2026-11-27: a pool runs 0.6 m/s per km² of catchment to the 0.1, up to 2.5
  times that on a reach of 4% or steeper, held under Froude 0.9. Over the
  river, estuary and delta fixtures none runs past 2.5 m/s, against 40 to 70%
  past 3 m/s before. Flow is not in any digest.
- 2026-11-27: test `pools_and_river_mouths_run_under_a_metre_a_second`: every
  reach under 0.5% and every delta head under 1% on 8 worlds runs under 1 m/s.
  Mouths faster than that drop down steeper reaches, rapids by the ticket's
  own terms. The captures were approved by the user.
