# 06: SIMD noise, if noise is still worth it

Status: done

Blocked by: 03, 04, 05

Spec: [Worldgen performance](../spec.md).

## What to build

First a re-measure: noise's share of generation in a sampled flight once tickets 02 to 05 have landed. At 15% or less, this ticket is closed as `wontfix` with the figure. Otherwise the 2D and 3D fBm gain a four-wide path through the `wide` crate, used where a caller already walks a row (building the column cache, the drainage window, the overhang push down a column), giving the scalar result bit for bit: the same operation order, octave sums per lane, and no fused multiply-add.

## Acceptance criteria

- [x] The re-measured share is in Comments, whichever way the gate goes.
- [x] If built: a property test over many seeds and positions shows the four-wide result equals the scalar one bit for bit, on every CI architecture.
- [x] If built: version 3 Chunks match the ticket 01 fingerprint; the committed version 3 digests pass unchanged.
- [x] If built: bench figures before and after in Comments.

## Comments

### The gate, a sampled flight, 2026-11-19

Noise is 21.0% of generation in a sampled flight of the end build, above the 15% gate in every one of six runs (20.1% to 22.3%): 17.1% at level 0 and 25.3% at the coarse levels. The figure is a lower bound, since inlined noise counts to its callers. **Decision: the gate passes; the ticket stays `ready-for-agent`.** Not built here.

How it was taken: the end build of ticket 05's flight profile, ac49c2f3 (`tesserae-end`, built with `RUSTFLAGS="-D warnings" cargo build --locked -p tesserae --features profiling --release`), flew the `flight` fixture over Terra in one `scripts/profile-frames` series of six runs (three repeats, logging on and off; every run passed the runner's audit). A watcher found each run's game process as it started and, 19.5 s later, took a macOS `sample` of it for 12 s: six samples, each a separate game process, covering 19.6 to 31.6 s after launch against a fixture window of 22.0 to 32.0 s. Nothing else built or ran a `tesserae` process meanwhile. Run from the worktree, with `$SCRATCH` a scratch folder:

```sh
scripts/profile-frames --binary $SCRATCH/tesserae-end --profile release \
  --features profiling --fixtures flight --repeats 3 $SCRATCH/runs/smp
# for each game process, 19.5 s after it appears:
sample <pid> 12 1 -mayDie -file $SCRATCH/smp/s<n>-<pid>.txt
rustfilt -i s<n>-<pid>.txt -o s<n>-<pid>.dm
```

`uptime` before the series `18:33 load averages: 1.59 4.07 5.24`, after it `18:36 load averages: 6.25 5.03 5.38`; at each sample from `2.56 4.13 5.23` to `6.10 4.95 5.36`.

The samples' call trees were summed by symbol. Generation is every sample on a worker thread at or below `Worldgen::chunk`; a level-0 Chunk comes from `streaming::dispatch_generation`, a coarse one from `lod::generate_coarse`, and the stack does not say which coarse level. About 4,000 samples per thread per run; inside generation:

| Run | Level 0 | Coarse (1–5) | All | Noise | 3D density |
|---|---|---|---|---|---|
| 1 | 1457 | 1343 | 2800 | 20.5% | 10.4% |
| 2 | 1517 | 1359 | 2876 | 20.1% | 9.4% |
| 3 | 1556 | 1388 | 2944 | 20.2% | 9.2% |
| 4 | 1527 | 1333 | 2860 | 21.7% | 10.6% |
| 5 | 1445 | 1361 | 2806 | 21.3% | 10.5% |
| 6 | 1465 | 1346 | 2811 | 22.3% | 11.4% |
| Pooled | 8967 | 8130 | 17097 | 21.0% | 10.2% |

| Share of generation | Level 0 | Coarse (1–5) | All |
|---|---|---|---|
| Noise | 17.1% | 25.3% | 21.0% |
| 3D density | 6.1% | 14.8% | 10.2% |

- Noise: the frames of `noise::fbm2`, `noise::fbm3_with` (`fbm3`) and `noise::fbm3_range`, inclusive, plus `CaveField::carves_near`, into which `fbm3_cached` is inlined. Every noise function is `#[inline]`, and an inlined call (`warp2` in `Base::warped` and the drainage meander, an inlined `fbm2`) counts to its caller, so this is a lower bound.
- 3D density: `fbm3` under the overhang push (inlined into `Worldgen::chunk`) and the cavern field (inlined into `cavern_open` and `chunk`), `fbm3_range` (the cavern and tunnel bounds), and every `CaveField` frame (the two tunnel fields), inclusive. `fbm3` under `underground::cave_biome_at`, the cave Biome blend, is noise but not density. Nearly all of the density is noise, so it is inside the noise figure too.

Top ten inclusive symbols inside generation, pooled (iterator adapters, `CellMap` wrappers and unwinder `???` frames left out):

| Symbol | Level 0 | Coarse | All |
|---|---|---|---|
| `Worldgen::fields_near` | 70.8% | 0.0% | 37.1% |
| `Worldgen::field_accepts` | 70.4% | 0.0% | 36.9% |
| `Drainage::wet` | 45.5% | 12.6% | 29.9% |
| `Worldgen::surface_height_m` | 48.0% | 1.9% | 26.1% |
| `Worldgen::contained_at_m` | 46.8% | 1.4% | 25.2% |
| `Base::base_height_m` | 14.1% | 8.7% | 11.5% |
| `noise::fbm3_with` (`fbm3`) | 4.9% | 14.6% | 9.5% |
| `OreField::at` (with `column_biome`) | 3.7% | 15.8% | 9.5% |
| `noise::fbm2` | 10.8% | 7.9% | 9.4% |
| `Worldgen::cavern_open` | 4.2% | 9.7% | 6.8% |

