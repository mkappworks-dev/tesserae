# Engine, Manifest and Trust, headless

Status: done
Blocked by: 01

Spec decisions 3 and 11. Everything that runs before a byte of Mod code does.

## What to build

- `Manifest` (`id`, `version`, `api`, `root`, `wasm`, `depends`, optional lower
  `limits`) parsed from `mod.toml` with `toml` and `serde`; validation of the id
  grammar, the semver, the `api` line against the lines the host supports (spec
  decision 2), and the wasm path staying inside the Mod's directory (no `..`, no
  absolute path, no symlink out).
- `discover(dir) -> Vec<Found>`: one directory per Mod, a directory with no
  `mod.toml` skipped with a warning, and `plan(found)` giving the load order:
  dependencies first, then by id; an unknown `depends`, a cycle, a duplicate id
  or a second `root` is an error naming the Mods.
- `Trust`: `trust.json` read and written; a record is `(kind, id, sha256)` with
  the hash over the component bytes and the Manifest bytes together;
  `check(&Found) -> Trusted | Untrusted { reason: New | Changed }`; `grant`. A
  missing or unparseable file is an empty one, and a write is atomic (temporary
  file, then rename).
- `Engine` construction with the settings of spec decision 10 that are not
  per-call (component model on, threads, memory64 and relaxed SIMD off, fuel on,
  async). The limits struct `ModLimits` as plain `Copy` data with the spec's
  placeholder values, and `ModLimits::lowered_by(manifest)` that never raises.
- `ModHost::load(plan, trust) -> Result<Vec<Compiled>, ModError>` that compiles
  only trusted Mods and refuses the rest before reading their component.

## Tests

- A temporary directory per test (the test creates it under the cargo target
  dir, not a shared `/tmp` name): good, no-manifest, bad-id, bad-api, `wasm =
  "../x"`, cycle, two roots, unknown dependency.
- Order is dependencies first, then id, and is the same whatever order the
  directory lists them in.
- Trust: unknown Mod is `New`; after `grant` it is `Trusted`; flipping one byte
  of the component or of `mod.toml` is `Changed`. An untrusted Mod's component
  file is never opened (give it a path that does not exist and expect
  `Untrusted`, not an I/O error).
- A Manifest cannot raise a limit above the host's.

## Acceptance criteria

- `./scripts/cargo-ci` passes.
- No `unwrap` on a path from the filesystem; every error names the Mod.

## Comments

- Added `serde_json` (for `trust.json`) beyond the dependencies the spec names;
  it was already in the lockfile. Semver is checked by hand
  (`major.minor.patch` plus optional pre-release/build), not with the `semver`
  crate.
- `wasm_threads(false)` is not called: that setter exists only with wasmtime's
  `threads` feature, which is not enabled, so threads are off by construction.
  Memory64 and relaxed SIMD are set off, and fuel is on, in `host::engine`.
  Existing tests now `set_fuel` on their Stores.
- `Verdict::Trusted` carries the component bytes the hash was taken over, and
  `ModHost::load` compiles those, so a file swapped between check and compile
  is not what runs. Checking a Mod with a record has to read its component to
  compare hashes (a `Changed` verdict needs it); a Mod with no record is `New`
  without any file being opened. All Mods are checked before any is compiled.
- The hash is over the component's length (u64 LE), the component bytes, then
  the Manifest bytes, so bytes cannot move between the two unnoticed.
- `ModLimits::HOST` values are placeholders (ticket 04 measures them). The
  Manifest's `[limits]` table has one optional field per limit and
  `lowered_by` takes the minimum.
- `SUPPORTED_API` is `["0.1"]`; an older Manifest `api` fails with a message
  naming it and the supported line.
- The symlink check canonicalizes the component path when it exists and
  re-runs at read time; a missing file is not an escape.
- A missing mods directory discovers as empty. Bad Manifest errors name the
  directory when the id itself is unparseable, otherwise the id.
- `bindings.rs`, `convert.rs` and `host.rs` keep their `dead_code` allowances,
  now pointing at ticket 03: only `host::engine` is used outside tests so far.
