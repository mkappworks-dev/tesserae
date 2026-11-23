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
