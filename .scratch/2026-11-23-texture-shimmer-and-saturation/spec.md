# Texture shimmer and saturation

Status: needs-triage

Source: branch `fix/texture-shimmer-and-saturation`, cut from `main` at
`38a92cd7`. Tickets: [issues/](issues/).

## Problem Statement

Two things the player sees, reported on 2026-11-23:

1. **Textures shimmer at a distance.** Far terrain, and coarse levels most of
   all, crawls and sparkles as the camera moves instead of reading as a clean
   surface.
2. **The picture is too saturated.** Lighting and colour read loud, not
   natural.

What the code does today, read from source:

- Material textures are one 2D atlas, `Rgba8UnormSrgb`, three 16-texel tiles
  (top, side, bottom) per Material row (`render/texture_atlas.rs`,
  `client/material_textures.rs`). It has no mip chain and an
  `ImageSampler::nearest()` sampler.
- The chunk shader samples it with `textureSampleLevel(..., 0.0)`
  (`render/material_texture.wesl`): always mip 0, always nearest. Merged quads
  repeat the tile with `fract(uv)`, so the shader could not use implicit
  derivatives even if mips existed; `fract` jumps at every Voxel edge.
- Terra's art is per-texel hash noise (`games/terra/src/textures.rs`, `grain`,
  `dirt`): the highest-frequency signal a texture can hold. With no mips, a
  pixel far away lands on one arbitrary texel of 16 and picks a different one
  each frame the camera moves. That is the shimmer, and it is a sampling bug,
  not an art problem.
- The inset `0.5 + fract * (EDGE - 1)` maps 16 texels onto 15 texel-widths, so
  the edge texels of every tile are drawn half as wide as the rest.
- Anti-aliasing defaults to SMAA, which finds geometric edges and does nothing
  for texel aliasing inside a face; there is no temporal filter.
- Lighting: one `DirectionalLight` at Bevy's default illuminance (ambient
  daylight, 10 000 lux), `GlobalAmbientLight` brightness 900 (Bevy's default is
  80), Bevy's default `Exposure` and `Tonemapping` (TonyMcMapface). Cameras are
  not HDR, so tonemapping runs in the shader. There is no `ColorGrading` except
  on death (`client/death.rs`).
- Block light is added as emissive at `BLOCK_LIGHT_STRENGTH` 4.0 times the
  albedo, tinted warm.

The saturation has no confirmed cause yet. Candidates, to be told apart on
captures, not guessed: sun plus ambient pushing lit faces well past 1.0 into the
tonemapper's shoulder; the palette and texture colours themselves; the sky and
fog colours the Biomes send; and the absence of any grade.

## Solution

1. **Baseline captures** on `main`, offscreen, at fixed Eyes: a near view, a
   far view across level 0 into coarse levels, a moving pair for shimmer, and
   a cave lit by block light. These are what every later ticket is judged
   against.
2. **Mipmapped Material textures.** Move the atlas to a 2D texture array, one
   layer per Material face, so each layer is a 16x16 tile with its own full mip
   chain (16, 8, 4, 2, 1) and no neighbour to bleed into. Mips are built on the
   CPU when the textures arrive, averaged in linear space with alpha-weighted
   colour so cutout leaves do not darken or vanish at distance. The sampler is
   nearest for magnification (the pixel-art look up close stays), linear between
   and within mips for minification, repeat addressing, and anisotropic
   filtering. The shader samples with `textureSampleGrad`, taking gradients from
   the continuous UV, not the `fract`ed one, so merged quads get the right mip
   with no seam. The inset goes away. Item models and tile icons, which share
   the atlas, move with it.
3. **Colour and light balance**, chosen on captures from ticket 01 once the
   cause is known. Expected levers, smallest first: the ambient brightness and
   sun illuminance relative to `Exposure`; the tonemapper (AgX desaturates
   bright colours more gracefully than TonyMcMapface); a mild
   `ColorGrading` saturation below 1.0; then palette or art changes in Terra.
   The values are a look decision, so the user approves them on a capture
   before they land.

## Decisions

- Pixel art stays crisp up close: magnification is nearest. Only distance
  filters.
- Texture arrays over a padded atlas: padding still bleeds at the small mips
  and wastes memory; an array layer cannot bleed.
- Mips are built on the Client, not shipped by Mods: a Mod sends 16x16 tiles as
  today and the WIT contract does not change.
- No TAA in this work. It would also help shimmer, but it ghosts on fast flight
  and costs GPU the frame does not have; mips fix the cause.

## Testing

- Unit tests for the mip builder: a uniform tile stays uniform at every level;
  a checkerboard averages to its mean; a half-transparent cutout tile keeps its
  colour and its coverage at the threshold.
- The textured-slot bitmask and alpha mode per slot keep their current tests.
- Capture comparisons from ticket 01's Eyes, before and after, reviewed by eye.
- GPU frame time on the `flight` fixture must not regress beyond noise
  (interleaved CPU-time A/B against `main`, per the benchmarking notes).

## Out of Scope

- A day cycle or moving sun.
- Higher-resolution texture packs.
- Changes to the Mod texture API.
