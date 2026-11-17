# 06: SIMD noise, if noise is still worth it

Status: ready-for-agent

Blocked by: 03, 04, 05

Spec: [Worldgen performance](../spec.md).

## What to build

First a re-measure: noise's share of generation in a sampled flight once tickets 02 to 05 have landed. At 15% or less, this ticket is closed as `wontfix` with the figure. Otherwise the 2D and 3D fBm gain a four-wide path through the `wide` crate, used where a caller already walks a row (building the column cache, the drainage window, the overhang push down a column), giving the scalar result bit for bit: the same operation order, octave sums per lane, and no fused multiply-add.

## Acceptance criteria

- [ ] The re-measured share is in Comments, whichever way the gate goes.
- [ ] If built: a property test over many seeds and positions shows the four-wide result equals the scalar one bit for bit, on every CI architecture.
- [ ] If built: version 3 Chunks match the ticket 01 fingerprint; versions 1 and 2 keep their recorded digests.
- [ ] If built: bench figures before and after in Comments.
