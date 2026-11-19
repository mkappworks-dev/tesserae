# 06: Gates and captures

Status: done
Blocked by: 01, 02

Spec: [Packed quads](../spec.md).

## What to build

The A/B and capture gate for each item that lands, against the commit before
it.

- Interleaved CPU-time A/B under the host-load rules: frame p50 and p95, mesh
  worker ms per second of flight, mesh bytes held, upload per frame.
- Offscreen captures (`TESSERAE_SCREENSHOT_OFFSCREEN=1`) of flight, active
  and transparency, before and after, in one `/tmp` folder, never committed.
  Pixels match; coarse split lake cells may differ by rounding only.
- A failing item is dropped, with the numbers in its ticket.

## Acceptance criteria

- [x] 02 gated, numbers here.
- [x] 03 gated, numbers here.

## Comments

2026-11-19: series, binaries and provenance as in ticket 01's Comments. 02 is
`ac3a04b3` and 03 is `0380cd9a`, each with `625f7a83`'s fix on top (below).
Frame and main CPU are the per-run median over each timing round's three
logging-on runs, as round 1 / round 2, since the host drifted about 7% slower
for every binary in round 2 and the rounds are interleaved; changes are
against the commit before, in the same round.

**The shader.** `chunk_vertex.wesl` compiled, but item 1 panicked at game
start: Bevy 0.20.1's mesh pipeline specializer keys its cache by
`vertex.buffers[0]`, and `specialize` had emptied the list. `625f7a83` keeps
one attribute-less buffer of stride 0; with it 02 and 03 draw.

**02, quad records, against the base: fails the gate.**

| 02 vs base                 | flight          | active          | transparency    |
| -------------------------- | --------------- | --------------- | --------------- |
| frame p50, r1 / r2         | +13% / +8%      | +12% / +11%     | +11% / +12%     |
| frame p95, r1 / r2         | +13% / +13%     | +26% / +16%     | +22% / +17%     |
| main CPU p50, r1 / r2      | +31% / +11%     | +42% / +19%     | +42% / +20%     |
| mesh worker ms/s, level 0  | 34.4 → 30.3     | 55.4 → 48.3     | 57.0 → 49.7     |
| mesh worker ms/s, all      | 278 → 280       | 372 → 401       | 372 → 411       |
| mesh bytes held            | 69.0 → 11.1 MB  | 75.2 → 12.0 MB  | 75.0 → 12.0 MB  |
| upload wall ms per frame   | 0.011 → 0.015   | 0.013 → 0.018   | 0.013 → 0.019   |
| uploads per frame          | 2.45 → 2.46     | 2.93 → 3.13     | 3.00 → 3.20     |

- Held memory falls 84%, as the spec expected. Level-0 mesh worker time
  falls 12-13%; level 1's coarse remeshing rises, so all levels are 1-10%
  higher.
- Frame p50 and p95 are worse in every fixture and both rounds, beyond the
  5% gate. Upload per frame is worse too, though still about 0.02 ms.
- The GPU is where it goes. Only one 02 GPU run (flight) kept a `frame_end`
  on most frames (1871 of 2069); the others kept it on about 20%, against
  75-95% at the base, consistent with a longer GPU frame. In that run the
  GPU frame p50 is 4.34 ms against the base's 3.97-4.01 (+9%), p95 4.72
  against 4.24-4.27 (+11%). The depth prepass (0.33 → 0.42 ms) and the shadow
  passes (0.46 → 0.56 ms) grew; the main opaque pass did not (0.16 ms). The
  vertex shader's quad buffer reads, run once per shadow cascade and in the
  prepass, are the likely cost.
- Meshing benches: 0.4-81% faster except `greedy_mesh/solid` (+1.2%) and
  `mesh_chunk/shore` (+0.5%), both within run spread.
- Code kept; ticket 02's status is unchanged.

**03, bitmask pass 1, against 02: passes the gate.**

| 03 vs 02                   | flight          | active          | transparency    |
| -------------------------- | --------------- | --------------- | --------------- |
| frame p50, r1 / r2         | +0.7% / -1.5%   | +0.1% / -1.4%   | +0.8% / -0.9%   |
| frame p95, r1 / r2         | +1.3% / -4.1%   | +1.2% / -1.8%   | +3.8% / -0.4%   |
| main CPU p50, r1 / r2      | 0% / -0.2%      | +0.9% / -0.6%   | +2.0% / -0.4%   |
| mesh worker, level 0       | -28% / -25%     | -29% / -30%     | -30% / -30%     |
| mesh worker, all levels    | -15% / -17%     | -13% / -15%     | -15% / -16%     |

- No consistent frame regression: under 4% one round, lower the other. GPU
  frame p50 in its one well-sampled run (flight) is 4.40 ms against 02's
  4.34. Held bytes and uploads are unchanged.
- Meshing benches, median of six: every one faster. `greedy_mesh` terrain
  202 → 49 µs (-76%), solid -74%, glass room -73%, terrain light gradient
  -74%, stairs at budget -52%, checkerboard -26%, all stairs -3%;
  `mesh_chunk` straddling 271 → 118 µs (-57%), cave crossed -70%, shore -31%.
- Left open with 02, which it builds on.

**Captures.** Every run's offscreen 1920x1080 capture (36 per binary),
compared pixel by pixel. The base does not match itself: two base
runs differ in 0.003-0.06% of pixels, all at small moving things. Against
that:

- Transparency: 02 and 03 match the base at that level (at most 0.06%).
- Flight: most captures match at that level. About a third differ in 0.2%,
  a strip of distant coarse terrain landed or not; base against base shows
  the same 0.2% strip, so it is landing timing.
- Active: most captures match at that level. About one in five differs in 0.3%, the water curtain's flowing edge at another step of the simulation;
  other runs of the same binaries match the base, so it is fluid state.
- No packing error seen; no coarse split lake cell showed a rounding
  difference.

Captures and diff images are in a `/tmp` review folder, not committed.
