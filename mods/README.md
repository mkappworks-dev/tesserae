# Shared Mods

Data-only Mods that Games depend on, each in `mods/<id>/`. They have no build: a
Mod is a folder of TOML and PNG files. Only a Game declares `world`, `view` and
`tools`, so these hold content alone.

| Mod                       | Content                                           | Depends on            |
| ------------------------- | ------------------------------------------------- | --------------------- |
| [strata](strata/)         | Stone, basalt, clay, sand, soil, turf             | -                     |
| [waters](waters/)         | Water fluid, steam, obsidian                      | -                     |
| [flora](flora/)           | Wood, leaves, grass, kelp                         | -                     |
| [hearth](hearth/)         | Lava, ash; lava and water reactions; fuel         | strata, waters, flora |
| [glassworks](glassworks/) | Glass, frosted glass, lantern; slab, stair, fence | -                     |

Validate and package all of them with the real commands:

```sh
tesserae validate mods/* --mods mods
tesserae package mods/* --out dist --mods mods
```
