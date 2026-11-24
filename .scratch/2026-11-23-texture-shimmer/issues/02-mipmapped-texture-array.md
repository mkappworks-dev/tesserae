# 02: Mipmapped Material texture array

Status: done
Blocked by: 01

Spec: [Texture shimmer](../spec.md).

## What to build

Solution item 2: the atlas becomes a 2D array, one 16x16 layer per Material
face, with a CPU-built mip chain, an all-linear 16x anisotropic repeat sampler,
a shader texel snap where the mip level is at or below 0, and
`textureSampleGrad` on continuous UVs. Item models and tile icons follow.

## Acceptance

- [x] Mip builder unit tests from the spec's Testing section.
- [x] The shimmer pair from 01 shows no texel crawl on level 0 or coarse levels.
- [x] Near view unchanged: texels still crisp, no seams on merged quads.
- [x] Leaves keep their coverage at distance.
- [x] `flight` GPU frame no worse than `main` beyond noise.

## Comments

2026-11-24: done at the commits ending `7b2c9e5e`. After-captures, from 01's
Eyes with the final dev build, are in `/tmp/shimmer02-after` (never committed),
with before/after sheets in `sheets/`.

Shimmer scores, `|a - b|` luminance with the HUD masked, before (main) and
after:

| Pair                                   | Before mean, > 10 | After mean, > 10 |
| -------------------------------------- | ----------------- | ---------------- |
| `far` (island)                         | 5.78, 15.2%       | 3.13, 7.5%       |
| `cold-far`                             | 6.60, 20.6%       | 2.25, 5.0%       |
| `jungle-top`, 0.25 m step              | 24.7, 66.7%       | 16.9, 44.7%      |
| `jungle-top`, 0.02 m step (~1/3 pixel) | 9.57, 29.2%       | 5.00, 10.9%      |
| `jungle-ground-far`                    | 14.8, 39.8%       | 13.6, 34.7%      |
| `dry-ground`                           | 5.40, 11.4%       | 4.99, 10.4%      |

- At 75 m, a 0.25 m step moves the canopy about 3.5 pixels, so that pair mostly
  measures real motion; the 0.02 m pair is the canopy's shimmer, and main's
  moiré bands are gone. The ground pairs are dominated by parallax of near
  trunks and steps, the same before and after.
- Near views keep crisp texels; the old inset's half-width edge texels are full
  width now. Leaves keep their coverage near and fill in far off.

Decisions made on the way, all in the spec: the texel snap is per axis; held
models and tile icons keep the 2D atlas; dense leaves fill in at a distance (the
user's choice over holding coverage exactly); layers only for distinct textured
faces (Terra: 295, not 771); and the array is uploaded one write per mip level.

`flight` GPU frame, release `profiling`, offscreen 1920x1080, three repeats per
round, rounds interleaved with main (load 5-7):

| Build                              | p50     | p95       | Slow rounds |
| ---------------------------------- | ------- | --------- | ----------- |
| main                               | 4.05 ms | 4.66 ms   | 0 of 11     |
| one layer per face, Bevy's upload  | 4.10 ms | 11-12 ms  | 6 of 7      |
| distinct layers, Bevy's upload     | 4.20 ms | 4.8-17 ms | 1 of 2      |
| distinct layers, one write per mip | 4.14 ms | 4.73 ms   | 0 of 2      |

The slow mode was Apple's GPU performance controller holding the clock low
(P5-P8 instead of P13, 3-4 W instead of 16-19 W) after Bevy's one write per
layer per mip, once the GPU got busy over dense forest. Instruments' Metal
System Trace suppressed it (0 of 11 traced runs slow); it was found by sampling
IOReport's GPU performance-state channels. Variants that never sampled the
texture, or uploaded it and never bound it, were slow too; bound with no upload,
it was not.
