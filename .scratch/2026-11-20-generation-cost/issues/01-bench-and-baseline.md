# 01: Crop fields in the bench, a warm-up that solves every region, and the starting figures

Status: done

Blocked by: None (can start immediately)

Spec: [Generation cost](../spec.md).

## What to build

The version 3 bench gains a World with crop fields like Terra's (one entry near the lake with a ditch on a 64 m lattice, one anywhere on a 96 m lattice) and cases over it: a straddling Chunk on a held field and a column of Chunks over one. Its warm-up solves every drainage region the case's Chunks and their neighbours within a tree's reach touch, so a lone Chunk's figure is generation, not a region solve. The uncommitted fingerprint gains a grid over the crop-field World, its baseline taken at this commit. The starting figures are recorded: the bench at this commit, the lone river case of this commit and of fcb5db58 (the last spec's starting commit) under the same new warm-up, the flight figures already in the spec's Further notes, and a sampled `flight` with the top symbols per level.

## Acceptance criteria

- [x] The bench runs the crop-field cases, and the lone river and cavern cases no longer include a region solve.
- [x] The starting bench figures and the sampled flight's top symbols, with `uptime` and how they were taken, are in Comments.
- [x] No generator code changes; the committed digests pass and the fingerprint's existing lines match 4e78a50f's.

## Comments

### The bench and the fingerprint, 2026-11-20

The bench's support World gains a twin with Terra's temperate fields (`crop_generator`): one entry near the lake with a ditch on a 64 m lattice, one anywhere on a 96 m lattice, three crops weighted 6, 3 and 2, planted in the forest and the plain as Terra plants its temperate forest and its grassland. Two cases run over it at each Voxel size: `straddling crop field`, the ground Chunk of the first column along the scan with a crop on an eighth of its columns, and `crop field column`, the band over that column. Both land in the plain, at `[-7, 5]` @ 1 m and `[4, -4]` @ 0.5 m. The existing World and cases are untouched.

The warm-up now asks the drainage at the corners of a 384 m square around each of a case's Chunks, which covers the neighbour Cells a tree's reach touches and a 96 m field cell giving way to the 64 m cells it crosses, each asking the lake 48 m from its field; the square is under half a 2 km region, so its corners hit every region it crosses. A comparison against another build matches cases by name, so a build from before the crop-field cases still compares.

Proof: the uncommitted fingerprint gains a `case` line for each crop-field case, a `grid crops` line at each size, and an assertion that each crop-field case holds crops; every line of the 4e78a50f baseline still matches. The committed version 3 digests (`generator_version`) and the tesserae-core tests pass. The figures are the parent's.

### Starting bench figures, 2026-11-20

`bench-gen01` (this commit) against `bench-fcb5`: the last spec's starting commit, fcb5db58, exported with `git archive` and given this commit's two bench files, so both builds run the same cases under the same warm-up. Each was built with `RUSTFLAGS="-D warnings" cargo bench --locked --no-run -p tesserae-core --bench worldgen_v3` in its own target dir and copied out; each round runs every case in a fresh subprocess of each binary, alternating which goes first; a case is thread CPU time per Chunk over 3 fresh generators; medians over 9 rounds, the ratio the median of the per-round ratios.

```sh
TESSERAE_BENCH_ROUNDS=9 TESSERAE_BENCH_AGAINST=$PWD/bench-fcb5 ./bench-gen01 --bench
```

The host was loaded: another worktree was building the `tesserae` crate throughout, `uptime` before `6:55 load averages: 11.14 10.82 8.00`, after `6:58 load averages: 13.32 11.98 8.97`. Thread CPU time interleaved keeps the ratios fair; the absolute figures run about twice the quiet ones of the last spec.

| Case | µs | fcb5db58 µs | Ratio |
| --- | --- | --- | --- |
| straddling forest @ 1 m (1 Chunks) | 18872.2 | 15275.2 | 108.9% |
| straddling dry @ 1 m (1 Chunks) | 12841.5 | 7535.8 | 198.7% |
| sky @ 1 m (1 Chunks) | 0.5 | 0.5 | 108.6% |
| deep rock @ 1 m (1 Chunks) | 2215.2 | 3018.1 | 72.3% |
| river @ 1 m (1 Chunks) | 9702.1 | 5189.7 | 175.0% |
| cavern @ 1 m (1 Chunks) | 11519.6 | 8308.6 | 143.6% |
| coarse level 3 @ 1 m (4 Chunks) | 1759.8 | 2860.5 | 60.4% |
| forest 3x3 columns @ 1 m (36 Chunks) | 2653.3 | 11528.0 | 23.0% |
| straddling crop field @ 1 m (1 Chunks) | 48002.4 | 35137.1 | 134.4% |
| crop field column @ 1 m (4 Chunks) | 17461.6 | 34027.7 | 50.5% |
| straddling forest @ 0.5 m (1 Chunks) | 7893.8 | 2483.9 | 336.9% |
| straddling dry @ 0.5 m (1 Chunks) | 7671.9 | 3250.8 | 231.9% |
| sky @ 0.5 m (1 Chunks) | 0.6 | 0.5 | 112.2% |
| deep rock @ 0.5 m (1 Chunks) | 2073.7 | 2843.0 | 72.8% |
| river @ 0.5 m (1 Chunks) | 6783.1 | 2742.0 | 234.9% |
| cavern @ 0.5 m (1 Chunks) | 9491.0 | 5431.2 | 176.1% |
| coarse level 3 @ 0.5 m (4 Chunks) | 1958.5 | 2700.9 | 72.2% |
| forest 3x3 columns @ 0.5 m (36 Chunks) | 1733.3 | 2467.0 | 66.5% |
| straddling crop field @ 0.5 m (1 Chunks) | 31086.1 | 4831.6 | 532.9% |
| crop field column @ 0.5 m (4 Chunks) | 9956.8 | 4712.1 | 213.3% |

Under one warm-up the lone river @ 0.5 m is 2.3 times its fcb5db58 cost, not 13: the rest of the last spec's figure was the old warm-up's region solve. A lone crop-field Chunk is the costliest case, 48 ms @ 1 m; over a column of four it is half fcb5db58's, which decided the fields for every Chunk alike.


### The starting sampled flight, 2026-11-20

Top symbols inside generation at 4e78a50f, from the sampled `flight` taken beside ticket 04's mid-point one (how, there):

Samples inside generation: level 0 8876, levels 1–5 7636.

| Symbol | Level 0 | Levels 1–5 | All |
| --- | --- | --- | --- |
| `Worldgen::fields_near` | 67.5% | 0.0% | 36.3% |
| `Worldgen::field_accepts` | 67.0% | 0.0% | 36.0% |
| `Drainage::wet` | 43.9% | 12.0% | 29.2% |
| `Worldgen::surface_height_m` | 45.9% | 1.5% | 25.3% |
| `Worldgen::contained_at_m` | 44.7% | 1.0% | 24.5% |
| `<&mut core::array::drain::Drain<[f32; 2], <core::ops::try_trait::NeverShortCircuit<(f32, core::option::Option<` | 20.4% | 0.5% | 11.2% |
| `tesserae_core::noise::fbm3_with` | 5.7% | 14.9% | 9.9% |
| `Base::base_height_m` | 13.9% | 4.7% | 9.6% |
| `OreField::at` | 3.5% | 16.3% | 9.4% |
| `tesserae_core::noise::fbm2` | 11.2% | 5.1% | 8.3% |
| `Worldgen::cavern_open` | 4.5% | 10.5% | 7.3% |
| `RandomState::hash_one` | 3.3% | 11.4% | 7.1% |
