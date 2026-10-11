# Tesserae

A high-performance, moddable voxel engine. Rust, Bevy 0.20, macOS first.

Tesserae ships no gameplay. A **Game** is a Mod with a root manifest, built on
the engine and living outside this repository.

## Status: early, and not usable for anything yet

Step **2 of 10** in [DESIGN.md](DESIGN.md)'s build order. `cargo run` opens a
window on nine Chunks of test terrain you can fly around. That is all it does.

| works today                                 | not written yet                                  |
| ------------------------------------------- | ------------------------------------------------ |
| Palette-compressed 32³ Chunk storage        | World generation — the terrain is a test fixture |
| Greedy mesher, ~360 µs per terrain Chunk    | Chunk streaming, and the client/server split     |
| PBR shading, directional light, shadow maps | Per-voxel light propagation                      |
| Fly camera                                  | Non-cube Shapes: stairs, slabs, fences           |
|                                             | LOD, and the 4km view distance it is for         |
| &nbsp;                                      | WASM mods, the WIT contract, look packs          |
|                                             | Fluids, collision, network play                  |

The right-hand column is the roadmap, not a feature list. Nothing in it works.

## Requirements

- Rust 1.97 or later — `rustup toolchain install stable`
- A GPU with Metal (macOS 11+), Vulkan (Linux) or DX12 (Windows)
- Node, only to pin Prettier — `npm install`

macOS is the primary development platform; Linux and Windows follow. Mobile, web
and consoles are explicit non-goals.

## Commands

```bash
cargo run -p tesserae --features dev
```

Aliased to `cargo dev`. `--features dev` adds hot-reloading for shaders and
assets; note that `cargo run -p tesserae dev` is **not** the same thing — that
passes `dev` to the binary as an argument and leaves the feature off. Controls,
and what each kind of edit costs to rebuild, are in
[CONTRIBUTING.md](CONTRIBUTING.md#development-setup).

```bash
cargo fmt --all && npm run fmt
```

```bash
cargo test --workspace && cargo lint
```

```bash
cargo bench -p tesserae-core
```

Everything CI checks, in CI's order, fail-fast:

```bash
./scripts/cargo-ci
```

Put `scripts/` on your PATH and that is `cargo ci` — cargo resolves
`cargo <name>` to a `cargo-<name>` executable. It cannot be a `[alias]`: cargo
aliases expand to cargo subcommands only, with no shell escape.

The first two of those gate CI, and the bench gates any change to the mesher,
chunk storage, lighting or fluids — put the before and after numbers in the PR.
The full pre-merge checklist is in
[the pull request template](.github/pull_request_template.md).

## Layout

Three crates, and one boundary that matters: `tesserae-core` depends on neither
Bevy nor wgpu, which is what keeps every expensive algorithm headless and
benchmarkable. CI enforces it.

| crate                  | what                                                          |
| ---------------------- | ------------------------------------------------------------- |
| `crates/tesserae-core` | Chunk storage, worldgen, meshing, lighting, fluids, collision |
| `crates/tesserae-mods` | WASM host, WIT contract, command queue — step 7               |
| `crates/tesserae`      | The Bevy app: rendering, input, and the in-process server     |

## Reading order

- [CONTEXT.md](CONTEXT.md) — the glossary. Voxel, Material, Shape, Chunk, Cell.
  These are the only names used in code, docs and the mod API.
- [docs/architecture.md](docs/architecture.md) — crate boundaries, a diagram,
  and the three invariants a change can break while looking correct.
- [DESIGN.md](DESIGN.md) — every decision and why, including the ones rejected.
- [docs/adr/](docs/adr/) — the decisions that are hard to reverse.
- [CONTRIBUTING.md](CONTRIBUTING.md) — setup, the dev loop, and the workflow.

## Licence

Business Source License 1.1, converting to **Apache-2.0 four years after each
release**. BSL's only prohibition is production use, and Tesserae's Additional
Use Grant gives that back for everything except reselling the engine itself.

**You may build a game on Tesserae and sell it** — commercially, at any price,
with no royalty, no revenue threshold and no reporting. You may ship your game's
binaries, run servers, sell mods and look packs, modify the engine, and fork and
redistribute this repository.

[USE-GRANT.md](USE-GRANT.md) is the plain-English summary and the file to read
first. [LICENSE](LICENSE) governs where the two differ.

> GitHub's sidebar shows a bare "View license" link rather than the licence
> name, because the library it uses to detect licences does not carry BUSL. That
> is a gap in GitHub's tooling, not an unlicensed repository.

`assets/` is **CC0**, not BSL, so the placeholder art is reusable — see
[assets/README.md](assets/README.md). Contributions require a CLA; see
[CONTRIBUTING.md](CONTRIBUTING.md#contributor-license-agreement-cla).
