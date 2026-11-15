# 05: Trees, scatter and fields keep off a Structure

Status: done

Blocked by: 03

Spec: [Phase 20](../spec.md).

## What to build

Trees, surface scatter and crop fields skip every column under a Structure's
footprint rectangle, after rotation. A Structure's acceptance never asks about
them, so the two never form a cycle.

## Acceptance criteria

- [x] No tree, scatter Voxel or field cell stands in a Structure's footprint.
- [x] A Biome with Structures and trees is whole and order-independent across
      Chunk borders.
- [x] A World with no Structure rows generates the same trees, scatter and
      fields as before, by recorded digest.

## Comments

- Trees, scatter and fields ask `Worldgen::footprints_near(region)` (the
  footprints of `structures_near`) once per Chunk, over the Chunk widened by
  twice the widest Plant reach: a tree a margin away reaches a margin further.
  `covers(kept, gx, gz)` is the per-column test; with no Structure near, the
  list is empty and `structures_near` returns at once without a pass.
- Scatter and field cells skip a covered column; the rest of a field stands,
  so a Structure can stand on part of a field. Field acceptance does not ask
  about Structures (it still asks `plant_raw` about trees, unchanged).
- A tree or vine Plant with any Voxel (canopy, hung vines, fruit beside a
  vine) in a covered column is left out whole, never cut, so every Chunk
  decides alike. The cells are computed only when the base is within one reach
  of a footprint.
- `clear_height_m` and `plants_clear_m` skip the same columns and Plants, so a
  spawn height agrees with what generates. They do not yet count a
  Structure's own Voxels (08 or 07 if needed).
- No Structure rows: no footprint, nothing changes; the digests in
  `surface_features.rs` and `fields.rs` pass untouched. No new Generator
  version.
- No edit to `worldgen/structures.rs`. Tests: `tests/structure_keep_off.rs`.
