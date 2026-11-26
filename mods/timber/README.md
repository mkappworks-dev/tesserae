# timber

Building timber: planks in four stains, squared frames, roofing, an awning, and
the doors and gates that hang in them, all made from `flora`'s logs and leaves
and `crops`' wheat at `workshop`'s Stations.

| Declares   | Name                                                                                                                                              |
| ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| Materials  | `planks`, `pine_planks`, `dark_planks`, `red_planks`, `frame`, `dark_frame`, `shingles`, `thatch`, `awning`, `door`, `gate` and their open states |
| Shapes     | `beam` (a bar along x), `panel` (3/16 thick against +Z: a Door's leaf, a shutter or a board)                                                      |
| Voxel uses | `door`, a Door 2 m by 3 m; `gate`, a Door 1 m by 1 m, cut out between its rails                                                                   |

| Recipe                                   | From                              | At        |
| ---------------------------------------- | --------------------------------- | --------- |
| 4 `planks`, `pine_planks`, `dark_planks` | 1 `flora:log`                     | hand      |
| 4 `red_planks`                           | 1 `flora:log`                     | workbench |
| `frame`                                  | 1 `flora:log`                     | hand      |
| `dark_frame`                             | 1 `flora:log`                     | workbench |
| 4 `shingles`                             | 2 `planks`                        | workbench |
| 2 `thatch`                               | 2 `crops:wheat`                   | hand      |
| 2 `awning`                               | 2 `crops:wheat`, 2 `flora:leaves` | workbench |
| `door`                                   | 3 `flora:log`                     | workbench |
| `gate`                                   | 2 `flora:log`                     | workbench |

**Depends on:** `flora`, `crops`, `workshop`.

**Used by:** nothing yet.

## Licence and artwork

MIT, in `LICENSE`. The PNGs in `art/` are original: `scripts/generate-mod-art`
draws them from code, so the artwork has no third-party source. Edit the script
and re-run it rather than the PNGs.

## Use it

```sh
tesserae validate mods/timber --mods mods
tesserae package mods/timber --out dist --mods mods
```

A Game lists it in `depends`. It declares no `world`, `view` or `tools`.
