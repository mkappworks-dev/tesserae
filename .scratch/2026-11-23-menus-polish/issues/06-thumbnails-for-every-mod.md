# 06: A thumbnail for every Game and Mod

Status: done

Blocked by: None (can start immediately)

Spec: [Menus polish](../spec.md).

## What to build

`scripts/generate-thumbnails` makes a thumbnail for every demo Game and Mod that has none, including Terra's required Mods (Aquatic, Crops, Flora, Strata, Waters) and the rest of `mods/`. A Character Mod's (Sprite, Wanderer, Golem) thumbnail draws its figure from its own skin.

## Acceptance criteria

- [x] Every Game and Mod in the repo declares a valid 512×288 thumbnail
- [x] Each Character Mod's thumbnail shows its figure
- [x] The Mods step and Mods library show no colour-strip fallback for a shipped Mod

## Comments

- Crystals' and Frost's thumbnails changed too: see-through texture pixels (ice,
  glass, water) now blend over the backdrop instead of drawing as dark holes.
- Crops shows wheat, carrots and berries as plants on soil and turf; the
  pumpkin's side texture has a see-through band and read as a black stripe.
- Every Mod's build hash changed with its `thumbnail` line: local Worlds saved
  with the old builds won't reopen, and Trust is asked again.
