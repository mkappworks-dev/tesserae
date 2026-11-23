# 05: Tree variety

Status: ready-for-agent
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

- [ ] A tree row with one canopy and no storeys generates exactly as before
      (test).
- [ ] Storey and variant choice deterministic per seed and position, and the
      same at another Voxel size (tests).
- [ ] v3 digests recorded again in the commit that changes them.
- [ ] `jungle-top-medium`, `jungle-ground-close`, `jungle-ground-110m`,
      `swamp-ground` and `swamp-forest-ground` show separate trees; approved.
- [ ] Impostors match level 0's storeys and variants; Canopy cover picks one
      variant per coarse Voxel; the LOD agreement tests pass.
- [ ] Coarse levels keep one variant per crown, not speckle.
