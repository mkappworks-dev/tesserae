# 12: Every Template Material is Obtainable

Status: done

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

- [x] A Template using only Obtainable Materials loads.
- [x] A Template using a Material only a Recipe makes loads when the Recipe's
      inputs are Obtainable, and is refused when they are not.
- [x] The refusal names the Template, the palette character and the Material, in
      Create World and on opening.
- [x] A Grown Material or a Door's open Material in a Template is refused.
- [x] A World whose Biomes list nothing still opens; the check is computed once
      per composition.

## Comments

- The fixed point lives in `crates/tesserae/src/server/obtainable.rs`
  (`obtainable(&LoadedMods, &Items) -> HashSet<MaterialId>`) and runs once at
  the end of `declare_all` in `server/mods.rs`, after Recipes and the World's
  Biomes and ores are resolved. Create World (`mods::preview`), opening
  (`load_with_opening`), the Mod clash check and `author.rs` all compose
  through `declare_all`, so every Game, ticket 16's included, is checked with
  no extra call.
- Obtainable is over Items: a Material is Obtainable when an Item that places
  it can be held. Sources: the World's ground, each Biome's ground, the lake
  fluid, every surface feature's Material, the Plant it generates and the
  crop fields, and every ore. Nothing is laid without the World's ground,
  since worldgen lays nothing then. Breaking a laid Material gives its Plant
  yields, else the Item that places it; a laid Plant Material grows every
  Material of that Plant (and a tree's vine Plant). Holding an Item that
  places a Material lays it, so a sapling grows its tree. Recipes then add
  outputs whose inputs are all held, to a fixed point; stations are not
  checked. A declared Plant nothing lays yields nothing.
- `DeclaredTemplates::check(&registry, refuse)` in `tesserae-mods` names the
  Template, the palette character (or `foundation`) and the Material. Every
  palette entry is checked, used in the layers or not, in symbol order.
- A Grown Material or a Door's open Material is refused in the shared
  validator (`solid()` in `tesserae-mods/src/template.rs`), so the data path
  and a component refuse it alike, before Obtainable is worked out.
- For now every Template the World declares is checked. Ticket 03/17 may
  narrow it to the Templates the Biomes' `structures` rows reference (see the
  comment in `declare_all`). Ticket 13's `ground` entry is a non-Voxel palette
  cell, which `check` already skips, so it is always Obtainable.
- Tests: `server::mods::tests` `a_template_material_only_a_recipe_makes_*`,
  `what_biomes_lay_plants_yield_and_ores_hold_is_obtainable` and
  `a_grown_material_or_a_doors_open_material_in_a_template_is_refused`, each
  through Create World and opening.
