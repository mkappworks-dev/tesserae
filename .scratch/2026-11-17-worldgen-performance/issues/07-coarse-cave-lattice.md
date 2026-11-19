# 07: A coarse lattice for the cavern and tunnel fields, if the density is still worth it

Status: ready-for-human

Blocked by: 06

Spec: [Worldgen performance](../spec.md).

## What to build

First a re-measure: the 3D density's share of generation once ticket 06 is settled. At 10% or less, this ticket is closed as `wontfix` with the figure. Otherwise the cavern field and the two tunnel fields are sampled on a 4 m lattice anchored at the World origin in metres and interpolated between samples, each field before it is thresholded, so it holds at every Voxel size; the range bounds that rule boxes out widen by a few ulps. The overhang push stays exact per Voxel. Cave shapes would move, and version 3 is frozen, so a passing gate does not build it here: the ticket goes to `ready-for-human` with the figure, for a decision on a new Generator version. Whichever way the gate goes, the last flight profile is recorded here.

## Acceptance criteria

- [x] The re-measured share and the final flight profile, against the ticket 01 one, are in Comments.
- [ ] If built: generated water has no air below or beside it, flooded caverns stay sealed from tunnels, and no overhang reaches water or a Structure, at 1.0 and 0.5 m.
- [ ] If built: the same metre point is carved at every Voxel size, and a box the bound rules out holds no carve.
- [ ] If built (only under a new Generator version): only cave Voxels differ from the ticket 01 fingerprint.

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
