# 09: Biomes step

Status: done

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

- [x] Dragging or arrow-keying the biome on the climate map changes its
      temperature and humidity
- [x] An edited biome is marked customised and its preview redraws
- [x] Reset restores one biome; Reset all restores every biome
- [x] Next with no biome enabled says why and stays

## Comments

Done on `feat/game-menus-09-biomes-step`. A Biome's preview is a cut through it
(sky fading to fog over surface, soil and rock bands, the rock holed by its cave
scale), drawn from Material colours once the Game is read. The ground selectors
cycle to the next Material on click or Enter, as before; the mockup's dropdowns
are not built. Sky and fog stay `#rrggbb` fields, without the mockup's colour
swatches. Arrow keys on the climate map's handle move the Biome (Shift ×10);
the D-pad still navigates past it so a controller is never trapped, and pad
players use the sliders. Reset all still resets the Climate sizes on the World
step too, as its dialog says.
