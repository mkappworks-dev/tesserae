# 01: Ship the five shared data Mods

Status: done

Blocked by: the terra rename merged

## What to build

Create `mods/strata`, `mods/waters`, `mods/flora`, `mods/hearth` and `mods/glassworks` as data-only Mods per the spec, each with a `mod.toml` (contract 0.3, `declaration = "content.toml"`), a Declaration and original or CC0 PNG textures.

## Acceptance criteria

- [x] Every Mod validates with `tesserae validate mods/<id>` and packages.
- [x] Dependencies between the Mods are declared with `depends` and load in order.
- [x] A test loads all five together and checks that every cross-Mod reference resolves.
- [x] Each Mod has a README: purpose, the Materials, fluids and Shapes it declares, what depends on it, and the licence and artwork sources.
- [x] CI validates `mods/`.
