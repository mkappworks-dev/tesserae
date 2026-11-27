# 19: Coarse water matches at every hour

Status: done
Blocked by: 16

Spec: [Water that looks natural](../spec.md), Coarse levels. From the user's
review.

## What to build

At level 1 and beyond, water reads as a flat, pale sheet, "a piece of ice",
against level 0's water, by day, at sunrise and sunset and at night. Coarse
water carries the same roughness, depth colour and reflection as level 0's at
that distance, so the edge between them does not show.

## Acceptance criteria

- [x] Captures from high over a lake and the sea, past level 0's edge, at
      noon, sunset and midnight, before and after, shown to the user.
- [x] The edge between level 0 and level 1 water does not show as a change of
      brightness or roughness.

## Comments

- 2026-11-27: the far water reflected the pale horizon at nearly full
  Fresnel, as a flat mirror. The slope variance too fine for the pixel now
  tilts the normal the view sees toward it, so the far water reflects higher
  sky and less of it. Before and after at noon, sunset and midnight in
  `/tmp/tesserae-water-look-captures/19-before/` and `/tmp/tesserae-water-look-captures/19-after/sheet.png`; the user had said the white
  horizon was the issue.
- 2026-11-27: the user reviewed the captures and approved them.
