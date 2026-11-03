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
- [ ] Record manual edit-and-restart results for both authoring paths, including
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

Done except the manual criterion, which is **outstanding**: no display was
available, so `tesserae dev` was never run in a real window.

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

- **Manual edit-and-restart in the running window, for either authoring path,
  including failure and correction.** `tesserae dev` started headless panics
  (`Resource does not exist`) as expected without a display; that is not
  evidence about the loop. To do: run `tesserae dev` on a copy of each starter,
  save a data/PNG change and a Rust change, break each, fix each, and record
  what the window showed (title, same seed, spawn reset, Edits discarded).
- Installing the packaged folder into `games/` and creating a World through the
  Create screen (the UI); the tests use the loader and reopen path.
- Build timeout and cancellation do not exist; the guide and Rust README say so.
  A hanging build command holds the loop.
- `cargo-deny` printed one "license not encountered" warning (BSL-1.0 in
  `deny.toml`); it passed.

Roadmap: DESIGN.md step 10 reads "Check pending", not Done, and README says the
SDK is built and tested but not marked done. Step 10 completes when the manual
check above is recorded; nothing else in the parent spec's scope is outstanding.

Checks: `./scripts/cargo-ci` (rustfmt, Prettier, clippy, docs, default-features
check, guests, tests with `tesserae/dev`, benches compile, boundaries,
cargo-deny) passed; `cargo test --workspace --all-features` passed (843 in
`tesserae`, 411, 2, 77, 138; 3 ignored each in `tesserae` and the last crate,
as before).
