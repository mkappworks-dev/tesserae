# 17: Default sky blue and white fog

Status: done

Blocked by: None (can start immediately)

Spec: [Menus polish](../spec.md).

## What to build

A biome that declares no sky and fog colours gets a default sky of sky blue
(`#87ceeb`) and a default fog of white (`#ffffff`), in the World and in Create
World's Biomes step. A biome whose Game declares its own colours keeps them.
Today a biome without `sky_fog` falls back to whatever the renderer uses when
`sky_fog_at` returns nothing; that fallback becomes these two colours, defined
once and shared by the renderer and the Biomes step. Reset on a biome returns
it to its Game's colours, or to these defaults when it declares none.

## Acceptance criteria

- [x] In a World, standing in a biome with no declared sky and fog shows a sky
      blue sky and white fog
- [x] The Biomes step shows `#87ceeb` and `#ffffff` for such a biome, and Reset
      returns to them
- [x] A biome that declares its own colours is unchanged
- [x] The defaults are defined in one place

## Comments
`SkyFog::DEFAULT` in `tesserae-core` is the one definition; the renderer's haze
(`FogSettings::default`) and the Biomes step read it. `sky_fog_at` now treats a
Biome without colours as having the default instead of skipping it, so a
colourless Biome no longer takes its coloured neighbour's sky, and a ranged
(landform) Biome without colours now sets its own sky. The tonemapper lift that
`fog_toward` adds is a separate measured constant now (`TONEMAP_DARKENING_SRGB`,
the old haze's 10/14/22). Typing one colour on a colourless Biome keeps the
other at the shown default, not a copy of the typed one. Every demo Game Biome
declares its own colours, so no Game's look changes; a World with no Biomes at
all now shows the default sky blue and white haze.

Follow-up (user decision): a colourless Biome surrounded by Biomes that declare
colours (a landform such as a mountain) carries on their sky and fog, as before
this ticket; the default applies only where no Biome in the blend declares
colours. A World with no Biomes keeps the new sky blue and white.

Follow-up done: `sky_fog_at` blends only the Biomes that declare colours again
(a landform counts as "of the land's shape" only when it is coloured), and gives
`SkyFog::DEFAULT` where none in the blend does. The core test
`a_biome_without_colours_does_not_dilute_the_ones_that_have_them` is back, with
`a_colourless_landform_biome_carries_on_the_sky_around_it` beside it. The
Server still sends the default for a World of only colourless Biomes.
