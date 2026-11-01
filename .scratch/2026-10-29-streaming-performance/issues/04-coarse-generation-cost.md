# 04: Profile and cut coarse generation at identical output

Status: done
Blocked by: 01

Spec: [streaming performance](../spec.md).
PR boundary: own branch `tech/streaming-coarse-generation`.

## What to build

- First, a breakdown of a coarse job per level: 2D height, 3D caves,
  downsample, the uniform early-outs. Also the share of coarse generation spent
  inside the region the finer level draws: `lod.rs`'s `dispatch` generates the
  whole load extent with no hole, which may be prefetch for when the Eye moves.
  Append both here.
- Build a cut only for a stage over 20% of coarse worker time, at identical
  output. Candidates:
  - level 1 only: generate its eight children as one 64-cubed pass in
    `tesserae-core` worldgen, with one smoothed height field instead of eight
    34-by-34 ones (about 13% fewer 2D samples) and one cave bound. Levels 2 and
    up run with `smoothing: None`, so they have no border overlap;
  - generating less inside the finer level's region, if the breakdown shows it
    is not needed as prefetch: a hole-size decision for you.
- A shared pass applies to untouched Chunks only. It excludes saved-change
  branches (ADR-0011's Persistent Worlds section) and level-1 jobs reading held,
  edited level-0 children (`generate_coarse`'s `held`).

## Acceptance criteria

- [x] Output is bit-equal to today's across several seeds and levels 1 to 3.
- [x] `tesserae-core` gains no Bevy, wgpu or rayon dependency.
- [x] Coarse worker CPU per Chunk passes the spec's effect gate, using the core
      comparison methodology.
- [x] The breakdown is recorded even if nothing is built.

## Validation and authorization

Local work only; no push or PR without approval.

## Comments

### 2026-10-31: breakdown, three cuts, per-Chunk numbers

Tool: `cargo bench -p tesserae-core --bench coarse_breakdown -- 3`. It
generates each level's whole load extent around the `teleport` fixture's
destination (Eye at 2000, 56.8, 0) with the demo's World (default seed, 1 m
Voxel, sea 24 m, relief 24 m, lake 34 m, caves and level-1 smoothing on) and
the demo's rings (view 160 m, vertical 96 m, hysteresis 32 m), on one thread.
Thread CPU, medians of 3 rounds inside each of 3 interleaved runs per binary.
Stages split by difference: caves = default minus `caves: None`, smoothing =
that minus `smoothing: None`, 2D height timed by sampling the same columns,
fill the remainder. The engine's per-job thread CPU in `frames.flow.csv` is
about 1.6 times this (shared workers, cold caches); shares are what count.

Before (`1eea57a`), ms of thread CPU per level's load extent:

| Level | Jobs | Early-outs | Carved rock | Height | Caves | Smooth | Fill | Downsample | Total | us/job |
| ----- | ---- | ---------- | ----------- | ------ | ----- | ------ | ---- | ---------- | ----- | ------ |
| 1     | 1639 | 0.05       | 69.0        | 24.9   | 105.9 | 23.9   | 104.8 | 92.0      | 421.1 | 257    |
| 2     | 1639 | 0.06       | 73.9        | 11.0   | 101.1 | 0      | 74.4 | 74.4       | 335.0 | 204    |
| 3     | 1639 | 0.06       | 74.2        | 11.0   | 96.1  | 0      | 38.1 | 65.0       | 284.6 | 174    |
| 4     | 1639 | 0.06       | 53.3        | 11.0   | 59.7  | 0      | 28.4 | 60.9       | 213.7 | 130    |
| 5     | 1243 | 0.04       | 59.0        | 8.4    | 35.2  | 0      | 18.1 | 43.5       | 164.5 | 132    |

- Over all levels: caves (in surface Chunks and carving rock) 51%,
  downsample 24%, fill 19% (25% at level 1), 2D height 5%, smoothing 2%. The
  uniform early-outs (air above the band, rock below the caves) cost nothing
  measurable: 11,300 of a level's 13,112 children take them.
- 1,355 of level 1's 1,639 jobs have eight identical uniform children and skip
  the downsample; the other 284 pay about 325 us each for it.
- The ticket's level-1 candidate, one 64-cubed pass with one height field,
  cuts the 2D height, which is 6% of level 1 and 5% overall: under the 20%
  bar, not built.

Where it lies (shares of CPU, unchanged by the cuts):

- Inside the finer level's region, every child within the finer level's mesh
  extent: 45 jobs per level, 8 to 10% of levels 1 to 4 and 13% of level 5.
  The mesh hole itself is only (0, 1), three Cells, at every level with these
  rings; the other 42 are meshed here and drawn under the finer level by 02's
  draw-until-replaced rule.
- Outside this level's mesh shell, the hysteresis ring and the neighbour ring
  the load extent adds: 1,072 of 1,639 jobs, 46 to 47% of levels 1 to 4 and
  56% of level 5. That is the "two thirds" 01 found; it is margin around the
  shell, not the region level 0 draws.

Cuts, each bit-equal (`crates/tesserae-core/tests/coarse_generation.rs`:
digests of every child and coarse Chunk, three seeds, levels 1 to 3, recorded
before any cut; plus unit tests against the uncached field and `Chunk::set`):

1. Caves (`89bec72`): `fbm3_cached` reuses a lattice cell's eight corner
   hashes across neighbouring samples, through `noise3`'s own interpolation.
2. Downsample (`9cd198a`): reads the four rows a coarse row covers at once; a
   block of eight identical Voxels skips the tally.
3. Fill (`f0edb21`): `ChunkBuilder` records the fill's writes and packs once,
   instead of `set`'s per-write index arithmetic and two or three repacks as
   the palette grows.

After, same runs:

| Level | Carved rock | Caves | Fill | Downsample | Total (runs)        | us/job | Change |
| ----- | ----------- | ----- | ---- | ---------- | ------------------- | ------ | ------ |
| 1     | 38.6        | 53.4  | 38.3 | 60.3       | 239.4, 239.6, 243.8 | 146    | -43%   |
| 2     | 45.3        | 59.4  | 22.1 | 48.5       | 186.4, 186.5, 188.4 | 114    | -44%   |
| 3     | 49.4        | 61.3  | 14.0 | 41.6       | 177.2, 177.4, 177.6 | 108    | -38%   |
| 4     | 42.7        | 43.7  | 9.9  | 40.0       | 147.4, 147.5, 147.7 | 90     | -31%   |
| 5     | 57.9        | 32.3  | 6.1  | 29.2       | 133.4, 133.8, 134.1 | 108    | -19%   |

Base runs were 419.3 to 421.9 ms at level 1 down to 164.0 to 164.7 at level 5:
no range overlaps. Level 0 generation shares the cave and fill cuts. The
frame and streaming effect is pending the combined review series.

- Caves remain the largest stage, about 38% of what is left at level 1. The
  carved-rock path at level 5 did not move: at 16 m Voxels a sample rarely
  shares a cell, and the sub-block bound dominates. The next candidate.
- For you, if coarse generation matters again: generating less where the
  finer level draws would save at most 8 to 13% per level, and needs those 42
  shell Cells not to be drawn under the finer level, against 02's rule. The
  margin is the larger lever, 46 to 56%: it is the keep ring (one Cell of
  hysteresis) and the neighbour ring, so it trades against eviction churn and
  seam meshing. Neither is built.
