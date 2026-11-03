# Rust starter

A Game written in Rust and compiled to a WebAssembly component. It declares two
Materials and ground, and has empty `start` and `tick` callbacks to grow from.

**Supported contract line:** 0.3, from the bundled `wit/` directory
(`tesserae:mods@0.3.0`). The only dependency is `wit-bindgen`, from crates.io;
there are no Engine crates and no paths outside this directory.

## Toolchain

```sh
rustup target add wasm32-wasip2
```

## Build

```sh
cargo build --target wasm32-wasip2 --release
cp target/wasm32-wasip2/release/starter_rust.wasm .
```

Rename the package in `Cargo.toml`, `wasm` in `mod.toml`, the `id` in `mod.toml`
and the `starter_rust:` names in `src/lib.rs` together.

## Validate

```sh
tesserae validate . --trust-for-run
```

It loads the component under the usual sandbox and limits, reports the Mod and
field of any error, and exits non-zero on failure. It opens no World. A
component runs code, so without a recorded Trust grant it needs
`--trust-for-run`, which applies to this project only and writes nothing.

## Watch

```sh
tesserae dev . [--mods <dir>] [--seed <n>]
```

Opens a disposable World from this project and restarts it when `mod.toml` or
the copied `.wasm` changes and still validates; otherwise the running World
stays and the terminal explains why. By default it never runs Cargo: rebuild and
copy the `.wasm` as above, and the new build is picked up. The project is
trusted for that run only; nothing is saved and no Trust is recorded.

To have `tesserae dev` rebuild on every source edit, opt in with a `dev.toml`
beside `mod.toml` (it is left out of the package):

```toml
[build]
enabled = true
command = ["cargo", "build", "--target", "wasm32-wasip2", "--release"]
output = "target/wasm32-wasip2/release/starter_rust.wasm"
```

The built component is copied over `starter_rust.wasm` only when Cargo succeeds.
A compile error is printed with the project name and the running World stays
until the next edit. The build has no timeout and cannot be cancelled from
`tesserae dev`. Rename `starter_rust.wasm` in `output` along with the other
names. The build runs with your permissions: enable it only for code you trust.

## Package

```sh
tesserae package . --out dist --trust-for-run
```

This writes `dist/games/<id>/` holding `mod.toml` and the built `.wasm`. Build
first; packaging does not run Cargo. By hand, copy exactly those two files.

## Install

Copy `dist/games/<id>/` into the Engine's `games/` directory (beside `mods/`),
then start the Engine and create a World with the Game. The first run asks you
to trust the build; rebuilding asks again.

## The WIT bundle

`wit/` is a verbatim copy of the Engine's published 0.3 contract. Edit nothing
in it; to move to a newer line, replace the whole directory and change `api` in
`mod.toml`. Types and host calls reach your Game only through `wit-bindgen`'s
bindings for that directory.

The whole workflow, including mixing with data Mods and how Trust and saved
Worlds treat a rebuilt component, is in
[`docs/authoring-guide.md`](../../docs/authoring-guide.md).
