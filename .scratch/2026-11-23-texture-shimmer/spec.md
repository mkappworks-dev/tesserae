# Texture shimmer

Status: ready-for-agent

Source: branch `fix/texture-shimmer-and-saturation`, cut from `main` at
`2f45144f`. Tickets: [issues/](issues/). The saturation half of the same report
is its own spec, [Colour, art and shader](../2026-11-23-colour-art-and-shader/spec.md).

## Problem Statement

Reported on 2026-11-23: textures shimmer at a distance. Far terrain, and coarse
levels most of all, crawls and sparkles as the camera moves instead of reading
as a clean surface.

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
  each frame the camera moves. That is the shimmer, and it is a sampling bug:
  the art makes it louder but does not cause it.
- The inset `0.5 + fract * (EDGE - 1)` maps 16 texels onto 15 texel-widths, so
  the edge texels of every tile are drawn half as wide as the rest.
- Anti-aliasing defaults to SMAA, which finds geometric edges and does nothing
  for texel aliasing inside a face; there is no temporal filter.

## Solution

1. **Baseline captures** on `main`, offscreen, at fixed Eyes: a near view, a
   far view across level 0 into coarse levels, a moving pair for shimmer, and
   a cave lit by block light. Every later ticket, in this spec and the colour
   one, is judged against them.
2. **Mipmapped Material textures.** Move the atlas to a 2D texture array, one
   layer per Material face, so each layer is a 16x16 tile with its own full mip
   chain (16, 8, 4, 2, 1) and no neighbour to bleed into. Mips are built on the
   CPU when the textures arrive, averaged in linear space with alpha-weighted
   colour so cutout leaves do not darken at distance, then alpha scaled per mip
   to keep mip 0's coverage at the cutoff. wgpu refuses anisotropy unless every
   filter is linear, so the sampler is all-linear with repeat addressing and 16x
   anisotropy, and the pixel-art look up close comes from the shader: where the
   mip level is at or below 0 it snaps the UV to the texel centre, which linear
   filtering returns unblended. The shader takes `dpdx`/`dpdy` of the
   continuous UV at the top of the fragment, before any branch or discard, and
   samples with `textureSampleGrad`; repeat addressing per layer replaces
   `fract`, so merged quads get the right mip with no seam. The inset goes
   away. Item models and tile icons, which share the atlas, move with it.

## Decisions

- Pixel art stays crisp up close: the shader's texel snap makes magnification
  read as nearest. Only distance filters.
- One all-linear sampler with 16x anisotropy and the texel snap, kept only if
  the `flight` fixture's GPU frame time does not regress beyond noise.
- Texture arrays over a padded atlas: padding still bleeds at the small mips
  and wastes memory; an array layer cannot bleed.
- Mips are built on the Client, not shipped by Mods: a Mod sends 16x16 tiles as
  today and the WIT contract does not change. Bevy's GPU mip generator takes no
  sRGB format and cannot preserve coverage, so it is not an option.
- Leaves keep mip 0's coverage at every mip.
- Fixed engine behaviour, not a Settings entry.
- No TAA. It would also help shimmer, but it ghosts on fast flight and costs
  GPU the frame does not have; mips fix the cause.
- Terra's art is not changed here; the colour spec owns it, so this PR's before
  and after differ only in sampling.

## Testing

- Unit tests for the mip builder: a uniform tile stays uniform at every level;
  a checkerboard averages to its mean; a half-transparent cutout tile keeps its
  colour; sparse foliage keeps mip 0's coverage at 4x4 and 2x2 within a
  tolerance.
- The textured-slot bitmask and alpha mode per slot keep their current tests.
- Capture comparisons from ticket 01's Eyes, before and after, reviewed by eye.
- GPU frame time on the `flight` fixture must not regress beyond noise
  (interleaved CPU-time A/B against `main`, per the benchmarking notes).

## Out of Scope

- Colour, palette, patterns, tonemapping and light balance: the colour spec.
- Higher-resolution texture packs.
- Changes to the Mod texture API.
