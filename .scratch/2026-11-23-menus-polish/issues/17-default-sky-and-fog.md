# 17: Default sky blue and white fog

Status: ready-for-agent

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

- [ ] In a World, standing in a biome with no declared sky and fog shows a sky
      blue sky and white fog
- [ ] The Biomes step shows `#87ceeb` and `#ffffff` for such a biome, and Reset
      returns to them
- [ ] A biome that declares its own colours is unchanged
- [ ] The defaults are defined in one place

## Comments
