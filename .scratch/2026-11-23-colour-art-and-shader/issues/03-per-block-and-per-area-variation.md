# 03: Per-block and per-area variation

Status: done

Spec: [Colour, art and light](../spec.md).

## What to build

Solution item 3, in the chunk shader, under one opt-in flag on a Material's
appearance. The flag extends the unreleased `0.3` `appearance` record in place,
and the data-only TOML beside it; unset, a Material draws as today.

1. Rotate or mirror the tile per Voxel, keyed on its world position; the
   sampling gradients rotate with the UV. Terra sets the flag on its leaves,
   stone, dirt, sand and grass top; `flora` on its leaves; `strata` on its
   rock and sand.
2. On leaves and grass top only, a crown-sized colour variation in metres:
   patches about 5 m across shifting value by about 8% and hue by a few degrees.

Judged after 01's period fixes land, since a pattern with a direction shows
through any rotation.

## Acceptance

- [x] A Material without the flag draws exactly as before (test).
- [x] Merged quads rotate per Voxel, not per quad; no seam or mip jump at Voxel
      edges up close.
- [ ] ~~The variation is the same at another Voxel size (in metres).~~
- [x] `jungle-top-medium`, `alpine-far` and the 110 m views show no repeat every
      metre and a broken grass and canopy carpet; approved.
- [ ] ~~`flight` GPU frame no worse than `main` beyond noise.~~

## Comments

2026-11-24: implemented, awaiting approval. `appearance` on 0.3 gains
`variation`: `fixed` (the default), `turned`, `turned-and-patched`; data-only
TOML `variation = "..."`. It rides in the face word's top two bits; `fixed`
keeps the word it had. A turned Voxel takes one of the square's eight
symmetries by a hash of its cell (from the mesh scale, so per Voxel at level 0
and per cell at coarse levels); a face whose tile differs from the top's
(grass's flank) only mirrors. Patches: 2D value noise on a 5 m lattice in
metres, value up to 8%, hue up to 4 degrees, on faces drawing the top's tile.
Terra: leaves and grass patched, stone, dirt and sand turned; `flora`: all 11
leaves patched; `strata`: stone, basalt, sand turned. Rotation judged on this
checkout's patterns, before 01's period fixes.

Captures in `/tmp/colour-03` (never committed), sheets in `sheets/`; before is
`/tmp/shimmer02-after` and `/tmp/shimmer-mid`. Settle 180 s: at host load
10-25 a 30 s settle left LOD levels 1-4 unfilled.

`flight` GPU frame, release `profiling`, offscreen 1920x1080, 3 rounds of 3
repeats interleaved, host load 6-25, median of per-run p50 over runs not in
the slow GPU mode:

| A/B                         | p50     | p95     |
| --------------------------- | ------- | ------- |
| main / branch (first shader) | 3.68 / 4.10 ms | -    |
| main / branch (final)       | 4.29 / 4.74 ms | 7.13 / 6.87 ms |
| branch, flags off / on      | 4.41 / 4.72 ms | 5.67 / 5.72 ms |

Unflagged draws within noise of main; the flagged path costs about 0.3 ms at
p50, p95 unchanged. 2D patches on a cheaper hash did not move it.

2026-11-25: `flight` cost cut (Q68). Profiled first, on a `profiling` build
with temporary shader switches: one measured `flight` run per variant per
round, variants rotated, 6 to 8 rounds, GPU frame from the timestamp markers,
2560x1440 offscreen. Over no turn at all, a turn uniform per Material cost
+0.09 ms p50, one per merged quad +0.2, one per Voxel +0.33. The cost is
neighbouring Voxels sampling apart inside a 2x2 pixel quad, not the turn's
arithmetic: two cheaper hashes and a float hash moved it 0 to 0.15 ms, and
per-Voxel layers of pre-turned tiles cost the same as turned UVs. About half
of it was in the sun's cascades (leaf cutouts); patches cost about 0.06 ms.

Changed: the Voxel is keyed per quad in the vertex stage (a flat word, the
quad's first Voxel in world cells) and a fragment steps from it by its UV's
whole part, so no fragment reads its world position or the mesh transform;
and the sun's cascades no longer turn the tile, since a leaf's holes blur into
the same dapple either way. The key hashes differently, so which way a given
Voxel turns changed; the grass and canopy carpets read the same.

Captures in `/tmp/colour-03-perf/{before,after}` (never committed), sheets in
`sheets/` (before above, after below): village 70 m and 110 m, jungle 110 m,
`jungle-top-medium`, and a jungle floor under the canopy for the dapple. All
fully loaded (level 1 at 6743 Chunks), `dev,profiling` release builds.

`scripts/profile-frames` `flight --instruments gpu`, offscreen 2560x1440,
release `profiling`, 3 rounds of 3 repeats per side in rotating order, median
of per-run figures (host slower today than in Q68: `main` p95 6.55, not 6.23):

| Side                 | p50 ms | p95 ms | p99 ms |
| -------------------- | ------ | ------ | ------ |
| `main` 0b03c594      | 5.77   | 6.55   | 6.99   |
| branch d82b525d      | 6.43   | 7.31   | 7.66   |
| branch + this change | 6.18   | 6.99   | 7.33   |

The branch over `main` falls from +0.66 / +0.77 / +0.67 ms to +0.41 / +0.45 /
+0.34 ms (p50 / p95 / p99). On Q68's `main` that puts the branch's p95 near
6.68 ms: under the 6.9 ms budget, short of the 6.6 ms aim. What is left is the
per-Voxel turn itself in the main pass; going lower changes the look.

2026-11-25: final frame check against `main` 0b03c594, 8 interleaved runs per
side, offscreen 2560x1440, medians: `flight` p95 6.28 -> 6.65 ms (+0.37),
`stationary` 5.18 -> 5.62, `teleport` 5.42 -> 5.81. Inside the 6.9 ms budget
but not "no worse than `main`": the cost of sampling a per-Voxel turn, left
open for the user. Logs: `/tmp/colour-final-2/`.
