# 01: Baseline captures

Status: done

Spec: [Texture shimmer](../spec.md).

## What to build

Captures on `main`, offscreen (`TESSERAE_SCREENSHOT_OFFSCREEN=1`), Terra, Eyes
with a fractional part:

- Near: terrain a few metres away, textured faces filling the frame.
- Far: a pitch that shows level 0 running into the coarse levels.
- Shimmer: two captures a fraction of a metre apart at the far Eye; their
  difference image shows the texel crawl.
- Cave: a space lit only by block light.

Record the Eyes and variables in Comments so later tickets repeat them exactly.
Images stay in one `/tmp` folder and are never committed. The colour spec reuses
these Eyes and images as its baseline.

## Acceptance

- [x] Four views and the shimmer pair captured, Eyes recorded.
- [x] The user has seen them. Which parts read too saturated is the colour
      spec's first ticket, on these same images.

## Comments

2026-11-24: captured on this branch at `6ccaf35e`, whose only code change from
`main` (`2f45144f`) is a shader comment, so these are `main`'s pixels. Images
are in `/tmp/shimmer01-baseline`. Terra from `scripts/build-guests` at the same
commit, default seed, Terra's declared view. The binary is a debug `dev` build
(`--workspace --features tesserae/dev`), plus `tesserae/profiling` for the cave.

Common variables: `TESSERAE_MODS_DIR=games/terra/dist/mods`,
`TESSERAE_TRUST_MODS=1`, `TESSERAE_SCREENSHOT_OFFSCREEN=1`,
`TESSERAE_SCREENSHOT_WIDTH=1920`, `TESSERAE_SCREENSHOT_HEIGHT=1080`,
`TESSERAE_SCREENSHOT_SPEED_M_S=0`, `TESSERAE_SCREENSHOT_FLY_S=0`,
`TESSERAE_SCREENSHOT_SETTLE_S=30`, `BEVY_ASSET_ROOT` at the worktree, under
`env -i` as in the phase 22 baseline recipe.

| Image                      | `TESSERAE_SPAWN_M`      | Yaw | Pitch | Extra                                                                                |
| -------------------------- | ----------------------- | --- | ----- | ------------------------------------------------------------------------------------ |
| `near.png`                 | `1700.37,38.22,-722.41` | -90 | -28   |                                                                                      |
| `far.png`, `shimmer-a.png` | `1654.37,96.21,-733.43` | 30  | -9    |                                                                                      |
| `shimmer-b.png`            | `1654.62,96.21,-733.43` | 30  | -9    | 0.25 m east of `shimmer-a`                                                           |
| `cave.png`                 | `1639.37,-810.57`       | 180 | -15   | `TESSERAE_PROFILE_FIXTURE=cave`, `TESSERAE_PROFILE_CAVE_COVER_M=20`, profiling build |

- Near: open grass east of the spawn Settlement, cutout grass on the right.
- Far: over the beach towards a terraced, wooded island about 700 m out, across
  level 0 into the coarse levels.
- Shimmer: `shimmer-diff.png` is `|a - b|` in grey, times 4; `shimmer-crop.png`
  stacks a, b and the difference over the island (box `440,330`–`1600,470`).
  There the mean is 34.8 of 255 and 30% of pixels exceed 40: the island's grass
  and leaves change texel from one capture to the other, while the sand and
  water, whose textures are near-flat, stay dark. That box's numbers are the
  "after" comparison for 02. `far.png` and `shimmer-a.png` are byte-identical in
  size from two runs, so the capture is repeatable.
- Cave: the fixture moves the Eye from that column to `1764.5,-6.5,-932.5`, in
  the Crystal cavern, with at least 20 m of rock overhead; Crystal (emission 10)
  is the only light. A direct `TESSERAE_SPAWN_M=x,y,z` underground Eye left the
  surrounding Chunks unmeshed after 30 s (levels 1-4 never filled), so use the
  fixture.

2026-11-24: trees, on request. The nearest jungle core to spawn is at
`2314,-813`. North of it the jungle gives way to open savanna about 110 m out,
so every view faces south (yaw 180) into the jungle. Same build and variables as
above.

