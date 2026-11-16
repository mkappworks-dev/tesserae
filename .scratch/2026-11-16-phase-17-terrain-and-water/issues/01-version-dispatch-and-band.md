# 01: Version dispatch and the declared band

Status: done

Spec: [Phase 17](../spec.md).

## What to build

One dispatch on Generator version for terrain height, caves and the water consumers, replacing `BIOMES_SINCE` as the only branch. The World declares `floor_m`, `ceiling_m` and `overhang_m`; the global surface bound, chunk early-outs, `above_the_surface` and the Structure band read them under version 3. New parameters are optional `Generation` snapshot fields. The save still refuses 3.

## Acceptance criteria

- [x] Versions 1 and 2 generate the recorded digests, bit for bit.
- [x] A test can generate under version 3 by passing it; the save refuses 3.
- [x] A save without the new fields reads as before.
- [x] No Voxel is solid above the version 3 bound; the deep early-out never declares a non-uniform Chunk uniform.
