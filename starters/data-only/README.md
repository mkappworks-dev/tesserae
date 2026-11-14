# Data-only starter

A Game made of text and PNG files: two Materials with artwork, ground built from
them, flight and physics enabled, a Workbench, a Chest and a Door with two
Recipes, and one growable crop with a seed Item and a `cross` Shape. No Rust, no
WebAssembly toolchain.

**Supported contract line:** 0.3 (`api = "0.3"` in `mod.toml`). The field
reference is [`docs/data-mods.md`](../../docs/data-mods.md) in the Engine
repository.

## Layout

| File                            | Purpose                                                                                     |
| ------------------------------- | ------------------------------------------------------------------------------------------- |
| `mod.toml`                      | Manifest: id, version, contract line, `declaration` source                                  |
| `content.toml`                  | The Declaration: Materials, appearance, ground, movement, Usable Voxels, Recipes and a crop |
| `art/stone.png`, `art/turf.png` | 48 by 16 pixel textures: three 16 by 16 faces side by side                                  |

Rename the `id` in `mod.toml` (`[a-z][a-z0-9_]{0,31}`) and replace every
`starter_data:` in `content.toml` with it.

## The crop

`content.toml` ends with a `crop`: the `grain` Plant passes through the
Materials `sprout`, `young` and `ripe`, drawn with the `cross` Shape and planted
on `turf` from the `seeds` Item. Place a seed on turf in light and it ripens
over about a minute; breaking it ripe drops `grain` and more seeds. Its
Materials have colour only: give each an `appearance` `texture` (a 48 by 16 PNG)
and the `grain` Item a `texture` (a 16 by 16 PNG) to draw them from artwork. The
field reference is the authoring guide's
[Plants](../../docs/authoring-guide.md#plants).

## Validate

```sh
tesserae validate .
```

It reports the Mod and field of any error and exits non-zero on failure. It
opens no World.

## Watch

```sh
tesserae dev . [--mods <dir>] [--seed <n>]
```

Opens a disposable World from this project and restarts it when you save a valid
change to `mod.toml`, `content.toml` or a referenced PNG. An invalid change is
explained in the terminal and the running World stays; the next valid save
recovers. Nothing is saved and no Trust is recorded.

## Package

```sh
tesserae package . --out dist
```

This writes `dist/games/<id>/` holding `mod.toml`, `content.toml` and the
referenced PNGs, and nothing else. By hand, copy exactly those files.

## Install

Copy `dist/games/<id>/` into the Engine's `games/` directory (beside `mods/`),
then start the Engine and create a World with the Game. The first run asks you
to trust the build; changing any listed file asks again.

For Mods other Games depend on, see the [shared Mods](../../docs/demo-games.md).

The whole workflow, including the full Declaration schema, diagnostics and how
Trust and saved Worlds treat a changed build, is in
[`docs/authoring-guide.md`](../../docs/authoring-guide.md).
