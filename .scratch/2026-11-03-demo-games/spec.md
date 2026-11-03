# Demo Games and shared Mods

Status: done

## Problem Statement

The repository ships two Games, `demo` and `meadow`, each one self-contained
Rust component. Nothing in the repository shows the part of the SDK that matters
most to authors: a small Rust Game for behaviour, with content shared as data
Mods that other Games reuse. Data-only Mods are new and have no shipped
examples, and no Game in the repository uses `depends`.

## Solution

Keep `terra` (the renamed `demo`) as the one self-contained Rust Game and the
template ADR-0004 describes. Add a top-level `mods/` folder of shared data Mods,
and rebuild `meadow` plus three new Games on them. Each of those four Games is a
small Rust component that declares its World and adds one behaviour, and takes
its Materials, fluids and Shapes from the data Mods it depends on. Every Game
and Mod is documented.

| Game       | Behaviour (Rust)                           | Depends on                      |
| ---------- | ------------------------------------------ | ------------------------------- |
| terra      | Everything; self-contained, unchanged      | -                               |
| meadow     | Gentle land; a few trees placed on start   | strata, flora, glassworks       |
| ember      | Lava vents erupt on a timer                | strata, waters, flora, hearth   |
| tidewater  | Sea level rises and falls slowly           | strata, waters, flora           |
| lumen      | Lanterns light up in sequence              | strata, glassworks              |

| Mod        | Content (data only)                                     |
| ---------- | ------------------------------------------------------- |
| strata     | Stone, basalt, clay, sand, soil, turf, with PNG textures |
| waters     | Water fluid; water, lava, steam and obsidian reactions   |
| flora      | Wood, leaves, grass, kelp; wood burns                    |
| hearth     | Fire, ash, lava; combustion rules                        |
| glassworks | Glass, frosted glass, lantern; slab, stair, fence Shapes |

## Decisions

- Only Games declare `world`, `view` and `tools`, so Mods hold content only and
  each Game declares its own World referencing `<mod id>:<name>` Materials.
- Mods are licensed MIT or CC0 like `terra`, because authors copy them
  (ADR-0004). PNG artwork is original or CC0 and its source is recorded.
- Mods live in `mods/<id>/`, Games in `games/<id>/`. Games are not workspace
  members; Mods have no build.
- No content is built into the Engine (ADR-0016); every Material is declared.
- The `tesserae validate` and `package` commands accept the whole set, and CI
  validates it.

## Out of scope

- Changing `terra` beyond its rename.
- New Engine capabilities. A Game that needs one the contract lacks is a
  contract gap to record, not to work around (ADR-0004).
