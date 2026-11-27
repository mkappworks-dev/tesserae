# 30: Seeing out from under water

Status: done
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

- [x] A capture looking up shows the sky and the shore through the surface at
      every angle, with a faint shimmering sheen where the surface is.
- [x] No white lines along the surface's quads from below.
- [x] GPU time under water against before, in Comments.
- [x] Captures shown to the user.

## Comments

- 2026-11-27: the user chose clear water with a light blue tint at any
  distance over a realistic fade by 30 m.
- 2026-11-27: the user chose a clear surface over Snell's window and a mirror:
  from below the surface is see-through at every angle with a faint shimmering
  sheen, under 29's 3% tint and shimmer.
- 2026-11-27: from below the surface hides 8% and carries a sheen up to 0.25
  of the sky that moves with the ripples; the shore and trees show through it
  (`/tmp/tesserae-water-look-captures/30-after/sheet.png`). The white lines
  along its edges are the same as 29's line, on `main` too.
- 2026-11-27: the white lines were 29's aim outline, now gone.
- 2026-11-27, the cost. The build before 29 against this one, profiling,
  `--instruments gpu`, runs A, B, B, A, load 7 to 10. GPU p50: under water
  3.17 to 3.32 ms (+0.15, the shimmer pass and the clear surface), the lake
  3.99 to 4.01 (+0.03). Frame p50: under water 4.47 to 4.72, the lake 4.96 to
  5.14.
