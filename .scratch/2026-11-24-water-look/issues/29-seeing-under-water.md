# 29: Seeing under water

Status: ready-for-agent
Blocked by: 28

Spec: [Water that looks natural](../spec.md), Under water. From the user's
review: the view under water is not the best, for the ground, plants and, later,
animals.

## What to build

Under water the floor, plants and anything else near read in their own colours,
and fade to the water's colour with distance, not within a few metres. Sunlight
reaching a surface under water is dimmed by the water above it, per channel, so
a shallow floor is bright and a deep one blue and dark, and it moves in drawn
caustics: bright, shifting lines of light that fade with depth. Every material
drawn under water (terrain, plants, Structures, and later Mobs) takes the same
fog and light.

Captured at four angles (`/tmp/tesserae-water-look-captures/29-under/sheet.png`):
the floor past a few metres is one flat blue-teal and sand has no colour even
near; a thin white line crosses the view looking down.

## Acceptance criteria

- [x] Under water reads as clear water with a light blue tint at any distance:
      a floor 5 m off and one 30 m off both keep their colours, tinted; only
      near the edge of what is streamed does the view fade to the water.
- [ ] A shallow floor lit brighter than a deep one, by the water above it.
- [ ] Caustics move on the floor near the surface and fade with depth.
- [x] No white line across the view under water.
- [ ] Captures at the four angles, before and after, shown to the user.

## Comments

- 2026-11-27: the user chose clear water with a light blue tint at any
  distance over a realistic fade by 30 m.
- 2026-11-27: under water is clear to 60% of the level-0 reach and fades to
  the water's colour by it (Linear fog; the chunk shader draws it), under a
  full-screen shimmer and a 3% tint on the Player's camera, still under
  Reduced motion. The user kept 3%. Captures
  `/tmp/tesserae-water-look-captures/29-after/sheet.png` (before, 0%, 3%).
  The white line across the view: `main` shows the same line at the same
  view (`30-after/main.png`), so it is not this branch's; it is not a coarse
  level (unchanged under `TESSERAE_LOD_TINT`) nor a vertical water face
  (unchanged with them painted magenta), and only shows under water. Its
  cause is not yet found. Caustics and the light from above are left out:
  the user chose clear water.
- 2026-11-27, the white line: the aim outline. From inside water the aim ray
  stopped at the Voxel the Eye was in, and its white outline drew that
  Voxel's edges round the whole view, on `main` too. The ray now passes
  through the fluid it starts in (`from_under_water_remove_reaches_the_bed_through_the_water`);
  `/tmp/tesserae-water-look-captures/32-lines/fixed.png`.
