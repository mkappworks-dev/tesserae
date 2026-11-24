# Colour, art and light

Status: ready-for-agent

Source: branch `fix/colour-art-and-shader`, on `main` at `0b03c594`, after
[Texture shimmer](../2026-11-23-texture-shimmer/spec.md) merged (#74). That
spec's ticket 01 captures are this spec's baseline. Tickets: [issues/](issues/).

## Problem Statement

Reported on 2026-11-23 and 2026-11-24: the picture is too saturated, and from
60 to 120 m at ground level houses, trees, wood and leaves look "wonky and
abstract and saturated… It's not natural!" The shimmer fix does not cause it:
its before and after match at those distances.

Read from the baseline captures (`/tmp/shimmer01-baseline`, the mid-distance
set in `/tmp/shimmer-mid`, never committed) and from source:

- **The palette is too pure.** Grass's top `[80, 144, 49]` is HSV S 0.66, about
  C\* 57; Terra's leaves are about C\* 49; `flora`'s jungle leaves
  `[38, 150, 56]` are S 0.75. A measured foliage reference sits near C\* 25.
  Savanna's acacia and the pine already read naturally. Dirt `[107, 74, 46]`
  reads salmon on the savanna floor. Sunlit sand and snow lose their texture to
  near-white.
- **The sky is authored grey.** Every Biome's sky is HSV S 0.15 to 0.25; Dry's
  `(176, 160, 128)` reads khaki. A grey sky makes the greens louder by contrast.
- **Patterns repeat every metre.** Every block draws the same 16x16 tile, so at
  60 to 120 m, where a block is 2 to 5 pixels, canopies become a knitted
  lattice. Six of Terra's patterns repeat on periods that do not divide 16
  (leaves, wood, stone vein, brick, mud, water) and draw a seam at every Voxel
  edge.
- **Detail is per-texel noise at full contrast.** A mip averages per-texel
  noise away by mip 2, so a far tile is its mean colour plus whatever larger
  shapes it has, and Terra's have almost none. Rare bright outlier texels in
  dirt, stone and ash flicker near, form a lattice on large faces, and lift the
  far mean (ash by about 12 levels). `tint` adds one offset to R, G and B, which
  makes every darkened texel more saturated. Bark, planks, brick and thatch in
  the building Mods read as grainy speckle at distance.
- **Nothing fades with distance.** Distance fog is sized from the reach: at the
  default reach of about 6 km a surface at 110 m is some 4% fogged, so 100 m is
  as vivid as 2 m.
- **Shading is flat.** No darkening where blocks meet, so canopies read as flat
  cut-outs; under the jungle canopy the floor is nearly black.
- **Trees in a forest merge into one lid.** Neighbouring crowns of one species
  are one Material, so the faces between them are culled and they share one
  colour and tile. Jungle trees stand about 4 m apart (clumped closer still)
  under 10 m crowns, all 18 to 24 m tall; swamp trees 4 m apart. Real canopy
  trees stand roughly 8 to 12 m apart.
- **The flat colour disagrees with the tile.** The map and the untextured paths
  draw a Material's appearance colour, which is 3 to 12 levels off its
  texture's mean for ash, dirt and wood.

Lighting, read from Bevy 0.20.1 source: the cameras are not HDR, so of
`ColorGrading` only `global.exposure` and `global.post_saturation` apply. A
sunlit grass top lands at about (0.26, 0.89, 0.10) linear before tonemapping,
where TonyMcMapface is close to identity: lit faces are not pushed into its
shoulder, so the tonemapper is not the cause. Block light is emissive with
`emissive_exposure_weight` 0, so it skips `Exposure`.

Most of the screen is Mod art, not Terra's own: `flora` draws every tree, grass
tuft and flower (its leaves are cutout), `strata` the rock, `timber` and
`brickwork` the houses.

## Solution

Six tickets, judged on the baseline captures, before and after:

1. **Art pass** over Terra's `textures.rs` and `building.rs`, its appearance
   colours and Biome sky and fog colours, and the PNGs of the Mods Terra ships
   (`flora`, `strata`, `timber`, `brickwork`): pattern fixes first, then colour
   targets.
2. **Light balance and aerial perspective:** a near haze in metres in Distance
   fog, ambient and Sky light balance, block light and `Exposure`, and the
   tonemapper.
3. **Per-block and per-area variation:** a rotated or mirrored tile per Voxel
   for Materials that read the same in any direction, and a crown-sized colour
   variation in metres on leaves and grass.
4. **Screen-space ambient occlusion**, landed only if it fits the frame budget.
5. **Tree variety:** storeys, canopy variants and wider jungle and swamp
   spacing, so a forest reads as separate trees.
6. **Loading terrain draws black:** a bug on `main` found in the captures; far
   terrain that is still loading draws black with orange dots.

Every value is a look decision: the user approves it on a capture before it
lands.

## Decisions

- Colour is fixed at its source first. A shader-wide desaturation would also
  flatten what is meant to be vivid (crystal, lava, the sky), so the shader
  balances light and does not stand in for the palette. `post_saturation` only
  if the picture is still loud after the art pass.
- The art pass covers the shared Mods Terra ships, since they are most of the
  screen: palette and pattern edits of the existing PNGs, no new art. `crops`
  and the smaller Mods wait for a capture that shows them.
- Pattern rules, for `textures.rs` and the Mod PNGs alike:
  - Every period divides 16; one mortar or plank line per tile edge.
  - Leaves get a period of 4 or 8 with clumped gaps, not a regular grid.
  - Outlier speckles become 2x2 clumps at no more than 1.3x base luminance.
  - About two thirds of a surface's spread moves into a coarse noise layer (2
    or 4 texels) and the per-texel term halves: natural surfaces, and the
    building Mods' planks, thatch, brick and cobble.
  - `tint` scales the colour by one factor in linear light instead of adding an
    offset.
  - Non-emissive texels stay within about 50 to 243 sRGB; `INK` rises to about
    50. Glass, lava and the lamp are exempt.
  - Thin sprite features are at least 2 texels wide; ripe wheat covers 25 to
    35% of its tile. A sprite's appearance colour is its edge colour, so
    transparent texels do not bleed a fringe.