| Image                      | `TESSERAE_SPAWN_M`        | Yaw | Pitch | Eye height                          |
| -------------------------- | ------------------------- | --- | ----- | ----------------------------------- |
| `jungle-ground-close.png`  | `2314.37,-800.41`         | 180 | 5     | ground (47.3), among the trunks     |
| `jungle-ground-medium.png` | `2314.37,-1003.41`        | 180 | 2     | ground (46.6), ~80 m from the edge  |
| `jungle-ground-far.png`    | `2314.37,-1163.41`        | 180 | 1     | ground (40.7), ~240 m from the edge |
| `jungle-top-close.png`     | `2314.37,92.21,-813.43`   | 180 | -70   | ~15 m over the canopy               |
| `jungle-top-medium.png`    | `2314.37,150.21,-923.43`  | 180 | -45   | ~75 m over the canopy               |
| `jungle-top-far.png`       | `2314.37,200.21,-1313.43` | 180 | -12   | 200 m, the jungle 400-600 m out     |

Leaves are cutout, so these views are also the check for leaf coverage at a
distance in 02. `jungle-top-medium.png` shows the canopy's moiré most clearly.

2026-11-24: other Biomes and more trees, on request. Each Biome's core comes
from `Worldgen::biome_at` sampled in a throwaway test (not committed): a point
whose 16 headings out to 120 m are all that Biome. Each Biome has three views
facing north (yaw 0): `-ground` at the core, eye height
(`TESSERAE_SPAWN_M=x,z`), pitch 3; `-top` 75 m over the ground and 60 m south,
pitch -40; `-far` 150 m up and 300 m south, pitch -10. The beach faces west
(yaw 90) instead, from the shore west of spawn, because Terra's beach core lies
under the sea.

| Biome     | Core (x, z)  | Ground (m) |
| --------- | ------------ | ---------- |
| cold      | `982,-2077`  | 52         |
| dry       | `2062,-3085` | 32         |
| swamp     | `-170,-4573` | 34         |
| alpine    | `3934,3443`  | 130        |
| savanna   | `2314,-1063` | 44         |
| temperate | `1700,-722`  | 37         |
| jungle2   | `1774,-4861` | 27         |
| beach     | `1560,-733`  | 28         |

Beach Eyes: ground `1560.37,-733.41`; top `1600.37,103.21,-733.43`; far
`1860.37,180.21,-733.43`. The others are the core plus `.37`/`.41` (ground) or
`.37`/`.43` with the offsets above.

More trees: `jungle-ground-up.png` at `2314.37,-800.41`, yaw 180, pitch 75, the
canopy's underside; `jungle-top-grazing.png` at `2314.37,110.21,-1003.43`, yaw
180, pitch -15, across the canopy at a low angle; `swamp-forest-ground.png` at
`1790.37,-770.41`, yaw 90, pitch 2, the swamp forest east of spawn.

2026-11-24: four more shimmer pairs, each `-a` and `-b` 0.25 m apart in x, the
rest as in the Eye column. `shimmer-scores.txt` is the baseline; 02 reruns the
same scorer (`|a - b|` luminance over the frame, minimap, hotbar, hand and hint
masked out) on its own captures. "> 10" is the share of pixels that change by
more than 10 of 255.

| Pair                | Eye (`-a`)               | Yaw | Pitch | Mean | > 10  |
| ------------------- | ------------------------ | --- | ----- | ---- | ----- |
| `far` (`shimmer-a`) | `1654.37,96.21,-733.43`  | 30  | -9    | 5.78 | 15.2% |
| `jungle-top`        | `2314.37,150.21,-923.43` | 180 | -45   | 24.7 | 66.7% |
| `jungle-ground-far` | `2314.37,40.70,-1163.41` | 180 | 1     | 14.8 | 39.8% |
| `dry-ground`        | `2062.37,33.14,-3085.41` | 0   | 3     | 5.40 | 11.4% |
| `cold-far`          | `982.37,202.21,-1777.43` | 0   | -10   | 6.60 | 20.6% |

The ground-level pairs include real parallax: near trunks and steps move by
whole texels. That part is the same before and after 02, so only the drop
between the two runs measures sampling.
