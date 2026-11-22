# 09: Biomes step

Status: ready-for-agent

Blocked by: [06: Live terrain preview](./06-live-terrain-preview.md)

Spec: [Menus that feel like a game](../spec.md).

## What to build

The Biomes step has search, filter (enabled, disabled, customised) and sort
(Game order, name, temperature, humidity), a tri-state checkbox for the biomes
shown, and Reset all. Each biome shows a preview drawn from its settings. The
pencil opens an edit panel in place of the terrain preview: a climate map where
the biome is dragged or moved with arrow keys, temperature, humidity and cave
sliders, surface, soil and rock selectors, sky and fog colours, Reset and Done.
The panel scrolls on its own.

## Acceptance criteria

- [ ] Dragging or arrow-keying the biome on the climate map changes its
      temperature and humidity
- [ ] An edited biome is marked customised and its preview redraws
- [ ] Reset restores one biome; Reset all restores every biome
- [ ] Next with no biome enabled says why and stays

## Comments
