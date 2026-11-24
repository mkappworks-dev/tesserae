# 03: Per-block and per-area variation

Status: ready-for-agent

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

- [ ] A Material without the flag draws exactly as before (test).
- [ ] Merged quads rotate per Voxel, not per quad; no seam or mip jump at Voxel
      edges up close.
- [ ] The variation is the same at another Voxel size (in metres).
- [ ] `jungle-top-medium`, `alpine-far` and the 110 m views show no repeat every
      metre and a broken grass and canopy carpet; approved.
- [ ] `flight` GPU frame no worse than `main` beyond noise.

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
