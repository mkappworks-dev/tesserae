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
tesserae validate .
```

It loads the component under the usual sandbox and limits, reports the Mod and
field of any error, and exits non-zero on failure. It opens no World.

## Package

```sh
tesserae package . --out dist
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
