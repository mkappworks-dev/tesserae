# Engine, Manifest and Trust, headless

Status: ready-for-agent
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
