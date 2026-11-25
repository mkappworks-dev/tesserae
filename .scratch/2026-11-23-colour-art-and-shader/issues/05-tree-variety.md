# 05: Tree variety

Status: done
Blocked by: 01

Spec: [Colour, art and light](../spec.md).

## What to build

Solution item 5, two steps, each on captures the user approves:

1. Storeys and spacing: optional weighted `storeys` on a tree row, picked per
   tree by the World seed and its root position in metres; the taller storey
   keeps a contested Voxel, whichever Chunk generates first. `storeys` and a
   `canopy` list extend the unreleased `0.3` tree record in place, and the
   data-only TOML beside it. Jungle, swamp and oak values and Terra's jungle and
   swamp spacing from the spec's Decisions. Record v3's digests again.
2. Canopy variants: a tree row's `canopy` takes a list, picked per tree the
   same way. Three colour-only variants each for jungle, oak, swamp, cherry,
   orange and apple in `flora`, derived from 01's retuned leaf colours.

## Acceptance

- [x] A tree row with one canopy and no storeys generates exactly as before
      (test).
- [x] Storey and variant choice deterministic per seed and position, and the
      same at another Voxel size (tests).
- [x] v3 digests recorded again in the commit that changes them.
- [x] `jungle-top-medium`, `jungle-ground-close`, `jungle-ground-110m`,
      `swamp-ground` and `swamp-forest-ground` show separate trees; approved.
- [x] Impostors match level 0's storeys and variants; Canopy cover picks one
      variant per crown-sized cell (Q77); the LOD agreement tests pass.
- [x] Coarse levels keep one variant per crown, not speckle.

## Comments

2026-11-25: review findings and Q76, Q77.

- Where crowns meet, only storeyed trees stand taller storey first. Trees
  without storeys rank below them and equal among themselves, so they keep
  `plants_on`'s fixed order, as on `0b03c594`. Test:
  `two_species_without_storeys_make_the_chunks_they_made_before_storeys`,
  with digests recorded on `0b03c594`; it fails on the height-sorted order.
  v3's recorded digests did not change.
- Q76: the oak's trunk is 8 to 14 m. At 8 m its limbs dipped to 3 m, under
  the 4 m headroom test, so they start at 0.65 of the trunk instead of 0.6.
- Q77: Canopy cover keeps one variant per crown-sized cell; the spec says so.
- `SurfaceTree`'s `storeys` and `variants` indices are `storeys_index` and
  `canopies_index`.
