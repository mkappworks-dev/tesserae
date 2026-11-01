# 02: Draw a level until its replacement lands

Status: done
Blocked by: 01

Spec: [streaming performance](../spec.md).
PR boundary: own branch `tech/streaming-no-gaps`.

## What to build

- Replace the metre cut in `render/chunk_material.wesl` with coverage: a level-k
  fragment is discarded wherever any finer level j < k has delivered the
  level-j Chunk at that point. The shader walks the finer levels, up to five
  bitset lookups. Delivered means membership of `ChunkEntities::delivered` in
  `client/chunk_view.rs`, which already includes empty meshes, though those
  spawn no entity.
- Keep one bitset per level in `drawn_mask.rs`'s layout, updated incrementally
  on delivery and unload. Opaque and cutout get the real buffer instead of the
  empty one.
- Client-side retention in both directions. An unloaded mesh stays drawn until
  its region is covered: a coarse Chunk once all eight children are covered,
  recursively; a finer Chunk once its coarser parent is delivered. Or until it
  lies outside every level's mesh shell. A retained mesh's despawn and its mask
  update land in the same frame.
- Drop `COARSE_NEAR_FRACTION`; the coarse near plane matches the Player's.
- An ADR: a level is drawn until coverage from another level replaces it, and
  retention is Client presentation only (ADR-0002).

## Acceptance criteria

- [ ] Zero rule-hole frames in every fixture.
- [ ] No blended surface is drawn twice where levels overlap; the existing
      `drawn_mask` water tests keep passing, plus one with level 2 over
      partially delivered level 0.
- [ ] Retained meshes are bounded: a teleport releases them.
- [ ] Frame p50 and p99 do not regress against 01's baseline.
- [ ] Review captures of a dive show coarse terrain, never the clear colour,
      where a finer level has not arrived.

## Validation and authorization

frame-profiling.md. Local work only; no push or PR without approval.

### 2026-11-01: final series; done, p50 cost accepted

Final series `/tmp/claude-streaming-impl/final`: release `profiling` builds,
offscreen, two interleaved rounds (cut from three), one binary per merge in
order (`bNN` is the integration branch just after ticket NN), the tip `075ff52`,
and the tip with one switch changed. Medians of the two runs; a range is the two
runs, low-high.

- Rule-hole frames: `teleport` 588 and 599 a run at `b01`, 0 at `b02` and in
  every fixture at every later binary.
- Retained meshes are released on `teleport`: Chunk entities 1,272 at both `b01`
  and `b02`.
- Frame p50 regresses 5-10% against `b01` in 6 of 8 fixtures, ranges apart:
  `active` 8.16 -> 8.81 ms, `transparency` 8.79 -> 9.42, `flight` 8.08 -> 8.64,
  `sprint-walk` 8.13 -> 8.91, `teleport` 7.82 -> 8.33. p99 within 0.6 ms. That
  fails "p50 does not regress"; you accepted it on 2026-11-01 as the cost of
  drawing no gaps.
- The tip is 40-50% under `b01` in every fixture (`stationary` 7.92 -> 4.54 ms,
  `dive` 6.82 -> 3.32).
