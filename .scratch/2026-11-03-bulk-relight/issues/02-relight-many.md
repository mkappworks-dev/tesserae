# 02: `relight_many` in core

Status: ready-for-agent
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

- [ ] Every existing light test passes unchanged.
- [ ] A seeded property test over many random scenes (stone, emitters, shaped
      and translucent Voxels, edits that add and remove, across Chunk seams,
      with open sky above): `relight_many` of a batch gives exactly the light of
      the same edits applied one `relight` at a time, and of a fresh
      `solve_bulk` of the final scene.
- [ ] The bench from 01 has a `relight_many` case per batch size (1, 64, 4096)
      and the figures, against 01's, are in Comments.
- [ ] `cargo bench -p tesserae-core` shows no regression in the other benches
      (interleaved A/B, CPU time).

## Comments
