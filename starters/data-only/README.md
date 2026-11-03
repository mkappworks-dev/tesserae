# Data-only starter

A Game made of text and PNG files: two Materials with artwork, ground built from
them, and flight and physics enabled. No Rust, no WebAssembly toolchain.

**Supported contract line:** 0.3 (`api = "0.3"` in `mod.toml`). The field
reference is [`docs/data-mods.md`](../../docs/data-mods.md) in the Engine
repository.

## Layout

| File                            | Purpose                                                    |
| ------------------------------- | ---------------------------------------------------------- |
| `mod.toml`                      | Manifest: id, version, contract line, `declaration` source |
| `content.toml`                  | The Declaration: Materials, appearance, ground, movement   |
| `art/stone.png`, `art/turf.png` | 48 by 16 pixel textures: three 16 by 16 faces side by side |

Rename the `id` in `mod.toml` (`[a-z][a-z0-9_]{0,31}`) and replace every
`starter_data:` in `content.toml` with it.

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

The whole workflow, including the full Declaration schema, diagnostics and how
Trust and saved Worlds treat a changed build, is in
[`docs/authoring-guide.md`](../../docs/authoring-guide.md).
