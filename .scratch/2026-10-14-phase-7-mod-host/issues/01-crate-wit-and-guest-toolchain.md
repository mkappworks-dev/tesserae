# The crate, the WIT package and the guest toolchain

Status: ready-for-agent
Blocked by: none

Spec decisions 1, 2 and 12. The skeleton every other ticket builds on.

## Before starting (needs the user)

`rustup target add wasm32-wasip2` downloads a toolchain component and `rustup
target list --installed` shows only `aarch64-apple-darwin` today. Ask the user
to approve it; do not run it unasked. Nothing below that builds a guest can be
verified without it.

## What to build

- `crates/tesserae-mods` as a workspace member, `edition`/`rust-version`/
  `license` from the workspace, depending on `tesserae-core` (with `version`, as
  `tesserae` does) and `wasmtime` with `default-features = false` and `runtime`,
  `cranelift`, `component-model`, `async`, `parallel-compilation`, `std`.
  Nothing else yet. A `[[licenses.exceptions]]` entry in `deny.toml` for
  `tesserae-mods`, as for the other two.
- `crates/tesserae-mods/wit/` with the real contract of spec decision 1: package
  `tesserae:mod@0.1.0`, interfaces `types`, `content`, `commands`, `host`,
  `guest`, world `mod`. Every field is in metres where it is a distance. Doc
  comments on every record say what the engine does with it.
- `wasmtime::component::bindgen!` over it in `src/bindings.rs`, with the
  generated types kept private; `lib.rs` exports nothing wasmtime.
- `scripts/build-guests`: builds every crate under
  `crates/tesserae-mods/tests/guests/` for `wasm32-wasip2` into
  `target/guests/<name>.wasm`. One trivial guest (`hello`) that exports the
  world with empty bodies. `scripts/cargo-ci` runs it before the tests, and
  `.github/workflows/ci.yml` runs `rustup target add wasm32-wasip2` and the
  script in the Test job.
- Root `Cargo.toml` `exclude` gains the guests directory, for the reason
  `games/demo` is in it.

## Tests

- Load `hello`, instantiate it, call `declare`: an empty Declaration.
- The shipped world's import list, read from the built component's type, is
  exactly `tesserae:mod/host` plus WASI interfaces (the ADR-0001 test of spec
  invariant 1). Add a fake guest import of `set-voxel` to a scratch copy of the
  WIT and see the test fail.
- **The export-version question of spec decision 2**: build `hello` against
  `@0.1.0`, give the host a `0.1.1` copy of the WIT with one added
  `@since(version = 0.1.1)` function, and record whether it instantiates and
  whether its exports are found. Write the finding in Comments; if exports are
  matched exactly, implement the fallback lookup and test it.
- The Declaration types convert to `tesserae-core`'s `ShapeDef` and
  `MaterialDef` without loss, field by field (a table-driven test over every
  field of both).

## Acceptance criteria

- `./scripts/cargo-ci` passes, and its Boundaries step prints `tesserae-mods is
  clean` and not "does not exist yet".
- `cargo tree -p tesserae-mods --edges normal` shows no `bevy` or `wgpu`.
- `cargo deny check` passes.
