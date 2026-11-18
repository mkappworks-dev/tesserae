# 14: Investigate two version 3 slowdowns

Status: done

Spec: [Generator determinism](../spec.md). The worldgen version 3 performance
branch lands after this one and takes its causes as input.

Blocked by: None (can start immediately).

## What to build

Two slowdowns ticket 02 found when its benches moved to version 3 (see its
Comments) are explained, and fixed where the fix leaves generator output
bit-identical:

1. `lod/coarse_chunk/air_level_1` no longer takes its early-out (416 ns before,
   129 µs after).
2. In the `structures` bench a fresh generator costs about eight times a warm
   one per Chunk (451 µs against 3490 µs, bare).

## Acceptance criteria

- [x] The cause of each is written in a comment here.
- [x] Any fix moves no recorded digest.
- [x] Figures are taken as interleaved CPU-time A/B against the integration
      branch, with the host load beside them.

## Comments

Neither is a regression in the generator. No generation code changed; one bench row moved, so no digest or spawn position can.

1. **`air_level_1` lost its early-out because the bench's row now sits inside the band.** It takes the coarse row four cells (256 m) over the origin's ground, and the sky early-out in `Worldgen::chunk` keys off the global band top, not the column's. Version 2's band was its relief over Sea level, so that row cleared it; version 3's band reaches 228 m (Sea level, `height_m` 192, `overhang_m` 12 and the Plants' rise), while the ground at the bench's seed is -35 m, so the row's children (floors 192 m and 224 m) fall inside it and each runs the full column pass, about 12.5 µs warm. The bench now takes the first coarse row above `surface_bounds_m`, as ticket 02 did for `meshing`'s early-outs.
2. **A fresh version 3 generator solves its drainage regions before its first Chunk.** `Terrain` owns a `Drainage` whose regions (and the coarse solve that seeds their edges) are solved on first use and cached per generator, and every Chunk column builds a `Stack` on first use; version 2 had neither, so cold and warm cost the same there. Per Chunk of the 9-Chunk ring, about 3.3 ms is the regions and 0.45 ms the Stacks. A warm generator, which is what streaming runs, pays neither again.

Figures: interleaved CPU-time medians in one throwaway harness (101 rounds for the LOD rows, 31 for the ring), the generator identical to the integration branch at 87e277e2, host load 6.9:

| Measure | Figure |
| --- | --- |
| `air_level_1`, old row (3) | 94.4 µs |
| `air_level_1`, new row (4) | 67.1 ns |
| bare 1 m ring, per Chunk, warm | 440.2 µs |
| bare 1 m ring, per Chunk, cold | 4223.0 µs |
| bare 1 m ring, per Chunk, regions solved, Stacks cold | 894.9 µs |

Criterion `--quick` on the moved bench, load 6.3: `air_level_1` 60.4 ns, `rock_level_1` 79.0 ns.

Not fixed, each for its own ticket:

- A sky Chunk between a column's top and the band's top runs the full pass for air. An early-out on the Stack's top would need its own proof that nothing (water, a Plant, a one-Voxel surface feature, a Structure) stands there.
- The Server's level 0 and each LOD source are separate generators over the same `Base`, so each solves the same drainage regions; sharing one `Drainage` between them would save those solves without changing what any returns.
