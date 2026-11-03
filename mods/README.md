# Shared Mods

Data-only Mods that Games depend on, each in `mods/<id>/`. They have no build: a
Mod is a folder of TOML and PNG files. Only a Game declares `world`, `view` and
`tools`, so these hold content alone.

| Mod                       | Content                                                   | Depends on            |
| ------------------------- | --------------------------------------------------------- | --------------------- |
| [strata](strata/)         | Stone, basalt, clay, sand, soil, turf                     | -                     |
| [waters](waters/)         | Water fluid, steam, obsidian                              | -                     |
| [flora](flora/)           | Wood, leaves, grass, kelp                                 | -                     |
| [hearth](hearth/)         | Lava, ash; lava and water reactions; wood and leaves burn | strata, waters, flora |
| [glassworks](glassworks/) | Glass, frosted glass, lantern; slab, stair, fence         | -                     |
| [timber](timber/)         | Planks, thatch; beam and panel Shapes                     | -                     |
| [brickwork](brickwork/)   | Brick, cobble, tile; connecting wall Shape                | -                     |
| [lamps](lamps/)           | Glowstone, amberlight, moonlight; bulb Shape              | -                     |
| [wool](wool/)             | Six coloured cloths; carpet Shape                         | -                     |
| [frost](frost/)           | Ice, snow, packed ice, slush fluid; snow layer Shape      | -                     |
| [crystals](crystals/)     | Amethyst, citrine, aquamarine; cluster Shape              | -                     |
| [bog](bog/)               | Peat, mud and tar fluids; lava reactions; peat burns      | hearth, brickwork     |
| [wanderer](wanderer/)     | Character: a traveller, 1.8 m                             | -                     |
| [golem](golem/)           | Character: a slow stone figure, 2.16 m                    | -                     |
| [sprite](sprite/)         | Character: a small quick figure, 1.44 m                   | -                     |

Validate and package all of them with the real commands:

```sh
for mod in mods/*/; do tesserae validate "$mod" --mods mods; done
for mod in mods/*/; do tesserae package "$mod" --out dist --mods mods; done
```
