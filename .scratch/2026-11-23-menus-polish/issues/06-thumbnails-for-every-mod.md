# 06: A thumbnail for every Game and Mod

Status: ready-for-agent

Blocked by: None (can start immediately)

Spec: [Menus polish](../spec.md).

## What to build

`scripts/generate-thumbnails` makes a thumbnail for every demo Game and Mod that has none, including Terra's required Mods (Aquatic, Crops, Flora, Strata, Waters) and the rest of `mods/`. A Character Mod's (Sprite, Wanderer, Golem) thumbnail draws its figure from its own skin.

## Acceptance criteria

- [ ] Every Game and Mod in the repo declares a valid 512×288 thumbnail
- [ ] Each Character Mod's thumbnail shows its figure
- [ ] The Mods step and Mods library show no colour-strip fallback for a shipped Mod

## Comments