- Colour targets, starting points the user approves on captures:
  - Grass top and Terra's leaves move together toward the acacia's chroma, at
    most about S 0.5 (C\* 35 to 40), slightly darker. `flora`'s jungle, oak,
    cherry, orange and apple leaves follow; pine and acacia stay.
  - Dirt to about S 0.40, hue toward yellow-brown.
  - Sand to L\* 70 to 75, C\* 20 to 25; snow to about 90% value. No sunlit sand
    or snow top above about 85% value on screen.
  - `flora:cane` (`[140, 190, 90]`, the swamp's pale cross plants) darker,
    with a two-tone stem and leaf.
  - Temperate's sky toward a clear blue at about S 0.4; Dry's toward a pale warm
    blue, its warmth kept in the fog. Every Biome's fog stays lighter than its
    sky, and the clear colour moves with it.
  - No saturation change underground; the cave reads muted already.
- The Mods' PNGs are the plant art. Terra's `sprite()` drawings and its stale
  tree aliases are deleted: a Game's appearance row only reaches its own
  Materials, so nothing Terra declares used them. Thicker wheat is drawn in
  `crops`' PNGs.
- A Material's appearance colour is its tile's linear mean (grass: its top),
  set after the pattern changes. Terra's are tested; the Mods' `content.toml`
  colours are set by hand from the same measure.
- Aerial perspective lives in Distance fog, not a new full-screen pass: a haze
  in metres that does not depend on the reach, fading blue first, beside the
  reach-sized fade. The glossary's Distance fog entry changes with it.
- Tile variation is opt-in per Material: leaves, stone, dirt, sand and grass top
  rotate or mirror; planks, brick, logs and thatch keep their orientation. The
  sampling gradients rotate with the UV, or mip selection breaks.
- Large-scale variation: patches about one crown across (around 5 m), keyed on
  world position in metres, shift value by about 8% and hue by a few degrees,
  on leaves and the grass top only, under the same flag as rotation. Stone,
  dirt and sand rotate only.
- Trees are told apart three ways at once: canopy variants per tree, the
  large-scale variation within and across crowns, and light (AO, shadow, haze).
- Canopy variants: a tree row's `canopy` takes one Material or a list; each tree
  picks one by a hash of the World seed and its root position in metres. Jungle,
  oak, swamp, cherry, orange and apple get three variants in `flora`, differing
  in colour only (value about 8%, hue about 4 degrees); pine, birch, palm and
  acacia stay single. The variants are Grown Materials, so they add no Items.
- Storeys: a tree row may declare weighted storeys, each with a trunk height
  range and a crown radius, picked per tree like the variant; a row without
  them generates as today. Starting values:
  - Jungle: 15% at 30 to 36 m with a 6 m crown, 60% at 18 to 24 m with 4.5 m,
    25% at 8 to 12 m with 3 m.
  - Swamp: 70% at 12 to 15 m, 30% at 6 to 9 m with a 2.5 m crown.
  - Oak: one storey widened to 8 to 14 m.
- Where crowns overlap, the taller storey keeps the Voxel, unless generation
  order already gives it.
- Spacing: jungle 4 m to 6 m mean, clump strength 0.5 to 0.25; swamp 4 m to
  5 m, clumping kept at 0.7.
- The rotation flag, a `canopy` list and `storeys` are new Declaration fields.
  The `0.3` contract line is unreleased, so its `appearance` and tree records
  are extended in place, with the data-only TOML beside them; a Mod that sets
  none of them declares what it does today. `flora` and `strata` set the flag
  on their own leaves and rock, not only Terra.
- Impostors come from the same pure tree draw, so they follow storeys and
  variants without their own rule. Canopy cover picks one variant per coarse
  Voxel by the same hash, and reads the new jungle and swamp spacing from the
  density fields it already uses.
- These change what Generator version 3 makes. Nothing is released, so v3
  changes in place and its digests are recorded again
  ([ADR-0033](../../docs/adr/0033-generator-version-3-is-the-only-version.md)).
- Ambient occlusion is screen-space. Per-corner AO in the mesher would split
  greedy-merged quads and give back part of the packed-quad mesh-memory saving;
  it is considered only if screen-space AO misses the frame budget.
- The tonemapper is compared (AgX beside TonyMcMapface) but is not expected to
  be the fix. A change re-measures the fog's tonemap darkening constant.
- No HDR cameras unless captures show the sectional grade is needed: that is an
  extra full-screen pass to measure first.

## Testing

- Capture comparisons at the baseline Eyes, before and after, reviewed by eye:
  `near`, `far`, `cave`, the `{biome}-{ground,top,far}` set, the jungle and
  swamp tree views, and the mid-distance village and jungle views at 70 and
  110 m.
- Terra's texture tests keep passing. New: every period-bound pattern tiles
  across a Voxel edge; Terra's appearance colours match their tile's linear
  mean. Pattern and palette changes are reviewed on captures, not asserted.
- Tile variation: a Material without the flag draws exactly as before.
- Screen-space AO: GPU frame time on `flight` against `main`.
- Tree variety: a tree row with one canopy and no storeys generates exactly as
  before; variant and storey choice are deterministic per World seed and
  position, and the same at another Voxel size; v3's digest tests are recorded
  again in the same commit as the change.

## Out of Scope

- Water's transparency and colour: it reads milky, not saturated.
- Dilating transparent texels in the mip builder, so every Mod's cutouts avoid
  edge bleed: a follow-up.
- Whether coarse LOD levels drawing the tile 2^k times larger is intended: a
  follow-up.
- Trunks wider than one Voxel for the tallest storey: new trunk, limb and
  felling rules, a follow-up.
- A day cycle or moving sun.
- The other demo Games' own art.
