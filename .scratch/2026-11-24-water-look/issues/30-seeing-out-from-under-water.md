# 30: Seeing out from under water

Status: ready-for-agent
Blocked by: 29

Spec: [Water that looks natural](../spec.md), Under water. From the user's
review: seeing outside from under water is not the best.

## What to build

From below, the surface is drawn. Looking up within about 49 degrees of straight
up (Snell's window) the sky and what stands above the water show through, bent
and squeezed toward the window's middle and moving with the ripples; past that
angle the surface mirrors the water below it (total internal reflection), so it
reads as a bright, rippling ceiling and not an open blue.

Captured looking up (`/tmp/tesserae-water-look-captures/29-under/sheet.png`):
the surface is not drawn at all, only the fog's blue with trees faint through
it, and white lines run along the edges of the surface's quads.

## Acceptance criteria

- [ ] A capture looking straight up shows the sky in a round window and the
      surface's mirror outside it, both rippling.
- [ ] Trees on the shore show through the window and not past it.
- [ ] No white lines along the surface's quads from below.
- [ ] GPU time under water against before, in Comments.
- [ ] Captures shown to the user.

## Comments

- 2026-11-27: the user chose clear water with a light blue tint at any
  distance over a realistic fade by 30 m.
