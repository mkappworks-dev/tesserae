# 02: `relight_many` in core

Status: done
Blocked by: 01

Spec: [bulk relight](../spec.md), decision 1.

## What to build

`tesserae_core::relight_many(volume, tables, edits: &[([i32; 3], Light)])` in
`light.rs`, beside `relight`, with no new dependency.

- Set every edited position's own light first (its emission, or dark).
- Per channel: one darken queue seeded with every edit whose old level is above
  its new one; one regrow list from what darkening uncovers, every edit that
  emits and every lit face-neighbour of every edit; one brighten.
- Brighten drains brightest level first (per-level buckets, skipping a cell
  whose level changed since it was queued), so a cell many seeds reach is
  raised once rather than once per seed.
- Duplicate positions are allowed and cost nothing extra in the result.
- `relight(volume, tables, at, was)` becomes `relight_many` of one, and its doc
  comment's warning about `light_chunk` stays true of both.

## Acceptance criteria

- [x] Every existing light test passes unchanged.
- [x] A seeded property test over many random scenes (stone, emitters, shaped
      and translucent Voxels, edits that add and remove, across Chunk seams,
      with open sky above): `relight_many` of a batch gives exactly the light of
      the same edits applied one `relight` at a time, and of a fresh
      `solve_bulk` of the final scene.
- [x] The bench from 01 has a `relight_many` case per batch size (1, 64, 4096)
      and the figures, against 01's, are in Comments.
- [ ] `cargo bench -p tesserae-core` shows no regression in the other benches
      (interleaved A/B, CPU time). Run once all core changes are in (ticket 05).

## Comments

### Result, 2026-11-04

The property test (24 seeded scenes round a Chunk seam under open sky: stone,
lanterns, glass, stairs, slabs and diagonals added and removed, batches of 1,
2, 64 and 600) passes: batched light equals one-at-a-time light and a fresh
`solve_bulk`. With darkening switched off it fails on the first seed.

Core bench, release, interleaved base (`3b44336`) and new binaries, five
rounds each, the fastest criterion median per case (load 2.2-4.3):

| Case                      | Base one at a time | New one at a time | Batch 64 | Batch 4096 |
| ------------------------- | ------------------ | ----------------- | -------- | ---------- |
| place_box_16 (4096)       | 4.39 ms            | 4.81 ms           | 2.51 ms  | 1.35 ms    |
| remove_box_16 (4096)      | 5.53 ms            | 5.61 ms           | 2.87 ms  | 1.34 ms    |
| roof_16x16 (256 over sky) | 17.03 ms           | 16.72 ms          | 2.39 ms  | 1.42 ms    |

In Voxels a second: a 4096 batch places 3.0 M, removes 3.1 M and roofs 180 K,
against 0.93 M, 0.74 M and 15 K one at a time on the base.

A single relight is about 0.1 us (10%) slower on place_box: a first version
read each seed's light twice to bucket it and was 19% slower, so seeds now
carry the level they were found at. What is left is the per-call allocation of
the edit and level lists. After this branch only player Edits, undo and the
`dev` builders relight one Voxel at a time, at 16 a tick. The other core
benches call `light_chunk`, not `relight`, and their code is unchanged.
