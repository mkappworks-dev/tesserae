# 07: A coarse lattice for the cavern and tunnel fields, if the density is still worth it

Status: wontfix

Blocked by: 06

Spec: [Worldgen performance](../spec.md).

## What to build

First a re-measure: the 3D density's share of generation once ticket 06 is settled. At 10% or less, this ticket is closed as `wontfix` with the figure. Otherwise the cavern field and the two tunnel fields are sampled on a 4 m lattice anchored at the World origin in metres and interpolated between samples, each field before it is thresholded, so it holds at every Voxel size; the range bounds that rule boxes out widen by a few ulps. The overhang push stays exact per Voxel. Cave shapes would move, and version 3 is frozen, so a passing gate does not build it here: the ticket goes to `ready-for-human` with the figure, for a decision on a new Generator version. Whichever way the gate goes, the last flight profile is recorded here.

## Acceptance criteria

- [x] The re-measured share and the final flight profile, against the ticket 01 one, are in Comments.
- [ ] ~~If built: generated water has no air below or beside it, flooded caverns stay sealed from tunnels, and no overhang reaches water or a Structure, at 1.0 and 0.5 m.~~
- [ ] ~~If built: the same metre point is carved at every Voxel size, and a box the bound rules out holds no carve.~~
- [ ] ~~If built (only under a new Generator version): only cave Voxels differ from the ticket 01 fingerprint.~~

## Comments

### The gate, a sampled flight, 2026-11-19

The 3D density (the overhang push, the cavern field and the two tunnel fields) is 10.2% of generation in a sampled flight of the end build, just above the 10% gate: 9.2% to 11.4% over six runs, 6.1% at level 0 and 14.8% at the coarse levels. It is measured before ticket 06, which the gate is meant to follow; almost all of the density is `fbm3`, so SIMD noise would bring it down, likely under 10%. **Decision: `ready-for-human`.** Building it moves cave shapes, which frozen version 3 does not allow, so it waits on a decision for a new Generator version, and that decision should take a re-measure after ticket 06.

