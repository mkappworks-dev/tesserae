# The crate, the WIT package and the guest toolchain

Status: done
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

## Comments

What differed from the ticket:

- **The package is `tesserae:mods`, not `tesserae:mod`.** `wit-bindgen` 0.57 and
  0.62 emit the package's name segment as a Rust module path without escaping
  it (the namespace and interface names are escaped; the package name is not),
  so a guest in a package called `mod` fails to compile with "expected
  identifier, found keyword `mod`". Every Rust Mod author would hit it. The
  world is still `mod`, which is fine. Spec decisions 1 and 2 and the interface
  names in `host`/`guest` should be read with `tesserae:mods`; the component
  names are `tesserae:mods/host@0.1.0` and `tesserae:mods/guest@0.1.0`.
- **`world` is a WIT keyword**, so the record field and the `start` parameter
  are written `%world`; the Rust name and the wire name are still `world`.
- **`wasmtime-wasi` (`p2` only) is a dependency now.** A `std` guest imports
  WASI, so instantiating `hello` needs it linked; the ticket said "nothing else
  yet".
- **The contract is wider than the spec's sketch.** Added `cave-params`,
  `smoothing-params`, `water-params`, `world-info`, `registered`,
  `registered-id`, `tick-context`, `edit-event`, `batch-report` and `srgba`
  (RGBA bytes). `registered` is two lists of `(full name, id)`.
- **Type-only interfaces are imports.** `types`, `content` and `commands`
  appear in a component's import list because the world's types come from them.
  They carry no functions. The import test therefore checks functions: `host`
  may expose exactly `log` and `default-world-params`, the type-only interfaces
  none, and anything else that is not `wasi:` is a violation. Adding a
  `set-voxel` function to `host` in a scratch WIT, with a guest that calls it,
  fails the test as `tesserae:mods/host@0.1.0.set-voxel`.
- `hello` does not call `host`, so its component does not import it; the test
  checks what is there, and the negative check above is the proof it can fail.
- The conversion is in `convert.rs` as `From` impls, in both directions where
  the contract hands core data to a Mod (`WorldgenParams`, boxes). It does no
  validation; registration does.
- `wasmtime` resolved to 49.0.3 (`49.0.2` is the floor in `Cargo.toml`).
- `scripts/build-guests` runs with `RUSTFLAGS` cleared, and the guest's `target`
  directory is gitignored. Each guest is its own workspace with its own
  `Cargo.lock`, built `--locked`.

**The export-version question (spec decision 2).** A component built against
`0.1.0` instantiates on a host whose Linker defines `0.1.1`, and its exports are
found under the newer name too: `get_export_index` for
`tesserae:mods/guest@0.1.1` and for `...@0.1.0` both resolve. Exports match
semver-compatibly, as imports do, so **no fallback lookup was needed and none
is implemented.** What does fail is the generated `0.1.1` bindings: they require
every function of the interface, and instantiation reports "instance export
`tesserae:mods/guest@0.1.1` does not have export `ping`" for the one added
function. So a host built against a newer patch must look up a function marked
`@since` as optional (through the export index) rather than through the
generated `instantiate`. The test is `tests::export_version`, against a `0.1.1`
copy of the WIT in `tests/fixtures/wit-0.1.1/`. Not tested: a guest built
against `0.1.1` on a host defining only `0.1.0`; the Manifest's `api` check is
what turns that into a sentence.
