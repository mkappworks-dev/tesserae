# 12: Every Template Material is Obtainable

Status: ready-for-agent

Blocked by: 02

Spec: [Phase 20](../spec.md).

## What to build

When a World's Game and Mods are composed, the Server works out which
Materials are Obtainable: what the Biomes lay (ground, soil, ores, surface
features), what Plants yield and what breaking any of those drops, then every
Recipe output whose inputs are already Obtainable, until nothing changes.
Structures, the Catalogue and Mod Commands are not sources. A Template naming a
Material that is not Obtainable, a Grown Material or a Door's open Material
refuses the World with the reason.

## Acceptance criteria

- [ ] A Template using only Obtainable Materials loads.
- [ ] A Template using a Material only a Recipe makes loads when the Recipe's
      inputs are Obtainable, and is refused when they are not.
- [ ] The refusal names the Template, the palette character and the Material, in
      Create World and on opening.
- [ ] A Grown Material or a Door's open Material in a Template is refused.
- [ ] A World whose Biomes list nothing still opens; the check is computed once
      per composition.