The final flight profile, against ticket 01's, is in [ticket 05's Comments](./05-coarse-sources-share-level-0.md#end-flight-profile-2026-11-19): it is the same end build, ac49c2f3.

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

### The gate after SIMD, and the final flight profile, 2026-11-19

The 3D density is 9.8% of generation in a sampled flight of the final build, 50aaeeec, after ticket 06: 9.2% to 10.5% over six runs, 5.4% at level 0 and 14.7% at the coarse levels. **Decision: `wontfix`**, at the 10% gate. SIMD did not move it: the end build, sampled again the same way just after, gives 10.2%, the figure of the first gate. Ticket 06 batched the 2D fBm and the overhang push; the cavern and tunnel fields, almost all of the density, still ask `fbm3` one point at a time, and the margin under the gate is inside the spread between runs.

| Share of generation | Level 0 | Coarse (1–5) | All | End, again |
|---|---|---|---|---|
| Noise | 19.5% | 24.2% | 21.8% | 21.4% |
| 3D density | 5.4% | 14.7% | 9.8% | 10.2% |

| Run | Level 0 | Coarse (1–5) | All | Noise | 3D density |
|---|---|---|---|---|---|
| 1 | 1532 | 1341 | 2873 | 22.6% | 10.5% |
| 2 | 1482 | 1387 | 2869 | 21.2% | 9.2% |
| 3 | 1520 | 1383 | 2903 | 22.4% | 9.9% |
| 4 | 1546 | 1395 | 2941 | 20.9% | 9.6% |
| 5 | 1530 | 1413 | 2943 | 20.9% | 9.5% |
| 6 | 1517 | 1379 | 2896 | 22.5% | 10.4% |
| Pooled | 9127 | 8298 | 17425 | 21.8% | 9.8% |

The density, pooled: the cavern field's `fbm3` under `cavern_open` 5.8%, `fbm3` called from `Worldgen::chunk` 2.2%, the tunnel fields' `CaveField` frames 1.8%, `fbm3_range` 0.1%. The four-wide overhang push (`ColumnPush`, `fbm3_lanes`) is inlined into `Worldgen::chunk` and has no frame of its own, so it is not counted; the scalar `fbm3` calls from `chunk` it replaced are as many as before (376 samples in each build), so it was next to nothing in this flight. Noise now also counts `noise::warp2_lanes::<f32>`, a frame of its own in this build (2.1%) that was inlined before; the four-wide noise inlined into `Base::base_height_x4` and the column cache is not counted, so noise is still a lower bound.

Top ten inclusive symbols inside generation, pooled, left out as before:

| Symbol | Level 0 | Coarse | All |
|---|---|---|---|
| `Worldgen::fields_near` | 70.2% | 0.0% | 36.8% |
| `Worldgen::field_accepts` | 69.6% | 0.0% | 36.5% |
| `Drainage::wet` | 45.0% | 12.2% | 29.4% |
| `Worldgen::surface_height_m` | 46.6% | 1.7% | 25.2% |
| `Worldgen::contained_at_m` | 45.5% | 1.3% | 24.4% |
| `Base::base_height_m` | 13.7% | 5.0% | 9.6% |
| `OreField::at` (with `column_biome`) | 4.0% | 15.3% | 9.4% |
| `noise::fbm3_with` (`fbm3`) | 4.2% | 14.9% | 9.3% |
| `noise::fbm2` | 11.1% | 5.5% | 8.4% |
| `RandomState::hash_one` (`[i32; 3]` keys) | 3.1% | 11.5% | 7.1% |

The final flight profile, against ticket 01's start (fcb5db58) and ticket 05's end (ac49c2f3); the end was rebuilt and flown again here, interleaved with the final build. Worker CPU, ms per second of flight:

| Level | Stage | Start | End (05) | End, again | Final | Final / start | Final / end, again |
|---|---|---|---|---|---|---|---|
| 0 | generate | 988.9 | 188.0 | 176.7 (170.2–185.8) | 186.3 (166.2–195.7) | 19% | 105% |
| 0 | relight | 77.6 | 94.7 | 94.5 (88.3–97.9) | 97.0 (90.4–108.4) | 125% | 103% |
| 0 | mesh | 19.5 | 27.6 | 27.2 (24.9–29.5) | 27.5 (25.4–30.6) | 141% | 101% |
| 1 | generate | 284.1 | 259.6 | 256.4 (250.6–264.0) | 255.1 (247.8–287.1) | 90% | 99% |
| 1 | mesh | 194.0 | 273.8 | 281.8 (257.1–295.3) | 276.8 (263.0–313.0) | 143% | 98% |
| 2 | generate | 63.5 | 50.6 | 50.2 (48.6–53.1) | 50.9 (48.4–56.1) | 80% | 101% |
| 3 | generate | 10.0 | 8.9 | 8.8 (8.4–9.8) | 9.0 (8.6–10.0) | 90% | 102% |
| Coarse (1–5) | generate | 357.6 | 318.8 | 318.9 (308.7–324.7) | 315.3 (304.8–352.8) | 88% | 99% |
| All levels | generate | 1350.2 | 510.0 | 495.8 (481.9–500.2) | 501.6 (475.8–548.5) | 37% | 101% |

Level 0's fluid work is 21.8 at the start, 33.4 for the end and 33.9 for the final build. Per Chunk, final against start: level 1 2.55 against 3.57 ms, level 2 3.09 against 3.80, level 3 0.55 against 0.60; the end, again, is 2.57, 3.04 and 0.53. Across the spec, generation in a real flight is 63% cheaper, level 0 81% and the coarse levels 12%. Ticket 06 shows no flight effect beyond the spread: at level 0, rounds 1 and 2 overlap (final 166.2 to 187.1 ms/s, end 170.2 to 182.1), and the final build's round 3 runs highest (189.8 to 195.7).

How they were taken: `tesserae-final` (50aaeeec) and `tesserae-end` (ac49c2f3, after `git switch --detach ac49c2f3`) were built with `RUSTFLAGS="-D warnings" cargo build --locked -p tesserae --features profiling --release` and copied out of the target dir into `$SCRATCH`; both builds finished before any timing. The flight profile is three rounds, interleaved: end, final; final, end; end, final, one series each, as in ticket 05:

```sh
scripts/profile-frames --binary $SCRATCH/tesserae-<point> --profile release \
  --features profiling --fixtures flight --repeats 3 $SCRATCH/runs/r<round>-<point>
```

Before each series the driver checked `uptime` and waited for no `cargo`, `rustc` or `tesserae` process and no runner lock; it never had to wait. Every run passed the runner's audit. Figures are the median over 9 logging-on runs, with the range in brackets. `uptime` from `19:18 load averages: 6.60 4.71 3.99` before the first series to `19:38 load averages: 6.54 6.38 5.77` after the last; the highest one-minute load was 6.74.

The samples, as for the first gate: one series of `tesserae-final`, then one of `tesserae-end`, each game process sampled 19.5 s after it appeared, for 12 s:

```sh
scripts/profile-frames --binary $SCRATCH/tesserae-<point> --profile release \
  --features profiling --fixtures flight --repeats 3 $SCRATCH/runs/smp-<point>
# for each game process, 19.5 s after it appears:
sample <pid> 12 1 -mayDie -file $SCRATCH/smp/<point>-s<n>-<pid>.txt
rustfilt -i <point>-s<n>-<pid>.txt -o <point>-s<n>-<pid>.dm
```

`uptime` before the final series `19:39 load averages: 4.43 5.88 5.60`, after the end series `19:45 load averages: 5.70 6.04 5.80`; at each sample from `4.82 5.87 5.61` to `7.28 6.24 5.77`. The end samples reproduce the first gate's figures (3D density 10.2% against 10.2%, noise 21.4% against 21.0%, every top-ten symbol within a point of it), so the two gates compare.
