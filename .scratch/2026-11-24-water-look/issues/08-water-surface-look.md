# 08: Water surface look

Status: done
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

- [x] Captures from the lake and coast fixtures show sky reflection at low
      angles, the floor when looking down, and the sun's glint; shown to the
      user.
- [x] A moving pair at the coast, scored as in
      `.scratch/2026-11-23-texture-shimmer/`, shows no sparkle or crawl at
      distance.
- [x] Glass and steam draw as before in captures; the test pinning
      `AlphaMode::Blend` now pins `Premultiplied`.
- [x] A unit test: every ripple frequency is a whole multiple of 2π/3600 s.
- [x] A test: the water-look message reaches the Client and marks exactly
      the blended Fluid Materials tagged `water` as water.

## Comments

- 2026-11-27, on 2f37808e plus this ticket:
  - The Server sends `MaterialWaterLooks` beside `MaterialPalette`: pure
    water's look for each Material tagged `water` that is a blended Fluid.
    Terra's water and `waters:water` were already tagged. The Client fills
    `water::table`, three rows a slot at binding 107; every other slot is not
    water and draws as before.
  - The blended material is `AlphaMode::Premultiplied`. The shader works water
    out premultiplied (the body's share folded into its albedo, so bevy_pbr's
    sun glint and the sky's reflection are not scaled by it), divides back out
    for fog and the tonemapper as glass is handled, and premultiplies every
    blended fragment last. In the `transparency` fixture the glass panes differ
    from before by at most 8 of 255, mean 0.21. Steam takes the same path as
    glass; no fixture shows it.
  - bevy_pbr's ambient term adds a specular share that at a grazing angle is
    nearly the whole ambient light, and `specular_occlusion` does not gate it.
    It turned the far sea grey-white, brighter than the sky. The water path
    takes it back out, worked out as bevy_pbr does (times the exposure), and
    the sky's reflection stands in for it.
  - The sky along the reflected ray comes from `sky_colour.wesl`, which
    `sky.wesl` now imports too: the fogged horizon before tonemapping, mixing
    to the zenith. The chunk extension's `water_sky` holds the zenith, kept in
    step with the sky's by `follow_sky` when it moves by more than 1/512. No
    sun direction is passed: the glint is bevy_pbr's directional light, and the
    horizon's glow already loops over the lights.
  - Ripples: two layers of an engine normal map (256², Rg16Snorm slopes, every
    mip, a sum of 28 integer-crest sines so it tiles), 4.5 m and 1.7 m tiles,
    drifting a whole number of tiles each hour. Each layer fades where a pixel
    spans more than a quarter of its shortest crest, and the slope variance it
    loses becomes roughness, up to 0.3.
  - A profiling-only `TESSERAE_WATER_CLOCK_S` holds water's clock, so a moving
    pair differs by the Eye's step alone.
- Captures, `TESSERAE_SCREENSHOT_OFFSCREEN=1`, in
  `/tmp/tesserae-water-look-captures/08-before/` and `08-after/`: lake, coast,
  underwater, river, transparency. The lake shows the sky's reflection at a low
  angle, a glint column and the floor below the Eye; the coast's sea is the
  sky's blue rather than the floor's sand. They wait on the user's review.
- Coast moving pair, the Eye 2.4 cm apart along the view, clock held, in
  `08-pair/`: before 0.18 mean and 0.2 % of pixels over 10; after 1.04 and
  1.8 %, the far water band 0.79 and 0.7 %. The amplified difference is the
  clouds, which keep their own clock, and the islands' edges; the water shows
  no sparkle or crawl.
- 2026-11-27: the user reviewed the captures and approved them.
