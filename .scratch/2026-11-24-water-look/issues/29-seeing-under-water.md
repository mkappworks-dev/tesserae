# 29: Seeing under water

Status: needs-info
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

- [ ] A floor 5 m off keeps most of its colour in Terra's lake; one 30 m off is
      mostly the water's colour.
- [ ] A shallow floor lit brighter than a deep one, by the water above it.
- [ ] Caustics move on the floor near the surface and fade with depth.
- [ ] No white line across the view under water.
- [ ] Captures at the four angles, before and after, shown to the user.
