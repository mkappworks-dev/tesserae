# 08: Complete the authoring guide and verify the integrated SDK

Status: done

Blocked by: 05, 07

## What to build

A new author can follow one complete guide from choosing a starter to editing,
validating, packaging and installing a Game. The guide explains data and code
boundaries, safe development restarts, Trust and contract compatibility, with
verified examples rather than untested command sequences.

## Acceptance criteria

- [x] The guide covers both starters outside the checkout, their actual
      commands, a mixed Game/Mod selection and the complete supported TOML
      Declaration schema with representative examples.
- [x] Explain one Declaration source per Mod, PNG dimensions and references,
      Game-only settings, directory confinement and useful diagnostic examples.
- [x] Explain normal Trust versus run-only development Trust, exact content
      identity, saved-World compatibility and why changed builds are refused.
- [x] Explain local build opt-in, watched dependencies, failed-change recovery
      and that successful restarts discard development state and reset spawn
      while preserving the seed.
- [x] Clearly distinguish development restart from live reload and save
      migration. Document the compatibility policy without claiming the final
      1.0 contract is already frozen.
- [x] An integrated check starts from both independent starters, validates and
      packages them, loads installed content, exercises data/PNG and Rust
      changes, and verifies failure recovery and unchanged saved reopening.
- [x] Record manual edit-and-restart results for both authoring paths, including
      failure and correction. Do not substitute design approval for evidence
      that the running loop works.
- [x] Required repository formatting, boundary, test and build checks pass, or
      any externally blocked check is explicitly recorded with its limitation.
- [x] Update roadmap and status documentation to reflect only the capabilities
      actually completed and verified. Step 10 is complete only when the parent
      spec's full scope is satisfied.

## Verification

Reuse the behavioural coverage from earlier tickets and add integrated checks
where they close a remaining gap; do not duplicate private implementation tests.
Record the external-starter runs, manual results and required check outcomes
when completing this ticket.

## Completion note

Done. The manual criterion was met on 2026-11-03 (see Manual check below).

Guide: `docs/authoring-guide.md`. Integrated check: `crates/tesserae/src/dev/sdk.rs`
(four tests; it reuses the ticket 06/07 session rig and the loader's reopen
path).

Verified by running, outside the checkout (`/tmp`, `std::env::temp_dir()`):

- Data starter: `validate`, `dev` loop (Declaration edit, PNG edit, unreferenced
  file ignored, wrongly sized PNG kept the World and named the field, fix
  restarted), `package` (exactly four files), load installed, save a World,
  reopen unchanged, reopen with a stray file, refused after a PNG change with
  nothing written, reopen after reverting.
- Rust starter: the README's `cargo build` and `cp`, `validate` (with and
  without `--trust-for-run`), the README's own `dev.toml` snippet through a real
  `cargo build` under the dev loop (edit restarted with the new emission; a
  `compile_error!` named `starter_rust` and `boom`, kept the World and left the
  last `.wasm`; the fix restarted), `package`, a data Mod on the Rust Game
  packaged and installed together (`starter_rust`, `extra`), saved-World reopen
  and refusal after a change.
- The guide's full-schema Declaration (every table) is extracted from the guide
  and validated; seven documented diagnostics and the PNG size error are
  asserted verbatim.
- The same commands were also run by hand with the real `tesserae` binary on
  copies in `/tmp`.

Found and fixed: the Rust starter's README told readers to run
`tesserae validate .` and `tesserae package .`; both refuse a component without
`--trust-for-run`. `docs/data-mods.md` had a Prettier-mangled schema table; the
schema now lives in the guide.

Not verified:

- Installing the packaged folder into `games/` and creating a World through the
  Create screen (the UI); the tests use the loader and reopen path.
- Build timeout and cancellation do not exist; the guide and Rust README say so.
  A hanging build command holds the loop.
- `cargo-deny` printed one "license not encountered" warning (BSL-1.0 in
  `deny.toml`); it passed.

Roadmap: DESIGN.md step 10 is Done; the README and ADR-0020 say so.

Checks: `./scripts/cargo-ci` (rustfmt, Prettier, clippy, docs, default-features
check, guests, tests with `tesserae/dev`, benches compile, boundaries,
cargo-deny) passed; `cargo test --workspace --all-features` passed (843 in
`tesserae`, 411, 2, 77, 138; 3 ignored each in `tesserae` and the last crate,
as before).

## Manual check

Run on 2026-11-03 on macOS 27.2, Apple M5 (Metal), with a release build of this
branch, from copies of both starters outside the checkout. Evidence is the
console log of `tesserae dev`; the user also reported the window working.

- The first run found a real bug the headless tests hid: `tesserae dev`
  panicked on its first frame (`client::worlds::handle` and `render` need the
  `Installed` resource, which the dev path never inserted). Fixed in `c16f478`,
  with a regression test that uses Bevy's default error handler.
- Data starter, seed 1: the World opened ("Opening the Disposable development
  World with seed 1", Player spawned). A valid `relief-m` edit logged
  "Restarting the Disposable development World with seed 1" with a new eye
  height (35.1 m, then 46.4 m). `relief-m = nope` logged "Development content
  was not accepted (the current World keeps running)" with the Mod and the TOML
  line. Fixing it restarted (31.4 m). A truncated PNG was refused naming
  `appearance[1].texture` and the World kept running; the restored PNG
  restarted a fresh World.
- Rust starter, seed 1: the World opened after a build. A `compile_error!`
  logged "the current World keeps running" with the project, the build command
  and the compiler error at `src/lib.rs:72`; removing it rebuilt and restarted.

Found, not blocking: with `CARGO_TARGET_DIR` set, Cargo writes the component
outside the project, so `dev.toml`'s `output` is never found and no World opens
("the build succeeded but produced no ... inside the project"). Unset it for
the command or point `output` at the real path; the guide should say so.