Level 0 is now mostly the crop fields' and Structure claims' `field_accepts`, through `surface_height_m` into the drainage (`contained_at_m`, then `Drainage::wet`, whose own code is 40% of level 0); noise is not what it waits on. That is outside this spec's tickets.

### Built, 2026-11-19

The 2D and 3D fBm have a four-wide form on `wide` 1.7.1 (pinned exactly): the same operations in the same order, each lane summing its own octaves, `floor` (exact on every backend) and `trunc_int` (saturating like `as`), and never `mul_add`. The scalar functions are kept for single points. Batched where a caller already walks a row:

- The column cache: each row's base heights (the warp and every field `base_height_m` always asks, four columns at once; the rest of its arithmetic, and the noise it asks only on some columns, stays per column) and the overhang reach over the Cell's columns.
- The drainage window: a region's solve asks its base heights four nodes at a time.
- The overhang push down a column: each Voxel the density moves, and the one above the last, four heights at a time, before the Voxel walk. A height the walk asks that is not bit for bit one sampled (a Voxel size that is not a power of two) is sampled alone.

`wide`'s `splat` compiles on macOS to a call to `memset_pattern16` per vector; with it, the first build ran 24% to 94% slower than 05. The noise builds its vectors from four-element arrays instead.

Proof:

- Property tests, in `noise.rs`: 20,000 four-lane draws each of `fbm2` and
  `fbm3` against the scalar, bit for bit, over hashed seeds, 0 to 10 octaves,
  and positions from a few metres to 1,000 km, on and just beside lattice points
  either side of zero. In `terrain.rs`: 24 seeds of 400 four-column draws out to
  100 km, the base height and the overhang reach against the scalar, bit for
  bit. The Determinism workflow runs all three in release on every target it
  digests: x86_64 Linux and Windows (SSE2), aarch64 macOS (NEON) and x86_64
  macOS under Rosetta; they pass on aarch64 here. A test that changes the order
  of the smoothstep's multiplications fails both.
- Version 3's fingerprint (`tests/v3_fingerprint.rs`, uncommitted) equals fcb5db58's, all 46 lines; the committed digests, `cargo test --locked -p tesserae-core` and the Server's LOD tests pass. `GENERATOR_VERSION` is unchanged.

### Bench figures, 2026-11-19

Built with `RUSTFLAGS="-D warnings" cargo bench --locked --no-run -p tesserae-core --bench worldgen_v3` and copied out as `bench-06`; all building, the fingerprint and the tests finished before timing, and nothing else built during the run. Method as in ticket 05: thread CPU time per Chunk over 3 fresh generators, medians over 9 interleaved rounds, the ratio the median of the per-round ratios.

`bench-06` against `bench-05` (ac49c2f3), `uptime` before `19:12  up 7 days, 18:11, 1 user, load averages: 4.03 4.10 3.73`, after `19:13  up 7 days, 18:11, 1 user, load averages: 3.97 4.06 3.72`:

```sh
TESSERAE_BENCH_ROUNDS=9 TESSERAE_BENCH_AGAINST=$PWD/bench-05 ./bench-06 --bench
```

| Case | µs | 05 µs | Ratio |
|---|---|---|---|
| straddling forest @ 1 m (1 Chunks) | 7895.2 | 9442.0 | 83.8% |
| straddling dry @ 1 m (1 Chunks) | 5937.2 | 7107.8 | 83.8% |
| sky @ 1 m (1 Chunks) | 0.2 | 0.2 | 93.3% |
| deep rock @ 1 m (1 Chunks) | 1063.5 | 1179.3 | 90.4% |
| river @ 1 m (1 Chunks) | 11761.0 | 16270.6 | 72.3% |
| cavern @ 1 m (1 Chunks) | 13132.6 | 17767.7 | 73.9% |
| coarse level 3 @ 1 m (4 Chunks) | 2733.1 | 3650.9 | 74.8% |
| forest 3x3 columns @ 1 m (36 Chunks) | 1233.1 | 1329.3 | 92.8% |
| straddling forest @ 0.5 m (1 Chunks) | 3384.5 | 4569.1 | 73.6% |
| straddling dry @ 0.5 m (1 Chunks) | 3609.4 | 4789.2 | 75.8% |
| sky @ 0.5 m (1 Chunks) | 0.2 | 0.2 | 100.0% |
| deep rock @ 0.5 m (1 Chunks) | 934.1 | 1059.4 | 88.2% |
| river @ 0.5 m (1 Chunks) | 10496.4 | 15065.1 | 69.8% |
| cavern @ 0.5 m (1 Chunks) | 11383.2 | 15895.5 | 71.7% |
| coarse level 3 @ 0.5 m (4 Chunks) | 2757.0 | 3633.7 | 76.2% |
| forest 3x3 columns @ 0.5 m (36 Chunks) | 737.2 | 821.1 | 89.7% |

Every case but sky (timer noise at 0.2 µs) is 7% to 30% cheaper; the most where a Cell's columns and a region's drainage are built cold (river, cavern, coarse level 3), the least where a warm column cache serves many Chunks (the 3x3 column of 36).

### Flight effect, 2026-11-19

None beyond the spread between runs: the final build against the end, interleaved, is 186.3 against 176.7 ms/s at level 0, 315.3 against 318.9 at the coarse levels and 501.6 against 495.8 in all ([ticket 07's Comments](./07-coarse-cave-lattice.md#the-gate-after-simd-and-the-final-flight-profile-2026-11-19)).
