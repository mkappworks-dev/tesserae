# 14: Benchmark, digests and accepting version 3

Status: ready-for-agent

Blocked by: 13

Spec: [Phase 17](../spec.md).

## What to build

Interleaved CPU-time A/B against `main`, recorded version 3 digests, and the bump: `GENERATOR_VERSION = 3`, the save accepts 1, 2 and 3, and the save tests that hard-code the set are updated.

## Acceptance criteria

- [ ] Surface Chunks within about 3x of version 2; early-outs unregressed.
- [x] Versions 1 and 2 digests unchanged.
- [x] A new World is version 3; a version 2 World opens as version 2.

- [ ] Flat-ground and overhang Chunks are within the benchmark budget (moved here from ticket 05).

## Comments

2026-11-17. `GENERATOR_VERSION` is 3. Version 3's digests are recorded over the
origin's box and a box over high ground, at 1 m and 0.5 m, for three seeds. Its
mountain and volcano arithmetic now runs on `libm`, so the terrain rounds alike
on every platform. Settlement roads still use the platform's `sin` and `cos` in
both versions, as before step 17.

Interleaved CPU-time A/B, thread clock, 10 rounds, load 4 to 10, µs per Chunk
(min / median), 48 Chunks a case, each round a fresh generator warmed on the
block beside:

| Case                     | main      | branch      |
| ------------------------ | --------- | ----------- |
| v2 sky early-out         | 0.006 / 0.007 | 0.008 / 0.010 |
| v2 deep early-out        | 0.010 / 0.010 | 0.010 / 0.010 |
| v2 surface               | 88.8 / 90.4 | 101.8 / 105.0 |
| v3 surface               |           | 951 / 953   |
| v3 rugged (overhangs)    |           | 868 / 875   |
| v3 surface, cold, apart  |           | 1352 / 1355 |

Early-outs hold. Version 2's surface Chunks are about 16% slower than on `main`
with identical output; it began at 3203954b, which built version 3's column
stacks, and looks like code layout rather than work. Version 3's surface Chunks
run about 9 times version 2's warm and 12 times cold, against the 3 times this
ticket asks: the samples are `fbm2` (45%), the drainage's `wet` (19%) and
`base_height_m` (15%), the per-column heightfield that
`tech/phase-17-worldgen-performance` targets. That branch's tickets 01 to 05 are
bit-identical and land after the bump unchanged; its gated tickets 06 (SIMD
noise) and 07 (cave lattice) must stay bit-identical too, now that version 3 is
frozen. The two performance boxes stay open for it.
