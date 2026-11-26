# 08: Water surface look

Status: ready-for-agent
Blocked by: 01

Spec: [Water that looks natural](../spec.md), The water path.

## What to build

Water reads as water at every angle.

- **Which slots are water.** The Server resolves each Material's water look (a
  blended Fluid tagged `water` gets the engine defaults) and publishes it beside
  `MaterialPalette`; the Client fills a per-slot uniform table, every other slot
  marked not water.
- **Blending.** The blended material moves to `AlphaMode::Premultiplied`, and
  the chunk shader premultiplies every blended fragment itself, so glass and
  steam look as before and water's reflection is added unscaled by alpha.
- **Surface.** Roughness about 0.08 and F0 0.02; ripple normals from the
  engine's normal map, generated at start-up, at frequencies defined in Rust as
  whole multiples of 2π/3600 s and passed as shader defs. The sky along the
  reflected ray, from a WESL function shared with the colour branch's sky and
  evaluated before tonemapping (the chunk extension gains the sky's zenith
  colour and sun direction), weighted by Fresnel, plus the sun's glint. Seen
  from below, no sky reflection.
- **Distance.** Ripples fade and roughness rises with distance, so the surface
  does not sparkle.
- Water skips ambient occlusion and no longer samples its 16 by 16 tile at
  level 0. Depth is not known yet, so the body keeps a fixed colour until 10.

## Acceptance criteria

- [ ] Captures from the lake and coast fixtures show sky reflection at low
      angles, the floor when looking down, and the sun's glint; shown to the
      user.
- [ ] A moving pair at the coast, scored as in
      `.scratch/2026-11-23-texture-shimmer/`, shows no sparkle or crawl at
      distance.
- [ ] Glass and steam draw as before in captures; the test pinning
      `AlphaMode::Blend` now pins `Premultiplied`.
- [ ] A unit test: every ripple frequency is a whole multiple of 2π/3600 s.
- [ ] A test: the water-look message reaches the Client and marks exactly
      the blended Fluid Materials tagged `water` as water.
